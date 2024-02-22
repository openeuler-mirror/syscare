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

use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let dir_path = path.as_ref();
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .with_context(|| format!("Failed to create directory \"{}\"", dir_path.display()))?;
    }
    Ok(())
}

pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let dir_path = path.as_ref();
    if dir_path.exists() {
        fs::remove_dir_all(dir_path)
            .with_context(|| format!("Failed to remove directory \"{}\"", dir_path.display()))?;
    }
    Ok(())
}
