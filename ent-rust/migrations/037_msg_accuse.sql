-- Accusés de lecture messagerie : un lecteur par message (idempotent).

CREATE TABLE IF NOT EXISTS ent_msg_accuse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    fil_id UUID NOT NULL REFERENCES ent_msg_fil (id),
    message_id UUID NOT NULL REFERENCES ent_msg (id),
    lecteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    lu_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, lecteur_user_id)
);

CREATE INDEX IF NOT EXISTS ent_msg_accuse_fil_idx
    ON ent_msg_accuse (etablissement_id, fil_id);

CREATE INDEX IF NOT EXISTS ent_msg_accuse_msg_idx
    ON ent_msg_accuse (etablissement_id, message_id);
