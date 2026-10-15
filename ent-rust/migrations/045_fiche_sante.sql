-- Fiche santé élève (une par élève) — résumé famille + note confidentielle staff.

CREATE TABLE IF NOT EXISTS ent_fiche_sante (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    allergies TEXT NOT NULL DEFAULT '',
    traitements TEXT NOT NULL DEFAULT '',
    contre_indications TEXT NOT NULL DEFAULT '',
    pai_actif BOOLEAN NOT NULL DEFAULT false,
    medecin_nom TEXT,
    urgence_tel TEXT,
    note_confidentielle TEXT NOT NULL DEFAULT '',
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fiche_sante_eleve_uq UNIQUE (etablissement_id, eleve_id)
);

CREATE INDEX IF NOT EXISTS ent_fiche_sante_pai_idx
    ON ent_fiche_sante (etablissement_id)
    WHERE pai_actif = true;
