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
pub struct XdotoolPasteBackend {
    command: String,
}

impl XdotoolPasteBackend {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

#[async_trait]
impl PasteBackend for XdotoolPasteBackend {
    async fn paste(&self) -> Result<(), PasteError> {
        let status = Command::new(OsStr::new(&self.command))
            .args(["key", "ctrl+v"])
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
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[tokio::test]
    async fn disabled_paste_backend_reports_not_available() {
        let paste = DisabledPasteBackend;
        let error = paste.paste().await.unwrap_err();

        assert!(matches!(error, PasteError::Unavailable(_)));
    }

    #[tokio::test]
    async fn xdotool_paste_backend_sends_ctrl_v_key_event() {
        let temp = temp_dir();
        let log = temp.join("xdotool.log");
        let xdotool = write_executable(
            &temp,
            "xdotool",
            &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > {}\n", shell_quote(&log)),
        );
        let paste = XdotoolPasteBackend::new(xdotool.to_string_lossy().into_owned());

        paste.paste().await.unwrap();

        assert_eq!(fs::read_to_string(log).unwrap(), "key ctrl+v\n");
    }

    #[tokio::test]
    async fn xdotool_paste_backend_reports_missing_command_as_unavailable() {
        let paste = XdotoolPasteBackend::new("rcopy-definitely-missing-xdotool");

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
        {
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(script_with_probe(script).as_bytes())
                .unwrap();
            file.sync_all().unwrap();
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        wait_until_executable_ready(&path);
        path
    }

    fn script_with_probe(script: &str) -> String {
        let probe = "if [ \"${RCOPY_FIXTURE_PROBE:-}\" = \"1\" ]; then exit 0; fi\n";
        if let Some(rest) = script.strip_prefix("#!") {
            if let Some((shebang, body)) = rest.split_once('\n') {
                return format!("#!{shebang}\n{probe}{body}");
            }
        }

        format!("#!/bin/sh\n{probe}{script}")
    }

    fn wait_until_executable_ready(path: &Path) {
        for attempt in 0..100 {
            match std::process::Command::new(path)
                .env("RCOPY_FIXTURE_PROBE", "1")
                .status()
            {
                Ok(status) if status.success() => return,
                Ok(status) => panic!("fixture executable probe failed with {status}"),
                Err(error) if error.raw_os_error() == Some(26) && attempt < 99 => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("fixture executable probe failed: {error}"),
            }
        }
    }

    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }
}
