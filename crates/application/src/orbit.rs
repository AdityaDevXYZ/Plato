use crate::error::ApplicationError;
use orbit::{
    model::*,
    astrodynamics,
    repository::OrbitalRepository,
};
use std::sync::Arc;
use uuid::Uuid;
use time::OffsetDateTime;

pub struct OrbitalQueryService {
    repo: Arc<dyn OrbitalRepository>,
}

impl OrbitalQueryService {
    pub fn new(repo: Arc<dyn OrbitalRepository>) -> Self {
        Self { repo }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_current_position(&self, id: Uuid) -> Result<SatelliteOrbit, ApplicationError> {
        let mut orbit = self.repo.get_orbit(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Orbit not found".into()))?;

        if let Some(tle) = &orbit.tle {
            let now = OffsetDateTime::now_utc();
            if let Ok((pos, vel, geodetic)) = astrodynamics::propagate_sgp4(tle, now) {
                orbit.position = pos;
                orbit.velocity = vel;
                orbit.geodetic = Some(geodetic);
                orbit.timestamp = now;
            }
        }

        Ok(orbit)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_predictions(&self, id: Uuid, minutes: u32, step_mins: u32) -> Result<Vec<OrbitalPrediction>, ApplicationError> {
        let orbit = self.repo.get_orbit(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Orbit not found".into()))?;

        if let Some(tle) = &orbit.tle {
            let now = OffsetDateTime::now_utc();
            let steps = (minutes / step_mins.max(1)).clamp(1, 720);
            let mut preds = Vec::with_capacity(steps as usize);

            for i in 1..=steps {
                let target_time = now + time::Duration::minutes((i * step_mins) as i64);
                if let Ok((pos, vel, geodetic)) = astrodynamics::propagate_sgp4(tle, target_time) {
                    preds.push(OrbitalPrediction {
                        satellite_id: id,
                        target_time,
                        predicted_position: pos,
                        predicted_velocity: vel,
                        geodetic: Some(geodetic),
                        confidence: (1.0 - (i as f32 * 0.0005)).max(0.85),
                    });
                }
            }
            Ok(preds)
        } else {
            self.repo.get_predictions(id).await.map_err(|e| ApplicationError::Repository(e.to_string()))
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_communication_windows(&self) -> Result<Vec<CommunicationWindow>, ApplicationError> {
        self.repo.get_windows(OffsetDateTime::now_utc()).await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_satellite_passes(
        &self,
        id: Uuid,
        gs_lat: f64,
        gs_lon: f64,
        gs_alt_m: f64,
        min_el_deg: f64,
        hours: u32,
    ) -> Result<Vec<GroundPass>, ApplicationError> {
        let orbit = self.repo.get_orbit(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Orbit not found".into()))?;

        let tle = orbit.tle.ok_or_else(|| ApplicationError::Validation("Satellite has no TLE configured".into()))?;
        let now = OffsetDateTime::now_utc();

        let passes = astrodynamics::predict_passes(
            id,
            &tle,
            Uuid::nil(),
            "Target Ground Station",
            gs_lat,
            gs_lon,
            gs_alt_m,
            min_el_deg,
            now,
            hours.clamp(1, 72),
            30,
        );

        Ok(passes)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_look_angles(
        &self,
        id: Uuid,
        gs_lat: f64,
        gs_lon: f64,
        gs_alt_m: f64,
    ) -> Result<LookAngles, ApplicationError> {
        let orbit = self.repo.get_orbit(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Orbit not found".into()))?;

        let tle = orbit.tle.ok_or_else(|| ApplicationError::Validation("Satellite has no TLE configured".into()))?;
        let now = OffsetDateTime::now_utc();

        let (pos, vel, _) = astrodynamics::propagate_sgp4(&tle, now)
            .map_err(|e| ApplicationError::Domain(e))?;

        let look = astrodynamics::calculate_look_angles(&pos, &vel, now, gs_lat, gs_lon, gs_alt_m, 2.2e9);
        Ok(look)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_formation_membership(&self, id: Uuid) -> Result<Vec<FormationMember>, ApplicationError> {
        let target = self.repo.get_orbit(id).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Target orbit not found".into()))?;

        let all = self.repo.get_all_orbits().await.map_err(|e| ApplicationError::Repository(e.to_string()))?;
        let mut members = Vec::new();

        for candidate in all {
            if candidate.satellite_id == id {
                continue;
            }

            let sma_diff = (candidate.elements.semi_major_axis - target.elements.semi_major_axis).abs();
            let inc_diff = (candidate.elements.inclination - target.elements.inclination).abs();

            if sma_diff <= 100.0 && inc_diff <= 2.0 {
                members.push(FormationMember {
                    satellite_id: candidate.satellite_id,
                    semi_major_axis_diff_km: sma_diff,
                    inclination_diff_deg: inc_diff,
                    in_plane: inc_diff <= 0.2,
                });
            }
        }

        Ok(members)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_nearby_satellites(&self, id: Uuid, radius_km: f64) -> Result<Vec<NearbySatellite>, ApplicationError> {
        let target = self.get_current_position(id).await?;
        let all = self.repo.get_all_orbits().await.map_err(|e| ApplicationError::Repository(e.to_string()))?;
        let now = OffsetDateTime::now_utc();
        let mut nearby = Vec::new();

        for candidate in all {
            if candidate.satellite_id == id {
                continue;
            }

            let (cand_pos, cand_vel) = if let Some(tle) = &candidate.tle {
                if let Ok((pos, vel, _)) = astrodynamics::propagate_sgp4(tle, now) {
                    (pos, vel)
                } else {
                    (candidate.position.clone(), candidate.velocity.clone())
                }
            } else {
                (candidate.position.clone(), candidate.velocity.clone())
            };

            let dx = cand_pos.x - target.position.x;
            let dy = cand_pos.y - target.position.y;
            let dz = cand_pos.z - target.position.z;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            if distance <= radius_km {
                let dvx = cand_vel.x - target.velocity.x;
                let dvy = cand_vel.y - target.velocity.y;
                let dvz = cand_vel.z - target.velocity.z;
                let rel_vel = (dvx * dvx + dvy * dvy + dvz * dvz).sqrt();

                nearby.push(NearbySatellite {
                    satellite_id: candidate.satellite_id,
                    distance_km: distance,
                    relative_velocity_km_s: rel_vel,
                });
            }
        }

        nearby.sort_by(|a, b| a.distance_km.partial_cmp(&b.distance_km).unwrap());
        Ok(nearby)
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_satellite_tle(&self, id: Uuid, tle: TleSet) -> Result<SatelliteOrbit, ApplicationError> {
        let elements_sgp4 = astrodynamics::parse_tle(&tle)
            .map_err(|e| ApplicationError::Validation(e))?;

        let now = OffsetDateTime::now_utc();
        let (pos, vel, geodetic) = astrodynamics::propagate_sgp4(&tle, now)
            .map_err(|e| ApplicationError::Domain(e))?;

        // Kepler's Third Law: a = (mu / n^2)^(1/3) where mu_earth = 398600.4418 km^3/s^2
        let n_rad_s = elements_sgp4.mean_motion * (2.0 * std::f64::consts::PI / 86400.0);
        let semi_major_axis_km = (398600.4418 / (n_rad_s * n_rad_s)).cbrt();

        let elements = OrbitalElements {
            semi_major_axis: semi_major_axis_km,
            eccentricity: elements_sgp4.eccentricity,
            inclination: elements_sgp4.inclination.to_degrees(),
            raan: elements_sgp4.right_ascension.to_degrees(),
            arg_of_perigee: elements_sgp4.argument_of_perigee.to_degrees(),
            true_anomaly: elements_sgp4.mean_anomaly.to_degrees(),
        };

        let orbit = SatelliteOrbit {
            satellite_id: id,
            elements,
            position: pos,
            velocity: vel,
            geodetic: Some(geodetic),
            tle: Some(tle),
            timestamp: now,
            prediction_horizon_sec: 86400,
            health: OrbitalHealth::Nominal,
        };

        self.repo.save_orbit(&orbit).await.map_err(|e| ApplicationError::Repository(e.to_string()))?;
        Ok(orbit)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ground_stations(&self) -> Result<Vec<GroundStation>, ApplicationError> {
        self.repo.get_ground_stations().await.map_err(|e| ApplicationError::Repository(e.to_string()))
    }
}
