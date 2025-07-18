use std::sync::Arc;
use std::fmt::Debug;
use color_eyre::Result;

/// Redis消费者任务信息
///
/// 不同的任务处理器这些信息不同：
/// - `stream_name`: 流名称不同，用于区分不同的消息业务类型
/// - `consumer_name`: 消费者名称不同，方便定位识别，实际执行的时候会加上序号（并发处理的多个消费者）
/// - `handler`: 核心业务处理器
pub struct RedisTask<T: RedisHandler> {
    /// Redis流名称
    pub stream_name: String,

    /// Redis消费者名称模板(不包含消费者索引ID)
    pub consumer_name_template: String,

    /// Redis消息处理器
    ///
    /// 这是一个强类型的处理器，需要实现 [`RedisHandler`] 特征
    pub handler: Arc<T>,
}

impl<T: RedisHandler> Debug for RedisTask<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisTask")
            .field("stream_name", &self.stream_name)
            .field("consumer_name", &self.consumer_name_template)
            .finish()
    }
}

/// 异步Redis处理器特征
///
/// 由于[`RedisHandler`]需要async move到协程中，因此需要实现线程安全的[`Send`]和[`Sync`]
/// 在个人项目中，我们可以直接使用 async fn，简洁明了！🎉
#[allow(async_fn_in_trait)]
pub trait RedisHandler: Send + Sync {
    async fn handle_task(&self, task: String) -> Result<()>;
}

/// Redis任务创建器特征
/// 
/// 用于创建不同类型的Redis任务实例
pub trait RedisTaskCreator<T: RedisHandler>: Send + Sync {
    fn new_redis_task() -> Arc<RedisTask<T>>;
}
