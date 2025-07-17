use chrono::{DateTime, Utc};
use redis::{AsyncCommands, RedisResult};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, instrument};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    info!("🚀 启动 Cronjob Service...");
    
    // 初始化Redis连接
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let client = redis::Client::open(redis_url)?;
    let conn = client.get_multiplexed_async_connection().await?;
    
    // 创建任务调度器
    let scheduler = JobScheduler::new().await?;
    
    // 添加定时任务
    setup_cron_jobs(&scheduler, client).await?;
    
    info!("📅 Cronjob Service 已启动，定时任务已设置");
    
    // 启动调度器
    scheduler.start().await?;
    
    // 保持程序运行
    loop {
        sleep(Duration::from_secs(30)).await;
        info!("💓 Cronjob Service 心跳检查");
    }
}

#[instrument(skip(scheduler, redis_client))]
async fn setup_cron_jobs(scheduler: &JobScheduler, redis_client: redis::Client) -> anyhow::Result<()> {
    // 每分钟执行的任务
    let redis_client_clone = redis_client.clone();
    let job1 = Job::new_async("0 * * * * *", move |_uuid, _l| {
        let redis_client = redis_client_clone.clone();
        Box::pin(async move {
            if let Err(e) = minute_task(redis_client).await {
                error!("❌ 分钟任务执行失败: {}", e);
            }
        })
    })?;
    
    // 每小时执行的任务
    let redis_client_clone = redis_client.clone();
    let job2 = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let redis_client = redis_client_clone.clone();
        Box::pin(async move {
            if let Err(e) = hourly_task(redis_client).await {
                error!("❌ 小时任务执行失败: {}", e);
            }
        })
    })?;
    
    // 每天执行的任务
    let redis_client_clone = redis_client.clone();
    let job3 = Job::new_async("0 0 0 * * *", move |_uuid, _l| {
        let redis_client = redis_client_clone.clone();
        Box::pin(async move {
            if let Err(e) = daily_task(redis_client).await {
                error!("❌ 每日任务执行失败: {}", e);
            }
        })
    })?;
    
    scheduler.add(job1).await?;
    scheduler.add(job2).await?;
    scheduler.add(job3).await?;
    
    info!("✅ 定时任务设置完成");
    Ok(())
}

#[instrument(skip(redis_client))]
async fn minute_task(redis_client: redis::Client) -> anyhow::Result<()> {
    info!("⏰ 执行分钟任务...");
    
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    // 创建任务消息
    let task_message = json!({
        "type": "minute_task",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "这是一个分钟任务"
        }
    });
    
    // 将任务放入队列
    let _: () = conn.rpush("task_queue", task_message.to_string()).await?;
    
    info!("✅ 分钟任务已加入队列");
    Ok(())
}

#[instrument(skip(redis_client))]
async fn hourly_task(redis_client: redis::Client) -> anyhow::Result<()> {
    info!("⏰ 执行小时任务...");
    
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    // 创建任务消息
    let task_message = json!({
        "type": "hourly_task",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "这是一个小时任务"
        }
    });
    
    // 将任务放入队列
    let _: () = conn.rpush("task_queue", task_message.to_string()).await?;
    
    info!("✅ 小时任务已加入队列");
    Ok(())
}

#[instrument(skip(redis_client))]
async fn daily_task(redis_client: redis::Client) -> anyhow::Result<()> {
    info!("⏰ 执行每日任务...");
    
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    // 创建任务消息
    let task_message = json!({
        "type": "daily_task",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "这是一个每日任务",
            "reports": ["用户活跃度报告", "系统健康检查"]
        }
    });
    
    // 将任务放入队列
    let _: () = conn.rpush("task_queue", task_message.to_string()).await?;
    
    info!("✅ 每日任务已加入队列");
    Ok(())
}
