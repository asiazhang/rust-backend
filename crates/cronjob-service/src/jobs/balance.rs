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

use anyhow::Result;
use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisResult, Value};
use redis::{ExistenceCheck, SetExpiry, SetOptions};
use shared_lib::models::redis_constants::{
    BATCH_SIZE, CONSUMER_GROUP_NAME, CONSUMER_HEARTBEAT_KEY, HEARTBEAT_TIMEOUT_SECONDS, LOCK_TTL_SECONDS, REBALANCE_LOCK_KEY,
};
use shared_lib::models::redis_task::RedisConsumerHeartBeat;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// 扩展的消费者状态信息，包含分组信息
#[derive(Debug, Clone)]
pub struct ConsumerStatus {
    /// 心跳数据
    heartbeat: RedisConsumerHeartBeat,
    /// 消费者所属分组（从Redis stream消费者组信息中获取）
    group: String,
}

/// 执行一次Redis消息重平衡检查
///
/// 这个函数会执行一次重平衡检查，由外部 cron 调度器来调用
pub async fn execute_rebalance_once(conn: &mut ConnectionManager) -> Result<()> {
    debug!("🔄 执行重平衡检查");

    match rebalance_with_retry(conn).await {
        Ok(()) => {
            debug!("✅ 重平衡检查完成");
            Ok(())
        }
        Err(e) => {
            error!("❌ 重平衡任务执行失败: {}", e);
            Err(e)
        }
    }
}

// 注意：心跳写入功能已在 src/tasks/mod.rs 的 consumer_task_send_heartbeat 函数中实现
// 这里不需要重复实现心跳写入功能

/// 尝试获取分布式锁
async fn acquire_rebalance_lock(conn: &mut ConnectionManager) -> RedisResult<bool> {
    let result: Option<String> = conn
        .set_options(
            REBALANCE_LOCK_KEY,
            "locked",
            SetOptions::default()
                .conditional_set(ExistenceCheck::NX)
                .get(true)
                .with_expiration(SetExpiry::EX(LOCK_TTL_SECONDS)),
        )
        .await?;
    Ok(result.is_some())
}

/// 释放分布式锁
async fn release_rebalance_lock(conn: &mut ConnectionManager) -> RedisResult<()> {
    let _: i32 = conn.del(REBALANCE_LOCK_KEY).await?;
    Ok(())
}

/// 执行重平衡（带分布式锁）
async fn rebalance_with_retry(conn: &mut ConnectionManager) -> Result<()> {
    // 尝试获取分布式锁
    if !acquire_rebalance_lock(conn).await.unwrap_or(false) {
        debug!("🔒 其他重平衡任务正在运行，跳过本次执行");
        return Ok(());
    }

    debug!("🔓 成功获取重平衡锁");

    // 执行重平衡逻辑
    let rebalance_result = rebalance(conn).await.map_err(|e| anyhow::anyhow!("重平衡执行失败: {}", e));

    // 无论成功还是失败，都要释放锁
    if let Err(e) = release_rebalance_lock(conn).await {
        warn!("⚠️ 释放重平衡锁失败: {}", e);
    }

    rebalance_result
}

