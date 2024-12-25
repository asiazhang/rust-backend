//! 使用Rust来开发Web后端
//!
//! 包含以下功能：
//!
//! - 对外提供的`json-api`
//! - 可视化的文档
//! - 异步消息处理器(`redis`)
//! - `cron`任务处理器(定时任务)
//!
//! 所有代码都放在一个程序中，方便部署和维护(适用于小型系统)

use crate::crons::start_cron_tasks;
use crate::models::config::AppConfig;
use crate::routes::start_axum_server;
use crate::tasks::start_job_consumers;
use color_eyre::Result;
use tokio::try_join;
use tracing::info;

mod crons;
mod models;
mod routes;
mod tasks;

/// 入口函数
///
/// - 使用tokio作为异步运行时，因此需要增加 `#[tokio::main]`
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    // 使用tracing作为日志记录器
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 加载配置数据（从环境变量或者本地的.env文件）
    let conf = AppConfig::load()?;

    // 优雅退出通知机制，通过watch来通知需要感知的协程优雅退出
    let (tx, rx) = tokio::sync::watch::channel(false);

    // 如果有任何一个服务启动失败，那么应该会退出并打印错误信息
    _ = try_join!(
        // 启动web-api服务
        start_axum_server(conf.clone(), tx),
        // 启动redis-consumer服务
        start_job_consumers(conf.clone(), rx.clone()),
        // 启动cron-jobs服务
        start_cron_tasks(rx.clone())
    )?;
    
    info!("rust backend exit successfully");

    Ok(())
}
