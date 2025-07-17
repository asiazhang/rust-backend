use color_eyre::Result;
use shared_lib::models::redis_task::{RedisHandler, RedisTask, RedisTaskCreator};
use shared_lib::models::tasks::TaskInfo;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace};

pub struct TaskTypeACreator;

impl RedisHandler for TaskTypeACreator {
    async fn handle_task(&self, raw: String) -> Result<()> {
        trace!("[DEMO]handle task data raw {}", raw);

        let task_info = serde_json::from_str::<TaskInfo>(&raw)?;

        debug!("[DEMO]handle task info {:?}", task_info);

        tokio::time::sleep(Duration::from_secs(5)).await;

        Ok(())
    }
}

impl RedisTaskCreator<TaskTypeACreator> for TaskTypeACreator {
    fn new_redis_task() -> Arc<RedisTask<TaskTypeACreator>> {
        Arc::new(RedisTask {
            stream_name: "task_type_a".to_string(),
            consumer_name_template: "task_consumer".to_string(),
            handler: Arc::new(TaskTypeACreator),
        })
    }
}
