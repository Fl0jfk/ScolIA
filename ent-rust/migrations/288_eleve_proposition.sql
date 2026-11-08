-- Socle 2 (l'élève), lot E5 : propositions (§7.8 de 02_ELEVE.md).
--
-- PRINCIPE : aucune donnée venue de l'extérieur (import SIECLE, famille, élève, ancien
-- établissement, personnel qui signale une erreur) n'écrit directement dans la fiche. Elle
-- devient une PROPOSITION (valeur actuelle -> valeur proposée, source, pièce, statut),
-- acceptée par une règle automatique explicite ou par une personne habilitée.
--
-- Les valeurs (`actuel`, `propose`) sont SCELLÉES (champs_chiffres, `enc:v1:`) : un nom, une
-- date de naissance ou un prénom d'usage proposé n'est lisible ni en base ni dans les
-- journaux ni dans les événements. Une proposition décidée est immuable ; seule une
-- proposition en attente peut changer de statut, une seule fois.
--
-- L'élève inconnu proposé par un import (`eleve_id` NULL, cible `nouvel_eleve`) et les cibles
-- `scolarisation`, `inscription`, `coordonnee`, `photo` sont prévues par le schéma mais leur
-- application relève des lots suivants : le service les refuse explicitement tant qu'aucun
-- applicateur n'existe. Additif, idempotent.

CREATE TABLE IF NOT EXISTS ent_eleve_proposition (
    id                 UUID PRIMARY KEY,
    etablissement_id   UUID NOT NULL,
    eleve_id           UUID,
    cible              TEXT NOT NULL,
    source             TEXT NOT NULL,
    origine            TEXT,
    source_ref         UUID,
    depose_par         UUID,
    piece_document_id  UUID,
    commentaire        TEXT,
    statut             TEXT NOT NULL DEFAULT 'en_attente',
    regle_auto         TEXT,
    base_version_id    UUID,
    decision_code      TEXT,
    decision_motif     TEXT,
    decide_par         UUID,
    decide_le          TIMESTAMPTZ,
    cree_le            TIMESTAMPTZ NOT NULL DEFAULT now(),
    expire_le          TIMESTAMPTZ,
    CONSTRAINT ent_eleve_proposition_cible_chk CHECK (cible IN (
        'identite', 'identifiant', 'scolarisation', 'inscription', 'coordonnee', 'photo', 'nouvel_eleve')),
    CONSTRAINT ent_eleve_proposition_source_chk CHECK (source IN (
        'import', 'famille', 'eleve', 'ancien_etablissement', 'educonnect', 'admission',
        'systeme', 'personnel', 'verification_campagne')),
    CONSTRAINT ent_eleve_proposition_origine_chk CHECK (origine IS NULL OR origine IN (
        'siecle', 'onde', 'aaf', 'bee', 'reprise', 'educonnect', 'autre')),
    CONSTRAINT ent_eleve_proposition_statut_chk CHECK (statut IN (
        'en_attente', 'acceptee', 'acceptee_partiellement', 'refusee', 'expiree',
        'appliquee_auto', 'obsolete')),
    CONSTRAINT ent_eleve_proposition_eleve_chk CHECK (eleve_id IS NOT NULL OR cible = 'nouvel_eleve'),
    CONSTRAINT ent_eleve_proposition_commentaire_chk CHECK (commentaire IS NULL OR char_length(commentaire) <= 500),
    CONSTRAINT ent_eleve_proposition_motif_chk CHECK (decision_motif IS NULL OR char_length(decision_motif) <= 500),
    CONSTRAINT ent_eleve_proposition_decision_chk CHECK (
        (statut = 'en_attente') = (decide_le IS NULL AND decision_code IS NULL AND regle_auto IS NULL)
        OR statut = 'obsolete' OR statut = 'expiree'),
    FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    UNIQUE (etablissement_id, id)
);

CREATE INDEX IF NOT EXISTS ent_eleve_proposition_file_idx
    ON ent_eleve_proposition (etablissement_id, statut, cree_le);
CREATE INDEX IF NOT EXISTS ent_eleve_proposition_eleve_idx
    ON ent_eleve_proposition (etablissement_id, eleve_id, cree_le DESC);
CREATE INDEX IF NOT EXISTS ent_eleve_proposition_depose_idx
    ON ent_eleve_proposition (etablissement_id, depose_par, cree_le DESC);

CREATE TABLE IF NOT EXISTS ent_eleve_proposition_champ (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL,
    proposition_id    UUID NOT NULL,
    champ             TEXT NOT NULL,
    actuel            TEXT,
    propose           TEXT,
    decision          TEXT NOT NULL DEFAULT 'en_attente',
    CONSTRAINT ent_eleve_proposition_champ_nom_chk CHECK (champ ~ '^[a-z][a-z0-9_]{0,59}$'),
    CONSTRAINT ent_eleve_proposition_champ_decision_chk CHECK (
        decision IN ('en_attente', 'acceptee', 'refusee', 'obsolete')),
    FOREIGN KEY (etablissement_id, proposition_id) REFERENCES ent_eleve_proposition (etablissement_id, id),
    UNIQUE (proposition_id, champ)
);

-- Une proposition décidée est immuable ; les valeurs ne changent jamais ; aucune suppression.
CREATE OR REPLACE FUNCTION ent_eleve_proposition_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'proposition : ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut <> 'en_attente' THEN
        RAISE EXCEPTION 'proposition : déjà décidée, immuable' USING ERRCODE = '42501';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.cible, NEW.source, NEW.origine,
        NEW.depose_par, NEW.piece_document_id, NEW.cree_le)
       IS DISTINCT FROM
       (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.cible, OLD.source, OLD.origine,
        OLD.depose_par, OLD.piece_document_id, OLD.cree_le) THEN
        RAISE EXCEPTION 'proposition : seule la décision peut changer' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_proposition_garde ON ent_eleve_proposition;
CREATE TRIGGER ent_eleve_proposition_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_proposition
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_proposition_garde();

CREATE OR REPLACE FUNCTION ent_eleve_proposition_champ_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE statut_parent TEXT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'proposition : une ligne ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    SELECT statut INTO statut_parent FROM ent_eleve_proposition
     WHERE etablissement_id = OLD.etablissement_id AND id = OLD.proposition_id;
    IF statut_parent IS DISTINCT FROM 'en_attente' OR OLD.decision <> 'en_attente' THEN
        RAISE EXCEPTION 'proposition : déjà décidée, immuable' USING ERRCODE = '42501';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.proposition_id, NEW.champ, NEW.actuel, NEW.propose)
       IS DISTINCT FROM
       (OLD.id, OLD.etablissement_id, OLD.proposition_id, OLD.champ, OLD.actuel, OLD.propose) THEN
        RAISE EXCEPTION 'proposition : seule la décision peut changer' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_proposition_champ_garde ON ent_eleve_proposition_champ;
CREATE TRIGGER ent_eleve_proposition_champ_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_proposition_champ
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_proposition_champ_garde();

-- ───────────────────────── Isolation par établissement (RLS, mode permissif) ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_proposition', 'ent_eleve_proposition_champ'] LOOP
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
