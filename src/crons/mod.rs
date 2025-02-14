use crate::crons::balance::re_balance_redis_message;
use color_eyre::Result;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, trace};

pub mod balance;

pub async fn start_cron_tasks(mut shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Result<()> {
    info!("starting cron tasks...");

    let mut sched = JobScheduler::new().await?;
    sched
        .add(Job::new_async("every 5 minutes", |_uuid, mut _l| {
            Box::pin(async move {
                // TODO: 增加redis消息再平衡，避免部分消息一直不被处理
                re_balance_redis_message().await;
            })
        })?)
        .await?;

    sched.start().await?;

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        // 避免初始状态已经是true导致无法退出
        if *shutdown_rx.borrow() {
            break;
        }

        tokio::select! {
            // 如果收到shutdown信号，则直接退出
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    sched.shutdown().await?;
                    break;
                }
            }
            _ = interval.tick() => {
                // 继续等待
                trace!("interval tick");
            }
        }
    }

    info!("finished cron tasks.");

    Ok(())
}
