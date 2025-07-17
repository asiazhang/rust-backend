use futures::StreamExt;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, RedisResult};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    info!("🚀 启动 Consumer Service...");
    
    // 初始化Redis连接
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    
    info!("📡 Consumer Service 已连接到Redis，开始监听队列...");
    
    // 启动消费者循环
    start_consumer_loop(&mut conn).await?;
    
    Ok(())
}

#[instrument(skip(conn))]
async fn start_consumer_loop(conn: &mut MultiplexedConnection) -> anyhow::Result<()> {
    loop {
        match consume_message(conn).await {
            Ok(processed) => {
                if !processed {
                    // 没有消息时稍作休息
                    sleep(Duration::from_secs(1)).await;
                }
            }
            Err(e) => {
                error!("❌ 消费消息时发生错误: {}", e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[instrument(skip(conn))]
async fn consume_message(conn: &mut MultiplexedConnection) -> RedisResult<bool> {
    // 尝试从队列中获取消息
    let queue_name = "task_queue";
    let result: Option<String> = conn.lpop(queue_name, None).await?;
    
    if let Some(message) = result {
        info!("📨 收到消息: {}", message);
        
        // 处理消息
        match process_message(&message).await {
            Ok(_) => {
                info!("✅ 消息处理成功");
            }
            Err(e) => {
                error!("❌ 消息处理失败: {}", e);
                // 可以选择将失败的消息放入死信队列
                let _: () = conn.rpush("dead_letter_queue", &message).await?;
            }
        }
        
        Ok(true)
    } else {
        Ok(false)
    }
}

#[instrument]
async fn process_message(message: &str) -> anyhow::Result<()> {
    // 解析消息
    let parsed: Value = serde_json::from_str(message)?;
    
    // 根据消息类型处理
    match parsed.get("type").and_then(|t| t.as_str()) {
        Some("task_a") => {
            info!("🔄 处理任务类型A");
            // 处理任务A的逻辑
            sleep(Duration::from_secs(2)).await;
            Ok(())
        }
        Some("task_b") => {
            info!("🔄 处理任务类型B");
            // 处理任务B的逻辑
            sleep(Duration::from_secs(3)).await;
            Ok(())
        }
        _ => {
            warn!("⚠️ 未知消息类型: {}", message);
            Ok(())
        }
    }
}
