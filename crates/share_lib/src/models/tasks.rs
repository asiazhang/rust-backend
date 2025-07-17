use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskInfo {
    pub title: String,
    pub description: Option<String>,
    pub command: String,
    pub author: String,
    pub ip: Option<String>,
}
