use anyhow::Result;
use rcopy_core::default_socket_path;
use rcopy_integration::{ClipboardBackend, PasteBackend};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::rc::Rc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::service::{DaemonService, ServiceError};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, PermissionsExt};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    Capture,
    Search {
        query: String,
    },
    Restore {
        id: Uuid,
        auto_paste: bool,
        #[serde(default)]
        plain_text_only: bool,
    },
    Delete {
        id: Uuid,
    },
    Pin {
        id: Uuid,
        value: bool,
    },
    Favorite {
        id: Uuid,
        value: bool,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IpcResponse<T> {
    Ok { value: T },
    Err { message: String },
}

pub async fn serve_default_socket<C, P>(service: DaemonService<C, P>) -> Result<()>
where
    C: ClipboardBackend + 'static,
    P: PasteBackend + 'static,
{
    let path = default_socket_path();
    prepare_socket_path(&path)?;
    let listener = UnixListener::bind(path)?;
    let service = Rc::new(Mutex::new(service));

    tokio::task::LocalSet::new()
        .run_until(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let service = Rc::clone(&service);
                tokio::task::spawn_local(async move {
                    if let Err(error) = handle_client(stream, service).await {
                        eprintln!("IPC client error: {error}");
                    }
                });
            }
        })
        .await
}

fn prepare_socket_path(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_private_dir(parent)?;
    }

    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => {
            std::fs::remove_file(path)?;
        }
        Ok(_) => {
            anyhow::bail!("refusing to remove non-socket IPC path: {}", path.display());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    Ok(())
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true).mode(0o700);
    builder.create(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

async fn handle_client<C, P>(
    stream: UnixStream,
    service: Rc<Mutex<DaemonService<C, P>>>,
) -> Result<()>
where
    C: ClipboardBackend,
    P: PasteBackend,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response = match serde_json::from_str::<IpcRequest>(&line) {
        Ok(request) => {
            let service = service.lock().await;
            dispatch_request(request, &service).await
        }
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
        IpcRequest::Capture => serialize_service_result(service.capture_current().await)?,
        IpcRequest::Search { query } => serialize_service_result(service.search(&query))?,
        IpcRequest::Restore {
            id,
            auto_paste,
            plain_text_only,
        } => serialize_service_result(service.restore(id, auto_paste, plain_text_only).await)?,
        IpcRequest::Delete { id } => serialize_service_result(service.soft_delete(id))?,
        IpcRequest::Pin { id, value } => serialize_service_result(service.set_pinned(id, value))?,
        IpcRequest::Favorite { id, value } => {
            serialize_service_result(service.set_favorite(id, value))?
        }
    };

    Ok(response)
}

fn serialize_service_result<T: Serialize>(result: Result<T, ServiceError>) -> Result<String> {
    match result {
        Ok(value) => Ok(serde_json::to_string(&IpcResponse::Ok { value })?),
        Err(error) => Ok(serde_json::to_string(&IpcResponse::<()>::Err {
            message: error.to_string(),
        })?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_request_deserializes_uuid_auto_paste_and_plain_text_only() {
        let id = uuid::Uuid::new_v4();
        let json =
            format!(r#"{{"type":"Restore","id":"{id}","auto_paste":true,"plain_text_only":true}}"#);

        let request: IpcRequest = serde_json::from_str(&json).unwrap();

        match request {
            IpcRequest::Restore {
                id: parsed_id,
                auto_paste,
                plain_text_only,
            } => {
                assert_eq!(parsed_id, id);
                assert!(auto_paste);
                assert!(plain_text_only);
            }
            other => panic!("unexpected request: {other:?}"),
        }
    }

    #[test]
    fn restore_request_defaults_plain_text_only_to_false() {
        let id = uuid::Uuid::new_v4();
        let json = format!(r#"{{"type":"Restore","id":"{id}","auto_paste":true}}"#);

        let request: IpcRequest = serde_json::from_str(&json).unwrap();

        match request {
            IpcRequest::Restore {
                plain_text_only, ..
            } => assert!(!plain_text_only),
            other => panic!("unexpected request: {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_returns_json_error_for_service_errors() {
        let repo = rcopy_storage::Repository::open_in_memory().unwrap();
        let clipboard = rcopy_integration::MemoryClipboard::new(rcopy_core::ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard, rcopy_integration::DisabledPasteBackend);
        let missing = Uuid::new_v4();

        let response = dispatch_request(IpcRequest::Delete { id: missing }, &service)
            .await
            .unwrap();

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response).unwrap(),
            serde_json::json!({
                "type": "Err",
                "message": format!("item not found: {missing}")
            })
        );
    }

    #[tokio::test]
    async fn prepare_socket_path_rejects_regular_files() {
        let dir = unique_temp_dir();
        let socket_path = dir.join("rcopyd.sock");
        std::fs::write(&socket_path, "not a socket").unwrap();

        assert!(prepare_socket_path(&socket_path).is_err());
        assert_eq!(
            std::fs::read_to_string(&socket_path).unwrap(),
            "not a socket"
        );
    }

    #[tokio::test]
    async fn prepare_socket_path_removes_stale_socket_files() {
        let dir = unique_temp_dir();
        let socket_path = dir.join("rcopyd.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        drop(listener);

        prepare_socket_path(&socket_path).unwrap();

        assert!(!socket_path.exists());
    }

    fn unique_temp_dir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rcopyd-test-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }
}
