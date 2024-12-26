use color_eyre::Result;
use tracing::info;

pub async fn start_cron_tasks(_rx: tokio::sync::watch::Receiver<bool>) -> Result<()> {
    info!("starting cron tasks...");
    
    // TODO: 增加redis消息再平衡，避免部分消息一直不被处理
    Ok(())
}
