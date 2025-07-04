//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

pub mod task_type_a;

pub mod task_type_b;

use crate::models::config::AppConfig;
use crate::models::redis_task::{RedisConsumerHeartBeat, RedisTask, RedisTaskCreator};
use crate::models::redis_constants::{CONSUMER_HEARTBEAT_KEY, CONSUMER_GROUP_NAME, HEARTBEAT_INTERVAL_SECONDS};
use crate::tasks::task_type_a::TaskTypeACreator;
use crate::tasks::task_type_b::TaskTypeBCreator;
use color_eyre::Result;
use color_eyre::eyre::Context;
use futures::StreamExt;
use futures::future::try_join_all;
use futures::stream::iter;
use redis::aio::ConnectionManager;
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, RedisResult, Value};
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
pub async fn start_job_consumers(
    app_config: Arc<AppConfig>,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    info!(
        "Starting redis job consumers with redis info {}...",
        &app_config.redis.redis_conn_str
    );

    // NOTE: 如果有其他需要处理的Redis类型，那么按照👇的例子来编写
    try_join!(
        guard_start_create_task_consumers(
            Arc::clone(&app_config),
            TaskTypeACreator::new_redis_task(),
            shutdown_rx.clone()
        ),
        guard_start_create_task_consumers(
            Arc::clone(&app_config),
            TaskTypeBCreator::new_redis_task(),
            shutdown_rx.clone()
        )
    )?;

    info!("Redis job consumers stopped");

    Ok(())
}

async fn new_redis_connection_manager(conn_str: &str) -> Result<ConnectionManager> {
    Ok(ConnectionManager::new(redis::Client::open(
        conn_str,
    )?)
    .await?)
}

// GROUP_NAME 现在从 redis_constants 模块导入

