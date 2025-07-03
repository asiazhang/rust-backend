use color_eyre::Result;
use tracing::{info, error};
use crate::models::config::AppConfig;
use std::sync::Arc;

pub mod balance;

pub async fn start_cron_tasks(
    app_config: Arc<AppConfig>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    info!("🕐 启动定时任务...");

    // 创建Redis连接用于重平衡任务
    let redis_client = redis::Client::open(app_config.redis.redis_conn_str.clone())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    
    // 启动Redis消息重平衡任务
    let balance_task = tokio::spawn(async move {
        if let Err(e) = balance::start_rebalance_job(redis_conn).await {
            error!("❌ Redis重平衡任务失败: {}", e);
        }
    });
    
    // 等待关闭信号
    let mut shutdown_rx = shutdown_rx;
    let _ = shutdown_rx.changed().await;
    
    if *shutdown_rx.borrow() {
        info!("📴 收到关闭信号，停止定时任务...");
        balance_task.abort();
    }
    
    Ok(())
}
