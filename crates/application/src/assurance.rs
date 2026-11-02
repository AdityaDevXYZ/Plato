use crate::error::ApplicationError;
use assurance::{
    events::*,
    model::{AssuranceDecision, AssuranceReport, EvidenceStatus, FlightReleaseManifest},
    pipeline::{AssuranceContext, AssurancePipeline},
    repository::{AssuranceReportRepository, FlightManifestRepository},
};
use eventbus::{EventMessage, EventPublisher};
use std::sync::Arc;
use time::OffsetDateTime;
use types::ReleaseId;
use uuid::Uuid;

pub struct ReleaseAssuranceService {
    pipeline: Arc<AssurancePipeline>,
    report_repo: Arc<dyn AssuranceReportRepository>,
    manifest_repo: Arc<dyn FlightManifestRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl ReleaseAssuranceService {
    pub fn new(
        pipeline: Arc<AssurancePipeline>,
        report_repo: Arc<dyn AssuranceReportRepository>,
        manifest_repo: Arc<dyn FlightManifestRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            pipeline,
            report_repo,
            manifest_repo,
            event_publisher,
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn save_manifest(&self, manifest: &FlightReleaseManifest) -> Result<(), ApplicationError> {
        self.manifest_repo
            .save_manifest(manifest)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_manifest(&self, release_id: ReleaseId) -> Result<FlightReleaseManifest, ApplicationError> {
        self.manifest_repo
            .find_manifest_by_release_id(release_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound(format!("Flight manifest for release {:?} not found", release_id)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn run_assurance(&self, release_id: ReleaseId, correlation_id: Uuid) -> Result<AssuranceReport, ApplicationError> {
        let report_id = Uuid::new_v4();

        let start_ev = EventMessage::new(
            Box::new(ReleaseAssuranceStarted { release_id, report_id }),
            correlation_id,
            "plato.rae",
        );
        let _ = self.event_publisher.publish(vec![start_ev]);

        let manifest = self
            .manifest_repo
            .find_manifest_by_release_id(release_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;

        let ctx = AssuranceContext {
            release_id,
            manifest,
        };
        let evidence = self.pipeline.run(&ctx).await;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut failed = false;
        let mut warned = false;

        for e in &evidence {
            match e.status {
                EvidenceStatus::Fail => {
                    failed = true;
                    errors.push(format!("{}: {}", e.name, e.details));
                }
                EvidenceStatus::Warning => {
                    warned = true;
                    warnings.push(format!("{}: {}", e.name, e.details));
                }
                _ => {}
            }
        }

        let decision = if failed {
            AssuranceDecision::NotAssured
        } else if warned {
            AssuranceDecision::AssuredWithAdvisory
        } else {
            AssuranceDecision::Assured
        };

        let confidence_score = if failed {
            0
        } else if warned {
            80
        } else {
            100
        };
        let summary = format!(
            "Pipeline completed with {} errors and {} warnings.",
            errors.len(),
            warnings.len()
        );

        let report = AssuranceReport {
            report_id,
            release_id,
            correlation_id,
            timestamp: OffsetDateTime::now_utc(),
            decision: decision.clone(),
            confidence_score,
            evidence,
            warnings,
            errors,
            summary,
        };

        self.report_repo
            .save(&report)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;

        let mut events = vec![EventMessage::new(
            Box::new(ReleaseAssuranceCompleted {
                release_id,
                report_id,
                decision: format!("{:?}", decision),
            }),
            correlation_id,
            "plato.rae",
        )];

        if decision == AssuranceDecision::NotAssured {
            events.push(EventMessage::new(
                Box::new(ReleaseAssuranceFailed {
                    release_id,
                    report_id,
                    reason: "Checks failed".into(),
                }),
                correlation_id,
                "plato.rae",
            ));
        } else {
            events.push(EventMessage::new(
                Box::new(ReleaseAssured {
                    release_id,
                    report_id,
                }),
                correlation_id,
                "plato.rae",
            ));
        }

        let _ = self.event_publisher.publish(events);

        Ok(report)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_report(&self, release_id: ReleaseId) -> Result<AssuranceReport, ApplicationError> {
        self.report_repo
            .find_by_release_id(release_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Report not found".into()))
    }
}
