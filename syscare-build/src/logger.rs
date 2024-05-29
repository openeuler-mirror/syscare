// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{ops::Deref, path::Path};

use anyhow::{Context, Result};
use flexi_logger::{
    DeferredNow, Duplicate, FileSpec, LogSpecification, Logger as FlexiLogger, LoggerHandle,
    WriteMode,
};

use log::{LevelFilter, Record};
use once_cell::sync::OnceCell;

const LOG_FILE_NAME: &str = "build";
static LOGGER: OnceCell<Logger> = OnceCell::new();

pub struct Logger {
    handle: LoggerHandle,
}

impl Logger {
    fn format_log(
        w: &mut dyn std::io::Write,
        _now: &mut DeferredNow,
        record: &Record,
    ) -> Result<(), std::io::Error> {
        write!(w, "{}", &record.args())
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
                .basename(LOG_FILE_NAME)
                .use_timestamp(false);

            let logger = FlexiLogger::with(log_spec)
                .log_to_file(file_spec)
                .duplicate_to_stdout(Self::stdout_duplicate(stdout_level))
                .format(Self::format_log)
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
#[cfg(test)]
use super::*;
#[test]
fn test_looger(){
    let log_dir=PathBuf::from("/var/log/kylin-warm");
    Logger::initialize(log_dir, LevelFilter::Trace,LevelFilter::Info);
    info!("test for kylin-warm-build logger!!!!!!");
    let mut out_put=fs::read_to_string("/var/log/kylin-warm/build.log").unwr    ap();
    let mut result=String::new();
    if let Some(line) = out_put.lines().next() {
        result=line.to_string().to_owned();
    }
    fs::remove_file("/var/log/kylin-warm/build.log").unwrap();
    let bool_result=result.contains("test for kylin-warm-build logger!!!!!!"    );
    assert!(bool_result);
}

