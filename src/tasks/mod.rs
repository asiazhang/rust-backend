//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

use crate::models::tasks::TaskInfo;
use anyhow::{Context, Result};
use deadpool_redis::{Config, Pool, Runtime};
use futures::future::try_join_all;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, Value};
use std::time::Duration;
use tokio::try_join;
use tracing::{debug, warn};

pub async fn start_job_consumers() -> Result<()> {
    let cfg = Config::from_url(
        std::env::var("REDIS_URL").context("Can not load REDIS_URL in environment")?,
    );
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    try_join!(start_create_task_consumers(pool.clone()))?;

    Ok(())
}

/// 并发启动新建任务的redis消费者
pub async fn start_create_task_consumers(pool: Pool) -> Result<()> {
    let stream_name = "example_task_stream";
    let group_name = "example_task_group";

    let mut con = pool.get().await?;
    con.xgroup_create(stream_name, group_name, "$").await?;

    let consumers: Vec<_> = (0..5)
        .map(|i| {
            let clone_p = pool.clone();
            tokio::spawn(async move {
                consumer_task_worker(
                    clone_p,
                    stream_name,
                    group_name,
                    &format!("task_consumer_{}", i),
                )
                .await
            })
        })
        .collect();

    try_join_all(consumers).await?;

    Ok(())
}

const MESSAGE_KEY: &str = "message";

async fn consumer_task_worker(
    pool: Pool,
    stream_name: &str,
    group_name: &str,
    consumer_name: &str,
) -> Result<()> {
    let mut con = pool.get().await?;

    loop {
        let opts = StreamReadOptions::default()
            .group(group_name, consumer_name)
            .block(0)
            .count(10);

        let result: StreamReadReply = con.xread_options(&[stream_name], &[">"], &opts).await?;

        for key in result.keys {
            for stream_id in key.ids {
                debug!("processing redis stream id {}", stream_id.id);

                if let Some(Value::BulkString(data)) = stream_id.map.get(MESSAGE_KEY) {
                    if let Ok(raw) = String::from_utf8(data.to_vec()) {
                        if let Ok(task_info) = serde_json::from_str::<TaskInfo>(&raw) {
                            handle_task(task_info).await
                        }
                    }
                } else {
                    warn!("stream id {} not found or format not string", stream_id.id);
                }
            }
        }
    }

    Ok(())
}

async fn handle_task(task: TaskInfo) {
    debug!("[DEMO]handle task {:?}", task);

    tokio::time::sleep(Duration::from_secs(5)).await;
}
