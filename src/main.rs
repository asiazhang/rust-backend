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

use color_eyre::eyre::Context;
use color_eyre::Result;
use consumer_service::start_job_consumers;
use cronjob_service::start_cron_tasks;
use database::initialize_database;
use shared_lib::models::config::AppConfig;
use std::sync::Arc;
use tokio::sync::watch::Sender;
use tokio::{signal, try_join};
use tracing::info;
use web_service::start_web_service;

/// 入口函数
///
/// - 使用tokio作为异步运行时，因此需要增加 `#[tokio::main]`
#[tokio::main]
async fn main() -> Result<()> {
    // 安装错误提示器
    color_eyre::install()?;

    // 使用tracing作为日志记录器
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    // 加载配置数据（从环境变量或者本地的.env文件）
    let conf = AppConfig::load()?;

    let pool = initialize_database(Arc::clone(&conf))
        .await
        .context("Failed to initialize database")?;

    // 优雅退出通知机制，通过watch来通知需要感知的协程优雅退出
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // 如果有任何一个服务启动失败，那么应该会退出并打印错误信息
    _ = try_join!(
        start_shutdown_signal(shutdown_tx),
        // 启动web-api服务
        start_web_service(pool, shutdown_rx.clone()),
        // 启动redis-consumer服务
        start_job_consumers(Arc::clone(&conf), shutdown_rx.clone()),
        // 启动cron-jobs服务
        start_cron_tasks(Arc::clone(&conf), shutdown_rx.clone()),
    )?;

    info!("rust backend exit successfully");

    Ok(())
}

/// 发送退出信号
///
/// 退出场景包括：
/// - 用户在控制台发送ctrl+c信号（全平台支持）
/// - SIGTerm: Unix系统下接受到的结束信号
///
/// 发送退出信号，让整个系统处理完当前业务，尽快退出。避免直接结束进程可能导致的：
/// - 脏数据
/// - 系统资源长时间占用
/// - 跟其他系统对接导致其他系统处理异常
/// - 多余的错误日志（计划内重启/停机）
async fn start_shutdown_signal(shutdown_tx: Sender<bool>) -> Result<()> {
    // 监听ctrl+c信号
    let ctrl_c = async { signal::ctrl_c().await.context("failed to install Ctrl+C handler") };

    // unix下同时监听SIGTERM信号
    #[cfg(unix)]
    let terminate = async {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).context("Failed to install SIGTERM handler")?;
        Ok::<_, color_eyre::Report>(sigterm.recv().await)
    };

    // windows下没有SIGTERM，忽略
    #[cfg(not(unix))]
    let terminate = std::future::pending::<Result<Option<()>>>();

    // 只要有监听到任何退出信号，就结束select!监听
    tokio::select! {
        result = ctrl_c => {result?; info!("Received Ctrl+C, initiating shutdown...");},
        result = terminate => {result?; info!("Received SIGTERM, initiating shutdown...");},
    }

    // 发送关闭信号，通知其他模块退出
    shutdown_tx.send(true).context("Failed to send shutdown signal")?;

    Ok(())
}
