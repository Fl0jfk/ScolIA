-- Socle 2 (l'élève), lot E6 : détection de doublons d'élèves (ELV-22) et mémoire des paires
-- déclarées distinctes (ELV-33, R-E20).
--
-- `ent_eleve_doublon` est un CACHE de la détection (reconstruit par la tâche nocturne ou à la
-- demande de la direction) : une paire (A, B) avec A < B, un score, les raisons (codes), un
-- statut. Rien n'est supprimé : une paire qui ne se détecte plus devient `obsolete`.
-- `ent_eleve_non_doublon` garde la décision « ce sont bien deux élèves » (jumeaux, homonymes)
-- avec les VERSIONS D'IDENTITÉ courantes au moment de la décision : la détection ignore la
-- paire tant qu'aucune des deux identités ne change (R-E20).
-- Ces tables désignent les deux élèves par `eleve_a` / `eleve_b` (jamais `eleve_id`) : elles ne
-- sont pas déplacées par une fusion, la fusion met leurs lignes à jour explicitement.

CREATE TABLE IF NOT EXISTS ent_eleve_doublon (
    id               UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_a          UUID NOT NULL,
    eleve_b          UUID NOT NULL,
    score            SMALLINT NOT NULL,
    raisons          TEXT[] NOT NULL DEFAULT '{}',
    statut           TEXT NOT NULL DEFAULT 'a_examiner',
    detecte_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_doublon_ordre_chk CHECK (eleve_a < eleve_b),
    CONSTRAINT ent_eleve_doublon_score_chk CHECK (score BETWEEN 0 AND 100),
    CONSTRAINT ent_eleve_doublon_statut_chk CHECK (statut IN ('a_examiner', 'fusionne', 'obsolete')),
    FOREIGN KEY (etablissement_id, eleve_a) REFERENCES ent_eleve (etablissement_id, id),
    FOREIGN KEY (etablissement_id, eleve_b) REFERENCES ent_eleve (etablissement_id, id),
    UNIQUE (etablissement_id, eleve_a, eleve_b)
);
CREATE INDEX IF NOT EXISTS ent_eleve_doublon_file_idx
    ON ent_eleve_doublon (etablissement_id, statut, score DESC);

CREATE TABLE IF NOT EXISTS ent_eleve_non_doublon (
    id                 UUID PRIMARY KEY,
    etablissement_id   UUID NOT NULL,
    eleve_a            UUID NOT NULL,
    eleve_b            UUID NOT NULL,
    motif_code         TEXT NOT NULL,
    identite_a         UUID,
    identite_b         UUID,
    decide_par         UUID,
    decide_le          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_non_doublon_ordre_chk CHECK (eleve_a < eleve_b),
    CONSTRAINT ent_eleve_non_doublon_motif_chk CHECK (motif_code IN ('jumeaux', 'homonymes', 'fratrie', 'autre')),
    FOREIGN KEY (etablissement_id, eleve_a) REFERENCES ent_eleve (etablissement_id, id),
    FOREIGN KEY (etablissement_id, eleve_b) REFERENCES ent_eleve (etablissement_id, id),
    UNIQUE (etablissement_id, eleve_a, eleve_b)
);

-- Dernier passage de la détection (une fois par jour et par établissement, tâche nocturne).
CREATE TABLE IF NOT EXISTS ent_eleve_doublon_passage (
    etablissement_id UUID PRIMARY KEY,
    dernier_jour     DATE NOT NULL,
    nb_paires        INTEGER NOT NULL DEFAULT 0,
    passe_le         TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_doublon', 'ent_eleve_non_doublon', 'ent_eleve_doublon_passage'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format($p$CREATE POLICY ent_rls_etablissement ON %I FOR ALL
            USING (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
            WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))$p$, t);
    END LOOP;
END $$;
