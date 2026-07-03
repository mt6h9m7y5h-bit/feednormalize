CREATE TYPE job_status AS ENUM ('queued', 'processing', 'finished', 'failed');

CREATE TABLE jobs (
    id UUID PRIMARY KEY,
    status job_status NOT NULL DEFAULT 'queued',
    format TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_jobs_status ON jobs (status);
CREATE INDEX idx_jobs_created_at ON jobs (created_at DESC);
