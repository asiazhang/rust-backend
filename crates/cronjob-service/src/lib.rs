pub mod jobs;
pub mod scheduler;
pub mod crons;

use redis::{AsyncCommands, Client as RedisClient};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, instrument};

/// 定时任务服务配置
#[derive(Debug, Clone)]
pub struct CronjobConfig {
    pub redis_url: String,
    pub queue_name: String,
    pub heartbeat_interval: Duration,
}

impl Default for CronjobConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            queue_name: "task_queue".to_string(),
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

/// 定时任务服务
pub struct CronjobService {
    config: CronjobConfig,
    scheduler: JobScheduler,
    redis_client: RedisClient,
}

impl CronjobService {
    /// 创建新的定时任务服务实例
    pub async fn new(config: CronjobConfig) -> anyhow::Result<Self> {
        let scheduler = JobScheduler::new().await?;
        let redis_client = RedisClient::open(config.redis_url.clone())?;
        
        // 测试Redis连接
        let _conn = redis_client.get_multiplexed_async_connection().await?;
        
        Ok(Self {
            config,
            scheduler,
            redis_client,
        })
    }
    
    /// 启动定时任务服务
    pub async fn start(&self) -> anyhow::Result<()> {
        info!("🚀 启动 Cronjob Service...");
        
        // 设置定时任务
        self.setup_cron_jobs().await?;
        
        info!("📅 Cronjob Service 已启动，定时任务已设置");
        
        // 启动调度器
        self.scheduler.start().await?;
        
        // 心跳检查循环
        loop {
            sleep(self.config.heartbeat_interval).await;
            info!("💓 Cronjob Service 心跳检查");
        }
    }
    
    /// 设置定时任务
    #[instrument(skip(self))]
    async fn setup_cron_jobs(&self) -> anyhow::Result<()> {
        // 每分钟执行的任务
        let redis_client = self.redis_client.clone();
        let queue_name = self.config.queue_name.clone();
        let job1 = Job::new_async("0 * * * * *", move |_uuid, _l| {
            let redis_client = redis_client.clone();
            let queue_name = queue_name.clone();
            Box::pin(async move {
                if let Err(e) = enqueue_task(&redis_client, &queue_name, "minute_task", "这是一个分钟任务").await {
                    error!("❌ 分钟任务执行失败: {}", e);
                }
            })
        })?;
        
        // 每小时执行的任务
        let redis_client = self.redis_client.clone();
        let queue_name = self.config.queue_name.clone();
        let job2 = Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let redis_client = redis_client.clone();
            let queue_name = queue_name.clone();
            Box::pin(async move {
                if let Err(e) = enqueue_task(&redis_client, &queue_name, "hourly_task", "这是一个小时任务").await {
                    error!("❌ 小时任务执行失败: {}", e);
                }
            })
        })?;
        
        // 每天执行的任务
        let redis_client = self.redis_client.clone();
        let queue_name = self.config.queue_name.clone();
        let job3 = Job::new_async("0 0 0 * * *", move |_uuid, _l| {
            let redis_client = redis_client.clone();
            let queue_name = queue_name.clone();
            Box::pin(async move {
                if let Err(e) = enqueue_daily_task(&redis_client, &queue_name).await {
                    error!("❌ 每日任务执行失败: {}", e);
                }
            })
        })?;
        
        self.scheduler.add(job1).await?;
        self.scheduler.add(job2).await?;
        self.scheduler.add(job3).await?;
        
        info!("✅ 定时任务设置完成");
        Ok(())
    }
    
    /// 手动添加定时任务
    pub async fn add_job(&self, cron_expr: &str, job: Job) -> anyhow::Result<()> {
        self.scheduler.add(job).await?;
        info!("✅ 新增定时任务: {}", cron_expr);
        Ok(())
    }
    
    /// 获取所有任务状态
    pub async fn get_job_status(&self) -> Vec<String> {
        // 这里可以实现获取任务状态的逻辑
        vec!["minute_task: active".to_string(), "hourly_task: active".to_string(), "daily_task: active".to_string()]
    }
}

/// 将任务加入队列
#[instrument(skip(redis_client))]
async fn enqueue_task(redis_client: &RedisClient, queue_name: &str, task_type: &str, message: &str) -> anyhow::Result<()> {
    info!("⏰ 执行{}任务...", task_type);
    
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    let task_message = json!({
        "type": task_type,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": {
            "message": message
        }
    });
    
    let _: () = conn.rpush(queue_name, task_message.to_string()).await?;
    
    info!("✅ {}任务已加入队列", task_type);
    Ok(())
}

/// 将每日任务加入队列
#[instrument(skip(redis_client))]
async fn enqueue_daily_task(redis_client: &RedisClient, queue_name: &str) -> anyhow::Result<()> {
    info!("⏰ 执行每日任务...");
    
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    let task_message = json!({
        "type": "daily_task",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": {
            "message": "这是一个每日任务",
            "reports": ["用户活跃度报告", "系统健康检查"]
        }
    });
    
    let _: () = conn.rpush(queue_name, task_message.to_string()).await?;
    
    info!("✅ 每日任务已加入队列");
    Ok(())
}
