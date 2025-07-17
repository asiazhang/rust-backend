//! 📋 错误处理模块
//! 
//! 定义了项目中使用的统一错误类型和处理方式

use thiserror::Error;

/// 共享库错误类型
#[derive(Error, Debug)]
pub enum SharedError {
    #[error("❌ 数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("❌ Redis错误: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("❌ 序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("❌ 验证错误: {0}")]
    Validation(#[from] validator::ValidationErrors),
    
    #[error("❌ 通用错误: {0}")]
    Generic(#[from] anyhow::Error),
    
    #[error("❌ 自定义错误: {0}")]
    Custom(String),
    
    #[error("❌ 未找到资源")]
    NotFound,
    
    #[error("❌ 权限不足")]
    Forbidden,
    
    #[error("❌ 无效参数: {0}")]
    InvalidInput(String),
}

/// 共享库的 Result 类型
pub type Result<T> = std::result::Result<T, SharedError>;

impl SharedError {
    /// 创建自定义错误
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        Self::Custom(msg.into())
    }
    
    /// 创建无效输入错误
    pub fn invalid_input<S: Into<String>>(msg: S) -> Self {
        Self::InvalidInput(msg.into())
    }
}
