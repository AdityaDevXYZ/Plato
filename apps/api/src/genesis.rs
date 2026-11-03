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
    bootstrap_constellation_if_needed(pool).await?;
    bootstrap_orbits_if_needed(pool).await?;
    Ok(())
}

async fn bootstrap_constellation_if_needed(pool: &PgPool) -> anyhow::Result<()> {
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

    let org_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    // 1. Owner User (initially without org_id to satisfy foreign key constraints)
    let admin_email = Email::new("admin@plato.aero").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let admin_user = User::new(
        admin_email,
        "PlatoAdmin2026!",
        Role::Owner,
        "Flight Operations Director".to_string(),
        None,
    ).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    user_repo.save(&admin_user).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let admin_user_id: Uuid = admin_user.id().into();

    // 2. Organization referencing admin_user as owner
    let _ = sqlx::query(
        "INSERT INTO organizations (id, name, owner_id, status) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING"
    )
    .bind(org_id)
    .bind("PLATO Commercial Spaceflight Operations")
    .bind(admin_user_id)
    .bind("Active")
    .execute(pool)
    .await?;

    // 3. Link user to organization
    let _ = sqlx::query("UPDATE users SET organization_id = $1 WHERE id = $2")
        .bind(org_id)
        .bind(admin_user_id)
        .execute(pool)
        .await?;

    // 4. Project & Mission
    let project_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    let _ = sqlx::query(
        "INSERT INTO projects (id, org_id, name, status) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING"
    )
    .bind(project_id)
    .bind(org_id)
    .bind("Alpha Constellation Project")
    .bind("Active")
    .execute(pool)
    .await?;

    let mission_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let _ = sqlx::query(
        "INSERT INTO missions (id, project_id, name, status) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING"
    )
    .bind(mission_id)
    .bind(project_id)
    .bind("Alpha Constellation LEO")
    .bind("Active")
    .execute(pool)
    .await?;

    // 5. Initial Constellation Satellites
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

    // 6. Genesis Audit Ledger Entry (SHA-256 hash chaining)
    let _ = audit_repo.append_entry(
        admin_user_id,
        "admin@plato.aero".to_string(),
        "GENESIS_INITIALIZED".to_string(),
        "SYSTEM".to_string(),
        org_id.to_string(),
        serde_json::json!({
            "message": "PLATO Mission Control Platform initialized for satellite constellation operations",
            "version": "1.0.0",
            "constellation": "Alpha Constellation LEO",
            "operator": "admin@plato.aero"
        }),
    ).await;

    // 7. Initial Approved Release
    let release_uuid = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let _ = sqlx::query(
        "INSERT INTO releases (id, mission_id, version, status, history, version_num) VALUES ($1, $2, $3, $4, '[]'::jsonb, 1) ON CONFLICT (id) DO NOTHING"
    )
    .bind(release_uuid)
    .bind(mission_id)
    .bind("v1.2.1-hotfix")
    .bind("Approved")
    .execute(pool)
    .await?;

    // 8. Initial Deployment Session
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

pub async fn bootstrap_orbits_if_needed(pool: &PgPool) -> anyhow::Result<()> {
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM satellite_orbits")
        .fetch_one(pool)
        .await?;

    if row.0 >= 3 {
        return Ok(());
    }

    info!("🛰️ [PLATO GENESIS] Bootstrapping SGP4 orbital ephemeris for constellation...");
    let orbit_repo = std::sync::Arc::new(storage::orbit::SqlxOrbitRepository::new(pool.clone()));
    let orbit_svc = application::orbit::OrbitalQueryService::new(orbit_repo);

    let orbit_configs = vec![
        (
            Uuid::parse_str("aaaaaaaa-0001-0000-0000-000000000001").unwrap(),
            "Alpha-1",
            "1 59001U 26001A   26300.00000000  .00001000  00000-0  50000-4 0  9995",
            "2 59001  97.5000 120.0000 0010000  45.0000 315.0000 15.10000000 10008",
        ),
        (
            Uuid::parse_str("aaaaaaaa-0002-0000-0000-000000000002").unwrap(),
            "Alpha-2",
            "1 59002U 26001B   26300.00000000  .00001000  00000-0  50000-4 0  9996",
            "2 59002  97.5000 240.0000 0010000  45.0000 315.0000 15.10000000 10002",
        ),
        (
            Uuid::parse_str("aaaaaaaa-0003-0000-0000-000000000003").unwrap(),
            "Alpha-3",
            "1 59003U 26001C   26300.00000000  .00001000  00000-0  50000-4 0  9997",
            "2 59003  97.5000 359.0000 0010000  45.0000 315.0000 15.10000000 10004",
        ),
    ];

    for (sat_id, name, line1, line2) in orbit_configs {
        let tle = orbit::model::TleSet {
            name: name.to_string(),
            line1: line1.to_string(),
            line2: line2.to_string(),
        };
        let _ = orbit_svc.set_satellite_tle(sat_id, tle).await;
    }

    info!("✅ [PLATO GENESIS] SGP4 Constellation ephemeris populated for all satellites.");
    Ok(())
}
