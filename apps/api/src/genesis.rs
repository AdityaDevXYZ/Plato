use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;
use types::{Email, MissionId, ReleaseId};
use identity::user::User;
use identity::repository::UserRepository;
use policy::Role;
use twin::model::{Satellite, OperationalStatus, CommunicationStatus};
use twin::repository::TwinRepository;
use audit::AuditLedgerRepository;
use deployment::model::{DeploymentSession, DeploymentState};
use deployment::repository::DeploymentSessionRepository;
use storage::{
    user::SqlxUserRepository,
    twin::SqlxDigitalTwinRepository,
    audit::SqlxAuditRepository,
    deployment::SqlxDeploymentRepository,
};
use time::OffsetDateTime;

pub async fn bootstrap_genesis_if_needed(pool: &PgPool) -> anyhow::Result<()> {
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await?;

    if row.0 > 0 {
        return Ok(());
    }

    info!("🚀 [PLATO GENESIS] Bootstrapping initial mission control constellation state...");

    let user_repo = SqlxUserRepository::new(pool.clone());
    let twin_repo = SqlxDigitalTwinRepository::new(pool.clone());
    let audit_repo = SqlxAuditRepository::new(pool.clone());
    let deployment_repo = SqlxDeploymentRepository::new(pool.clone());

    // 1. Organization
    let org_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let org_data = serde_json::json!({
        "id": org_id,
        "name": "PLATO Commercial Spaceflight Operations",
        "description": "Primary constellation mission control organization",
        "created_at": OffsetDateTime::now_utc(),
        "updated_at": OffsetDateTime::now_utc()
    });
    let _ = sqlx::query(
        "INSERT INTO organizations (id, data) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING"
    )
    .bind(org_id)
    .bind(org_data)
    .execute(pool)
    .await;

    // 2. Owner User
    let admin_email = Email::new("admin@plato.aero").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let admin_user = User::new(
        admin_email,
        "PlatoAdmin2026!",
        Role::Owner,
        "Flight Operations Director".to_string(),
        Some(org_id),
    ).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    user_repo.save(&admin_user).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // 3. Project & Mission
    let project_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    let project_data = serde_json::json!({
        "id": project_id,
        "organization_id": org_id,
        "name": "Alpha Constellation Project",
        "status": "Active"
    });
    let _ = sqlx::query(
        "INSERT INTO projects (id, data) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING"
    )
    .bind(project_id)
    .bind(project_data)
    .execute(pool)
    .await;

    let mission_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let mission_data = serde_json::json!({
        "id": mission_id,
        "project_id": project_id,
        "name": "Alpha Constellation LEO",
        "status": "Active"
    });
    let _ = sqlx::query(
        "INSERT INTO missions (id, data) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING"
    )
    .bind(mission_id)
    .bind(mission_data)
    .execute(pool)
    .await;

    // 4. Initial Constellation Satellites
    let sat_configs = vec![
        ("aaaaaaaa-0001-0000-0000-000000000001", "Alpha-1", 59001, 98, 94.2, 42.0, 41.0, 22.5, 21.4),
        ("aaaaaaaa-0002-0000-0000-000000000002", "Alpha-2", 59002, 95, 88.0, 39.0, 48.5, 31.0, 23.1),
        ("aaaaaaaa-0003-0000-0000-000000000003", "Alpha-3", 59003, 99, 96.5, 35.0, 36.0, 18.2, 19.8),
    ];

    for (s_id, name, norad, health, battery, storage, memory, cpu, temp) in sat_configs {
        let sid = Uuid::parse_str(s_id).unwrap();
        let sat = Satellite {
            id: sid,
            mission_id: MissionId::from(mission_id),
            name: name.to_string(),
            norad_id: Some(norad),
            software_version: "v1.2.0".to_string(),
            operational_status: OperationalStatus::Active,
            health_score: health,
            battery_level: battery,
            storage_usage: storage,
            memory_usage: memory,
            cpu_utilization: cpu,
            temperature: temp,
            last_contact: OffsetDateTime::now_utc(),
            comm_status: CommunicationStatus::Connected,
            version: 1,
        };
        let _ = twin_repo.save_satellite(&sat).await;
    }

    // 5. Genesis Audit Ledger Entry (SHA-256 hash chaining)
    let _ = audit_repo.append_entry(
        admin_user.id().into(),
        "admin@plato.aero".to_string(),
        "GENESIS_INITIALIZED".to_string(),
        "SYSTEM".to_string(),
        "00000000-0000-0000-0000-000000000001".to_string(),
        serde_json::json!({
            "message": "PLATO Mission Control Platform initialized for satellite constellation operations",
            "version": "1.0.0",
            "constellation": "Alpha Constellation LEO",
            "operator": "admin@plato.aero"
        }),
    ).await;

    // 6. Initial Deployment Session
    let release_uuid = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let release_data = serde_json::json!({
        "id": release_uuid,
        "name": "Flight Firmware v1.2.1-hotfix",
        "version": "1.2.1",
        "status": "Approved",
        "mission_id": mission_id
    });
    let _ = sqlx::query(
        "INSERT INTO releases (id, data) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING"
    )
    .bind(release_uuid)
    .bind(release_data)
    .execute(pool)
    .await;

    let mut session = DeploymentSession::new(
        ReleaseId::from(release_uuid),
        Uuid::new_v4(),
    );
    session.target_satellite_id = Some(Uuid::parse_str("aaaaaaaa-0001-0000-0000-000000000001").unwrap());
    session.target_partition = Some("bank_b".to_string());
    session.status = DeploymentState::Executing;
    session.packets_total = 120;
    session.packets_transmitted = 72;
    session.packets_acknowledged = 70;
    session.log("INFO", "Initial telecommand session established with Alpha-1 over White Sands ground station");
    session.log("INFO", "Uploading flight firmware v1.2.1-hotfix chunk sequence 1..72 to standby bank B");
    let _ = deployment_repo.save(&session).await;

    info!("✅ [PLATO GENESIS] Constellation and Flight Operations bootstrapped successfully! Initial Operator: admin@plato.aero / PlatoAdmin2026!");
    Ok(())
}
