// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{ops::Deref, path::Path, thread::Thread};

use anyhow::{Context, Result};
use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification,
    Logger as FlexiLogger, LoggerHandle, Naming, WriteMode,
};

use lazy_static::lazy_static;
use log::{LevelFilter, Record};
use once_cell::sync::OnceCell;

use syscare_common::os;

const MAIN_THREAD_NAME: &str = "main";
const UNNAMED_THREAD_NAME: &str = "<unnamed>";

lazy_static! {
    static ref PROCESS_NAME: &'static str =
        os::process::name().to_str().unwrap_or(UNNAMED_THREAD_NAME);
}

static LOGGER: OnceCell<Logger> = OnceCell::new();

pub struct Logger {
    handle: LoggerHandle,
}

impl Logger {
    fn thread_name(thread: &Thread) -> &str {
        match thread.name() {
            Some(MAIN_THREAD_NAME) => &PROCESS_NAME,
            Some(thread_name) => thread_name,
            None => UNNAMED_THREAD_NAME,
        }
    }

    fn format_log(
        w: &mut dyn std::io::Write,
        now: &mut DeferredNow,
        record: &Record,
    ) -> Result<(), std::io::Error> {
        const LOG_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f";

        write!(
            w,
            "[{}] [{}] [{}] {}",
            now.format(LOG_FORMAT),
            record.level(),
            Self::thread_name(&std::thread::current()),
            &record.args()
        )?;

        Ok(())
    }

    fn stdout_duplicate(stdout_level: LevelFilter) -> Duplicate {
        match stdout_level {
            LevelFilter::Off => Duplicate::None,
            LevelFilter::Error => Duplicate::Error,
            LevelFilter::Warn => Duplicate::Warn,
            LevelFilter::Info => Duplicate::Info,
            LevelFilter::Debug => Duplicate::Debug,
            LevelFilter::Trace => Duplicate::Trace,
        }
    }
}

impl Logger {
    pub fn is_inited() -> bool {
        LOGGER.get().is_some()
    }

    pub fn initialize<P: AsRef<Path>>(
        log_dir: P,
        max_level: LevelFilter,
        stdout_level: LevelFilter,
    ) -> Result<()> {
        LOGGER.get_or_try_init(|| -> Result<Logger> {
            let log_spec = LogSpecification::builder().default(max_level).build();
            let file_spec = FileSpec::default()
                .directory(log_dir.as_ref())
                .use_timestamp(false);

            let logger = FlexiLogger::with(log_spec)
                .log_to_file(file_spec)
                .format(Self::format_log)
                .duplicate_to_stdout(Self::stdout_duplicate(stdout_level))
                .rotate(
                    Criterion::Age(Age::Day),
                    Naming::Timestamps,
                    Cleanup::KeepCompressedFiles(30),
                )
                .write_mode(WriteMode::Direct);

            let handle = logger.start().context("Failed to start logger")?;

            Ok(Self { handle })
        })?;

        Ok(())
    }
}

impl Deref for Logger {
    type Target = LoggerHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
