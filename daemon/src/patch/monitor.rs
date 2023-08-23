use std::{path::Path, sync::Arc, thread::JoinHandle, time::Duration};

use anyhow::{ensure, Context, Result};
use inotify::{EventMask, Inotify, WatchMask};
use log::{error, info};
use parking_lot::RwLock;

use super::manager::{PatchManager, PATCH_INFO_FILE_NAME};

const MONITOR_THREAD_NAME: &str = "patch_monitor";
const MONITOR_CHECK_PERIOD: u64 = 500;
const PATCH_INSTALLED_WAIT_TIMEOUT: u64 = 500;
const PATCH_INSTALLED_MAX_RETRY: usize = 10;

pub struct PatchMonitor {
    thread_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl PatchMonitor {
    fn wait_patch_installed<P: AsRef<Path>>(
        patch_info_file: P,
        timeout: Duration,
        max_retry: usize,
    ) -> Result<()> {
        let mut retry_count = 0;
        while !patch_info_file.as_ref().exists() {
            ensure!(retry_count < max_retry);
            std::thread::sleep(timeout);
            retry_count += 1;
        }
        Ok(())
    }

    fn monitor_thread(
        mut inotify: Inotify,
        patch_manager: Arc<RwLock<PatchManager>>,
        thread_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    ) {
        let patch_install_dir = patch_manager.read().patch_install_dir().to_path_buf();
        info!(
            "Monitoring patch directory \"{}\"...",
            patch_install_dir.display()
        );

        while thread_handle.read().is_some() {
            let mut buffer = [0; 1024];
            if let Ok(events) = inotify.read_events(&mut buffer) {
                for event in events {
                    if let Some(patch_directory) = event.name {
                        if event.mask.contains(EventMask::CREATE) {
                            let patch_info_file = patch_install_dir
                                .join(patch_directory)
                                .join(PATCH_INFO_FILE_NAME);
                            let wait_timeout = Duration::from_millis(PATCH_INSTALLED_WAIT_TIMEOUT);
                            if Self::wait_patch_installed(
                                patch_info_file,
                                wait_timeout,
                                PATCH_INSTALLED_MAX_RETRY,
                            )
                            .is_err()
                            {
                                error!("Waiting for patch installation timed out");
                                continue;
                            }
                        }

                        info!("Detected patch change, rescanning patches...");
                        if let Err(e) = patch_manager
                            .write()
                            .rescan_patches()
                            .context("An error occored while rescanning patches")
                        {
                            {
                                error!("{:?}", e)
                            }
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(MONITOR_CHECK_PERIOD));
        }
    }

    pub fn new(patch_manager: Arc<RwLock<PatchManager>>) -> Result<Self> {
        let mut inotify = Inotify::init().context("Failed to initialize inotify")?;
        inotify
            .add_watch(
                patch_manager.read().patch_install_dir(),
                WatchMask::CREATE | WatchMask::DELETE | WatchMask::ONLYDIR,
            )
            .context("Failed to monitor patch directory")?;

        let thread_handle = Arc::new(RwLock::new(None));
        {
            let mut write_guard = thread_handle.write();
            let patch_manager = patch_manager.clone();
            let thread_handle = thread_handle.clone();
            let new_thread_handle = std::thread::Builder::new()
                .name(MONITOR_THREAD_NAME.to_string())
                .spawn(move || Self::monitor_thread(inotify, patch_manager, thread_handle))
                .with_context(|| format!("Failed to create {} thread", MONITOR_THREAD_NAME))?;

            let _ = write_guard.insert(new_thread_handle);
        }

        Ok(Self { thread_handle })
    }
}

impl Drop for PatchMonitor {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.write().take() {
            handle.join().ok();
        }
    }
}
