-- 655 — Socle 42, lot V3 « suites » (contre-revue de V3, D-069) : M-1, M-3, M-4, F-d, F-h.
-- Réf. : docs/pilotage/revues/42_VOYAGES_V3_CONTRE_REVUE.md ; docs/socles/42_VOYAGES_V3_LIVRAISON.md.
--
-- Additive : 653 et 654 sont versées et ne changent pas (règle 25). Règle 38 : cette migration est en FIN de tableau ;
-- chaque fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et des relations qualifiées
-- `public.`. Exception nommée : `ent_internat_attendu_calcule` appartient à la famille « qualifiée » (fonction SQL qui
-- retourne un ensemble de lignes, inlinée dans ses appelants : un `SET` en ferait perdre l'inlining ; voir
-- `garde_search_path.rs`) : ses relations sont qualifiées `public.` et elle ne porte pas de clause `SET`, comme la garde l'exige.
--
-- Rejouable : colonnes, contraintes et déclencheurs ne sont posés que s'ils manquent (aucun DROP/CREATE à chaque rejeu :
-- pas de verrou ACCESS EXCLUSIVE inutile, F-h de la contre-revue). Les DROP TRIGGER/CREATE TRIGGER de 653 sont versés et
-- ne peuvent pas être modifiés (règle 25) : ils restent rejoués tels quels.

-- ───────────────────────── M-4 : l'instant d'annulation d'une sortie a sa propre colonne ─────────────────────────
-- L'attendu de l'internat d'une sortie annulée en cours de route ne retire que les nuits déjà passées dehors AVANT
-- l'annulation : cet instant ne se lit plus dans `modifie_le` (qu'un futur UPDATE de la sortie décalerait).
ALTER TABLE ent_sortie ADD COLUMN IF NOT EXISTS annulee_le TIMESTAMPTZ;
-- Rattrapage : la meilleure information disponible pour les sorties déjà annulées.
UPDATE ent_sortie SET annulee_le = modifie_le WHERE etat = 'annulee' AND annulee_le IS NULL;

-- Posée à la transition vers « annulée » (instant fourni par l'application, sinon l'heure de la base) ; immuable
-- ensuite ; vide pour toute autre étape.
CREATE OR REPLACE FUNCTION ent_sortie_annulee_le_pose() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF NEW.etat = 'annulee' THEN
        IF TG_OP = 'UPDATE' AND OLD.etat = 'annulee' THEN
            NEW.annulee_le := COALESCE(OLD.annulee_le, NEW.annulee_le, now());
        ELSE
            NEW.annulee_le := COALESCE(NEW.annulee_le, now());
        END IF;
    ELSE
        NEW.annulee_le := NULL;
    END IF;
    RETURN NEW;
END
$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie'::regclass
                    AND tgname = 'ent_sortie_annulee_le_tg') THEN
        CREATE TRIGGER ent_sortie_annulee_le_tg BEFORE INSERT OR UPDATE ON public.ent_sortie
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_annulee_le_pose();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie'::regclass
                    AND conname = 'ent_sortie_annulee_le_chk') THEN
        ALTER TABLE public.ent_sortie ADD CONSTRAINT ent_sortie_annulee_le_chk
            CHECK ((etat = 'annulee') = (annulee_le IS NOT NULL));
    END IF;
END
$$;

