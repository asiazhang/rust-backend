//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

pub mod task;

use crate::models::config::AppConfig;
use crate::models::redis_task::RedisTask;
use crate::tasks::task::TaskCreator;
use color_eyre::eyre::Context;
use color_eyre::{eyre, Result};
use deadpool_redis::{Config, Runtime};
use futures::future::try_join_all;
use futures::FutureExt;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, RedisResult, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tracing::{debug, error, info, warn};

pub async fn start_job_consumers(app_config: Arc<AppConfig>, rx: Receiver<bool>) -> Result<()> {
    info!(
        "Starting redis job consumers with redis info {}...",
        &app_config.redis_addr
    );
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

    start_create_task_consumers(app_config, create_task_info).await?;

    info!("Redis job consumers stopped");

    Ok(())
}

/// 并发启动新建任务的redis消费者
pub async fn start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: RedisTask,
) -> Result<()> {
    let mut con = redis_task
        .pool
        .get()
        .await
        .context("get redis connection from pool")?;

    let re: RedisResult<()> = con
        .xgroup_create_mkstream(&redis_task.stream_name, &redis_task.group_name, "$")
        .await;
    if let Err(err) = re {
        warn!(
            "Failed to create redis task group {}: {}",
            redis_task.group_name, err
        );
    }

    let consumers: Vec<_> = (0..app_config.redis.max_consumer_count)
        .map(|i| {
            let consumer_name = format!("{}_{}", redis_task.consumer_name, i);
            let redis_task_clone = redis_task.clone();

            tokio::spawn(async move { consumer_task_worker(redis_task_clone, consumer_name).await })
        })
        .collect();

    try_join_all(consumers).await.context(format!(
        "wait for all consumer [{}] end",
        redis_task.consumer_name
    ))?;

    Ok(())
}

const MESSAGE_KEY: &str = "message";

async fn consumer_task_worker(mut redis_task: RedisTask, consumer_name: String) -> Result<()> {
    debug!("Redis job consumer {} started", consumer_name);

    let mut undelivered_conn = redis_task.pool.get().await?;
    let mut pending_conn = redis_task.pool.get().await?;

    let mut rng = StdRng::from_entropy();
    let value = rng.gen_range(1..=5);
    if value < 2 {
        warn!("This consumer [{}] random failed", consumer_name);
        eyre::bail!("This consumer [{}] random failed", consumer_name);
    }

    let opts = StreamReadOptions::default()
        .group(&redis_task.group_name, &consumer_name)
        .count(10);
    let streams = vec![redis_task.stream_name.clone()];

    loop {
        tokio::select! {
            _ = redis_task.cancel_rx.changed() => {
                if *redis_task.cancel_rx.borrow() {
                    break;
                }
            }
            pending_result = pending_conn.xread_options::<String, &str, StreamReadReply>(&streams, &["0"], &opts) => {
                match pending_result {
                    Ok(reply) if reply.keys.iter().all(|item| item.ids.is_empty()) => {
                        debug!("{} pending_conn: sleeping...", consumer_name);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    },
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
            result = undelivered_conn.xread_options::<String, &str, StreamReadReply>(&streams, &[">"], &opts) => {
                match result {
                    Ok(reply) if reply.keys.iter().all(|item| item.ids.is_empty())  => {
                        debug!("{} undelivered_conn: sleeping...", consumer_name);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    },
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
                    },
                }
            }
        }
    }

    debug!("Redis job consumer {} ended", consumer_name);

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
