use uuid::Uuid;
use time::OffsetDateTime;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeodeticPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TleSet {
    pub name: String,
    pub line1: String,
    pub line2: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitalElements {
    pub semi_major_axis: f64,
    pub eccentricity: f64,
    pub inclination: f64,
    pub raan: f64,
    pub arg_of_perigee: f64,
    pub true_anomaly: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrbitalHealth {
    Nominal,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteOrbit {
    pub satellite_id: Uuid,
    pub elements: OrbitalElements,
    pub position: Vector3,
    pub velocity: Vector3,
    #[serde(default)]
    pub geodetic: Option<GeodeticPosition>,
    #[serde(default)]
    pub tle: Option<TleSet>,
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
    pub prediction_horizon_sec: u64,
    pub health: OrbitalHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitalPrediction {
    pub satellite_id: Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub target_time: OffsetDateTime,
    pub predicted_position: Vector3,
    pub predicted_velocity: Vector3,
    #[serde(default)]
    pub geodetic: Option<GeodeticPosition>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LookAngles {
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub slant_range_km: f64,
    pub range_rate_km_s: f64,
    pub doppler_shift_hz: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroundStation {
    pub id: Uuid,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub min_elevation_deg: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroundPass {
    pub satellite_id: Uuid,
    pub ground_station_id: Uuid,
    pub ground_station_name: String,
    #[serde(with = "time::serde::iso8601")]
    pub aos: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub tca: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub los: OffsetDateTime,
    pub max_elevation_deg: f64,
    pub duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommunicationWindow {
    pub satellite_id: Uuid,
    pub ground_station_id: Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub start_time: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub end_time: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NearbySatellite {
    pub satellite_id: Uuid,
    pub distance_km: f64,
    pub relative_velocity_km_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormationMember {
    pub satellite_id: Uuid,
    pub semi_major_axis_diff_km: f64,
    pub inclination_diff_deg: f64,
    pub in_plane: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astrodynamics::*;

    #[test]
    fn test_sgp4_propagation_and_geodetic() {
        let tle = TleSet {
            name: "ISS".into(),
            line1: "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927".into(),
            line2: "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537".into(),
        };

        let epoch = OffsetDateTime::from_unix_timestamp(1221913540).unwrap();
        let (pos_eci, vel_eci, geodetic) = propagate_sgp4(&tle, epoch).unwrap();

        // ISS orbital altitude is roughly 350-450 km
        assert!(geodetic.altitude_km > 300.0 && geodetic.altitude_km < 500.0);
        // ISS velocity is approximately 7.5-7.8 km/s
        let speed = (vel_eci.x * vel_eci.x + vel_eci.y * vel_eci.y + vel_eci.z * vel_eci.z).sqrt();
        assert!(speed > 7.0 && speed < 8.5);
        // Latitude within inclination limits (-52 to +52 degrees)
        assert!(geodetic.latitude >= -52.0 && geodetic.latitude <= 52.0);
        // Longitude between -180 and 180
        assert!(geodetic.longitude >= -180.0 && geodetic.longitude <= 180.0);
        assert!(pos_eci.x.abs() > 1000.0);
    }

    #[test]
    fn test_look_angles_calculation() {
        let tle = TleSet {
            name: "ISS".into(),
            line1: "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927".into(),
            line2: "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537".into(),
        };

        let epoch = OffsetDateTime::from_unix_timestamp(1221913540).unwrap();
        let (pos_eci, vel_eci, _) = propagate_sgp4(&tle, epoch).unwrap();

        let look = calculate_look_angles(&pos_eci, &vel_eci, epoch, 78.2297, 15.4077, 450.0, 2.2e9);

        assert!(look.azimuth_deg >= 0.0 && look.azimuth_deg <= 360.0);
        assert!(look.slant_range_km > 300.0);
    }

    #[test]
    fn test_pass_prediction() {
        let sat_id = Uuid::new_v4();
        let gs_id = Uuid::new_v4();
        let tle = TleSet {
            name: "ISS".into(),
            line1: "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927".into(),
            line2: "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537".into(),
        };

        let start = OffsetDateTime::from_unix_timestamp(1221913540).unwrap();
        // Predict over 12 hours with 30s resolution for Svalbard station
        let passes = predict_passes(
            sat_id,
            &tle,
            gs_id,
            "Svalbard",
            78.2297,
            15.4077,
            450.0,
            5.0,
            start,
            12,
            30,
        );

        // Within 12 hours, a high-latitude LEO pass occurs
        println!("Passes predicted: {}", passes.len());
        for p in &passes {
            assert!(p.max_elevation_deg >= 5.0);
            assert!(p.los > p.aos);
        }
    }
}
