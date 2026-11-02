-- Migration 0019: Asynchronous Deployment Queue & Telecommand Space Frames
ALTER TABLE deployment_sessions
ADD COLUMN IF NOT EXISTS release_id UUID REFERENCES releases(id) ON DELETE CASCADE,
ADD COLUMN IF NOT EXISTS status VARCHAR(32) NOT NULL DEFAULT 'Queued',
ADD COLUMN IF NOT EXISTS rollback_status VARCHAR(32) NOT NULL DEFAULT 'None',
ADD COLUMN IF NOT EXISTS failure_reason TEXT,
ADD COLUMN IF NOT EXISTS started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE INDEX IF NOT EXISTS idx_deployment_sessions_status ON deployment_sessions(status);
CREATE INDEX IF NOT EXISTS idx_deployment_sessions_release_id ON deployment_sessions(release_id);

CREATE TABLE IF NOT EXISTS telecommand_packets (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES deployment_sessions(id) ON DELETE CASCADE,
    sequence_number INT NOT NULL,
    packet_type VARCHAR(32) NOT NULL,
    target_partition VARCHAR(32) NOT NULL,
    frame_hex TEXT NOT NULL,
    crc32 BIGINT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'Queued',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_telecommand_packets_session ON telecommand_packets(session_id, sequence_number);
