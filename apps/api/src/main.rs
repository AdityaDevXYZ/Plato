pub mod state;
pub mod middleware;
pub mod auth_handlers;
pub mod roe_handlers;
pub mod health_handlers;
pub mod twin_handlers;
pub mod orbit_handlers;
pub mod planning_handlers;
pub mod resource_handlers;
pub mod risk_handlers;
pub mod coordination_handlers;
pub mod analytics_handlers;
pub mod entity_handlers;
pub mod ws_handlers;
pub mod genesis;

use axum::{routing::{get, post}, Router, Json, extract::State, middleware::from_fn_with_state};
use serde_json::{json, Value};
use tokio::signal;
use plato_config::Settings;
use telemetry::init_telemetry;
use std::sync::Arc;
use state::AppState;
use tower_http::trace::TraceLayer;

use storage::{
    assurance::SqlxAssuranceRepository,
    readiness::SqlxReadinessRepository,
    deployment::SqlxDeploymentRepository,
    outbox::SqlxOutboxRepository,
    twin::SqlxDigitalTwinRepository,
    orbit::SqlxOrbitRepository,
    planning::SqlxPlanningRepository,
    resource::SqlxResourceRepository,
    risk::SqlxRiskRepository,
    coordination::SqlxCoordinationRepository,
    analytics::SqlxAnalyticsRepository,
    db::{init_pool, run_migrations},
    user::SqlxUserRepository,
    organization::SqlxOrganizationRepository,
    project::SqlxProjectRepository,
    mission::SqlxMissionRepository,
    release::SqlxReleaseRepository,
    audit::SqlxAuditRepository,
};

