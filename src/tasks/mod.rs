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
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, RedisResult, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{debug, error, info, warn};

/// 启动redis消费者
///
/// ## 参数说明
/// - `app_config`: 程序配置
/// - `rx`: 用于接收关闭信号
///
/// ## 通用处理
///
/// 代码中的[`start_create_task_consumers`]是一个通用redis处理器，封装了相关逻辑，用户仅需要创建一个
/// [`RedisTask`]类型的结构体，传递给通用处理器即可。
///
/// 主要核心处理函数在handler，这是一个实现了[`crate::models::redis_task::RedisHandler`] 特征的处理器。
///
/// 用户一般这样使用：
///
/// ```rust
/// let info1 = RedisTask {handler: Box::new(Task1), ...}
/// let info2 = RedisTask {handler: Box::new(Task2), ...}
///
/// try_join!(
///     start_create_task_consumers(app_config, info1),
///     start_create_task_consumers(app_config, info2),
/// )?;
/// ```
///
/// ## 推荐设计
///
/// 一个redis stream中仅保存固定类型的数据，以方便程序处理。举例：
///
/// - `topic_task_a`: 仅处理`TypeA`类型数据
/// - `topic_task_b`: 仅处理`TypeB`类型数据
///
/// 这样的好处：
/// - 可以充分利用rust的**强类型**
/// - 方便数据序列化和反序列化
///
/// 缺点：
/// - 需要生成比较多的消费者
/// - 需要比较多的redis链接（特别是当前每个Redis消费者需要2个链接）
///
pub async fn start_job_consumers(
    app_config: Arc<AppConfig>,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    info!(
        "Starting redis job consumers with redis info {}...",
        &app_config.redis.redis_conn_str
    );

    // 使用deadpool_redis直接创建配置并生成数据库连接池
    let cfg = Config::from_url(&app_config.redis.redis_conn_str);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    // 调整redis数据库连接池的大小
    pool.resize(app_config.redis.max_redis_pool_size);

    // 声明需要处理的Redis类型数据
    let create_task_info = RedisTask {
        stream_name: "example_task_stream".to_string(),
        pool: pool.clone(),
        shutdown_rx: shutdown_rx.clone(),
        consumer_name: "task_consumer".to_string(),
        handler: Box::new(TaskCreator),
    };

    try_join!(start_create_task_consumers(app_config, create_task_info))?;

    info!("Redis job consumers stopped");

    Ok(())
}

const GROUP_NAME: &str = "rust-backend";

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
        .xgroup_create_mkstream(&redis_task.stream_name, GROUP_NAME, "$")
        .await;
    if let Err(err) = re {
        warn!("Failed to create redis task group {}: {}", GROUP_NAME, err);
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
        .group(GROUP_NAME, &consumer_name)
        .count(10);
    let streams = vec![redis_task.stream_name.clone()];

    loop {
        tokio::select! {
            _ = redis_task.shutdown_rx.changed() => {
                if *redis_task.shutdown_rx.borrow() {
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
                            GROUP_NAME,
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
