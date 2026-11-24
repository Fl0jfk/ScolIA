-- 656 — Socle 42, lot V3b « suites » (contre-revue des suites de V3, D-069) : Moyens 2 et 3, Faibles 1, 2, 5, 6, 7.
-- Réf. : docs/pilotage/revues/42_VOYAGES_V3_SUITES_CONTRE_REVUE.md ; docs/socles/42_VOYAGES_V3_LIVRAISON.md (« Suites V3b »).
--
-- Additive : 640–655 sont versées et ne changent pas (règle 25). Règle 38 : cette migration est en FIN de tableau ; chaque
-- fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et des relations qualifiées `public.`,
-- sauf `ent_internat_attendu_calcule` (famille « qualifiée » : SQL inlinée, relations `public.`, aucune clause `SET`).
-- Rejouable : colonnes, contraintes, déclencheurs et tables ne sont posés que s'ils manquent.

-- ───────────────────────── Moyen 2 : l'instant de passage en « rentrée » a sa propre colonne ─────────────────────────
-- Un participant resté « parti » sur une sortie rentrée ne perd son retrait d'internat qu'à partir du jour de ce passage ;
-- les nuits déjà passées dehors restent retirées. Même mécanique que `annulee_le` (655) : posé à la transition, immuable.
ALTER TABLE public.ent_sortie ADD COLUMN IF NOT EXISTS rentree_le TIMESTAMPTZ;

-- Rattrapage, établissement par établissement : sous un rôle soumis à la politique stricte (docs/RLS_ISOLATION.md §2),
-- un UPDATE sans variable d'établissement ne verrait aucune ligne. On pose donc la variable pour chacun (le réglage vaut
-- pour la transaction de la migration et est remis à vide ensuite). Faible 5 : le même rattrapage est rejoué pour
-- `annulee_le` (655), idempotent. Un cas non rattrapé ne passerait de toute façon pas en silence : la contrainte
-- `ent_sortie_annulee_le_chk` de 655 est validée sur TOUTES les lignes, sans tenir compte de la RLS.
DO $$
DECLARE
    e UUID;
BEGIN
    FOR e IN SELECT DISTINCT etablissement_id FROM public.ent_sortie
              WHERE (etat = 'rentree' AND rentree_le IS NULL) OR (etat = 'annulee' AND annulee_le IS NULL)
    LOOP
        PERFORM set_config('app.etablissement_id', e::text, true);
        UPDATE public.ent_sortie SET rentree_le = modifie_le WHERE etablissement_id = e AND etat = 'rentree' AND rentree_le IS NULL;
        UPDATE public.ent_sortie SET annulee_le = modifie_le WHERE etablissement_id = e AND etat = 'annulee' AND annulee_le IS NULL;
    END LOOP;
    PERFORM set_config('app.etablissement_id', '', true);
END
$$;

-- Posée à la transition vers « rentrée » (instant fourni par l'application, sinon l'heure de la base) ; immuable tant que
-- la sortie reste rentrée ou close ; vide pour toute autre étape (une réouverture efface l'instant).
CREATE OR REPLACE FUNCTION ent_sortie_rentree_le_pose() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF NEW.etat = 'rentree' THEN
        IF TG_OP = 'UPDATE' AND OLD.etat IN ('rentree', 'close') AND OLD.rentree_le IS NOT NULL THEN
            NEW.rentree_le := OLD.rentree_le;
        ELSE
            NEW.rentree_le := COALESCE(NEW.rentree_le, now());
        END IF;
    ELSIF NEW.etat = 'close' THEN
        IF TG_OP = 'UPDATE' AND OLD.etat IN ('rentree', 'close') THEN
            NEW.rentree_le := OLD.rentree_le;
        ELSE
            NEW.rentree_le := NULL;
        END IF;
    ELSE
        NEW.rentree_le := NULL;
    END IF;
    RETURN NEW;
END
$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie'::regclass
                    AND tgname = 'ent_sortie_rentree_le_tg') THEN
        CREATE TRIGGER ent_sortie_rentree_le_tg BEFORE INSERT OR UPDATE ON public.ent_sortie
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_rentree_le_pose();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie'::regclass
                    AND conname = 'ent_sortie_rentree_le_chk') THEN
        ALTER TABLE public.ent_sortie ADD CONSTRAINT ent_sortie_rentree_le_chk
            CHECK ((etat <> 'rentree' OR rentree_le IS NOT NULL) AND (rentree_le IS NULL OR etat IN ('rentree', 'close')));
    END IF;
END
$$;

