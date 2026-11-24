-- Lot I5, suites de la passe 5 (socle 3, `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`, « Suites, passe 6 ») — **317**.
--
-- Additif et rejouable. 313 à 316 sont versées et **ne changent pas** (règle 25) ; 317 est ajoutée **à la fin** du tableau
-- de `migrate.rs` (règle 38). Toute fonction ci-dessous porte `SET search_path = pg_catalog, public, pg_temp`, ses relations
-- sont qualifiées `public.`. Aucune **preuve de verrou** ne repose sur `PG_CONTEXT` ni sur `set_config` : la preuve est lue
-- dans `pg_locks`. (Précision sur 316, F2 : son en-tête dit « aucune garde ne lit `set_config` » ; c'est exact pour la preuve
-- du verrou, mais l'**exemption de suppression** de la sentinelle lit `ent_inscription_purge_voulue()`, donc le réglage de
-- session `ent.purge_etablissement`, que tout appelant peut poser. L'exemption ne couvre que la **suppression** de lignes de
-- places/décisions, jamais leur création ni leur modification : un appelant qui pose ce réglage peut supprimer, dans la
-- limite de ce que `ent_inscription_ajout_seul` lui laisse déjà faire (la même fonction, le même réglage). La sentinelle est
-- un **filet** contre les oublis de verrou dans le code, pas un contrôle de sécurité contre un appelant hostile.)
--
-- 1. **Sentinelle par instruction (M4).** Les déclencheurs de 316 s'exécutaient une fois **par ligne**, et chaque exécution
--    lit `pg_locks` (coût qui croît avec le nombre de verrous de l'instance). Une écriture de masse (publication de séance,
--    reprise) payait donc N lectures. Les déclencheurs passent `FOR EACH STATEMENT` avec tables de transition : une seule
--    vérification par **offre distincte** de l'instruction. Mêmes règles qu'en 316 (offre de la ligne ou, à défaut, celle de
--    la liste de son dossier ; sur une mise à jour, l'ancienne **et** la nouvelle offre ; exemption de suppression d'un
--    dossier purgé ; mêmes erreur nommée et SQLSTATE 55000).
-- 2. **`ENABLE ALWAYS TRIGGER` (F1).** La sentinelle ne se coupe plus par `SET session_replication_role = replica` : seuls
--    `ALTER TABLE … DISABLE TRIGGER` (propriétaire de la table) ou `DISABLE TRIGGER ALL` (superutilisateur, ce que fait
--    `pg_restore --disable-triggers`) la suspendent, explicitement. Procédure de restauration : voir la documentation.
-- 3. **Liste d'attente en une passe (F3).** `ent_inscription_liste_attente` appelait `ent_inscription_offre_du_dossier`
--    pour **chaque dossier de l'établissement**, puis, côté famille, `ent_inscription_en_liste_visible` (une requête
--    récursive) pour chaque dossier de l'offre. L'offre se calcule maintenant par jointure, et la fonction par dossier ne
--    s'exécute que pour les dossiers **de l'offre** (liste de l'offre, pas établissement entier).

-- ───────────────────────────── 1. Sentinelle par instruction ─────────────────────────────
-- Places et propositions. Chaque déclencheur (insertion, mise à jour, suppression) a ses tables de transition ; la fonction
-- ne lit que celles qui existent pour l'opération (PL/pgSQL ne planifie une requête qu'au moment de l'exécuter).
CREATE OR REPLACE FUNCTION ent_inscription_sentinelle_instruction() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_offre uuid;
BEGIN
    -- Suppression en cascade d'un dossier purgé (droit à l'effacement) : aucune file à protéger, le dossier disparaît.
    IF TG_OP = 'DELETE' AND public.ent_inscription_purge_voulue() THEN
        RETURN NULL;
    END IF;
    -- Offres des lignes d'après (insertion, mise à jour) : colonne `offre_id`, à défaut l'offre de la liste du dossier.
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        FOR v_offre IN
            SELECT DISTINCT COALESCE(n.offre_id, public.ent_inscription_offre_du_dossier(n.etablissement_id, n.dossier_id)) AS o
              FROM nouv n
             ORDER BY 1
        LOOP
            PERFORM public.ent_inscription_exiger_verrou_offre(v_offre, TG_TABLE_NAME::text);
        END LOOP;
    END IF;
    -- Offres des lignes d'avant (mise à jour, suppression) : une ligne qui change d'offre ou de dossier exige aussi l'ancien
    -- verrou (316, M1).
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        FOR v_offre IN
            SELECT DISTINCT COALESCE(a.offre_id, public.ent_inscription_offre_du_dossier(a.etablissement_id, a.dossier_id)) AS o
              FROM anc a
             ORDER BY 1
        LOOP
            PERFORM public.ent_inscription_exiger_verrou_offre(v_offre, TG_TABLE_NAME::text);
        END LOOP;
    END IF;
    RETURN NULL;
END $$;

-- Places de l'offre elle-même : capacité et compteur de places rendues, seulement pour les offres dont l'une des deux change.
CREATE OR REPLACE FUNCTION ent_inscription_sentinelle_offre_instruction() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_offre uuid;
BEGIN
    FOR v_offre IN
        SELECT n.id
          FROM nouv n
          JOIN anc a ON a.id = n.id
         WHERE n.capacite IS DISTINCT FROM a.capacite OR n.liberations IS DISTINCT FROM a.liberations
         ORDER BY n.id
    LOOP
        PERFORM public.ent_inscription_exiger_verrou_offre(v_offre, TG_TABLE_NAME::text);
    END LOOP;
    RETURN NULL;
