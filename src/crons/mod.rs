use crate::crons::balance::re_balance_redis_message;
use color_eyre::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

pub mod balance;

pub async fn start_cron_tasks(mut shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Result<()> {
    info!("starting cron tasks...");

    let mut sched = JobScheduler::new().await?;
    sched
        .add(Job::new_async("every 1 minutes", |_uuid, mut _l| {
            Box::pin(async move {
                // TODO: 增加redis消息再平衡，避免部分消息一直不被处理
                re_balance_redis_message().await;
            })
        })?)
        .await?;

    sched.start().await?;

    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        // 等待信号变化或发送端关闭
        if shutdown_rx.changed().await.is_err() {
            // 发送端已关闭，退出循环
            break;
        }
    }

    sched.shutdown().await?;

    info!("finished cron tasks.");

    Ok(())
}
