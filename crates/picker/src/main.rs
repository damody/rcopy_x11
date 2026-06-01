mod ipc_client;
mod ui;
mod view_model;

fn main() {
    let Some(_guard) = SingleInstanceGuard::acquire() else {
        return;
    };
    ui::run();
}

struct SingleInstanceGuard {
    path: std::path::PathBuf,
}

impl SingleInstanceGuard {
    fn acquire() -> Option<Self> {
        let path = lock_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match create_lock(&path) {
            Ok(()) => Some(Self { path }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if lock_owner_is_alive(&path) {
                    None
                } else {
                    let _ = std::fs::remove_file(&path);
                    create_lock(&path).ok().map(|()| Self { path })
                }
            }
            Err(_) => None,
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn create_lock(path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    writeln!(file, "{}", std::process::id())
}

fn lock_owner_is_alive(path: &std::path::Path) -> bool {
    let Ok(pid) = std::fs::read_to_string(path)
        .map(|content| content.trim().to_string())
        .and_then(|content| {
            content
                .parse::<u32>()
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })
    else {
        return false;
    };

    std::path::PathBuf::from(format!("/proc/{pid}")).exists()
}

fn lock_path() -> std::path::PathBuf {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(runtime_dir).join("rcopy/rcopy-picker.lock");
    }

    std::env::temp_dir().join(format!(
        "rcopy-{}/rcopy-picker.lock",
        std::env::var("UID").unwrap_or_else(|_| "unknown".to_string())
    ))
}
