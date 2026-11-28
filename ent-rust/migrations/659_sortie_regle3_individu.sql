-- 659 — Transverse « la deuxième personne fondée sur la personne physique », lot 2 : voyages (socle 42).
-- Conception : docs/socles/TRANSVERSE_INDIVIDU.md §5. Dernière de la plage 640–659 du socle 42 (vérifiée libre sur toutes
-- les branches). Faible 9 de la contre-revue V3 : la règle 3 et la seconde personne comparaient des COMPTES.
--
-- Additive : 640–658 sont versées et ne changent pas (règle 25) ; les fonctions recréées ici gardent leur signature et
-- leur corps, sauf les comparaisons converties sur l'individu (279) ; elles portent `SET search_path = pg_catalog,
-- public, pg_temp` et des relations qualifiées `public.` (règle 38 : fin de tableau, après 279). Aucune table, aucun
-- déclencheur recréé (ils appellent les fonctions par leur nom). Rejouable.
--
-- 1. Règle 3 : `ent_sortie_argent_decideur_interdit` regarde TOUTES les fiches et tous les comptes de l'individu du
--    décideur : un lien familial porté par sa fiche parent (avec ou sans compte), le compte d'élève, la personne ou le
--    foyer payeur. Tous les déclencheurs de pouvoir sur l'argent (653, 655, 656, 657) en héritent.
-- 2. Seconde validation (653) : le valideur n'est pas le même individu que le demandeur.
-- 3. Contournement (658, 657) : la décision, et l'autorisation qui ouvre l'usage, viennent d'un autre individu que la
--    demandeuse. Restent sur le compte (principe 1 : ce qui permet) : seule l'autrice révoque, seule la demandeuse utilise.

-- 1. Règle 3 ------------------------------------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION ent_sortie_argent_decideur_interdit(etab UUID, usr UUID, compte UUID, eleves UUID[])
RETURNS BOOLEAN LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    WITH moi AS (
        SELECT public.ent_individu_personnes_du_compte(etab, usr) AS ps,
               public.ent_individu_comptes(etab, usr) AS us
    )
    SELECT usr IS NOT NULL AND (
           EXISTS (SELECT 1 FROM public.ent_eleve_lien l, moi
                    WHERE l.etablissement_id = etab AND l.eleve_id = ANY(eleves)
                      AND (l.personne_id = ANY(moi.ps) OR l.user_id = ANY(moi.us)))
        OR EXISTS (SELECT 1 FROM public.ent_eleve_compte ec, moi
                    WHERE ec.etablissement_id = etab AND ec.user_id = ANY(moi.us) AND ec.eleve_id = ANY(eleves))
        OR EXISTS (SELECT 1 FROM public.ent_personne p, moi
                    WHERE p.etablissement_id = etab AND p.id = ANY(moi.ps) AND p.eleve_id = ANY(eleves))
        OR EXISTS (SELECT 1 FROM public.ent_fin_compte c, moi
                    WHERE c.etablissement_id = etab AND c.id = compte
                      AND (EXISTS (SELECT 1 FROM public.ent_eleve_lien l
                                    WHERE l.etablissement_id = etab AND l.foyer_id = c.foyer_id
                                      AND (l.personne_id = ANY(moi.ps) OR l.user_id = ANY(moi.us)))
                        OR c.personne_id = ANY(moi.ps)
                        OR EXISTS (SELECT 1 FROM public.ent_personne p
                                    WHERE p.etablissement_id = etab AND p.id = c.personne_id
                                      AND p.user_id = ANY(moi.us))
                        OR EXISTS (SELECT 1 FROM public.ent_personne p
                                    JOIN public.ent_eleve_compte ec ON ec.etablissement_id = etab AND ec.eleve_id = p.eleve_id
                                   WHERE p.etablissement_id = etab AND p.id = c.personne_id AND ec.user_id = ANY(moi.us)))))
$$;

-- 2. Seconde validation -------------------------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION ent_sortie_argent_garde_decideur() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    dem public.ent_sortie_argent_demande%ROWTYPE;
    qui UUID;
    eleves UUID[];
    cpt UUID;
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_argent_demande' THEN
        qui := NEW.demande_par_user_id; cpt := NEW.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM public.ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id IN (NEW.participant_id, COALESCE(NEW.vers_participant_id, NEW.participant_id));
    ELSIF TG_TABLE_NAME = 'ent_sortie_aide' THEN
        qui := NEW.decide_par_user_id; cpt := NEW.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM public.ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id = NEW.participant_id;
    ELSIF TG_TABLE_NAME = 'ent_sortie_report' THEN
        qui := NEW.decide_par_user_id; cpt := NEW.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM public.ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id IN (NEW.de_participant_id, NEW.vers_participant_id);
    ELSE
        -- Issue : seule une issue « appliquée » exerce le pouvoir ; la seconde personne n'est pas le demandeur.
        SELECT * INTO dem FROM public.ent_sortie_argent_demande WHERE etablissement_id = NEW.etablissement_id AND id = NEW.demande_id;
        IF NEW.issue <> 'appliquee' THEN
            RETURN NEW;
        END IF;
        -- 659 : la seconde personne est un autre INDIVIDU que le demandeur.
        IF dem.seconde_validation AND public.ent_meme_individu(NEW.etablissement_id, NEW.par_user_id, dem.demande_par_user_id) THEN
            RAISE EXCEPTION 'une seconde personne valide : le demandeur ne se valide pas lui-même' USING ERRCODE = '23514';
        END IF;
        qui := NEW.par_user_id; cpt := dem.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM public.ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id IN (dem.participant_id, COALESCE(dem.vers_participant_id, dem.participant_id));
    END IF;
    IF public.ent_sortie_argent_decideur_interdit(NEW.etablissement_id, qui, cpt, COALESCE(eleves, ARRAY[]::uuid[])) THEN
        RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas pour son propre enfant ni pour son propre compte'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

-- 3. Contournement ------------------------------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION ent_sortie_contournement_evt_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    c public.ent_sortie_argent_contournement%ROWTYPE;
    au UUID;
    au_le TIMESTAMPTZ;
BEGIN
    NEW.le := now();
    SELECT * INTO c FROM public.ent_sortie_argent_contournement
     WHERE etablissement_id = NEW.etablissement_id AND id = NEW.contournement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'un contournement inconnu ne se décide ni ne s''utilise pas' USING ERRCODE = '23514';
    END IF;
    -- M-2 (658) : verrou de la sortie, même clé que `verrou_sortie` côté application. Décision, usage et révocation d'un même
    -- contournement se sérialisent, y compris en SQL direct : une révocation et un usage concurrents ne passent plus tous
    -- les deux (chacun lit les faits de l'autre une fois le verrou obtenu, en lecture validée).
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('voyages_argent:' || c.sortie_id::text, 0));
    IF NEW.evenement IN ('autorise', 'refuse') THEN
        -- 659 : la décision vient d'un autre INDIVIDU que la demandeuse (un second compte de la demandeuse ne décide pas).
        IF public.ent_meme_individu(NEW.etablissement_id, NEW.par_user_id, c.demande_par_user_id) THEN
            RAISE EXCEPTION 'la décision d''un contournement vient d''une autre personne que la demandeuse' USING ERRCODE = '23514';
        END IF;
        IF NOT public.ent_sortie_habilite_argent(NEW.etablissement_id, NEW.par_user_id) THEN
            RAISE EXCEPTION 'la décision d''un contournement vient d''un membre habilité de l''établissement' USING ERRCODE = '23514';
        END IF;
        IF now() > c.demande_le + interval '30 days' THEN
            RAISE EXCEPTION 'la décision d''un contournement ne se prend plus : la demande a expiré' USING ERRCODE = '23514';
        END IF;
        IF NEW.evenement = 'autorise' AND NOT public.ent_sortie_libre_de_la_sortie(NEW.etablissement_id, c.sortie_id, NEW.par_user_id) THEN
            RAISE EXCEPTION 'la personne qui autorise un contournement est libre de tout lien avec la sortie' USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.evenement = 'revoque' THEN
        SELECT e.par_user_id INTO au FROM public.ent_sortie_argent_contournement_evt e
         WHERE e.etablissement_id = NEW.etablissement_id AND e.contournement_id = NEW.contournement_id
           AND e.evenement = 'autorise';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'un contournement non autorisé ne se révoque pas' USING ERRCODE = '23514';
        END IF;
        IF NEW.par_user_id <> au THEN
            RAISE EXCEPTION 'seule l''autrice d''une autorisation la révoque' USING ERRCODE = '23514';
        END IF;
        IF EXISTS (SELECT 1 FROM public.ent_sortie_argent_contournement_evt e
                    WHERE e.etablissement_id = NEW.etablissement_id AND e.contournement_id = NEW.contournement_id
                      AND e.evenement = 'utilise') THEN
            RAISE EXCEPTION 'un contournement déjà utilisé ne se révoque plus' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF NEW.par_user_id <> c.demande_par_user_id THEN
            RAISE EXCEPTION 'seule la demandeuse utilise son contournement' USING ERRCODE = '23514';
        END IF;
        SELECT e.par_user_id, e.le INTO au, au_le FROM public.ent_sortie_argent_contournement_evt e
         WHERE e.etablissement_id = NEW.etablissement_id AND e.contournement_id = NEW.contournement_id
           AND e.evenement = 'autorise';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'un contournement non autorisé ne s''utilise pas' USING ERRCODE = '23514';
        END IF;
        IF EXISTS (SELECT 1 FROM public.ent_sortie_argent_contournement_evt e
                    WHERE e.etablissement_id = NEW.etablissement_id AND e.contournement_id = NEW.contournement_id
                      AND e.evenement = 'revoque') THEN
            RAISE EXCEPTION 'un contournement révoqué ne s''utilise pas' USING ERRCODE = '23514';
        END IF;
        IF now() > au_le + interval '30 days' THEN
            RAISE EXCEPTION 'un contournement expiré ne s''utilise pas' USING ERRCODE = '23514';
        END IF;
        IF NOT public.ent_sortie_habilite_argent(NEW.etablissement_id, au)
           OR NOT public.ent_sortie_libre_de_la_sortie(NEW.etablissement_id, c.sortie_id, au) THEN
            RAISE EXCEPTION 'un contournement dont l''autorisatrice n''est plus habilitée ne s''utilise pas' USING ERRCODE = '23514';
        END IF;
        IF c.cible_id IS NOT NULL AND NEW.cible_id IS DISTINCT FROM c.cible_id THEN
            RAISE EXCEPTION 'un contournement ne couvre pas cette cible' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_sortie_contournement_ouvert(
    etab UUID, cid UUID, sortie UUID, usr UUID, act TEXT, cible UUID, montant BIGINT, presta UUID, at TIMESTAMPTZ)
RETURNS BOOLEAN LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM public.ent_sortie_argent_contournement c
          JOIN public.ent_sortie_argent_contournement_evt ok
            ON ok.etablissement_id = c.etablissement_id AND ok.contournement_id = c.id AND ok.evenement = 'autorise'
         WHERE c.etablissement_id = etab AND c.id = cid AND c.sortie_id = sortie AND c.action = act
           AND c.demande_par_user_id = usr
           AND NOT public.ent_meme_individu(etab, ok.par_user_id, usr)
           AND ok.le + interval '30 days' >= at
           AND (c.cible_id IS NULL OR c.cible_id = cible)
           AND (c.montant_max_centimes IS NULL OR montant <= c.montant_max_centimes)
           AND (c.prestataire_id IS NULL OR presta = c.prestataire_id)
           AND NOT EXISTS (SELECT 1 FROM public.ent_sortie_argent_contournement_evt rv
                            WHERE rv.etablissement_id = c.etablissement_id AND rv.contournement_id = c.id
                              AND rv.evenement = 'revoque')
           AND public.ent_sortie_habilite_argent(etab, ok.par_user_id)
           AND public.ent_sortie_libre_de_la_sortie(etab, sortie, ok.par_user_id))
$$;
