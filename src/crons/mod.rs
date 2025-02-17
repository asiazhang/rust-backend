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
                re_balance_redis_message().await;
            })
        })?)
        .await?;

    sched.start().await?;
    
    // 由于shutdown_rx.changed()会等待信号，因此while循环不会导致CPU空转
    while *shutdown_rx.borrow() {
        // 通过 changed() 等待信号变化，同时自动检查发送端状态
        shutdown_rx.changed().await?;
    }

    sched.shutdown().await?;

    info!("finished cron tasks.");

    Ok(())
}
