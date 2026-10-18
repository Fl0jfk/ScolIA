-- Compétences / LSU — référentiel socle par établissement + évaluations élèves.
-- Le LSU ne se devine pas depuis une matière : il se saisit (niveau de maîtrise).
CREATE TABLE IF NOT EXISTS ent_competence_item (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    domaine TEXT NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    ordre INTEGER NOT NULL DEFAULT 0,
    actif BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_competence_item_code_uniq
    ON ent_competence_item (etablissement_id, code);

CREATE INDEX IF NOT EXISTS ent_competence_item_domaine_idx
    ON ent_competence_item (etablissement_id, domaine, ordre);

-- Une évaluation par (élève, item, période) — ré-évaluer met à jour le niveau.
CREATE TABLE IF NOT EXISTS ent_competence_evaluation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    item_id UUID NOT NULL REFERENCES ent_competence_item (id) ON DELETE CASCADE,
    periode_id UUID NOT NULL,
    niveau TEXT NOT NULL,
    commentaire TEXT,
    auteur_user_id UUID NOT NULL,
    maj_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_competence_evaluation_niveau_chk
        CHECK (niveau IN ('insuffisante', 'fragile', 'satisfaisante', 'tres_bonne'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_competence_evaluation_uniq
    ON ent_competence_evaluation (etablissement_id, eleve_id, item_id, periode_id);

CREATE INDEX IF NOT EXISTS ent_competence_evaluation_eleve_idx
    ON ent_competence_evaluation (etablissement_id, eleve_id, periode_id);
