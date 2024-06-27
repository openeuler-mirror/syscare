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

use anyhow::{ensure, Context, Result};
use inotify::{EventMask, Inotify, WatchMask};
use log::{error, info};
use parking_lot::{Mutex, RwLock};

use super::{manager::PatchManager, PATCH_INFO_FILE_NAME, PATCH_INSTALL_DIR};

const MONITOR_THREAD_NAME: &str = "patch_monitor";
const MONITOR_CHECK_PERIOD: u64 = 300;
const MONITOR_EVENT_BUFFER_CAPACITY: usize = 16 * 64; // inotify event size: 16

const PATCH_INSTALLED_WAIT_TIMEOUT: u64 = 100;
const PATCH_INSTALLED_WAIT_RETRY: usize = 10;

pub struct PatchMonitor {
    inotify: Arc<Mutex<Option<Inotify>>>,
    montor_thread: Option<thread::JoinHandle<()>>,
}

impl PatchMonitor {
    pub fn new<P: AsRef<Path>>(
        patch_root: P,
        patch_manager: Arc<RwLock<PatchManager>>,
    ) -> Result<Self> {
        let patch_install_dir = patch_root.as_ref().join(PATCH_INSTALL_DIR);

        let inotify = Arc::new(Mutex::new(Some({
            let mut inotify = Inotify::init().context("Failed to initialize inotify")?;
            inotify
                .add_watch(
                    &patch_install_dir,
                    WatchMask::CREATE | WatchMask::DELETE | WatchMask::ONLYDIR,
                )
                .context("Failed to monitor patch directory")?;

            inotify
        })));

        let monitor_thread = MonitorThread {
            patch_root: patch_install_dir,
            inotify: inotify.clone(),
            patch_manager,
        }
        .run()?;

        Ok(Self {
            inotify,
            montor_thread: Some(monitor_thread),
        })
    }
}

impl Drop for PatchMonitor {
    fn drop(&mut self) {
        if let Some(inotify) = self.inotify.lock().deref_mut().take() {
            inotify.close().ok();
        }
        if let Some(thread) = self.montor_thread.take() {
            thread.join().ok();
        }
    }
}

struct MonitorThread {
    patch_root: PathBuf,
    inotify: Arc<Mutex<Option<Inotify>>>,
    patch_manager: Arc<RwLock<PatchManager>>,
}

impl MonitorThread {
    fn run(self) -> Result<thread::JoinHandle<()>> {
        let thread_handle = std::thread::Builder::new()
            .name(MONITOR_THREAD_NAME.to_string())
            .spawn(|| self.thread_main())
            .with_context(|| format!("Failed to create thread '{}'", MONITOR_THREAD_NAME))?;

        Ok(thread_handle)
    }

    fn wait_patch_installed<P: AsRef<Path>>(patch_info_file: P) -> Result<()> {
        let mut retry_count = 0;

        let timeout = Duration::from_millis(PATCH_INSTALLED_WAIT_TIMEOUT);
        while !patch_info_file.as_ref().exists() {
            ensure!(retry_count < PATCH_INSTALLED_WAIT_RETRY);
            std::thread::sleep(timeout);
            retry_count += 1;
        }

        Ok(())
    }

    fn thread_main(self) {
        let patch_root = self.patch_root.as_path();
        info!("Monitoring patch directory {}...", patch_root.display());

        while let Some(inotify) = self.inotify.lock().as_mut() {
            let mut buffer = [0; MONITOR_EVENT_BUFFER_CAPACITY];

            if let Ok(events) = inotify.read_events(&mut buffer) {
                for event in events {
                    if let Some(event_path) = event.name {
                        if event.mask.contains(EventMask::CREATE) {
                            let patch_info_file =
                                patch_root.join(event_path).join(PATCH_INFO_FILE_NAME);
                            if Self::wait_patch_installed(patch_info_file).is_err() {
                                error!("Waiting for patch installation timed out");
                                continue;
                            }
                        }

                        info!("Detected patch change, rescanning patches...");
                        if let Err(e) = self
                            .patch_manager
                            .write()
                            .rescan_patches()
                            .context("An error occored while rescanning patches")
                        {
                            error!("{:?}", e)
                        }
                    }
                }
            }

            std::thread::park_timeout(Duration::from_millis(MONITOR_CHECK_PERIOD));
        }
    }
}
