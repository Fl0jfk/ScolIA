-- Lot I5, passe 3 (socle 3, `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`, section « Passe 3 ») — **314**.
--
-- Additif et rejouable. 313 est versée et **ne change pas** (règle 25) : tout ce qui la corrige est ici.
--
-- * Décision **caduque** : `caduque_le`, `caduque_motif` (écrits une fois, par le déclencheur d'ajout seul) — une décision
--   programmée dont l'état du dossier ne correspond plus à la publication n'est ni publiée ni notifiée (correctif 3).
-- * Séquence `ent_inscription_attente_ordre_seq` avancée au-delà des `ordre_attente` repris de `rang_attente` (correctif 4).
-- * Réparation **déterministe** de la chaîne `remplace_decision_id` d'un dossier à plusieurs racines ou à double
--   successeur, avant la pose des index uniques (correctif 6).
-- * Les fonctions de garde de 313 sont recréées avec `SET search_path = pg_catalog, public, pg_temp` : une table
--   temporaire homonyme ne les trompe pas.

ALTER TABLE ent_inscription_decision ADD COLUMN IF NOT EXISTS caduque_le TIMESTAMPTZ;
ALTER TABLE ent_inscription_decision ADD COLUMN IF NOT EXISTS caduque_motif TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_decision_caduque_chk') THEN
        ALTER TABLE ent_inscription_decision
            ADD CONSTRAINT ent_inscription_decision_caduque_chk
            CHECK ((caduque_le IS NULL) = (caduque_motif IS NULL)
                   AND (caduque_le IS NULL OR publiee_le IS NULL));
    END IF;
END $$;
CREATE INDEX IF NOT EXISTS ent_inscription_decision_caduque_idx
    ON ent_inscription_decision (etablissement_id, dossier_id) WHERE caduque_le IS NOT NULL;

-- ───────────────────────────── Fonctions de garde (recréées, search_path fixé) ─────────────────────────────
CREATE OR REPLACE FUNCTION ent_inscription_purge_voulue() RETURNS boolean
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$ SELECT COALESCE(current_setting('ent.purge_etablissement', true), '') = 'on' $$;

CREATE OR REPLACE FUNCTION ent_inscription_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'TRUNCATE' THEN
        IF ent_inscription_purge_voulue() THEN
            RETURN NULL;
        END IF;
        RAISE EXCEPTION 'historique en ajout seul : troncature refusée (%)', TG_TABLE_NAME
            USING ERRCODE = 'check_violation';
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF NOT ent_inscription_purge_voulue() THEN
            RAISE EXCEPTION 'historique en ajout seul : suppression refusée (%)', TG_TABLE_NAME
                USING ERRCODE = 'check_violation';
        END IF;
        RETURN OLD;
    END IF;
    IF TG_TABLE_NAME = 'ent_inscription_decision' THEN
        IF (to_jsonb(NEW) - 'publiee_le' - 'publiee_par' - 'publication_prevue_le'
                                  - 'caduque_le' - 'caduque_motif')
           IS DISTINCT FROM (to_jsonb(OLD) - 'publiee_le' - 'publiee_par' - 'publication_prevue_le'
                                  - 'caduque_le' - 'caduque_motif') THEN
            RAISE EXCEPTION 'une décision ne se modifie pas : une nouvelle décision la remplace'
                USING ERRCODE = 'check_violation';
        END IF;
        -- Caduque (passe 3) : une décision non publiée, jamais remplacée, se déclare caduque **une fois** ; une décision
        -- caduque ou publiée ne change plus (ni de motif, ni de date) et ne se publie jamais.
        IF OLD.caduque_le IS NOT NULL AND (NEW.caduque_le IS DISTINCT FROM OLD.caduque_le
                                           OR NEW.caduque_motif IS DISTINCT FROM OLD.caduque_motif
                                           OR NEW.publiee_le IS DISTINCT FROM OLD.publiee_le
                                           OR NEW.publiee_par IS DISTINCT FROM OLD.publiee_par
                                           OR NEW.publication_prevue_le IS DISTINCT FROM OLD.publication_prevue_le) THEN
            RAISE EXCEPTION 'une décision caduque ne se modifie plus'
                USING ERRCODE = 'check_violation';
        END IF;
        IF OLD.caduque_le IS NULL AND NEW.caduque_le IS NOT NULL
           AND (OLD.publiee_le IS NOT NULL OR NEW.publiee_le IS NOT NULL) THEN
            RAISE EXCEPTION 'une décision publiée ne devient pas caduque'
                USING ERRCODE = 'check_violation';
        END IF;
        IF OLD.publiee_le IS NOT NULL AND (NEW.publiee_le IS DISTINCT FROM OLD.publiee_le
                                           OR NEW.publiee_par IS DISTINCT FROM OLD.publiee_par
                                           OR NEW.publication_prevue_le IS DISTINCT FROM OLD.publication_prevue_le) THEN
            RAISE EXCEPTION 'une décision publiée ne se modifie plus'
                USING ERRCODE = 'check_violation';
        END IF;
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'historique en ajout seul : modification refusée (%)', TG_TABLE_NAME
        USING ERRCODE = 'check_violation';
END $$;

CREATE OR REPLACE FUNCTION ent_inscription_decision_chaine() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.remplace_decision_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ent_inscription_decision p
         WHERE p.id = NEW.remplace_decision_id AND p.dossier_id = NEW.dossier_id
           AND p.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'une décision ne remplace qu''une décision du même dossier'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION ent_inscription_membre_lie(p_etab UUID, p_dossier UUID, p_user UUID) RETURNS boolean
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_user u
         WHERE u.id = p_user
           AND (
                EXISTS (SELECT 1 FROM ent_inscription_dossier d
                         WHERE d.etablissement_id = p_etab AND d.id = p_dossier
                           AND (d.deposant_user_id = u.id
                                OR (u.email IS NOT NULL AND lower(d.deposant_email) = lower(u.email))
                                OR EXISTS (SELECT 1 FROM ent_eleve_lien l
                                            WHERE l.eleve_id = d.eleve_id AND l.user_id = u.id)))
             OR EXISTS (SELECT 1 FROM ent_inscription_responsable r
                         WHERE r.etablissement_id = p_etab AND r.dossier_id = p_dossier
                           AND ((u.email IS NOT NULL AND lower(r.email) = lower(u.email))
                                OR EXISTS (SELECT 1 FROM ent_personne pr
                                             JOIN ent_personne pu
                                               ON pu.etablissement_id = pr.etablissement_id AND pu.user_id = u.id
                                            WHERE pr.id = r.personne_id
                                              AND (pu.id = pr.id OR pu.fusionnee_dans = pr.id
                                                   OR pr.fusionnee_dans = pu.id
                                                   OR (pu.fusionnee_dans IS NOT NULL
                                                       AND pu.fusionnee_dans = pr.fusionnee_dans)))))
             OR EXISTS (SELECT 1 FROM ent_inscription_invitation i
                         WHERE i.etablissement_id = p_etab AND i.dossier_id = p_dossier
                           AND (i.user_id = u.id OR (u.email IS NOT NULL AND lower(i.email) = lower(u.email))))
           )
    )
