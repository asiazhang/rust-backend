use crate::models::redis_task::RedisHandler;
use crate::models::tasks::TaskInfo;
use color_eyre::Result;
use async_trait::async_trait;
use std::time::Duration;
use tracing::{debug, trace};

pub struct TaskCreator;

#[async_trait]
impl RedisHandler for TaskCreator {
    async fn handle_task(&self, raw: String) -> Result<()> {
        trace!("[DEMO]handle task data raw {}", raw);

        let task_info = serde_json::from_str::<TaskInfo>(&raw)?;

        debug!("[DEMO]handle task info {:?}", task_info);

        tokio::time::sleep(Duration::from_secs(5)).await;

        Ok(())
    }

    fn clone_handler(&self) -> Box<dyn RedisHandler> {
        Box::new(TaskCreator {})
    }
}
