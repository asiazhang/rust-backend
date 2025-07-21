//! 数据库仓库 trait 定义
//!
//! 这里定义了各种数据库仓库的抽象接口
//!
//! ## Repository Trait 设计模式 🎯
//!
//! 所有 Repository trait 都应该遵循统一的设计模式，实现以下 trait 约束：
//!
//! ```rust
//! pub trait XxxRepositoryTrait: Send + Sync + Clone + 'static {
//!     // 异步方法定义...
//! }
//! ```
//!
//! ### Trait 约束说明 📚
//!
//! #### `Send` trait 🚀
//! - **作用**：表示类型可以安全地在线程间转移所有权
//! - **必要性**：异步方法返回的 `Future` 需要在不同线程间传递
//! - **场景**：Web 服务器中，不同的请求可能在不同线程处理
//!
//! #### `Sync` trait 🔄
//! - **作用**：表示类型可以安全地在多个线程间共享引用
//! - **必要性**：Repository 实例通常作为共享服务在应用中使用
//! - **场景**：多个并发请求同时访问同一个 Repository 实例
//!
//! #### `Clone` trait 📋
//! - **作用**：允许创建类型的副本
//! - **必要性**：在依赖注入系统中，需要克隆 Repository 实例传递给不同服务层
//! - **场景**：每个请求处理器都需要自己的 Repository 实例副本
//!
//! #### `'static` 生命周期 ⏰
//! - **作用**：表示类型不包含非静态引用，可以在程序整个生命周期中存活
//! - **必要性**：
//!   - 异步 trait 方法返回的 `Future` 需要 `'static` 生命周期
//!   - 依赖注入容器通常要求服务具有 `'static` 生命周期
//! - **场景**：作为应用服务长期运行，不依赖于短期引用
//!
//! ### 实际应用场景 💡
//!
//! 这些 trait 组合使得 Repository trait 可以：
//!
//! ```rust
//! // 1. Policy Based Design - 使用泛型而非 trait object
//! // 更高效，零成本抽象，编译时优化
//! #[derive(Clone)]
//! struct AppState<P: ProjectRepositoryTrait, U: UserRepositoryTrait> {
//!     project_repo: P,
//!     user_repo: U,
//! }
//!
//! // 2. Axum 共享数据模式 - 使用泛型参数
//! async fn create_project<P: ProjectRepositoryTrait>(
//!     State(app_state): State<AppState<P>>,
//!     Json(payload): Json<ProjectCreate>,
//! ) -> Result<Json<ProjectInfo>, ApiError> {
//!     let project = app_state.project_repo.create_project(payload).await?;
//!     Ok(Json(project))
//! }
//!
//! // 3. 应用启动时的状态初始化
//! let app_state = AppState {
//!     project_repo: PostgresProjectRepository::new(pool.clone()),
//!     user_repo: PostgresUserRepository::new(pool.clone()),
//! };
//!
//! let app = Router::new()
//!     .route("/projects", post(create_project::<PostgresProjectRepository>))
//!     .with_state(app_state);
//!
//! // 4. 并发访问场景 (Send + Sync)
//! async fn batch_process<P: ProjectRepositoryTrait>(repo: P) {
//!     let handles: Vec<_> = (0..10).map(|i| {
//!         let repo_clone = repo.clone();
//!         tokio::spawn(async move {
//!             repo_clone.get_project_by_id(i).await
//!         })
//!     }).collect();
//!     
//!     for handle in handles {
//!         let _ = handle.await;
//!     }
//! }
//! ```
//!
//! ### 最佳实践 ✅
//!
//! 1. **统一约束**：所有 Repository trait 都应该使用相同的 trait 约束
//! 2. **异步优先**：所有数据库操作方法都应该是异步的
//! 3. **错误处理**：使用统一的 `DatabaseResult<T>` 类型进行错误处理
//! 4. **文档完善**：为每个方法提供详细的文档说明
//! 5. **接口简洁**：保持接口简洁，专注于数据访问逻辑
//!
//! 这是现代 Rust 异步 Web 开发中的标准模式！🎉

pub mod project;

// 重新导出
pub use project::ProjectRepositoryTrait;
