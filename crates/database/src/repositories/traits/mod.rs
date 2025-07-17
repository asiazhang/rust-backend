//! 数据库仓库 trait 定义
//!
//! 这里定义了各种数据库仓库的抽象接口

pub mod project;

// 重新导出
pub use project::ProjectRepositoryTrait;
