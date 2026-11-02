use async_trait::async_trait;
use sqlx::PgPool;
use assurance::{
    model::{AssuranceReport, FlightReleaseManifest},
    repository::{AssuranceReportRepository, FlightManifestRepository, RepositoryError},
};
use types::ReleaseId;
use uuid::Uuid;

pub struct SqlxAssuranceRepository {
    pool: PgPool,
}

impl SqlxAssuranceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ReportRow {
    #[allow(dead_code)]
    release_id: Uuid,
    data: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct ManifestRow {
    #[allow(dead_code)]
    release_id: Uuid,
    data: serde_json::Value,
}

#[async_trait]
impl AssuranceReportRepository for SqlxAssuranceRepository {
    async fn save(&self, report: &AssuranceReport) -> Result<(), RepositoryError> {
        let rel_id: Uuid = report.release_id.into();
        let data = serde_json::to_value(report)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let decision_str = format!("{:?}", report.decision);
        let confidence = report.confidence_score as i16;

        sqlx::query(
            "INSERT INTO assurance_reports (release_id, data, decision, confidence_score, evaluated_at)
             VALUES ($1, $2, $3, $4, NOW())
             ON CONFLICT (release_id) DO UPDATE SET data = $2, decision = $3, confidence_score = $4, evaluated_at = NOW()"
        )
        .bind(rel_id)
        .bind(data)
        .bind(decision_str)
        .bind(confidence)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_by_release_id(&self, release_id: ReleaseId) -> Result<Option<AssuranceReport>, RepositoryError> {
        let rel_id: Uuid = release_id.into();
        let row = sqlx::query_as::<_, ReportRow>("SELECT release_id, data FROM assurance_reports WHERE release_id = $1")
            .bind(rel_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        if let Some(r) = row {
            let report: AssuranceReport = serde_json::from_value(r.data)
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
            Ok(Some(report))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl FlightManifestRepository for SqlxAssuranceRepository {
    async fn save_manifest(&self, manifest: &FlightReleaseManifest) -> Result<(), RepositoryError> {
        let rel_id: Uuid = manifest.release_id.into();
        let data = serde_json::to_value(manifest)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let max_size = manifest.max_binary_size_bytes as i64;

        sqlx::query(
            "INSERT INTO release_manifests (release_id, version, target_architecture, max_binary_size_bytes, dual_bank_enabled, data, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (release_id) DO UPDATE SET
                 version = $2,
                 target_architecture = $3,
                 max_binary_size_bytes = $4,
                 dual_bank_enabled = $5,
                 data = $6"
        )
        .bind(rel_id)
        .bind(&manifest.version)
        .bind(&manifest.target_architecture)
        .bind(max_size)
        .bind(manifest.dual_bank_enabled)
        .bind(data)
        .bind(manifest.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_manifest_by_release_id(&self, release_id: ReleaseId) -> Result<Option<FlightReleaseManifest>, RepositoryError> {
        let rel_id: Uuid = release_id.into();
        let row = sqlx::query_as::<_, ManifestRow>("SELECT release_id, data FROM release_manifests WHERE release_id = $1")
            .bind(rel_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        if let Some(r) = row {
            let manifest: FlightReleaseManifest = serde_json::from_value(r.data)
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
            Ok(Some(manifest))
        } else {
            Ok(None)
        }
    }
}