$$;

CREATE OR REPLACE FUNCTION ent_inscription_ecarte_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_membre UUID;
BEGIN
    IF TG_TABLE_NAME = 'ent_inscription_avis' THEN
        v_membre := NEW.auteur_id;
    ELSE
        v_membre := NEW.decide_par;
    END IF;
    IF v_membre IS NULL THEN
        RETURN NEW;
    END IF;
    IF EXISTS (SELECT 1 FROM ent_inscription_deport x
                WHERE x.etablissement_id = NEW.etablissement_id AND x.dossier_id = NEW.dossier_id
                  AND x.membre_id = v_membre)
       OR ent_inscription_membre_lie(NEW.etablissement_id, NEW.dossier_id, v_membre) THEN
        RAISE EXCEPTION 'membre écarté de ce dossier (déport ou lien avec le candidat)'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION ent_inscription_en_liste_visible(p_etab UUID, p_dossier UUID, p_etat TEXT)
RETURNS boolean LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    WITH RECURSIVE chaine AS (
        SELECT d.id, d.sens, d.publiee_le, d.caduque_le, 1 AS rang_chaine FROM ent_inscription_decision d
         WHERE d.etablissement_id = p_etab AND d.dossier_id = p_dossier AND d.remplace_decision_id IS NULL
        UNION ALL
        SELECT x.id, x.sens, x.publiee_le, x.caduque_le, c.rang_chaine + 1 FROM ent_inscription_decision x
          JOIN chaine c ON x.remplace_decision_id = c.id
         WHERE x.etablissement_id = p_etab AND x.dossier_id = p_dossier)
    SELECT CASE
        WHEN (SELECT (publiee_le IS NOT NULL OR caduque_le IS NOT NULL) FROM chaine ORDER BY rang_chaine DESC LIMIT 1) IS DISTINCT FROM false
            THEN p_etat = 'liste_attente'
        ELSE COALESCE((SELECT sens = 'liste_attente' FROM chaine WHERE publiee_le IS NOT NULL
                        ORDER BY rang_chaine DESC LIMIT 1), false)
    END
