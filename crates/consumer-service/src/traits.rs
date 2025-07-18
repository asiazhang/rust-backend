use color_eyre::Result;

/// 异步Redis处理器特征
///
/// 由于[`RedisHandler`]需要async move到协程中，因此需要实现线程安全的[`Send`]和[`Sync`]
/// 在个人项目中，我们可以直接使用 async fn，简洁明了！🎉
///
/// 不同的任务处理器这些信息不同：
/// - `handler`: 核心业务处理器
/// - `stream_name`: 流名称不同，用于区分不同的消息业务类型
/// - `consumer_name`: 消费者名称不同，方便定位识别，实际执行的时候会加上序号（并发处理的多个消费者）
#[allow(async_fn_in_trait)]
pub trait RedisHandlerTrait: Send + Sync {
    async fn handle_task(&self, task: String) -> Result<()>;
    fn stream_name(&self) -> String;
    fn consumer_name_template(&self) -> String;
}