/// 让redis消费者一直执行，直到收到shutdown信号
///
/// 只要是失败的场景，则一直不停重试，确保消费者不退出
async fn guard_start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: Arc<RedisTask>,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    loop {
        // 只要不是收到shutdown信号，则一直循环执行，避免中途redis执行出错导致消费者退出
        let re = start_create_task_consumers(
            Arc::clone(&app_config),
            Arc::clone(&redis_task),
            shutdown_rx.clone(),
        )
        .await;
        match re {
            Ok(_) => break, // OK表示收到shutdown信号，正常退出
            Err(err) => {
                warn!("{}", err);
                warn!("Failed to start create task consumers, retrying...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

/// 并发启动新建任务的redis消费者
///
/// 为了提升效率，我们这里会在同一个消费组中启动多个消费者，并行处理消息。
/// 用户可通过配置 `MAX_CONSUMER_COUNT` 环境变量来设置并发程度
///
async fn start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: Arc<RedisTask>,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    create_task_group(app_config.redis.redis_conn_str.clone(), &redis_task).await?;

    // 根据配置数据，创建多个消费者。
    // 这几个消费者会并行从redis读取消息并消费。
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

    // 有任何一个消费者创建失败，则返回失败，让上层进行重试
    try_join_all(consumers).await.context(format!(
        "wait for all consumer [{}] end",
        redis_task.consumer_name_template
    ))?;

    Ok(())
}

const MESSAGE_KEY: &str = "message";

async fn create_task_group(conn_str: String, redis_task: &RedisTask) -> Result<()> {
    // 创建消费组，来支持并行消费消息。
    // 由于消费组不能多次创建，因此失败了提示warning即可
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

/// 启动一个带心跳发送的Redis消费者
///
/// 使用心跳主要是避免以下问题：
/// - 用户调整Redis消费者数目，导致部分消息一直pending无法处理
/// - 部分消费者由于某些原因导致无法处理消息，进而导致部分消息一直pending无法处理
///
/// 通过记录心跳消息，我们能检测到哪些消费者已经失效，就可以将发给此消费者的信息重平衡到其他消费者
async fn consumer_task_worker_with_heartbeat(
    conn_str: String,
    redis_task: Arc<RedisTask>,
    consumer_name: String,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    let conn = new_redis_connection_manager(&conn_str).await?;
    _ = try_join!(
        consumer_task_send_heartbeat(
            conn.clone(),
            Arc::clone(&redis_task),
            consumer_name.clone(),
            shutdown_rx.clone()
        ),
        consumer_task_worker(
            conn.clone(),
            Arc::clone(&redis_task),
            consumer_name.clone(),
            shutdown_rx.clone()
        ),
    )
    .context(format!(
        "Creating consumer {consumer_name} with auto heartbeat"
    ))?;

    Ok(())
}

/// 定时发送心跳消息
async fn consumer_task_send_heartbeat(
    mut conn: ConnectionManager,
    redis_task: Arc<RedisTask>,
    consumer_name: String,
    mut shutdown_rx: Receiver<bool>,
) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS));

    loop {
        // 避免初始状态已经是true导致无法退出
        if *shutdown_rx.borrow() {
            break;
        }

        tokio::select! {
            // 如果收到shutdown信号，则直接退出
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
        .block(1000) // 最长等待时间1秒，可以满足大多数场景
        .count(10); // 最多获取10个数据
    let streams = vec![redis_task.stream_name.clone()];

    // 这里必须要把shutdown_rx克隆一次
    // 否则在[`select!`]多个分支中，都会访问到 &mut redis_task
    // 这样会违反redis的借用原则（mut借用在作用域里面只能有一个）
    let mut shutdown_rx = shutdown_rx.clone();

    loop {
        // 避免初始状态已经是true导致无法退出
        if *shutdown_rx.borrow() {
            break;
        }

        tokio::select! {
            // 如果收到shutdown信号，则直接退出
          _ = shutdown_rx.changed() => {
              if *shutdown_rx.borrow() {
                  break;
              }
          }
            // 没有收到shutdown信号，则读取redis消息并处理
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

/// 调用xread group读取redis流里面的数据
///
/// ## `0`流
///
/// `0`流表示读取redis中的pending数据（之前没有ack处理完成的）。
///
/// **注意**：block超时参数对0流无效，所以读取0流不会阻塞。
///
/// > https://redis.io/docs/latest/commands/xreadgroup/
/// > 因此，基本上，如果 ID 不是> ，那么该命令只会让客户端访问其挂起的条目：消息已传递给它，但尚未确认。
/// > 请注意，在这种情况下，**`BLOCK` 和 `NOACK` 都被忽略**。
///
/// ## `>`流
///
/// `>`流表示读取redis中的undelivered数据。会正常遵守block时间。
async fn xread_group(
    conn: &mut ConnectionManager,
    streams: &[String],
    opts: &StreamReadOptions,
    redis_task: &Arc<RedisTask>,
) -> Result<()> {
    // 先处理pending数据
    let pending_msg = conn
        .xread_options::<String, &str, StreamReadReply>(streams, &["0"], opts)
        .await?;
    consume_redis_message(conn, pending_msg, redis_task).await?;

    // 再处理未发布消息，这里会block，因此不用担心对redis读取太快
    let undelivered_msg = conn
        .xread_options::<String, &str, StreamReadReply>(streams, &[">"], opts)
        .await?;
    consume_redis_message(conn, undelivered_msg, redis_task).await?;

    Ok(())
}

async fn consume_redis_message(
    conn: &mut ConnectionManager,
    reply: StreamReadReply,
    redis_task: &Arc<RedisTask>,
) -> Result<()> {
    for key in reply.keys {
        // 为空不处理，避免后续多余操作
        if key.ids.is_empty() {
            continue;
        }

        let tasks = key
            .ids
            .iter()
            .map(|id| consume_single_redis_message(Arc::clone(redis_task), id))
            .collect::<Vec<_>>();

        // 并行处理任务，加快处理速度
        iter(tasks).buffer_unordered(5).collect::<Vec<_>>().await;

        // 批量ack数据，提升效率
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

/// 处理单个Redis消息
///
/// 处理要求：
/// 1. 消息必须是String类型
/// 2. 消息必须能转换为utf-8字符串
///
/// 这样才会把对应的String交给 [`RedisTask`]中的`handler`来处理。
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
