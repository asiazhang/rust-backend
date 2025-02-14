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

use tracing::debug;

pub async fn re_balance_redis_message() {
    debug!("Crons re-balance message");
}