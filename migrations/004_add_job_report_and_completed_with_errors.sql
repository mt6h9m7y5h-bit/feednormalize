ALTER TYPE job_status ADD VALUE IF NOT EXISTS 'completed_with_errors';

ALTER TABLE jobs ADD COLUMN IF NOT EXISTS job_report JSONB;
