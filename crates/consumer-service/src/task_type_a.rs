use crate::traits::RedisHandlerTrait;
use color_eyre::Result;
use shared_lib::models::tasks::TaskInfo;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace};

pub struct TaskTypeACreator;

impl TaskTypeACreator {
    /// 创建新实例
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl RedisHandlerTrait for TaskTypeACreator {
    async fn handle_task(&self, raw: String) -> Result<()> {
        trace!("[DEMO]handle task data raw {}", raw);

        let task_info = serde_json::from_str::<TaskInfo>(&raw)?;

        debug!("[DEMO]handle task info {:?}", task_info);

        tokio::time::sleep(Duration::from_secs(5)).await;

        Ok(())
    }

    fn stream_name(&self) -> &'static str {
        "task_type_a"
    }

    fn consumer_name_template(&self) -> &'static str {
        "task_consumer"
    }
}
