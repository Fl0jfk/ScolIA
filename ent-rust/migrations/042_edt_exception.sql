-- Exceptions EDT jour J : annulation / remplacement / changement de salle.

CREATE TABLE IF NOT EXISTS ent_edt_exception (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    creneau_id UUID NOT NULL REFERENCES ent_edt_creneau (id),
    jour DATE NOT NULL,
    kind TEXT NOT NULL,
    remplacant_user_id UUID REFERENCES ent_user (id),
    nouvelle_salle TEXT,
    motif TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_exception_kind_chk
        CHECK (kind IN ('annulation', 'remplacement', 'changement_salle')),
    CONSTRAINT ent_edt_exception_uq UNIQUE (etablissement_id, creneau_id, jour)
);

CREATE INDEX IF NOT EXISTS ent_edt_exception_jour_idx
    ON ent_edt_exception (etablissement_id, jour);

CREATE INDEX IF NOT EXISTS ent_edt_exception_remplacant_idx
    ON ent_edt_exception (etablissement_id, remplacant_user_id, jour)
    WHERE remplacant_user_id IS NOT NULL;
