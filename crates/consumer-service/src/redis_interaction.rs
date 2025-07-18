use color_eyre::eyre::Context;
use color_eyre::Result;
use futures::stream::iter;
use futures::StreamExt;
use redis::aio::ConnectionManager;
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, RedisError, RedisResult, Value};
use shared_lib::models::redis_constants::{CONSUMER_GROUP_NAME, CONSUMER_HEARTBEAT_KEY, HEARTBEAT_INTERVAL_SECONDS};
use shared_lib::models::redis_task::RedisConsumerHeartBeat;
use crate::traits::{RedisHandler, RedisTask};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{debug, error, trace, warn};

pub async fn new_redis_connection_manager(conn_str: &str) -> Result<ConnectionManager> {
    Ok(ConnectionManager::new(redis::Client::open(conn_str)?).await?)
}

pub async fn create_task_group<T: RedisHandler>(conn_str: String, redis_task: &RedisTask<T>) -> Result<()> {
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

pub async fn consumer_task_worker_with_heartbeat<T: RedisHandler>(
    conn_str: String,
    redis_task: Arc<RedisTask<T>>,
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

pub async fn xread_group<T: RedisHandler>(
    conn: &mut ConnectionManager,
    streams: &[String],
    opts: &StreamReadOptions,
    redis_task: &Arc<RedisTask<T>>,
) -> Result<()> {
    let pending_msg = conn.xread_options::<String, &str, StreamReadReply>(streams, &["0"], opts).await?;
    consume_redis_message(conn, pending_msg, redis_task).await?;

    let undelivered_msg = conn.xread_options::<String, &str, StreamReadReply>(streams, &[">"], opts).await?;
    consume_redis_message(conn, undelivered_msg, redis_task).await?;

    Ok(())
}

pub async fn consume_redis_message<T: RedisHandler>(conn: &mut ConnectionManager, reply: StreamReadReply, redis_task: &Arc<RedisTask<T>>) -> Result<()> {
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

async fn consume_single_redis_message<T: RedisHandler>(redis_task: Arc<RedisTask<T>>, stream_id: &StreamId) {
    if let Some(Value::BulkString(data)) = stream_id.map.get("message") {
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

async fn consumer_task_worker<T: RedisHandler>(
    mut conn: ConnectionManager,
    redis_task: Arc<RedisTask<T>>,
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

async fn consumer_task_send_heartbeat<T: RedisHandler>(
    mut conn: ConnectionManager,
    redis_task: Arc<RedisTask<T>>,
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

