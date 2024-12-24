use color_eyre::Result;
use async_trait::async_trait;
use deadpool_redis::Pool;
use tokio::sync::watch::Receiver;

pub struct RedisTask {
    pub stream_name: String,
    pub group_name: String,
    pub consumer_name: String,
    pub pool: Pool,
    pub cancel_rx: Receiver<bool>,
    pub handler: Box<dyn RedisHandler>,
}

impl Clone for RedisTask {
    fn clone(&self) -> Self {
        Self {
            stream_name: self.stream_name.clone(),
            group_name: self.group_name.clone(),
            consumer_name: self.consumer_name.clone(),
            pool: self.pool.clone(),
            cancel_rx: self.cancel_rx.clone(),
            handler: self.handler.clone_handler(),
        }
    }
}

#[async_trait]
pub trait RedisHandler: Send + Sync {
    async fn handle_task(&self, task: String) -> Result<()>;

    // 添加克隆方法
    fn clone_handler(&self) -> Box<dyn RedisHandler>;
}
