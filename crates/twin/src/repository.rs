use crate::model::{Satellite, TelemetryRecord};
use types::MissionId;
use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(String),
}

#[async_trait]
pub trait TwinRepository: Send + Sync {
    async fn save_satellite(&self, sat: &Satellite) -> Result<(), RepositoryError>;
    async fn get_satellite(&self, id: Uuid) -> Result<Option<Satellite>, RepositoryError>;
    async fn get_mission_assets(&self, mission_id: MissionId) -> Result<Vec<Satellite>, RepositoryError>;
    async fn get_all_satellites(&self) -> Result<Vec<Satellite>, RepositoryError>;
    async fn get_unhealthy_satellites(&self, threshold: u8) -> Result<Vec<Satellite>, RepositoryError>;
    async fn get_online_satellites(&self) -> Result<Vec<Satellite>, RepositoryError>;
    async fn get_deployable_satellites(&self) -> Result<Vec<Satellite>, RepositoryError>;
    async fn record_telemetry(&self, record: &TelemetryRecord) -> Result<(), RepositoryError>;
    async fn get_telemetry_history(&self, satellite_id: Uuid, limit: i64) -> Result<Vec<TelemetryRecord>, RepositoryError>;
}
