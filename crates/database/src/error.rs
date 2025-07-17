use thiserror::Error;

/// 数据库操作错误类型
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// SQLX 错误
    #[error("数据库操作错误: {0}")]
    SqlxError(#[from] sqlx::Error),
    
    /// 连接错误
    #[error("数据库连接错误: {0}")]
    ConnectionError(String),
    
    /// 迁移错误
    #[error("数据库迁移错误: {0}")]
    MigrationError(String),
    
    /// 记录不存在错误
    #[error("记录不存在: {0}")]
    NotFound(String),
    
    /// 数据验证错误
    #[error("数据验证错误: {0}")]
    ValidationError(String),
    
    /// 通用错误
    #[error("数据库操作失败: {0}")]
    Generic(String),
}

impl DatabaseError {
    /// 创建连接错误
    pub fn connection<T: ToString>(msg: T) -> Self {
        Self::ConnectionError(msg.to_string())
    }
    
    /// 创建迁移错误
    pub fn migration<T: ToString>(msg: T) -> Self {
        Self::MigrationError(msg.to_string())
    }
    
    /// 创建记录不存在错误
    pub fn not_found<T: ToString>(msg: T) -> Self {
        Self::NotFound(msg.to_string())
    }
    
    /// 创建验证错误
    pub fn validation<T: ToString>(msg: T) -> Self {
        Self::ValidationError(msg.to_string())
    }
    
    /// 创建通用错误
    pub fn generic<T: ToString>(msg: T) -> Self {
        Self::Generic(msg.to_string())
    }
}
