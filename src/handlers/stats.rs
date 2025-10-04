use axum::{Json, extract::State};

use crate::{error::Result, handlers::objects::AppState, models::StatsResponse};

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>> {
    tracing::info!("GET request for stats");

    let (total_objects, total_size) = state.metadata.get_stats().await?;

    let stats = StatsResponse {
        total_objects,
        total_size,
        storage_path: state.storage.clone().base_path.display().to_string(),
    };

    tracing::debug!("Stats: {} objects, {} bytes", total_objects, total_size);

    Ok(Json(stats))
}
