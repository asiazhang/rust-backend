use crate::{DatabaseError, DatabaseResult};
use share_lib::models::config::AppConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// 数据库连接池
pub type DatabasePool = Pool<Postgres>;

/// 创建数据库连接池并执行迁移（一站式函数）
pub async fn initialize_database(config: Arc<AppConfig>) -> DatabaseResult<DatabasePool> {
    // 创建数据库连接池
    // 使用默认配置，如果有调整需要可参考sqlx文档
    // 注意：pool已经是一个智能指针了，所以可以使用.clone()安全跨线程使用
    let pool = PgPoolOptions::new()
        // 启动预留，加快获取速度
        .min_connections(10)
        // 生产环境配置30~40即可
        .max_connections(40)
        .acquire_timeout(Duration::from_secs(3))
        // 1小时空闲则释放
        .idle_timeout(Duration::from_secs(3600))
        // 6小时强制释放，避免长时间链接导致数据库问题
        .max_lifetime(Duration::from_secs(3600 * 6))
        .test_before_acquire(true)
        .connect(&config.postgresql_conn_str)
        .await
        .map_err(|e| DatabaseError::connection(format!("连接PostgreSQL数据库失败: {e}")))?;

    info!("🗄️ 数据库连接池创建成功");

    // 执行数据库迁移
    info!("🔄 开始执行数据库迁移...");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .map_err(|e| DatabaseError::migration(format!("数据库迁移失败: {e}")))?;

    info!("✅ 数据库迁移完成");

    Ok(pool)
}
