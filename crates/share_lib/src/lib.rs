//! 🔧 共享库模块
//! 
//! 这个模块包含了在多个服务之间共享的通用代码，包括：
//! - 数据库操作
//! - 缓存操作
//! - 工具函数
//! - 错误类型
//! - 公共数据结构

pub mod cache;
pub mod database;
pub mod error;
pub mod models;
pub mod utils;

// 重新导出常用类型
pub use error::{Result, SharedError};
pub use models::*;
