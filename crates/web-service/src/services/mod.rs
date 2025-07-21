//! 服务层模块
//!
//! 包含业务逻辑的服务层实现，遵循六边形架构原则

pub mod project;
pub mod traits;

pub use project::ProjectService;
pub use traits::ProjectServiceTrait;
