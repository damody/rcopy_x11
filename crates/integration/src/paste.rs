use async_trait::async_trait;
use std::ffi::OsStr;
use std::io::ErrorKind;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum PasteError {
    #[error("paste backend unavailable: {0}")]
    Unavailable(String),
    #[error("paste command failed: {0}")]
    Command(String),
}

#[async_trait]
pub trait PasteBackend: Send + Sync {
    async fn paste(&self) -> Result<(), PasteError>;
}

pub struct DisabledPasteBackend;

#[async_trait]
impl PasteBackend for DisabledPasteBackend {
    async fn paste(&self) -> Result<(), PasteError> {
        Err(PasteError::Unavailable("automatic paste disabled".into()))
    }
}

#[derive(Clone, Debug)]
pub struct WtypePasteBackend {
    command: String,
}

impl WtypePasteBackend {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

#[async_trait]
impl PasteBackend for WtypePasteBackend {
    async fn paste(&self) -> Result<(), PasteError> {
        let status = Command::new(OsStr::new(&self.command))
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status()
            .await
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound | ErrorKind::PermissionDenied => {
                    PasteError::Unavailable(error.to_string())
                }
                _ => PasteError::Command(error.to_string()),
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(PasteError::Command(format!(
                "{} exited with {status}",
                self.command
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[tokio::test]
    async fn disabled_paste_backend_reports_not_available() {
        let paste = DisabledPasteBackend;
        let error = paste.paste().await.unwrap_err();

        assert!(matches!(error, PasteError::Unavailable(_)));
    }

    #[tokio::test]
    async fn wtype_paste_backend_sends_ctrl_v_key_event() {
        let temp = temp_dir();
        let log = temp.join("wtype.log");
        let wtype = write_executable(
            &temp,
            "wtype",
            &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > {}\n", shell_quote(&log)),
        );
        let paste = WtypePasteBackend::new(wtype.to_string_lossy().into_owned());

        paste.paste().await.unwrap();

        assert_eq!(fs::read_to_string(log).unwrap(), "-M ctrl -k v -m ctrl\n");
    }

    #[tokio::test]
    async fn wtype_paste_backend_reports_missing_command_as_unavailable() {
        let paste = WtypePasteBackend::new("rcopy-definitely-missing-wtype");

        let error = paste.paste().await.unwrap_err();

        assert!(matches!(error, PasteError::Unavailable(_)));
    }

    fn temp_dir() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("rcopy-integration-{id}-{counter}"));
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