-- ───────────────────────── Faible 7 : `ent_sortie_retrait_effectif` qualifiée `public.` ─────────────────────────
-- Corps identique à 654 ; relations qualifiées ; porte son `search_path` (règle 38). La fonction ne lit pas l'état de la
-- sortie : le retrait lui-même porte la dernière nuit retirée (la synchronisation la borne à la veille de `rentree_le`).
CREATE OR REPLACE FUNCTION ent_sortie_retrait_effectif(etab UUID, retrait UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM public.ent_sortie_internat_retrait l
          JOIN public.ent_internat_retrait r ON r.etablissement_id = l.etablissement_id AND r.id = l.retrait_id
          JOIN public.ent_sortie_participant p ON p.etablissement_id = l.etablissement_id AND p.id = l.participant_id
         WHERE l.etablissement_id = etab AND l.retrait_id = retrait
           AND (p.etat IN ('parti', 'remis', 'parti_seul_autorise')
                OR (p.etat = 'rentre'
                    AND r.au_nuit < public.ent_aujourdhui_etab(etab, COALESCE(
                          (SELECT max(f.a) FROM public.ent_sortie_participant_fait f
                            WHERE f.etablissement_id = p.etablissement_id AND f.participant_id = p.id
                              AND f.etat_apres = 'rentre'), p.modifie_le)))))
$$;

-- ───────────────────────── Faible 1 : une seule source de vérité, sans branche directe ─────────────────────────
-- La branche « séjour scolaire » directe de 655 était redondante avec le CTE `retraits` (même condition de retrait
-- effectif) ET plus large que lui : son `EXISTS` acceptait un lien terminé (retrait annulé par la synchronisation). Elle est
-- SUPPRIMÉE : l'attendu ne retire un élève pour un voyage que par une ligne `ent_internat_retrait` active et effective.
-- Corps identique à 655 pour tout le reste.
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

-- ───────────────────────── Moyen 3 : contournement TRACÉ de la règle « sortie entière » ─────────────────────────
-- Acomptes fournisseurs (création, paiement, annulation) et annulation collective d'avoirs engagent la sortie entière :
-- la personne n'est liée à AUCUN participant (655). Si la seule personne habilitée l'est, la sortie ne doit pas rester
-- bloquée, mais pas d'exception silencieuse : elle DEMANDE un contournement (motif obligatoire) ; une SECONDE personne,
-- distincte, habilitée et libre de tout lien avec la sortie, l'AUTORISE (ou le refuse) ; la demandeuse l'UTILISE ensuite,
-- une seule fois, pour une seule écriture. Trois faits en ajout seul : demande, décision, utilisation (avec la cible).
-- Aucun nom d'élève, aucun montant dans ces tables.
CREATE TABLE IF NOT EXISTS ent_sortie_argent_contournement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('acompte_creer', 'acompte_payer', 'acompte_annuler', 'annulation_collective')),
    demande_par_user_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 500),
    demande_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_argent_contournement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_contournement_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_argent_contournement_idx
    ON ent_sortie_argent_contournement (etablissement_id, sortie_id, demande_le);

CREATE TABLE IF NOT EXISTS ent_sortie_argent_contournement_evt (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    contournement_id UUID NOT NULL,
    evenement TEXT NOT NULL CHECK (evenement IN ('autorise', 'refuse', 'utilise')),
    par_user_id UUID NOT NULL,
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    -- Pour « utilise » : l'écriture visée (l'acompte, ou la sortie pour l'annulation collective).
    cible_id UUID,
    le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_argent_contournement_evt_cible_chk CHECK ((evenement = 'utilise') = (cible_id IS NOT NULL)),
    CONSTRAINT ent_sortie_argent_contournement_evt_motif_chk CHECK (evenement <> 'refuse' OR motif IS NOT NULL),
    CONSTRAINT ent_sortie_argent_contournement_evt_fk FOREIGN KEY (etablissement_id, contournement_id)
        REFERENCES ent_sortie_argent_contournement (etablissement_id, id)
);
-- Une décision par demande ; une utilisation par demande.
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_argent_contournement_evt_decision_uq
    ON ent_sortie_argent_contournement_evt (etablissement_id, contournement_id) WHERE evenement IN ('autorise', 'refuse');
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_argent_contournement_evt_usage_uq
    ON ent_sortie_argent_contournement_evt (etablissement_id, contournement_id) WHERE evenement = 'utilise';

