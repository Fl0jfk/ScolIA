-- Compte ENT lié à un élève (face élève) — un user par élève dans l'établissement.

CREATE TABLE IF NOT EXISTS ent_eleve_compte (
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, eleve_id),
    CONSTRAINT ent_eleve_compte_user_uq UNIQUE (etablissement_id, user_id)
);

CREATE INDEX IF NOT EXISTS ent_eleve_compte_user_idx
    ON ent_eleve_compte (etablissement_id, user_id);