use application::{
    auth::AuthenticationService,
    audit::AuditService,
    roe::ReleaseOrchestrationService,
    assurance::ReleaseAssuranceService,
    readiness::DeploymentReadinessService,
    deployment::DeploymentExecutionService,
    twin::{TwinQueryService, TwinUpdateService},
    orbit::OrbitalQueryService,
    planning::MissionPlanningService,
    resource::ResourceQueryService,
    risk::RiskAssessmentService,
    coordination::CoordinationService,
    analytics::DeploymentAnalyticsService,
    organization::OrganizationService,
    project::ProjectService,
    mission::MissionService,
    user::UserService,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::load()?;
    init_telemetry(&settings.app.name)?;
    tracing::info!("Starting {} version {}", settings.app.name, settings.app.version);

    let pool = init_pool(
        &settings.db.url,
        settings.db.max_connections,
        settings.db.connection_timeout
    ).await?;

    if let Err(e) = run_migrations(&pool).await {
        tracing::warn!("Database migration note: {:?}", e);
    }

    let user_repo = Arc::new(SqlxUserRepository::new(pool.clone()));
    let event_publisher = Arc::new(eventbus::memory::InMemoryEventBus::new());
    
    let assurance_repo = Arc::new(SqlxAssuranceRepository::new(pool.clone()));
    let checks: Vec<std::sync::Arc<dyn assurance::pipeline::AssuranceCheck>> = vec![
        Arc::new(assurance::checks::ArtifactIntegrityCheck),
        Arc::new(assurance::checks::SignatureVerificationCheck),
        Arc::new(assurance::checks::FlightSafetyComplianceCheck),
    ];
    let assurance_pipeline = Arc::new(assurance::pipeline::AssurancePipeline::new(checks));
    
    let readiness_repo = Arc::new(SqlxReadinessRepository::new(pool.clone()));
    let read_checks: Vec<std::sync::Arc<dyn readiness::pipeline::ReadinessCheck>> = vec![
        Arc::new(readiness::mock_checks::DeploymentWindowCheck),
        Arc::new(readiness::mock_checks::ConcurrentDeploymentCheck),
        Arc::new(readiness::mock_checks::DependencyCheck { should_fail: false }),
    ];
    let readiness_pipeline = Arc::new(readiness::pipeline::DeploymentReadinessPipeline::new(read_checks));
    
    let deployment_repo = Arc::new(SqlxDeploymentRepository::new(pool.clone()));
    let dep_steps: Vec<std::sync::Arc<dyn deployment::pipeline::DeploymentStep>> = vec![
        Arc::new(deployment::steps::PreDeploymentValidation::default()),
        Arc::new(deployment::steps::PackagePreparation::default()),
        Arc::new(deployment::steps::TargetSelection::default()),
        Arc::new(deployment::steps::DeploymentExecution::default()),
        Arc::new(deployment::steps::PostDeploymentVerification::default()),
    ];
    let rollback_strat = Arc::new(deployment::steps::DualBankRollbackStrategy);
    let deployment_service = Arc::new(DeploymentExecutionService::new(dep_steps, rollback_strat, deployment_repo.clone(), event_publisher.clone()));
        
    let twin_repo = Arc::new(SqlxDigitalTwinRepository::new(pool.clone()));
    let twin_query = Arc::new(TwinQueryService::new(twin_repo.clone()));
    let twin_update = Arc::new(TwinUpdateService::new(twin_repo.clone(), event_publisher.clone()));
    
    let orbit_repo = Arc::new(SqlxOrbitRepository::new(pool.clone()));
    let orbit_query = Arc::new(OrbitalQueryService::new(orbit_repo));
    
    let planning_repo = Arc::new(SqlxPlanningRepository::new(pool.clone()));
    let planning_service = Arc::new(MissionPlanningService::new(planning_repo, twin_query.clone(), orbit_query.clone(), event_publisher.clone()));
    
    let resource_repo = Arc::new(SqlxResourceRepository::new(pool.clone()));
    let resource_query = Arc::new(ResourceQueryService::new(resource_repo));
    
    let risk_repo = Arc::new(SqlxRiskRepository::new(pool.clone()));
    let risk_pipeline = Arc::new(risk::pipeline::RiskAssessmentPipeline::new(vec![
        Box::new(risk::mock_assessments::OperationalRiskAssessment),
        Box::new(risk::mock_assessments::OrbitalRiskAssessment),
        Box::new(risk::mock_assessments::ResourceRiskAssessment),
        Box::new(risk::mock_assessments::CommunicationRiskAssessment),
    ]));
    let risk_service = Arc::new(RiskAssessmentService::new(risk_repo, risk_pipeline, event_publisher.clone()));
    
    let coord_repo = Arc::new(SqlxCoordinationRepository::new(pool.clone()));
    let coordination_service = Arc::new(CoordinationService::new(coord_repo, planning_service.clone(), event_publisher.clone()));
    
    let analytics_repo = Arc::new(SqlxAnalyticsRepository::new(pool.clone()));
    let analytics_service = Arc::new(DeploymentAnalyticsService::new(analytics_repo, event_publisher.clone()));

    let org_repo = Arc::new(SqlxOrganizationRepository::new(pool.clone()));
    let proj_repo = Arc::new(SqlxProjectRepository::new(pool.clone()));
    let mission_repo = Arc::new(SqlxMissionRepository::new(pool.clone()));
    let release_repo = Arc::new(SqlxReleaseRepository::new(pool.clone()));
    let audit_repo = Arc::new(SqlxAuditRepository::new(pool.clone()));
    if let Err(e) = genesis::bootstrap_genesis_if_needed(&pool).await {
        tracing::warn!("Genesis bootstrap note: {:?}", e);
    }
    let audit_service = Arc::new(AuditService::new(audit_repo.clone()));
    let auth_service = Arc::new(AuthenticationService::with_audit(
        user_repo.clone(),
        event_publisher.clone(),
        settings.app.jwt_secret.clone(),
        audit_repo.clone(),
    ));
    let (telemetry_tx, _) = tokio::sync::broadcast::channel(1024);

    let app_state = AppState {
        config: Arc::new(settings.clone()),
        db_pool: pool.clone(),
        org_service: Arc::new(OrganizationService::new(org_repo)),
        project_service: Arc::new(ProjectService::new(proj_repo)),
        mission_service: Arc::new(MissionService::new(mission_repo)),
        roe_service: Arc::new(ReleaseOrchestrationService::new(release_repo, event_publisher.clone())),
        rae_service: Arc::new(ReleaseAssuranceService::new(assurance_pipeline, assurance_repo.clone(), assurance_repo.clone(), event_publisher.clone())),
        dra_service: Arc::new(DeploymentReadinessService::new(readiness_pipeline, readiness_repo, event_publisher.clone())),
        dee_service: deployment_service,
        twin_query,
        twin_update,
        orbit_query,
        planning_service,
        resource_query,
        risk_service,
        coordination_service,
        analytics_service,
        user_service: Arc::new(UserService::new(user_repo.clone())),
        auth_service,
        audit_service: audit_service.clone(),
        telemetry_tx,
    };

    let roe_routes = Router::new()
        .route("/", post(roe_handlers::create_release))
        .route("/:id", get(roe_handlers::get_release))
        .route("/:id/history", get(roe_handlers::get_history))
        .route("/:id/manifest", post(roe_handlers::create_release_manifest))
        .route("/:id/manifest", get(roe_handlers::get_release_manifest))
        .route("/:id/validate", post(roe_handlers::validate_release))
        .route("/:id/submit", post(roe_handlers::submit_release))
        .route("/:id/approve", post(roe_handlers::approve_release))
        .route("/:id/assure", post(roe_handlers::assure_release))
        .route("/:id/assurance", get(roe_handlers::get_assurance))
        .route("/:id/readiness", post(roe_handlers::assess_readiness))
        .route("/:id/readiness", get(roe_handlers::get_readiness))
        .route("/:id/deploy", post(roe_handlers::deploy_release))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));

    let dee_routes = Router::new()
        .route("/", get(roe_handlers::list_deployments_queue))
        .route("/queue", get(roe_handlers::list_deployments_queue))
        .route("/:id", get(roe_handlers::get_deployment))
        .route("/:id/logs", get(roe_handlers::get_deployment_logs))
        .route("/:id/packets", get(roe_handlers::get_deployment_packets))
        .route("/:id/pause", post(roe_handlers::pause_deployment))
        .route("/:id/cancel", post(roe_handlers::cancel_deployment))
        .route("/:id/rollback", post(roe_handlers::rollback_deployment))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));

    let auth_routes = Router::new()
        .route("/me", get(auth_handlers::me_handler))
        .route("/refresh", post(auth_handlers::refresh_handler))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware))
        .route("/login", post(auth_handlers::login_handler))
        .route("/register", post(auth_handlers::register_handler))
        .route("/health", get(auth_handlers::health_handler));

    let outbox_repo = Arc::new(SqlxOutboxRepository::new(pool.clone()));
    let outbox_worker = Arc::new(application::outbox_worker::OutboxWorker::new(outbox_repo, event_publisher.clone()));
    let worker_clone = outbox_worker.clone();
    tokio::spawn(async move {
        loop {
            let _ = worker_clone.run_once().await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    let org_routes = Router::new()
        .route("/", post(entity_handlers::create_organization))
        .route("/:id", get(entity_handlers::get_organization))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));

    let project_routes = Router::new()
        .route("/", post(entity_handlers::create_project))
        .route("/:id", get(entity_handlers::get_project))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));

    let twin_routes = Router::new()
        .route("/fleet", get(twin_handlers::get_fleet_summary))
        .route("/assets/:id", get(twin_handlers::get_asset_details))
        .route("/satellites", get(twin_handlers::list_satellites).post(twin_handlers::register_satellite))
        .route("/satellites/:id", get(twin_handlers::get_satellite))
        .route("/satellites/:id/telemetry", post(twin_handlers::ingest_telemetry))
        .route("/satellites/:id/telemetry/history", get(twin_handlers::get_telemetry_history))
        .route("/satellites/unhealthy", get(twin_handlers::get_unhealthy))
        .route("/missions/:id", get(twin_handlers::get_mission_assets))
        .layer(TraceLayer::new_for_http());

    let orbit_routes = Router::new()
        .route("/positions", get(twin_handlers::get_orbital_positions))
        .route("/satellites/:id", get(orbit_handlers::get_orbit))
        .route("/satellites/:id/tle", post(orbit_handlers::set_tle))
        .route("/satellites/:id/prediction", get(orbit_handlers::get_prediction))
        .route("/satellites/:id/look-angles", get(orbit_handlers::get_look_angles))
        .route("/satellites/:id/passes", get(orbit_handlers::get_passes))
        .route("/satellites/:id/nearby", get(orbit_handlers::get_nearby))
        .route("/satellites/:id/formation", get(orbit_handlers::get_formation))
        .route("/ground-stations", get(orbit_handlers::get_ground_stations))
        .route("/windows", get(orbit_handlers::get_windows))
        .layer(TraceLayer::new_for_http());

    let mission_routes = Router::new()
        .route("/", post(entity_handlers::create_mission))
        .route("/:id", get(entity_handlers::get_mission))
        .route("/:id/plan", post(planning_handlers::create_plan))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));
        
    let plan_routes = Router::new()
        .route("/:id", get(planning_handlers::get_plan))
        .route("/:id/risk", post(risk_handlers::evaluate_plan))
        .route("/:id/coordinate", post(coordination_handlers::start_coordination))
        .layer(TraceLayer::new_for_http());

    let resource_routes = Router::new()
        .route("/satellites", get(resource_handlers::list_satellites))
        .route("/satellites/:id", get(resource_handlers::get_satellite))
        .route("/fleet", get(resource_handlers::get_fleet_summary))
        .layer(TraceLayer::new_for_http());

    let risk_routes = Router::new()
        .route("/:id", get(risk_handlers::get_report))
        .layer(TraceLayer::new_for_http());

    let coord_routes = Router::new()
        .route("/:id", get(coordination_handlers::get_session))
        .layer(TraceLayer::new_for_http());

    let analytics_routes = Router::new()
        .route("/kpis", get(analytics_handlers::get_kpis))
        .route("/reports", get(analytics_handlers::get_reports))
        .route("/deployments", get(analytics_handlers::get_deployments))
        .route("/strategies", get(analytics_handlers::get_strategies))
        .route("/reliability", get(analytics_handlers::get_reliability))
        .layer(TraceLayer::new_for_http());

    let audit_routes = Router::new()
        .route("/", get(auth_handlers::get_audit_ledger_handler))
        .route("/verify", get(auth_handlers::verify_audit_integrity_handler))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));

    let admin_routes = Router::new()
        .route("/users", get(auth_handlers::list_users_handler))
        .route("/audit", get(auth_handlers::get_audit_ledger_handler))
        .route("/audit/verify", get(auth_handlers::verify_audit_integrity_handler))
        .route_layer(from_fn_with_state(app_state.clone(), middleware::auth_middleware));

    let app = Router::new()
        .route("/health/live", get(health_handlers::liveness))
        .route("/health/ready", get(health_handlers::readiness))
        .route("/status", get(health_handlers::status))
        .route("/health", get(health_check))
        .route("/ws", get(ws_handlers::telemetry_stream_handler))
        .route("/ws/telemetry", get(ws_handlers::telemetry_stream_handler))
        .route("/api/v1/telemetry/stream", get(ws_handlers::telemetry_stream_handler))
        .route("/api/v1/ws/telemetry", get(ws_handlers::telemetry_stream_handler))
        .nest("/api/v1/auth", auth_routes)
        .nest("/api/v1/audit", audit_routes)
        .nest("/api/v1/admin", admin_routes)
        .nest("/api/v1/organizations", org_routes)
        .nest("/api/v1/projects", project_routes)
        .nest("/api/v1/missions", mission_routes)
        .nest("/api/v1/releases", roe_routes)
        .nest("/api/v1/deployments", dee_routes)
        .nest("/api/v1/twin", twin_routes.clone())
        .nest("/api/v1/orbital", orbit_routes.clone())
        .nest("/api/v1/satellites", Router::new()
            .route("/", get(twin_handlers::list_satellites).post(twin_handlers::register_satellite))
            .route("/:id", get(twin_handlers::get_satellite))
            .route("/:id/telemetry", post(twin_handlers::ingest_telemetry))
            .route("/:id/telemetry/history", get(twin_handlers::get_telemetry_history))
            .route("/unhealthy", get(twin_handlers::get_unhealthy))
            .layer(TraceLayer::new_for_http())
        )
        .nest("/api/v1/orbits", orbit_routes)
        .nest("/api/v1/plans", plan_routes)
        .nest("/api/v1/resources", resource_routes)
        .nest("/api/v1/risk", risk_routes)
        .nest("/api/v1/coordination", coord_routes)
        .nest("/api/v1/analytics", analytics_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn health_check(State(state): State<AppState>) -> Json<Value> {
    let db_status = sqlx::query("SELECT 1").execute(&state.db_pool).await.is_ok();
    
    Json(json!({
        "status": if db_status { "ok" } else { "degraded" },
        "service": state.config.app.name.clone(),
        "version": state.config.app.version.clone(),
        "database": if db_status { "connected" } else { "unreachable" }
    }))
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.unwrap(); };
    #[cfg(unix)]
    let terminate = async { signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap().recv().await; };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received Ctrl+C"); },
        _ = terminate => { tracing::info!("Received SIGTERM"); },
    }
}
