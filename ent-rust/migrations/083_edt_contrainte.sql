-- Contraintes EDT soft (indispos / préférences) — pas un solveur.

CREATE TABLE IF NOT EXISTS ent_edt_contrainte (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    kind TEXT NOT NULL,
    prof_user_id UUID REFERENCES ent_user (id),
    salle_code TEXT,
    jour_semaine SMALLINT,
    heure_debut TIME,
    heure_fin TIME,
    detail TEXT NOT NULL DEFAULT '',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_contrainte_kind_chk
        CHECK (kind IN ('prof_indispo', 'salle_indispo', 'preference')),
    CONSTRAINT ent_edt_contrainte_js_chk
        CHECK (jour_semaine IS NULL OR (jour_semaine BETWEEN 1 AND 7)),
    CONSTRAINT ent_edt_contrainte_heures_chk
        CHECK (
            heure_debut IS NULL
            OR heure_fin IS NULL
            OR heure_fin > heure_debut
        )
);

CREATE INDEX IF NOT EXISTS ent_edt_contrainte_etab_idx
    ON ent_edt_contrainte (etablissement_id);
