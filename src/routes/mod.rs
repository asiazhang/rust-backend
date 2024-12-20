//! 路由入口
//!
//! 提供 [`routers`] 函数，导出当前App的所有路由。
//!
//! 用户可以在导出路由时传入共享数据 shared_state，这样所有路由函数都可以访问。

use crate::models::app::AppState;
use crate::routes::projects::__path_create_project;
use crate::routes::projects::__path_delete_project;
use crate::routes::projects::__path_find_projects;
use crate::routes::projects::__path_get_project;
use crate::routes::projects::__path_update_project;
use crate::routes::projects::{
    create_project, delete_project, find_projects, get_project, update_project,
};
use crate::routes::users::__path_create_user;
use crate::routes::users::__path_delete_user;
use crate::routes::users::__path_find_users;
use crate::routes::users::__path_get_user;
use crate::routes::users::__path_update_user;
use crate::routes::users::{create_user, delete_user, find_users, get_user, update_user};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub mod projects;
pub mod users;

/// 导出当前App的所有路由
///
/// ## 参数定义
/// - state: 共享数据，参考 [`AppState`] 定义。一般存放数据库连接池之类的全局共享数据。
///
/// ## **❗️注意事项：**
///
/// 由于 [`routes!`] 宏限制，在同一个宏里面不能同时定义多个相同类型的http接口。
/// 不能这样定义：
///
/// ```rust
/// routes!(get, get, post)
/// ```
///
/// 这样会导致Panic
///
/// 需要拆开定义
///
/// ```rust
/// routes!(get, post)
/// .routes!(get)
/// ```
///
pub(super) fn routers(state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(find_projects))
        .routes(routes!(
            get_project,
            create_project,
            update_project,
            delete_project
        ))
        .routes(routes!(find_users))
        .routes(routes!(get_user, create_user, update_user, delete_user))
        .with_state(state)
}
