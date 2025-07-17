//! 消费者服务模块
//!
//! 这个模块提供了消息队列消费的基础功能。

pub mod task_type_a;
pub mod task_type_b;
pub mod redis_interaction;

use self::task_type_a::TaskTypeACreator;
use self::task_type_b::TaskTypeBCreator;
use color_eyre::eyre::Context;
use color_eyre::Result;
use futures::future::try_join_all;
use crate::redis_interaction::{create_task_group, consumer_task_worker_with_heartbeat};
use shared_lib::models::config::AppConfig;
use shared_lib::models::redis_task::{RedisTask, RedisTaskCreator};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tokio::try_join;
use tracing::{info, warn};

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

