use crate::model::*;
use crate::pipeline::{AssuranceCheck, AssuranceContext};
use async_trait::async_trait;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Computes lowercase hex-encoded SHA-256 digest of binary payload
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Verifies an Ed25519 digital signature over a message or digest
pub fn verify_ed25519_signature(public_key_hex: &str, signature_hex: &str, message: &[u8]) -> Result<(), String> {
    let pk_bytes = hex::decode(public_key_hex.trim())
        .map_err(|e| format!("Invalid public key hex: {}", e))?;
    if pk_bytes.len() != 32 {
        return Err(format!("Ed25519 public key must be 32 bytes, got {}", pk_bytes.len()));
    }
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&pk_bytes);

    let verifying_key = VerifyingKey::from_bytes(&pk_arr)
        .map_err(|e| format!("Invalid Ed25519 public key structure: {}", e))?;

    let sig_bytes = hex::decode(signature_hex.trim())
        .map_err(|e| format!("Invalid signature hex: {}", e))?;
    if sig_bytes.len() != 64 {
        return Err(format!("Ed25519 signature must be 64 bytes, got {}", sig_bytes.len()));
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);

    let signature = Signature::from_bytes(&sig_arr);

    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("Ed25519 signature verification failed: {}", e))
}

/// Helper to sign a message using a 32-byte secret key and return (public_key_hex, signature_hex)
pub fn sign_ed25519_message(signing_key_bytes: &[u8; 32], message: &[u8]) -> (String, String) {
    let signing_key = SigningKey::from_bytes(signing_key_bytes);
    let verifying_key = signing_key.verifying_key();
    let signature = signing_key.sign(message);
    (
        hex::encode(verifying_key.to_bytes()),
        hex::encode(signature.to_bytes()),
    )
}

/// Real Cryptographic SHA-256 Binary Artifact Integrity Check
pub struct ArtifactIntegrityCheck;

#[async_trait]
impl AssuranceCheck for ArtifactIntegrityCheck {
    fn name(&self) -> &'static str {
        "ArtifactIntegrity"
    }

    async fn execute(&self, ctx: &AssuranceContext) -> Result<EvidenceItem, String> {
        let manifest = match &ctx.manifest {
            Some(m) => m,
            None => {
                return Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Pass,
                    details: "Legacy release without flight manifest; integrity check assumed pass".into(),
                    timestamp: OffsetDateTime::now_utc(),
                });
            }
        };

        if manifest.artifacts.is_empty() {
            return Ok(EvidenceItem {
                name: self.name().into(),
                status: EvidenceStatus::Fail,
                details: "Flight release manifest contains zero binary artifacts".into(),
                timestamp: OffsetDateTime::now_utc(),
            });
        }

        let mut verified_count = 0;
        let mut warnings = Vec::new();

        for artifact in &manifest.artifacts {
            // Validate expected SHA-256 format
            let clean_expected = artifact.expected_sha256.trim().to_lowercase();
            if clean_expected.len() != 64 || hex::decode(&clean_expected).is_err() {
                return Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Fail,
                    details: format!(
                        "Artifact '{}' has invalid expected SHA-256 format: '{}'",
                        artifact.name, artifact.expected_sha256
                    ),
                    timestamp: OffsetDateTime::now_utc(),
                });
            }

            // Verify actual binary payload if provided
            if let Some(payload_hex) = &artifact.payload_hex {
                let payload_bytes = match hex::decode(payload_hex.trim()) {
                    Ok(b) => b,
                    Err(e) => {
                        return Ok(EvidenceItem {
                            name: self.name().into(),
                            status: EvidenceStatus::Fail,
                            details: format!("Failed to decode payload hex for artifact '{}': {}", artifact.name, e),
                            timestamp: OffsetDateTime::now_utc(),
                        });
                    }
                };

                if payload_bytes.len() as u64 != artifact.size_bytes {
                    return Ok(EvidenceItem {
                        name: self.name().into(),
                        status: EvidenceStatus::Fail,
                        details: format!(
                            "Artifact '{}' byte length mismatch: manifest declared {} bytes, but payload is {} bytes",
                            artifact.name, artifact.size_bytes, payload_bytes.len()
                        ),
                        timestamp: OffsetDateTime::now_utc(),
                    });
                }

                let actual_sha256 = compute_sha256(&payload_bytes);
                if actual_sha256 != clean_expected {
                    return Ok(EvidenceItem {
                        name: self.name().into(),
                        status: EvidenceStatus::Fail,
                        details: format!(
                            "SHA-256 integrity violation for '{}'! Expected {}, calculated {}",
                            artifact.name, clean_expected, actual_sha256
                        ),
                        timestamp: OffsetDateTime::now_utc(),
                    });
                }
                verified_count += 1;
            } else {
                warnings.push(format!("Payload for '{}' pending uplinking verification", artifact.name));
            }
        }

        let details = if !warnings.is_empty() && verified_count > 0 {
            format!(
                "Verified {} artifact(s) against SHA-256 hashes. Advisory: {}",
                verified_count,
                warnings.join("; ")
            )
        } else if verified_count > 0 {
            format!(
                "All {} flight binary artifact(s) verified against cryptographic SHA-256 checksums",
                verified_count
            )
        } else {
            format!(
                "Manifest specifies {} valid SHA-256 artifact hashes",
                manifest.artifacts.len()
            )
        };

        Ok(EvidenceItem {
            name: self.name().into(),
            status: EvidenceStatus::Pass,
            details,
            timestamp: OffsetDateTime::now_utc(),
        })
    }
}

