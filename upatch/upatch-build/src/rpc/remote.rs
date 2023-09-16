use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use jsonrpc::{simple_uds::UdsTransport, Client, Error};
use log::debug;
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
        debug!("{:?}", request);

        let response = self
            .client
            .send_request(request)
            .map_err(|e| self.parse_error(e))?;
        debug!("{:?}", response);

        response.result().map_err(|e| self.parse_error(e))
    }
}

impl RpcRemote {
    fn parse_error(&self, error: Error) -> anyhow::Error {
        match error {
            Error::Transport(e) => {
                anyhow!(
                    "Cannot connect to syscare daemon at unix://{}, {}",
                    self.socket.display(),
                    e.source()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "Connection timeout".to_string())
                )
            }
            Error::Json(e) => {
                debug!("Json parse error: {:?}", e);
                anyhow!("Failed to parse response")
            }
            Error::Rpc(ref e) => match e.message == "Method not found" {
                true => {
                    anyhow!("Method is unimplemented")
                }
                false => {
                    anyhow!("{}", e.message)
                }
            },
            _ => {
                debug!("{:?}", error);
                anyhow!("Response is invalid")
            }
        }
    }
}
