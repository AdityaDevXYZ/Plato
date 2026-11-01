use crate::model::{Vector3, GeodeticPosition, LookAngles, GroundPass, TleSet};
use time::OffsetDateTime;
use uuid::Uuid;

const WGS84_A: f64 = 6378.137; // Semi-major axis in km
const WGS84_F: f64 = 1.0 / 298.257223563; // Flattening
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F); // 6356.752314245 km
const WGS84_E2: f64 = WGS84_F * (2.0 - WGS84_F); // First eccentricity squared
const WGS84_E_PRIME2: f64 = (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);
const SPEED_OF_LIGHT_KM_S: f64 = 299792.458;
const EARTH_ROTATION_RATE_RAD_S: f64 = 7.2921151467e-5;

pub fn parse_tle(tle: &TleSet) -> Result<sgp4::Elements, String> {
    sgp4::Elements::from_tle(
        Some(tle.name.clone()),
        tle.line1.trim().as_bytes(),
        tle.line2.trim().as_bytes(),
    ).map_err(|e| format!("Invalid TLE: {:?}", e))
}

pub fn gmst_radians(time: OffsetDateTime) -> f64 {
    let unix_secs = time.unix_timestamp() as f64 + (time.nanosecond() as f64 / 1_000_000_000.0);
    let jd = 2440587.5 + (unix_secs / 86400.0);
    let d = jd - 2451545.0;
    // IAU-82 GMST expression in degrees
    let mut gmst_deg = (280.46061837 + 360.98564736629 * d) % 360.0;
    if gmst_deg < 0.0 {
        gmst_deg += 360.0;
    }
    gmst_deg.to_radians()
}

pub fn eci_to_ecef(eci: &Vector3, time: OffsetDateTime) -> Vector3 {
    let theta = gmst_radians(time);
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    Vector3 {
        x: eci.x * cos_t + eci.y * sin_t,
        y: -eci.x * sin_t + eci.y * cos_t,
        z: eci.z,
    }
}

pub fn ecef_to_geodetic(ecef: &Vector3) -> GeodeticPosition {
    let p = (ecef.x * ecef.x + ecef.y * ecef.y).sqrt();
    let lon_rad = ecef.y.atan2(ecef.x);
    let mut lon_deg = lon_rad.to_degrees();
    if lon_deg > 180.0 {
        lon_deg -= 360.0;
    }

    // Bowring's method for sub-millimeter precision geodetic latitude
    let theta_b = (ecef.z * WGS84_A).atan2(p * WGS84_B);
    let lat_rad = (ecef.z + WGS84_E_PRIME2 * WGS84_B * theta_b.sin().powi(3))
        .atan2(p - WGS84_E2 * WGS84_A * theta_b.cos().powi(3));
    let lat_deg = lat_rad.to_degrees();

    let sin_lat = lat_rad.sin();
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    let alt_km = (p / lat_rad.cos()) - n;

    GeodeticPosition {
        latitude: lat_deg,
        longitude: lon_deg,
        altitude_km: alt_km,
    }
}

pub fn geodetic_to_ecef(lat_deg: f64, lon_deg: f64, alt_km: f64) -> Vector3 {
    let lat_rad = lat_deg.to_radians();
    let lon_rad = lon_deg.to_radians();
    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    Vector3 {
        x: (n + alt_km) * cos_lat * cos_lon,
        y: (n + alt_km) * cos_lat * sin_lon,
        z: (n * (1.0 - WGS84_E2) + alt_km) * sin_lat,
    }
}

pub fn propagate_sgp4(
    tle: &TleSet,
    target_time: OffsetDateTime,
) -> Result<(Vector3, Vector3, GeodeticPosition), String> {
    let elements = parse_tle(tle)?;
    let constants = sgp4::Constants::from_elements(&elements)
        .map_err(|e| format!("Failed to initialize SGP4 constants: {:?}", e))?;

    let epoch_secs = elements.datetime.and_utc().timestamp() as f64
        + (elements.datetime.and_utc().timestamp_subsec_nanos() as f64 / 1_000_000_000.0);
    let target_secs = target_time.unix_timestamp() as f64
        + (target_time.nanosecond() as f64 / 1_000_000_000.0);

    let minutes_since_epoch = (target_secs - epoch_secs) / 60.0;
    let prediction = constants.propagate(sgp4::MinutesSinceEpoch(minutes_since_epoch))
        .map_err(|e| format!("SGP4 propagation error: {:?}", e))?;

    let pos_eci = Vector3 {
        x: prediction.position[0],
        y: prediction.position[1],
        z: prediction.position[2],
    };
    let vel_eci = Vector3 {
        x: prediction.velocity[0],
        y: prediction.velocity[1],
        z: prediction.velocity[2],
    };

    let pos_ecef = eci_to_ecef(&pos_eci, target_time);
    let geodetic = ecef_to_geodetic(&pos_ecef);

    Ok((pos_eci, vel_eci, geodetic))
}

