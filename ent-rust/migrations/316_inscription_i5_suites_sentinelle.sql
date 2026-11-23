-- Lot I5, suites de la passe 4 (socle 3, `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`, « Suites (passe 5) ») — **316**.
--
-- Additif et rejouable. 313 à 315 sont versées et **ne changent pas** (règles 25 et 38) ; 316 est ajoutée **à la fin**
-- du tableau de `migrate.rs`. Toute fonction ci-dessous porte `SET search_path = pg_catalog, public, pg_temp`, ses
-- relations sont qualifiées `public.`. Aucune garde n'utilise `PG_CONTEXT` ni `set_config` : la preuve est lue dans
-- `pg_locks`, que le rôle applicatif ne peut pas falsifier (il ne peut qu'**acquérir** réellement le verrou).
--
-- 1. **Sentinelle de l'ordre des verrous (M-2).** La garde statique de la passe 4 se contourne (un appel construit, une
--    requête dans un autre fichier). Un déclencheur sur les places, les propositions et les décisions exige maintenant
--    que la transaction tienne le verrou consultatif de l'offre concernée (`ent_inscription_offre_cle`), sinon il lève
--    l'erreur nommée `ent_inscription_verrou_offre_manquant` (SQLSTATE 55000).
-- 2. **Source unique du rang (F-c).** `ent_inscription_offre_du_dossier` (l'offre de la liste d'un dossier) et
--    `ent_inscription_liste_attente` (la liste ordonnée d'une offre, avec la position) : le rang, la liste d'attente
--    affichée et la désignation du suivant n'ont plus qu'une formule.

-- ───────────────────────────── Offre de la liste d'un dossier ─────────────────────────────
-- L'offre dont le dossier occupe (ou a occupé) la liste : `offre_attente_id`, à défaut l'offre de son vœu retenu.
CREATE OR REPLACE FUNCTION ent_inscription_offre_du_dossier(p_etab uuid, p_dossier uuid) RETURNS uuid
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT COALESCE(m.offre_attente_id, v.offre_id)
      FROM public.ent_inscription_dossier m
      LEFT JOIN public.ent_inscription_voeu v
             ON v.etablissement_id = m.etablissement_id AND v.id = m.voeu_retenu_id
     WHERE m.etablissement_id = p_etab AND m.id = p_dossier
$$;

-- ───────────────────────────── La liste ordonnée d'une offre (seule formule) ─────────────────────────────
-- `p_famille = false` : le personnel, parmi les dossiers réellement en liste. `p_famille = true` : la famille, parmi les
-- dossiers dont l'entrée ou la sortie est **publiée**. Ordre : entrée sur la liste (`ordre_attente`, repli sur l'ancien
-- `rang_attente` repris), puis identifiant. **Seul endroit** où cette formule est écrite.
CREATE OR REPLACE FUNCTION ent_inscription_liste_attente(p_etab uuid, p_offre uuid, p_famille boolean)
RETURNS TABLE (dossier_id uuid, pos bigint)
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT x.id,
           row_number() OVER (ORDER BY COALESCE(x.ordre_attente, x.rang_attente::bigint) NULLS LAST, x.id)
      FROM public.ent_inscription_dossier x
     WHERE x.etablissement_id = p_etab
       AND public.ent_inscription_offre_du_dossier(x.etablissement_id, x.id) = p_offre
       AND CASE WHEN p_famille
                THEN public.ent_inscription_en_liste_visible(x.etablissement_id, x.id, x.etat)
                ELSE x.etat = 'liste_attente' END
$$;

-- Le rang d'un dossier : sa position dans la liste de son offre (même fonction, un seul filtre).
CREATE OR REPLACE FUNCTION ent_inscription_rang_attente(p_etab uuid, p_dossier uuid, p_famille boolean) RETURNS bigint
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT l.pos
      FROM public.ent_inscription_liste_attente(
               p_etab, public.ent_inscription_offre_du_dossier(p_etab, p_dossier), p_famille) l
     WHERE l.dossier_id = p_dossier
$$;

-- ───────────────────────────── Sentinelle : le verrou de l'offre est-il tenu ? ─────────────────────────────
-- Vrai si cette session tient (accordé) le verrou consultatif transactionnel ou de session de l'offre. `p_offre` NULL :
-- rien à protéger. Volatile : la réponse change au fil de la transaction.
CREATE OR REPLACE FUNCTION ent_inscription_verrou_offre_tenu(p_offre uuid) RETURNS boolean
LANGUAGE sql VOLATILE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT p_offre IS NULL
        OR EXISTS (SELECT 1
                     FROM pg_catalog.pg_locks l
                    WHERE l.locktype = 'advisory'
                      AND l.pid = pg_catalog.pg_backend_pid()
                      AND l.granted
                      AND l.objsubid = 1
                      AND l.database = (SELECT d.oid FROM pg_catalog.pg_database d
                                         WHERE d.datname = pg_catalog.current_database())
                      AND ((l.classid::bigint << 32) | l.objid::bigint) = public.ent_inscription_offre_cle(p_offre))
