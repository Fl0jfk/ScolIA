-- Journal d'import EDT (CSV dry-run / apply + conflits détectés).

CREATE TABLE IF NOT EXISTS ent_edt_import (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    dry_run BOOLEAN NOT NULL DEFAULT true,
    lines_count INT NOT NULL,
    applied_count INT NOT NULL DEFAULT 0,
    conflicts_count INT NOT NULL DEFAULT 0,
    report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_edt_import_etab_idx
    ON ent_edt_import (etablissement_id, created_at DESC);
