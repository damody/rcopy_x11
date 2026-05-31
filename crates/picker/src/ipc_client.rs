use std::path::PathBuf;

use anyhow::{Context, Result};
use rcopy_core::{default_socket_path, ClipboardItem};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    Search { query: String },
    Restore { id: Uuid, auto_paste: bool },
    Delete { id: Uuid },
    Pin { id: Uuid, value: bool },
    Favorite { id: Uuid, value: bool },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RestoreResponse {
    pub paste_attempted: bool,
    pub paste_succeeded: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IpcResponse<T> {
    Ok { value: T },
    Err { message: String },
}

#[derive(Clone, Debug)]
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    pub fn default_socket() -> Self {
        Self::new(default_socket_path())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<ClipboardItem>> {
        self.request(IpcRequest::Search {
            query: query.to_string(),
        })
        .await
    }

    pub async fn restore(&self, id: Uuid, auto_paste: bool) -> Result<RestoreResponse> {
        self.request(IpcRequest::Restore { id, auto_paste }).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        self.request(IpcRequest::Delete { id }).await
    }

    pub async fn pin(&self, id: Uuid, value: bool) -> Result<()> {
        self.request(IpcRequest::Pin { id, value }).await
    }

    pub async fn favorite(&self, id: Uuid, value: bool) -> Result<()> {
        self.request(IpcRequest::Favorite { id, value }).await
    }

    async fn request<T>(&self, request: IpcRequest) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| format!("connect IPC socket {}", self.socket_path.display()))?;
        let mut encoded = serde_json::to_vec(&request)?;
        encoded.push(b'\n');
        stream.write_all(&encoded).await?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        match serde_json::from_str::<IpcResponse<T>>(&line)? {
            IpcResponse::Ok { value } => Ok(value),
            IpcResponse::Err { message } => anyhow::bail!(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_request_matches_daemon_protocol() {
        let request = IpcRequest::Search {
            query: "alpha".to_string(),
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"type": "Search", "query": "alpha"})
        );
    }
}
