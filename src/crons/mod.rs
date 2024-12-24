use anyhow::Result;

pub async fn start_cron_tasks(rx: tokio::sync::watch::Receiver<bool>) -> Result<()> {
    Ok(())
}