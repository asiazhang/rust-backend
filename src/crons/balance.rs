//! Redis消息自动重平衡
//!
//! 当前使用Redis作为消息队列，但是Redis本身并没有自动重平衡功能，当某些消费者失效时，消息不会自动再平衡到其他消费者。
//!
//! 再平衡原理：
//! - 不同的消费者往 `_consumer_status` 中hset其心跳信息，包含以下数据：
//!     - 消费者名称
//!     - 消费者分组
//!     - 读取的stream信息
//!     - 心跳时间(unix timestamp)
//! - 每个消费者每5秒写入一次心跳数据
//! - 有一个cronjob定时任务，每隔10秒检查 `_consumer_status` 中的消费者心跳状态：
//!     - 如果已经60秒没有心跳数据写入（12次失败），则认为此消费者失效，发起数据再平衡功能
//!         - 将pending消息随机分发到同组的其他有效消费者
//!         - 分发完成后，删除其心跳数据
//!     - 如果心跳正常，则什么也不做
//!

use redis::{AsyncCommands, RedisResult, StreamId, StreamPendingReply, Value};
use redis::aio::ConnectionManager;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};
use tracing::{info, warn, error, debug};
use chrono::Utc;
use rand::seq::SliceRandom;
use anyhow::Result;
use crate::models::redis_task::RedisConsumerHeartBeat;
use crate::models::redis_constants::{
    CONSUMER_HEARTBEAT_KEY, CONSUMER_GROUP_NAME, HEARTBEAT_TIMEOUT_SECONDS, REBALANCE_CHECK_INTERVAL_SECONDS
};

/// 扩展的消费者状态信息，包含分组信息
#[derive(Debug, Clone)]
struct ConsumerStatus {
    /// 心跳数据
    heartbeat: RedisConsumerHeartBeat,
    /// 消费者所属分组（从Redis stream消费者组信息中获取）
    group: String,
}

/// 检查间隔时间
const CHECK_INTERVAL: Duration = Duration::from_secs(REBALANCE_CHECK_INTERVAL_SECONDS);

/// 启动Redis消息重平衡定时任务
/// 
/// 这个函数会持续运行，每隔10秒检查一次消费者状态
pub async fn start_rebalance_job(conn: ConnectionManager) -> Result<()> {
    info!("🔄 启动Redis消息重平衡定时任务");
    
    let mut conn = conn;
    
    loop {
        if let Err(e) = rebalance(&mut conn).await {
            error!("❌ 重平衡任务执行失败: {}", e);
        }
        
        sleep(CHECK_INTERVAL).await;
    }
}

// 注意：心跳写入功能已在 src/tasks/mod.rs 的 consumer_task_send_heartbeat 函数中实现
// 这里不需要重复实现心跳写入功能

/// 执行消息重平衡逻辑
async fn rebalance(conn: &mut ConnectionManager) -> RedisResult<()> {
    debug!("🔍 开始检查消费者状态...");
    
    // 1. 获取所有消费者状态
    let consumer_statuses = get_all_consumer_statuses(conn).await?;
    
    if consumer_statuses.is_empty() {
        debug!("📭 没有发现任何消费者状态");
        return Ok(());
    }
    
    let current_time = Utc::now().timestamp();
    let mut failed_consumers = Vec::new();
    let mut active_consumers_by_group: HashMap<String, Vec<ConsumerStatus>> = HashMap::new();
    
    // 2. 分析消费者状态，区分失效和正常的消费者
    for status in consumer_statuses {
        let time_since_heartbeat = current_time - status.heartbeat.last_heartbeat;
        
        if time_since_heartbeat > HEARTBEAT_TIMEOUT_SECONDS {
            warn!("💀 发现失效消费者: {} ({}秒无响应)", status.heartbeat.consumer_name, time_since_heartbeat);
            failed_consumers.push(status);
        } else {
            debug!("✅ 消费者正常: {} ({}秒前活跃)", status.heartbeat.consumer_name, time_since_heartbeat);
            active_consumers_by_group
                .entry(status.group.clone())
                .or_insert_with(Vec::new)
                .push(status);
        }
    }
    
    // 3. 对每个失效的消费者执行重平衡
    for failed_consumer in failed_consumers {
        if let Err(e) = rebalance_failed_consumer(conn, &failed_consumer, &active_consumers_by_group).await {
            error!("❌ 重平衡失效消费者 {} 失败: {}", failed_consumer.heartbeat.consumer_name, e);
        }
    }
    
    Ok(())
}

