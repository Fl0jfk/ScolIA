-- Dossier psy structuré (une fiche par élève) — axes partageables + note / contacts staff.

CREATE TABLE IF NOT EXISTS ent_psy_dossier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    axes_suivi TEXT NOT NULL DEFAULT '',
    contacts_externes TEXT NOT NULL DEFAULT '',
    note_confidentielle TEXT NOT NULL DEFAULT '',
    suivi_actif BOOLEAN NOT NULL DEFAULT false,
    signale_famille BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_psy_dossier_eleve_uq UNIQUE (etablissement_id, eleve_id)
);

CREATE INDEX IF NOT EXISTS ent_psy_dossier_suivi_idx
    ON ent_psy_dossier (etablissement_id)
    WHERE suivi_actif = true;
