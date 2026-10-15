-- EDT consultable (P0) + appel lié au créneau (pas « classe + jour » libre).

CREATE TABLE IF NOT EXISTS ent_edt_creneau (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    classe TEXT NOT NULL,
    matiere_id UUID,
    matiere_libelle TEXT NOT NULL,
    prof_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- ISO : 1 = lundi … 7 = dimanche
    jour_semaine SMALLINT NOT NULL,
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    salle TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_creneau_js_chk CHECK (jour_semaine BETWEEN 1 AND 7),
    CONSTRAINT ent_edt_creneau_heures_chk CHECK (heure_fin > heure_debut)
);

CREATE INDEX IF NOT EXISTS ent_edt_creneau_etab_js_idx
    ON ent_edt_creneau (etablissement_id, jour_semaine);

CREATE INDEX IF NOT EXISTS ent_edt_creneau_prof_idx
    ON ent_edt_creneau (etablissement_id, prof_user_id);

ALTER TABLE ent_appel
    ADD COLUMN IF NOT EXISTS creneau_id UUID REFERENCES ent_edt_creneau (id);

ALTER TABLE ent_appel
    ADD COLUMN IF NOT EXISTS matiere_libelle TEXT;

ALTER TABLE ent_appel
    DROP CONSTRAINT IF EXISTS ent_appel_classe_jour_uq;

CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_creneau_jour_uq
    ON ent_appel (etablissement_id, creneau_id, jour)
    WHERE creneau_id IS NOT NULL;
