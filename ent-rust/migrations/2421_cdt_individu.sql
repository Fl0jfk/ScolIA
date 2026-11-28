-- 2421 — Transverse « la deuxième personne fondée sur la personne physique », lot 3 : cahier de textes (socle 20).
-- Conception : docs/socles/TRANSVERSE_INDIVIDU.md §5. Plage d'extension 2420–2429 du socle 20 (2421 vérifiée libre sur
-- toutes les branches). Règle 38 : en fin de tableau, après 279 (fonctions de l'individu). 2407 est versée et ne change pas
-- (règle 25) : sa fonction est recréée à l'identique, sauf la comparaison convertie, avec `SET search_path` et des
-- relations qualifiées `public.`. Les CHECK de 2407 (comptes distincts) restent : ils sont plus faibles et toujours vrais.
--
-- 1. Réouverture du cahier (M1 de C8) : le validateur n'est pas le même individu que le demandeur.
-- 2. Accès d'inspection : nul ne s'accorde un accès à lui-même, ni à un autre de ses comptes.

CREATE OR REPLACE FUNCTION ent_cdt_cloture_coherence() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    v_dem public.ent_cdt_cloture%ROWTYPE;
    v_id UUID;
BEGIN
    PERFORM 1 FROM public.ent_annee_scolaire a WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.annee_id FOR NO KEY UPDATE;
    NEW.cree_le := pg_catalog.clock_timestamp();
    IF NEW.action = 'cloture' THEN
        IF public.ent_cdt_cahier_fige(NEW.etablissement_id, NEW.annee_id) THEN
            RAISE EXCEPTION 'journal de clôture : le cahier de cette année est déjà clôturé' USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.action = 'reouverture_demandee' THEN
        IF NOT public.ent_cdt_cahier_fige(NEW.etablissement_id, NEW.annee_id) THEN
            RAISE EXCEPTION 'journal de clôture : demande de réouverture sans cahier clôturé' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF NOT public.ent_cdt_cahier_fige(NEW.etablissement_id, NEW.annee_id) THEN
            RAISE EXCEPTION 'journal de clôture : réouverture sans cahier clôturé' USING ERRCODE = '23514';
        END IF;
        v_id := public.ent_cdt_demande_reouverture(NEW.etablissement_id, NEW.annee_id);
        IF v_id IS NULL THEN
            RAISE EXCEPTION 'journal de clôture : réouverture sans demande valable (absente, expirée ou déjà consommée)'
                USING ERRCODE = '23514';
        END IF;
        SELECT * INTO v_dem FROM public.ent_cdt_cloture WHERE id = v_id;
        -- 2421 : la réouverture valide la demande d'un autre INDIVIDU (un second compte du demandeur ne vaut pas).
        IF public.ent_meme_individu(NEW.etablissement_id, v_dem.par_user_id, NEW.par_user_id)
           OR NEW.demandeur_user_id IS DISTINCT FROM v_dem.par_user_id THEN
            RAISE EXCEPTION 'journal de clôture : la réouverture doit valider la demande d''une autre personne'
                USING ERRCODE = '23514';
        END IF;
        IF NEW.motif IS DISTINCT FROM v_dem.motif THEN
            RAISE EXCEPTION 'journal de clôture : la réouverture reprend le motif de la demande' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_cdt_acces_inspection_pas_soi() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF public.ent_meme_individu(NEW.etablissement_id, NEW.beneficiaire_user_id, NEW.accorde_par_user_id) THEN
        RAISE EXCEPTION 'un accès d''inspection ne s''accorde pas à soi-même' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_cdt_acces_inspection_pas_soi_trg ON ent_cdt_acces_inspection;
CREATE TRIGGER ent_cdt_acces_inspection_pas_soi_trg BEFORE INSERT ON ent_cdt_acces_inspection
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_acces_inspection_pas_soi();
