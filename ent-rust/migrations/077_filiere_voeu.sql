-- Filières / MEF + vœux inscription élève.

CREATE TABLE IF NOT EXISTS ent_filiere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    mef TEXT,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_filiere_code_uq UNIQUE (etablissement_id, code)
);

CREATE INDEX IF NOT EXISTS ent_filiere_etab_idx
    ON ent_filiere (etablissement_id)
    WHERE actif = true;

CREATE TABLE IF NOT EXISTS ent_voeu_inscription (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    filiere_id UUID NOT NULL REFERENCES ent_filiere (id) ON DELETE CASCADE,
    rang SMALLINT NOT NULL,
    annee_scolaire TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_voeu_rang_chk CHECK (rang BETWEEN 1 AND 5),
    CONSTRAINT ent_voeu_uq UNIQUE (etablissement_id, eleve_id, annee_scolaire, rang),
    CONSTRAINT ent_voeu_filiere_uq UNIQUE (etablissement_id, eleve_id, annee_scolaire, filiere_id)
);

CREATE INDEX IF NOT EXISTS ent_voeu_eleve_idx
    ON ent_voeu_inscription (etablissement_id, eleve_id, annee_scolaire);
