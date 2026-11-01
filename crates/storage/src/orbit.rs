use async_trait::async_trait;
use sqlx::PgPool;
use orbit::{
    model::{SatelliteOrbit, OrbitalPrediction, CommunicationWindow, GroundStation},
    repository::{OrbitalRepository, RepositoryError},
};
use uuid::Uuid;
use time::OffsetDateTime;

pub struct SqlxOrbitRepository { pool: PgPool }
impl SqlxOrbitRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

#[derive(sqlx::FromRow)]
struct Row { id: Uuid, data: serde_json::Value }

#[derive(sqlx::FromRow)]
struct CommRow { data: serde_json::Value }

#[derive(sqlx::FromRow)]
struct GsRow {
    id: Uuid,
    name: String,
    latitude: f64,
    longitude: f64,
    altitude_m: f64,
    min_elevation_deg: f64,
    status: String,
}

#[async_trait]
impl OrbitalRepository for SqlxOrbitRepository {
    async fn save_orbit(&self, orbit: &SatelliteOrbit) -> Result<(), RepositoryError> {
        let data = serde_json::to_value(orbit).unwrap();
        let tle_line1 = orbit.tle.as_ref().map(|t| t.line1.clone());
        let tle_line2 = orbit.tle.as_ref().map(|t| t.line2.clone());
        let (lat, lon, alt) = if let Some(g) = &orbit.geodetic {
            (Some(g.latitude), Some(g.longitude), Some(g.altitude_km))
        } else {
            (None, None, None)
        };

        sqlx::query(
            r#"INSERT INTO satellite_orbits 
               (id, tle_line1, tle_line2, epoch, semi_major_axis_km, eccentricity, inclination_deg, raan_deg, arg_of_perigee_deg, mean_anomaly_deg, latitude, longitude, altitude_km, last_propagated_at, data) 
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) 
               ON CONFLICT (id) DO UPDATE SET 
               tle_line1 = $2, tle_line2 = $3, epoch = $4, semi_major_axis_km = $5, eccentricity = $6, inclination_deg = $7, raan_deg = $8, arg_of_perigee_deg = $9, mean_anomaly_deg = $10, latitude = $11, longitude = $12, altitude_km = $13, last_propagated_at = $14, data = $15"#
        )
        .bind(orbit.satellite_id)
        .bind(tle_line1)
        .bind(tle_line2)
        .bind(orbit.timestamp)
        .bind(orbit.elements.semi_major_axis)
        .bind(orbit.elements.eccentricity)
        .bind(orbit.elements.inclination)
        .bind(orbit.elements.raan)
        .bind(orbit.elements.arg_of_perigee)
        .bind(orbit.elements.true_anomaly)
        .bind(lat)
        .bind(lon)
        .bind(alt)
        .bind(orbit.timestamp)
        .bind(data)
        .execute(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_orbit(&self, id: Uuid) -> Result<Option<SatelliteOrbit>, RepositoryError> {
        let row = sqlx::query_as::<_, Row>("SELECT id, data FROM satellite_orbits WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        if let Some(r) = row { Ok(Some(serde_json::from_value(r.data).unwrap())) } else { Ok(None) }
    }

    async fn get_all_orbits(&self) -> Result<Vec<SatelliteOrbit>, RepositoryError> {
        let rows = sqlx::query_as::<_, Row>("SELECT id, data FROM satellite_orbits")
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }

    async fn save_prediction(&self, pred: &OrbitalPrediction) -> Result<(), RepositoryError> {
        let data = serde_json::to_value(pred).unwrap();
        sqlx::query("INSERT INTO orbital_predictions (id, target_time, data) VALUES ($1, $2, $3)")
            .bind(pred.satellite_id)
            .bind(pred.target_time)
            .bind(data)
            .execute(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_predictions(&self, id: Uuid) -> Result<Vec<OrbitalPrediction>, RepositoryError> {
        let rows = sqlx::query_as::<_, Row>("SELECT id, data FROM orbital_predictions WHERE id = $1 ORDER BY target_time ASC")
            .bind(id)
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }

    async fn save_window(&self, window: &CommunicationWindow) -> Result<(), RepositoryError> {
        let data = serde_json::to_value(window).unwrap();
        sqlx::query("INSERT INTO comm_windows (sat_id, ground_id, start_time, end_time, data) VALUES ($1, $2, $3, $4, $5)")
            .bind(window.satellite_id)
            .bind(window.ground_station_id)
            .bind(window.start_time)
            .bind(window.end_time)
            .bind(data)
            .execute(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_windows(&self, after: OffsetDateTime) -> Result<Vec<CommunicationWindow>, RepositoryError> {
        let rows = sqlx::query_as::<_, CommRow>("SELECT data FROM comm_windows WHERE end_time > $1 ORDER BY start_time ASC")
            .bind(after)
            .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| serde_json::from_value(r.data).unwrap()).collect())
    }

    async fn get_ground_stations(&self) -> Result<Vec<GroundStation>, RepositoryError> {
        let rows = sqlx::query_as::<_, GsRow>(
            "SELECT id, name, latitude, longitude, altitude_m, min_elevation_deg, status FROM ground_stations WHERE status = 'Active'"
        )
        .fetch_all(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| GroundStation {
            id: r.id,
            name: r.name,
            latitude: r.latitude,
            longitude: r.longitude,
            altitude_m: r.altitude_m,
            min_elevation_deg: r.min_elevation_deg,
            status: r.status,
        }).collect())
    }

    async fn save_ground_station(&self, gs: &GroundStation) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"INSERT INTO ground_stations (id, name, latitude, longitude, altitude_m, min_elevation_deg, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (id) DO UPDATE SET 
               name = $2, latitude = $3, longitude = $4, altitude_m = $5, min_elevation_deg = $6, status = $7"#
        )
        .bind(gs.id)
        .bind(&gs.name)
        .bind(gs.latitude)
        .bind(gs.longitude)
        .bind(gs.altitude_m)
        .bind(gs.min_elevation_deg)
        .bind(&gs.status)
        .execute(&self.pool).await.map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }
}
