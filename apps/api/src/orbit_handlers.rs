use axum::{Json, extract::{State, Path, Query}, http::StatusCode};
use crate::state::AppState;
use uuid::Uuid;
use orbit::model::{
    SatelliteOrbit, OrbitalPrediction, CommunicationWindow, TleSet,
    LookAngles, GroundPass, GroundStation, NearbySatellite, FormationMember,
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PredictionQuery {
    pub minutes: Option<u32>,
    pub step: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct LookAnglesQuery {
    pub lat: f64,
    pub lon: f64,
    pub alt: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct PassesQuery {
    pub lat: f64,
    pub lon: f64,
    pub alt: Option<f64>,
    pub min_el: Option<f64>,
    pub hours: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct NearbyQuery {
    pub radius_km: Option<f64>,
}

#[tracing::instrument(skip(state))]
pub async fn get_orbit(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<SatelliteOrbit>, StatusCode> {
    let orbit = state.orbit_query.get_current_position(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(orbit))
}

#[tracing::instrument(skip(state))]
pub async fn set_tle(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TleSet>,
) -> Result<(StatusCode, Json<SatelliteOrbit>), StatusCode> {
    let orbit = state.orbit_query.set_satellite_tle(id, payload).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok((StatusCode::CREATED, Json(orbit)))
}

#[tracing::instrument(skip(state))]
pub async fn get_prediction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<PredictionQuery>,
) -> Result<Json<Vec<OrbitalPrediction>>, StatusCode> {
    let minutes = query.minutes.unwrap_or(120);
    let step = query.step.unwrap_or(5);
    let preds = state.orbit_query.get_predictions(id, minutes, step).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(preds))
}

#[tracing::instrument(skip(state))]
pub async fn get_look_angles(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<LookAnglesQuery>,
) -> Result<Json<LookAngles>, StatusCode> {
    let alt_m = query.alt.unwrap_or(0.0);
    let look = state.orbit_query.get_look_angles(id, query.lat, query.lon, alt_m).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(look))
}

#[tracing::instrument(skip(state))]
pub async fn get_passes(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<PassesQuery>,
) -> Result<Json<Vec<GroundPass>>, StatusCode> {
    let alt_m = query.alt.unwrap_or(0.0);
    let min_el = query.min_el.unwrap_or(5.0);
    let hours = query.hours.unwrap_or(24);

    let passes = state.orbit_query.get_satellite_passes(id, query.lat, query.lon, alt_m, min_el, hours).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(passes))
}

#[tracing::instrument(skip(state))]
pub async fn get_nearby(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<NearbyQuery>,
) -> Result<Json<Vec<NearbySatellite>>, StatusCode> {
    let radius = query.radius_km.unwrap_or(100.0);
    let nearby = state.orbit_query.get_nearby_satellites(id, radius).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(nearby))
}

#[tracing::instrument(skip(state))]
pub async fn get_formation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<FormationMember>>, StatusCode> {
    let members = state.orbit_query.get_formation_membership(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(members))
}

#[tracing::instrument(skip(state))]
pub async fn get_ground_stations(State(state): State<AppState>) -> Result<Json<Vec<GroundStation>>, StatusCode> {
    let stations = state.orbit_query.get_ground_stations().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(stations))
}

#[tracing::instrument(skip(state))]
pub async fn get_windows(State(state): State<AppState>) -> Result<Json<Vec<CommunicationWindow>>, StatusCode> {
    let wins = state.orbit_query.get_communication_windows().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(wins))
}
