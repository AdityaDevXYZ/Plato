use crate::error::ApplicationError;
use audit::{AuditEntry, AuditLedgerRepository};
use std::sync::Arc;
use uuid::Uuid;

pub struct AuditService {
    repo: Arc<dyn AuditLedgerRepository>,
}

impl AuditService {
    pub fn new(repo: Arc<dyn AuditLedgerRepository>) -> Self {
        Self { repo }
    }

    #[tracing::instrument(skip(self))]
    pub async fn log(
        &self,
        actor_id: Uuid,
        actor_email: String,
        action: String,
        resource_type: String,
        resource_id: String,
        details: serde_json::Value,
    ) -> Result<AuditEntry, ApplicationError> {
        self.repo
            .append_entry(actor_id, actor_email, action, resource_type, resource_id, details)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ledger(&self, limit: i64, offset: i64) -> Result<Vec<AuditEntry>, ApplicationError> {
        self.repo
            .get_entries(limit, offset)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn verify_integrity(&self) -> Result<bool, ApplicationError> {
        self.repo
            .verify_chain_integrity()
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))
    }
}
