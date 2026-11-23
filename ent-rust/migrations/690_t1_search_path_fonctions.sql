-- Chantier transverse T1 (plage 690–699, sécurité transverse) : « une table temporaire ne trompe aucune fonction ».
-- Réf. : docs/securite/T1_SEARCH_PATH.md.
--
-- Constat : PostgreSQL cherche les relations dans `pg_temp` EN PREMIER (même quand `pg_temp` n'est pas dans le
-- `search_path`), le privilège TEMP est accordé à PUBLIC par défaut, et la plupart de nos fonctions (gardes d'ajout
-- seul, de gel, de cohérence, calculs d'attendu) lisent leurs tables sans les qualifier. Une session du rôle applicatif
-- capable d'exécuter du SQL pouvait donc désarmer une garde dans sa propre session :
-- `CREATE TEMP TABLE ent_xxx (LIKE public.ent_xxx)`. Pas d'escalade de privilège pour les fonctions « invoker », mais la
-- promesse « garanti en base » ne tenait pas — et trois fonctions SECURITY DEFINER avaient `search_path = pg_catalog,
-- public` SANS `pg_temp` (donc `pg_temp` cherché en premier, avec les droits du propriétaire).
--
-- Ce fichier : (1) fixe `search_path = pg_catalog, public, pg_temp` sur toute fonction de `public` (hors fonctions
-- d'extension) qui ne l'a pas, ou qui l'a sans `pg_temp` en dernier ; (2) SAUF deux familles, où `SET` ferait perdre
-- l'inlining d'un `LANGUAGE sql` (mesures : docs/securite/T1_SEARCH_PATH.md §3) :
--   a. les fonctions « pures » dont le corps ne lit AUCUNE relation (politiques RLS, index, contraintes CHECK, requêtes
--      chaudes) : rien à détourner, `SET` n'apporterait que la perte de l'inlining ;
--   b. les fonctions `LANGUAGE sql` qui retournent un ensemble de lignes et que le planificateur inline dans les
--      requêtes qui les appellent : leur corps est RECRÉÉ avec toutes les relations qualifiées `public.` (le texte est
--      lu dans le catalogue à l'exécution — aucune copie de définition dans ce fichier, donc aucune régression
--      possible si un lot a redéfini la fonction avant lui). `pg_temp` n'est jamais cherché pour les fonctions ni les
--      opérateurs ; les relations seules étaient exposées.
-- Un test d'`ent-db` (`garde_search_path`) vérifie, sur le catalogue d'une base migrée : aucune fonction sans clause hors
-- de ces deux listes ; chaque fonction « pure » ne cite aucune relation de `public` ; chaque fonction « qualifiée » n'en
-- cite aucune sans `public.`, reste inlinée et résiste à des tables temporaires homonymes ; et que ces deux listes sont
-- identiques à `PURES` et `QUALIFIEES` de crates/ent-db/src/garde_search_path.rs (fichier unique côté Rust).
--
-- Rejouable ET destiné à être rejoué EN DERNIER : un `CREATE OR REPLACE FUNCTION` ultérieur (ou le rejeu d'une
-- migration antérieure) efface la clause `SET` ; le test de rejeu (`rejouer_tout`) rejoue ce fichier après tous les
-- autres, puis la garde du catalogue vérifie qu'aucune fonction n'est restée sans clause. Aucune migration versée
-- n'est modifiée (règle 25) ; aucune table, aucune donnée touchée.
DO $$
DECLARE
    f regprocedure;
    v_def TEXT;
    v_a INTEGER;
    v_z INTEGER;
    v_re TEXT;
    v_pures TEXT[] := ARRAY[
        -- PURES-DEBUT (identique à `PURES` de crates/ent-db/src/garde_search_path.rs, ordre alphabétique)
        'ent_aujourdhui',
        'ent_cle_recherche',
        'ent_coord_groupe',
        'ent_dem_fait_canonique',
        'ent_dem_purge_en_cours',
        'ent_dem_rapprochement_en_cours',
        'ent_dem_requalif_en_cours',
        'ent_eleve_majorite_le',
        'ent_eleve_sexe_normalise',
        'ent_eleves_lecture_seule',
        'ent_enseignement_suivi_le',
        'ent_frm_purge',
        'ent_fusion_en_cours',
        'ent_import_ligne_donnees',
        'ent_inscription_etat_effectif',
        'ent_inscription_etat_valide',
        'ent_inscription_statut_depuis_etat',
        'ent_internat_ancre_nuit',
        'ent_internat_lien_parental',
        'ent_internat_personne_vise',
        'ent_jour_de_annee_label',
        'ent_niveau_de_code',
        'ent_parametre_valeur_json',
        'ent_rls_etablissement',
        'ent_rls_strict',
        'ent_siret_cle_valide',
        'ent_unaccent',
        'eva_dec3',
        'eva_etat_rang'
        -- PURES-FIN
    ];
    v_qualifiees TEXT[] := ARRAY[
        -- QUALIFIEES-DEBUT (identique à `QUALIFIEES` de crates/ent-db/src/garde_search_path.rs, ordre alphabétique)
        'ent_cdt_reprise_compteurs',
        'ent_coord_projetees',
        'ent_cours_vacants',
        'ent_eleve_enseignement_comparaison',
        'ent_eleve_indicateurs_attendus',
        'ent_eleve_majorites_a_traiter',
        'ent_eleve_option_comparaison',
        'ent_eleve_pai_courant',
        'ent_enseignement_incoherences',
        'ent_groupe_membres_jour',
        'ent_groupes_eleve_jour',
        'ent_internat_attendu_calcule',
        'ent_internat_personne_chaine',
        'ent_internat_remise',
        'ent_msg_boite_membres',
        'ent_msg_fil_lisible',
        'ent_msg_mes_participations',
        'ent_public_cours_jour',
        'ent_service_intervenants_a',
        'ent_services_vacants',
        'ent_stage_siret_doublons',
        'eva_fiches_de_l_eleve',
        'eva_grille_eleves',
        'eva_periode_eleve',
        'eva_periode_eleve_jour'
        -- QUALIFIEES-FIN
    ];
BEGIN
    -- (b) Corps recréés : toute relation de `public` citée sans qualification reçoit `public.`. Les noms déjà
    -- qualifiés (précédés d'un point), entre guillemets ou en littéral (précédés d'une apostrophe) ne bougent pas ;
    -- un nom suivi d'un caractère d'identifiant n'est pas un nom de relation (`ent_msg_boite_membres` n'est pas
    -- `ent_msg_boite`). Rejouable : un corps déjà qualifié ressort identique.
    SELECT string_agg(c.relname, '|' ORDER BY length(c.relname) DESC) INTO v_re
      FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
     WHERE n.nspname = 'public' AND c.relkind IN ('r', 'v', 'm', 'p', 'f', 'S');
    FOR f IN
        SELECT p.oid::regprocedure
          FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
         WHERE n.nspname = 'public' AND p.prokind = 'f' AND p.proname = ANY (v_qualifiees)
           AND NOT EXISTS (SELECT 1 FROM pg_depend d
                            WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e')
         ORDER BY p.oid
    LOOP
        v_def := pg_get_functiondef(f);
        v_a := strpos(v_def, '$function$') + 9;                                   -- dernier caractère du délimiteur ouvrant
        v_z := length(v_def) - strpos(reverse(v_def), reverse('$function$')) - 8; -- premier caractère du délimiteur fermant
        IF v_a < 10 OR v_z <= v_a THEN
            RAISE EXCEPTION 'T1 : corps de % introuvable dans sa définition', f;
        END IF;
        EXECUTE left(v_def, v_a)
             || regexp_replace(substr(v_def, v_a + 1, v_z - v_a - 1),
                               '(^|[^A-Za-z0-9_."''])(' || v_re || ')(?![A-Za-z0-9_])', '\1public.\2', 'gi')
             || substr(v_def, v_z);
    END LOOP;

    -- (1) `search_path` fixé sur tout le reste.
    FOR f IN
        SELECT p.oid::regprocedure
          FROM pg_proc p
          JOIN pg_namespace n ON n.oid = p.pronamespace
         WHERE n.nspname = 'public'
           AND p.prokind IN ('f', 'p')
           AND NOT (p.proname = ANY (v_pures))
           AND NOT (p.proname = ANY (v_qualifiees))
           -- fonctions d'extension : jamais touchées
           AND NOT EXISTS (SELECT 1 FROM pg_depend d
                            WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e')
           -- déjà conforme : `search_path` commençant par pg_catalog et finissant par pg_temp
           AND NOT EXISTS (SELECT 1 FROM unnest(coalesce(p.proconfig, '{}'::text[])) c
                            WHERE c ~ '^search_path=pg_catalog, .*pg_temp$')
         ORDER BY p.oid
    LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog, public, pg_temp', f);
    END LOOP;
END
$$;
