// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use inotify::{Event, EventMask, Inotify, WatchDescriptor, WatchMask};
use log::{debug, error, info};

use super::{manager::PatchManager, PATCH_INFO_FILE_NAME, PATCH_INSTALL_DIR};

const MONITOR_THREAD_NAME: &str = "patch_monitor";
const MONITOR_THREAD_TIMEOUT: Duration = Duration::from_millis(100);

const ROOT_DIR_WATCH_MASK: WatchMask = WatchMask::empty()
    .union(WatchMask::CREATE)
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::DELETE)
    .union(WatchMask::MOVED_FROM)
    .union(WatchMask::ONLYDIR);

const PATCH_DIR_WATCH_MASK: WatchMask = WatchMask::empty()
    .union(WatchMask::CREATE)
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::DELETE)
    .union(WatchMask::MOVED_FROM);

const EVENT_BUFFER_CAPACITY: usize = 16 * 1024; // inotify event size: 16

/// Runtime state for inotify-based patch directory monitoring.
struct InotifyMonitor {
    /// Underlying inotify handle used to read and manage watches.
    inotify: Inotify,
    /// Local cache: watched directory path -> watch descriptor.
    wd_map: HashMap<PathBuf, WatchDescriptor>,
    /// Local cache: watch descriptor -> watched directory path.
    path_map: HashMap<WatchDescriptor, PathBuf>,
}

impl InotifyMonitor {
    /// Create an empty monitor with one initialized inotify instance.
    #[inline]
    fn new() -> io::Result<Self> {
        let monitor = Self {
            inotify: Inotify::init()?,
            wd_map: HashMap::new(),
            path_map: HashMap::new(),
        };

        Ok(monitor)
    }

    /// Add or refresh a watch for `path`, keeping local maps in sync.
    #[inline]
    fn add_watch_path<P: AsRef<Path>>(&mut self, path: P, mask: WatchMask) -> io::Result<()> {
        let path = path.as_ref().to_path_buf();
        let wd = self.inotify.add_watch(&path, mask)?;

        // Update path -> wd mapping. If the path previously pointed to another
        // descriptor, remove that stale reverse mapping first.
        if let Some(old_wd) = self.wd_map.insert(path.clone(), wd.clone()) {
            self.path_map.remove(&old_wd);
        }

        // Update wd -> path mapping. If this descriptor previously pointed to a
        // different path, remove that stale forward mapping to keep maps in sync.
        if let Some(old_path) = self.path_map.insert(wd, path.clone()) {
            if old_path != path {
                self.wd_map.remove(&old_path);
            }
        }

        Ok(())
    }

