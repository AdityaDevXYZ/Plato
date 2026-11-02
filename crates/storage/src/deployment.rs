use async_trait::async_trait;
use sqlx::PgPool;
use deployment::{
    model::{DeploymentSession, TelecommandPacket},
    repository::{DeploymentSessionRepository, RepositoryError},
};
use uuid::Uuid;

pub struct SqlxDeploymentRepository {
    pool: PgPool,
}

impl SqlxDeploymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    #[allow(dead_code)]
    id: Uuid,
    data: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct PacketRow {
    id: Uuid,
    session_id: Uuid,
    sequence_number: i32,
    packet_type: String,
    target_partition: String,
    frame_hex: String,
    crc32: i64,
    status: String,
}

#[async_trait]
impl DeploymentSessionRepository for SqlxDeploymentRepository {
    async fn save(&self, session: &DeploymentSession) -> Result<(), RepositoryError> {
        let id = session.session_id;
        let release_id: Uuid = session.release_id.into();
        let data = serde_json::to_value(session)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let status_str = format!("{:?}", session.status);
        let rollback_str = format!("{:?}", session.rollback_status);

        sqlx::query(
            "INSERT INTO deployment_sessions (id, release_id, status, rollback_status, failure_reason, started_at, completed_at, data, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
             ON CONFLICT (id) DO UPDATE SET
                 status = $3,
                 rollback_status = $4,
                 failure_reason = $5,
                 started_at = $6,
                 completed_at = $7,
                 data = $8,
                 updated_at = NOW()"
        )
        .bind(id)
        .bind(release_id)
        .bind(status_str)
        .bind(rollback_str)
        .bind(&session.failure_reason)
        .bind(session.started_at)
        .bind(session.completed_at)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, session_id: Uuid) -> Result<Option<DeploymentSession>, RepositoryError> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT id, data FROM deployment_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        if let Some(r) = row {
            let session: DeploymentSession = serde_json::from_value(r.data)
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    async fn find_queued_sessions(&self, limit: i64) -> Result<Vec<DeploymentSession>, RepositoryError> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, data FROM deployment_sessions WHERE status IN ('Queued', 'Initializing') ORDER BY created_at ASC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        let mut sessions = Vec::with_capacity(rows.len());
        for r in rows {
            let s: DeploymentSession = serde_json::from_value(r.data)
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
            sessions.push(s);
        }
        Ok(sessions)
    }

    async fn find_recent_sessions(&self, limit: i64) -> Result<Vec<DeploymentSession>, RepositoryError> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, data FROM deployment_sessions ORDER BY created_at DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        let mut sessions = Vec::with_capacity(rows.len());
        for r in rows {
            let s: DeploymentSession = serde_json::from_value(r.data)
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
            sessions.push(s);
        }
        Ok(sessions)
    }

    async fn save_telecommand_packets(&self, packets: &[TelecommandPacket]) -> Result<(), RepositoryError> {
        for packet in packets {
            let seq = packet.sequence_number as i32;
            let crc = packet.crc32 as i64;
            sqlx::query(
                "INSERT INTO telecommand_packets (id, session_id, sequence_number, packet_type, target_partition, frame_hex, crc32, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (id) DO UPDATE SET
                     status = $8"
            )
            .bind(packet.id)
            .bind(packet.session_id)
            .bind(seq)
            .bind(&packet.packet_type)
            .bind(&packet.target_partition)
            .bind(&packet.frame_hex)
            .bind(crc)
            .bind(&packet.status)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_packets_by_session_id(&self, session_id: Uuid) -> Result<Vec<TelecommandPacket>, RepositoryError> {
        let rows = sqlx::query_as::<_, PacketRow>(
            "SELECT id, session_id, sequence_number, packet_type, target_partition, frame_hex, crc32, status
             FROM telecommand_packets WHERE session_id = $1 ORDER BY sequence_number ASC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        let packets = rows
            .into_iter()
            .map(|r| TelecommandPacket {
                id: r.id,
                session_id: r.session_id,
                sequence_number: r.sequence_number as u32,
                packet_type: r.packet_type,
                target_partition: r.target_partition,
                frame_hex: r.frame_hex,
                crc32: r.crc32 as u32,
                status: r.status,
            })
            .collect();

        Ok(packets)
    }
}
