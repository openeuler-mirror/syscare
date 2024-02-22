// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::process;

mod cmd;
mod dwarf;
mod elf;
mod log;
mod rpc;
mod tool;
mod upatch;

use upatch::UpatchBuild;

fn main() {
    let exit_code = match UpatchBuild::start_and_run() {
        Ok(_) => {
            println!("SUCCESS!");
            0
        }
        Err(e) => {
            match log::Logger::is_inited() {
                true => log::error!("{}", e),
                false => eprintln!("Error: {}", e),
            };
            e.code()
        }
    };
    process::exit(exit_code);
}
