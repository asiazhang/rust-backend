use crate::traits::RedisHandlerTrait;
use color_eyre::Result;
use shared_lib::models::tasks::TaskInfo;
use std::time::Duration;
use tracing::debug;

pub struct TaskTypeBCreator;

impl RedisHandlerTrait for TaskTypeBCreator {
    async fn handle_task(&self, raw: String) -> Result<()> {
        let task_info = serde_json::from_str::<TaskInfo>(&raw)?;

        debug!("[DEMO]handle task type b info {:?}", task_info);

        tokio::time::sleep(Duration::from_secs(10)).await;

        Ok(())
    }

    fn stream_name(&self) -> String {
        "task_type_b".to_string()
    }

    fn consumer_name_template(&self) -> String {
        "task_consumer".to_string()
    }
}