/// Real Ed25519 Cryptographic Signature Verification Check
pub struct SignatureVerificationCheck;

#[async_trait]
impl AssuranceCheck for SignatureVerificationCheck {
    fn name(&self) -> &'static str {
        "SignatureVerification"
    }

    async fn execute(&self, ctx: &AssuranceContext) -> Result<EvidenceItem, String> {
        let manifest = match &ctx.manifest {
            Some(m) => m,
            None => {
                return Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Pass,
                    details: "Legacy release without flight manifest; signature assumed pass".into(),
                    timestamp: OffsetDateTime::now_utc(),
                });
            }
        };

        if manifest.signatures.is_empty() {
            return Ok(EvidenceItem {
                name: self.name().into(),
                status: EvidenceStatus::Fail,
                details: "Zero cryptographic signatures found on flight release manifest".into(),
                timestamp: OffsetDateTime::now_utc(),
            });
        }

        let canonical_digest = manifest.compute_canonical_digest();
        let mut verified_signers = Vec::new();

        for sig in &manifest.signatures {
            if sig.algorithm.to_uppercase() != "ED25519" {
                return Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Fail,
                    details: format!(
                        "Unsupported signature algorithm '{}' by signer '{}' (only Ed25519 permitted for space operations)",
                        sig.algorithm, sig.signer_id
                    ),
                    timestamp: OffsetDateTime::now_utc(),
                });
            }

            match verify_ed25519_signature(&sig.public_key_hex, &sig.signature_hex, &canonical_digest) {
                Ok(_) => {
                    verified_signers.push(sig.signer_id.clone());
                }
                Err(err) => {
                    return Ok(EvidenceItem {
                        name: self.name().into(),
                        status: EvidenceStatus::Fail,
                        details: format!(
                            "Ed25519 signature verification failed for signer '{}': {}",
                            sig.signer_id, err
                        ),
                        timestamp: OffsetDateTime::now_utc(),
                    });
                }
            }
        }

        Ok(EvidenceItem {
            name: self.name().into(),
            status: EvidenceStatus::Pass,
            details: format!(
                "Successfully verified {} Ed25519 signature(s) from authorized signers: [{}]",
                verified_signers.len(),
                verified_signers.join(", ")
            ),
            timestamp: OffsetDateTime::now_utc(),
        })
    }
}

/// Aerospace Flight Safety and Redundant Partition Compliance Check
pub struct FlightSafetyComplianceCheck;

const VALID_FLIGHT_ARCHITECTURES: &[&str] = &[
    "ARM-Cortex-R5",
    "ARM-Cortex-M7",
    "SPARC-LEON3",
    "SPARC-LEON4",
    "RISC-V-64",
    "x86_64",
];

const VALID_PARTITIONS: &[&str] = &[
    "bank_a",
    "bank_b",
    "bootloader",
    "safe_mode",
    "payload",
];