-- La personne n'est liée à aucun participant de la sortie et n'est payeuse d'aucun de ses comptes (même règle que
-- `exiger_libre_de_la_sortie`, côté base).
CREATE OR REPLACE FUNCTION ent_sortie_libre_de_la_sortie(etab UUID, sortie UUID, usr UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT NOT public.ent_sortie_argent_decideur_interdit(
                   etab, usr, NULL,
                   COALESCE((SELECT array_agg(p.eleve_id) FROM public.ent_sortie_participant p
                              WHERE p.etablissement_id = etab AND p.sortie_id = sortie), ARRAY[]::uuid[]))
       AND NOT EXISTS (SELECT 1 FROM public.ent_sortie_part q
                         JOIN public.ent_sortie_participant p ON p.etablissement_id = q.etablissement_id AND p.id = q.participant_id
                        WHERE q.etablissement_id = etab AND p.sortie_id = sortie
                          AND public.ent_sortie_argent_decideur_interdit(etab, usr, q.compte_id, ARRAY[]::uuid[]))
$$;

-- Garde des faits : la décision vient d'une AUTRE personne que la demandeuse, libre de la sortie ; l'utilisation vient de
-- la demandeuse, après une autorisation.
CREATE OR REPLACE FUNCTION ent_sortie_contournement_evt_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    c public.ent_sortie_argent_contournement%ROWTYPE;
BEGIN
    SELECT * INTO c FROM public.ent_sortie_argent_contournement
     WHERE etablissement_id = NEW.etablissement_id AND id = NEW.contournement_id;
    IF NEW.evenement IN ('autorise', 'refuse') THEN
        IF NEW.par_user_id = c.demande_par_user_id THEN
            RAISE EXCEPTION 'la décision d''un contournement vient d''une autre personne que la demandeuse' USING ERRCODE = '23514';
        END IF;
        IF NEW.evenement = 'autorise' AND NOT public.ent_sortie_libre_de_la_sortie(NEW.etablissement_id, c.sortie_id, NEW.par_user_id) THEN
            RAISE EXCEPTION 'la personne qui autorise un contournement est libre de tout lien avec la sortie' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF NEW.par_user_id <> c.demande_par_user_id THEN
            RAISE EXCEPTION 'seule la demandeuse utilise son contournement' USING ERRCODE = '23514';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM public.ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = NEW.etablissement_id AND e.contournement_id = NEW.contournement_id
                          AND e.evenement = 'autorise') THEN
            RAISE EXCEPTION 'un contournement non autorisé ne s''utilise pas' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

-- Un contournement valable : demandé par `usr` pour cette action sur cette sortie, autorisé par une AUTRE personne
-- (encore libre de la sortie), utilisé pour CETTE écriture.
CREATE OR REPLACE FUNCTION ent_sortie_contournement_valide(etab UUID, sortie UUID, usr UUID, act TEXT, cible UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM public.ent_sortie_argent_contournement c
          JOIN public.ent_sortie_argent_contournement_evt ok
            ON ok.etablissement_id = c.etablissement_id AND ok.contournement_id = c.id AND ok.evenement = 'autorise'
          JOIN public.ent_sortie_argent_contournement_evt us
            ON us.etablissement_id = c.etablissement_id AND us.contournement_id = c.id AND us.evenement = 'utilise'
         WHERE c.etablissement_id = etab AND c.sortie_id = sortie AND c.action = act AND c.demande_par_user_id = usr
           AND ok.par_user_id <> usr AND us.cible_id = cible
           AND public.ent_sortie_libre_de_la_sortie(etab, sortie, ok.par_user_id))
$$;

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_sortie_argent_contournement', 'ent_sortie_argent_contournement_evt'] LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = ('public.' || t)::regclass AND tgname = t || '_immuable') THEN
            EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON public.%I FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul()',
                           t || '_immuable', t);
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = ('public.' || t)::regclass AND tgname = t || '_vidage') THEN
            EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON public.%I FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul()',
                           t || '_vidage', t);
        END IF;
    END LOOP;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.ent_sortie_argent_contournement_evt'::regclass
                    AND tgname = 'ent_sortie_argent_contournement_evt_garde_tg') THEN
        CREATE TRIGGER ent_sortie_argent_contournement_evt_garde_tg BEFORE INSERT ON public.ent_sortie_argent_contournement_evt
            FOR EACH ROW EXECUTE FUNCTION ent_sortie_contournement_evt_garde();
    END IF;
END
$$;
SELECT ent_rls_poser(ARRAY['ent_sortie_argent_contournement', 'ent_sortie_argent_contournement_evt']);

-- ───────────────────────── Faible 6 : l'annulation d'un acompte garde son auteur ─────────────────────────
-- NOT VALID : les acomptes déjà annulés n'ont pas d'auteur connu (jamais inventé) ; la contrainte vaut pour toute écriture
-- nouvelle ou modifiée. Le déclencheur d'acompte (ci-dessous) garde désormais aussi l'annulation côté base.
ALTER TABLE public.ent_sortie_acompte_fournisseur ADD COLUMN IF NOT EXISTS annule_par_user_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'public.ent_sortie_acompte_fournisseur'::regclass
                    AND conname = 'ent_sortie_acompte_fournisseur_annule_par_chk') THEN
        ALTER TABLE public.ent_sortie_acompte_fournisseur ADD CONSTRAINT ent_sortie_acompte_fournisseur_annule_par_chk
            CHECK ((annule_le IS NULL) = (annule_par_user_id IS NULL)) NOT VALID;
    END IF;
