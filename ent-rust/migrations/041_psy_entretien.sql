-- Entretiens psychologie scolaire (P0) : note strictement confidentielle.

CREATE TABLE IF NOT EXISTS ent_psy_entretien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    jour DATE NOT NULL,
    kind TEXT NOT NULL,
    duree_minutes INTEGER NOT NULL,
    note_confidentielle TEXT NOT NULL,
    signale_famille BOOLEAN NOT NULL DEFAULT false,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_psy_entretien_kind_chk
        CHECK (kind IN ('individuel', 'famille', 'orientation', 'crise')),
    CONSTRAINT ent_psy_entretien_duree_chk
        CHECK (duree_minutes >= 5 AND duree_minutes <= 240)
);

CREATE INDEX IF NOT EXISTS ent_psy_entretien_eleve_idx
    ON ent_psy_entretien (etablissement_id, eleve_id, jour DESC);

CREATE INDEX IF NOT EXISTS ent_psy_entretien_jour_idx
    ON ent_psy_entretien (etablissement_id, jour DESC);
