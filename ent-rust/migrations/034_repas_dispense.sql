-- Dispense / prévenance jour repas : hors facturation + hors présent cantine.
-- Unique actif (etab, élève, activité, jour) ; annulation soft (annulee).

CREATE TABLE IF NOT EXISTS ent_eleve_dispense_repas (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    activite_id UUID NOT NULL REFERENCES ent_repas_activite (id) ON DELETE RESTRICT,
    jour DATE NOT NULL,
    motif TEXT NOT NULL,
    source TEXT NOT NULL,
    annulee BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_dispense_repas_source_chk
        CHECK (source IN ('parent', 'staff')),
    CONSTRAINT ent_eleve_dispense_repas_motif_chk
        CHECK (char_length(trim(motif)) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_dispense_repas_actif_uq
    ON ent_eleve_dispense_repas (etablissement_id, eleve_id, activite_id, jour)
    WHERE annulee = false;

CREATE INDEX IF NOT EXISTS ent_eleve_dispense_repas_eleve_idx
    ON ent_eleve_dispense_repas (etablissement_id, eleve_id, jour);

CREATE INDEX IF NOT EXISTS ent_eleve_dispense_repas_periode_idx
    ON ent_eleve_dispense_repas (etablissement_id, eleve_id, jour)
    WHERE annulee = false;
