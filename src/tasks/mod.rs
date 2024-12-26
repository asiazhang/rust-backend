//! 后台消费者服务
//!
//! 从Redis中读取消息并处理。通常来说都会正常ack，避免消息无限投递。

pub mod task;

use crate::models::config::AppConfig;
use crate::models::redis_task::RedisTask;
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
            consumer_task_worker(
                redis_task.clone(),
                format!("{}_{}", redis_task.consumer_name, i),
            )
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
        tokio::select! {
          _ = shutdown_rx.changed() => {
              if *shutdown_rx.borrow() {
                  break;
              }
          }
          result = xread_group(&mut redis_conn,&streams,&opts,&mut redis_task) => {
              match result {
                  Ok(_) => {}
                  Err(err) => {
                        warn!("{} xread group failed, err: {}, reconnecting...", consumer_name, err);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
    streams: &Vec<String>,
    opts: &StreamReadOptions,
    redis_task: &mut RedisTask,
) -> Result<()> {
    // 先处理pending数据
    let pending_msg = conn
        .xread_options::<String, &str, StreamReadReply>(streams, &["0"], &opts)
        .await?;
    consume_redis_message(conn, pending_msg, redis_task).await?;

    // 再处理未发布消息，这里会block，因此不用担心对redis读取太快
    let undelivered_msg = conn
        .xread_options::<String, &str, StreamReadReply>(streams, &[">"], &opts)
        .await?;
    consume_redis_message(conn, undelivered_msg, redis_task).await?;

    Ok(())
}

async fn consume_redis_message(
    conn: &mut Connection,
    reply: StreamReadReply,
    redis_task: &RedisTask,
) -> Result<()> {
    for key in reply.keys {
        // 为空不处理，避免后续多余操作
        if key.ids.is_empty() {
            continue;
        }

        let tasks = key
            .ids
            .iter()
            .map(|id| consume_single_redis_message(&redis_task, id))
            .collect::<Vec<_>>();

        iter(tasks).buffer_unordered(5).collect::<Vec<_>>().await;

        // 批量ack数据
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

async fn consume_single_redis_message(redis_task: &&RedisTask, stream_id: &StreamId) {
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
