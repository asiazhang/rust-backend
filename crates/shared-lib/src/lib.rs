//! 🔧 共享库模块
//!
//! 这个模块包含了在多个服务之间共享的通用代码，包括：
//! - 公共数据结构
//! - Redis 模型和常量
//! - 分布式锁工具

pub mod distributed_lock;
pub mod models;

// 重新导出常用类型
pub use models::{
    AppConfig, RedisConfig, RedisConsumerHeartBeat, TaskInfo,
    // Redis 常量
    BATCH_SIZE, CONSUMER_GROUP_NAME, CONSUMER_HEARTBEAT_KEY, HEARTBEAT_INTERVAL_SECONDS,
    HEARTBEAT_TIMEOUT_SECONDS, LOCK_TTL_SECONDS, REBALANCE_LOCK_KEY,
};

// 重新导出分布式锁功能
pub use distributed_lock::{execute_with_lock, DistributedLock, LockGuard};
