-- 658 — Socle 42, palier des faibles regroupés, lot 2 (contre-revue V3b suites 2 : M-2, D-069).
-- Réf. : docs/pilotage/revues/42_VOYAGES_V3B_SUITES_2_CONTRE_REVUE.md (sur ent-mc0hoe) ; docs/socles/PALIER_FAIBLES_1_LIVRAISON.md.
--
-- Additive : 640–657 sont versées et ne changent pas (règle 25). Règle 38 : cette migration est en FIN de tableau ; la
-- fonction recréée porte `SET search_path = pg_catalog, public, pg_temp` et des relations qualifiées `public.`.
-- Rejouable : fonction recréée à l'identique. Aucune donnée touchée, aucune table, aucun déclencheur recréé (celui de 656
-- appelle la fonction par son nom).
--
-- M-2 : la garde des faits de contournement (657) lisait « déjà utilisé » et « déjà révoqué » sans verrou. Deux
-- transactions concurrentes en SQL direct (une révocation, un usage) passaient donc toutes les deux. La garde reprend le
-- verrou consultatif de la sortie avant toute lecture des faits. Le reste est identique à 657.

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
        IF NEW.par_user_id = c.demande_par_user_id THEN
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
