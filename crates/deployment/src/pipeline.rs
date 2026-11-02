use crate::model::*;
use async_trait::async_trait;
use types::ReleaseId;
use uuid::Uuid;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DeploymentContext {
    pub release_id: ReleaseId,
    pub session_id: Uuid,
    pub target_satellite_id: Option<Uuid>,
    pub target_partition: String,
    pub binary_payload: Option<Vec<u8>>,
    pub generated_packets: Arc<Mutex<Vec<TelecommandPacket>>>,
}

impl DeploymentContext {
    pub fn new(release_id: ReleaseId, session_id: Uuid) -> Self {
        Self {
            release_id,
            session_id,
            target_satellite_id: None,
            target_partition: "bank_a".to_string(),
            binary_payload: None,
            generated_packets: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_satellite(
        release_id: ReleaseId,
        session_id: Uuid,
        target_satellite_id: Option<Uuid>,
        target_partition: String,
        binary_payload: Option<Vec<u8>>,
    ) -> Self {
        Self {
            release_id,
            session_id,
            target_satellite_id,
            target_partition,
            binary_payload,
            generated_packets: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
pub trait DeploymentStep: Send + Sync {
    fn name(&self) -> &'static str;
    fn retry_policy(&self) -> RetryPolicy;
    async fn execute(&self, ctx: &DeploymentContext) -> Result<(), String>;
}

#[async_trait]
pub trait RollbackStrategy: Send + Sync {
    async fn invoke_rollback(&self, ctx: &DeploymentContext, reason: &str) -> Result<Option<TelecommandPacket>, String>;
}
