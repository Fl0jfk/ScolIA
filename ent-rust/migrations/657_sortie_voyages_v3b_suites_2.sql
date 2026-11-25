-- 657 — Socle 42, lot V3b « suites, 2 » (contre-revue des correctifs V3b : M2, faibles 1 et 5, D-069).
-- Réf. : docs/pilotage/revues/42_VOYAGES_V3B_CORRECTIFS_CONTRE_REVUE.md ; docs/socles/42_VOYAGES_V3_LIVRAISON.md.
--
-- Additive : 640–656 sont versées et ne changent pas (règle 25). Règle 38 : cette migration est en FIN de tableau ; chaque
-- fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et des relations qualifiées `public.`.
-- Rejouable : fonctions recréées à l'identique, contraintes et index posés seulement s'ils manquent. Aucune donnée touchée.

-- ───────────────────────── Faible 5 : une autorisation se révoque par son autrice (ajout seul) ─────────────────────────
-- Nouveau fait « revoque » : l'autrice de l'autorisation retire son accord tant qu'il n'a pas servi. La base l'applique : une
-- autorisation révoquée n'ouvre plus rien (`ent_sortie_contournement_ouvert`) et ne s'utilise plus.
ALTER TABLE public.ent_sortie_argent_contournement_evt
    DROP CONSTRAINT IF EXISTS ent_sortie_argent_contournement_evt_evenement_check;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie_argent_contournement_evt'::regclass
                    AND conname = 'ent_sortie_argent_contournement_evt_evenement_chk') THEN
        ALTER TABLE public.ent_sortie_argent_contournement_evt
            ADD CONSTRAINT ent_sortie_argent_contournement_evt_evenement_chk
            CHECK (evenement IN ('autorise', 'refuse', 'utilise', 'revoque'));
    END IF;
END
$$;
-- Une révocation par autorisation.
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_argent_contournement_evt_revocation_uq
    ON public.ent_sortie_argent_contournement_evt (etablissement_id, contournement_id) WHERE evenement = 'revoque';

-- Garde des faits (656) reprise avec la révocation : seule l'autrice de l'autorisation révoque, une autorisation non
-- encore utilisée ; une autorisation révoquée ne s'utilise pas. Le reste est identique à 656.
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

-- Une autorisation ouverte (656) n'est plus ouverte une fois révoquée.
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
           AND c.demande_par_user_id = usr AND ok.par_user_id <> usr
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

-- ───────────────────────── Moyen 2 (revue) : borne de l'annulation collective ─────────────────────────
-- Argent net d'un payeur pour un participant : versements et reports reçus moins annulations et reports cédés (jamais les
-- aides) ; même calcul que `verses_cash` côté application.
CREATE OR REPLACE FUNCTION ent_sortie_paye_net(etab UUID, sortie UUID, part UUID, cpte UUID)
RETURNS BIGINT LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT COALESCE(SUM(x.m), 0)::bigint FROM (
        SELECT CASE WHEN v.sens = 'versement' THEN v.montant_centimes ELSE -v.montant_centimes END AS m
          FROM public.ent_sortie_versement v
         WHERE v.etablissement_id = etab AND v.sortie_id = sortie AND v.participant_id = part AND v.compte_id = cpte
        UNION ALL
        SELECT CASE WHEN r.sens = 'report' THEN r.montant_centimes ELSE -r.montant_centimes END
          FROM public.ent_sortie_report r
         WHERE r.etablissement_id = etab AND r.sortie_id = sortie AND r.vers_participant_id = part AND r.compte_id = cpte
        UNION ALL
        SELECT CASE WHEN r.sens = 'report' THEN -r.montant_centimes ELSE r.montant_centimes END
          FROM public.ent_sortie_report r
         WHERE r.etablissement_id = etab AND r.sortie_id = sortie AND r.de_participant_id = part AND r.compte_id = cpte
    ) x
$$;