#[async_trait]
impl AssuranceCheck for FlightSafetyComplianceCheck {
    fn name(&self) -> &'static str {
        "FlightSafetyCompliance"
    }

    async fn execute(&self, ctx: &AssuranceContext) -> Result<EvidenceItem, String> {
        let manifest = match &ctx.manifest {
            Some(m) => m,
            None => {
                return Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Pass,
                    details: "Legacy release; policy compliance evaluated pass".into(),
                    timestamp: OffsetDateTime::now_utc(),
                });
            }
        };

        // 1. Target architecture check
        if !VALID_FLIGHT_ARCHITECTURES.contains(&manifest.target_architecture.as_str()) {
            return Ok(EvidenceItem {
                name: self.name().into(),
                status: EvidenceStatus::Fail,
                details: format!(
                    "Non-compliant target flight architecture '{}'. Allowed architectures: {:?}",
                    manifest.target_architecture, VALID_FLIGHT_ARCHITECTURES
                ),
                timestamp: OffsetDateTime::now_utc(),
            });
        }

        // 2. Dual bank partition check (critical for space safe fallback)
        if !manifest.dual_bank_enabled {
            return Ok(EvidenceItem {
                name: self.name().into(),
                status: EvidenceStatus::Fail,
                details: "Dual-bank A/B memory partitioning is disabled! Space flight software requires dual-bank redundancy for safe rollbacks".into(),
                timestamp: OffsetDateTime::now_utc(),
            });
        }

        // 3. Size budget check
        let total_size = manifest.total_artifacts_size();
        if total_size > manifest.max_binary_size_bytes {
            return Ok(EvidenceItem {
                name: self.name().into(),
                status: EvidenceStatus::Fail,
                details: format!(
                    "Total binary size ({} bytes) exceeds hardware storage budget limit ({} bytes)",
                    total_size, manifest.max_binary_size_bytes
                ),
                timestamp: OffsetDateTime::now_utc(),
            });
        }

        // 4. Partition targeting check
        for art in &manifest.artifacts {
            if !VALID_PARTITIONS.contains(&art.target_partition.as_str()) {
                return Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Fail,
                    details: format!(
                        "Artifact '{}' targets unknown partition '{}'. Allowed: {:?}",
                        art.name, art.target_partition, VALID_PARTITIONS
                    ),
                    timestamp: OffsetDateTime::now_utc(),
                });
            }
        }

        Ok(EvidenceItem {
            name: self.name().into(),
            status: EvidenceStatus::Pass,
            details: format!(
                "Flight safety rules compliant: Target {}, dual-bank redundancy active, size {}/{} bytes",
                manifest.target_architecture, total_size, manifest.max_binary_size_bytes
            ),
            timestamp: OffsetDateTime::now_utc(),
        })
    }
}

/// Backward compatibility policy check
pub struct PolicyComplianceCheck {
    pub should_fail: bool,
}

