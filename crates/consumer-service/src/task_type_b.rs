use color_eyre::Result;
use shared_lib::models::redis_task::{RedisHandler, RedisTask, RedisTaskCreator};
use shared_lib::models::tasks::TaskInfo;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

pub struct TaskTypeBCreator;

impl RedisHandler for TaskTypeBCreator {
    async fn handle_task(&self, raw: String) -> Result<()> {
        let task_info = serde_json::from_str::<TaskInfo>(&raw)?;

        debug!("[DEMO]handle task type b info {:?}", task_info);

        tokio::time::sleep(Duration::from_secs(10)).await;

        Ok(())
    }
}

impl RedisTaskCreator<TaskTypeBCreator> for TaskTypeBCreator {
    fn new_redis_task() -> Arc<RedisTask<TaskTypeBCreator>> {
        Arc::new(RedisTask {
            stream_name: "task_type_b".to_string(),
            consumer_name_template: "task_consumer".to_string(),
            handler: Arc::new(TaskTypeBCreator),
        })
    }
}
