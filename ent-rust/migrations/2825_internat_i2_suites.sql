-- Socle 41, lot I2 — suites de la contre-revue de sécurité du pilote (2824 est versée : on ne la modifie plus,
-- règle 25 ; toute correction passe par cette migration, rejouable : CREATE OR REPLACE, IF NOT EXISTS).
--
--  1. L7 inversé : `ent_eleve.date_sortie` est le PREMIER jour d'absence (période `[entrée, sortie[`, migration 283).
--     Un élève dont la date de sortie est le 28 est attendu la nuit du 27, plus celle du 28. Le filtre
--     `date_sortie >= nuit` le gardait « attendu » après son départ : fausse alerte « enfant disparu ».
--  2. M9 : un participant `remis` (remis à sa famille) ou `parti_seul_autorise` d'une sortie en cours n'est pas à
--     l'internat : il sort de l'attendu comme `pret` et `parti`. `absent_depart` (n'est pas parti) et `rentre`
--     restent attendus.
--  3. Note de clôture d'un cas : colonne `issue_note` (jusqu'à 300 caractères), figée avec l'issue.
--  4. Transmission : `eleves_partis`, pour que l'idempotence compare l'ensemble des partis et non un seul compteur.

-- ───────────────────────── Attendu de la nuit (L7, M9) ─────────────────────────
CREATE OR REPLACE FUNCTION ent_internat_attendu_calcule(etab UUID, nuit DATE)
RETURNS TABLE (eleve_id UUID, chambre_id UUID, lit_id UUID, statut TEXT, motif_code TEXT)
LANGUAGE sql STABLE AS $$
    WITH ancre AS (
        SELECT ent_internat_ancre_nuit(etab, nuit) AS t, ent_internat_fuseau(etab) AS tz
    ),
    du_rythme AS (
        SELECT s.eleve_id
          FROM ent_internat_sejour s
         WHERE s.etablissement_id = etab AND s.du <= nuit AND (s.au IS NULL OR s.au >= nuit)
           AND COALESCE((
                SELECT CASE extract(dow FROM nuit)::int
                         WHEN 0 THEN r.dimanche WHEN 1 THEN r.lundi WHEN 2 THEN r.mardi
                         WHEN 3 THEN r.mercredi WHEN 4 THEN r.jeudi WHEN 5 THEN r.vendredi
                         ELSE r.samedi END
                  FROM ent_internat_rythme r
                 WHERE r.etablissement_id = etab AND r.sejour_id = s.id AND r.du <= nuit
                 ORDER BY r.du DESC LIMIT 1), false)
    ),
    ajoutes AS (
        SELECT x.eleve_id FROM ent_internat_nuit_exceptionnelle x
         WHERE x.etablissement_id = etab AND x.nuit = ent_internat_attendu_calcule.nuit AND x.annulee_le IS NULL
    ),
    hebergees AS (
        SELECT DISTINCT ON (a.eleve_id) a.eleve_id, a.chambre_id, a.lit_id
          FROM ent_internat_affectation a
         WHERE a.etablissement_id = etab AND a.debut <= nuit AND (a.fin IS NULL OR a.fin >= nuit)
         ORDER BY a.eleve_id, a.debut DESC
    ),
    retraits AS (
        SELECT r.eleve_id, r.motif_code AS motif FROM ent_internat_retrait r
         WHERE r.etablissement_id = etab AND r.annule_le IS NULL AND r.du_nuit <= nuit AND r.au_nuit >= nuit
        UNION ALL
        SELECT sit.eleve_id,
               CASE sit.nature WHEN 'declaration' THEN 'absence_annoncee' WHEN 'sortie' THEN 'sortie'
                               WHEN 'mobilite' THEN 'mobilite' ELSE 'stage' END
          FROM ent_asd_situation sit, ancre
         WHERE sit.etablissement_id = etab AND sit.etat = 'active'
           AND sit.nature IN ('declaration', 'sortie', 'mobilite', 'pfmp')
           AND ((sit.debut_jour + COALESCE(sit.debut_heure, TIME '00:00')) AT TIME ZONE ancre.tz) <= ancre.t
           AND ancre.t < ((sit.fin_jour + COALESCE(sit.fin_heure, TIME '23:59:59')) AT TIME ZONE ancre.tz)
        UNION ALL
        SELECT m.eleve_id, 'mobilite' FROM ent_eleve_mobilite m
         WHERE m.etablissement_id = etab AND m.ferme_le IS NULL AND m.du <= nuit AND (m.au IS NULL OR m.au >= nuit)
        UNION ALL
        SELECT p.eleve_id, 'sejour_scolaire'
          FROM ent_sortie so
          JOIN ent_sortie_participant p ON p.etablissement_id = so.etablissement_id AND p.sortie_id = so.id
          CROSS JOIN ancre
         WHERE so.etablissement_id = etab AND so.etat IN ('prete', 'en_cours')
           AND p.etat IN ('pret', 'parti', 'remis', 'parti_seul_autorise')
           AND so.debut <= ancre.t AND ancre.t < so.fin
        UNION ALL
        SELECT d.eleve_id, 'fermeture'
          FROM (SELECT eleve_id FROM du_rythme UNION SELECT eleve_id FROM ajoutes) d
         WHERE EXISTS (
                SELECT 1 FROM ent_calendrier_evenement ev
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
          JOIN ent_eleve el ON el.etablissement_id = etab AND el.id = c.eleve_id
         -- L7 (corrigé) : `date_sortie` est le PREMIER jour d'absence (période `[entrée, sortie[`,
         -- migration 283) : l'élève est encore attendu la nuit qui précède, plus à partir de ce jour.
         WHERE el.date_sortie IS NULL OR el.date_sortie > nuit
         ORDER BY c.eleve_id, CASE c.origine WHEN 'attendu' THEN 1 WHEN 'ajoute' THEN 2 ELSE 3 END
    )
    SELECT u.eleve_id, h.chambre_id, h.lit_id,
           -- E1 : un hébergé hors rythme reste « non_attendu » même couvert par un retrait (sinon
           -- « retire » sans motif viole la contrainte et l'appel ne s'ouvre plus).
           CASE WHEN u.origine = 'non_attendu' THEN 'non_attendu'
                WHEN rr.motif IS NOT NULL THEN 'retire'
                ELSE u.origine END,
           CASE WHEN u.origine = 'non_attendu' THEN NULL ELSE rr.motif END
      FROM unique_eleve u
      LEFT JOIN hebergees h ON h.eleve_id = u.eleve_id
      LEFT JOIN retrait_retenu rr ON rr.eleve_id = u.eleve_id
