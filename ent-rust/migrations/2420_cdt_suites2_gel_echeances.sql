-- Socle 20, suites 2 du cahier C8/C9 — migration 2420 (première de la plage d'extension 2420–2429, D-069) : deux durcissements.
-- Réf. : docs/pilotage/revues/20_CAHIER_SUITES_CONTRE_REVUE.md (faibles 6 et 8) ; règles 25 et 38 du pilotage.
-- Additive et rejouable. Placée à la FIN du tableau de `migrate.rs` (règle 38). 2419 est versée : elle ne change pas.
-- Aucune colonne JSON (D-036). Toute fonction créée ici porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses
-- relations `public.`.

-- ── 1. Faible 6 : la reprise n'est exécutable par aucun rôle autre que son propriétaire ───────────────────────────────
-- 2419 retirait le droit à PUBLIC et au seul rôle nommé `ent_app`. Ici : tout rôle qui porte une autorisation explicite sur
-- `ent_cdt_reprise(uuid)` la perd, quel que soit son nom (aucun nom codé en dur), sauf le propriétaire de la fonction et le
-- rôle courant (celui des migrations : il ne se retire pas son propre droit).
REVOKE ALL ON FUNCTION public.ent_cdt_reprise(uuid) FROM PUBLIC;
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT DISTINCT ro.rolname
          FROM pg_catalog.pg_proc p
          CROSS JOIN LATERAL pg_catalog.aclexplode(p.proacl) a
          JOIN pg_catalog.pg_roles ro ON ro.oid = a.grantee
         WHERE p.oid = 'public.ent_cdt_reprise(uuid)'::pg_catalog.regprocedure
           AND a.grantee <> 0
           AND a.grantee <> p.proowner
           AND ro.rolname <> current_user
    LOOP
        EXECUTE pg_catalog.format('REVOKE ALL ON FUNCTION public.ent_cdt_reprise(uuid) FROM %I', r.rolname);
    END LOOP;
END
$$;

-- ── 2. Faible 8 (D-069, valeur prudente) : prolongations et demandes de délai rejoignent le gel des années closes ────────
-- `ent_cdt_echeance_eleve` (prolongation individuelle, ajout seul) et `ent_cdt_delai_demande` (demande d'un élève ou d'une
-- famille, décidée par le professeur) portent un travail : leur année est celle du travail. Même mécanisme que les autres
-- tables filles (2419) : aucune écriture dans une année gelée, échec fermé (parent sans année lisible : refus), ancienne ET
-- nouvelle ligne examinées (une ligne ne se déplace ni vers ni depuis une année close). La fonction de 2419 est recréée avec
-- deux branches de plus.
CREATE OR REPLACE FUNCTION public.ent_cdt_ecriture_gardee_fille() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_etab UUID;
    v_annees UUID[] := ARRAY[]::uuid[];
    v_annee UUID;
    r RECORD;
    i INTEGER;
BEGIN
    FOR i IN 1..2
    LOOP
        IF i = 1 THEN
            IF TG_OP = 'DELETE' THEN CONTINUE; END IF;
            r := NEW;
        ELSE
            IF TG_OP = 'INSERT' THEN CONTINUE; END IF;
            r := OLD;
        END IF;
        v_etab := r.etablissement_id;
        v_annee := NULL;
        IF TG_TABLE_NAME IN ('ent_cdt_groupe', 'ent_cdt_echeance_eleve', 'ent_cdt_delai_demande') THEN
            v_annee := public.ent_cdt_annee_de_travail(v_etab, r.travail_id);
        ELSIF TG_TABLE_NAME = 'ent_cdt_groupe_membre' THEN
            v_annee := (SELECT public.ent_cdt_annee_de_travail(g.etablissement_id, g.travail_id)
                          FROM public.ent_cdt_groupe g WHERE g.etablissement_id = v_etab AND g.id = r.groupe_id);
        ELSIF TG_TABLE_NAME = 'ent_cdt_note_accompagnement' THEN
            v_annee := CASE WHEN r.entree_id IS NOT NULL THEN public.ent_cdt_annee_de_entree(v_etab, r.entree_id)
                            ELSE public.ent_cdt_annee_de_travail(v_etab, r.travail_id) END;
        ELSE -- ent_cdt_continuite_cours
            v_annee := (SELECT c.annee_id FROM public.ent_cours c WHERE c.etablissement_id = v_etab AND c.id = r.cours_id);
        END IF;
        IF v_annee IS NULL THEN
            RAISE EXCEPTION 'cahier de textes : année introuvable ou invisible, écriture refusée (R-C28)' USING ERRCODE = '23514';
        END IF;
        IF NOT (v_annee = ANY (v_annees)) THEN
            v_annees := v_annees || v_annee;
        END IF;
    END LOOP;
    v_etab := CASE WHEN TG_OP = 'DELETE' THEN OLD.etablissement_id ELSE NEW.etablissement_id END;
    FOREACH v_annee IN ARRAY v_annees
    LOOP
        PERFORM 1 FROM public.ent_annee_scolaire a WHERE a.etablissement_id = v_etab AND a.id = v_annee FOR SHARE;
        IF public.ent_cdt_annee_gelee(v_etab, v_annee) THEN
            RAISE EXCEPTION 'cahier de textes : l''année est figée (clôturée), aucune écriture sans réouverture tracée (R-C28)'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END
$$;

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_cdt_echeance_eleve', 'ent_cdt_delai_demande']
    LOOP
        EXECUTE pg_catalog.format('DROP TRIGGER IF EXISTS %I ON public.%I', t || '_annee_figee', t);
        EXECUTE pg_catalog.format('CREATE TRIGGER %I BEFORE INSERT OR UPDATE OR DELETE ON public.%I FOR EACH ROW '
                                  'EXECUTE FUNCTION public.ent_cdt_ecriture_gardee_fille()', t || '_annee_figee', t);
        -- Comme les autres tables filles (2419, faible 1) : pas de TRUNCATE.
        EXECUTE pg_catalog.format('DROP TRIGGER IF EXISTS %I ON public.%I', t || '_sans_truncate', t);
        EXECUTE pg_catalog.format('CREATE TRIGGER %I BEFORE TRUNCATE ON public.%I FOR EACH STATEMENT '
                                  'EXECUTE FUNCTION public.ent_cdt_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
END
$$;
