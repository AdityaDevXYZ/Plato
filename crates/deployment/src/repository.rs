use crate::model::{DeploymentSession, TelecommandPacket};
use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(String),
}

#[async_trait]
pub trait DeploymentSessionRepository: Send + Sync {
    async fn save(&self, session: &DeploymentSession) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, session_id: Uuid) -> Result<Option<DeploymentSession>, RepositoryError>;
    async fn find_queued_sessions(&self, limit: i64) -> Result<Vec<DeploymentSession>, RepositoryError>;
    async fn find_recent_sessions(&self, limit: i64) -> Result<Vec<DeploymentSession>, RepositoryError>;
    async fn save_telecommand_packets(&self, packets: &[TelecommandPacket]) -> Result<(), RepositoryError>;
    async fn find_packets_by_session_id(&self, session_id: Uuid) -> Result<Vec<TelecommandPacket>, RepositoryError>;
}
