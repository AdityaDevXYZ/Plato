use crate::error::ApplicationError;
use twin::{
    model::{Satellite, PartialSatelliteTelemetry, TelemetryRecord, AnomalyReport},
    repository::TwinRepository,
    events::*,
};
use types::MissionId;
use std::sync::Arc;
use eventbus::{EventPublisher, EventMessage};
use uuid::Uuid;

pub struct TwinQueryService {
    repo: Arc<dyn TwinRepository>,
}

impl TwinQueryService {
    pub fn new(repo: Arc<dyn TwinRepository>) -> Self {
        Self { repo }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_satellite(&self, id: Uuid) -> Result<Satellite, ApplicationError> {
        self.repo.get_satellite(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Satellite not found".into()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mission_assets(&self, mission_id: MissionId) -> Result<Vec<Satellite>, ApplicationError> {
        self.repo.get_mission_assets(mission_id).await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_all_satellites(&self) -> Result<Vec<Satellite>, ApplicationError> {
        self.repo.get_all_satellites().await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_unhealthy_satellites(&self) -> Result<Vec<Satellite>, ApplicationError> {
        self.repo.get_unhealthy_satellites(50).await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_online_satellites(&self) -> Result<Vec<Satellite>, ApplicationError> {
        self.repo.get_online_satellites().await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_deployable_satellites(&self) -> Result<Vec<Satellite>, ApplicationError> {
        self.repo.get_deployable_satellites().await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_telemetry_history(&self, satellite_id: Uuid, limit: i64) -> Result<Vec<TelemetryRecord>, ApplicationError> {
        self.repo.get_telemetry_history(satellite_id, limit).await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }
}

pub struct TwinUpdateService {
    repo: Arc<dyn TwinRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl TwinUpdateService {
    pub fn new(repo: Arc<dyn TwinRepository>, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { repo, event_publisher }
    }

    #[tracing::instrument(skip(self))]
    pub async fn register_satellite(
        &self,
        id: Uuid,
        mission_id: MissionId,
        name: String,
        norad_id: Option<u32>,
        software_version: String,
    ) -> Result<Satellite, ApplicationError> {
        let sat = Satellite::new(id, mission_id, name, norad_id, software_version);
        self.repo.save_satellite(&sat).await.map_err(|e| ApplicationError::Repository(e.to_string()))?;
        
        let record = sat.to_telemetry_record();
        let _ = self.repo.record_telemetry(&record).await;

        let event = EventMessage::new(
            Box::new(TwinCreated { entity_id: sat.id, entity_type: "Satellite".into() }),
            Uuid::new_v4(),
            "plato.twin",
        );
        let _ = self.event_publisher.publish(vec![event]);
        Ok(sat)
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_telemetry(&self, id: Uuid, telemetry: PartialSatelliteTelemetry) -> Result<Satellite, ApplicationError> {
        let (sat, _) = self.ingest_and_evaluate(id, telemetry).await?;
        Ok(sat)
    }

    #[tracing::instrument(skip(self))]
    pub async fn ingest_and_evaluate(&self, id: Uuid, telemetry: PartialSatelliteTelemetry) -> Result<(Satellite, Vec<AnomalyReport>), ApplicationError> {
        let mut sat = self.repo.get_satellite(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Satellite not found".into()))?;
        
        let old_health = sat.health_score;
        sat.apply_partial(telemetry);
        
        let anomalies = sat.evaluate_anomalies(old_health);

        // Persist state to digital twin table
        self.repo.save_satellite(&sat).await.map_err(|e| ApplicationError::Repository(e.to_string()))?;
        
        // Persist time-series telemetry record
        let record = sat.to_telemetry_record();
        let _ = self.repo.record_telemetry(&record).await;

        let mut events = vec![EventMessage::new(Box::new(TwinUpdated { entity_id: id, version: sat.version }), Uuid::new_v4(), "plato.twin")];
        if sat.health_score != old_health {
            events.push(EventMessage::new(Box::new(SatelliteHealthChanged { entity_id: id, new_score: sat.health_score }), Uuid::new_v4(), "plato.twin"));
        }

        for anomaly in &anomalies {
            events.push(EventMessage::new(
                Box::new(SatelliteAnomalyDetected {
                    entity_id: id,
                    anomaly_type: anomaly.anomaly_type.clone(),
                    severity: anomaly.severity.clone(),
                    description: anomaly.description.clone(),
                }),
                Uuid::new_v4(),
                "plato.twin",
            ));
        }
        
        let _ = self.event_publisher.publish(events);
        
        Ok((sat, anomalies))
    }
}
