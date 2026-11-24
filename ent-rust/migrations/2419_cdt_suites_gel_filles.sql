-- Socle 20, suites C8/C9 — migration 2419 (dernier numéro libre de la plage 2400–2419) : trois durcissements du cahier.
-- Réf. : docs/socles/20_CAHIER_C8_C9_LIVRAISON.md, « Reste à faire » ; règles 25 et 38 du pilotage.
-- Additive et rejouable. Placée à la FIN du tableau de `migrate.rs` (règle 38). Aucune colonne JSON (D-036).
-- Toute fonction créée ici porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`.

-- ── 1. La reprise de l'ancien cahier n'est pas appelable par le rôle applicatif ───────────────────────────────────────
-- `ent_cdt_reprise(uuid)` (2412, versée) écrit dans tout le cahier, de tous les établissements si `p_etab` est NULL. Elle ne
-- sert qu'aux migrations (propriétaire) : aucun code applicatif ne l'appelle. Un `CREATE OR REPLACE` conserve les droits.
REVOKE ALL ON FUNCTION public.ent_cdt_reprise(uuid) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ent_app') THEN
        REVOKE ALL ON FUNCTION public.ent_cdt_reprise(uuid) FROM ent_app;
    END IF;
END
$$;

-- ── 2. Le gel d'année échoue FERMÉ (F3) ───────────────────────────────────────────────────────────────────────────────
-- Avant : une année dont la ligne n'était pas visible (RLS, ligne absente) était « non gelée » (COALESCE(…, false)) et
-- l'écriture passait. Désormais : un identifiant d'année non NULL dont la ligne n'est pas lue est GELÉ. (Un identifiant
-- NULL reste « non gelé » : c'est l'absence de rattachement à une année, que les gardes traitent à part.)
CREATE OR REPLACE FUNCTION public.ent_cdt_annee_gelee(p_etab UUID, p_annee UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT CASE
        WHEN p_annee IS NULL THEN false
        ELSE COALESCE((SELECT a.cloturee_le IS NOT NULL FROM public.ent_annee_scolaire a
                        WHERE a.etablissement_id = p_etab AND a.id = p_annee), true)
             OR public.ent_cdt_cahier_fige(p_etab, p_annee)
    END
$$;

-- ── 3. F2 : tables filles non classées au gel d'année ─────────────────────────────────────────────────────────────────
-- ent_cdt_groupe / ent_cdt_groupe_membre (groupes de remise d'un travail), ent_cdt_note_accompagnement (note d'une AESH sur
-- une entrée ou un travail), ent_cdt_continuite_cours (cours couverts par une continuité pédagogique : l'année est celle du
-- cours). Comme leurs tables mères : aucune écriture dans une année gelée, et fail-closed ici aussi (un parent sans année
-- lisible refuse l'écriture). Prudence (D-069) : une continuité qui toucherait un cours d'une année close ne s'y rattache
-- plus ; la réouverture tracée du cahier est le seul chemin.
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
        IF TG_TABLE_NAME = 'ent_cdt_groupe' THEN
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
    FOREACH t IN ARRAY ARRAY['ent_cdt_groupe', 'ent_cdt_groupe_membre', 'ent_cdt_note_accompagnement', 'ent_cdt_continuite_cours']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON public.%I', t || '_annee_figee', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE INSERT OR UPDATE OR DELETE ON public.%I FOR EACH ROW '
                       'EXECUTE FUNCTION public.ent_cdt_ecriture_gardee_fille()', t || '_annee_figee', t);
    END LOOP;
    -- Contre-revue du pilote (Moyen 2, D-069) : pas de règle « jamais supprimé » sur ces tables (non demandée ; les notes
    -- d'accompagnement sont des données sensibles soumises à minimisation et à effacement). Le gel d'année protège déjà les
    -- années closes ; les clés étrangères RESTRICT protègent les liens. TRUNCATE (Faible 1) est refusé comme pour 2407.
    FOREACH t IN ARRAY ARRAY['ent_cdt_groupe', 'ent_cdt_groupe_membre', 'ent_cdt_note_accompagnement', 'ent_cdt_continuite_cours']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON public.%I', t || '_jamais_supprime', t);
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON public.%I', t || '_sans_truncate', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON public.%I FOR EACH STATEMENT '
                       'EXECUTE FUNCTION public.ent_cdt_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
    DROP TRIGGER IF EXISTS ent_cdt_continuite_cours_ajout_seul ON public.ent_cdt_continuite_cours;
    CREATE TRIGGER ent_cdt_continuite_cours_ajout_seul BEFORE UPDATE OR DELETE ON public.ent_cdt_continuite_cours
        FOR EACH ROW EXECUTE FUNCTION public.ent_cdt_ajout_seul();
END
$$;
