use crate::model::*;
use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(String),
}

#[async_trait]
pub trait OrbitalRepository: Send + Sync {
    async fn save_orbit(&self, orbit: &SatelliteOrbit) -> Result<(), RepositoryError>;
    async fn get_orbit(&self, id: Uuid) -> Result<Option<SatelliteOrbit>, RepositoryError>;
    async fn get_all_orbits(&self) -> Result<Vec<SatelliteOrbit>, RepositoryError>;
    
    async fn save_prediction(&self, pred: &OrbitalPrediction) -> Result<(), RepositoryError>;
    async fn get_predictions(&self, id: Uuid) -> Result<Vec<OrbitalPrediction>, RepositoryError>;
    
    async fn save_window(&self, window: &CommunicationWindow) -> Result<(), RepositoryError>;
    async fn get_windows(&self, after: OffsetDateTime) -> Result<Vec<CommunicationWindow>, RepositoryError>;

    async fn get_ground_stations(&self) -> Result<Vec<GroundStation>, RepositoryError>;
    async fn save_ground_station(&self, gs: &GroundStation) -> Result<(), RepositoryError>;
}
