use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{ensure, Context, Result};
use inotify::{EventMask, Inotify, WatchMask};
use log::{error, info};
use parking_lot::RwLock;

use super::{PatchManager, PATCH_INFO_FILE_NAME};

const MONITOR_THREAD_NAME: &str = "patch_monitor";
const MONITOR_CHECK_PERIOD: u64 = 300;
const PATCH_INSTALLED_WAIT_TIMEOUT: u64 = 100;
const PATCH_INSTALLED_WAIT_RETRY: usize = 10;

pub struct PatchMonitor {
    run_flag: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl PatchMonitor {
    pub fn new() -> Result<Self> {
        let patch_manager = PatchManager::get_instance()?;
        let monitor_dir = patch_manager.read().patch_install_dir.clone();

        let mut inotify = Inotify::init().context("Failed to initialize inotify")?;
        inotify
            .add_watch(
                &monitor_dir,
                WatchMask::CREATE | WatchMask::DELETE | WatchMask::ONLYDIR,
            )
            .context("Failed to monitor patch directory")?;

        let run_flag = Arc::new(AtomicBool::new(true));
        let thread_handle = PatchMonitorThread {
            monitor_dir,
            inotify,
            patch_manager,
            run_flag: run_flag.clone(),
        }
        .run()?;

        Ok(Self {
            run_flag,
            thread_handle: Some(thread_handle),
        })
    }
}

impl Drop for PatchMonitor {
    fn drop(&mut self) {
        self.run_flag.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            handle.join().ok();
        }
    }
}

struct PatchMonitorThread {
    monitor_dir: PathBuf,
    inotify: Inotify,
    patch_manager: Arc<RwLock<PatchManager>>,
    run_flag: Arc<AtomicBool>,
}

impl PatchMonitorThread {
    fn run(self) -> Result<JoinHandle<()>> {
        let thread_handle = std::thread::Builder::new()
            .name(MONITOR_THREAD_NAME.to_string())
            .spawn(|| self.monitor_thread())
            .with_context(|| format!("Failed to create {} thread", MONITOR_THREAD_NAME))?;

        Ok(thread_handle)
    }
}

impl PatchMonitorThread {
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

    fn monitor_thread(mut self) {
        let monitor_dir = self.monitor_dir;
        info!(
            "Monitoring patch directory \"{}\"...",
            monitor_dir.display()
        );

        while self.run_flag.load(Ordering::Relaxed) {
            let mut buffer = [0; 1024];
            if let Ok(events) = self.inotify.read_events(&mut buffer) {
                for event in events {
                    if let Some(patch_directory) = event.name {
                        if event.mask.contains(EventMask::CREATE) {
                            if Self::wait_patch_installed(
                                monitor_dir.join(patch_directory).join(PATCH_INFO_FILE_NAME),
                                Duration::from_millis(PATCH_INSTALLED_WAIT_TIMEOUT),
                                PATCH_INSTALLED_WAIT_RETRY,
                            )
                            .is_err()
                            {
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
}
