-- Matières + notes : première verticale pédagogique pour scénarios multi-personas.
-- Ex. administratif crée « Maths » → prof saisit une note → parent lié la voit.

CREATE TABLE IF NOT EXISTS ent_matiere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    -- Provenance éventuelle nomenclature (SIECLE / Omogen) — texte libre pour l'instant.
    source_nomenclature TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_matiere_code_uq UNIQUE (etablissement_id, code)
);

CREATE INDEX IF NOT EXISTS ent_matiere_etab_idx
    ON ent_matiere (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_note (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    matiere_id UUID NOT NULL REFERENCES ent_matiere (id),
    valeur DOUBLE PRECISION NOT NULL,
    sur DOUBLE PRECISION NOT NULL DEFAULT 20,
    date_note DATE NOT NULL,
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    libelle TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_note_sur_chk CHECK (sur > 0)
);

CREATE INDEX IF NOT EXISTS ent_note_eleve_idx
    ON ent_note (etablissement_id, eleve_id, date_note);

CREATE INDEX IF NOT EXISTS ent_note_matiere_idx
    ON ent_note (etablissement_id, matiere_id);
