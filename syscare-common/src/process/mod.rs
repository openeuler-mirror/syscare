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

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::Path,
    process,
};

use anyhow::{Context, Result};
use log::{trace, Level};

mod child;
mod output;

#[derive(Debug, Clone)]
pub struct CommandArgs(Vec<OsString>);

impl CommandArgs {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn arg<S>(&mut self, arg: S) -> &mut Self
    where
        S: AsRef<OsStr>,
    {
        self.0.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }
}

impl Default for CommandArgs {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for CommandArgs {
    type Item = OsString;

    type IntoIter = std::vec::IntoIter<OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct CommandEnvs(HashMap<OsString, OsString>);

impl CommandEnvs {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.0
            .insert(key.as_ref().to_os_string(), value.as_ref().to_os_string());
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (key, value) in vars {
            self.env(key, value);
        }
        self
    }
}

impl Default for CommandEnvs {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for CommandEnvs {
    type Item = (OsString, OsString);

    type IntoIter = std::collections::hash_map::IntoIter<OsString, OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct Command {
    inner: process::Command,
    log_level: output::LogLevel,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            inner: process::Command::new(program),
            log_level: output::LogLevel::default(),
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg.as_ref());
        }
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, val);
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (key, val) in vars {
            self.env(key, val);
        }
        self
    }

    pub fn env_clear(&mut self) -> &mut Self {
        self.inner.env_clear();
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn stdin<T: Into<process::Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.inner.stdin(cfg);
        self
    }

    pub fn stdout(&mut self, level: Level) -> &mut Self {
        self.log_level.stdout = Some(level);
        self
    }

    pub fn stderr(&mut self, level: Level) -> &mut Self {
        self.log_level.stderr = Some(level);
        self
    }

    pub fn pipe_output(&mut self) -> &mut Self {
        self.inner
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped());
        self
    }

    pub fn ignore_output(&mut self) -> &mut Self {
        self.inner
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null());
        self
    }

    pub fn spawn(&mut self) -> Result<child::Child> {
        let name = Path::new(self.inner.get_program())
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        trace!("Executing {:?}", self.inner);
        let child = self
            .inner
            .spawn()
            .with_context(|| format!("Failed to start {}", name))?;
        let log_level = self.log_level;

        Ok(child::Child {
            name,
            child,
            log_level,
        })
    }

    pub fn run(&mut self) -> Result<child::ExitStatus> {
        self.ignore_output().spawn()?.wait()
    }

    pub fn run_with_output(&mut self) -> Result<child::Output> {
        self.pipe_output().spawn()?.wait_with_output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;
    use std::ffi::OsStr;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn test_command_args_new() {
        let args = CommandArgs::new();
        assert!(args.0.is_empty());
    }

    #[test]
    fn test_command_args_default() {
        let args = CommandArgs::default();
        assert!(args.0.is_empty());
    }

    #[test]
    fn test_command_args_arg() {
        let mut args = CommandArgs::new();
        args.arg("test");
        assert_eq!(args.0.len(), 1);
        assert_eq!(args.0[0], OsStr::new("test"));
    }

    #[test]
    fn test_command_args_args() {
        let mut args = CommandArgs::new();
        args.args(vec!["arg1", "arg2"]);
        assert_eq!(args.0.len(), 2);
        assert_eq!(args.0[0], OsStr::new("arg1"));
        assert_eq!(args.0[1], OsStr::new("arg2"));
    }

    #[test]
    fn test_command_args_into_iter() {
        let mut args = CommandArgs::new();
        args.args(vec!["a", "b", "c"]);
        let mut iter = args.into_iter();
        assert_eq!(iter.next(), Some(OsString::from("a")));
        assert_eq!(iter.next(), Some(OsString::from("b")));
        assert_eq!(iter.next(), Some(OsString::from("c")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_command_envs_new() {
        let envs = CommandEnvs::new();
        assert!(envs.0.is_empty());
    }

    #[test]
    fn test_command_envs_default() {
        let envs = CommandEnvs::default();
        assert!(envs.0.is_empty());
    }

    #[test]
    fn test_command_envs_env() {
        let mut envs = CommandEnvs::new();
        envs.env("KEY", "VALUE");
        assert_eq!(envs.0.len(), 1);
        assert_eq!(
            envs.0.get(&OsString::from("KEY")),
            Some(&OsString::from("VALUE"))
        );
    }

    #[test]
    fn test_command_envs_envs() {
        let mut envs = CommandEnvs::new();
        envs.envs([("K1", "V1"), ("K2", "V2")]);
        assert_eq!(envs.0.len(), 2);
        assert_eq!(
            envs.0.get(&OsString::from("K1")),
            Some(&OsString::from("V1"))
        );
        assert_eq!(
            envs.0.get(&OsString::from("K2")),
            Some(&OsString::from("V2"))
        );
    }

    #[test]
    fn test_command_new() {
        let cmd = Command::new("test");
        assert_eq!(cmd.inner.get_program(), OsStr::new("test"));
        assert!(cmd.inner.get_args().next().is_none());
    }

    #[test]
    fn test_command_arg() {
        let mut cmd = Command::new("test");
        cmd.arg("arg1");
        let mut args = cmd.inner.get_args();
        assert_eq!(args.next(), Some(OsStr::new("arg1")));
        assert!(args.next().is_none());
    }

    #[test]
    fn test_command_args() {
        let mut cmd = Command::new("test");
        cmd.args(["a", "b"]);
        let mut args = cmd.inner.get_args();
        assert_eq!(args.next(), Some(OsStr::new("a")));
        assert_eq!(args.next(), Some(OsStr::new("b")));
        assert!(args.next().is_none());
    }

    #[test]
    fn test_command_env() {
        let mut cmd = Command::new("test");
        cmd.env("K", "V");
        let envs = cmd.inner.get_envs().collect::<HashMap<_, _>>();
        assert_eq!(envs.get(&OsStr::new("K")), Some(&Some(OsStr::new("V"))));
    }

    #[test]
    fn test_command_envs() {
        let mut cmd = Command::new("test");
        cmd.envs([("K1", "V1"), ("K2", "V2")]);

        let envs = cmd.inner.get_envs().collect::<HashMap<_, _>>();
        assert_eq!(envs.len(), 2);
        assert_eq!(envs.get(&OsStr::new("K1")), Some(&Some(OsStr::new("V1"))));
        assert_eq!(envs.get(&OsStr::new("K2")), Some(&Some(OsStr::new("V2"))));
    }

    #[test]
    fn test_command_env_clear() {
        let mut cmd = Command::new("test");
        cmd.env("K", "V").env_clear();
        let envs = cmd.inner.get_envs().collect::<Vec<_>>();
        assert!(envs.is_empty());
    }

    #[test]
    fn test_command_current_dir() {
        let mut cmd = Command::new("test");
        cmd.current_dir("/tmp");
        assert_eq!(cmd.inner.get_current_dir(), Some(Path::new("/tmp")));
    }

    #[test]
    fn test_command_stdout_stderr() {
        let mut cmd = Command::new("test");
        cmd.stdout(Level::Info).stderr(Level::Error);
        assert_eq!(cmd.log_level.stdout, Some(Level::Info));
        assert_eq!(cmd.log_level.stderr, Some(Level::Error));
    }

    #[test]
    fn test_command_pipe_output() {
        let mut cmd = Command::new("test");

        cmd.pipe_output();
        assert!(cmd.inner.spawn().unwrap().stdout.is_some());
        assert!(cmd.inner.spawn().unwrap().stderr.is_some());
    }

    #[test]
    fn test_command_ignore_output() {
        let mut cmd = Command::new("test");

        cmd.pipe_output();
        assert!(cmd.inner.spawn().unwrap().stdout.is_some());
        assert!(cmd.inner.spawn().unwrap().stderr.is_some());
    }

    #[test]
    fn test_command_spawn_kill() {
        let mut cmd = Command::new("yes");
        cmd.ignore_output();

        let mut child = cmd.spawn().unwrap();
        std::thread::sleep(Duration::from_millis(100));

        assert!(child.kill().is_ok());
    }

    #[test]
    fn test_command_spawn_wait() {
        let mut cmd = Command::new("echo");
        cmd.arg("test").ignore_output();

        let mut child = cmd.spawn().unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_command_run() {
        let mut cmd = Command::new("true");
        let status = cmd.run().unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_command_run_with_output() {
        let mut cmd = Command::new("env");
        let output = cmd.run_with_output().unwrap();

        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
    }
}
