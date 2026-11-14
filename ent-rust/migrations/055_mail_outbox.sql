-- Journal des mails sortants (carnet, seuil absences, courrier psy…).
-- Dry-run lab et envois live partagent la même table pour audit.

CREATE TABLE IF NOT EXISTS ent_mail_outbox (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID,
    kind TEXT NOT NULL,
    to_email TEXT NOT NULL,
    subject TEXT NOT NULL,
    body_text TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    source_aggregate TEXT,
    source_id UUID,
    message_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_mail_outbox_status_chk CHECK (
        status IN ('dry_run', 'sent', 'failed', 'skipped_no_config')
    )
);

CREATE INDEX IF NOT EXISTS ent_mail_outbox_etab_idx
    ON ent_mail_outbox (etablissement_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ent_mail_outbox_source_idx
    ON ent_mail_outbox (etablissement_id, source_id)
    WHERE source_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ent_mail_outbox_kind_idx
    ON ent_mail_outbox (etablissement_id, kind, created_at DESC);
