-- Foyers + liens parent ↔ élève (droits fins CONFIGURABLE).
-- Payeur / parts 100 % viendront ensuite ; ici = accès lecture / justif pour tests croisés.

CREATE TABLE IF NOT EXISTS ent_foyer (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    libelle TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_foyer_etab_idx
    ON ent_foyer (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_eleve_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    foyer_id UUID REFERENCES ent_foyer (id) ON DELETE SET NULL,
    lien TEXT NOT NULL DEFAULT 'autre',
    peut_voir_notes BOOLEAN NOT NULL DEFAULT true,
    peut_voir_vs BOOLEAN NOT NULL DEFAULT true,
    peut_justifier_absence BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_lien_unique UNIQUE (etablissement_id, eleve_id, user_id)
);

CREATE INDEX IF NOT EXISTS ent_eleve_lien_user_idx
    ON ent_eleve_lien (etablissement_id, user_id);

CREATE INDEX IF NOT EXISTS ent_eleve_lien_eleve_idx
    ON ent_eleve_lien (etablissement_id, eleve_id);
