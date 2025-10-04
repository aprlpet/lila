use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub id: String,
    pub key: String,
    pub size: i64,
    pub content_type: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_objects: i64,
    pub total_size: i64,
    pub storage_path: String,
}

#[derive(Debug, Serialize)]
pub struct ListObjectsResponse {
    pub objects: Vec<ObjectMetadata>,
    pub total: usize,
    pub prefixes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ObjectInfo {
    pub metadata: ObjectMetadata,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub objects: Vec<ObjectMetadata>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub storage_path: String,
    pub database_url: String,
    pub auth_token: String,
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size_mb: usize,
}

fn default_max_upload_size() -> usize {
    100
}
