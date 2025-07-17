//! 数据库操作模块
//!
//! 这个模块提供了数据库连接、迁移、查询等功能

pub mod connection;
pub mod error;

pub use connection::*;
pub use error::DatabaseError;

/// 数据库操作结果类型
pub type DatabaseResult<T> = Result<T, DatabaseError>;
