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

use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Error, Result};
use jsonrpc::{
    serde_json::value::RawValue,
    simple_uds::{self, UdsTransport},
    Client,
};
use log::debug;
use serde::{Deserialize, Serialize};

use syscare_common::fs::{self, FileLock};

#[derive(Debug, Default)]
pub struct RpcArguments {
    args: Vec<Box<RawValue>>,
}

impl RpcArguments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg<T: Serialize>(mut self, arg: T) -> Self {
        self.args.push(jsonrpc::arg(arg));
        self
    }
}

impl Deref for RpcArguments {
    type Target = [Box<RawValue>];

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

pub struct RpcClient {
    lock: PathBuf,
    socket: PathBuf,
    client: Client,
}

impl RpcClient {
    pub fn new(work_dir: &Path) -> Result<Self> {
        const LOCK_FILE_NAME: &str = "syscare.lock";
        const SOCKET_FILE_NAME: &str = "syscared.sock";

        ensure!(
            work_dir.is_dir(),
            "Working directory '{}' is invalid",
            work_dir.display()
        );
        let lock = work_dir.join(LOCK_FILE_NAME);
        let socket = work_dir.join(SOCKET_FILE_NAME);
        let client = Client::with_transport(UdsTransport::new(&socket));

        Ok(Self {
            lock,
            socket,
            client,
        })
    }

    pub fn lock(&self) -> Result<FileLock> {
        fs::flock(&self.lock, fs::FileLockType::Exclusive)
            .with_context(|| format!("Failed to lock {}", self.lock.display()))
    }

    pub fn call<T>(&self, cmd: &str) -> Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        self.call_with_args::<T>(cmd, RpcArguments::default())
    }

    pub fn call_with_args<T>(&self, cmd: &str, args: RpcArguments) -> Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let request = self.client.build_request(cmd, &args);
        debug!("{:#?}", request);

        let response = self
            .client
            .send_request(request)
            .map_err(|e| self.parse_request_error(e))?;
        debug!("{:#?}", response);

        response.result().map_err(|e| self.parse_response_error(e))
    }
}

impl RpcClient {
    fn parse_transport_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> Error {
        anyhow!(
            "Cannot connect to syscare daemon at unix://{}, {}",
            self.socket.display(),
            match error.downcast::<simple_uds::Error>() {
                Ok(err) => match *err {
                    simple_uds::Error::SocketError(e) => e.to_string(),
                    simple_uds::Error::Timeout => String::from("Connection timeout"),
                    simple_uds::Error::Json(_) => String::from("Invalid data format"),
                },
                Err(_) => {
                    String::from("Unknown error")
                }
            }
        )
    }

    fn parse_request_error(&self, error: jsonrpc::Error) -> Error {
        match error {
            jsonrpc::Error::Transport(err) => self.parse_transport_error(err),
            jsonrpc::Error::Json(_) => anyhow!("Request format error"),
            _ => anyhow!("Unknown request error"),
        }
    }

    fn parse_response_error(&self, error: jsonrpc::Error) -> Error {
        match error {
            jsonrpc::Error::Transport(err) => self.parse_transport_error(err),
            jsonrpc::Error::Json(_) => anyhow!("Response format error"),
            jsonrpc::Error::Rpc(e) => anyhow!(e.message),
            _ => anyhow!("Unknown response error"),
        }
    }
}
