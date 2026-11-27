-- Vannes documentaires réinscription N+1.
-- L'établissement pose des exigences (code) ; le parent dépose un doc famille
-- portant le même code ; la confirmation est bloquée tant qu'il manque un
-- document obligatoire.

ALTER TABLE ent_eleve_document
    ADD COLUMN IF NOT EXISTS code TEXT;

CREATE INDEX IF NOT EXISTS ent_eleve_document_code_idx
    ON ent_eleve_document (etablissement_id, eleve_id, code)
    WHERE code IS NOT NULL;

CREATE TABLE IF NOT EXISTS ent_reinscription_exigence_doc (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_cible TEXT NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    tiroir TEXT NOT NULL DEFAULT 'famille',
    obligatoire BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_reinscription_exigence_uq
        UNIQUE (etablissement_id, annee_cible, code),
    CONSTRAINT ent_reinscription_exigence_tiroir_chk
        CHECK (tiroir IN ('scolaire', 'administratif', 'sante', 'famille', 'autre'))
);

CREATE INDEX IF NOT EXISTS ent_reinscription_exigence_campagne_idx
    ON ent_reinscription_exigence_doc (etablissement_id, annee_cible);
