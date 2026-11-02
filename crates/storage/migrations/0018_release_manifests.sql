-- Migration 0018: Release Flight Manifests & RAE Normalized Columns
CREATE TABLE IF NOT EXISTS release_manifests (
    release_id UUID PRIMARY KEY REFERENCES releases(id) ON DELETE CASCADE,
    version VARCHAR(64) NOT NULL,
    target_architecture VARCHAR(64) NOT NULL,
    max_binary_size_bytes BIGINT NOT NULL DEFAULT 33554432,
    dual_bank_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE assurance_reports
ADD COLUMN IF NOT EXISTS decision VARCHAR(32),
ADD COLUMN IF NOT EXISTS confidence_score SMALLINT,
ADD COLUMN IF NOT EXISTS evaluated_at TIMESTAMPTZ DEFAULT NOW();
