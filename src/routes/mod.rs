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
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub mod users;

pub mod projects;

pub(super) fn routers() -> OpenApiRouter {
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
}
