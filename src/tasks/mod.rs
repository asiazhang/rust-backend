//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

pub mod task;

use crate::models::config::AppConfig;
use crate::models::redis_task::RedisTask;
use crate::tasks::task::TaskCreator;
use anyhow::Result;
use deadpool_redis::{Config, Runtime};
use futures::future::try_join_all;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, Value};
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{debug, error, warn};

pub async fn start_job_consumers(app_config: Arc<AppConfig>, rx: Receiver<bool>) -> Result<()> {
    let cfg = Config::from_url(&app_config.redis_addr);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    pool.resize(app_config.max_redis_concurrency);

    let create_task_info = RedisTask {
        stream_name: "example_task_stream".to_string(),
        group_name: "example_task_group".to_string(),
        pool: pool.clone(),
        cancel_rx: rx.clone(),
        consumer_name: "task_consumer".to_string(),
        handler: Box::new(TaskCreator),
    };

    try_join!(start_create_task_consumers(app_config, create_task_info))?;

    Ok(())
}

/// 并发启动新建任务的redis消费者
pub async fn start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: RedisTask,
) -> Result<()> {
    let mut con = redis_task.pool.get().await?;
    let _: () = con
        .xgroup_create(&redis_task.stream_name, &redis_task.group_name, "$")
        .await?;

    let consumers: Vec<_> = (0..app_config.redis.max_consumer_count)
        .map(|i| {
            let consumer_name = format!("{}_{}", redis_task.consumer_name, i);
            let redis_task_clone = redis_task.clone();

            tokio::spawn(async move { consumer_task_worker(redis_task_clone, consumer_name).await })
        })
        .collect();

    try_join_all(consumers).await?;

    Ok(())
}

const MESSAGE_KEY: &str = "message";

async fn consumer_task_worker(mut redis_task: RedisTask, consumer_name: String) -> Result<()> {
    let mut undelivered_conn = redis_task.pool.get().await?;
    let mut pending_conn = redis_task.pool.get().await?;

    let opts = StreamReadOptions::default()
        .group(&redis_task.group_name, consumer_name)
        .block(1000)
        .count(10);
    let streams = vec![redis_task.stream_name.clone()];

    loop {
        tokio::select! {
            _ = redis_task.cancel_rx.changed() => {
                if *redis_task.cancel_rx.borrow() {
                    break;
                }
            }

            result = undelivered_conn.xread_options(&streams, &[">"], &opts) => {
                match result {
                    Ok(reply) => {
                        consume_redis_message(
                            &mut undelivered_conn,
                            reply,
                            &redis_task,
                        ).await?
                    },
                    Err(e) => {
                        warn!("xread failed, err: {}", e);
                        undelivered_conn = redis_task.pool.get().await?;
                    }
                }
            }
            pending_result = pending_conn.xread_options(&streams, &["0"], &opts) => {
                match pending_result {
                    Ok(reply) => {
                        consume_redis_message(
                            &mut pending_conn,
                            reply,
                            &redis_task,
                        ).await?
                    },
                    Err(e) => {
                        warn!("xread failed, err: {}", e);
                        pending_conn = redis_task.pool.get().await?;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn consume_redis_message(
    conn: &mut deadpool_redis::Connection,
    reply: StreamReadReply,
    redis_task: &RedisTask,
) -> Result<()> {
    for key in reply.keys {
        for stream_id in key.ids {
            debug!("processing redis stream id {}", stream_id.id);

            if let Some(Value::BulkString(data)) = stream_id.map.get(MESSAGE_KEY) {
                if let Ok(raw) = String::from_utf8(data.to_vec()) {
                    if let Err(err) = redis_task.handler.handle_task(raw).await {
                        error!("failed to handle redis message: {}", err);
                    }

                    let xack_ret: Result<(), RedisError> = conn
                        .xack(
                            &redis_task.stream_name,
                            &redis_task.group_name,
                            &[stream_id.id.as_str()],
                        )
                        .await;

                    if let Err(err) = xack_ret {
                        error!(
                            "consumer redis message {} from stream {} failed, err = {}",
                            stream_id.id, &redis_task.stream_name, err
                        )
                    }
                } else {
                    warn!("stream id {} format is not a string", stream_id.id);
                }
            } else {
                warn!("stream id {} not found", stream_id.id);
            }
        }
    }

    Ok(())
}
