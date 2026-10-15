-- Infos complémentaires paramétrables (champs établissement + valeurs élève).

CREATE TABLE IF NOT EXISTS ent_info_comp_champ (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    type_valeur TEXT NOT NULL,
    obligatoire BOOLEAN NOT NULL DEFAULT false,
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_info_comp_champ_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_info_comp_champ_type_chk
        CHECK (type_valeur IN ('texte', 'nombre', 'booleen', 'date'))
);

CREATE TABLE IF NOT EXISTS ent_info_comp_valeur (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    champ_id UUID NOT NULL REFERENCES ent_info_comp_champ (id),
    valeur TEXT NOT NULL DEFAULT '',
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_info_comp_valeur_uq UNIQUE (etablissement_id, eleve_id, champ_id)
);

CREATE INDEX IF NOT EXISTS ent_info_comp_valeur_eleve_idx
    ON ent_info_comp_valeur (etablissement_id, eleve_id);
