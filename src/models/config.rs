use color_eyre::eyre::Context;
use color_eyre::{Help, Result};
use std::sync::Arc;

pub struct RedisConfig {
    /// redis链接字符串
    pub redis_conn_str: String,

    /// redis pool的大小
    /// 需要根据下面的max_consumer_count来配置
    /// 可通过环境变量 `MAX_REDIS_POOL_SIZE` 来调整
    pub max_redis_pool_size: usize,

    /// 每个类型的任务最多启动的consumer个数
    ///
    /// 例如：当前有A/B两种类型的consumer，如果这个值设置为5，那么最多启动5个A类型的消费者和5个B类型的消费者
    /// 最终需要的pool_size > 10
    ///
    /// 可通过环境变量 `MAX_CONSUMER_COUNT` 来调整
    pub max_consumer_count: usize,
}

/// 程序配置
pub struct AppConfig {
    /// postgresql数据库链接字符串
    pub postgresql_conn_str: String,

    /// redis配置
    pub redis: RedisConfig,
}

impl AppConfig {
    pub fn load() -> Result<Arc<AppConfig>> {
        // 加载.env文件中的数据注入到环境变量中，方便本地测试
        // 线上环境部署时会直接使用环境变量，不需要.env文件
        dotenvy::dotenv()?;

        // 读取数据库地址信息（仅支持postgresql）
        let db_url = std::env::var("DATABASE_URL")
            .context("Can not load DATABASE_URL in environment")
            .suggestion("设置 DATABASE_URL 环境变量")?;

        let redis_url = std::env::var("REDIS_URL")
            .context("Can not load REDIS_URL in environment")
            .suggestion("设置 REDIS_URL 环境变量")?;

        let config = AppConfig {
            postgresql_conn_str: db_url,
            redis: RedisConfig {
                max_consumer_count: std::env::var("MAX_CONSUMER_COUNT")
                    .map_or(5, |s| s.parse().unwrap_or(5)),
                redis_conn_str: redis_url,
                max_redis_pool_size: std::env::var("MAX_REDIS_POOL_SIZE")
                    .map_or(16, |s| s.parse().unwrap_or(16)),
            },
        };
        Ok(Arc::new(config))
    }
}