$$;

-- ───────────────────────── Note de clôture d'un cas ─────────────────────────
ALTER TABLE ent_internat_cas ADD COLUMN IF NOT EXISTS issue_note TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_cas_issue_note_chk') THEN
        ALTER TABLE ent_internat_cas ADD CONSTRAINT ent_internat_cas_issue_note_chk
            CHECK (issue_note IS NULL OR (issue IS NOT NULL AND char_length(btrim(issue_note)) BETWEEN 1 AND 300));
    END IF;
END
$$;

-- ───────────────────────── Transmission : ensemble des partis ─────────────────────────
-- (la table reste en ajout seul : ADD COLUMN ne déclenche pas la garde de modification.)
ALTER TABLE ent_internat_transmission
    ADD COLUMN IF NOT EXISTS eleves_partis UUID[] NOT NULL DEFAULT ARRAY[]::uuid[];

-- La garde du cas fige aussi la note de clôture.
CREATE OR REPLACE FUNCTION ent_internat_cas_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_cas : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF OLD.issue IS NOT NULL AND (OLD.issue, OLD.issue_le, OLD.issue_par_user_id, OLD.eleve_id, OLD.nuit,
                                  OLD.ouvert_le, OLD.nature, OLD.issue_lieu_code,
                                  OLD.issue_confirmee_par_personne_id, OLD.numero_intervention, OLD.appel_id, OLD.issue_note)
       IS DISTINCT FROM (NEW.issue, NEW.issue_le, NEW.issue_par_user_id, NEW.eleve_id, NEW.nuit,
                         NEW.ouvert_le, NEW.nature, NEW.issue_lieu_code,
                         NEW.issue_confirmee_par_personne_id, NEW.numero_intervention, NEW.appel_id, NEW.issue_note) THEN
        RAISE EXCEPTION 'un cas clos ne se modifie pas (R-IT18) : seule sa transmission s''ajoute'
            USING ERRCODE = '42501';
    END IF;
    IF (OLD.eleve_id, OLD.nuit, OLD.ouvert_le, OLD.procedure_version_id)
       IS DISTINCT FROM (NEW.eleve_id, NEW.nuit, NEW.ouvert_le, NEW.procedure_version_id) THEN
        RAISE EXCEPTION 'un cas ne change ni d''élève, ni de nuit, ni de procédure' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
