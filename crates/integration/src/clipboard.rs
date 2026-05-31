use async_trait::async_trait;
use rcopy_core::ClipboardPayload;
use std::ffi::OsStr;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("clipboard command failed: {0}")]
    Command(String),
    #[error("clipboard state unavailable")]
    StateUnavailable,
}

#[async_trait]
pub trait ClipboardBackend: Send + Sync {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError>;
    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError>;
}

#[derive(Clone)]
pub struct MemoryClipboard {
    current: Arc<Mutex<ClipboardPayload>>,
    written: Arc<Mutex<Option<ClipboardPayload>>>,
}

impl MemoryClipboard {
    pub fn new(payload: ClipboardPayload) -> Self {
        Self {
            current: Arc::new(Mutex::new(payload)),
            written: Arc::new(Mutex::new(None)),
        }
    }

    pub fn last_written(&self) -> Result<ClipboardPayload, ClipboardError> {
        self.written
            .lock()
            .map_err(|_| ClipboardError::StateUnavailable)?
            .clone()
            .ok_or(ClipboardError::StateUnavailable)
    }
}

#[async_trait]
impl ClipboardBackend for MemoryClipboard {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError> {
        self.current
            .lock()
            .map_err(|_| ClipboardError::StateUnavailable)
            .map(|payload| payload.clone())
    }

    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError> {
        *self
            .written
            .lock()
            .map_err(|_| ClipboardError::StateUnavailable)? = Some(payload.clone());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct WlClipboard {
    paste_command: String,
    copy_command: String,
}

impl Default for WlClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl WlClipboard {
    pub fn new() -> Self {
        Self::with_commands("wl-paste", "wl-copy")
    }

    pub fn with_commands(
        paste_command: impl Into<String>,
        copy_command: impl Into<String>,
    ) -> Self {
        Self {
            paste_command: paste_command.into(),
            copy_command: copy_command.into(),
        }
    }
}

#[async_trait]
impl ClipboardBackend for WlClipboard {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError> {
        let text_plain = read_mime(&self.paste_command, "text/plain").await.ok();
        let text_html = read_mime(&self.paste_command, "text/html").await.ok();
        let image_png = read_bytes(&self.paste_command, "image/png").await.ok();

        Ok(ClipboardPayload {
            text_plain,
            text_html,
            image_png,
        })
    }

    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError> {
        if let Some(html) = &payload.text_html {
            write_mime(&self.copy_command, "text/html", html.as_bytes()).await?;
        }
        if let Some(image) = &payload.image_png {
            write_mime(&self.copy_command, "image/png", image).await?;
        }
        if let Some(text) = &payload.text_plain {
            write_mime(&self.copy_command, "text/plain", text.as_bytes()).await?;
        }
        Ok(())
    }
}

async fn read_mime(command: impl AsRef<OsStr>, mime: &str) -> Result<String, ClipboardError> {
    let output = Command::new(command)
        .args(["--no-newline", "--type", mime])
        .output()
        .await
        .map_err(|error| ClipboardError::Command(error.to_string()))?;

    if !output.status.success() {
        return Err(ClipboardError::Command(format!(
            "wl-paste failed for {mime}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    String::from_utf8(output.stdout).map_err(|error| ClipboardError::Command(error.to_string()))
}

async fn read_bytes(command: impl AsRef<OsStr>, mime: &str) -> Result<Vec<u8>, ClipboardError> {
    let output = Command::new(command)
        .args(["--type", mime])
        .output()
        .await
        .map_err(|error| ClipboardError::Command(error.to_string()))?;

    if !output.status.success() {
        return Err(ClipboardError::Command(format!(
            "wl-paste failed for {mime}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(output.stdout)
}

async fn write_mime(
    command: impl AsRef<OsStr>,
    mime: &str,
    bytes: &[u8],
) -> Result<(), ClipboardError> {
    let mut child = Command::new(command)
        .args(["--type", mime])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| ClipboardError::Command(error.to_string()))?;

    child
        .stdin
        .as_mut()
        .ok_or_else(|| ClipboardError::Command("wl-copy stdin unavailable".into()))?
        .write_all(bytes)
        .await
        .map_err(|error| ClipboardError::Command(error.to_string()))?;

    let status = child
        .wait()
        .await
        .map_err(|error| ClipboardError::Command(error.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(ClipboardError::Command(format!(
            "wl-copy failed for {mime} with {status}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn mock_clipboard_round_trips_payload() {
        let payload = ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: Some(vec![137, 80, 78, 71]),
        };
        let clipboard = MemoryClipboard::new(payload.clone());

        assert_eq!(clipboard.read_supported().await.unwrap(), payload);
        clipboard.write_payload(&payload).await.unwrap();
        assert_eq!(clipboard.last_written().unwrap(), payload);
    }

    #[tokio::test]
    async fn wl_clipboard_reads_supported_mimes_from_configured_command() {
        let temp = temp_dir();
        let wl_paste = write_executable(
            &temp,
            "wl-paste",
            r#"#!/bin/sh
if [ "$1" = "--no-newline" ]; then
  mime="$3"
else
  mime="$2"
fi
case "$mime" in
  text/plain) printf 'alpha' ;;
  text/html) printf '<b>alpha</b>' ;;
  image/png) printf '\211PNG' ;;
  *) exit 1 ;;
esac
"#,
        );
        let wl_copy = write_executable(&temp, "wl-copy", "#!/bin/sh\ncat >/dev/null\n");
        let clipboard = WlClipboard::with_commands(
            wl_paste.to_string_lossy().into_owned(),
            wl_copy.to_string_lossy().into_owned(),
        );

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(payload.text_plain.as_deref(), Some("alpha"));
        assert_eq!(payload.text_html.as_deref(), Some("<b>alpha</b>"));
        assert_eq!(payload.image_png.as_deref(), Some(&[137, 80, 78, 71][..]));
    }

    #[tokio::test]
    async fn wl_clipboard_writes_payload_to_configured_command_by_mime() {
        let temp = temp_dir();
        let log = temp.join("copy.log");
        let wl_paste = write_executable(&temp, "wl-paste", "#!/bin/sh\nexit 1\n");
        let wl_copy = write_executable(
            &temp,
            "wl-copy",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\ncat >/dev/null\n",
                shell_quote(&log)
            ),
        );
        let clipboard = WlClipboard::with_commands(
            wl_paste.to_string_lossy().into_owned(),
            wl_copy.to_string_lossy().into_owned(),
        );
        let payload = ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: Some(vec![137, 80, 78, 71]),
        };

        clipboard.write_payload(&payload).await.unwrap();

        assert_eq!(
            fs::read_to_string(log).unwrap(),
            "--type text/html\n--type image/png\n--type text/plain\n"
        );
    }

    fn temp_dir() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rcopy-integration-{id}"));
        fs::create_dir(&path).unwrap();
        path
    }

    fn write_executable(dir: &Path, name: &str, script: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }
}
