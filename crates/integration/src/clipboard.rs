use async_trait::async_trait;
use rcopy_core::{ClipboardPayload, MimeKind};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

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
pub struct X11Clipboard {
    paste_command: String,
    copy_command: String,
}

impl Default for X11Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl X11Clipboard {
    pub fn new() -> Self {
        Self::with_commands("xclip", "xclip")
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
impl ClipboardBackend for X11Clipboard {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError> {
        let offered = list_offered_mimes(&self.paste_command).await?;

        let text_plain = if let Some(mime) = offered_mime_for(&offered, MimeKind::TextPlain) {
            read_optional_mime(&self.paste_command, mime)
                .await?
                .map(normalize_plain_text)
        } else {
            None
        };
        let text_html = if text_plain.is_none() {
            if let Some(mime) = offered_mime_for(&offered, MimeKind::TextHtml) {
                read_optional_mime(&self.paste_command, mime).await?
            } else {
                None
            }
        } else {
            None
        };
        let image_png = if let Some(mime) = offered_mime_for(&offered, MimeKind::ImagePng) {
            read_optional_bytes(&self.paste_command, mime).await?
        } else {
            None
        };

        Ok(ClipboardPayload {
            text_plain,
            text_html,
            image_png,
        })
    }

    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError> {
        if let Some(selection) = select_single_write_mime(payload) {
            write_mime(&self.copy_command, selection.mime(), selection.bytes()).await?;
        }
        Ok(())
    }
}

fn offered_mime_for(offered: &[String], kind: MimeKind) -> Option<&str> {
    let expected = match kind {
        MimeKind::TextPlain => "text/plain",
        MimeKind::TextHtml => "text/html",
        MimeKind::ImagePng => "image/png",
    };

    offered
        .iter()
        .find(|mime| normalize_offered_mime(mime) == expected)
        .map(String::as_str)
}

fn normalize_offered_mime(mime: &str) -> String {
    mime.split_once(';')
        .map_or(mime, |(base, _parameters)| base)
        .trim()
        .to_ascii_lowercase()
}

fn normalize_plain_text(text: String) -> String {
    normalize_plain_text_cow(&text).into_owned()
}

fn normalize_plain_text_cow(text: &str) -> Cow<'_, str> {
    if !looks_like_html_document(text) {
        return Cow::Borrowed(text);
    }

    let stripped = strip_html_document(text);
    if stripped.is_empty() {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(stripped)
    }
}

fn looks_like_html_document(text: &str) -> bool {
    let trimmed = text.trim_start().to_ascii_lowercase();
    trimmed.starts_with("<html")
        || trimmed.starts_with("<!doctype html")
        || trimmed.contains("<!--startfragment-->")
}

fn strip_html_document(html: &str) -> String {
    let without_comments = strip_html_comments(html);
    let mut text = String::new();
    let mut in_tag = false;

    for character in without_comments.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }

