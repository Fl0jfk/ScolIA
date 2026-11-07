-- Passages infirmière (P0) : note de soins confidentielle, option visible famille.

CREATE TABLE IF NOT EXISTS ent_infirmerie_passage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    jour DATE NOT NULL,
    motif TEXT NOT NULL,
    note_soins TEXT NOT NULL,
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_infirmerie_passage_motif_chk
        CHECK (motif IN ('soins', 'malaise', 'accident', 'pai_suivi', 'autre'))
);

CREATE INDEX IF NOT EXISTS ent_infirmerie_passage_eleve_idx
    ON ent_infirmerie_passage (etablissement_id, eleve_id, jour DESC);

CREATE INDEX IF NOT EXISTS ent_infirmerie_passage_jour_idx
    ON ent_infirmerie_passage (etablissement_id, jour DESC);
