// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

mod args;
mod child;
mod command;
mod envs;
mod stdio;

pub use args::*;
pub use child::*;
pub use command::*;
pub use envs::*;

use stdio::{Stdio, StdioLevel};

#[test]
fn test() {
    use log::Level;
    use std::fs::File;

    use crate::ffi::OsStrExt;

    println!("Testing Command::new()...");
    let mut echo_cmd = Command::new("echo");
    let mut env_cmd = Command::new("env");
    let mut pwd_cmd = Command::new("pwd");
    let mut grep_cmd = Command::new("grep");
    let mut ls_cmd = Command::new("ls");

    let mut test_cmd = Command::new("test");
    let mut cat_cmd = Command::new("cat");

    let mut err_cmd = Command::new("/cmd/not/exist");

    println!("Testing Command::arg()...");
    echo_cmd.arg("Test:");

    test_cmd.arg("1");
    ls_cmd.arg("/file/not/exist");

    println!("Testing Command::args()...");
    echo_cmd.args(["Hello", "World!"]);

    println!("Testing Command::env_clear()...");
    env_cmd.env_clear();

    println!("Testing Command::env()...");
    env_cmd.env("test_key1", "test_val1");

    println!("Testing Command::envs()...");
    env_cmd.envs([("test_key2", "test_val2"), ("test_key3", "test_val3")]);

    println!("Testing Command::current_dir()...");
    pwd_cmd.current_dir("/tmp");

    println!("Testing Command::stdin()...");
    grep_cmd.stdin(File::open("/proc/self/maps").expect("Failed to open file"));
    grep_cmd.arg("vdso");

    println!("Testing Command::stdout()...");
    echo_cmd.stdout(Level::Info);

    println!("Testing Command::stderr()...");
    echo_cmd.stderr(Level::Info);

    println!("Testing Command::spawn()...");
    let mut echo_proc = echo_cmd.spawn().expect("Failed to spawn process");
    let mut env_proc = env_cmd.spawn().expect("Failed to spawn process");
    let mut pwd_proc = pwd_cmd.spawn().expect("Failed to spawn process");
    let mut grep_proc = grep_cmd.spawn().expect("Failed to spawn process");
    let mut ls_proc = ls_cmd.spawn().expect("Failed to spawn process");

    let mut test_proc = test_cmd.spawn().expect("Failed to spawn process");
    let mut cat_proc = cat_cmd.spawn().expect("Failed to spawn process");

    assert_eq!(err_cmd.spawn().is_err(), true);

    println!("Testing Child::kill()...");
    cat_proc.kill().expect("Failed to kill process");

    println!("Testing Child::wait()...");
    let test_status = test_proc.wait().expect("Failed to wait process");
    let cat_status = cat_proc.wait().expect("Process should not be waited");

    println!("Testing Child::wait_with_output()...");
    let echo_output = echo_proc
        .wait_with_output()
        .expect("Failed to wait process");
    let env_output = env_proc.wait_with_output().expect("Failed to wait process");
    let pwd_output = pwd_proc.wait_with_output().expect("Failed to wait process");
    let grep_output = grep_proc
        .wait_with_output()
        .expect("Failed to wait process");
    let ls_output = ls_proc.wait_with_output().expect("Failed to wait process");

    println!("Testing ExitStatus::exit_code()...");
    assert_eq!(test_status.exit_code(), 0);
    assert_eq!(cat_status.exit_code(), 137);

    println!("Testing ExitStatus::exit_ok()...");
    assert_eq!(test_status.exit_ok().is_ok(), true);
    assert_eq!(cat_status.exit_ok().is_ok(), false);

    println!("Testing ExitStatus::success()...");
    assert_eq!(test_status.success(), true);
    assert_eq!(cat_status.success(), false);

    println!("Testing Output::exit_code()...");
    assert_eq!(echo_output.exit_code(), 0);
    assert_eq!(env_output.exit_code(), 0);
    assert_eq!(pwd_output.exit_code(), 0);
    assert_eq!(grep_output.exit_code(), 0);
    assert_eq!(ls_output.exit_code(), 2);

    println!("Testing Output::exit_ok()...");
    assert_eq!(echo_output.exit_ok().is_ok(), true);
    assert_eq!(env_output.exit_ok().is_ok(), true);
    assert_eq!(pwd_output.exit_ok().is_ok(), true);
    assert_eq!(grep_output.exit_ok().is_ok(), true);
    assert_eq!(ls_output.exit_ok().is_ok(), false);

    println!("Testing Output::success()...");
    assert_eq!(echo_output.success(), true);
    assert_eq!(env_output.success(), true);
    assert_eq!(pwd_output.success(), true);
    assert_eq!(grep_output.success(), true);
    assert_eq!(ls_output.success(), false);

    println!("Testing Output::stdout...");
    assert_eq!(echo_output.stdout.is_empty(), false);
    assert_eq!(env_output.stdout.is_empty(), false);
    assert_eq!(pwd_output.stdout.is_empty(), false);
    assert_eq!(grep_output.stdout.is_empty(), false);
    assert_eq!(ls_output.stdout.is_empty(), true);

    assert_eq!(echo_output.stdout, "Test: Hello World!");
    assert_eq!(
        env_output.stdout,
        "test_key1=test_val1\ntest_key2=test_val2\ntest_key3=test_val3"
    );
    assert_eq!(pwd_output.stdout, "/tmp");
    assert_eq!(grep_output.stdout.contains("vdso"), true);

    println!("Testing ProcessOutput::stderr...");
    assert_eq!(echo_output.stderr.is_empty(), true);
    assert_eq!(env_output.stderr.is_empty(), true);
    assert_eq!(pwd_output.stderr.is_empty(), true);
    assert_eq!(grep_output.stderr.is_empty(), true);
    assert_eq!(ls_output.stderr.is_empty(), false);
}
