use anyhow::{Context, Result};
use std::sync::Arc;

pub struct RedisConfig {
    pub max_consumer_count: usize,
}

pub struct AppConfig {
    pub postgresql_addr: String,
    pub redis_addr: String,
    pub max_redis_concurrency: usize,
    pub redis: RedisConfig,
}

impl AppConfig {
    pub fn load() -> Result<Arc<AppConfig>> {
        // 加载.env文件中的数据注入到环境变量中，方便本地测试
        // 线上环境部署时会直接使用环境变量，不需要.env文件
        dotenvy::dotenv()?;

        // 读取数据库地址信息（仅支持postgresql）
        let db_url =
            std::env::var("DATABASE_URL").context("Can not load DATABASE_URL in environment")?;

        let redis_url =
            std::env::var("REDIS_URL").context("Can not load REDIS_URL in environment")?;

        let config = AppConfig {
            postgresql_addr: db_url,
            redis_addr: redis_url,
            max_redis_concurrency: 16,
            redis: RedisConfig {
                max_consumer_count: 5,
            },
        };
        Ok(Arc::new(config))
    }
}
