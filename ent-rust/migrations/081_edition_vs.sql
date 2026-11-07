-- Éditions / documents VS — modèles paramétrables.

CREATE TABLE IF NOT EXISTS ent_edition_vs (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edition_vs_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_edition_vs_code_chk CHECK (char_length(code) BETWEEN 1 AND 40),
    CONSTRAINT ent_edition_vs_titre_chk CHECK (char_length(titre) BETWEEN 1 AND 200),
    CONSTRAINT ent_edition_vs_corps_chk CHECK (char_length(corps) BETWEEN 1 AND 20000)
);

CREATE INDEX IF NOT EXISTS ent_edition_vs_etab_idx
    ON ent_edition_vs (etablissement_id);
