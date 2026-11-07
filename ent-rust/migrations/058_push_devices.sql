-- Devices push (apps natives / web) + journal des envois.

CREATE TABLE IF NOT EXISTS ent_push_device (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    token TEXT NOT NULL,
    app_id TEXT,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    CONSTRAINT ent_push_device_platform_chk CHECK (
        platform IN ('ios', 'android', 'web')
    ),
    CONSTRAINT ent_push_device_token_uq UNIQUE (etablissement_id, token)
);

CREATE INDEX IF NOT EXISTS ent_push_device_user_idx
    ON ent_push_device (etablissement_id, user_id)
    WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS ent_push_outbox (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID,
    user_id UUID,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    platform TEXT,
    token TEXT,
    status TEXT NOT NULL,
    error TEXT,
    source_aggregate TEXT,
    source_id UUID,
    message_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_push_outbox_status_chk CHECK (
        status IN ('dry_run', 'sent', 'failed', 'skipped_no_config')
    )
);

CREATE INDEX IF NOT EXISTS ent_push_outbox_etab_idx
    ON ent_push_outbox (etablissement_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ent_push_outbox_kind_idx
    ON ent_push_outbox (etablissement_id, kind, created_at DESC);

CREATE INDEX IF NOT EXISTS ent_push_outbox_source_idx
    ON ent_push_outbox (etablissement_id, source_id, kind)
    WHERE source_id IS NOT NULL;
