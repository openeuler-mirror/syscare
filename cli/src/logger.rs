// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::ops::Deref;

use anyhow::Result;
use flexi_logger::{DeferredNow, LogSpecification, Logger as FlexiLogger, LoggerHandle, WriteMode};

use log::{LevelFilter, Record};
use once_cell::sync::OnceCell;

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
}

impl Logger {
    pub fn is_inited() -> bool {
        LOGGER.get().is_some()
    }

    pub fn initialize(max_level: LevelFilter) -> Result<()> {
        LOGGER.get_or_try_init(|| -> Result<Logger> {
            let log_spec = LogSpecification::builder().default(max_level).build();
            let handle = FlexiLogger::with(log_spec)
                .log_to_stdout()
                .format(Self::format_log)
                .write_mode(WriteMode::Direct)
                .start()?;

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
