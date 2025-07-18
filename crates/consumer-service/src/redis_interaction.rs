//! 📡 Redis 交互模块
//!
//! 此模块提供了完整的 Redis 流（Stream）消费者功能实现，包括：
//! - 🔗 Redis 连接管理
//! - 📊 消费者组管理
//! - 💓 心跳机制
//! - 📝 消息处理
//! - 🛑 优雅关闭
//!
//! 该模块的核心功能是从 Redis 流中读取消息，并通过实现了 `RedisHandlerTrait` 的处理器来处理这些消息。

use crate::traits::RedisHandlerTrait;
use color_eyre::Result;
use color_eyre::eyre::Context;
use futures::StreamExt;
use futures::stream::iter;
use redis::aio::ConnectionManager;
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, RedisResult, Value};
use shared_lib::models::redis_constants::{CONSUMER_GROUP_NAME, CONSUMER_HEARTBEAT_KEY, HEARTBEAT_INTERVAL_SECONDS};
use shared_lib::models::redis_task::RedisConsumerHeartBeat;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{debug, error, trace, warn};

/// 🔗 创建 Redis 连接管理器
/// 
/// 这个函数用于创建一个异步的 Redis 连接管理器，它能够自动管理连接池。
/// 连接管理器会在连接断开时自动重连，确保连接的稳定性。
/// 
/// # 参数
/// 
/// * `conn_str` - Redis 连接字符串，格式通常为 `redis://host:port`
/// 
/// # 返回值
/// 
/// 返回一个 `Result<ConnectionManager>`，成功时包含连接管理器实例
/// 
/// # 错误
/// 
/// 如果连接字符串无效或连接失败，将返回相应的错误
pub async fn new_redis_connection_manager(conn_str: &str) -> Result<ConnectionManager> {
    Ok(ConnectionManager::new(redis::Client::open(conn_str)?).await?)
}

/// 📊 创建 Redis 消费者组
/// 
/// 这个函数用于创建一个 Redis 流的消费者组，如果流不存在则会自动创建。
/// 消费者组是 Redis 流的核心概念，允许多个消费者协同处理消息。
/// 
/// # 参数
/// 
/// * `conn_str` - Redis 连接字符串
/// * `redis_task` - 实现了 `RedisHandlerTrait` 的任务处理器，用于获取流名称
/// 
/// # 行为
/// 
/// - 使用 `XGROUP CREATE` 命令创建消费者组
/// - 如果流不存在，会自动创建（通过 `mkstream` 选项）
/// - 消费者组从流的末尾开始读取（使用 `"$"` 参数）
/// - 如果消费者组已经存在，会记录警告但不会返回错误
/// 
/// # 返回值
/// 
/// 返回 `Result<()>`，即使创建失败也会返回 `Ok(())`
pub async fn create_task_group<T: RedisHandlerTrait>(conn_str: String, redis_task: Arc<T>) -> Result<()> {
    let conn = new_redis_connection_manager(&conn_str).await?;

    let re: RedisResult<()> = conn
        .clone()
        .xgroup_create_mkstream(redis_task.stream_name(), CONSUMER_GROUP_NAME, "$")
        .await;
    if let Err(err) = re {
        warn!("Failed to create redis task group {}: {}", CONSUMER_GROUP_NAME, err);
    }

    Ok(())
}

