//! Redis常量定义模块
//!
//! 统一管理所有Redis相关的键名、配置常量等，
//! 确保整个系统中使用的Redis键名保持一致。

/// Redis消费者心跳存储键
///
/// 用于存储所有消费者的心跳状态信息，格式为Hash:
/// - Key: 消费者名称
/// - Value: RedisConsumerHeartBeat的JSON序列化数据
pub const CONSUMER_HEARTBEAT_KEY: &str = "rust_backend_consumers:heartbeat";

/// Redis消费者组名称
///
/// 所有Redis Stream消费者都属于这个统一的组
pub const CONSUMER_GROUP_NAME: &str = "rust-backend";

/// 消费者心跳超时时间（秒）
///
/// 超过此时间没有心跳的消费者将被视为失效
pub const HEARTBEAT_TIMEOUT_SECONDS: i64 = 60;

/// 心跳发送间隔（秒）
///
/// 每个消费者发送心跳的频率
pub const HEARTBEAT_INTERVAL_SECONDS: u64 = 5;

/// 重平衡分布式锁键名
///
/// 防止多个重平衡任务同时运行导致竞态条件
pub const REBALANCE_LOCK_KEY: &str = "rust_backend:rebalance_lock";

/// 分布式锁超时时间（秒）
///
/// 防止锁永远不释放
pub const LOCK_TTL_SECONDS: u64 = 30;

/// 批量处理消息的大小
pub const BATCH_SIZE: usize = 10;
