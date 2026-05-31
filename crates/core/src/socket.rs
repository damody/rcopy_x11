use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub fn default_socket_path() -> PathBuf {
    socket_path_for_runtime_dir(
        std::env::var_os("XDG_RUNTIME_DIR")
            .as_deref()
            .map(Path::new),
    )
}

fn socket_path_for_runtime_dir(runtime_dir: Option<&Path>) -> PathBuf {
    match runtime_dir {
        Some(runtime_dir) => runtime_dir.join("rcopy").join("rcopyd.sock"),
        None => std::env::temp_dir()
            .join(format!("rcopy-{}", current_uid()))
            .join("rcopyd.sock"),
    }
}

#[cfg(unix)]
fn current_uid() -> u32 {
    std::env::var("UID")
        .ok()
        .and_then(|uid| uid.parse().ok())
        .or_else(|| {
            std::fs::metadata("/proc/self")
                .ok()
                .map(|metadata| metadata.uid())
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::socket_path_for_runtime_dir;

    #[test]
    fn socket_path_uses_runtime_dir_when_available() {
        let runtime_dir = Path::new("/run/user/1000");

        assert_eq!(
            socket_path_for_runtime_dir(Some(runtime_dir)),
            runtime_dir.join("rcopy").join("rcopyd.sock")
        );
    }

    #[test]
    fn socket_path_falls_back_to_user_scoped_temp_dir() {
        let path = socket_path_for_runtime_dir(None);

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("rcopyd.sock")
        );
        assert!(path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rcopy-")));
    }
}