/// 🚀 启动带心跳的 Redis 任务消费者
/// 
/// 这是一个高级函数，同时启动消息消费者和心跳发送器。
/// 它使用 `tokio::try_join!` 确保两个任务并发运行，如果其中一个失败，另一个也会停止。
/// 
/// # 参数
/// 
/// * `conn_str` - Redis 连接字符串
/// * `redis_task` - 实现了 `RedisHandlerTrait` 的任务处理器
/// * `consumer_name` - 消费者的唯一名称，用于标识和心跳
/// * `shutdown_rx` - 用于接收关闭信号的接收器
/// 
/// # 并发任务
/// 
/// 1. **消息消费者** (`consumer_task_worker`): 从 Redis 流中读取并处理消息
/// 2. **心跳发送器** (`consumer_task_send_heartbeat`): 定期发送心跳以表明消费者仍在运行
/// 
/// # 优雅关闭
/// 
/// 当 `shutdown_rx` 接收到关闭信号时，两个任务都会优雅地停止
/// 
/// # 返回值
/// 
/// 返回 `Result<()>`，如果任一任务失败，整个函数都会失败
pub async fn consumer_task_worker_with_heartbeat<T: RedisHandlerTrait>(
    conn_str: String,
    redis_task: Arc<T>,
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

/// 📥 从 Redis 流读取组消息
/// 
/// 该函数用途是从 Redis 流中读取未处理和未传递的消息。
/// 它分为两个阶段：首先处理待处理消息，然后处理新的未传递消息。
/// 
/// # 参数
/// 
/// * `conn` - Redis 连接管理器的可变引用
/// * `streams` - 要读取消息的 Redis 流数组
/// * `opts` - `StreamReadOptions`，用于定制读取行为
/// * `redis_task` - 消息处理器，用于处理读取的消息
/// 
/// # 流程
/// 
/// 1. 🔄 从指定的流中读取待处理消息（偏移量为 "0"），然后调用 `consume_redis_message` 处理
/// 2. 🆕 从流中读取未发送的消息（偏移量为 ">"），然后调用 `consume_redis_message` 处理
/// 
/// # 返回值
/// 
/// 返回 `Result<()>`，表示操作状态
pub async fn xread_group<T: RedisHandlerTrait>(
    conn: &mut ConnectionManager,
    streams: &[String],
    opts: &StreamReadOptions,
    redis_task: &Arc<T>,
) -> Result<()> {
    let pending_msg = conn.xread_options::<String, &str, StreamReadReply>(streams, &["0"], opts).await?;
    consume_redis_message(conn, pending_msg, redis_task).await?;

    let undelivered_msg = conn.xread_options::<String, &str, StreamReadReply>(streams, &[">"], opts).await?;
    consume_redis_message(conn, undelivered_msg, redis_task).await?;

    Ok(())
}

/// 📝 处理从 Redis 流读取的消息
/// 
/// 这个函数负责处理从 Redis 读取的流消息，使用并发方式处理多个消息，并在处理完成后确认消息。
/// 
/// # 参数
/// 
/// * `conn` - Redis 连接管理器的可变引用，用于确认（acknowledge）消息
/// * `reply` - 包含了读取消息的流应答
/// * `redis_task` - 消息处理器，用于实际处理每一条消息
/// 
/// # 流程
/// 
/// 1. 🔄 对每个流的键遍历消息 ID
/// 2. 🚀 对每个消息 ID，在并发中调用 `consume_single_redis_message` 处理
/// 3. 📊 使用 `buffer_unordered(5)` 并发处理最多 5 条消息
/// 4. ✅ 在所有消息处理完成后，调用 `xack` 批量确认消息
/// 5. ⚠️ 如果确认失败，记录错误但不中断流程
/// 
/// # 性能特性
/// 
/// - 并发处理最多 5 条消息提高处理效率
/// - 使用批量确认减少 Redis 网络开销
/// 
/// # 返回值
/// 
/// 返回 `Result<()>`，表示操作状态
pub async fn consume_redis_message<T: RedisHandlerTrait>(
    conn: &mut ConnectionManager,
    reply: StreamReadReply,
    redis_task: &Arc<T>,
) -> Result<()> {
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
                redis_task.stream_name(),
                CONSUMER_GROUP_NAME,
                &key.ids.iter().map(|it| &it.id).collect::<Vec<_>>(),
            )
            .await;

        if let Err(err) = xack_ret {
            error!(
                "xack batch consumer redis message from stream {} failed, err = {}",
                &redis_task.stream_name(),
                err
            )
        }
    }

    Ok(())
}

/// 💡 处理单条 Redis 流消息
/// 
/// 此函数用于从流的消息 ID 中提取并处理实际消息。
/// 它会尝试从流中提取 "message" 字段，并将其传递给处理器。
/// 
/// # 参数
/// 
/// * `redis_task` - 消息处理器，用于处理提取的消息
/// * `stream_id` - 包含消息 ID 和对应消息数据的结构
/// 
/// # 流程
/// 
/// 1. 🔍 尝试从 `stream_id.map` 中提取 `"message"` 键对应的值
/// 2. 🌍 如果获取成功，将消息从字节数组转换为 UTF-8 字符串
/// 3. 🛠️ 调用 `redis_task.handle_task` 异步处理消息
/// 4. ⚠️ 如果任何步骤失败，记录警告或错误日志
/// 
/// # 错误处理
/// 
/// - 如果找不到 "message" 字段，记录警告
/// - 如果数据不是有效的 UTF-8 字符串，记录警告
/// - 如果消息处理失败，记录错误
async fn consume_single_redis_message<T: RedisHandlerTrait>(redis_task: Arc<T>, stream_id: &StreamId) {
    if let Some(Value::BulkString(data)) = stream_id.map.get("message") {
        if let Ok(raw) = String::from_utf8(data.to_vec()) {
            if let Err(err) = redis_task.handle_task(raw).await {
                error!("failed to handle redis message: {}", err);
            }
        } else {
            warn!("stream id {} format is not a string", stream_id.id);
        }
    } else {
        warn!("stream id {} not found", stream_id.id);
    }
}

