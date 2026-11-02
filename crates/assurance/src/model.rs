use types::ReleaseId;
use uuid::Uuid;
use time::OffsetDateTime;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssuranceDecision { Assured, AssuredWithAdvisory, NotAssured, Indeterminate }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceStatus { Pass, Fail, Warning, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub name: String,
    pub status: EvidenceStatus,
    pub details: String,
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssuranceReport {
    pub report_id: Uuid,
    pub release_id: ReleaseId,
    pub correlation_id: Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
    pub decision: AssuranceDecision,
    pub confidence_score: u8,
    pub evidence: Vec<EvidenceItem>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryArtifact {
    pub artifact_id: Uuid,
    pub name: String,
    pub target_partition: String,
    pub size_bytes: u64,
    pub expected_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptographicSignature {
    pub signer_id: String,
    pub algorithm: String, // e.g. "Ed25519"
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlightReleaseManifest {
    pub release_id: ReleaseId,
    pub version: String,
    pub target_architecture: String,
    pub max_binary_size_bytes: u64,
    pub dual_bank_enabled: bool,
    pub artifacts: Vec<BinaryArtifact>,
    pub signatures: Vec<CryptographicSignature>,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
}

impl FlightReleaseManifest {
    pub fn total_artifacts_size(&self) -> u64 {
        self.artifacts.iter().map(|a| a.size_bytes).sum()
    }

    /// Computes deterministic canonical byte digest of the manifest for Ed25519 digital signing
    pub fn compute_canonical_digest(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        let rel_id_str: Uuid = self.release_id.into();
        let header = format!(
            "PLATO-MANIFEST-V1:{}:{}:{}:{}:{}",
            rel_id_str,
            self.version,
            self.target_architecture,
            self.max_binary_size_bytes,
            self.dual_bank_enabled
        );
        hasher.update(header.as_bytes());

        // Sort artifacts deterministically by name
        let mut sorted_artifacts = self.artifacts.clone();
        sorted_artifacts.sort_by(|a, b| a.name.cmp(&b.name));

        for art in &sorted_artifacts {
            let art_entry = format!(
                "|{}:{}:{}:{}",
                art.name,
                art.target_partition,
                art.size_bytes,
                art.expected_sha256.to_lowercase()
            );
            hasher.update(art_entry.as_bytes());
        }

        hasher.finalize().to_vec()
    }

    pub fn compute_canonical_digest_hex(&self) -> String {
        hex::encode(self.compute_canonical_digest())
    }
}
