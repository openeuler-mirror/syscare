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
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use inotify::{Inotify, WatchDescriptor, WatchMask};
use log::info;
use parking_lot::{Mutex, RwLock};
use syscare_common::ffi::OsStrExt;

const MONITOR_THREAD_NAME: &str = "upatch_monitor";
const MONITOR_CHECK_PERIOD: u64 = 100;
const MONITOR_EVENT_BUFFER_CAPACITY: usize = 16 * 64; // inotify event size: 16

pub(super) struct UserPatchMonitor {
    inotify: Arc<Mutex<Option<Inotify>>>,
    watch_wd_map: Arc<Mutex<IndexMap<PathBuf, WatchDescriptor>>>,
    watch_file_map: Arc<RwLock<IndexMap<WatchDescriptor, PathBuf>>>,
    monitor_thread: Option<thread::JoinHandle<()>>,
}

impl UserPatchMonitor {
    pub fn new<F>(callback: F) -> Result<Self>
    where
        F: Fn(Vec<&Path>) + Send + Sync + 'static,
    {
        let inotify = Arc::new(Mutex::new(Some(
            Inotify::init().context("Failed to initialize inotify")?,
        )));
        let watch_wd_map = Arc::new(Mutex::new(IndexMap::new()));
        let watch_file_map = Arc::new(RwLock::new(IndexMap::new()));
        let monitor_thread = MonitorThread {
            inotify: inotify.clone(),
            watch_file_map: watch_file_map.clone(),
            callback,
        }
        .run()?;

        Ok(Self {
            inotify,
            watch_wd_map,
            watch_file_map,
            monitor_thread: Some(monitor_thread),
        })
    }
}

impl UserPatchMonitor {
    pub fn watch_file<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let watch_file = file_path.as_ref();
        if self.watch_wd_map.lock().contains_key(watch_file) {
            return Ok(());
        }

        match self.inotify.lock().as_mut() {
            Some(inotify) => {
                let wd = inotify
                    .add_watch(watch_file, WatchMask::OPEN)
                    .map_err(|e| {
                        anyhow!(
                            "Failed to watch file '{}', {}",
                            watch_file.display(),
                            e.to_string().to_lowercase()
                        )
                    })?;

                self.watch_file_map
                    .write()
                    .insert(wd.clone(), watch_file.to_owned());
                self.watch_wd_map.lock().insert(watch_file.to_owned(), wd);
                info!("Start watching file '{}'", watch_file.display());
            }
            None => bail!("Inotify does not exist"),
        }

        Ok(())
    }

    pub fn ignore_file<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let ignore_file = file_path.as_ref();

        if let Some(wd) = self.watch_wd_map.lock().remove(ignore_file) {
            match self.inotify.lock().as_mut() {
                Some(inotify) => {
                    self.watch_file_map.write().remove(&wd);

                    inotify.rm_watch(wd).map_err(|e| {
                        anyhow!(
                            "Failed to stop watch file '{}', {}",
                            ignore_file.display(),
                            e.to_string().to_lowercase()
                        )
                    })?;
                    info!("Stop watching file '{}'", ignore_file.display());
                }
                None => bail!("Inotify does not exist"),
            }
        }

        Ok(())
    }
}

struct MonitorThread<F> {
    inotify: Arc<Mutex<Option<Inotify>>>,
    watch_file_map: Arc<RwLock<IndexMap<WatchDescriptor, PathBuf>>>,
    callback: F,
}

impl<F> MonitorThread<F>
where
    F: Fn(Vec<&Path>) + Send + Sync + 'static,
{
    fn run(self) -> Result<thread::JoinHandle<()>> {
        thread::Builder::new()
            .name(MONITOR_THREAD_NAME.to_string())
            .spawn(move || self.thread_main())
            .map_err(|e| {
                anyhow!(
                    "Failed to create thread '{}', {}",
                    MONITOR_THREAD_NAME,
                    e.to_string().to_lowercase()
                )
            })
    }

    #[inline]
    fn filter_blacklist_path(path: &Path) -> bool {
        const BLACKLIST_KEYWORDS: [&str; 2] = ["syscare", "upatch"];

        for keyword in BLACKLIST_KEYWORDS {
            if path.contains(keyword) {
                return false;
            }
        }
        true
    }

    fn thread_main(self) {
        while let Some(inotify) = self.inotify.lock().as_mut() {
            let mut buffer = [0; MONITOR_EVENT_BUFFER_CAPACITY];

            if let Ok(events) = inotify.read_events(&mut buffer) {
                let watch_file_map = self.watch_file_map.read();
                let path_list = events
                    .filter_map(|event| watch_file_map.get(&event.wd))
                    .filter(|path| Self::filter_blacklist_path(path))
                    .map(|path| path.as_ref())
                    .collect::<Vec<_>>();
                (self.callback)(path_list)
            }

            thread::park_timeout(Duration::from_millis(MONITOR_CHECK_PERIOD))
        }
    }
}

impl Drop for UserPatchMonitor {
    fn drop(&mut self) {
        if let Some(inotify) = self.inotify.lock().deref_mut().take() {
            inotify.close().ok();
        }
        if let Some(thread) = self.monitor_thread.take() {
            thread.join().ok();
        }
    }
}
