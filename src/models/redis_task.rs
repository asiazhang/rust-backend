use async_trait::async_trait;
use color_eyre::Result;
use deadpool_redis::Pool;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio::sync::watch::Receiver;

/// Redis消费者任务信息
///
/// 不同的任务处理器这些信息不同：
/// - `stream_name`: 流名称不同，用于区分不同的消息业务类型
/// - `consumer_name`: 消费者名称不同，方便定位识别，实际执行的时候会加上序号（并发处理的多个消费者）
/// - `handler`: 核心业务处理器
pub struct RedisTask {
    /// Redis流名称
    pub stream_name: String,

    /// Redis消费者名称模板(不包含消费者索引ID)
    pub consumer_name_template: String,

    /// Redis数据库连接池
    pub pool: Pool,

    /// 系统关闭的型号
    pub shutdown_rx: Receiver<bool>,

    /// Redis消息处理器
    ///
    /// 这是一个动态的处理器，需要符合 [`RedisHandler`] 特征
    pub handler: Box<dyn RedisHandler>,
}

/// Redis消费者心跳信息
#[derive(Debug, Serialize, Deserialize)]
pub struct RedisConsumerHeartBeat {
    /// Redis流名称
    pub stream_name: String,

    /// Redis消费者名称
    pub consumer_name: String,

    /// 此消费者上次心跳时间
    pub last_heartbeat: i64,
}

impl Debug for RedisTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisTask")
            .field("stream_name", &self.stream_name)
            .field("consumer_name", &self.consumer_name_template)
            .field("pool", &self.pool)
            .field("shutdown_rx", &self.shutdown_rx)
            .finish()
    }
}

/// 异步Redis处理器特征
///
/// 由于[`RedisHandler`]需要async move到协程中，因此需要实现线程安全的[`Send`]和[`Sync`]
#[async_trait]
pub trait RedisHandler: Send + Sync {
    async fn handle_task(&self, task: String) -> Result<()>;
}
