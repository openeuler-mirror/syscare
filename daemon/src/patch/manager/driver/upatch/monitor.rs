use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use log::{error, info};
use parking_lot::Mutex;

const MONITOR_THREAD_NAME: &str = "upatch_monitor";
const MONITOR_CHECK_PERIOD: u64 = 300;

pub struct UserPatchMonitor {
    inotify: Arc<Mutex<Inotify>>,
    file_map: Arc<Mutex<IndexMap<PathBuf, WatchDescriptor>>>,
    wd_map: Arc<Mutex<IndexMap<WatchDescriptor, PathBuf>>>,
    run_flag: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl UserPatchMonitor {
    pub fn new<F>(callback: F) -> Result<Self>
    where
        F: Fn(&Path) -> Result<()> + Send + Sync + 'static,
    {
        let inotify = Arc::new(Mutex::new(
            Inotify::init().context("Failed to initialize inotify")?,
        ));
        let file_map = Arc::new(Mutex::new(IndexMap::new()));
        let wd_map = Arc::new(Mutex::new(IndexMap::new()));
        let run_flag = Arc::new(AtomicBool::new(true));

        let thread_run_flag = run_flag.clone();
        let thread_inotify = inotify.clone();
        let thread_wd_map = wd_map.clone();
        let thread_handle = Some(
            std::thread::Builder::new()
                .name(MONITOR_THREAD_NAME.to_owned())
                .spawn(move || {
                    Self::monitor_thread(thread_run_flag, thread_inotify, thread_wd_map, callback)
                })
                .with_context(|| format!("Failed to create thread \"{}\"", MONITOR_THREAD_NAME))?,
        );

        Ok(Self {
            inotify,
            file_map,
            wd_map,
            run_flag,
            thread_handle,
        })
    }
}

/* Monitor manage */
impl UserPatchMonitor {
    pub fn watch_file<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        let mut file_map = self.file_map.lock();
        let mut wd_map = self.wd_map.lock();

        let file_path = file.as_ref();
        if file_map.contains_key(file_path) {
            return Ok(());
        }
        info!("Start watching file \"{}\"", file_path.display());

        let wd = self
            .inotify
            .lock()
            .add_watch(file_path, WatchMask::OPEN)
            .with_context(|| format!("Failed to watch file \"{}\"", file_path.display()))?;

        wd_map.insert(wd.clone(), file_path.to_owned());
        file_map.insert(file_path.to_owned(), wd);

        Ok(())
    }

    pub fn remove_file<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        let mut file_map = self.file_map.lock();
        let mut wd_map = self.wd_map.lock();

        let file_path = file.as_ref();
        if !file_map.contains_key(file_path) {
            return Ok(());
        }
        info!("Stop watching file \"{}\"", file_path.display());

        if let Some(wd) = file_map.remove(file_path) {
            wd_map.remove(&wd);
            self.inotify
                .lock()
                .rm_watch(wd)
                .with_context(|| format!("Failed to stop watch file \"{}\"", file_path.display()))?
        }

        Ok(())
    }
}

/* Monitor thread */
impl UserPatchMonitor {
    fn monitor_thread<F>(
        run_flag: Arc<AtomicBool>,
        inotify: Arc<Mutex<Inotify>>,
        wd_map: Arc<Mutex<IndexMap<WatchDescriptor, PathBuf>>>,
        callback: F,
    ) where
        F: Fn(&Path) -> Result<()> + Send + Sync,
    {
        while run_flag.load(Ordering::Relaxed) {
            let mut buffer = [0; 1024];

            if let Ok(events) = inotify.lock().read_events(&mut buffer) {
                for event in events {
                    if !event.mask.contains(EventMask::OPEN) {
                        continue;
                    }
                    if let Some(file) = wd_map.lock().get(&event.wd) {
                        if let Err(e) = callback(file) {
                            error!("{:?}", e);
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(MONITOR_CHECK_PERIOD));
        }
    }
}

impl Drop for UserPatchMonitor {
    fn drop(&mut self) {
        self.run_flag.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            handle.join().ok();
        }
    }
}