$$;

CREATE OR REPLACE FUNCTION ent_inscription_exiger_verrou_offre(p_offre uuid, p_table text) RETURNS void
LANGUAGE plpgsql VOLATILE
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NOT public.ent_inscription_verrou_offre_tenu(p_offre) THEN
        RAISE EXCEPTION 'ent_inscription_verrou_offre_manquant : écriture de % sans le verrou de l''offre %', p_table, p_offre
            USING ERRCODE = '55000';
    END IF;
END $$;

-- Places et propositions : l'offre de la ligne est sa colonne `offre_id`, à défaut celle de la liste de son dossier.
-- Les déclencheurs sont **après** la ligne (`AFTER`) : une écriture déjà refusée par une contrainte ou par une garde de la
-- table (unicité, écart, ajout seul) garde son refus d'origine ; la sentinelle ne parle que des écritures qui passeraient.
CREATE OR REPLACE FUNCTION ent_inscription_sentinelle_ligne() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_etab uuid;
    v_dossier uuid;
    v_offre uuid;
    v_ancienne uuid;
BEGIN
    -- Suppression en cascade d'un dossier purgé (droit à l'effacement) : aucune file à protéger, le dossier disparaît.
    IF TG_OP = 'DELETE' AND public.ent_inscription_purge_voulue() THEN
        RETURN OLD;
    END IF;
    IF TG_OP = 'DELETE' THEN
        v_etab := OLD.etablissement_id; v_dossier := OLD.dossier_id; v_offre := OLD.offre_id;
    ELSE
        v_etab := NEW.etablissement_id; v_dossier := NEW.dossier_id; v_offre := NEW.offre_id;
    END IF;
    IF v_offre IS NULL THEN
        v_offre := public.ent_inscription_offre_du_dossier(v_etab, v_dossier);
    END IF;
    PERFORM public.ent_inscription_exiger_verrou_offre(v_offre, TG_TABLE_NAME::text);
    IF TG_OP = 'UPDATE' THEN
        -- L'offre d'avant : sa colonne, à défaut celle de la liste de son ancien dossier (contre-revue, M1 : une ligne qui
        -- passe d'une offre déduite du dossier à une offre propre, ou qui change de dossier, exige aussi l'ancien verrou).
        v_ancienne := COALESCE(OLD.offre_id, public.ent_inscription_offre_du_dossier(OLD.etablissement_id, OLD.dossier_id));
        IF v_ancienne IS NOT NULL AND v_ancienne IS DISTINCT FROM v_offre THEN
            PERFORM public.ent_inscription_exiger_verrou_offre(v_ancienne, TG_TABLE_NAME::text);
        END IF;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;

DROP TRIGGER IF EXISTS ent_inscription_sentinelle_proposition_trg ON ent_inscription_proposition_place;
CREATE TRIGGER ent_inscription_sentinelle_proposition_trg
    AFTER INSERT OR UPDATE OR DELETE ON ent_inscription_proposition_place
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_sentinelle_ligne();

DROP TRIGGER IF EXISTS ent_inscription_sentinelle_decision_trg ON ent_inscription_decision;
CREATE TRIGGER ent_inscription_sentinelle_decision_trg
    AFTER INSERT OR UPDATE OR DELETE ON ent_inscription_decision
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_sentinelle_ligne();

-- Places de l'offre elle-même : capacité et compteur de places rendues (la fonction ne réagit qu'à ces deux colonnes ; le
-- déclencheur n'est pas lié à leurs noms : une base rejouée depuis un état ancien reste modifiable).
CREATE OR REPLACE FUNCTION ent_inscription_sentinelle_offre() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.capacite IS DISTINCT FROM OLD.capacite OR NEW.liberations IS DISTINCT FROM OLD.liberations THEN
        PERFORM public.ent_inscription_exiger_verrou_offre(OLD.id, TG_TABLE_NAME::text);
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_inscription_sentinelle_offre_trg ON ent_inscription_offre;
CREATE TRIGGER ent_inscription_sentinelle_offre_trg
    AFTER UPDATE ON ent_inscription_offre
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_sentinelle_offre();
