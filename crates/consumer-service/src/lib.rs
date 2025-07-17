//! 消费者服务模块
//!
//! 这个模块提供了消息队列消费的基础功能。

pub mod task_type_a;
pub mod task_type_b;

use self::task_type_a::TaskTypeACreator;
use self::task_type_b::TaskTypeBCreator;
use color_eyre::eyre::Context;
use color_eyre::Result;
use futures::future::try_join_all;
use futures::stream::iter;
use futures::StreamExt;
use redis::aio::ConnectionManager;
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, RedisResult, Value};
use share_lib::models::config::AppConfig;
use share_lib::models::redis_constants::{CONSUMER_GROUP_NAME, CONSUMER_HEARTBEAT_KEY, HEARTBEAT_INTERVAL_SECONDS};
use share_lib::models::redis_task::{RedisConsumerHeartBeat, RedisTask, RedisTaskCreator};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{debug, error, info, trace, warn};

/// 启动redis消费者
///
/// ## 参数说明
/// - `app_config`: 程序配置
/// - `shutdown_rx`: 用于接收关闭信号
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
/// let info1 = RedisTask {handler: Arc::new(Task1), ...}
/// let info2 = RedisTask {handler: Arc::new(Task2), ...}
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
pub async fn start_job_consumers(app_config: Arc<AppConfig>, shutdown_rx: Receiver<bool>) -> Result<()> {
    info!(
        "Starting redis job consumers with redis info {}...",
        &app_config.redis.redis_conn_str
    );

    try_join!(
        guard_start_create_task_consumers(Arc::clone(&app_config), TaskTypeACreator::new_redis_task(), shutdown_rx.clone()),
        guard_start_create_task_consumers(Arc::clone(&app_config), TaskTypeBCreator::new_redis_task(), shutdown_rx.clone())
    )?;

    info!("Redis job consumers stopped");

    Ok(())
}

async fn new_redis_connection_manager(conn_str: &str) -> Result<ConnectionManager> {
    Ok(ConnectionManager::new(redis::Client::open(conn_str)?).await?)
}

