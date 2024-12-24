//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

use crate::models::config::AppConfig;
use crate::models::tasks::TaskInfo;
use anyhow::Result;
use deadpool_redis::{Config, Pool, Runtime};
use futures::future::try_join_all;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{debug, error, warn};

pub async fn start_job_consumers(app_config: Arc<AppConfig>, rx: Receiver<bool>) -> Result<()> {
    let cfg = Config::from_url(&app_config.redis_addr);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    pool.resize(app_config.max_redis_concurrency);

    try_join!(start_create_task_consumers(pool.clone(), rx))?;

    Ok(())
}

/// 并发启动新建任务的redis消费者
pub async fn start_create_task_consumers(pool: Pool, cancel_rx: Receiver<bool>) -> Result<()> {
    let stream_name = "example_task_stream";
    let group_name = "example_task_group";

    let mut con = pool.get().await?;
    let _: () = con.xgroup_create(stream_name, group_name, "$").await?;

    let consumers: Vec<_> = (0..5)
        .map(|i| {
            let clone_p = pool.clone();
            let cancel_rx_clone = cancel_rx.clone();

            tokio::spawn(async move {
                consumer_task_worker(
                    cancel_rx_clone,
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
    mut cancel_rx: Receiver<bool>,
    pool: Pool,
    stream_name: &str,
    group_name: &str,
    consumer_name: &str,
) -> Result<()> {
    let mut undelivered_conn = pool.get().await?;
    let mut pending_conn = pool.get().await?;

    let opts = StreamReadOptions::default()
        .group(group_name, consumer_name)
        .block(1000)
        .count(10);
    let streams = vec![stream_name];

    loop {
        tokio::select! {
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    break;
                }
            }

            result = undelivered_conn.xread_options(&streams, &[">"], &opts) => {
                match result {
                    Ok(reply) => {
                        consume_redis_message(
                            &mut undelivered_conn,
                            reply,
                            stream_name,
                            group_name
                        ).await?
                    },
                    Err(e) => {
                        warn!("xread failed, err: {}", e);
                        undelivered_conn = pool.get().await?;
                    }
                }
            }
            pending_result = pending_conn.xread_options(&streams, &["0"], &opts) => {
                match pending_result {
                    Ok(reply) => {
                        consume_redis_message(
                            &mut pending_conn,
                            reply,
                            stream_name,
                            group_name
                        ).await?
                    },
                    Err(e) => {
                        warn!("xread failed, err: {}", e);
                        pending_conn = pool.get().await?;
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
    stream_name: &str,
    group_name: &str,
) -> Result<()> {
    for key in reply.keys {
        for stream_id in key.ids {
            debug!("processing redis stream id {}", stream_id.id);

            if let Some(Value::BulkString(data)) = stream_id.map.get(MESSAGE_KEY) {
                if let Ok(raw) = String::from_utf8(data.to_vec()) {
                    if let Ok(task_info) = serde_json::from_str::<TaskInfo>(&raw) {
                        handle_task(task_info).await;

                        let xack_ret: Result<(), RedisError> = conn
                            .xack(stream_name, group_name, &[stream_id.id.as_str()])
                            .await;

                        if let Err(err) = xack_ret {
                            error!(
                                "consumer redis message {} failed, err = {}",
                                stream_id.id, err
                            )
                        }
                    } else {
                        warn!(
                            "stream id {} format is not a TaskInfo, raw={}",
                            stream_id.id, raw
                        );
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

async fn handle_task(task: TaskInfo) {
    debug!("[DEMO]handle task {:?}", task);

    tokio::time::sleep(Duration::from_secs(5)).await;
}
