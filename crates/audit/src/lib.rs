use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Cryptographic hash chain broken at sequence {0}: {1}")]
    CorruptedChain(i64, String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEntry {
    pub id: Uuid,
    pub sequence_number: i64,
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
    pub actor_id: Uuid,
    pub actor_email: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: serde_json::Value,
    pub prev_hash: String,
    pub entry_hash: String,
}

impl AuditEntry {
    /// Computes deterministic SHA-256 hash chaining over the audit entry fields
    pub fn compute_hash(
        sequence_number: i64,
        timestamp_unix: i64,
        actor_id: Uuid,
        actor_email: &str,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        details: &serde_json::Value,
        prev_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        let payload = format!(
            "AUDIT-V1:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            sequence_number,
            timestamp_unix,
            actor_id,
            actor_email,
            action,
            resource_type,
            resource_id,
            details,
            prev_hash
        );
        hasher.update(payload.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Validates internal cryptographic hash of this record
    pub fn verify_hash(&self) -> bool {
        let expected = Self::compute_hash(
            self.sequence_number,
            self.timestamp.unix_timestamp(),
            self.actor_id,
            &self.actor_email,
            &self.action,
            &self.resource_type,
            &self.resource_id,
            &self.details,
            &self.prev_hash,
        );
        self.entry_hash == expected
    }
}

#[async_trait]
pub trait AuditLedgerRepository: Send + Sync {
    async fn append_entry(
        &self,
        actor_id: Uuid,
        actor_email: String,
        action: String,
        resource_type: String,
        resource_id: String,
        details: serde_json::Value,
    ) -> Result<AuditEntry, AuditError>;

    async fn get_entries(&self, limit: i64, offset: i64) -> Result<Vec<AuditEntry>, AuditError>;
    async fn get_latest_entry(&self) -> Result<Option<AuditEntry>, AuditError>;
    async fn verify_chain_integrity(&self) -> Result<bool, AuditError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_hash_chaining() {
        let actor_id = Uuid::new_v4();
        let timestamp = OffsetDateTime::now_utc();
        let details = serde_json::json!({"release_version": "1.0.0", "target": "bank_a"});

        let hash_1 = AuditEntry::compute_hash(
            1,
            timestamp.unix_timestamp(),
            actor_id,
            "flight-lead@plato.space",
            "RELEASE_APPROVED",
            "Release",
            "rel-001",
            &details,
            GENESIS_HASH,
        );
        assert_eq!(hash_1.len(), 64);

        let entry_1 = AuditEntry {
            id: Uuid::new_v4(),
            sequence_number: 1,
            timestamp,
            actor_id,
            actor_email: "flight-lead@plato.space".into(),
            action: "RELEASE_APPROVED".into(),
            resource_type: "Release".into(),
            resource_id: "rel-001".into(),
            details: details.clone(),
            prev_hash: GENESIS_HASH.into(),
            entry_hash: hash_1.clone(),
        };
        assert!(entry_1.verify_hash());

        // Block 2 chains to Block 1
        let hash_2 = AuditEntry::compute_hash(
            2,
            timestamp.unix_timestamp(),
            actor_id,
            "flight-director@plato.space",
            "DEPLOYMENT_QUEUED",
            "DeploymentSession",
            "sess-001",
            &details,
            &hash_1,
        );
        assert_ne!(hash_1, hash_2);

        // Tampering with details invalidates the entry hash
        let tampered_details = serde_json::json!({"release_version": "2.0.0-HACKED"});
        let tampered_entry = AuditEntry {
            id: entry_1.id,
            sequence_number: 1,
            timestamp,
            actor_id,
            actor_email: "flight-lead@plato.space".into(),
            action: "RELEASE_APPROVED".into(),
            resource_type: "Release".into(),
            resource_id: "rel-001".into(),
            details: tampered_details,
            prev_hash: GENESIS_HASH.into(),
            entry_hash: hash_1,
        };
        assert!(!tampered_entry.verify_hash());
    }
}
