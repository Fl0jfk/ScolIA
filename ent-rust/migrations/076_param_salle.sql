-- Paramètres établissement + salles / locaux.

CREATE TABLE IF NOT EXISTS ent_param_etab (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    cle TEXT NOT NULL,
    valeur TEXT NOT NULL DEFAULT '',
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_param_etab_cle_uq UNIQUE (etablissement_id, cle)
);

CREATE INDEX IF NOT EXISTS ent_param_etab_etab_idx
    ON ent_param_etab (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_salle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    capacite INTEGER,
    site TEXT,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_salle_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_salle_capacite_chk CHECK (capacite IS NULL OR (capacite >= 0 AND capacite <= 500))
);

CREATE INDEX IF NOT EXISTS ent_salle_etab_idx
    ON ent_salle (etablissement_id)
    WHERE actif = true;
