/// Search Projects by query params.
///
/// Search `Project` by query params and return matching `Project`.
#[utoipa::path(post, path = "/search-projects")]
pub async fn find_projects() {}

#[utoipa::path(post, path = "/projects")]
pub async fn create_project() {}

#[utoipa::path(get, path = "/projects/{id}")]
pub async fn get_project() {}

#[utoipa::path(patch, path = "/projects/{id}")]
pub async fn update_project() {}

#[utoipa::path(delete, path = "/projects/{id}")]
pub async fn delete_project() {}
