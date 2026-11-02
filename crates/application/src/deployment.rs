use crate::error::ApplicationError;
use deployment::{
    events::*,
    model::{DeploymentSession, DeploymentState, RollbackStatus, TelecommandPacket},
    pipeline::{DeploymentContext, DeploymentStep, RollbackStrategy},
    repository::DeploymentSessionRepository,
};
use eventbus::{EventMessage, EventPublisher};
use std::sync::Arc;
use time::OffsetDateTime;
use types::ReleaseId;
use uuid::Uuid;

pub struct DeploymentExecutionService {
    steps: Vec<Arc<dyn DeploymentStep>>,
    rollback_strategy: Arc<dyn RollbackStrategy>,
    session_repo: Arc<dyn DeploymentSessionRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl DeploymentExecutionService {
    pub fn new(
        steps: Vec<Arc<dyn DeploymentStep>>,
        rollback_strategy: Arc<dyn RollbackStrategy>,
        session_repo: Arc<dyn DeploymentSessionRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            steps,
            rollback_strategy,
            session_repo,
            event_publisher,
        }
    }

    /// Queues a deployment session asynchronously
    #[tracing::instrument(skip(self))]
    pub async fn queue_deployment(
        &self,
        release_id: ReleaseId,
        correlation_id: Uuid,
        target_satellite_id: Option<Uuid>,
        target_partition: Option<String>,
    ) -> Result<DeploymentSession, ApplicationError> {
        let mut session = DeploymentSession::new(release_id, correlation_id);
        session.status = DeploymentState::Queued;
        session.target_satellite_id = target_satellite_id;
        session.target_partition = target_partition.or_else(|| Some("bank_a".to_string()));
        session.log("INFO", "Deployment session queued for ground station pass");

        self.session_repo
            .save(&session)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;

        let _ = self.event_publisher.publish(vec![EventMessage::new(
            Box::new(DeploymentStarted {
                session_id: session.session_id,
                release_id,
            }),
            correlation_id,
            "plato.dee",
        )]);

        Ok(session)
    }

