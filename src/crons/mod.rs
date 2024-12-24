use color_eyre::Result;
use tracing::info;

pub async fn start_cron_tasks(_rx: tokio::sync::watch::Receiver<bool>) -> Result<()> {
    info!("starting cron tasks...");
    Ok(())
}