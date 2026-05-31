use anyhow::Result;
use rcopy_integration::{ClipboardBackend, PasteBackend};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use uuid::Uuid;

use crate::service::DaemonService;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    Search { query: String },
    Restore { id: Uuid, auto_paste: bool },
    Delete { id: Uuid },
    Pin { id: Uuid, value: bool },
    Favorite { id: Uuid, value: bool },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IpcResponse<T> {
    Ok { value: T },
    Err { message: String },
}

pub async fn serve_default_socket<C, P>(service: DaemonService<C, P>) -> Result<()>
where
    C: ClipboardBackend,
    P: PasteBackend,
{
    let path = "/tmp/rcopyd.sock";
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        handle_client(stream, &service).await?;
    }
}

async fn handle_client<C, P>(stream: UnixStream, service: &DaemonService<C, P>) -> Result<()>
where
    C: ClipboardBackend,
    P: PasteBackend,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response = match serde_json::from_str::<IpcRequest>(&line) {
        Ok(request) => dispatch_request(request, service).await,
        Err(error) => Ok(serde_json::to_string(&IpcResponse::<()>::Err {
            message: error.to_string(),
        })?),
    }?;

    let stream = reader.get_mut();
    stream.write_all(response.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    Ok(())
}

async fn dispatch_request<C, P>(
    request: IpcRequest,
    service: &DaemonService<C, P>,
) -> Result<String>
where
    C: ClipboardBackend,
    P: PasteBackend,
{
    let response = match request {
        IpcRequest::Search { query } => serde_json::to_string(&IpcResponse::Ok {
            value: service.search(&query)?,
        })?,
        IpcRequest::Restore { id, auto_paste } => serde_json::to_string(&IpcResponse::Ok {
            value: service.restore(id, auto_paste).await?,
        })?,
        IpcRequest::Delete { id } => {
            service.soft_delete(id)?;
            serde_json::to_string(&IpcResponse::Ok { value: () })?
        }
        IpcRequest::Pin { id, value } => {
            service.set_pinned(id, value)?;
            serde_json::to_string(&IpcResponse::Ok { value: () })?
        }
        IpcRequest::Favorite { id, value } => {
            service.set_favorite(id, value)?;
            serde_json::to_string(&IpcResponse::Ok { value: () })?
        }
    };

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_request_deserializes_uuid_and_auto_paste() {
        let id = uuid::Uuid::new_v4();
        let json = format!(r#"{{"type":"Restore","id":"{id}","auto_paste":true}}"#);

        let request: IpcRequest = serde_json::from_str(&json).unwrap();

        match request {
            IpcRequest::Restore {
                id: parsed_id,
                auto_paste,
            } => {
                assert_eq!(parsed_id, id);
                assert!(auto_paste);
            }
            other => panic!("unexpected request: {other:?}"),
        }
    }
}
