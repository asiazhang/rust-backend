use color_eyre::Result;
use tracing::{info, error};
use crate::models::config::AppConfig;
use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use tokio::sync::watch::Receiver;

pub mod balance;

pub async fn start_cron_tasks(
    app_config: Arc<AppConfig>,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    info!("🕐 启动定时任务调度器...");

    // 创建 cron 调度器
    let mut sched = JobScheduler::new().await?;
    
    // 创建Redis连接用于重平衡任务
    let redis_client = redis::Client::open(app_config.redis.redis_conn_str.clone())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    
    // 添加Redis消息重平衡任务 - 每10秒执行一次
    let rebalance_job = Job::new_async("0/10 * * * * *", move |_uuid, _l| {
        let mut conn = redis_conn.clone();
        Box::pin(async move {
            if let Err(e) = balance::execute_rebalance_once(&mut conn).await {
                error!("❌ Redis重平衡任务执行失败: {}", e);
            }
        })
    })?;
    
    // 添加任务到调度器
    sched.add(rebalance_job).await?;
    
    // 启动调度器（非阻塞）
    sched.start().await?;
    info!("✅ 定时任务调度器已启动，Redis重平衡任务每10秒执行一次");
    
    // 等待关闭信号（这里才真正阻塞等待）
    wait_for_shutdown(shutdown_rx).await;
    
    info!("📴 收到关闭信号，停止定时任务调度器...");
    sched.shutdown().await?;
    info!("✅ 定时任务调度器已停止");
    
    Ok(())
}

/// 等待关闭信号
async fn wait_for_shutdown(mut shutdown_rx: Receiver<bool>) {
    // 如果已经是 true，直接返回
    if *shutdown_rx.borrow() {
        return;
    }
    
    // 等待信号变化
    let _ = shutdown_rx.changed().await;
}
