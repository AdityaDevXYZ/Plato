use axum::{Json, extract::{State, Path}, http::StatusCode};
use crate::state::AppState;
use uuid::Uuid;
use twin::model::{Satellite, PartialSatelliteTelemetry};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct RegisterSatelliteReq {
    pub id: Option<Uuid>,
    pub mission_id: Uuid,
    pub name: String,
    pub norad_id: Option<u32>,
    pub software_version: Option<String>,
}

#[tracing::instrument(skip(state))]
pub async fn register_satellite(
    State(state): State<AppState>,
    Json(payload): Json<RegisterSatelliteReq>,
) -> Result<(StatusCode, Json<Satellite>), StatusCode> {
    let id = payload.id.unwrap_or_else(Uuid::new_v4);
    let software_version = payload.software_version.unwrap_or_else(|| "v1.0.0".into());
    let sat = state.twin_update.register_satellite(
        id,
        payload.mission_id.into(),
        payload.name,
        payload.norad_id,
        software_version,
    ).await.map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((StatusCode::CREATED, Json(sat)))
}

#[tracing::instrument(skip(state))]
pub async fn ingest_telemetry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PartialSatelliteTelemetry>,
) -> Result<Json<Satellite>, StatusCode> {
    let sat = state.twin_update.update_telemetry(id, payload).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(sat))
}

#[tracing::instrument(skip(state))]
pub async fn list_satellites(State(state): State<AppState>) -> Result<Json<Vec<Satellite>>, StatusCode> {
    let sats = state.twin_query.get_all_satellites().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sats))
}

#[tracing::instrument(skip(state))]
pub async fn get_satellite(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Satellite>, StatusCode> {
    let sat = state.twin_query.get_satellite(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(sat))
}

#[tracing::instrument(skip(state))]
pub async fn get_mission_assets(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Vec<Satellite>>, StatusCode> {
    let sats = state.twin_query.get_mission_assets(id.into()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sats))
}

#[tracing::instrument(skip(state))]
pub async fn get_unhealthy(State(state): State<AppState>) -> Result<Json<Vec<Satellite>>, StatusCode> {
    let sats = state.twin_query.get_unhealthy_satellites().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sats))
}
