//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

pub mod task;

use crate::models::config::AppConfig;
use crate::models::redis_task::{RedisConsumerHeartBeat, RedisTask};
use crate::tasks::task::TaskCreator;
use color_eyre::eyre::Context;
use color_eyre::Result;
use deadpool_redis::{Config, Connection, Runtime};
use futures::future::try_join_all;
use futures::stream::iter;
use futures::StreamExt;
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
        consumer_name_template: "task_consumer".to_string(),
        handler: Box::new(TaskCreator),
    };

    // NOTE: 如果有其他需要处理的Redis类型，那么按照👆的例子来编写
    // 统一调用 `guard_start_create_task_consumers(app_config.clone(), xxx)`
    try_join!(guard_start_create_task_consumers(
        app_config.clone(),
        Arc::new(create_task_info)
    ))?;

    info!("Redis job consumers stopped");

    Ok(())
}

const GROUP_NAME: &str = "rust-backend";

/// 让redis消费者一直执行，直到收到shutdown信号
///
/// 只要是失败的场景，则一直不停重试，确保消费者不退出
async fn guard_start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: Arc<RedisTask>,
) -> Result<()> {
    loop {
        let re = start_create_task_consumers(app_config.clone(), redis_task.clone()).await;
        match re {
            Ok(_) => break, // OK表示收到shutdown信号，正常退出
            Err(err) => {
                warn!("Failed to start create task consumers, retrying...");
                warn!("{}", err);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

/// 并发启动新建任务的redis消费者
async fn start_create_task_consumers(
    app_config: Arc<AppConfig>,
    redis_task: Arc<RedisTask>,
) -> Result<()> {
    create_task_group(&redis_task).await?;

    // 根据配置数据，创建多个消费者。
    // 这几个消费者会并行从redis读取消息并消费。
    let consumers: Vec<_> = (0..app_config.redis.max_consumer_count)
        .map(|i| {
            let consumer_name = format!("{}_{}", redis_task.consumer_name_template, i);
            consumer_task_worker_with_heartbeat(redis_task.clone(), consumer_name)
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

async fn create_task_group(redis_task: &RedisTask) -> Result<()> {
    let mut con = redis_task
        .pool
        .get()
        .await
        .context("get redis connection from pool")?;

    // 创建消费组，来支持并行消费消息。
    // 由于消费组不能多次创建，因此失败了提示warning即可
    let re: RedisResult<()> = con
        .xgroup_create_mkstream(&redis_task.stream_name, GROUP_NAME, "$")
        .await;
    if let Err(err) = re {
        warn!("Failed to create redis task group {}: {}", GROUP_NAME, err);
    }

    Ok(())
}

async fn consumer_task_worker_with_heartbeat(
    redis_task: Arc<RedisTask>,
    consumer_name: String,
) -> Result<()> {
    _ = try_join!(
        consumer_task_send_heartbeat(redis_task.clone(), consumer_name.clone()),
        consumer_task_worker(redis_task.clone(), consumer_name.clone()),
    );

    Ok(())
}

async fn consumer_task_send_heartbeat(
    redis_task: Arc<RedisTask>,
    consumer_name: String,
) -> Result<()> {
    let mut redis_conn = redis_task.pool.get().await?;
    let mut shutdown_rx = redis_task.shutdown_rx.clone();
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let heartbeat_key = "rust_backend_consumers:heartbeat";

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
                    let res :Result<(), RedisError> = redis_conn.hset(heartbeat_key, &consumer_name, json_data).await;
                    if let Err(err) = res {
                        warn!("Consumer {} redis heartbeat error: {}", consumer_name, err);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn consumer_task_worker(redis_task: Arc<RedisTask>, consumer_name: String) -> Result<()> {
    debug!("Redis job consumer {} started", consumer_name);

    let mut redis_conn = redis_task.pool.get().await?;

    let opts = StreamReadOptions::default()
        .group(GROUP_NAME, &consumer_name)
        .block(1000) // 最长等待时间1秒，可以满足大多数场景
        .count(10); // 最多获取10个数据
    let streams = vec![redis_task.stream_name.clone()];

    // 这里必须要把shutdown_rx克隆一次
    // 否则在[`select!`]多个分支中，都会访问到 &mut redis_task
    // 这样会违反redis的借用原则（mut借用在作用域里面只能有一个）
    let mut shutdown_rx = redis_task.shutdown_rx.clone();

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
          result = xread_group(&mut redis_conn,&streams,&opts,&redis_task) => {
              match result {
                  Ok(_) => {}
                  Err(err) => {
                        warn!("{} xread group failed, err: {}, reconnecting...", consumer_name, err);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        match redis_task.pool.get().await {
                            Ok(conn) => redis_conn = conn,
                            Err(err) => {
                                warn!("{} get redis conn from pool failed, err: {}, reconnecting...", consumer_name,err)
                            }
                        }
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
    conn: &mut Connection,
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
    conn: &mut Connection,
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
            .map(|id| consume_single_redis_message(redis_task.clone(), id))
            .collect::<Vec<_>>();

        // 并行处理任务，加快处理速度
        iter(tasks).buffer_unordered(5).collect::<Vec<_>>().await;

        // 批量ack数据，提升效率
        let xack_ret: Result<(), RedisError> = conn
            .xack(
                &redis_task.stream_name,
                GROUP_NAME,
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
