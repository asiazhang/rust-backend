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

use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};
// use chrono::Utc;

#[derive(Debug, Clone)]
struct ConsumerStatus {
    name: String,
    group: String,
    stream: String,
    last_heartbeat: i64,
}

async fn rebalance(conn: &mut redis::aio::ConnectionManager) -> redis::RedisResult<()> {
    // 1. 读取 _consumer_status 哈希表所有消费者信息
    // let consumer_data: HashMap<String, String> = conn.hgetall("_consumer_status").await?;
    // let now_ts = Utc::now().timestamp();
    //
    // // 解析消费者信息，假设value为json或类似格式，这里为了简单假设用逗号分隔: "group,stream,last_heartbeat"
    // let mut consumers: Vec<ConsumerStatus> = vec![];
    // for (consumer_name, val) in consumer_data {
    //     let parts: Vec<&str> = val.split(',').collect();
    //     if parts.len() != 3 {
    //         eprintln!("Invalid consumer status format for {}", consumer_name);
    //         continue;
    //     }
    //     let group = parts[0].to_string();
    //     let stream = parts[1].to_string();
    //     let last_heartbeat = parts[2].parse::<i64>().unwrap_or(0);
    //
    //     consumers.push(ConsumerStatus {
    //         name: consumer_name,
    //         group,
    //         stream,
    //         last_heartbeat,
    //     });
    // }
    //
    // // 2. 找出失效消费者
    // let timeout_seconds = 60;
    // let mut inactive_consumers = vec![];
    // let mut active_consumers = vec![];
    //
    // for c in &consumers {
    //     if now_ts - c.last_heartbeat > timeout_seconds {
    //         inactive_consumers.push(c.clone());
    //     } else {
    //         active_consumers.push(c.clone());
    //     }
    // }
    //
    // // 3. 对失效消费者进行消息再平衡
    // for inactive in inactive_consumers {
    //     println!("Consumer {} is inactive, rebalancing...", inactive.name);
    //
    //     // 获取该消费者的 pending 消息 ID 列表 (XPENDING)
    //     // XPENDING <stream> <group> - + 1000 consumer
    //     let pending: Vec<(String, String, i64, i64)> = conn.xpending_count(
    //         &inactive.stream,
    //         &inactive.group,
    //         "-", "+",
    //         1000,
    //         &inactive.name,
    //     ).await.unwrap_or_default();
    //
    //     // 提取消息ID列表
    //     let message_ids: Vec<String> = pending.iter().map(|(id, _, _, _)| id.clone()).collect();
    //
    //     // 找同组的其他活跃消费者
    //     let other_consumers: Vec<&ConsumerStatus> = active_consumers.iter()
    //         .filter(|c| c.group == inactive.group && c.name != inactive.name)
    //         .collect();
    //
    //     if other_consumers.is_empty() {
    //         eprintln!("No other active consumers in group {} to rebalance", inactive.group);
    //         continue;
    //     }
    //
    //     // 将 pending 消息随机分配给其他消费者
    //     for (idx, msg_id) in message_ids.iter().enumerate() {
    //         let target_consumer = &other_consumers[idx % other_consumers.len()];
    //         // XCLAIM <stream> <group> <consumer> <min-idle-time> <ID> [ID ...]
    //         // 这里min-idle-time可设置为0，直接转让
    //         let _: () = redis::cmd("XCLAIM")
    //             .arg(&inactive.stream)
    //             .arg(&inactive.group)
    //             .arg(&target_consumer.name)
    //             .arg(0)
    //             .arg(msg_id)
    //             .query_async(conn)
    //             .await?;
    //     }
    //
    //     // 删除失效消费者心跳信息
    //     let _: () = conn.hdel("_consumer_status", &inactive.name).await?;
    //     println!("Rebalance for consumer {} completed.", inactive.name);
    // }

    Ok(())
}
