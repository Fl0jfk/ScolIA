-- Messagerie établissement ↔ familles (P0) : fil lié à un élève + messages.

CREATE TABLE IF NOT EXISTS ent_msg_fil (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    sujet TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_msg_fil_etab_eleve_idx
    ON ent_msg_fil (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_msg_fil_etab_last_idx
    ON ent_msg_fil (etablissement_id, last_message_at DESC);

CREATE TABLE IF NOT EXISTS ent_msg (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    fil_id UUID NOT NULL REFERENCES ent_msg_fil (id),
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    corps TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_msg_fil_idx
    ON ent_msg (etablissement_id, fil_id, created_at);
