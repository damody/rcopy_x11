mod ipc_client;
mod ui;
mod view_model;

use std::time::{Duration, SystemTime};

const TOGGLE_DEBOUNCE: Duration = Duration::from_millis(700);

fn main() {
    let AcquireResult::Run(_guard) = SingleInstanceGuard::acquire() else {
        return;
    };
    ui::run();
}

enum AcquireResult {
    Run(SingleInstanceGuard),
    ToggledExisting,
    SuppressedRepeat,
}

struct SingleInstanceGuard {
    path: std::path::PathBuf,
}

impl SingleInstanceGuard {
    fn acquire() -> AcquireResult {
        let path = lock_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if toggle_was_recent(&toggle_marker_path(&path)) {
            return AcquireResult::SuppressedRepeat;
        }

        match create_lock(&path) {
            Ok(()) => AcquireResult::Run(Self { path }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Some(pid) = lock_owner_pid(&path).filter(|pid| pid_is_alive(*pid)) {
                    mark_toggled(&toggle_marker_path(&path));
                    terminate_process(pid);
                    AcquireResult::ToggledExisting
                } else {
                    let _ = std::fs::remove_file(&path);
                    create_lock(&path)
                        .ok()
                        .map(|()| AcquireResult::Run(Self { path }))
                        .unwrap_or(AcquireResult::SuppressedRepeat)
                }
            }
            Err(_) => AcquireResult::SuppressedRepeat,
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

fn lock_owner_pid(path: &std::path::Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .map(|content| content.trim().to_string())
        .and_then(|content| {
            content
                .parse::<u32>()
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })
        .ok()
}

fn pid_is_alive(pid: u32) -> bool {
    std::path::PathBuf::from(format!("/proc/{pid}")).exists()
}

fn terminate_process(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
}

fn toggle_was_recent(path: &std::path::Path) -> bool {
    let Ok(modified) = std::fs::metadata(path).and_then(|metadata| metadata.modified()) else {
        return false;
    };

    SystemTime::now()
        .duration_since(modified)
        .map(|elapsed| elapsed < TOGGLE_DEBOUNCE)
        .unwrap_or(false)
}

fn mark_toggled(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, b"toggled\n");
}

fn toggle_marker_path(lock_path: &std::path::Path) -> std::path::PathBuf {
    lock_path.with_extension("toggle")
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
