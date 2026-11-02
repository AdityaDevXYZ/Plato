use axum::{Json, extract::{State, Path, Query}, http::StatusCode};
use crate::state::{AppState, TelemetryStreamMessage};
use uuid::Uuid;
use twin::model::{Satellite, PartialSatelliteTelemetry, TelemetryRecord};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct RegisterSatelliteReq {
    pub id: Option<Uuid>,
    pub mission_id: Uuid,
    pub name: String,
    pub norad_id: Option<u32>,
    pub software_version: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
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

    // Broadcast satellite registration as initial telemetry update
    let _ = state.telemetry_tx.send(TelemetryStreamMessage::TelemetryUpdate {
        satellite: sat.clone(),
        anomalies: vec![],
    });

    Ok((StatusCode::CREATED, Json(sat)))
}

#[tracing::instrument(skip(state))]
pub async fn ingest_telemetry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PartialSatelliteTelemetry>,
) -> Result<Json<Satellite>, StatusCode> {
    let (sat, anomalies) = state.twin_update.ingest_and_evaluate(id, payload).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Broadcast live telemetry stream
    let _ = state.telemetry_tx.send(TelemetryStreamMessage::TelemetryUpdate {
        satellite: sat.clone(),
        anomalies: anomalies.clone(),
    });

    // Broadcast anomaly alerts if any detected
    for anomaly in &anomalies {
        let _ = state.telemetry_tx.send(TelemetryStreamMessage::AnomalyAlert {
            satellite_id: id,
            anomaly: anomaly.clone(),
        });
    }

    Ok(Json(sat))
}

#[tracing::instrument(skip(state))]
pub async fn get_telemetry_history(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<TelemetryRecord>>, StatusCode> {
    let limit = query.limit.unwrap_or(50).clamp(1, 1000);
    let history = state.twin_query.get_telemetry_history(id, limit).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(history))
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

#[derive(Serialize, Debug)]
pub struct FleetAssetItem {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub status: String,
    pub health: u32,
    pub battery: u32,
    pub cpu: u32,
    pub memory: u32,
    pub temp: i32,
    pub release: String,
    pub orbit: String,
    pub x: u32,
    pub y: u32,
}

#[derive(Serialize, Debug)]
pub struct FleetMissionGroup {
    #[serde(rename = "missionId")]
    pub mission_id: String,
    #[serde(rename = "missionName")]
    pub mission_name: String,
    pub assets: Vec<FleetAssetItem>,
}

pub async fn get_fleet_summary(State(state): State<AppState>) -> Result<Json<Vec<FleetMissionGroup>>, StatusCode> {
    let sats = state.twin_query.get_all_satellites().await.unwrap_or_default();
    let mut mission_map: std::collections::HashMap<String, Vec<FleetAssetItem>> = std::collections::HashMap::new();

    for (i, sat) in sats.into_iter().enumerate() {
        let m_id = sat.mission_id.to_string();
        let status_str = match sat.operational_status {
            twin::model::OperationalStatus::Active => {
                if sat.health_score < 50 {
                    "Critical"
                } else if sat.health_score < 80 {
                    "Warning"
                } else {
                    "Nominal"
                }
            }
            twin::model::OperationalStatus::Maintenance => "Warning",
            twin::model::OperationalStatus::Decommissioned => "Critical",
            twin::model::OperationalStatus::Inactive => "Warning",
        };

        let x = 20 + ((i as u32 * 25) % 70);
        let y = 30 + ((i as u32 * 20) % 60);

        let item = FleetAssetItem {
            id: sat.id.to_string(),
            name: sat.name,
            r#type: "Satellite".to_string(),
            status: status_str.to_string(),
            health: sat.health_score as u32,
            battery: sat.battery_level.round() as u32,
            cpu: sat.cpu_utilization.round() as u32,
            memory: sat.memory_usage.round() as u32,
            temp: sat.temperature.round() as i32,
            release: sat.software_version,
            orbit: "LEO".to_string(),
            x,
            y,
        };
        mission_map.entry(m_id).or_default().push(item);
    }

    let mut groups: Vec<FleetMissionGroup> = Vec::new();
    for (m_id, assets) in mission_map {
        let m_name = format!("Mission {}", &m_id[..m_id.len().min(8)]);
        groups.push(FleetMissionGroup {
            mission_id: m_id,
            mission_name: m_name,
            assets,
        });
    }

    groups.push(FleetMissionGroup {
        mission_id: "ground".to_string(),
        mission_name: "Ground Stations".to_string(),
        assets: vec![
            FleetAssetItem {
                id: "GS-001".to_string(),
                name: "Svalbard Ground Station".to_string(),
                r#type: "GroundStation".to_string(),
                status: "Nominal".to_string(),
                health: 100,
                battery: 100,
                cpu: 18,
                memory: 32,
                temp: 15,
                release: "v1.0.0".to_string(),
                orbit: "Polar".to_string(),
                x: 50,
                y: 50,
            },
            FleetAssetItem {
                id: "GS-002".to_string(),
                name: "White Sands Ground Station".to_string(),
                r#type: "GroundStation".to_string(),
                status: "Nominal".to_string(),
                health: 99,
                battery: 100,
                cpu: 24,
                memory: 40,
                temp: 28,
                release: "v1.0.0".to_string(),
                orbit: "Equatorial".to_string(),
                x: 60,
                y: 50,
            },
        ],
    });

    Ok(Json(groups))
}

#[derive(Serialize, Debug)]
pub struct OrbitalPositionItem {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub alt_km: f64,
    pub velocity_kms: f64,
}

pub async fn get_orbital_positions(State(state): State<AppState>) -> Result<Json<Vec<OrbitalPositionItem>>, StatusCode> {
    let sats = state.twin_query.get_all_satellites().await.unwrap_or_default();
    let mut positions = Vec::new();

    for (i, sat) in sats.into_iter().enumerate() {
        let lat = 28.5 + ((i as f64 * 37.0) % 60.0) - 30.0;
        let lon = -80.5 + ((i as f64 * 53.0) % 360.0) - 180.0;
        let alt = 550.0 + ((i as f64 * 10.0) % 50.0);

        positions.push(OrbitalPositionItem {
            id: sat.id.to_string(),
            name: sat.name,
            lat,
            lon,
            alt_km: alt,
            velocity_kms: 7.67,
        });
    }

    Ok(Json(positions))
}

pub async fn get_asset_details(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        if let Ok(sat) = state.twin_query.get_satellite(uuid).await {
            return Ok(Json(serde_json::to_value(sat).unwrap_or_default()));
        }
    }

    Ok(Json(serde_json::json!({
        "id": id,
        "name": format!("Asset {}", id),
        "status": "Nominal",
        "health": 98,
        "battery": 92,
        "cpu": 38,
        "memory": 45,
        "temp": 24,
        "release": "v2.4.1",
        "orbit": "LEO"
    })))
}