-- ───────────────────────── M-1 : une seule source de vérité pour le retrait « séjour scolaire » ─────────────────────────
-- Reprise de la définition de 2829 (dernière définition avant celle-ci), identique hors la condition commentée « 655 » :
-- la branche directe d'un séjour ne retire un élève que s'il existe un retrait EFFECTIF posé par la synchronisation.
CREATE OR REPLACE FUNCTION ent_internat_attendu_calcule(etab UUID, nuit DATE)
RETURNS TABLE (eleve_id UUID, chambre_id UUID, lit_id UUID, statut TEXT, motif_code TEXT)
LANGUAGE sql STABLE AS $$
    WITH ancre AS (
        SELECT ent_internat_ancre_nuit(etab, nuit) AS t, ent_internat_fuseau(etab) AS tz,
               (((nuit + 1) + TIME '06:00') AT TIME ZONE ent_internat_fuseau(etab)) AS matin
    ),
    du_rythme AS (
        SELECT s.eleve_id
          FROM public.ent_internat_sejour s
         WHERE s.etablissement_id = etab AND s.du <= nuit AND (s.au IS NULL OR s.au >= nuit)
           AND COALESCE((
                SELECT CASE extract(dow FROM nuit)::int
                         WHEN 0 THEN r.dimanche WHEN 1 THEN r.lundi WHEN 2 THEN r.mardi
                         WHEN 3 THEN r.mercredi WHEN 4 THEN r.jeudi WHEN 5 THEN r.vendredi
                         ELSE r.samedi END
                  FROM public.ent_internat_rythme r
                 WHERE r.etablissement_id = etab AND r.sejour_id = s.id AND r.du <= nuit
                 ORDER BY r.du DESC LIMIT 1), false)
    ),
    ajoutes AS (
        SELECT x.eleve_id FROM public.ent_internat_nuit_exceptionnelle x
         WHERE x.etablissement_id = etab AND x.nuit = ent_internat_attendu_calcule.nuit AND x.annulee_le IS NULL
    ),
    hebergees AS (
        SELECT DISTINCT ON (a.eleve_id) a.eleve_id, a.chambre_id, a.lit_id
          FROM public.ent_internat_affectation a
         WHERE a.etablissement_id = etab AND a.debut <= nuit AND (a.fin IS NULL OR a.fin >= nuit)
         ORDER BY a.eleve_id, a.debut DESC
    ),
    retraits AS (
        SELECT r.eleve_id, r.motif_code AS motif FROM public.ent_internat_retrait r
         WHERE r.etablissement_id = etab AND r.annule_le IS NULL AND r.du_nuit <= nuit AND r.au_nuit >= nuit
           -- 654 (D-069, règle 1) : un retrait posé pour un voyage ne vaut qu'à partir du DÉPART CONSTATÉ du participant.
           AND (r.source <> 'voyage' OR ent_sortie_retrait_effectif(etab, r.id))
        UNION ALL
        SELECT ab.eleve_id, 'absence_annoncee' FROM public.ent_internat_absence ab, ancre
         WHERE ab.etablissement_id = etab AND ab.du_nuit <= nuit AND ab.au_nuit >= nuit
           AND (ab.statut = 'regularisee'
                -- Passe 3 : une absence dont l'auteur ou la personne remise est devenu visé APRÈS le départ ouvre un cas
                -- pour le CPE ; tant qu'il n'a pas tranché (cas ouvert), les nuits SUIVANTES ne retirent plus l'élève.
                OR (NOT EXISTS (SELECT 1 FROM public.ent_internat_cas cx
                                 WHERE cx.etablissement_id = etab AND cx.eleve_id = ab.eleve_id
                                   AND cx.nature = 'interdiction_auteur' AND cx.issue IS NULL
                                   AND cx.nuit < ent_internat_attendu_calcule.nuit)
                    AND ab.statut IN ('annoncee', 'acceptee')
                    AND (ent_internat_eleve_majeur(etab, ab.eleve_id, nuit)
                         OR (ab.depart_effectif IS NOT NULL AND ab.depart_effectif <= ancre.t)))
                -- Blocage posé après le début d'une nuit : il ne vaut, pour un élève réellement parti, que pour les
                -- nuits suivantes.
                OR (ab.statut = 'bloquee' AND ab.blocage_effet_nuit IS NOT NULL AND nuit < ab.blocage_effet_nuit
                    AND ab.depart_effectif IS NOT NULL AND ab.depart_effectif <= ancre.t))
        UNION ALL
        -- E-3 : seule une sortie effectivement PARTIE (ou rentrée) retire l'élève de l'attendu. Une sortie
        -- autorisée qui n'a jamais eu lieu (départ refusé au portail, famille absente) le laisse attendu : sans
        -- cela, un élève resté à l'internat ne serait ni appelé ni cherché.
        SELECT so.eleve_id, 'sortie' FROM public.ent_internat_sortie so, ancre
         WHERE so.etablissement_id = etab AND so.statut IN ('partie', 'rentree')
           AND NOT EXISTS (SELECT 1 FROM public.ent_internat_cas cx
                            WHERE cx.etablissement_id = etab AND cx.eleve_id = so.eleve_id
                              AND cx.nature = 'interdiction_auteur' AND cx.issue IS NULL
                              AND cx.nuit < ent_internat_attendu_calcule.nuit)
           AND so.debut <= ancre.t
           AND COALESCE(so.retour_effectif, so.fin_ajustee) >= ancre.matin
        UNION ALL
        -- Règle 2 (I4) : une sortie de GROUPE qui couvre la nuit ne retire l'élève de l'attendu qu'après son DÉPART
        -- CONSTATÉ (parti avant l'ancre de la nuit) ; un élève autorisé qui n'est pas parti reste attendu.
        SELECT gp.eleve_id, 'sortie' FROM public.ent_internat_sortie_groupe_participant gp
          JOIN public.ent_internat_sortie_groupe sg ON sg.etablissement_id = gp.etablissement_id AND sg.id = gp.sortie_groupe_id
          CROSS JOIN ancre
         WHERE gp.etablissement_id = etab AND gp.etat IN ('parti', 'rentre') AND sg.statut IN ('acceptee', 'close')
           AND gp.parti_le <= ancre.t
           AND COALESCE(gp.rentre_le, sg.fin) >= ancre.matin
        UNION ALL
        SELECT sit.eleve_id,
               CASE sit.nature WHEN 'declaration' THEN 'absence_annoncee' WHEN 'sortie' THEN 'sortie'
                               WHEN 'mobilite' THEN 'mobilite' ELSE 'stage' END
          FROM public.ent_asd_situation sit, ancre
         WHERE sit.etablissement_id = etab AND sit.etat = 'active'
           AND sit.nature IN ('declaration', 'sortie', 'mobilite', 'pfmp')
           AND ((sit.debut_jour + COALESCE(sit.debut_heure, TIME '00:00')) AT TIME ZONE ancre.tz) <= ancre.t
           AND ancre.t < ((sit.fin_jour + COALESCE(sit.fin_heure, TIME '23:59:59')) AT TIME ZONE ancre.tz)
        UNION ALL
        SELECT m.eleve_id, 'mobilite' FROM public.ent_eleve_mobilite m
         WHERE m.etablissement_id = etab AND m.ferme_le IS NULL AND m.du <= nuit AND (m.au IS NULL OR m.au >= nuit)
        UNION ALL
        SELECT p.eleve_id, 'sejour_scolaire'
          FROM public.ent_sortie so
          JOIN public.ent_sortie_participant p ON p.etablissement_id = so.etablissement_id AND p.sortie_id = so.id
          CROSS JOIN ancre
         WHERE so.etablissement_id = etab AND so.etat IN ('prete', 'en_cours')
           -- 654 (D-069, règle 1) : « prêt » n'est pas « parti » ; seul le départ constaté retire l'élève de l'attendu.
           AND p.etat IN ('parti', 'remis', 'parti_seul_autorise')
           -- 655 (M-1, D-069) : UNE SEULE source de vérité. Un départ constaté ne retire l'élève que s'il existe un retrait
           -- EFFECTIF posé par la synchronisation (`internat_resynchroniser`) : un participant passé « parti » par une
           -- écriture directe, sans resynchronisation, reste attendu, appelé et cherché ; sans ligne de retrait, le CPE n'aurait
           -- d'ailleurs aucun `id` à annuler pour le rétablir.
           AND EXISTS (SELECT 1 FROM public.ent_sortie_internat_retrait le
                        WHERE le.etablissement_id = so.etablissement_id AND le.sortie_id = so.id
                          AND le.participant_id = p.id
                          AND public.ent_sortie_retrait_effectif(etab, le.retrait_id))
           -- 654 (M2) : une annulation du CPE (retrait « voyage » annulé hors synchronisation) est mémorisée et respectée.
           AND NOT EXISTS (SELECT 1 FROM public.ent_sortie_internat_retrait l
                             JOIN public.ent_internat_retrait rl ON rl.etablissement_id = l.etablissement_id AND rl.id = l.retrait_id
                            WHERE l.etablissement_id = so.etablissement_id AND l.sortie_id = so.id
                              AND l.participant_id = p.id AND rl.annule_le IS NOT NULL
                              AND NOT EXISTS (SELECT 1 FROM public.ent_sortie_internat_retrait_fin fi
                                               WHERE fi.etablissement_id = l.etablissement_id AND fi.retrait_id = l.retrait_id))
           AND so.debut <= ancre.t AND ancre.t < so.fin
        UNION ALL
        SELECT d.eleve_id, 'fermeture'
          FROM (SELECT eleve_id FROM du_rythme UNION SELECT eleve_id FROM ajoutes) d
         WHERE EXISTS (
                SELECT 1 FROM public.ent_calendrier_evenement ev
                 WHERE ev.etablissement_id = etab AND ev.type = 'fermeture_exceptionnelle'
                   AND ev.annule_le IS NULL AND 'internat' = ANY (ev.services_touches)
                   AND NOT ev.internes_presents AND ev.du <= nuit AND ev.au >= nuit)
    ),
    retrait_retenu AS (
        SELECT DISTINCT ON (r.eleve_id) r.eleve_id, r.motif
          FROM retraits r
         ORDER BY r.eleve_id, CASE r.motif
               WHEN 'fermeture' THEN 1 WHEN 'exclusion_internat' THEN 2 WHEN 'sante' THEN 3
               WHEN 'sejour_scolaire' THEN 4 WHEN 'stage' THEN 5 WHEN 'mobilite' THEN 6
               WHEN 'sortie' THEN 7 ELSE 8 END
    ),
    concerne AS (
        SELECT eleve_id, 'attendu' AS origine FROM du_rythme
        UNION
        SELECT eleve_id, 'ajoute' FROM ajoutes
        UNION
        SELECT eleve_id, 'non_attendu' FROM hebergees
    ),
    unique_eleve AS (
        SELECT DISTINCT ON (c.eleve_id) c.eleve_id, c.origine
          FROM concerne c
          JOIN public.ent_eleve el ON el.etablissement_id = etab AND el.id = c.eleve_id
         WHERE el.date_sortie IS NULL OR el.date_sortie > nuit
         ORDER BY c.eleve_id, CASE c.origine WHEN 'attendu' THEN 1 WHEN 'ajoute' THEN 2 ELSE 3 END
    )
    SELECT u.eleve_id, h.chambre_id, h.lit_id,
           CASE WHEN u.origine = 'non_attendu' THEN 'non_attendu'
                WHEN rr.motif IS NOT NULL THEN 'retire'
                ELSE u.origine END,
           CASE WHEN u.origine = 'non_attendu' THEN NULL ELSE rr.motif END
      FROM unique_eleve u
      LEFT JOIN hebergees h ON h.eleve_id = u.eleve_id
      LEFT JOIN retrait_retenu rr ON rr.eleve_id = u.eleve_id
$$;

-- ───────────────────────── M-3 : « jamais pour son propre enfant » pour TOUT pouvoir sur l'argent ─────────────────────────
-- D-069 (réponse du pilote) : la règle de 653 (demande, aide, report, issue) s'étend aux plans, aux versements (pointage ;
-- la remise de bordereau est contrôlée par l'application seule), aux frais réels, aux décisions d'avoir (individuelles et annulation collective) et aux acomptes
-- fournisseurs. Même fonction de décision que 653 (`ent_sortie_argent_decideur_interdit`), même code d'erreur (23514) ;
-- le contrôle de l'application donne le message, ce déclencheur interdit le contournement.
--   * plan, versement (et son annulation), frais réel (et son annulation) : la personne qui écrit n'est liée ni à
--     l'élève du participant ni payeuse du compte ;
--   * avoir : la personne qui DÉCIDE (retenir, écarter) ;
--   * acompte fournisseur : la sortie entière est concernée (aucun élève en particulier) : la personne qui crée ou paie
--     l'acompte n'est liée à AUCUN participant de la sortie. Limite documentée : l'annulation d'un acompte ne porte pas
--     l'identité de son auteur en base ; elle est contrôlée par l'application seule.
CREATE OR REPLACE FUNCTION ent_sortie_argent_pouvoir_libre() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    qui UUID;
    cpt UUID;
    pids UUID[];
    eleves UUID[];
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_plan' THEN
        qui := NEW.accorde_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_versement' THEN
        qui := NEW.saisi_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_frais_reel' THEN
        -- Pas de compte sur un frais : tous les payeurs du participant comptent.
        qui := NEW.saisi_par_user_id; cpt := NULL; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_avoir_propose' THEN
        -- Seule une décision humaine (retenir, écarter) exerce le pouvoir (l'écartement « signé système » n'a pas d'acteur).
        IF NEW.decide_par_user_id IS NULL OR NEW.statut NOT IN ('retenu', 'ecarte')
           OR (TG_OP = 'UPDATE' AND OLD.decide_par_user_id IS NOT DISTINCT FROM NEW.decide_par_user_id
               AND OLD.statut IS NOT DISTINCT FROM NEW.statut) THEN
            RETURN NEW;
        END IF;
        qui := NEW.decide_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSE
        -- Acompte fournisseur : création, puis paiement. La sortie entière est concernée.
        IF TG_OP = 'UPDATE' AND (NEW.paye_par_user_id IS NULL OR OLD.paye_par_user_id IS NOT NULL) THEN
            RETURN NEW;
        END IF;
        qui := COALESCE(NEW.paye_par_user_id, NEW.cree_par_user_id); cpt := NULL;
        SELECT array_agg(p.id) INTO pids FROM public.ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.sortie_id = NEW.sortie_id;
    END IF;
    pids := COALESCE(pids, ARRAY[]::uuid[]);
    SELECT array_agg(p.eleve_id) INTO eleves FROM public.ent_sortie_participant p
     WHERE p.etablissement_id = NEW.etablissement_id AND p.id = ANY(pids);
    eleves := COALESCE(eleves, ARRAY[]::uuid[]);
    IF public.ent_sortie_argent_decideur_interdit(NEW.etablissement_id, qui, cpt, eleves)
       OR (cpt IS NULL AND EXISTS (SELECT 1 FROM public.ent_sortie_part q
                                    WHERE q.etablissement_id = NEW.etablissement_id AND q.participant_id = ANY(pids)
                                      AND public.ent_sortie_argent_decideur_interdit(
                                            NEW.etablissement_id, qui, q.compte_id, ARRAY[]::uuid[]))) THEN
        RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas pour son propre enfant ni pour son propre compte'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_plan'::regclass AND tgname = 'ent_sortie_plan_pouvoir_tg') THEN
        CREATE TRIGGER ent_sortie_plan_pouvoir_tg BEFORE INSERT ON public.ent_sortie_plan
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_pouvoir_libre();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_versement'::regclass AND tgname = 'ent_sortie_versement_pouvoir_tg') THEN
        CREATE TRIGGER ent_sortie_versement_pouvoir_tg BEFORE INSERT ON public.ent_sortie_versement
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_pouvoir_libre();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_frais_reel'::regclass AND tgname = 'ent_sortie_frais_reel_pouvoir_tg') THEN
        CREATE TRIGGER ent_sortie_frais_reel_pouvoir_tg BEFORE INSERT ON public.ent_sortie_frais_reel
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_pouvoir_libre();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_avoir_propose'::regclass AND tgname = 'ent_sortie_avoir_propose_pouvoir_tg') THEN
        CREATE TRIGGER ent_sortie_avoir_propose_pouvoir_tg BEFORE INSERT OR UPDATE ON public.ent_sortie_avoir_propose
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_pouvoir_libre();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_acompte_fournisseur'::regclass AND tgname = 'ent_sortie_acompte_pouvoir_tg') THEN
        CREATE TRIGGER ent_sortie_acompte_pouvoir_tg BEFORE INSERT OR UPDATE ON public.ent_sortie_acompte_fournisseur
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_pouvoir_libre();
    END IF;
END
$$;

-- ───────────────────────── F-d : défense en profondeur de la base ─────────────────────────
-- (1) La fin d'un retrait de séjour vise un retrait de séjour posé : clé étrangère vers la table de lien (qui, elle, reste
--     sans clé vers `ent_internat_retrait`, voir 654).
-- (2) Le participant d'une demande, d'une aide ou d'un report est BIEN de la sortie visée : clé composite
--     (établissement, sortie, participant).
-- (3) Une aide ou un report issu d'une demande reprend exactement cette demande (nature, sortie, participant, payeur,
--     source, montant).
-- Les tables de 653 sont versées : on ajoute des contraintes et des déclencheurs, on ne les redéfinit pas.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie_participant'::regclass
                    AND conname = 'ent_sortie_participant_etab_sortie_id_uq') THEN
        ALTER TABLE public.ent_sortie_participant
            ADD CONSTRAINT ent_sortie_participant_etab_sortie_id_uq UNIQUE (etablissement_id, sortie_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie_internat_retrait_fin'::regclass
                    AND conname = 'ent_sortie_internat_retrait_fin_lien_fk') THEN
        ALTER TABLE public.ent_sortie_internat_retrait_fin
            ADD CONSTRAINT ent_sortie_internat_retrait_fin_lien_fk FOREIGN KEY (etablissement_id, retrait_id)
            REFERENCES public.ent_sortie_internat_retrait (etablissement_id, retrait_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie_argent_demande'::regclass
                    AND conname = 'ent_sortie_argent_demande_participant_sortie_fk') THEN
        ALTER TABLE public.ent_sortie_argent_demande
            ADD CONSTRAINT ent_sortie_argent_demande_participant_sortie_fk FOREIGN KEY (etablissement_id, sortie_id, participant_id)
            REFERENCES public.ent_sortie_participant (etablissement_id, sortie_id, id);
        ALTER TABLE public.ent_sortie_argent_demande
            ADD CONSTRAINT ent_sortie_argent_demande_vers_sortie_fk FOREIGN KEY (etablissement_id, sortie_id, vers_participant_id)
            REFERENCES public.ent_sortie_participant (etablissement_id, sortie_id, id);
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION ent_sortie_issue_coherente() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    d public.ent_sortie_argent_demande%ROWTYPE;
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_aide' THEN
        IF NEW.sens <> 'aide' THEN
            RETURN NEW;
        END IF;
        SELECT * INTO d FROM public.ent_sortie_argent_demande
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.demande_id;
        IF NOT FOUND OR d.nature <> 'aide' OR d.sortie_id <> NEW.sortie_id OR d.participant_id <> NEW.participant_id
           OR d.compte_id <> NEW.compte_id OR d.source IS DISTINCT FROM NEW.source
           OR d.montant_centimes <> NEW.montant_centimes THEN
            RAISE EXCEPTION 'une aide reprend exactement la demande qui la produit' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF NEW.sens <> 'report' THEN
            RETURN NEW;
        END IF;
        SELECT * INTO d FROM public.ent_sortie_argent_demande
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.demande_id;
        IF NOT FOUND OR d.nature <> 'report' OR d.sortie_id <> NEW.sortie_id OR d.participant_id <> NEW.de_participant_id
           OR d.vers_participant_id IS DISTINCT FROM NEW.vers_participant_id
           OR d.compte_id <> NEW.compte_id OR d.montant_centimes <> NEW.montant_centimes THEN
            RAISE EXCEPTION 'un report reprend exactement la demande qui le produit' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_aide'::regclass AND tgname = 'ent_sortie_aide_coherence_tg') THEN
        CREATE TRIGGER ent_sortie_aide_coherence_tg BEFORE INSERT ON public.ent_sortie_aide
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_issue_coherente();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_report'::regclass AND tgname = 'ent_sortie_report_coherence_tg') THEN
        CREATE TRIGGER ent_sortie_report_coherence_tg BEFORE INSERT ON public.ent_sortie_report
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_issue_coherente();
    END IF;
