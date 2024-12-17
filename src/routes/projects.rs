/// Search Projects by query params.
///
/// Search `Project` by query params and return matching `Project`.
#[utoipa::path(
    get,
    path = "/projects",
    tag = "project",
    params(
    ),
    responses(
            (status = 200, description = "List matching projects by query", body = [String])
    )
)]
pub async fn find_projects() {}

pub async fn create_project() {}

pub async fn get_project() {}

pub async fn update_project() {}

pub async fn delete_project() {}
