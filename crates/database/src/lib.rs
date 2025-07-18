//! 数据库操作模块
//!
//! 这个模块提供了数据库连接、迁移、查询等功能

pub mod connection;
pub mod error;
pub mod models;
pub mod repositories;

pub use connection::{initialize_database, DatabasePool};
pub use error::DatabaseError;
pub use models::project::{ProjectCreate, ProjectInfo, ProjectSearchResult, ProjectUpdate};
pub use repositories::{project::ProjectRepository, traits::ProjectRepositoryTrait};

/// 数据库操作结果类型
pub type DatabaseResult<T> = Result<T, DatabaseError>;
