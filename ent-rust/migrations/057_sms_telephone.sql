-- Téléphone responsables (SMS) + journal SMS sortants.

ALTER TABLE ent_user
    ADD COLUMN IF NOT EXISTS telephone TEXT;

CREATE INDEX IF NOT EXISTS ent_user_telephone_idx
    ON ent_user (telephone)
    WHERE telephone IS NOT NULL AND trim(telephone) <> '';

CREATE TABLE IF NOT EXISTS ent_sms_outbox (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID,
    kind TEXT NOT NULL,
    to_phone TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    source_aggregate TEXT,
    source_id UUID,
    message_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sms_outbox_status_chk CHECK (
        status IN ('dry_run', 'sent', 'failed', 'skipped_no_config')
    )
);

CREATE INDEX IF NOT EXISTS ent_sms_outbox_etab_idx
    ON ent_sms_outbox (etablissement_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ent_sms_outbox_source_idx
    ON ent_sms_outbox (etablissement_id, source_id, kind)
    WHERE source_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ent_sms_outbox_kind_idx
    ON ent_sms_outbox (etablissement_id, kind, created_at DESC);
