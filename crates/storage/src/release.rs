use async_trait::async_trait;
use sqlx::PgPool;
use release::{model::{Release, ReleaseStatus, ReleaseVersion, TransitionRecord}, repository::{ReleaseRepository, RepositoryError}};
use types::ReleaseId;
use uuid::Uuid;
use eventbus::EventMessage;

pub struct SqlxReleaseRepository { pool: PgPool }
impl SqlxReleaseRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

#[derive(sqlx::FromRow)]
struct Row { id: Uuid, mission_id: Uuid, version: String, status: String, history: serde_json::Value, version_num: i64 }

#[derive(sqlx::FromRow)]
struct IdRow { id: Uuid }

#[async_trait]
impl ReleaseRepository for SqlxReleaseRepository {
    async fn save(&self, rel: &mut Release, events: Vec<EventMessage>) -> Result<(), RepositoryError> {
        let id: Uuid = rel.id().into();
        let mission_id: Uuid = rel.mission_id().into();
        let version = rel.version().as_str();
        let status = format!("{:?}", rel.status());
        let history = serde_json::to_value(rel.history()).map_err(|e| RepositoryError::Database(e.to_string()))?;
        let current_version = rel.version_num as i64;
        let next_version = current_version + 1;

        let mut tx = self.pool.begin().await.map_err(|e| RepositoryError::Database(e.to_string()))?;

        let exists = sqlx::query_as::<_, IdRow>("SELECT id FROM releases WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx).await.map_err(|e| RepositoryError::Database(e.to_string()))?;

        if exists.is_some() {
            let res = sqlx::query("UPDATE releases SET mission_id = $1, version = $2, status = $3, history = $4, version_num = $5 WHERE id = $6 AND version_num = $7")
                .bind(mission_id)
                .bind(version)
                .bind(status)
                .bind(history)
                .bind(next_version)
                .bind(id)
                .bind(current_version)
                .execute(&mut *tx).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
            if res.rows_affected() == 0 {
                return Err(RepositoryError::Concurrency);
            }
        } else {
            sqlx::query("INSERT INTO releases (id, mission_id, version, status, history, version_num) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(id)
                .bind(mission_id)
                .bind(version)
                .bind(status)
                .bind(history)
                .bind(next_version)
                .execute(&mut *tx).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        }

        for ev in events {
            let p_json = serde_json::json!({
                "event_type": ev.event_type,
                "correlation_id": ev.correlation_id,
                "source": ev.source,
                "debug": format!("{:?}", ev.payload)
            });
            let evt_type = ev.event_type.clone();
            sqlx::query("INSERT INTO outbox_messages (id, event_type, payload, status, created_at, retry_count) VALUES ($1, $2, $3, 'PENDING', NOW(), 0)")
                .bind(ev.event_id)
                .bind(evt_type)
                .bind(p_json)
                .execute(&mut *tx).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        rel.version_num = next_version as u64;
        Ok(())
    }

    async fn find_by_id(&self, id: ReleaseId) -> Result<Option<Release>, RepositoryError> {
        let uuid: Uuid = id.into();
        let row = sqlx::query_as::<_, Row>("SELECT id, mission_id, version, status, history, version_num FROM releases WHERE id = $1")
            .bind(uuid)
            .fetch_optional(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        if let Some(r) = row {
            let status = match r.status.as_str() {
                "Validated" => ReleaseStatus::Validated,
                "Submitted" => ReleaseStatus::Submitted,
                "Approved" => ReleaseStatus::Approved,
                "Assured" => ReleaseStatus::Assured,
                "Ready" => ReleaseStatus::Ready,
                "Deploying" => ReleaseStatus::Deploying,
                "Completed" => ReleaseStatus::Completed,
                "Failed" => ReleaseStatus::Failed,
                "Cancelled" => ReleaseStatus::Cancelled,
                _ => ReleaseStatus::Draft,
            };
            let version = ReleaseVersion::new(r.version).map_err(|_| RepositoryError::Database("Invalid version format".into()))?;
            let history: Vec<TransitionRecord> = serde_json::from_value(r.history).map_err(|e| RepositoryError::Database(e.to_string()))?;
            Ok(Some(Release::reconstruct(r.id.into(), r.mission_id.into(), version, status, history, r.version_num as u64)))
        } else { Ok(None) }
    }
}