END
$$;

-- La garde d'immutabilité d'un acompte (651, versée) refusait toute colonne hors alerte/paiement/annulation : l'auteur de
-- l'annulation rejoint les colonnes libres, et se fige comme le motif une fois posé. Corps identique à 651 hors cela.
CREATE OR REPLACE FUNCTION ent_sortie_acompte_fournisseur_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    libres TEXT[] := ARRAY['alerte_le', 'paye_le', 'paye_centimes', 'paye_reference', 'paye_par_user_id',
                           'annule_le', 'annule_motif', 'annule_par_user_id'];
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'un acompte fournisseur ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF (to_jsonb(NEW) - libres) IS DISTINCT FROM (to_jsonb(OLD) - libres)
       OR (OLD.alerte_le IS NOT NULL AND NEW.alerte_le IS DISTINCT FROM OLD.alerte_le)
       OR (OLD.paye_le IS NOT NULL AND (NEW.paye_le IS DISTINCT FROM OLD.paye_le
                                        OR NEW.paye_centimes IS DISTINCT FROM OLD.paye_centimes
                                        OR NEW.paye_reference IS DISTINCT FROM OLD.paye_reference
                                        OR NEW.paye_par_user_id IS DISTINCT FROM OLD.paye_par_user_id))
       OR (OLD.annule_le IS NOT NULL AND (NEW.annule_le IS DISTINCT FROM OLD.annule_le
                                          OR NEW.annule_motif IS DISTINCT FROM OLD.annule_motif
                                          OR NEW.annule_par_user_id IS DISTINCT FROM OLD.annule_par_user_id)) THEN
        RAISE EXCEPTION 'un acompte fournisseur ne change que par son alerte, son paiement ou son annulation'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;

-- `ent_sortie_argent_pouvoir_libre` (655) reprise à l'identique, avec deux ajouts : (1) l'annulation d'un acompte est
-- gardée (auteur en base) ; (2) un acompte (création, paiement, annulation) d'une personne liée à la sortie passe si un
-- contournement autorisé par une AUTRE personne libre a été utilisé pour CETTE écriture.
CREATE OR REPLACE FUNCTION ent_sortie_argent_pouvoir_libre() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    qui UUID;
    cpt UUID;
    pids UUID[];
    eleves UUID[];
    act TEXT;
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_plan' THEN
        qui := NEW.accorde_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_versement' THEN
        qui := NEW.saisi_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_frais_reel' THEN
        qui := NEW.saisi_par_user_id; cpt := NULL; pids := ARRAY[NEW.participant_id];
    ELSIF TG_TABLE_NAME = 'ent_sortie_avoir_propose' THEN
        IF NEW.decide_par_user_id IS NULL OR NEW.statut NOT IN ('retenu', 'ecarte')
           OR (TG_OP = 'UPDATE' AND OLD.decide_par_user_id IS NOT DISTINCT FROM NEW.decide_par_user_id
               AND OLD.statut IS NOT DISTINCT FROM NEW.statut) THEN
            RETURN NEW;
        END IF;
        qui := NEW.decide_par_user_id; cpt := NEW.compte_id; pids := ARRAY[NEW.participant_id];
    ELSE
        -- Acompte fournisseur : création, paiement, annulation. La sortie entière est concernée.
        IF TG_OP = 'INSERT' THEN
            qui := COALESCE(NEW.paye_par_user_id, NEW.cree_par_user_id); act := 'acompte_creer';
        ELSIF NEW.paye_par_user_id IS NOT NULL AND OLD.paye_par_user_id IS NULL THEN
            qui := NEW.paye_par_user_id; act := 'acompte_payer';
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
       OR (cpt IS NULL AND EXISTS (SELECT 1 FROM public.ent_sortie_part q
                                    WHERE q.etablissement_id = NEW.etablissement_id AND q.participant_id = ANY(pids)
                                      AND public.ent_sortie_argent_decideur_interdit(
                                            NEW.etablissement_id, qui, q.compte_id, ARRAY[]::uuid[]))) THEN
        IF act IS NOT NULL AND public.ent_sortie_contournement_valide(NEW.etablissement_id, NEW.sortie_id, qui, act, NEW.id) THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas pour son propre enfant ni pour son propre compte'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

-- Faible 4 : plafond de débit des traces de refus (ajout seul, jamais purgé) : au plus N lignes par personne et par heure,
-- appliqué par l'application (voir `tracer_refus`) ; l'index sert ce comptage.
CREATE INDEX IF NOT EXISTS ent_sortie_argent_refus_user_idx ON ent_sortie_argent_refus (etablissement_id, user_id, le);
