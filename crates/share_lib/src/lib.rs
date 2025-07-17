//! 🔧 共享库模块
//! 
//! 这个模块包含了在多个服务之间共享的通用代码，包括：
//! - 错误类型
//! - 公共数据结构
//! - Redis 模型和常量

pub mod error;
pub mod models;

// 重新导出常用类型
pub use error::{Result, SharedError};
pub use models::*;
