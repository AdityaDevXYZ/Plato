-- Migration 0016: Historical Telemetry Records & Thermal Telemetry
ALTER TABLE twin_satellites
ADD COLUMN IF NOT EXISTS temperature REAL DEFAULT 20.0;

CREATE TABLE IF NOT EXISTS satellite_telemetry_history (
    id UUID PRIMARY KEY,
    satellite_id UUID NOT NULL REFERENCES twin_satellites(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    battery_level REAL NOT NULL,
    cpu_utilization REAL NOT NULL,
    memory_usage REAL NOT NULL,
    storage_usage REAL NOT NULL,
    temperature REAL DEFAULT 20.0,
    health_score SMALLINT NOT NULL,
    comm_status VARCHAR(32) NOT NULL,
    operational_status VARCHAR(32) NOT NULL,
    raw_payload JSONB
);

CREATE INDEX IF NOT EXISTS idx_telemetry_history_sat_time ON satellite_telemetry_history(satellite_id, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_telemetry_history_recorded_at ON satellite_telemetry_history(recorded_at DESC);
