use sqlx::PgPool;
use plato_config::Settings;
use std::sync::Arc;
use application::{
    auth::AuthenticationService, roe::ReleaseOrchestrationService, assurance::ReleaseAssuranceService,
    readiness::DeploymentReadinessService, deployment::DeploymentExecutionService, twin::{TwinQueryService, TwinUpdateService},
    orbit::OrbitalQueryService, planning::MissionPlanningService, resource::ResourceQueryService,
    risk::RiskAssessmentService, coordination::CoordinationService, analytics::DeploymentAnalyticsService,
    organization::OrganizationService, project::ProjectService, mission::MissionService,
    user::UserService,
};
use twin::model::{Satellite, AnomalyReport};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TelemetryStreamMessage {
    TelemetryUpdate {
        satellite: Satellite,
        anomalies: Vec<AnomalyReport>,
    },
    AnomalyAlert {
        satellite_id: uuid::Uuid,
        anomaly: AnomalyReport,
    },
    Heartbeat {
        timestamp: String,
    },
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Settings>,
    pub db_pool: PgPool,
    pub org_service: Arc<OrganizationService>,
    pub project_service: Arc<ProjectService>,
    pub mission_service: Arc<MissionService>,
    pub roe_service: Arc<ReleaseOrchestrationService>,
    pub rae_service: Arc<ReleaseAssuranceService>,
    pub dra_service: Arc<DeploymentReadinessService>,
    pub dee_service: Arc<DeploymentExecutionService>,
    pub twin_query: Arc<TwinQueryService>,
    pub twin_update: Arc<TwinUpdateService>,
    pub orbit_query: Arc<OrbitalQueryService>,
    pub planning_service: Arc<MissionPlanningService>,
    pub resource_query: Arc<ResourceQueryService>,
    pub risk_service: Arc<RiskAssessmentService>,
    pub coordination_service: Arc<CoordinationService>,
    pub analytics_service: Arc<DeploymentAnalyticsService>,
    pub user_service: Arc<UserService>,
    pub auth_service: Arc<AuthenticationService>,
    pub audit_service: Arc<application::audit::AuditService>,
    pub telemetry_tx: tokio::sync::broadcast::Sender<TelemetryStreamMessage>,
}
