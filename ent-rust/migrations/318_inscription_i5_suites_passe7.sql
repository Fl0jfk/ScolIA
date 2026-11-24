-- Lot I5, suites de la passe 6 (socle 3, `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`, « Suites, passe 7 ») — **318**.
--
-- Additif et rejouable. 313 à 317 sont versées et **ne changent pas** (règle 25) ; 318 est ajoutée **à la fin** du tableau
-- de `migrate.rs` (règle 38). Toute fonction ci-dessous porte `SET search_path = pg_catalog, public, pg_temp`, ses relations
-- sont qualifiées `public.`.
--
-- **Moyen D (contre-revue de la passe 6).** La sentinelle par instruction de 317 appelait encore, quand la colonne `offre_id`
-- d'une ligne est NULL, `ent_inscription_offre_du_dossier` **une fois par ligne** (dans `SELECT DISTINCT COALESCE(…, f(ligne))`).
-- Une écriture de masse de lignes sans `offre_id` payait donc N appels de fonction SQL. Elle devient une **jointure
-- ensembliste** : l'offre de la liste d'un dossier est `offre_attente_id`, à défaut l'offre de son vœu retenu (même règle que la
-- fonction, et que `ent_inscription_liste_attente` depuis 317). Un seul balayage de la table de transition, une vérification
-- (lecture de `pg_locks`) par offre distincte. Mêmes règles qu'en 317 : ancienne **et** nouvelle offre sur une mise à jour,
-- exemption de suppression d'un dossier purgé, même erreur nommée, SQLSTATE 55000. Les déclencheurs de 317 appellent cette
-- fonction par son nom : `CREATE OR REPLACE` suffit, aucun déclencheur n'est recréé (leur état `ENABLE ALWAYS` est conservé).
--
-- **Limite connue, assumée (échec ouvert hérité de 316).** Si le dossier d'une ligne n'est pas visible de l'écrivain (isolation
-- par établissement, dossier absent), la jointure ne trouve pas d'offre : la ligne n'exige alors aucun verrou. C'est cohérent
-- avec le rôle de la sentinelle (un **filet** contre l'oubli du verrou dans le code, pas un contrôle de sécurité) ; la
-- fermer casserait la suppression en cascade d'un dossier déjà disparu au moment du déclencheur. Même comportement qu'en 316.
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
    -- Offres des lignes d'après (insertion, mise à jour) : colonne `offre_id`, à défaut l'offre de la liste du dossier (jointure).
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        FOR v_offre IN
            SELECT DISTINCT COALESCE(n.offre_id, d.offre_attente_id, v.offre_id) AS o
              FROM nouv n
              LEFT JOIN public.ent_inscription_dossier d
                     ON d.etablissement_id = n.etablissement_id AND d.id = n.dossier_id
              LEFT JOIN public.ent_inscription_voeu v
                     ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
             ORDER BY 1
        LOOP
            PERFORM public.ent_inscription_exiger_verrou_offre(v_offre, TG_TABLE_NAME::text);
        END LOOP;
    END IF;
    -- Offres des lignes d'avant (mise à jour, suppression) : une ligne qui change d'offre ou de dossier exige aussi l'ancien
    -- verrou (316, M1).
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        FOR v_offre IN
            SELECT DISTINCT COALESCE(a.offre_id, d.offre_attente_id, v.offre_id) AS o
              FROM anc a
              LEFT JOIN public.ent_inscription_dossier d
                     ON d.etablissement_id = a.etablissement_id AND d.id = a.dossier_id
              LEFT JOIN public.ent_inscription_voeu v
                     ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
             ORDER BY 1
        LOOP
            PERFORM public.ent_inscription_exiger_verrou_offre(v_offre, TG_TABLE_NAME::text);
        END LOOP;
    END IF;
    RETURN NULL;
END $$;