pub fn calculate_look_angles(
    sat_pos_eci: &Vector3,
    sat_vel_eci: &Vector3,
    time: OffsetDateTime,
    gs_lat: f64,
    gs_lon: f64,
    gs_alt_m: f64,
    center_freq_hz: f64,
) -> LookAngles {
    let theta = gmst_radians(time);
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // Satellite in ECEF
    let r_ecef = Vector3 {
        x: sat_pos_eci.x * cos_t + sat_pos_eci.y * sin_t,
        y: -sat_pos_eci.x * sin_t + sat_pos_eci.y * cos_t,
        z: sat_pos_eci.z,
    };

    // Satellite velocity in ECEF: v_ecef = R_z(theta) * v_eci - omega x r_ecef
    let vx_rot = sat_vel_eci.x * cos_t + sat_vel_eci.y * sin_t;
    let vy_rot = -sat_vel_eci.x * sin_t + sat_vel_eci.y * cos_t;
    let vz_rot = sat_vel_eci.z;

    let v_ecef = Vector3 {
        x: vx_rot - (-EARTH_ROTATION_RATE_RAD_S * r_ecef.y),
        y: vy_rot - (EARTH_ROTATION_RATE_RAD_S * r_ecef.x),
        z: vz_rot,
    };

    // Ground Station in ECEF
    let gs_ecef = geodetic_to_ecef(gs_lat, gs_lon, gs_alt_m / 1000.0);

    // Relative vector rho = r_sat - r_gs
    let rho_vec = Vector3 {
        x: r_ecef.x - gs_ecef.x,
        y: r_ecef.y - gs_ecef.y,
        z: r_ecef.z - gs_ecef.z,
    };

    let slant_range = (rho_vec.x * rho_vec.x + rho_vec.y * rho_vec.y + rho_vec.z * rho_vec.z).sqrt();

    // Topocentric Horizon frame (South, East, Zenith)
    let lat_rad = gs_lat.to_radians();
    let lon_rad = gs_lon.to_radians();
    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    let s = sin_lat * cos_lon * rho_vec.x + sin_lat * sin_lon * rho_vec.y - cos_lat * rho_vec.z;
    let e = -sin_lon * rho_vec.x + cos_lon * rho_vec.y;
    let z = cos_lat * cos_lon * rho_vec.x + cos_lat * sin_lon * rho_vec.y + sin_lat * rho_vec.z;

    let el_rad = (z / slant_range).asin();
    let elevation_deg = el_rad.to_degrees();

    let az_rad = e.atan2(-s);
    let mut azimuth_deg = az_rad.to_degrees();
    if azimuth_deg < 0.0 {
        azimuth_deg += 360.0;
    }

    // Range rate = (rho_vec . v_ecef) / slant_range
    let range_rate = (rho_vec.x * v_ecef.x + rho_vec.y * v_ecef.y + rho_vec.z * v_ecef.z) / slant_range;

    // Doppler shift: delta_f = -f0 * (range_rate / c)
    let doppler_shift = -center_freq_hz * (range_rate / SPEED_OF_LIGHT_KM_S);

    LookAngles {
        azimuth_deg,
        elevation_deg,
        slant_range_km: slant_range,
        range_rate_km_s: range_rate,
        doppler_shift_hz: doppler_shift,
    }
}

pub fn predict_passes(
    satellite_id: Uuid,
    tle: &TleSet,
    gs_id: Uuid,
    gs_name: &str,
    gs_lat: f64,
    gs_lon: f64,
    gs_alt_m: f64,
    min_el_deg: f64,
    start_time: OffsetDateTime,
    duration_hours: u32,
    step_seconds: u32,
) -> Vec<GroundPass> {
    let mut passes = Vec::new();
    let total_steps = (duration_hours * 3600) / step_seconds.max(10);
    
    let mut in_pass = false;
    let mut current_aos: Option<OffsetDateTime> = None;
    let mut max_el: f64 = 0.0;
    let mut tca_time: Option<OffsetDateTime> = None;

    for i in 0..total_steps {
        let step_offset = time::Duration::seconds((i * step_seconds) as i64);
        let current_time = start_time + step_offset;

        if let Ok((pos_eci, vel_eci, _)) = propagate_sgp4(tle, current_time) {
            let look = calculate_look_angles(&pos_eci, &vel_eci, current_time, gs_lat, gs_lon, gs_alt_m, 2.2e9);

            if look.elevation_deg >= min_el_deg {
                if !in_pass {
                    // Acquisition of Signal
                    in_pass = true;
                    current_aos = Some(current_time);
                    max_el = look.elevation_deg;
                    tca_time = Some(current_time);
                } else if look.elevation_deg > max_el {
                    max_el = look.elevation_deg;
                    tca_time = Some(current_time);
                }
            } else if in_pass {
                // Loss of Signal
                in_pass = false;
                let aos = current_aos.unwrap_or(current_time);
                let los = current_time;
                let duration_seconds = (los.unix_timestamp() - aos.unix_timestamp()).max(0) as u32;

                passes.push(GroundPass {
                    satellite_id,
                    ground_station_id: gs_id,
                    ground_station_name: gs_name.to_string(),
                    aos,
                    tca: tca_time.unwrap_or(aos),
                    los,
                    max_elevation_deg: max_el,
                    duration_seconds,
                });

                current_aos = None;
                max_el = 0.0;
                tca_time = None;
            }
        }
    }

    passes
}
