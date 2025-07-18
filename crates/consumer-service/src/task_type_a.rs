use crate::traits::RedisHandlerTrait;
use color_eyre::Result;
use shared_lib::models::tasks::TaskInfo;
use std::time::Duration;
use tracing::{debug, trace};

pub struct TaskTypeACreator;

impl RedisHandlerTrait for TaskTypeACreator {
    async fn handle_task(&self, raw: String) -> Result<()> {
        trace!("[DEMO]handle task data raw {}", raw);

        let task_info = serde_json::from_str::<TaskInfo>(&raw)?;

        debug!("[DEMO]handle task info {:?}", task_info);

        tokio::time::sleep(Duration::from_secs(5)).await;

        Ok(())
    }

    fn stream_name(&self) -> String {
        "task_type_a".to_string()
    }

    fn consumer_name_template(&self) -> String {
        "task_consumer".to_string()
    }
}
