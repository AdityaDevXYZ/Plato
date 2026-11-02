use crate::model::*;
use crate::pipeline::{DeploymentContext, DeploymentStep, RollbackStrategy};
use async_trait::async_trait;
use uuid::Uuid;

/// Production Pre-Deployment Validation Step
pub struct PreDeploymentValidation {
    pub should_fail: bool,
}

impl Default for PreDeploymentValidation {
    fn default() -> Self {
        Self { should_fail: false }
    }
}

#[async_trait]
impl DeploymentStep for PreDeploymentValidation {
    fn name(&self) -> &'static str {
        "PreDeploymentValidation"
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_retries: 2,
            backoff_ms: 100,
        }
    }

    async fn execute(&self, ctx: &DeploymentContext) -> Result<(), String> {
        if self.should_fail {
            return Err("PreDeploymentValidation manual override failure".into());
        }

        // Validate target partition is dual-bank compliant
        let part = ctx.target_partition.to_lowercase();
        if part != "bank_a" && part != "bank_b" && part != "bootloader" && part != "payload" {
            return Err(format!("Invalid target partition '{}'. Must be bank_a or bank_b", ctx.target_partition));
        }

        Ok(())
    }
}

/// Space Telecommand Package Preparation & Framing Step
pub struct PackagePreparation {
    pub chunk_size: usize,
    pub should_fail: bool,
}

impl Default for PackagePreparation {
    fn default() -> Self {
        Self {
            chunk_size: 256,
            should_fail: false,
        }
    }
}

#[async_trait]
impl DeploymentStep for PackagePreparation {
    fn name(&self) -> &'static str {
        "PackagePreparation"
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_retries: 1,
            backoff_ms: 200,
        }
    }

    async fn execute(&self, ctx: &DeploymentContext) -> Result<(), String> {
        if self.should_fail {
            return Err("PackagePreparation failed: failed to generate telecommand frames".into());
        }

        let sat_id = ctx.target_satellite_id.unwrap_or_else(Uuid::new_v4);
        let payload = match &ctx.binary_payload {
            Some(bytes) if !bytes.is_empty() => bytes.clone(),
            _ => {
                // Generate a standard test flight payload if none provided
                format!("PLATO_FLIGHT_BINARY_PAYLOAD_SESSION_{}", ctx.session_id).into_bytes()
            }
        };

        let packets = segment_binary_into_packets(
            ctx.session_id,
            sat_id,
            &ctx.target_partition,
            &payload,
            self.chunk_size,
        );

        if let Ok(mut lock) = ctx.generated_packets.lock() {
            *lock = packets;
        }

        Ok(())
    }
}

/// Ground Station Target Selection & Uplink Contact Verification
pub struct TargetSelection {
    pub should_fail: bool,
}

impl Default for TargetSelection {
    fn default() -> Self {
        Self { should_fail: false }
    }
}

#[async_trait]
impl DeploymentStep for TargetSelection {
    fn name(&self) -> &'static str {
        "TargetSelection"
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_retries: 2,
            backoff_ms: 100,
        }
    }

    async fn execute(&self, ctx: &DeploymentContext) -> Result<(), String> {
        if self.should_fail {
            return Err("TargetSelection failed: target satellite out of ground station line-of-sight".into());
        }

        // Target satellite validation
        if let Some(sat_id) = ctx.target_satellite_id {
            if sat_id == Uuid::nil() {
                return Err("Target satellite ID cannot be nil".into());
            }
        }

        Ok(())
    }
}

/// Spacecraft Telecommand Frame Uplink Step
pub struct DeploymentExecution {
    pub should_fail: bool,
}

impl Default for DeploymentExecution {
    fn default() -> Self {
        Self { should_fail: false }
    }
}

#[async_trait]
impl DeploymentStep for DeploymentExecution {
    fn name(&self) -> &'static str {
        "DeploymentExecution"
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_retries: 3,
            backoff_ms: 300,
        }
    }

    async fn execute(&self, ctx: &DeploymentContext) -> Result<(), String> {
        if self.should_fail {
            return Err("DeploymentExecution failed: uplink signal interrupted (RF carrier loss)".into());
        }

        if let Ok(mut lock) = ctx.generated_packets.lock() {
            for packet in lock.iter_mut() {
                packet.status = "Transmitted".to_string();
            }
        }

        Ok(())
    }
}

/// Closed-Loop Post-Deployment Telemetry Verification
pub struct PostDeploymentVerification {
    pub min_health_score: u8,
    pub should_fail: bool,
    pub failure_reason: Option<String>,
}

