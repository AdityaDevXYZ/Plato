use types::MissionId;
use uuid::Uuid;
use time::OffsetDateTime;
use serde::{Serialize, Deserialize};

fn default_temperature() -> f32 {
    20.0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationalStatus { Active, Inactive, Maintenance, Decommissioned }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommunicationStatus { Connected, Disconnected, Degraded }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Satellite {
    pub id: Uuid,
    pub mission_id: MissionId,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub norad_id: Option<u32>,
    pub software_version: String,
    pub operational_status: OperationalStatus,
    pub health_score: u8,
    pub battery_level: f32,
    pub storage_usage: f32,
    pub memory_usage: f32,
    pub cpu_utilization: f32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(with = "time::serde::iso8601")]
    pub last_contact: OffsetDateTime,
    pub comm_status: CommunicationStatus,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialSatelliteTelemetry {
    pub name: Option<String>,
    pub norad_id: Option<u32>,
    pub software_version: Option<String>,
    pub operational_status: Option<OperationalStatus>,
    pub health_score: Option<u8>,
    pub battery_level: Option<f32>,
    pub storage_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub cpu_utilization: Option<f32>,
    pub temperature: Option<f32>,
    pub comm_status: Option<CommunicationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    pub id: Uuid,
    pub satellite_id: Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub recorded_at: OffsetDateTime,
    pub battery_level: f32,
    pub cpu_utilization: f32,
    pub memory_usage: f32,
    pub storage_usage: f32,
    pub temperature: f32,
    pub health_score: u8,
    pub comm_status: CommunicationStatus,
    pub operational_status: OperationalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
}

impl Satellite {
    pub fn new(id: Uuid, mission_id: MissionId, name: String, norad_id: Option<u32>, software_version: String) -> Self {
        Self {
            id,
            mission_id,
            name,
            norad_id,
            software_version,
            operational_status: OperationalStatus::Active,
            health_score: 100,
            battery_level: 100.0,
            storage_usage: 5.0,
            memory_usage: 12.0,
            cpu_utilization: 3.0,
            temperature: 20.0,
            last_contact: OffsetDateTime::now_utc(),
            comm_status: CommunicationStatus::Connected,
            version: 1,
        }
    }

    pub fn calculate_health(&self) -> u8 {
        let mut score: i32 = 100;

        // Battery penalties
        if self.battery_level < 20.0 {
            score -= 40;
        } else if self.battery_level < 40.0 {
            score -= 20;
        } else if self.battery_level < 60.0 {
            score -= 10;
        }

        // CPU penalties
        if self.cpu_utilization > 95.0 {
            score -= 30;
        } else if self.cpu_utilization > 80.0 {
            score -= 15;
        }

        // Memory penalties
        if self.memory_usage > 95.0 {
            score -= 25;
        } else if self.memory_usage > 80.0 {
            score -= 10;
        }

        // Storage penalties
        if self.storage_usage > 95.0 {
            score -= 20;
        } else if self.storage_usage > 85.0 {
            score -= 10;
        }

        // Thermal penalties (nominal spacecraft bus range is -10C to 50C)
        if self.temperature > 65.0 || self.temperature < -20.0 {
            score -= 30;
        } else if self.temperature > 50.0 || self.temperature < -10.0 {
            score -= 15;
        }

        // Communication link penalties
        match self.comm_status {
            CommunicationStatus::Disconnected => score -= 35,
            CommunicationStatus::Degraded => score -= 15,
            CommunicationStatus::Connected => (),
        }

        // Operational status penalties
        match self.operational_status {
            OperationalStatus::Active => (),
            OperationalStatus::Maintenance => score -= 20,
            OperationalStatus::Inactive => score -= 50,
            OperationalStatus::Decommissioned => score = 0,
        }

        score.clamp(0, 100) as u8
    }

    pub fn evaluate_anomalies(&self, prev_health: u8) -> Vec<AnomalyReport> {
        let mut anomalies = Vec::new();

        if self.battery_level < 20.0 {
            anomalies.push(AnomalyReport {
                anomaly_type: "CRITICAL_BATTERY_DEPLETION".into(),
                severity: "CRITICAL".into(),
                description: format!("Battery level is critically low at {:.1}%", self.battery_level),
            });
        }

        if self.cpu_utilization > 95.0 {
            anomalies.push(AnomalyReport {
                anomaly_type: "HIGH_CPU_SATURATION".into(),
                severity: "WARNING".into(),
                description: format!("CPU utilization saturated at {:.1}%", self.cpu_utilization),
            });
        }

        if self.temperature > 65.0 || self.temperature < -20.0 {
            anomalies.push(AnomalyReport {
                anomaly_type: "THERMAL_EXCURSION".into(),
                severity: "CRITICAL".into(),
                description: format!("Bus temperature out of safe limits: {:.1}°C", self.temperature),
            });
        }

        if self.comm_status == CommunicationStatus::Disconnected {
            anomalies.push(AnomalyReport {
                anomaly_type: "COMMUNICATION_LOSS".into(),
                severity: "CRITICAL".into(),
                description: "Ground uplink/downlink telemetry carrier lost".into(),
            });
        }

        if prev_health > self.health_score && (prev_health - self.health_score >= 20 || self.health_score < 50) {
            anomalies.push(AnomalyReport {
                anomaly_type: "TELEMETRY_REGRESSION".into(),
                severity: "CRITICAL".into(),
                description: format!("Spacecraft health score regressed from {} to {}", prev_health, self.health_score),
            });
        }

        anomalies
    }

    pub fn apply_partial(&mut self, partial: PartialSatelliteTelemetry) {
        if let Some(v) = partial.name { self.name = v; }
        if let Some(v) = partial.norad_id { self.norad_id = Some(v); }
        if let Some(v) = partial.software_version { self.software_version = v; }
        if let Some(v) = partial.operational_status { self.operational_status = v; }
        if let Some(v) = partial.battery_level { self.battery_level = v; }
        if let Some(v) = partial.storage_usage { self.storage_usage = v; }
        if let Some(v) = partial.memory_usage { self.memory_usage = v; }
        if let Some(v) = partial.cpu_utilization { self.cpu_utilization = v; }
        if let Some(v) = partial.temperature { self.temperature = v; }
        if let Some(v) = partial.comm_status { self.comm_status = v; }

        if let Some(v) = partial.health_score {
            self.health_score = v;
        } else {
            self.health_score = self.calculate_health();
        }

        self.last_contact = OffsetDateTime::now_utc();
        self.version += 1;
    }

    pub fn to_telemetry_record(&self) -> TelemetryRecord {
        TelemetryRecord {
            id: Uuid::new_v4(),
            satellite_id: self.id,
            recorded_at: self.last_contact,
            battery_level: self.battery_level,
            cpu_utilization: self.cpu_utilization,
            memory_usage: self.memory_usage,
            storage_usage: self.storage_usage,
            temperature: self.temperature,
            health_score: self.health_score,
            comm_status: self.comm_status.clone(),
            operational_status: self.operational_status.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundStation {
    pub id: Uuid,
    pub availability: bool,
    pub connectivity: String,
    pub current_sessions: u32,
    pub maintenance_state: bool,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwinEntity {
    Satellite(Satellite),
    GroundStation(GroundStation),
    MissionAsset(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nominal_satellite_health() {
        let sat = Satellite::new(Uuid::new_v4(), Uuid::new_v4().into(), "SAT-1".into(), Some(25544), "v1.0.0".into());
        assert_eq!(sat.health_score, 100);
        assert_eq!(sat.calculate_health(), 100);
        assert!(sat.evaluate_anomalies(100).is_empty());
    }

    #[test]
    fn test_battery_depletion_anomaly() {
        let mut sat = Satellite::new(Uuid::new_v4(), Uuid::new_v4().into(), "SAT-1".into(), None, "v1.0.0".into());
        sat.apply_partial(PartialSatelliteTelemetry {
            battery_level: Some(15.0),
            ..Default::default()
        });

        assert_eq!(sat.battery_level, 15.0);
        assert!(sat.health_score <= 60);
        let anomalies = sat.evaluate_anomalies(100);
        assert!(anomalies.iter().any(|a| a.anomaly_type == "CRITICAL_BATTERY_DEPLETION"));
        assert!(anomalies.iter().any(|a| a.anomaly_type == "TELEMETRY_REGRESSION"));
    }

    #[test]
    fn test_thermal_and_comm_loss_anomalies() {
        let mut sat = Satellite::new(Uuid::new_v4(), Uuid::new_v4().into(), "SAT-2".into(), None, "v1.0.0".into());
        sat.apply_partial(PartialSatelliteTelemetry {
            temperature: Some(72.5),
            comm_status: Some(CommunicationStatus::Disconnected),
            ..Default::default()
        });

        let anomalies = sat.evaluate_anomalies(100);
        assert!(anomalies.iter().any(|a| a.anomaly_type == "THERMAL_EXCURSION"));
        assert!(anomalies.iter().any(|a| a.anomaly_type == "COMMUNICATION_LOSS"));
    }

    #[test]
    fn test_telemetry_record_generation() {
        let sat = Satellite::new(Uuid::new_v4(), Uuid::new_v4().into(), "SAT-3".into(), None, "v1.0.0".into());
        let record = sat.to_telemetry_record();
        assert_eq!(record.satellite_id, sat.id);
        assert_eq!(record.health_score, 100);
        assert_eq!(record.temperature, 20.0);
    }
}