    /// Executes an existing queued deployment session
    #[tracing::instrument(skip(self))]
    pub async fn execute_session(&self, session_id: Uuid) -> Result<DeploymentSession, ApplicationError> {
        let mut session = self
            .session_repo
            .find_by_id(session_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound(format!("Deployment session {} not found", session_id)))?;

        let correlation_id = session.correlation_id;
        let release_id = session.release_id;
        let partition = session.target_partition.clone().unwrap_or_else(|| "bank_a".to_string());

        let ctx = DeploymentContext::with_satellite(
            release_id,
            session.session_id,
            session.target_satellite_id,
            partition,
            None,
        );

        session.status = DeploymentState::Executing;
        session.log("INFO", "Deployment claimed and executing");
        self.session_repo.save(&session).await.ok();

        let mut failed_step: Option<(String, String)> = None;

        for step in &self.steps {
            let name = step.name();
            session.log("INFO", &format!("Executing step: {}", name));
            self.session_repo.save(&session).await.ok();

            let _ = self.event_publisher.publish(vec![EventMessage::new(
                Box::new(DeploymentProgressUpdated {
                    session_id: session.session_id,
                    step: name.into(),
                }),
                correlation_id,
                "plato.dee",
            )]);

            let policy = step.retry_policy();
            let mut attempt = 0;
            let mut success = false;

            while attempt <= policy.max_retries {
                if attempt > 0 {
                    session.status = DeploymentState::Retrying;
                    session.log("WARN", &format!("Retrying step {} (attempt {})", name, attempt));
                    self.session_repo.save(&session).await.ok();
                    let _ = self.event_publisher.publish(vec![EventMessage::new(
                        Box::new(DeploymentRetrying {
                            session_id: session.session_id,
                            step: name.into(),
                            attempt,
                        }),
                        correlation_id,
                        "plato.dee",
                    )]);
                }

                match step.execute(&ctx).await {
                    Ok(_) => {
                        success = true;
                        break;
                    }
                    Err(e) => {
                        session.log("ERROR", &format!("Step {} failed: {}", name, e));
                        if attempt == policy.max_retries {
                            failed_step = Some((name.into(), e));
                        }
                    }
                }
                attempt += 1;
            }

            if !success {
                break;
            }
        }

        // Persist generated telecommand packets to storage
        let packets = if let Ok(lock) = ctx.generated_packets.lock() {
            lock.clone()
        } else {
            Vec::new()
        };

        if !packets.is_empty() {
            session.packets_total = packets.len() as u32;
            session.packets_transmitted = packets.iter().filter(|p| p.status == "Transmitted").count() as u32;
            session.packets_acknowledged = session.packets_transmitted;
            let _ = self.session_repo.save_telecommand_packets(&packets).await;
        }

        if let Some((failed_name, reason)) = failed_step {
            session.status = DeploymentState::RollingBack;
            session.rollback_status = RollbackStatus::InProgress;
            let fail_reason = format!("Failed at step {}: {}", failed_name, reason);
            session.failure_reason = Some(fail_reason.clone());
            session.log("ERROR", &format!("Initiating autonomous dual-bank rollback: {}", fail_reason));
            self.session_repo.save(&session).await.ok();

            let _ = self.event_publisher.publish(vec![EventMessage::new(
                Box::new(DeploymentRollbackStarted {
                    session_id: session.session_id,
                    reason: fail_reason.clone(),
                }),
                correlation_id,
                "plato.dee",
            )]);

            match self.rollback_strategy.invoke_rollback(&ctx, &reason).await {
                Ok(maybe_packet) => {
                    session.rollback_status = RollbackStatus::Succeeded;
                    session.log("INFO", "Dual-bank fallback command generated: booted to Golden Bank B");
                    if let Some(packet) = maybe_packet {
                        let _ = self.session_repo.save_telecommand_packets(&[packet]).await;
                    }
                    let _ = self.event_publisher.publish(vec![EventMessage::new(
                        Box::new(DeploymentRollbackCompleted {
                            session_id: session.session_id,
                            success: true,
                        }),
                        correlation_id,
                        "plato.dee",
                    )]);
                }
                Err(e) => {
                    session.rollback_status = RollbackStatus::Failed;
                    session.log("ERROR", &format!("Rollback failed: {}", e));
                    let _ = self.event_publisher.publish(vec![EventMessage::new(
                        Box::new(DeploymentRollbackCompleted {
                            session_id: session.session_id,
                            success: false,
                        }),
                        correlation_id,
                        "plato.dee",
                    )]);
                }
            }

            session.status = DeploymentState::Failed;
            session.completed_at = Some(OffsetDateTime::now_utc());
            self.session_repo
                .save(&session)
                .await
                .map_err(|e| ApplicationError::Repository(e.to_string()))?;

            let _ = self.event_publisher.publish(vec![EventMessage::new(
                Box::new(DeploymentFailed {
                    session_id: session.session_id,
                    release_id,
                    reason: fail_reason,
                }),
                correlation_id,
                "plato.dee",
            )]);
        } else {
            session.status = DeploymentState::Succeeded;
            session.completed_at = Some(OffsetDateTime::now_utc());
            session.log("INFO", "Deployment succeeded across satellite contact window");
            self.session_repo
                .save(&session)
                .await
                .map_err(|e| ApplicationError::Repository(e.to_string()))?;

            let _ = self.event_publisher.publish(vec![EventMessage::new(
                Box::new(DeploymentSucceeded {
                    session_id: session.session_id,
                    release_id,
                }),
                correlation_id,
                "plato.dee",
            )]);
        }

        Ok(session)
    }

    /// Convenience end-to-end execution
    #[tracing::instrument(skip(self))]
    pub async fn execute_deployment(
        &self,
        release_id: ReleaseId,
        correlation_id: Uuid,
    ) -> Result<DeploymentSession, ApplicationError> {
        let session = self.queue_deployment(release_id, correlation_id, None, None).await?;
        self.execute_session(session.session_id).await
    }

    /// Fetches deployment session by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_session(&self, session_id: Uuid) -> Result<DeploymentSession, ApplicationError> {
        self.session_repo
            .find_by_id(session_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Session not found".into()))
    }

    /// Retrieves all generated telecommand frames for a session
    #[tracing::instrument(skip(self))]
    pub async fn get_session_packets(&self, session_id: Uuid) -> Result<Vec<TelecommandPacket>, ApplicationError> {
        self.session_repo
            .find_packets_by_session_id(session_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    /// Claims and executes any pending queued deployment sessions (used by worker daemon)
    #[tracing::instrument(skip(self))]
    pub async fn claim_and_execute_queued(&self, limit: i64) -> Result<Vec<DeploymentSession>, ApplicationError> {
        let queued = self
            .session_repo
            .find_queued_sessions(limit)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;

        let mut results = Vec::new();
        for item in queued {
            match self.execute_session(item.session_id).await {
                Ok(s) => results.push(s),
                Err(err) => {
                    tracing::error!("Failed executing claimed deployment session {}: {}", item.session_id, err);
                }
            }
        }

        Ok(results)
    }
}
