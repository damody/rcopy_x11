mod capture;
mod ipc;
mod service;

use anyhow::Result;
use rcopy_core::AppConfig;
use rcopy_integration::{WlClipboard, WtypePasteBackend};
use rcopy_storage::Repository;
use service::DaemonService;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::default();
    let service_repo = Repository::open(&config.database_path)?;

    let service = DaemonService::new(
        service_repo,
        WlClipboard::new(),
        WtypePasteBackend::new(config.paste_command.clone()),
    );
    ipc::serve_default_socket(service).await
}
