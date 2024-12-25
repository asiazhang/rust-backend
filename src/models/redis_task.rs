use color_eyre::Result;
use async_trait::async_trait;
use deadpool_redis::Pool;
use tokio::sync::watch::Receiver;

/// Redis消费者任务信息
///
/// 不同的任务处理器这些信息不同：
/// - `stream_name`: 流名称不同，用于区分不同的消息业务类型
/// - `consumer_name`: 消费者名称不同，方便定位识别，实际执行的时候会加上序号（并发处理的多个消费者）
/// - `handler`: 核心业务处理器
#[derive(Debug)]
pub struct RedisTask {

    /// Redis流名称
    pub stream_name: String,

    /// Redis消费者名称
    pub consumer_name: String,

    /// Redis数据库连接池
    pub pool: Pool,

    /// 系统关闭的型号
    pub shutdown_rx: Receiver<bool>,

    /// Redis消息处理器
    ///
    /// 这是一个动态的处理器，需要符合 [`RedisHandler`] 特征
    #[debug(skip)]
    pub handler: Box<dyn RedisHandler>,
}

/// 由于[`RedisTask`]需要在多个协程中并发执行，因此最好的方式是实现Clone特征
/// 这样就避免数据竞争问题。
///
/// 在其他语言中，并发访问并不会检查所有权（比如`go`/`java`/`c++`/...），可能会遇到难以定位的数据竞争问题
/// Rust虽然复杂了些，但是语言模型保证了不会出现数据竞争。
impl Clone for RedisTask {
    fn clone(&self) -> Self {
        Self {
            stream_name: self.stream_name.clone(),
            consumer_name: self.consumer_name.clone(),
            pool: self.pool.clone(),
            shutdown_rx: self.shutdown_rx.clone(),
            handler: self.handler.clone_handler(),
        }
    }
}

/// 异步Redis处理器特征
///
/// 由于[`RedisHandler`]需要async move到协程中，因此需要实现线程安全的[`Send`]和[`Sync`]
#[async_trait]
pub trait RedisHandler: Send + Sync {
    async fn handle_task(&self, task: String) -> Result<()>;

    // 添加克隆方法
    fn clone_handler(&self) -> Box<dyn RedisHandler>;
}
