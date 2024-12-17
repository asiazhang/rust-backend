use crate::routes::projects::{
    create_project, delete_project, find_projects, get_project, update_project,
};
use crate::routes::users::{create_user, delete_user, find_users, get_user, update_user};
use axum::routing::get;
use axum::Router;

mod routes;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route(
            "/projects",
            get(find_projects)
                .post(create_project)
                .delete(delete_project)
                .patch(update_project),
        )
        .route("/projects/:id", get(get_project))
        .route(
            "/users",
            get(find_users)
                .post(create_user)
                .delete(delete_user)
                .patch(update_user),
        )
        .route("/users/:id", get(get_user));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