/// 🏃‍♂️ Redis 消息消费者工作器
/// 
/// 这个异步函数持续地从 Redis 流中读取消息并调用处理器处理，每秒阻塞读取，轮询新消息。
/// 它实现了一个带有优雅关闭机制的消费者工作循环。
/// 
/// # 参数
/// 
/// * `conn` - Redis 连接管理器，用于与 Redis 服务器通信
/// * `redis_task` - 消息处理器，用于处理读取的消息
/// * `consumer_name` - 用于标识消费者的唯一名称
/// * `shutdown_rx` - 可以发出关闭信号的接收器
/// 
/// # 流程
/// 
/// 1. 🔧 初始化读取选项，设置消费者组、阻塞时间(1秒)和最大读取计数(10)
/// 2. 🔄 在主循环中，等待 `shutdown_rx` 来决定是否关闭
/// 3. 📡 如果没有关闭信号，通过 `xread_group` 从流中消费消息
/// 4. 🔄 如果读取失败，休眠 5 秒并重试
/// 5. 📝 记录开始与结束日志
/// 
/// # 配置参数
/// 
/// - **阻塞时间**: 1000ms (1秒) - 如果没有消息可读，最多等待 1 秒
/// - **批量大小**: 10 - 每次最多读取 10 条消息
/// - **重试间隔**: 5 秒 - 连接失败后等待 5 秒再重试
/// 
/// # 优雅关闭
/// 
/// 使用 `tokio::select!` 监听关闭信号，确保能够及时响应停止请求
/// 
/// # 返回值
/// 
/// 返回 `Result<()>`，表示操作状态
async fn consumer_task_worker<T: RedisHandlerTrait>(
    mut conn: ConnectionManager,
    redis_task: Arc<T>,
    consumer_name: String,
    shutdown_rx: Receiver<bool>,
) -> Result<()> {
    debug!("Redis job consumer {} started", consumer_name);

    let opts = StreamReadOptions::default()
        .group(CONSUMER_GROUP_NAME, &consumer_name)
        .block(1000)
        .count(10);
    let streams = vec![redis_task.stream_name().to_string()];

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

/// 💓 发送消费者心跳
/// 
/// 此函数定期向 Redis 发送消费者心跳信号，以表明消费者正在正常运行。
/// 这对于系统监控和高可用性非常重要。
/// 
/// # 参数
/// 
/// * `conn` - Redis 连接管理器，用于向 Redis 写入心跳数据
/// * `redis_task` - 消息处理器，提供流名称上下文
/// * `consumer_name` - 消费者的唯一名称，作为心跳数据的标识符
/// * `shutdown_rx` - 接收关闭信号的接收器
/// 
/// # 流程
/// 
/// 1. ⏱️ 设置心跳发送的时间间隔，通过 `tokio::time::interval` 实现
/// 2. 🔄 在心跳间隔内等待关闭信号或定时器触发
/// 3. 📊 每次心跳时构建包含流名称、消费者名称和当前时间戳的心跳数据
/// 4. 📝 将心跳数据序列化为 JSON 字符串
/// 5. 💾 通过 `hset` 命令将心跳信息存储到 Redis 哈希表中
/// 6. ⚠️ 如果有错误发生，记录警告信息但不停止心跳
/// 
/// # 心跳数据结构
/// 
/// 心跳数据包含以下信息：
/// - `stream_name`: 消费者正在处理的流名称
/// - `consumer_name`: 消费者的唯一名称
/// - `last_heartbeat`: 最后一次心跳的 Unix 时间戳
/// 
/// # 优雅关闭
/// 
/// 使用 `tokio::select!` 监听关闭信号，确保能够及时响应停止请求
/// 
/// # 返回值
/// 
/// 返回 `Result<()>`，表示操作状态
async fn consumer_task_send_heartbeat<T: RedisHandlerTrait>(
    mut conn: ConnectionManager,
    redis_task: Arc<T>,
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
                    stream_name: redis_task.stream_name().to_string(),
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
