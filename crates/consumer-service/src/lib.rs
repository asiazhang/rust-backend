pub mod processors;
pub mod queue;

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, RedisResult};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

/// 消费者服务配置
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    pub redis_url: String,
    pub queue_name: String,
    pub dead_letter_queue: String,
    pub poll_interval: Duration,
    pub error_retry_interval: Duration,
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            queue_name: "task_queue".to_string(),
            dead_letter_queue: "dead_letter_queue".to_string(),
            poll_interval: Duration::from_secs(1),
            error_retry_interval: Duration::from_secs(5),
        }
    }
}

/// 消费者服务
pub struct ConsumerService {
    config: ConsumerConfig,
    conn: MultiplexedConnection,
}

impl ConsumerService {
    /// 创建新的消费者服务实例
    pub async fn new(config: ConsumerConfig) -> anyhow::Result<Self> {
        let client = redis::Client::open(config.redis_url.clone())?;
        let conn = client.get_multiplexed_async_connection().await?;
        
        Ok(Self { config, conn })
    }
    
    /// 启动消费者循环
    pub async fn start(&mut self) -> anyhow::Result<()> {
        info!("📡 Consumer Service 已连接到Redis，开始监听队列...");
        
        loop {
            match self.consume_message().await {
                Ok(processed) => {
                    if !processed {
                        sleep(self.config.poll_interval).await;
                    }
                }
                Err(e) => {
                    error!("❌ 消费消息时发生错误: {}", e);
                    sleep(self.config.error_retry_interval).await;
                }
            }
        }
    }
    
    /// 消费单条消息
    #[instrument(skip(self))]
    async fn consume_message(&mut self) -> RedisResult<bool> {
        let result: Option<String> = self.conn.lpop(&self.config.queue_name, None).await?;
        
        if let Some(message) = result {
            info!("📨 收到消息: {}", message);
            
            match self.process_message(&message).await {
                Ok(_) => {
                    info!("✅ 消息处理成功");
                }
                Err(e) => {
                    error!("❌ 消息处理失败: {}", e);
                    let _: () = self.conn.rpush(&self.config.dead_letter_queue, &message).await?;
                }
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// 处理消息
    #[instrument(skip(self))]
    async fn process_message(&mut self, message: &str) -> anyhow::Result<()> {
        let parsed: Value = serde_json::from_str(message)?;
        
        match parsed.get("type").and_then(|t| t.as_str()) {
            Some("task_a") => {
                info!("🔄 处理任务类型A");
                processors::task_a::process(&parsed).await
            }
            Some("task_b") => {
                info!("🔄 处理任务类型B");
                processors::task_b::process(&parsed).await
            }
            Some("minute_task") => {
                info!("🔄 处理分钟任务");
                processors::cron_tasks::process_minute_task(&parsed).await
            }
            Some("hourly_task") => {
                info!("🔄 处理小时任务");
                processors::cron_tasks::process_hourly_task(&parsed).await
            }
            Some("daily_task") => {
                info!("🔄 处理每日任务");
                processors::cron_tasks::process_daily_task(&parsed).await
            }
            _ => {
                warn!("⚠️ 未知消息类型: {}", message);
                Ok(())
            }
        }
    }
}
