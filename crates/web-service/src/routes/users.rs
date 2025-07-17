#[utoipa::path(post, path = "/search-users", tag = "users")]
pub async fn find_users() {}

#[utoipa::path(get, path = "/users/{id}", tag = "users")]
pub async fn get_user() {}

#[utoipa::path(post, path = "/users", tag = "users")]
pub async fn create_user() {}

#[utoipa::path(patch, path = "/users/{id}", tag = "users")]
pub async fn update_user() {}

#[utoipa::path(delete, path = "/users/{id}", tag = "users")]
pub async fn delete_user() {}