$$;

-- ───────────────────────────── Réparation de la chaîne des décisions (correctif 6) ─────────────────────────────
-- Un dossier dont les décisions n'ont **pas une seule racine** ou dont une décision a **deux successeurs** (historique
-- partiel : d1 racine, d2 remplace d1, d3 racine) est rechaîné, dans l'ordre (`decide_le`, puis `id`) : la première est
-- la racine, chacune remplace la précédente. Un dossier déjà bien chaîné n'est pas touché. L'ajout seul est suspendu le
-- temps de la réparation, puis rétabli ; les index uniques sont posés **après** (jamais d'échec sur un historique partiel).
DO $$
DECLARE
    v_garde boolean;
    v_casses boolean;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM ent_inscription_decision
         GROUP BY dossier_id
        HAVING count(*) FILTER (WHERE remplace_decision_id IS NULL) > 1
        UNION ALL
        SELECT 1 FROM ent_inscription_decision WHERE remplace_decision_id IS NOT NULL
         GROUP BY dossier_id, remplace_decision_id HAVING count(*) > 1) INTO v_casses;
    IF v_casses THEN
        SELECT EXISTS (SELECT 1 FROM pg_trigger
                        WHERE tgname = 'ent_inscription_decision_ajout_seul_trg'
                          AND tgrelid = 'ent_inscription_decision'::regclass) INTO v_garde;
        IF v_garde THEN
            ALTER TABLE ent_inscription_decision DISABLE TRIGGER ent_inscription_decision_ajout_seul_trg;
        END IF;
        DROP INDEX IF EXISTS ent_inscription_decision_racine_uq;
        DROP INDEX IF EXISTS ent_inscription_decision_successeur_uq;
        WITH casses AS (
            SELECT dossier_id FROM ent_inscription_decision
             GROUP BY dossier_id HAVING count(*) FILTER (WHERE remplace_decision_id IS NULL) > 1
            UNION
            SELECT dossier_id FROM ent_inscription_decision WHERE remplace_decision_id IS NOT NULL
             GROUP BY dossier_id, remplace_decision_id HAVING count(*) > 1),
        ordre AS (
            SELECT id, lag(id) OVER (PARTITION BY dossier_id ORDER BY decide_le, id) AS precedent
              FROM ent_inscription_decision WHERE dossier_id IN (SELECT dossier_id FROM casses))
        UPDATE ent_inscription_decision d SET remplace_decision_id = o.precedent
          FROM ordre o
         WHERE d.id = o.id AND d.remplace_decision_id IS DISTINCT FROM o.precedent;
        IF v_garde THEN
            ALTER TABLE ent_inscription_decision ENABLE TRIGGER ent_inscription_decision_ajout_seul_trg;
        END IF;
    END IF;
END $$;
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_decision_racine_uq
    ON ent_inscription_decision (dossier_id) WHERE remplace_decision_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_decision_successeur_uq
    ON ent_inscription_decision (remplace_decision_id) WHERE remplace_decision_id IS NOT NULL;

-- ───────────────────────────── Séquence de l'ordre d'attente (correctif 4) ─────────────────────────────
-- 313 recopie `ordre_attente = rang_attente` (1..n) alors que la séquence démarre à 1 : un nouvel entrant recevrait un
-- ordre déjà pris et passerait devant un dossier repris. La séquence ne recule jamais.
SELECT setval('ent_inscription_attente_ordre_seq',
              GREATEST((SELECT max(ordre_attente) FROM ent_inscription_dossier),
                       (SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM ent_inscription_attente_ordre_seq),
                       1));