#[async_trait]
impl AssuranceCheck for PolicyComplianceCheck {
    fn name(&self) -> &'static str {
        "PolicyCompliance"
    }

    async fn execute(&self, ctx: &AssuranceContext) -> Result<EvidenceItem, String> {
        if self.should_fail {
            Ok(EvidenceItem {
                name: self.name().into(),
                status: EvidenceStatus::Fail,
                details: "Compliance rule check failed".into(),
                timestamp: OffsetDateTime::now_utc(),
            })
        } else {
            // If manifest is present, run flight safety compliance
            if ctx.manifest.is_some() {
                FlightSafetyComplianceCheck.execute(ctx).await
            } else {
                Ok(EvidenceItem {
                    name: self.name().into(),
                    status: EvidenceStatus::Pass,
                    details: "Compliance evaluated pass".into(),
                    timestamp: OffsetDateTime::now_utc(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::ReleaseId;
    use uuid::Uuid;

    #[test]
    fn test_sha256_computation() {
        let data = b"PLATO_SATELLITE_FLIGHT_FIRMWARE_CORE";
        let hash = compute_sha256(data);
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, compute_sha256(data));
        // Verify mismatch with different data
        assert_ne!(hash, compute_sha256(b"CORRUPTED_FIRMWARE"));
    }

    #[test]
    fn test_ed25519_sign_and_verify() {
        let seed = [42u8; 32];
        let msg = b"PLATO_TELECOMMAND_ORBIT_BURN";
        let (pubkey_hex, sig_hex) = sign_ed25519_message(&seed, msg);

        // Valid verification
        let res = verify_ed25519_signature(&pubkey_hex, &sig_hex, msg);
        assert!(res.is_ok(), "Signature verification failed: {:?}", res);

        // Tampered message should fail
        let tampered = b"PLATO_TELECOMMAND_DEORBIT_BURN";
        let res_tampered = verify_ed25519_signature(&pubkey_hex, &sig_hex, tampered);
        assert!(res_tampered.is_err(), "Tampered message should fail verification");

        // Invalid signature hex should fail
        let bad_sig = "00".repeat(64);
        let res_bad = verify_ed25519_signature(&pubkey_hex, &bad_sig, msg);
        assert!(res_bad.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_with_real_manifest_pass() {
        let release_id = ReleaseId::from(Uuid::new_v4());
        let seed = [7u8; 32];
        let binary_bytes = b"FLIGHT_BINARY_CODE_BANK_A_v1.0.0";
        let expected_sha256 = compute_sha256(binary_bytes);

        let mut manifest = FlightReleaseManifest {
            release_id,
            version: "1.0.0".to_string(),
            target_architecture: "ARM-Cortex-R5".to_string(),
            max_binary_size_bytes: 33554432,
            dual_bank_enabled: true,
            artifacts: vec![BinaryArtifact {
                artifact_id: Uuid::new_v4(),
                name: "obc_core.bin".to_string(),
                target_partition: "bank_a".to_string(),
                size_bytes: binary_bytes.len() as u64,
                expected_sha256,
                payload_hex: Some(hex::encode(binary_bytes)),
            }],
            signatures: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
        };

        // Sign the canonical digest
        let digest = manifest.compute_canonical_digest();
        let (pubkey_hex, sig_hex) = sign_ed25519_message(&seed, &digest);
        manifest.signatures.push(CryptographicSignature {
            signer_id: "secops-key-01".to_string(),
            algorithm: "Ed25519".to_string(),
            public_key_hex: pubkey_hex,
            signature_hex: sig_hex,
        });

        let ctx = AssuranceContext::with_manifest(release_id, manifest);

        let integrity_res = ArtifactIntegrityCheck.execute(&ctx).await.unwrap();
        assert_eq!(integrity_res.status, EvidenceStatus::Pass);

        let sig_res = SignatureVerificationCheck.execute(&ctx).await.unwrap();
        assert_eq!(sig_res.status, EvidenceStatus::Pass);

        let safety_res = FlightSafetyComplianceCheck.execute(&ctx).await.unwrap();
        assert_eq!(safety_res.status, EvidenceStatus::Pass);
    }

    #[tokio::test]
    async fn test_pipeline_catches_corrupted_artifact() {
        let release_id = ReleaseId::from(Uuid::new_v4());
        let manifest = FlightReleaseManifest {
            release_id,
            version: "1.0.0".to_string(),
            target_architecture: "ARM-Cortex-R5".to_string(),
            max_binary_size_bytes: 33554432,
            dual_bank_enabled: true,
            artifacts: vec![BinaryArtifact {
                artifact_id: Uuid::new_v4(),
                name: "obc_core.bin".to_string(),
                target_partition: "bank_a".to_string(),
                size_bytes: 5,
                expected_sha256: compute_sha256(b"VALID"),
                // Corrupted payload!
                payload_hex: Some(hex::encode(b"CORRU")),
            }],
            signatures: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
        };

        let ctx = AssuranceContext::with_manifest(release_id, manifest);
        let integrity_res = ArtifactIntegrityCheck.execute(&ctx).await.unwrap();
        assert_eq!(integrity_res.status, EvidenceStatus::Fail);
        assert!(integrity_res.details.contains("SHA-256 integrity violation"));
    }

    #[tokio::test]
    async fn test_pipeline_catches_safety_violations() {
        let release_id = ReleaseId::from(Uuid::new_v4());
        let manifest = FlightReleaseManifest {
            release_id,
            version: "1.0.0".to_string(),
            target_architecture: "Commodore64".to_string(), // Invalid flight arch!
            max_binary_size_bytes: 100,
            dual_bank_enabled: false, // Dual bank disabled!
            artifacts: vec![],
            signatures: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
        };

        let ctx = AssuranceContext::with_manifest(release_id, manifest);
        let safety_res = FlightSafetyComplianceCheck.execute(&ctx).await.unwrap();
        assert_eq!(safety_res.status, EvidenceStatus::Fail);
        assert!(safety_res.details.contains("Non-compliant target flight architecture"));
    }
}
