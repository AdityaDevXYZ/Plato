use async_trait::async_trait;
use audit::{AuditEntry, AuditError, AuditLedgerRepository, GENESIS_HASH};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct SqlxAuditRepository {
    pool: PgPool,
}

impl SqlxAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    sequence_number: i64,
    timestamp: OffsetDateTime,
    actor_id: Uuid,
    actor_email: String,
    action: String,
    resource_type: String,
    resource_id: String,
    details: serde_json::Value,
    prev_hash: String,
    entry_hash: String,
}

impl From<Row> for AuditEntry {
    fn from(r: Row) -> Self {
        AuditEntry {
            id: r.id,
            sequence_number: r.sequence_number,
            timestamp: r.timestamp,
            actor_id: r.actor_id,
            actor_email: r.actor_email,
            action: r.action,
            resource_type: r.resource_type,
            resource_id: r.resource_id,
            details: r.details,
            prev_hash: r.prev_hash,
            entry_hash: r.entry_hash,
        }
    }
}

#[async_trait]
impl AuditLedgerRepository for SqlxAuditRepository {
    async fn append_entry(
        &self,
        actor_id: Uuid,
        actor_email: String,
        action: String,
        resource_type: String,
        resource_id: String,
        details: serde_json::Value,
    ) -> Result<AuditEntry, AuditError> {
        let mut tx = self.pool.begin().await.map_err(|e| AuditError::Database(e.to_string()))?;

        // Lock latest row to ensure monotonic sequence and uninterrupted hash chain
        let latest = sqlx::query_as::<_, Row>(
            "SELECT id, sequence_number, timestamp, actor_id, actor_email, action, resource_type, resource_id, details, prev_hash, entry_hash
             FROM audit_ledger ORDER BY sequence_number DESC LIMIT 1 FOR UPDATE"
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AuditError::Database(e.to_string()))?;

        let (seq, prev_hash) = match latest {
            Some(l) => (l.sequence_number + 1, l.entry_hash),
            None => (1, GENESIS_HASH.to_string()),
        };

        let timestamp = OffsetDateTime::now_utc();
        let id = Uuid::new_v4();
        let entry_hash = AuditEntry::compute_hash(
            seq,
            timestamp.unix_timestamp(),
            actor_id,
            &actor_email,
            &action,
            &resource_type,
            &resource_id,
            &details,
            &prev_hash,
        );

        let row = sqlx::query_as::<_, Row>(
            "INSERT INTO audit_ledger (id, sequence_number, timestamp, actor_id, actor_email, action, resource_type, resource_id, details, prev_hash, entry_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING id, sequence_number, timestamp, actor_id, actor_email, action, resource_type, resource_id, details, prev_hash, entry_hash"
        )
        .bind(id)
        .bind(seq)
        .bind(timestamp)
        .bind(actor_id)
        .bind(&actor_email)
        .bind(&action)
        .bind(&resource_type)
        .bind(&resource_id)
        .bind(&details)
        .bind(&prev_hash)
        .bind(&entry_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AuditError::Database(e.to_string()))?;

        tx.commit().await.map_err(|e| AuditError::Database(e.to_string()))?;

        Ok(row.into())
    }

    async fn get_entries(&self, limit: i64, offset: i64) -> Result<Vec<AuditEntry>, AuditError> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, sequence_number, timestamp, actor_id, actor_email, action, resource_type, resource_id, details, prev_hash, entry_hash
             FROM audit_ledger ORDER BY sequence_number DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_latest_entry(&self) -> Result<Option<AuditEntry>, AuditError> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, sequence_number, timestamp, actor_id, actor_email, action, resource_type, resource_id, details, prev_hash, entry_hash
             FROM audit_ledger ORDER BY sequence_number DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AuditError::Database(e.to_string()))?;

        Ok(row.map(Into::into))
    }

    async fn verify_chain_integrity(&self) -> Result<bool, AuditError> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, sequence_number, timestamp, actor_id, actor_email, action, resource_type, resource_id, details, prev_hash, entry_hash
             FROM audit_ledger ORDER BY sequence_number ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Database(e.to_string()))?;

        let mut expected_prev = GENESIS_HASH.to_string();
        for r in rows {
            let entry: AuditEntry = r.into();
            if entry.prev_hash != expected_prev {
                return Err(AuditError::CorruptedChain(
                    entry.sequence_number,
                    format!("prev_hash mismatch: expected {}, got {}", expected_prev, entry.prev_hash),
                ));
            }
            if !entry.verify_hash() {
                return Err(AuditError::CorruptedChain(
                    entry.sequence_number,
                    format!("entry_hash recalculation mismatch at sequence {}", entry.sequence_number),
                ));
            }
            expected_prev = entry.entry_hash;
        }

        Ok(true)
    }
}