/// 获取消费者所属的组信息
/// 
/// 通过Redis stream的消费者组信息获取指定消费者所属的组
async fn get_consumer_group(
    conn: &mut ConnectionManager,
    stream_name: &str,
    consumer_name: &str,
) -> RedisResult<String> {
    // 使用统一的消费者组名称
    
    // 验证消费者是否确实存在于该组中
    // 使用XINFO CONSUMERS命令检查
    let consumers_info: Vec<Value> = conn
        .xinfo_consumers(stream_name, CONSUMER_GROUP_NAME)
        .await
        .unwrap_or_else(|_| Vec::new());
    
    // 检查是否找到了指定的消费者
    for consumer_info in consumers_info {
        if let Value::Array(consumer_data) = consumer_info {
            // 解析消费者信息，查找名称匹配的消费者
            for chunk in consumer_data.chunks(2) {
                if chunk.len() == 2 {
                    if let (Value::BulkString(key), Value::BulkString(value)) = (&chunk[0], &chunk[1]) {
                        if let (Ok(key_str), Ok(value_str)) = (String::from_utf8(key.clone()), String::from_utf8(value.clone())) {
                            if key_str == "name" && value_str == consumer_name {
                                return Ok(CONSUMER_GROUP_NAME.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 如果没有找到消费者，返回默认组名
    // 这是为了向后兼容，因为心跳数据可能存在但消费者已经停止
    debug!("消费者 {} 在stream {} 中未找到，使用默认组名", consumer_name, stream_name);
    Ok(CONSUMER_GROUP_NAME.to_string())
}

/// 获取所有消费者状态
async fn get_all_consumer_statuses(conn: &mut ConnectionManager) -> RedisResult<Vec<ConsumerStatus>> {
    let heartbeat_map: HashMap<String, String> = conn.hgetall(CONSUMER_HEARTBEAT_KEY).await?;
    
    let mut statuses = Vec::new();
    for (consumer_name, heartbeat_json) in heartbeat_map {
        match serde_json::from_str::<RedisConsumerHeartBeat>(&heartbeat_json) {
            Ok(heartbeat) => {
                // 验证消费者名称一致性
                if heartbeat.consumer_name == consumer_name {
                    // 获取消费者所属的组信息
                    if let Ok(group) = get_consumer_group(conn, &heartbeat.stream_name, &heartbeat.consumer_name).await {
                        let status = ConsumerStatus {
                            heartbeat,
                            group,
                        };
                        statuses.push(status);
                    } else {
                        warn!("⚠️ 无法获取消费者 {} 的组信息", consumer_name);
                    }
                } else {
                    warn!("⚠️ 消费者状态不一致: key={}, name={}", consumer_name, heartbeat.consumer_name);
                }
            }
            Err(e) => {
                warn!("⚠️ 解析消费者心跳失败: {} -> {}", consumer_name, e);
            }
        }
    }
    
    Ok(statuses)
}

/// 重平衡失效消费者的pending消息
async fn rebalance_failed_consumer(
    conn: &mut ConnectionManager,
    failed_consumer: &ConsumerStatus,
    active_consumers_by_group: &HashMap<String, Vec<ConsumerStatus>>,
) -> RedisResult<()> {
    info!("🔄 开始重平衡失效消费者: {}", failed_consumer.heartbeat.consumer_name);
    
    // 1. 检查同组是否有活跃的消费者
    let active_consumers = match active_consumers_by_group.get(&failed_consumer.group) {
        Some(consumers) if !consumers.is_empty() => consumers,
        _ => {
            warn!("⚠️ 组 {} 中没有活跃的消费者，跳过重平衡", failed_consumer.group);
            // 仍然删除失效消费者的状态
            remove_consumer_status(conn, &failed_consumer.heartbeat.consumer_name).await?;
            return Ok(());
        }
    };
    
    // 2. 获取失效消费者的pending消息
    let pending_messages = get_pending_messages(conn, &failed_consumer.heartbeat.stream_name, &failed_consumer.group, &failed_consumer.heartbeat.consumer_name).await?;
    
    if pending_messages.is_empty() {
        info!("📭 消费者 {} 没有pending消息需要重平衡", failed_consumer.heartbeat.consumer_name);
    } else {
        info!("📬 消费者 {} 有 {} 条pending消息需要重平衡", failed_consumer.heartbeat.consumer_name, pending_messages.len());
        
        // 3. 将pending消息随机分发给同组的活跃消费者
        redistribute_messages(conn, &failed_consumer.heartbeat.stream_name, &failed_consumer.group, &pending_messages, active_consumers).await?;
    }
    
    // 4. 删除失效消费者的状态记录
    remove_consumer_status(conn, &failed_consumer.heartbeat.consumer_name).await?;
    
    info!("✅ 完成消费者 {} 的重平衡", failed_consumer.heartbeat.consumer_name);
    Ok(())
}

/// 获取指定消费者的pending消息
async fn get_pending_messages(
    conn: &mut ConnectionManager,
    stream: &str,
    group: &str,
    consumer: &str,
) -> RedisResult<Vec<String>> {
    // 使用XPENDING命令获取特定消费者的pending消息
    let pending_reply: StreamPendingReply = conn
        .xpending_consumer_count(stream, group, "-", "+", 1000, consumer)
        .await?;
    
    let message_ids: Vec<String> = pending_reply
        .ids
        .into_iter()
        .map(|info| info.id)
        .collect();
    
    debug!("📋 获取到 {} 条pending消息: {:?}", message_ids.len(), message_ids);
    Ok(message_ids)
}

/// 重新分发消息到活跃的消费者
async fn redistribute_messages(
    conn: &mut ConnectionManager,
    stream: &str,
    group: &str,
    message_ids: &[String],
    active_consumers: &[ConsumerStatus],
) -> RedisResult<()> {
    let mut rng = rand::thread_rng();
    let consumer_names: Vec<&str> = active_consumers.iter().map(|c| c.heartbeat.consumer_name.as_str()).collect();
    
    for message_id in message_ids {
        // 随机选择一个活跃的消费者
        if let Some(&target_consumer) = consumer_names.choose(&mut rng) {
            // 使用XCLAIM命令将消息重新分配给目标消费者
            let claimed: Vec<StreamId> = conn
                .xclaim(
                    stream,
                    group,
                    target_consumer,
                    0, // min_idle_time设为0，强制claim
                    &[message_id],
                )
                .await?;
            
            if !claimed.is_empty() {
                info!("📤 消息 {} 已重新分配给消费者 {}", message_id, target_consumer);
            } else {
                warn!("⚠️ 消息 {} 重新分配失败", message_id);
            }
        } else {
            error!("❌ 没有可用的活跃消费者来接收消息 {}", message_id);
        }
    }
    
    Ok(())
}

/// 删除消费者状态记录
async fn remove_consumer_status(conn: &mut ConnectionManager, consumer_name: &str) -> RedisResult<()> {
    let _: i32 = conn.hdel(CONSUMER_HEARTBEAT_KEY, consumer_name).await?;
    info!("🗑️ 已删除失效消费者状态: {}", consumer_name);
    Ok(())
}

/// 获取组内所有消费者状态（用于监控）
pub async fn get_group_consumers(
    conn: &mut ConnectionManager,
    group_name: &str,
) -> RedisResult<Vec<ConsumerStatus>> {
    let all_statuses = get_all_consumer_statuses(conn).await?;
    
    let group_consumers: Vec<ConsumerStatus> = all_statuses
        .into_iter()
        .filter(|status| status.group == group_name)
        .collect();
    
    Ok(group_consumers)
}

/// 手动触发重平衡（用于调试和管理）
pub async fn trigger_manual_rebalance(conn: &mut ConnectionManager) -> RedisResult<()> {
    info!("🔧 手动触发重平衡");
    rebalance(conn).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Client;
    
    async fn get_test_connection() -> ConnectionManager {
        let client = Client::open("redis://127.0.0.1/").unwrap();
        client.get_tokio_connection_manager().await.unwrap()
    }
    
    #[tokio::test]
    async fn test_consumer_heartbeat() {
        let mut conn = get_test_connection().await;
        
        // 创建测试心跳数据
        let test_heartbeat = RedisConsumerHeartBeat {
            stream_name: "test_stream".to_string(),
            consumer_name: "test_consumer".to_string(),
            last_heartbeat: Utc::now().timestamp(),
        };
        
        let heartbeat_json = serde_json::to_string(&test_heartbeat).unwrap();
        let _: () = conn.hset(CONSUMER_HEARTBEAT_KEY, "test_consumer", heartbeat_json).await.unwrap();
        
        // 验证心跳数据被正确写入
        let statuses = get_all_consumer_statuses(&mut conn).await.unwrap();
        assert!(statuses.iter().any(|s| s.heartbeat.consumer_name == "test_consumer"));
        
        // 清理测试数据
        let _: () = conn.del(CONSUMER_HEARTBEAT_KEY).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_rebalance() {
        let mut conn = get_test_connection().await;
        
        // 创建一个过期的消费者心跳状态
        let old_time = Utc::now().timestamp() - HEARTBEAT_TIMEOUT_SECONDS - 10;
        let old_heartbeat = RedisConsumerHeartBeat {
            stream_name: "test_stream".to_string(),
            consumer_name: "old_consumer".to_string(),
            last_heartbeat: old_time,
        };
        
        let heartbeat_json = serde_json::to_string(&old_heartbeat).unwrap();
        let _: () = conn.hset(CONSUMER_HEARTBEAT_KEY, "old_consumer", heartbeat_json).await.unwrap();
        
        // 执行重平衡
        let result = rebalance(&mut conn).await;
        assert!(result.is_ok());
        
        // 验证过期消费者状态被删除
        let statuses = get_all_consumer_statuses(&mut conn).await.unwrap();
        assert!(!statuses.iter().any(|s| s.heartbeat.consumer_name == "old_consumer"));
        
        // 清理测试数据
        let _: () = conn.del(CONSUMER_HEARTBEAT_KEY).await.unwrap();
    }
}