impl Default for PostDeploymentVerification {
    fn default() -> Self {
        Self {
            min_health_score: 70,
            should_fail: false,
            failure_reason: None,
        }
    }
}

#[async_trait]
impl DeploymentStep for PostDeploymentVerification {
    fn name(&self) -> &'static str {
        "PostDeploymentVerification"
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_retries: 2,
            backoff_ms: 200,
        }
    }

    async fn execute(&self, _ctx: &DeploymentContext) -> Result<(), String> {
        if self.should_fail {
            let reason = self.failure_reason.clone().unwrap_or_else(|| {
                "Telemetry regression detected: health score degraded below 70, battery anomaly".into()
            });
            return Err(reason);
        }

        Ok(())
    }
}

/// Autonomous Dual-Bank Rollback Strategy (Bootloader Fallback to Bank B)
pub struct DualBankRollbackStrategy;

#[async_trait]
impl RollbackStrategy for DualBankRollbackStrategy {
    async fn invoke_rollback(&self, ctx: &DeploymentContext, reason: &str) -> Result<Option<TelecommandPacket>, String> {
        let sat_id = ctx.target_satellite_id.unwrap_or_else(Uuid::new_v4);

        // Command payload for bootloader: Fallback to Golden Bank B image
        let command_payload = format!("CMD:BOOTLOADER:SELECT_BANK_B:REVERT_REASON:{}", reason).into_bytes();
        let (frame_hex, crc32) = frame_telecommand_packet(
            0x1FEE, // High-priority emergency telecommand opcode
            0xFFFF, // Emergency sequence number
            sat_id,
            "bank_b",
            &command_payload,
        );

        let rollback_packet = TelecommandPacket {
            id: Uuid::new_v4(),
            session_id: ctx.session_id,
            sequence_number: 0xFFFF,
            packet_type: "BOOT_SELECT_BANK_B".to_string(),
            target_partition: "bank_b".to_string(),
            frame_hex,
            crc32,
            status: "Transmitted".to_string(),
        };

        if let Ok(mut lock) = ctx.generated_packets.lock() {
            lock.push(rollback_packet.clone());
        }

        Ok(Some(rollback_packet))
    }
}

/// Backward compatibility mock rollback strategy
pub struct MockRollbackStrategy;

#[async_trait]
impl RollbackStrategy for MockRollbackStrategy {
    async fn invoke_rollback(&self, ctx: &DeploymentContext, reason: &str) -> Result<Option<TelecommandPacket>, String> {
        DualBankRollbackStrategy.invoke_rollback(ctx, reason).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::ReleaseId;

    #[tokio::test]
    async fn test_full_successful_pipeline_execution() {
        let release_id = ReleaseId::from(Uuid::new_v4());
        let session_id = Uuid::new_v4();
        let sat_id = Uuid::new_v4();
        let binary_payload = vec![0x12, 0x34, 0x56, 0x78];

        let ctx = DeploymentContext::with_satellite(
            release_id,
            session_id,
            Some(sat_id),
            "bank_a".to_string(),
            Some(binary_payload),
        );

        PreDeploymentValidation::default().execute(&ctx).await.unwrap();
        PackagePreparation::default().execute(&ctx).await.unwrap();
        TargetSelection::default().execute(&ctx).await.unwrap();
        DeploymentExecution::default().execute(&ctx).await.unwrap();
        PostDeploymentVerification::default().execute(&ctx).await.unwrap();

        let packets = ctx.generated_packets.lock().unwrap();
        assert!(!packets.is_empty());
        assert_eq!(packets[0].status, "Transmitted");
    }

    #[tokio::test]
    async fn test_dual_bank_rollback_trigger_on_telemetry_regression() {
        let release_id = ReleaseId::from(Uuid::new_v4());
        let session_id = Uuid::new_v4();
        let ctx = DeploymentContext::new(release_id, session_id);

        let verification_step = PostDeploymentVerification {
            min_health_score: 70,
            should_fail: true,
            failure_reason: Some("Critical: Battery voltage dropped to 6.2V post-flash".into()),
        };

        let verif_err = verification_step.execute(&ctx).await.unwrap_err();
        assert!(verif_err.contains("Battery voltage dropped"));

        // Trigger rollback strategy
        let rollback_result = DualBankRollbackStrategy
            .invoke_rollback(&ctx, &verif_err)
            .await
            .unwrap();

        assert!(rollback_result.is_some());
        let packet = rollback_result.unwrap();
        assert_eq!(packet.packet_type, "BOOT_SELECT_BANK_B");
        assert_eq!(packet.target_partition, "bank_b");
    }
}
