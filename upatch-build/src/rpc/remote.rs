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

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use jsonrpc::{simple_uds::UdsTransport, Client, Error};
use log::{debug, trace};
use serde::Deserialize;

use super::args::RpcArguments;

pub struct RpcRemote {
    socket: PathBuf,
    client: Client,
}

impl RpcRemote {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            socket: file_path.as_ref().to_path_buf(),
            client: Client::with_transport(UdsTransport::new(file_path)),
        }
    }

    pub fn call_with_args<T>(&self, cmd: &str, args: RpcArguments) -> Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let request = self.client.build_request(cmd, &args);
        trace!("{:?}", request);

        let response = self
            .client
            .send_request(request)
            .map_err(|e| self.parse_error(e))?;
        trace!("{:?}", response);

        response.result().map_err(|e| self.parse_error(e))
    }
}

impl RpcRemote {
    fn parse_error(&self, error: Error) -> anyhow::Error {
        match error {
            Error::Transport(err) => {
                anyhow!(
                    "Cannot connect to upatch daemon at unix://{}, {}",
                    self.socket.display(),
                    err.source()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "Connection timeout".to_string())
                )
            }
            Error::Json(err) => {
                debug!("Json parse error: {:?}", err);
                anyhow!("Failed to parse response")
            }
            Error::Rpc(err) => {
                anyhow!("{}", err.message)
            }
            _ => {
                debug!("{:?}", error);
                anyhow!("Response is invalid")
            }
        }
    }
}