-- `ent_sortie_argent_pouvoir_libre` (656) reprise avec :
--  * Moyen 2 : un avoir « annulation collective » dont le couple (participant, compte) n'a rien versé (net nul ou négatif),
--    ou dont `paye_net_centimes` dépasse l'argent net réellement versé, est refusé (23514) ;
--  * Faible 1 : la garde porte aussi sur `NEW.participant_id` et `NEW.compte_id` (et non seulement sur les participants
--    qui ont versé) ; la décision portée par la ligne (`decide_par_user_id`) est contrôlée même dans la branche collective.
-- Le reste est identique à 656.
CREATE OR REPLACE FUNCTION ent_sortie_argent_pouvoir_libre() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    qui UUID;
    cpt UUID;
    cpt_avoir UUID;
    pids UUID[];
    eleves UUID[];
    act TEXT;
    cible UUID;
    montant BIGINT;
    presta UUID;
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_plan' THEN
        qui := NEW.accorde_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_versement' THEN
        qui := NEW.saisi_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_frais_reel' THEN
        qui := NEW.saisi_par_user_id; cpt := NULL; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_avoir_propose' THEN
        IF TG_OP = 'INSERT' AND NEW.motif_code = 'annulation_collective' THEN
            IF NEW.propose_par_user_id IS NULL THEN
                RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas sans auteur : l''annulation collective porte sa personne'
                    USING ERRCODE = '23514';
            END IF;
            -- Moyen 2 : l'avoir ne dépasse jamais l'argent net réellement versé par ce payeur pour ce participant.
            IF NEW.paye_net_centimes <= 0
               OR NEW.paye_net_centimes > public.ent_sortie_paye_net(NEW.etablissement_id, NEW.sortie_id, NEW.participant_id, NEW.compte_id) THEN
                RAISE EXCEPTION 'un avoir d''annulation collective ne dépasse pas l''argent versé par ce payeur pour ce participant'
                    USING ERRCODE = '23514';
            END IF;
            -- Faible 1 : la décision portée par la ligne est contrôlée comme celle d'un avoir individuel (aucun contournement).
            IF NEW.decide_par_user_id IS NOT NULL AND NEW.statut IN ('retenu', 'ecarte')
               AND public.ent_sortie_argent_decideur_interdit(
                       NEW.etablissement_id, NEW.decide_par_user_id, NEW.compte_id,
                       COALESCE((SELECT array_agg(p.eleve_id) FROM public.ent_sortie_participant p
                                  WHERE p.etablissement_id = NEW.etablissement_id AND p.id = NEW.participant_id),
                                ARRAY[]::uuid[])) THEN
                RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas pour son propre enfant ni pour son propre compte'
                    USING ERRCODE = '23514';
            END IF;
            -- Contre-revue du pilote (M-1, M-2) : la borne vaut pour le CUMUL. Sous le verrou de la sortie (même clé que
            -- `verrou_sortie`, ce qui sérialise aussi révocation et usage), un payeur déjà couvert pour ce participant par
            -- un avoir actif (proposé, retenu ou à revoir, désistement compris) n'en reçoit pas un second : même règle que
            -- l'application (`collective_participant`).
            PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('voyages_argent:' || NEW.sortie_id::text, 0));
            IF EXISTS (SELECT 1 FROM public.ent_sortie_avoir_propose a
                        WHERE a.etablissement_id = NEW.etablissement_id AND a.participant_id = NEW.participant_id
                          AND a.compte_id = NEW.compte_id AND a.statut IN ('propose', 'retenu', 'a_revoir')) THEN
                RAISE EXCEPTION 'un avoir d''annulation collective ne dépasse pas l''argent versé : ce payeur est déjà couvert pour ce participant'
                    USING ERRCODE = '23514';
            END IF;
            qui := NEW.propose_par_user_id; cpt := NULL; cpt_avoir := NEW.compte_id;
            act := 'annulation_collective'; cible := NEW.sortie_id;
            -- Les participants qui ont versé, et celui de la ligne : la sortie entière décide l'annulation collective.
            SELECT array_agg(DISTINCT v.participant_id) INTO pids FROM public.ent_sortie_versement v
             WHERE v.etablissement_id = NEW.etablissement_id AND v.sortie_id = NEW.sortie_id;
            pids := COALESCE(pids, ARRAY[]::uuid[]) || NEW.participant_id;
        ELSIF NEW.decide_par_user_id IS NULL OR NEW.statut NOT IN ('retenu', 'ecarte')
           OR (TG_OP = 'UPDATE' AND OLD.decide_par_user_id IS NOT DISTINCT FROM NEW.decide_par_user_id
               AND OLD.statut IS NOT DISTINCT FROM NEW.statut) THEN
            RETURN NEW;
        ELSE
            qui := NEW.decide_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
        END IF;
    ELSE
        -- Acompte fournisseur : création, paiement, annulation. La sortie entière est concernée.
        presta := NEW.prestataire_id; cible := NEW.id;
        IF TG_OP = 'INSERT' THEN
            IF NEW.paye_le IS NOT NULL OR NEW.paye_centimes IS NOT NULL OR NEW.paye_par_user_id IS NOT NULL
               OR NEW.annule_le IS NOT NULL OR NEW.annule_par_user_id IS NOT NULL THEN
                RAISE EXCEPTION 'un acompte fournisseur se crée ni payé ni annulé : le paiement et l''annulation sont des étapes à part'
                    USING ERRCODE = '23514';
            END IF;
            qui := NEW.cree_par_user_id; act := 'acompte_creer';
            montant := NEW.montant_centimes;
        ELSIF NEW.paye_par_user_id IS NOT NULL AND OLD.paye_par_user_id IS NULL THEN
            qui := NEW.paye_par_user_id; act := 'acompte_payer';
            montant := COALESCE(NEW.paye_centimes, NEW.montant_centimes);
        ELSIF NEW.annule_le IS NOT NULL AND OLD.annule_le IS NULL AND NEW.annule_par_user_id IS NOT NULL THEN
            qui := NEW.annule_par_user_id; act := 'acompte_annuler';
        ELSE
            RETURN NEW;
        END IF;
        cpt := NULL;
        SELECT array_agg(p.id) INTO pids FROM public.ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.sortie_id = NEW.sortie_id;
    END IF;
    pids := COALESCE(pids, ARRAY[]::uuid[]);
    SELECT array_agg(p.eleve_id) INTO eleves FROM public.ent_sortie_participant p
     WHERE p.etablissement_id = NEW.etablissement_id AND p.id = ANY(pids);
    eleves := COALESCE(eleves, ARRAY[]::uuid[]);
    IF public.ent_sortie_argent_decideur_interdit(NEW.etablissement_id, qui, cpt, eleves)
       OR (cpt_avoir IS NOT NULL
           AND public.ent_sortie_argent_decideur_interdit(NEW.etablissement_id, qui, cpt_avoir, ARRAY[]::uuid[]))
       OR (cpt IS NULL AND EXISTS (SELECT 1 FROM public.ent_sortie_part q
                                    WHERE q.etablissement_id = NEW.etablissement_id AND q.participant_id = ANY(pids)
                                      AND public.ent_sortie_argent_decideur_interdit(
                                            NEW.etablissement_id, qui, q.compte_id, ARRAY[]::uuid[]))) THEN
        IF act IS NOT NULL
           AND public.ent_sortie_contournement_valide(NEW.etablissement_id, NEW.sortie_id, qui, act, cible, montant, presta) THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas pour son propre enfant ni pour son propre compte'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
