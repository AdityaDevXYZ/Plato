use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};
use plato_config::Settings;
use storage::db::{init_pool, run_migrations};
use storage::deployment::SqlxDeploymentRepository;
use application::deployment::DeploymentExecutionService;
use deployment::steps::{
    PreDeploymentValidation, PackagePreparation, TargetSelection,
    DeploymentExecution, PostDeploymentVerification, DualBankRollbackStrategy,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,worker=debug".into()),
        )
        .init();

    info!("Starting PLATO Asynchronous Deployment Execution Worker (DEE)...");

    let settings = Settings::load()?;
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| settings.db.url.clone());
    info!("DEE Worker connecting to database: {}", db_url);

    let pool = init_pool(&db_url, settings.db.max_connections, settings.db.connection_timeout).await?;

    if let Err(e) = run_migrations(&pool).await {
        warn!("Database migration note: {:?}", e);
    }

    let event_publisher = Arc::new(eventbus::memory::InMemoryEventBus::new());
    let deployment_repo = Arc::new(SqlxDeploymentRepository::new(pool.clone()));

    let deploy_steps: Vec<std::sync::Arc<dyn deployment::pipeline::DeploymentStep>> = vec![
        Arc::new(PreDeploymentValidation::default()),
        Arc::new(PackagePreparation::default()),
        Arc::new(TargetSelection::default()),
        Arc::new(DeploymentExecution::default()),
        Arc::new(PostDeploymentVerification::default()),
    ];
    let rollback_strategy = Arc::new(DualBankRollbackStrategy);
    let dee_service = Arc::new(DeploymentExecutionService::new(
        deploy_steps,
        rollback_strategy,
        deployment_repo.clone(),
        event_publisher.clone(),
    ));

    info!("DEE Worker active. Polling queued satellite deployments every 3 seconds...");

    let mut interval = tokio::time::interval(Duration::from_secs(3));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("DEE Worker received shutdown signal. Exiting gracefully...");
                break;
            }
            _ = interval.tick() => {
                match dee_service.claim_and_execute_queued(5).await {
                    Ok(executed) => {
                        for session in executed {
                            info!(
                                "DEE Worker executed session {} [status: {:?}, packets: {}/{}]",
                                session.session_id, session.status, session.packets_transmitted, session.packets_total
                            );
                        }
                    }
                    Err(e) => {
                        error!("DEE Worker queue polling error: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
