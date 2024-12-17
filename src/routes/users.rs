#[utoipa::path(post, path = "/search-users")]
pub async fn find_users() {}

#[utoipa::path(get, path = "/users/:id")]
pub async fn get_user() {}

#[utoipa::path(post, path = "/users")]
pub async fn create_user() {}

#[utoipa::path(patch, path = "/users/:id")]
pub async fn update_user() {}

#[utoipa::path(delete, path = "/users/:id")]
pub async fn delete_user() {}
