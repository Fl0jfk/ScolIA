-- Cahier de texte : travail pour une classe, visible élève / parent liés.

CREATE TABLE IF NOT EXISTS ent_cahier_texte (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    classe TEXT NOT NULL,
    matiere_libelle TEXT NOT NULL,
    titre TEXT NOT NULL,
    consigne TEXT NOT NULL,
    pour_le DATE NOT NULL,
    cree_le DATE NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_cahier_texte_classe_idx
    ON ent_cahier_texte (etablissement_id, classe, pour_le);

CREATE INDEX IF NOT EXISTS ent_cahier_texte_pour_le_idx
    ON ent_cahier_texte (etablissement_id, pour_le);