    decode_basic_html_entities(&text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_html_comments(html: &str) -> String {
    let mut output = String::new();
    let mut remaining = html;

    while let Some(start) = remaining.find("<!--") {
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start + 4..];
        if let Some(end) = after_start.find("-->") {
            remaining = &after_start[end + 3..];
        } else {
            return output;
        }
    }

    output.push_str(remaining);
    output
}

fn decode_basic_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

async fn list_offered_mimes(command: impl AsRef<OsStr>) -> Result<Vec<String>, ClipboardError> {
    let output = command_output(
        Command::new(command).args(["-selection", "clipboard", "-t", "TARGETS", "-o"]),
        "xclip -selection clipboard -t TARGETS -o",
    )
    .await?;

    if !output.status.success() {
        return Err(ClipboardError::Command(format!(
            "xclip TARGETS failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

enum ReadResult<T> {
    Available(T),
    Unsupported,
}

enum WriteMime<'a> {
    ImagePng(&'a [u8]),
    TextHtml(&'a str),
    TextPlain(Cow<'a, str>),
}

impl WriteMime<'_> {
    fn mime(&self) -> &'static str {
        match self {
            Self::ImagePng(_) => "image/png",
            Self::TextHtml(_) => "text/html",
            Self::TextPlain(_) => "text/plain",
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Self::ImagePng(bytes) => bytes,
            Self::TextHtml(text) => text.as_bytes(),
            Self::TextPlain(text) => text.as_bytes(),
        }
    }
}

fn select_single_write_mime(payload: &ClipboardPayload) -> Option<WriteMime<'_>> {
    // xclip accepts one advertised type per invocation. Until the backend grows
    // real multi-MIME support, write exactly one best representation.
    if let Some(image) = &payload.image_png {
        Some(WriteMime::ImagePng(image))
    } else if let Some(text) = &payload.text_plain {
        Some(WriteMime::TextPlain(normalize_plain_text_cow(text)))
    } else {
        payload.text_html.as_deref().map(WriteMime::TextHtml)
    }
}

async fn read_optional_mime(
    command: impl AsRef<OsStr>,
    mime: &str,
) -> Result<Option<String>, ClipboardError> {
    match read_mime(command, mime).await? {
        ReadResult::Available(text) => Ok(Some(text)),
        ReadResult::Unsupported => Ok(None),
    }
}

async fn read_optional_bytes(
    command: impl AsRef<OsStr>,
    mime: &str,
) -> Result<Option<Vec<u8>>, ClipboardError> {
    match read_bytes(command, mime).await? {
        ReadResult::Available(bytes) => Ok(Some(bytes)),
        ReadResult::Unsupported => Ok(None),
    }
}

async fn read_mime(
    command: impl AsRef<OsStr>,
    mime: &str,
) -> Result<ReadResult<String>, ClipboardError> {
    let output = command_output(
        Command::new(command).args(["-selection", "clipboard", "-t", mime, "-o"]),
        &format!("xclip -selection clipboard -t {mime} -o"),
    )
    .await?;

    if !output.status.success() {
        return classify_xclip_failure(mime, &output);
    }

    String::from_utf8(output.stdout)
        .map(ReadResult::Available)
        .map_err(|error| ClipboardError::Command(error.to_string()))
}

async fn read_bytes(
    command: impl AsRef<OsStr>,
    mime: &str,
) -> Result<ReadResult<Vec<u8>>, ClipboardError> {
    let output = command_output(
        Command::new(command).args(["-selection", "clipboard", "-t", mime, "-o"]),
        &format!("xclip -selection clipboard -t {mime} -o"),
    )
    .await?;

    if !output.status.success() {
        return classify_xclip_failure(mime, &output);
    }

    Ok(ReadResult::Available(output.stdout))
}

async fn command_output(
    command: &mut Command,
    description: &str,
) -> Result<std::process::Output, ClipboardError> {
    timeout(COMMAND_TIMEOUT, command.output())
        .await
        .map_err(|_| ClipboardError::Command(format!("{description} timed out")))?
        .map_err(|error| ClipboardError::Command(error.to_string()))
}

fn classify_xclip_failure<T>(
    mime: &str,
    output: &std::process::Output,
) -> Result<ReadResult<T>, ClipboardError> {
    if output.status.code() == Some(1) {
        Ok(ReadResult::Unsupported)
    } else {
        Err(ClipboardError::Command(format!(
            "xclip failed for {mime}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

async fn write_mime(
    command: impl AsRef<OsStr>,
    mime: &str,
    bytes: &[u8],
) -> Result<(), ClipboardError> {
    let mut child = Command::new(command)
        .args(["-selection", "clipboard", "-t", mime, "-i"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| ClipboardError::Command(error.to_string()))?;

    child
        .stdin
        .as_mut()
        .ok_or_else(|| ClipboardError::Command("xclip stdin unavailable".into()))?
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
            "xclip failed for {mime} with {status}"
        )))
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
    async fn x11_clipboard_prefers_plain_text_over_html_when_both_are_available() {
        let temp = temp_dir();
        let xclip = write_executable(
            &temp,
            "xclip",
            r#"#!/bin/sh
case "$4" in
  TARGETS) printf 'text/plain\ntext/html\nimage/png\n' ;;
  text/plain) printf 'alpha' ;;
  text/html) printf '<b>alpha</b>' ;;
  image/png) printf '\211PNG' ;;
  *) exit 1 ;;
esac
"#,
        );
        let clipboard = x11_clipboard(&xclip);

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(payload.text_plain.as_deref(), Some("alpha"));
        assert_eq!(payload.text_html, None);
        assert_eq!(payload.image_png.as_deref(), Some(&[137, 80, 78, 71][..]));
    }

    #[tokio::test]
    async fn x11_clipboard_reads_parameterized_plain_text_mime() {
        let temp = temp_dir();
        let xclip = xclip_read_fixture(
            &temp,
            "text/plain;charset=utf-8\ntext/html\n",
            &[
                ("text/plain;charset=utf-8", "plain alpha"),
                ("text/html", "<b>html alpha</b>"),
            ],
        );
        let clipboard = x11_clipboard(&xclip);

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(payload.text_plain.as_deref(), Some("plain alpha"));
        assert_eq!(payload.text_html, None);
        assert_eq!(payload.image_png, None);
    }

    #[tokio::test]
    async fn x11_clipboard_strips_html_document_from_plain_text_payload() {
        let temp = temp_dir();
        let xclip = xclip_read_fixture(
            &temp,
            "text/plain;charset=utf-8\n",
            &[(
                "text/plain;charset=utf-8",
                "<html><body><!--StartFragment--><pre><div><span>  現在 `Ctrl+`` 的行為是：</span></div></pre><!--EndFragment--></body></html>",
            )],
        );
        let clipboard = x11_clipboard(&xclip);

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(
            payload.text_plain.as_deref(),
            Some("現在 `Ctrl+`` 的行為是：")
        );
        assert_eq!(payload.text_html, None);
    }

    #[tokio::test]
    async fn x11_clipboard_reads_html_when_plain_text_is_unavailable() {
        let temp = temp_dir();
        let xclip = xclip_read_fixture(&temp, "text/html\n", &[("text/html", "<b>alpha</b>")]);
        let clipboard = x11_clipboard(&xclip);

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(payload.text_plain, None);
        assert_eq!(payload.text_html.as_deref(), Some("<b>alpha</b>"));
        assert_eq!(payload.image_png, None);
    }

    #[tokio::test]
    async fn x11_clipboard_writes_payload_to_configured_command_by_mime() {
        let temp = temp_dir();
        let log = temp.join("copy.log");
        let xclip = xclip_write_fixture(&temp, &log);
        let clipboard = x11_clipboard(&xclip);
        let payload = ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: Some(vec![137, 80, 78, 71]),
        };

        clipboard.write_payload(&payload).await.unwrap();

        assert_eq!(
            fs::read_to_string(log).unwrap(),
            "-selection clipboard -t image/png -i\n"
        );
    }

    #[tokio::test]
    async fn x11_clipboard_write_prefers_plain_text_over_html_when_both_are_present() {
        let temp = temp_dir();
        let log = temp.join("copy.log");
        let xclip = xclip_write_fixture(&temp, &log);
        let clipboard = x11_clipboard(&xclip);
        let payload = ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: None,
        };

        clipboard.write_payload(&payload).await.unwrap();

        assert_eq!(
            fs::read_to_string(log).unwrap(),
            "-selection clipboard -t text/plain -i\n"
        );
    }

    #[tokio::test]
    async fn x11_clipboard_write_uses_html_when_plain_text_is_absent() {
        let temp = temp_dir();
        let log = temp.join("copy.log");
        let xclip = xclip_write_fixture(&temp, &log);
        let clipboard = x11_clipboard(&xclip);
        let payload = ClipboardPayload {
            text_plain: None,
            text_html: Some("<b>alpha</b>".into()),
            image_png: None,
        };

        clipboard.write_payload(&payload).await.unwrap();

        assert_eq!(
            fs::read_to_string(log).unwrap(),
            "-selection clipboard -t text/html -i\n"
        );
    }

    #[tokio::test]
    async fn x11_clipboard_write_uses_plain_text_when_only_text_is_present() {
        let temp = temp_dir();
        let log = temp.join("copy.log");
        let xclip = xclip_write_fixture(&temp, &log);
        let clipboard = x11_clipboard(&xclip);
        let payload = ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: None,
            image_png: None,
        };

        clipboard.write_payload(&payload).await.unwrap();

        assert_eq!(
            fs::read_to_string(log).unwrap(),
            "-selection clipboard -t text/plain -i\n"
        );
    }

