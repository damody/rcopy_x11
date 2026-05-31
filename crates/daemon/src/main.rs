mod capture;
mod ipc;
mod service;

use anyhow::Result;
use rcopy_core::AppConfig;
use rcopy_integration::{WlClipboard, WtypePasteBackend};
use rcopy_storage::Repository;
use service::DaemonService;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::default();
    let capture_repo = Repository::open(&config.database_path)?;
    let service_repo = Repository::open(&config.database_path)?;

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("monitor runtime");
        runtime.block_on(capture::monitor_loop(
            capture_repo,
            WlClipboard::new(),
            Duration::from_millis(750),
        ));
    });

    let service = DaemonService::new(
        service_repo,
        WlClipboard::new(),
        WtypePasteBackend::new(config.paste_command.clone()),
    );
    ipc::serve_default_socket(service).await
}