    /// Remove watch by monitored path, ignoring missing paths.
    #[inline]
    fn remove_watch_path(&mut self, path: &Path) -> io::Result<()> {
        let wd = match self.wd_map.remove(path) {
            Some(wd) => wd,
            None => return Ok(()),
        };
        self.path_map.remove(&wd);

        // Try to remove watch from kernel side. `NotFound`/`InvalidInput` are
        // tolerated because the watch may have already been removed asynchronously.
        if let Err(e) = self.inotify.rm_watch(wd) {
            if !matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::InvalidInput
            ) {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Remove local watch mappings by descriptor, used for `IGNORED` events.
    #[inline]
    fn remove_watch_descriptor(&mut self, wd: &WatchDescriptor) {
        let path = match self.path_map.remove(wd) {
            Some(path) => path,
            None => return,
        };
        let _ = self.wd_map.remove(&path);
    }

    /// Resolve monitored path from watch descriptor, returns `None` if not found.
    #[inline]
    fn get_watch_path(&self, wd: &WatchDescriptor) -> Option<&Path> {
        self.path_map.get(wd).map(PathBuf::as_path)
    }
}

pub struct PatchMonitor {
    is_running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl PatchMonitor {
    #[inline]
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_manager: Arc<PatchManager>) -> Result<Self> {
        let root_dir = patch_root.as_ref().join(PATCH_INSTALL_DIR);

        let monitor = Self::initialize_monitor(&root_dir)?;
        let is_running = Arc::new(AtomicBool::new(true));

        let thread_is_running = is_running.clone();
        let thread_handle = thread::Builder::new()
            .name(MONITOR_THREAD_NAME.to_string())
            .spawn(move || {
                info!("Monitoring patch directory '{}'...", root_dir.display());
                Self::thread_main(monitor, patch_manager, thread_is_running)
            })
            .with_context(|| format!("Failed to create thread '{}'", MONITOR_THREAD_NAME))?;

        Ok(Self {
            is_running,
            handle: Some(thread_handle),
        })
    }

    /// Initialize the monitor with the root patch directory.
    #[inline]
    fn initialize_monitor(root_dir: &Path) -> io::Result<InotifyMonitor> {
        let mut monitor = InotifyMonitor::new()?;

        // Watch the root patch directory first, so newly created/renamed
        // sub-directories can be detected while we are scanning existing ones.
        monitor.add_watch_path(root_dir, ROOT_DIR_WATCH_MASK)?;

        // Register watches for already existing first-level patch directories.
        // Log unreadable entries to keep initialization failures observable.
        let entries = fs::read_dir(root_dir)?.flatten();
        for entry in entries {
            if entry.file_type().map_or(false, |f| f.is_dir()) {
                monitor.add_watch_path(entry.path(), PATCH_DIR_WATCH_MASK)?;
            }
        }

        Ok(monitor)
    }

    /// Main thread loop for inotify-based patch directory monitoring.
    #[inline]
    fn thread_main(
        mut monitor: InotifyMonitor,
        patch_manager: Arc<PatchManager>,
        is_running: Arc<AtomicBool>,
    ) {
        let mut buffer = [0; EVENT_BUFFER_CAPACITY];

        // Poll inotify events until a stop signal is observed.
        while is_running.load(Ordering::Relaxed) {
            let mut need_rescan = false;

            // Read one batch of currently available events.
            let events = match monitor.inotify.read_events(&mut buffer) {
                Ok(events) => events,
                Err(e) => {
                    let need_break = Self::handle_error(&e);
                    if need_break {
                        break;
                    }

                    thread::park_timeout(MONITOR_THREAD_TIMEOUT);
                    continue;
                }
            };

            // Process this batch and aggregate whether a rescan is required.
            for event in events {
                need_rescan |= Self::handle_event(&mut monitor, event);
            }

            // Rescan once per batch to avoid repeated expensive scans.
            if need_rescan {
                info!("Detected patch change, rescanning patches...");
                if let Err(e) = patch_manager.rescan_patches() {
                    error!("Failed to rescan patches, {}", e);
                }
            }

            // Back off before the next polling iteration.
            thread::park_timeout(MONITOR_THREAD_TIMEOUT);
        }
    }

    /// Handle inotify read errors, returns `true` if a shutdown is required.
    #[inline]
    fn handle_error(error: &io::Error) -> bool {
        match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted => false,
            io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof => {
                info!("Monitor is shutting down...");
                true
            }
            _ => {
                error!("Failed to read monitor events, {}", error);
                false
            }
        }
    }

    /// Handle inotify event, returns `true` if a rescan is required.
    #[inline]
    fn handle_event(monitor: &mut InotifyMonitor, event: Event<&OsStr>) -> bool {
        let mask = event.mask;

        // `IGNORED` means the watch is already dropped by kernel/user.
        // Keep local descriptor maps in sync and ignore this event.
        if mask.contains(EventMask::IGNORED) {
            monitor.remove_watch_descriptor(&event.wd);
            return false;
        }

        // Resolve event path from watch descriptor + child entry name.
        // Events without child names are ignored in this monitor.
        let path = {
            let parent = match monitor.get_watch_path(&event.wd) {
                Some(path) => path,
                None => {
                    error!("Cannot find event path for {:?}", event.wd);
                    return false;
                }
            };
            let name = match event.name {
                Some(name) => name,
                None => return false,
            };

            parent.join(name)
        };

        debug!("PATH: '{}', EVENT: {:#?}", path.display(), mask);

        // File events only trigger rescan when patch metadata file changed.
        if !mask.contains(EventMask::ISDIR) {
            let file_name = path.file_name().unwrap_or_default();

            let is_patch_info = file_name == PATCH_INFO_FILE_NAME;
            let is_file_change = mask.contains(EventMask::CREATE)
                || mask.contains(EventMask::DELETE)
                || mask.contains(EventMask::MOVED_TO)
                || mask.contains(EventMask::MOVED_FROM);

            return is_patch_info && is_file_change;
        }

        // When directory creates, keep sub-directory watches up to date.
        if mask.contains(EventMask::CREATE) || mask.contains(EventMask::MOVED_TO) {
            if let Err(e) = monitor.add_watch_path(&path, PATCH_DIR_WATCH_MASK) {
                error!(
                    "Failed to add monitor directory '{}', {}",
                    path.display(),
                    e
                );
            }

            // Check if patch info file is already exists.
            return path.join(PATCH_INFO_FILE_NAME).is_file();
        }

        // When directory removes, remove sub-directory watches.
        if mask.contains(EventMask::DELETE) || mask.contains(EventMask::MOVED_FROM) {
            if let Err(e) = monitor.remove_watch_path(&path) {
                error!(
                    "Failed to remove monitor directory '{}', {}",
                    path.display(),
                    e
                );
            }
        }

        false
    }
}

impl Drop for PatchMonitor {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            if let Err(e) = handle.join() {
                error!("Patch monitor thread panicked: {:?}", e);
            }
        }
    }
}