    #[tokio::test]
    async fn x11_clipboard_write_strips_html_document_from_plain_text_payload() {
        let temp = temp_dir();
        let output = temp.join("copy.out");
        let xclip = write_executable(
            &temp,
            "xclip",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >/dev/null\ncat > {}\n",
                shell_quote(&output)
            ),
        );
        let clipboard = x11_clipboard(&xclip);
        let payload = ClipboardPayload {
            text_plain: Some(
                "<html><body><!--StartFragment--><span>現在 `Ctrl+`` 的行為是：</span></body></html>"
                    .into(),
            ),
            text_html: None,
            image_png: None,
        };

        clipboard.write_payload(&payload).await.unwrap();

        assert_eq!(
            fs::read_to_string(output).unwrap(),
            "現在 `Ctrl+`` 的行為是："
        );
    }

    #[tokio::test]
    async fn x11_clipboard_read_returns_error_when_command_cannot_spawn() {
        let clipboard = X11Clipboard::with_commands(
            "rcopy-definitely-missing-xclip",
            "rcopy-definitely-missing-xclip",
        );

        let error = clipboard.read_supported().await.unwrap_err();

        assert!(matches!(error, ClipboardError::Command(_)));
    }

    #[tokio::test]
    async fn x11_clipboard_read_leaves_unsupported_mimes_empty() {
        let temp = temp_dir();
        let xclip = xclip_read_fixture(&temp, "text/plain\n", &[("text/plain", "alpha")]);
        let clipboard = x11_clipboard(&xclip);

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(payload.text_plain.as_deref(), Some("alpha"));
        assert_eq!(payload.text_html, None);
        assert_eq!(payload.image_png, None);
    }

    #[tokio::test]
    async fn x11_clipboard_read_returns_error_when_backend_fails_for_every_mime() {
        let temp = temp_dir();
        let xclip = write_executable(
            &temp,
            "xclip",
            "#!/bin/sh\nprintf 'x11 clipboard unavailable\\n' >&2\nexit 2\n",
        );
        let clipboard = x11_clipboard(&xclip);

        let error = clipboard.read_supported().await.unwrap_err();

        assert!(matches!(error, ClipboardError::Command(_)));
    }

    #[tokio::test]
    async fn x11_clipboard_does_not_read_unoffered_image_mime() {
        let temp = temp_dir();
        let xclip = write_executable(
            &temp,
            "xclip",
            r#"#!/bin/sh
if [ "$4" = "TARGETS" ]; then
  printf 'text/plain\n'
  exit 0
fi
if [ "$4" = "text/plain" ]; then
  printf 'alpha'
  exit 0
fi
if [ "$4" = "image/png" ]; then
  printf 'image/png should not be read\n' >&2
  exit 9
fi
exit 1
"#,
        );
        let clipboard = x11_clipboard(&xclip);

        let payload = clipboard.read_supported().await.unwrap();

        assert_eq!(payload.text_plain.as_deref(), Some("alpha"));
        assert_eq!(payload.image_png, None);
    }

    fn x11_clipboard(xclip: &Path) -> X11Clipboard {
        let command = xclip.to_string_lossy().into_owned();
        X11Clipboard::with_commands(command.clone(), command)
    }

    fn xclip_read_fixture(dir: &Path, targets: &str, entries: &[(&str, &str)]) -> PathBuf {
        let mut script = String::from("#!/bin/sh\n");
        script.push_str("case \"$4\" in\n");
        script.push_str(&format!(
            "  TARGETS) printf '%s' {} ;;\n",
            shell_quote_str(targets)
        ));
        for (mime, value) in entries {
            script.push_str(&format!(
                "  {}) printf '%s' {} ;;\n",
                shell_case_pattern(mime),
                shell_quote_str(value)
            ));
        }
        script.push_str("  *) exit 1 ;;\nesac\n");
        write_executable(dir, "xclip", &script)
    }

    fn xclip_write_fixture(dir: &Path, log: &Path) -> PathBuf {
        write_executable(
            dir,
            "xclip",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\ncat >/dev/null\n",
                shell_quote(log)
            ),
        )
    }

    fn shell_case_pattern(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    fn shell_quote_str(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
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