async fn guard_start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: Arc<RedisTask>,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    loop {
        let re = start_create_task_consumers(Arc::clone(&app_config), Arc::clone(&redis_task), shutdown_rx.clone()).await;
        match re {
            Ok(_) => break,
            Err(err) => {
                warn!("{}", err);
                warn!("Failed to start create task consumers, retrying...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

async fn start_create_task_consumers(app_config: Arc<AppConfig>, redis_task: Arc<RedisTask>, shutdown_rx: Receiver<bool>) -> Result<()> {
    create_task_group(app_config.redis.redis_conn_str.clone(), &redis_task).await?;

    let consumers: Vec<_> = (0..app_config.redis.max_consumer_count)
        .map(|i| {
            let consumer_name = format!("{}_{}", redis_task.consumer_name_template, i);

            consumer_task_worker_with_heartbeat(
                app_config.redis.redis_conn_str.clone(),
                Arc::clone(&redis_task),
                consumer_name,
                shutdown_rx.clone(),
            )
        })
        .collect();

    try_join_all(consumers)
        .await
        .context(format!("wait for all consumer [{}] end", redis_task.consumer_name_template))?;

    Ok(())
}

const MESSAGE_KEY: &str = "message";

async fn create_task_group(conn_str: String, redis_task: &RedisTask) -> Result<()> {
    let conn = new_redis_connection_manager(&conn_str).await?;

    let re: RedisResult<()> = conn
        .clone()
        .xgroup_create_mkstream(&redis_task.stream_name, CONSUMER_GROUP_NAME, "$")
        .await;
    if let Err(err) = re {
        warn!("Failed to create redis task group {}: {}", CONSUMER_GROUP_NAME, err);
    }

    Ok(())
}

async fn consumer_task_worker_with_heartbeat(
    conn_str: String,
    redis_task: Arc<RedisTask>,
    consumer_name: String,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    let conn = new_redis_connection_manager(&conn_str).await?;
    _ = try_join!(
        consumer_task_send_heartbeat(conn.clone(), Arc::clone(&redis_task), consumer_name.clone(), shutdown_rx.clone()),
        consumer_task_worker(conn.clone(), Arc::clone(&redis_task), consumer_name.clone(), shutdown_rx.clone()),
    )
    .context(format!("Creating consumer {consumer_name} with auto heartbeat"))?;

    Ok(())
}

async fn consumer_task_send_heartbeat(
    mut conn: ConnectionManager,
    redis_task: Arc<RedisTask>,
    consumer_name: String,
    mut shutdown_rx: Receiver<bool>,
) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS));

    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        tokio::select! {
            _ = shutdown_rx.changed() => {
              if *shutdown_rx.borrow() {
                  break;
              }
            }
          _ = interval.tick() => {
                let redis_heartbeat = RedisConsumerHeartBeat {
                    stream_name: redis_task.stream_name.clone(),
                    consumer_name: consumer_name.clone(),
                    last_heartbeat: OffsetDateTime::now_utc().unix_timestamp(),
                };

                if let Ok(json_data) = serde_json::to_string(&redis_heartbeat) {
                    trace!("Sending heartbeat to Redis: {}", json_data);
                    let res :Result<(), RedisError> = conn.hset(CONSUMER_HEARTBEAT_KEY, &consumer_name, json_data).await;
                    if let Err(err) = res {
                        warn!("Consumer {} redis heartbeat error: {}", consumer_name, err);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn consumer_task_worker(
    mut conn: ConnectionManager,
    redis_task: Arc<RedisTask>,
    consumer_name: String,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    debug!("Redis job consumer {} started", consumer_name);

    let opts = StreamReadOptions::default()
        .group(CONSUMER_GROUP_NAME, &consumer_name)
        .block(1000)
        .count(10);
    let streams = vec![redis_task.stream_name.clone()];

    let mut shutdown_rx = shutdown_rx.clone();

    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        tokio::select! {
          _ = shutdown_rx.changed() => {
              if *shutdown_rx.borrow() {
                  break;
              }
          }
          result = xread_group(&mut conn,&streams,&opts,&redis_task) => {
              match result {
                  Ok(_) => {}
                  Err(err) => {
                        warn!("{} xread group failed, err: {}, reconnecting...", consumer_name, err);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                  }
              }
          }
        }
    }

    debug!("Redis job consumer {} ended", consumer_name);

    Ok(())
}

async fn xread_group(
    conn: &mut ConnectionManager,
    streams: &[String],
    opts: &StreamReadOptions,
    redis_task: &Arc<RedisTask>,
) -> Result<()> {
    let pending_msg = conn.xread_options::<String, &str, StreamReadReply>(streams, &["0"], opts).await?;
    consume_redis_message(conn, pending_msg, redis_task).await?;

    let undelivered_msg = conn.xread_options::<String, &str, StreamReadReply>(streams, &[">"], opts).await?;
    consume_redis_message(conn, undelivered_msg, redis_task).await?;

    Ok(())
}

async fn consume_redis_message(conn: &mut ConnectionManager, reply: StreamReadReply, redis_task: &Arc<RedisTask>) -> Result<()> {
    for key in reply.keys {
        if key.ids.is_empty() {
            continue;
        }

        let tasks = key
            .ids
            .iter()
            .map(|id| consume_single_redis_message(Arc::clone(redis_task), id))
            .collect::<Vec<_>>();

        iter(tasks).buffer_unordered(5).collect::<Vec<_>>().await;

        let xack_ret: Result<(), RedisError> = conn
            .xack(
                &redis_task.stream_name,
                CONSUMER_GROUP_NAME,
                &key.ids.iter().map(|it| &it.id).collect::<Vec<_>>(),
            )
            .await;

        if let Err(err) = xack_ret {
            error!(
                "xack batch consumer redis message from stream {} failed, err = {}",
                &redis_task.stream_name, err
            )
        }
    }

    Ok(())
}

async fn consume_single_redis_message(redis_task: Arc<RedisTask>, stream_id: &StreamId) {
    if let Some(Value::BulkString(data)) = stream_id.map.get(MESSAGE_KEY) {
        if let Ok(raw) = String::from_utf8(data.to_vec()) {
            if let Err(err) = redis_task.handler.handle_task(raw).await {
                error!("failed to handle redis message: {}", err);
            }
        } else {
            warn!("stream id {} format is not a string", stream_id.id);
        }
    } else {
        warn!("stream id {} not found", stream_id.id);
    }
}
