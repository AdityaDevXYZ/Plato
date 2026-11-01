use async_trait::async_trait;
use sqlx::PgPool;
use twin::{model::{Satellite, OperationalStatus, CommunicationStatus}, repository::{TwinRepository, RepositoryError}};
use types::MissionId;
use uuid::Uuid;

pub struct SqlxDigitalTwinRepository { pool: PgPool }
impl SqlxDigitalTwinRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

#[derive(sqlx::FromRow)]
struct SatRow { id: Uuid, mission_id: Uuid, data: serde_json::Value }

#[async_trait]
impl TwinRepository for SqlxDigitalTwinRepository {
    async fn save_satellite(&self, sat: &Satellite) -> Result<(), RepositoryError> {
        let mission_id: Uuid = sat.mission_id.into();
        let data = serde_json::to_value(sat).unwrap();
        let status = format!("{:?}", sat.operational_status);
        let comms = format!("{:?}", sat.comm_status);
        let norad = sat.norad_id.map(|n| n as i32);

        sqlx::query(
            r#"INSERT INTO twin_satellites 
               (id, mission_id, name, norad_id, health_score, operational_status, battery_level, cpu_utilization, memory_usage, storage_usage, comm_status, software_version, last_contact, version, data) 
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) 
               ON CONFLICT (id) DO UPDATE SET 
               mission_id = $2, name = $3, norad_id = $4, health_score = $5, operational_status = $6, battery_level = $7, cpu_utilization = $8, memory_usage = $9, storage_usage = $10, comm_status = $11, software_version = $12, last_contact = $13, version = $14, data = $15"#
        )
        .bind(sat.id)
        .bind(mission_id)
        .bind(&sat.name)
        .bind(norad)
        .bind(sat.health_score as i16)
        .bind(status)
        .bind(sat.battery_level)
        .bind(sat.cpu_utilization)
        .bind(sat.memory_usage)
        .bind(sat.storage_usage)
        .bind(comms)
        .bind(&sat.software_version)
        .bind(sat.last_contact)
        .bind(sat.version as i64)
        .bind(data)
        .execute(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }
    async fn get_satellite(&self, id: Uuid) -> Result<Option<Satellite>, RepositoryError> {
        let row = sqlx::query_as::<_, SatRow>("SELECT id, mission_id, data FROM twin_satellites WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        if let Some(r) = row { Ok(Some(serde_json::from_value(r.data).unwrap())) } else { Ok(None) }
    }
    async fn get_mission_assets(&self, mission_id: MissionId) -> Result<Vec<Satellite>, RepositoryError> {
        let mid: Uuid = mission_id.into();
        let rows = sqlx::query_as::<_, SatRow>("SELECT id, mission_id, data FROM twin_satellites WHERE mission_id = $1")
            .bind(mid)
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }
    async fn get_all_satellites(&self) -> Result<Vec<Satellite>, RepositoryError> {
        let rows = sqlx::query_as::<_, SatRow>("SELECT id, mission_id, data FROM twin_satellites")
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }
    async fn get_unhealthy_satellites(&self, threshold: u8) -> Result<Vec<Satellite>, RepositoryError> {
        let rows = sqlx::query_as::<_, SatRow>("SELECT id, mission_id, data FROM twin_satellites WHERE health_score < $1")
            .bind(threshold as i16)
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }
    async fn get_online_satellites(&self) -> Result<Vec<Satellite>, RepositoryError> {
        let rows = sqlx::query_as::<_, SatRow>("SELECT id, mission_id, data FROM twin_satellites WHERE operational_status = 'Active' AND comm_status = 'Connected'")
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }
    async fn get_deployable_satellites(&self) -> Result<Vec<Satellite>, RepositoryError> {
        let rows = sqlx::query_as::<_, SatRow>("SELECT id, mission_id, data FROM twin_satellites WHERE operational_status = 'Active' AND health_score >= 80")
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }
}
