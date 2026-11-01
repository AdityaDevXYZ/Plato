use async_trait::async_trait;
use sqlx::PgPool;
use twin::{
    model::{Satellite, TelemetryRecord, OperationalStatus, CommunicationStatus},
    repository::{TwinRepository, RepositoryError},
};
use types::MissionId;
use uuid::Uuid;

pub struct SqlxDigitalTwinRepository { pool: PgPool }
impl SqlxDigitalTwinRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

#[derive(sqlx::FromRow)]
struct SatRow { id: Uuid, mission_id: Uuid, data: serde_json::Value }

#[derive(sqlx::FromRow)]
struct HistoryRow {
    id: Uuid,
    satellite_id: Uuid,
    recorded_at: time::OffsetDateTime,
    battery_level: f32,
    cpu_utilization: f32,
    memory_usage: f32,
    storage_usage: f32,
    temperature: Option<f32>,
    health_score: i16,
    comm_status: String,
    operational_status: String,
}

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
               (id, mission_id, name, norad_id, health_score, operational_status, battery_level, cpu_utilization, memory_usage, storage_usage, temperature, comm_status, software_version, last_contact, version, data) 
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) 
               ON CONFLICT (id) DO UPDATE SET 
               mission_id = $2, name = $3, norad_id = $4, health_score = $5, operational_status = $6, battery_level = $7, cpu_utilization = $8, memory_usage = $9, storage_usage = $10, temperature = $11, comm_status = $12, software_version = $13, last_contact = $14, version = $15, data = $16"#
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
        .bind(sat.temperature)
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

    async fn record_telemetry(&self, record: &TelemetryRecord) -> Result<(), RepositoryError> {
        let comms = format!("{:?}", record.comm_status);
        let status = format!("{:?}", record.operational_status);
        sqlx::query(
            r#"INSERT INTO satellite_telemetry_history
               (id, satellite_id, recorded_at, battery_level, cpu_utilization, memory_usage, storage_usage, temperature, health_score, comm_status, operational_status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#
        )
        .bind(record.id)
        .bind(record.satellite_id)
        .bind(record.recorded_at)
        .bind(record.battery_level)
        .bind(record.cpu_utilization)
        .bind(record.memory_usage)
        .bind(record.storage_usage)
        .bind(record.temperature)
        .bind(record.health_score as i16)
        .bind(comms)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_telemetry_history(&self, satellite_id: Uuid, limit: i64) -> Result<Vec<TelemetryRecord>, RepositoryError> {
        let rows = sqlx::query_as::<_, HistoryRow>(
            r#"SELECT id, satellite_id, recorded_at, battery_level, cpu_utilization, memory_usage, storage_usage, temperature, health_score, comm_status, operational_status
               FROM satellite_telemetry_history
               WHERE satellite_id = $1
               ORDER BY recorded_at DESC
               LIMIT $2"#
        )
        .bind(satellite_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| {
            let comm_status = match r.comm_status.as_str() {
                "Connected" => CommunicationStatus::Connected,
                "Disconnected" => CommunicationStatus::Disconnected,
                _ => CommunicationStatus::Degraded,
            };
            let operational_status = match r.operational_status.as_str() {
                "Active" => OperationalStatus::Active,
                "Inactive" => OperationalStatus::Inactive,
                "Maintenance" => OperationalStatus::Maintenance,
                _ => OperationalStatus::Decommissioned,
            };
            TelemetryRecord {
                id: r.id,
                satellite_id: r.satellite_id,
                recorded_at: r.recorded_at,
                battery_level: r.battery_level,
                cpu_utilization: r.cpu_utilization,
                memory_usage: r.memory_usage,
                storage_usage: r.storage_usage,
                temperature: r.temperature.unwrap_or(20.0),
                health_score: r.health_score as u8,
                comm_status,
                operational_status,
            }
        }).collect())
    }
}
