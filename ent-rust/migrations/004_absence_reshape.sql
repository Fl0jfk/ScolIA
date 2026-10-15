-- Table canonique ent_absence (période + motif). DROP pour sortir de l'ancienne forme.

DROP TABLE IF EXISTS ent_absence;

CREATE TABLE ent_absence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    motif TEXT NOT NULL REFERENCES ent_absence_motif (code),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (fin >= debut)
);

CREATE INDEX ent_absence_eleve_periode_idx
    ON ent_absence (etablissement_id, eleve_id, debut, fin);

CREATE INDEX ent_absence_periode_idx
    ON ent_absence (etablissement_id, debut, fin);
