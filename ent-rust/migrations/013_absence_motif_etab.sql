-- Motifs d'absence propres à un établissement (en plus du référentiel global).
-- La FK globale sur ent_absence.motif est retirée : validation app = global ∪ etab.

CREATE TABLE IF NOT EXISTS ent_absence_motif_etab (
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, code),
    CONSTRAINT ent_absence_motif_etab_code_chk CHECK (code ~ '^[a-z0-9_]{2,40}$')
);

ALTER TABLE ent_absence DROP CONSTRAINT IF EXISTS ent_absence_motif_fkey;