END $$;

-- Les déclencheurs par ligne de 316 sont remplacés (leurs fonctions restent définies : un rejeu de 316 les recrée).
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_proposition_trg ON ent_inscription_proposition_place;
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_decision_trg ON ent_inscription_decision;
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_offre_trg ON ent_inscription_offre;

DROP TRIGGER IF EXISTS ent_inscription_sentinelle_proposition_ins_trg ON ent_inscription_proposition_place;
CREATE TRIGGER ent_inscription_sentinelle_proposition_ins_trg
    AFTER INSERT ON ent_inscription_proposition_place
    REFERENCING NEW TABLE AS nouv
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_instruction();
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_proposition_upd_trg ON ent_inscription_proposition_place;
CREATE TRIGGER ent_inscription_sentinelle_proposition_upd_trg
    AFTER UPDATE ON ent_inscription_proposition_place
    REFERENCING OLD TABLE AS anc NEW TABLE AS nouv
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_instruction();
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_proposition_del_trg ON ent_inscription_proposition_place;
CREATE TRIGGER ent_inscription_sentinelle_proposition_del_trg
    AFTER DELETE ON ent_inscription_proposition_place
    REFERENCING OLD TABLE AS anc
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_instruction();

DROP TRIGGER IF EXISTS ent_inscription_sentinelle_decision_ins_trg ON ent_inscription_decision;
CREATE TRIGGER ent_inscription_sentinelle_decision_ins_trg
    AFTER INSERT ON ent_inscription_decision
    REFERENCING NEW TABLE AS nouv
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_instruction();
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_decision_upd_trg ON ent_inscription_decision;
CREATE TRIGGER ent_inscription_sentinelle_decision_upd_trg
    AFTER UPDATE ON ent_inscription_decision
    REFERENCING OLD TABLE AS anc NEW TABLE AS nouv
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_instruction();
DROP TRIGGER IF EXISTS ent_inscription_sentinelle_decision_del_trg ON ent_inscription_decision;
CREATE TRIGGER ent_inscription_sentinelle_decision_del_trg
    AFTER DELETE ON ent_inscription_decision
    REFERENCING OLD TABLE AS anc
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_instruction();

DROP TRIGGER IF EXISTS ent_inscription_sentinelle_offre_upd_trg ON ent_inscription_offre;
CREATE TRIGGER ent_inscription_sentinelle_offre_upd_trg
    AFTER UPDATE ON ent_inscription_offre
    REFERENCING OLD TABLE AS anc NEW TABLE AS nouv
    FOR EACH STATEMENT EXECUTE FUNCTION ent_inscription_sentinelle_offre_instruction();

-- ───────────────────────────── 2. ENABLE ALWAYS (F1) ─────────────────────────────
ALTER TABLE ent_inscription_proposition_place ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_proposition_ins_trg;
ALTER TABLE ent_inscription_proposition_place ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_proposition_upd_trg;
ALTER TABLE ent_inscription_proposition_place ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_proposition_del_trg;
ALTER TABLE ent_inscription_decision ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_decision_ins_trg;
ALTER TABLE ent_inscription_decision ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_decision_upd_trg;
ALTER TABLE ent_inscription_decision ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_decision_del_trg;
ALTER TABLE ent_inscription_offre ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_offre_upd_trg;

-- ───────────────────────────── 3. Liste d'attente : l'offre par jointure ─────────────────────────────
-- Même résultat et même formule d'ordre qu'en 316 (seul endroit où elle est écrite). L'offre de la liste d'un dossier est
-- `offre_attente_id`, à défaut l'offre de son vœu retenu (jointure, plus d'appel de fonction par dossier). La fonction
-- par dossier de la famille (`ent_inscription_en_liste_visible`) ne s'exécute que sur les candidats **de l'offre** : la
-- CTE matérialisée interdit au planificateur de la remonter avant le filtre d'offre.
CREATE OR REPLACE FUNCTION ent_inscription_liste_attente(p_etab uuid, p_offre uuid, p_famille boolean)
RETURNS TABLE (dossier_id uuid, pos bigint)
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    WITH candidats AS MATERIALIZED (
        SELECT x.id, x.etablissement_id, x.etat, x.ordre_attente, x.rang_attente
          FROM public.ent_inscription_dossier x
          LEFT JOIN public.ent_inscription_voeu v
                 ON v.etablissement_id = x.etablissement_id AND v.id = x.voeu_retenu_id
         WHERE x.etablissement_id = p_etab
           AND COALESCE(x.offre_attente_id, v.offre_id) = p_offre
    )
    SELECT c.id,
           row_number() OVER (ORDER BY COALESCE(c.ordre_attente, c.rang_attente::bigint) NULLS LAST, c.id)
      FROM candidats c
     WHERE CASE WHEN p_famille
                THEN public.ent_inscription_en_liste_visible(c.etablissement_id, c.id, c.etat)
                ELSE c.etat = 'liste_attente' END
$$;
