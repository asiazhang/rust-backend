use sqlx::PgPool;

/// App共享数据
/// 
/// 方便跨线程在多个axum handler中使用。
/// 一般用于访问线程池/全局配置
/// 
/// 
/// 
pub struct AppState {
    pub db_pool: PgPool,
}
