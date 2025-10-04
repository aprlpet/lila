use std::collections::HashSet;

use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
};
use chrono::Utc;
use serde::Deserialize;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::{ListObjectsResponse, ObjectInfo, ObjectMetadata, SearchResponse},
    storage::{FileStorage, MetadataStore},
};

#[derive(Clone)]
pub struct AppState {
    pub metadata: MetadataStore,
    pub storage: FileStorage,
    pub auth_token: String,
    pub max_upload_size: usize,
}

#[derive(Deserialize)]
pub struct ListQuery {
    prefix: Option<String>,
    limit: Option<i64>,
    delimiter: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    key: Option<String>,
    content_type: Option<String>,
    min_size: Option<i64>,
    max_size: Option<i64>,
    limit: Option<i64>,
}

pub async fn put_object(
    State(state): State<AppState>,
    Path(key): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<ObjectMetadata>> {
    tracing::info!("PUT request for object: {}", key);

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    tracing::debug!("Content-Type: {}", content_type);

    let max_size = state.max_upload_size * 1024 * 1024;
    let stream = body.into_data_stream();

    let (etag, size) = state.storage.write_stream(&key, stream, max_size).await?;

    tracing::debug!("File written with ETag: {}, size: {} bytes", etag, size);

    let metadata = ObjectMetadata {
        id: Uuid::new_v4().to_string(),
        key: key.clone(),
        size,
        content_type,
        etag,
        created_at: Utc::now(),
    };

    state.metadata.insert(&metadata).await?;
    tracing::info!("Object {} stored successfully", key);

    Ok(Json(metadata))
}

pub async fn get_object(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Response> {
    tracing::info!("GET request for object: {}", key);

    let metadata = state
        .metadata
        .get(&key)
        .await?
        .ok_or_else(|| AppError::NotFound(key.clone()))?;

    tracing::debug!("Found metadata for {}: {} bytes", key, metadata.size);

    let file = state.storage.open(&key).await?;
    tracing::debug!("Opened file for streaming");

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .header("content-type", metadata.content_type)
        .header("etag", metadata.etag)
        .header("content-length", metadata.size.to_string())
        .body(body)
        .unwrap();

    tracing::info!("Object {} streaming started", key);
    Ok(response)
}

pub async fn get_object_metadata(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ObjectMetadata>> {
    tracing::info!("HEAD request for object: {}", key);

    let metadata = state
        .metadata
        .get(&key)
        .await?
        .ok_or_else(|| AppError::NotFound(key.clone()))?;

    tracing::debug!("Found metadata for {}", key);
    Ok(Json(metadata))
}

pub async fn list_objects(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ListObjectsResponse>> {
    tracing::info!("LIST request with prefix: {:?}", params.prefix);

    let objects = state
        .metadata
        .list(params.prefix.as_deref(), params.limit)
        .await?;

    let delimiter = params.delimiter.unwrap_or_else(|| "/".to_string());
    let prefix = params.prefix.as_deref().unwrap_or("");

    let mut prefixes = HashSet::new();
    let mut filtered_objects = Vec::new();

    for obj in objects {
        if let Some(rest) = obj.key.strip_prefix(prefix) {
            if let Some(idx) = rest.find(&delimiter) {
                let folder = format!("{}{}{}", prefix, &rest[..idx], delimiter);
                prefixes.insert(folder);
            } else {
                filtered_objects.push(obj);
            }
        }
    }

    let total = filtered_objects.len();
    let mut prefix_vec: Vec<String> = prefixes.into_iter().collect();
    prefix_vec.sort();

    tracing::info!("Found {} objects and {} prefixes", total, prefix_vec.len());

    Ok(Json(ListObjectsResponse {
        objects: filtered_objects,
        total,
        prefixes: prefix_vec,
    }))
}

pub async fn search_objects(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    tracing::info!(
        "SEARCH request with params: key={:?}, content_type={:?}, min_size={:?}, max_size={:?}",
        params.key,
        params.content_type,
        params.min_size,
        params.max_size
    );

    let objects = state
        .metadata
        .search(
            params.key.as_deref(),
            params.content_type.as_deref(),
            params.min_size,
            params.max_size,
            params.limit,
        )
        .await?;

    let total = objects.len();

    tracing::info!("Found {} objects matching search criteria", total);

    Ok(Json(SearchResponse { objects, total }))
}

pub async fn delete_object(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!("DELETE request for object: {}", key);

    state.storage.delete(&key).await?;
    tracing::debug!("File deleted from storage");

    let deleted = state.metadata.delete(&key).await?;

    if !deleted {
        tracing::warn!("Metadata for {} not found", key);
        return Err(AppError::NotFound(key));
    }

    tracing::info!("Object {} deleted successfully", key);
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn delete_folder(
    State(state): State<AppState>,
    Path(prefix): Path<String>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!("DELETE folder request for prefix: {}", prefix);

    let prefix = if !prefix.ends_with('/') {
        format!("{}/", prefix)
    } else {
        prefix
    };

    let objects = state.metadata.list(Some(&prefix), None).await?;

    for obj in &objects {
        state.storage.delete(&obj.key).await?;
    }

    let deleted = state.metadata.delete_by_prefix(&prefix).await?;

    tracing::info!("Deleted {} objects with prefix {}", deleted, prefix);
    Ok(Json(serde_json::json!({
        "success": true,
        "deleted": deleted
    })))
}

pub async fn get_object_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ObjectInfo>> {
    tracing::info!("INFO request for object: {}", key);

    let metadata = state
        .metadata
        .get(&key)
        .await?
        .ok_or_else(|| AppError::NotFound(key.clone()))?;

    let path = state.storage.get_object_path_string(&key);

    Ok(Json(ObjectInfo { metadata, path }))
}