/// 重平衡逻辑
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
            warn!(
                "💀 发现失效消费者: {} ({}秒无响应)",
                status.heartbeat.consumer_name, time_since_heartbeat
            );
            failed_consumers.push(status);
        } else {
            debug!(
                "✅ 消费者正常: {} ({}秒前活跃)",
                status.heartbeat.consumer_name, time_since_heartbeat
            );
            active_consumers_by_group.entry(status.group.clone()).or_default().push(status);
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

/// 获取消费者所属的组信息（简化版本）
///
/// 由于系统设计是所有消费者都在同一个统一的组中，直接返回组名
async fn get_consumer_group(_conn: &mut ConnectionManager, _stream_name: &str, _consumer_name: &str) -> RedisResult<String> {
    // 简化逻辑：系统设计所有消费者都在同一个组中
    // 这样可以避免复杂的Redis查询和解析逻辑
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
                        let status = ConsumerStatus { heartbeat, group };
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
    let pending_messages = get_pending_messages(
        conn,
        &failed_consumer.heartbeat.stream_name,
        &failed_consumer.group,
        &failed_consumer.heartbeat.consumer_name,
    )
    .await?;

    if pending_messages.is_empty() {
        info!("📭 消费者 {} 没有pending消息需要重平衡", failed_consumer.heartbeat.consumer_name);
    } else {
        info!(
            "📬 消费者 {} 有 {} 条pending消息需要重平衡",
            failed_consumer.heartbeat.consumer_name,
            pending_messages.len()
        );

        // 3. 将pending消息批量分发给同组的活跃消费者
        let _ = redistribute_messages_batch(
            conn,
            &failed_consumer.heartbeat.stream_name,
            &failed_consumer.group,
            &pending_messages,
            active_consumers,
        )
        .await?;
    }

    // 4. 删除失效消费者的状态记录
    remove_consumer_status(conn, &failed_consumer.heartbeat.consumer_name).await?;

    info!("✅ 完成消费者 {} 的重平衡", failed_consumer.heartbeat.consumer_name);
    Ok(())
}

/// 获取指定消费者的pending消息
async fn get_pending_messages(conn: &mut ConnectionManager, stream: &str, group: &str, consumer: &str) -> RedisResult<Vec<String>> {
    // 使用更简单的方式获取pending消息
    // 这里我们使用 XPENDING 命令的简化版本
    #[allow(clippy::type_complexity)]
    let pending_info: (u64, String, String, Vec<(String, String, u64, u64)>) =
        conn.xpending_count(stream, group, "-", "+", 1000).await.unwrap_or_default();

    // 从第4个元素（pending消息列表）中过滤出属于指定消费者的消息
    let message_ids: Vec<String> = pending_info
        .3
        .into_iter()
        .filter(|(_, consumer_name, _, _)| consumer_name == consumer)
        .map(|(id, _, _, _)| id)
        .collect();

    debug!("📋 获取到 {} 条pending消息: {:?}", message_ids.len(), message_ids);
    Ok(message_ids)
}

/// 批量重新分发消息到活跃的消费者（优化版本）
async fn redistribute_messages_batch(
    conn: &mut ConnectionManager,
    stream: &str,
    group: &str,
    message_ids: &[String],
    active_consumers: &[ConsumerStatus],
) -> RedisResult<u64> {
    if message_ids.is_empty() || active_consumers.is_empty() {
        return Ok(0);
    }

    let consumer_names: Vec<&str> = active_consumers.iter().map(|c| c.heartbeat.consumer_name.as_str()).collect();

    let mut redistributed_count = 0;

    // 将消息ID按批次大小分组处理
    for (chunk_idx, chunk) in message_ids.chunks(BATCH_SIZE).enumerate() {
        // 为每个批次轮询选择消费者（简单但有效的分配策略）
        let target_consumer = match consumer_names.get(chunk_idx % consumer_names.len()) {
            Some(&consumer) => consumer,
            None => {
                error!("❌ 没有可用的活跃消费者来接收消息批次");
                continue;
            }
        };

        // 批量claim消息，使用高层API
        let chunk_refs: Vec<&String> = chunk.iter().collect();
        match conn.xclaim(stream, group, target_consumer, 0, &chunk_refs).await {
            Ok(claimed) => {
                let claimed_count = match claimed {
                    Value::Array(ref items) => items.len(),
                    _ => 0,
                };
                redistributed_count += claimed_count as u64;

                if claimed_count == chunk.len() {
                    info!("✅ 批量重分配 {} 条消息给消费者 {}", claimed_count, target_consumer);
                } else if claimed_count > 0 {
                    warn!(
                        "⚠️ 部分成功：重分配 {}/{} 条消息给消费者 {}",
                        claimed_count,
                        chunk.len(),
                        target_consumer
                    );
                } else {
                    warn!("⚠️ 消息批次重新分配失败，尝试逐条处理");
                    // 如果批量失败，尝试逐条处理
                    redistributed_count += redistribute_messages_individually(conn, stream, group, chunk, &consumer_names).await?;
                }
            }
            Err(e) => {
                warn!("⚠️ 批量claim失败: {}，尝试逐条处理", e);
                // 如果批量失败，尝试逐条处理
                redistributed_count += redistribute_messages_individually(conn, stream, group, chunk, &consumer_names).await?;
            }
        }
    }

    Ok(redistributed_count)
}

/// 逐条重新分发消息（当批量失败时的备用方案）
async fn redistribute_messages_individually(
    conn: &mut ConnectionManager,
    stream: &str,
    group: &str,
    message_ids: &[String],
    consumer_names: &[&str],
) -> RedisResult<u64> {
    let mut redistributed_count = 0;

    for (msg_idx, message_id) in message_ids.iter().enumerate() {
        if let Some(&target_consumer) = consumer_names.get(msg_idx % consumer_names.len()) {
            match conn.xclaim(stream, group, target_consumer, 0, &[message_id]).await {
                Ok(Value::Array(ref arr)) if !arr.is_empty() => {
                    redistributed_count += 1;
                    debug!("📤 消息 {} 已重新分配给消费者 {}", message_id, target_consumer);
                }
                Ok(_) => {
                    warn!("⚠️ 消息 {} 重新分配失败", message_id);
                }
                Err(e) => {
                    warn!("⚠️ 消息 {} claim失败: {}", message_id, e);
                }
            }
        } else {
            error!("❌ 没有可用的活跃消费者来接收消息 {}", message_id);
        }
    }

    Ok(redistributed_count)
}

/// 删除消费者状态记录
async fn remove_consumer_status(conn: &mut ConnectionManager, consumer_name: &str) -> RedisResult<()> {
    let _: i32 = conn.hdel(CONSUMER_HEARTBEAT_KEY, consumer_name).await?;
    info!("🗑️ 已删除失效消费者状态: {}", consumer_name);
    Ok(())
}

/// 获取组内所有消费者状态（用于监控）
#[allow(dead_code)]
pub async fn get_group_consumers(conn: &mut ConnectionManager, group_name: &str) -> RedisResult<Vec<ConsumerStatus>> {
    let all_statuses = get_all_consumer_statuses(conn).await?;

    let group_consumers: Vec<ConsumerStatus> = all_statuses.into_iter().filter(|status| status.group == group_name).collect();

    Ok(group_consumers)
}

/// 手动触发重平衡（用于调试和管理）
#[allow(dead_code)]
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
        client.get_connection_manager().await.unwrap()
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
