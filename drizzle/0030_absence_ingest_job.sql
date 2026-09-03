-- Jobs d’import PDF absences : état en Postgres (plus de absences/ingest-jobs/*.json sur S3).
-- Le PDF source reste sur S3 (colonne document_key).

CREATE TABLE IF NOT EXISTS "absence_ingest_job" (
  "job_id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "user_id" text NOT NULL,
  "creator_name" text NOT NULL DEFAULT '',
  "creator_email" text NOT NULL DEFAULT '',
  "creator_roles" text[] NOT NULL DEFAULT '{}',
  "status" text NOT NULL DEFAULT 'pending',
  "started_at" timestamp with time zone NOT NULL,
  "updated_at" timestamp with time zone NOT NULL,
  "source_file_name" text NOT NULL DEFAULT '',
  "document_key" text NOT NULL,
  "processing_started_at" timestamp with time zone,
  "phase" text,
  "error" text,
  "code" text,
  "created_payload" text,
  "parsed_payload" text,
  "locked_at" timestamp with time zone,
  "locked_by" text
);

CREATE INDEX IF NOT EXISTS "absence_ingest_job_etab_idx"
  ON "absence_ingest_job" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "absence_ingest_job_status_idx"
  ON "absence_ingest_job" ("etablissement_id", "status");