END
$$;

-- ───────────────────────── F-c : les refus de la règle 3 laissent une trace ─────────────────────────
-- Un refus (« pas pour son propre enfant ») annule la transaction de l'écriture : sa trace s'écrit donc dans sa propre
-- transaction, dans cette table en ajout seul. Aucun nom d'élève, aucun montant, aucun motif : qui, quelle action, quelle
-- sortie, quand. Jamais dans le journal de l'organisateur (il n'a pas à savoir qu'un pouvoir sur l'argent a été refusé).
CREATE TABLE IF NOT EXISTS ent_sortie_argent_refus (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID,
    user_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 60),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_argent_refus_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_argent_refus_idx ON ent_sortie_argent_refus (etablissement_id, sortie_id, le);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_argent_refus'::regclass
                    AND tgname = 'ent_sortie_argent_refus_immuable') THEN
        CREATE TRIGGER ent_sortie_argent_refus_immuable BEFORE UPDATE OR DELETE ON public.ent_sortie_argent_refus
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_argent_refus'::regclass
                    AND tgname = 'ent_sortie_argent_refus_vidage') THEN
        CREATE TRIGGER ent_sortie_argent_refus_vidage BEFORE TRUNCATE ON public.ent_sortie_argent_refus
            FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
    END IF;
END
$$;

SELECT ent_rls_poser(ARRAY['ent_sortie_argent_refus']);
