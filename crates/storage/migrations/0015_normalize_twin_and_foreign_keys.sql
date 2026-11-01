-- 0015: Normalize twin_satellites table and create operational indexes

ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS name VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS norad_id INT;
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS health_score SMALLINT NOT NULL DEFAULT 100;
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS operational_status VARCHAR(50) NOT NULL DEFAULT 'Active';
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS battery_level REAL NOT NULL DEFAULT 100.0;
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS cpu_utilization REAL NOT NULL DEFAULT 0.0;
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS memory_usage REAL NOT NULL DEFAULT 0.0;
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS storage_usage REAL NOT NULL DEFAULT 0.0;
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS comm_status VARCHAR(50) NOT NULL DEFAULT 'Connected';
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS software_version VARCHAR(255) NOT NULL DEFAULT 'v1.0.0';
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS last_contact TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE twin_satellites ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_twin_satellites_mission_id ON twin_satellites (mission_id);
CREATE INDEX IF NOT EXISTS idx_twin_satellites_health_score ON twin_satellites (health_score);
CREATE INDEX IF NOT EXISTS idx_twin_satellites_status ON twin_satellites (operational_status);
CREATE INDEX IF NOT EXISTS idx_twin_satellites_comm ON twin_satellites (comm_status);

CREATE INDEX IF NOT EXISTS idx_missions_project_id ON missions (project_id);
CREATE INDEX IF NOT EXISTS idx_projects_org_id ON projects (org_id);
