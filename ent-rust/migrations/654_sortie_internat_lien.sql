-- 654 — Socle 42, lot V3 « argent relié » (2/2) : lien entre un séjour et l'internat (socle 41). Un interne qui part en
-- voyage ne dort pas à l'internat : l'attendu de la nuit (I2) doit le savoir sans que personne ne le ressaisisse, et
-- l'inverse quand le participant se désiste. Réf. : 42_SORTIES_VOYAGES.md §6.2, §6.4 ; 41_INTERNAT.md (adaptateur
-- `ent_internat_retrait`, motif « sejour_scolaire »).
--
-- Additive : 2824 (versée) n'est pas modifiée. On ajoute seulement la source « voyage » à la liste des sources
-- autorisées d'un retrait, par un remplacement de la contrainte (même nom, liste élargie) ; rien d'autre ne change
-- dans les tables du socle 41. La trace du retrait posé pour un participant vit dans le socle 42 (ajout seul).

-- Rejeu : la contrainte élargie porte un nom propre ; si elle existe, on ne retouche rien (aucun DROP/ADD à chaque
-- rejeu, F3 de la revue du pilote) ; sinon on remplace d'abord l'ancienne contrainte de 2824 (celle de 2826 n'y touche pas).
DO $$
DECLARE
    c TEXT;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = 'ent_internat_retrait'::regclass AND conname = 'ent_internat_retrait_source_v3_chk') THEN
        FOR c IN SELECT conname FROM pg_constraint
                  WHERE conrelid = 'ent_internat_retrait'::regclass AND contype = 'c'
                    AND pg_get_constraintdef(oid) LIKE '%absence_internat%' LOOP
            EXECUTE format('ALTER TABLE ent_internat_retrait DROP CONSTRAINT %I', c);
        END LOOP;
        ALTER TABLE ent_internat_retrait ADD CONSTRAINT ent_internat_retrait_source_v3_chk CHECK (source IN (
            'cpe', 'direction', 'vie_scolaire', 'infirmerie', 'absence_internat', 'reprise', 'voyage'));
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS ent_sortie_internat_retrait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    retrait_id UUID NOT NULL,
    du_nuit DATE NOT NULL,
    au_nuit DATE NOT NULL,
    cree_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_internat_retrait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_internat_retrait_periode_chk CHECK (au_nuit >= du_nuit),
    CONSTRAINT ent_sortie_internat_retrait_retrait_uq UNIQUE (etablissement_id, retrait_id),
    CONSTRAINT ent_sortie_internat_retrait_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_internat_retrait_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id)
    -- Volontairement SANS clé étrangère vers `ent_internat_retrait` : le socle 41 garde la main sur sa table (son
    -- TRUNCATE est éprouvé par ses propres tests) ; le lien est écrit dans la même transaction que le retrait.
);
CREATE INDEX IF NOT EXISTS ent_sortie_internat_retrait_part_idx
    ON ent_sortie_internat_retrait (etablissement_id, sortie_id, participant_id);

DROP TRIGGER IF EXISTS ent_sortie_internat_retrait_immuable ON ent_sortie_internat_retrait;
CREATE TRIGGER ent_sortie_internat_retrait_immuable BEFORE UPDATE OR DELETE ON ent_sortie_internat_retrait
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_internat_retrait_vidage ON ent_sortie_internat_retrait;
CREATE TRIGGER ent_sortie_internat_retrait_vidage BEFORE TRUNCATE ON ent_sortie_internat_retrait
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Fin d'un retrait de voyage PAR LA SYNCHRONISATION (dates changées, participant retiré, sortie annulée, retour
-- anticipé). Un retrait « voyage » annulé SANS cette trace l'a été par le CPE : l'annulation est mémorisée et
-- respectée (M2) — le participant n'est pas retiré de nouveau, jamais en silence. Ajout seul.
CREATE TABLE IF NOT EXISTS ent_sortie_internat_retrait_fin (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    retrait_id UUID NOT NULL,
    cause TEXT NOT NULL CHECK (cause IN ('dates', 'participant', 'sortie', 'retour_anticipe', 'perime')),
    par_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_internat_retrait_fin_retrait_uq UNIQUE (etablissement_id, retrait_id),
    CONSTRAINT ent_sortie_internat_retrait_fin_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_sortie_internat_retrait_fin_immuable ON ent_sortie_internat_retrait_fin;
CREATE TRIGGER ent_sortie_internat_retrait_fin_immuable BEFORE UPDATE OR DELETE ON ent_sortie_internat_retrait_fin
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_internat_retrait_fin_vidage ON ent_sortie_internat_retrait_fin;
CREATE TRIGGER ent_sortie_internat_retrait_fin_vidage BEFORE TRUNCATE ON ent_sortie_internat_retrait_fin
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Un retrait de voyage est-il en vigueur ? (D-069, règle 1 : pas de départ constaté = élève attendu.)
-- Oui quand le participant lié est PARTI (ou remis à sa famille / parti seul avec autorisation). Un participant
-- « rentré » (retour anticipé) ne reste retiré que pour les nuits QUI PRÉCÈDENT son retour ; « prêt », « inscrit »,
-- « bloqué », « absent au départ » ou désisté : jamais retiré, même si une ligne subsiste.
CREATE OR REPLACE FUNCTION ent_sortie_retrait_effectif(etab UUID, retrait UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ent_sortie_internat_retrait l
          JOIN ent_internat_retrait r ON r.etablissement_id = l.etablissement_id AND r.id = l.retrait_id
          JOIN ent_sortie_participant p ON p.etablissement_id = l.etablissement_id AND p.id = l.participant_id
         WHERE l.etablissement_id = etab AND l.retrait_id = retrait
           AND (p.etat IN ('parti', 'remis', 'parti_seul_autorise')
                OR (p.etat = 'rentre'
                    AND r.au_nuit < ent_aujourdhui_etab(etab, COALESCE(
                          (SELECT max(f.a) FROM ent_sortie_participant_fait f
                            WHERE f.etablissement_id = p.etablissement_id AND f.participant_id = p.id
                              AND f.etat_apres = 'rentre'), p.modifie_le)))))
$$;

-- L'attendu de l'internat suit l'état réel du voyage (D-069, règles 1 et 2). 2826 est versée : la fonction est
-- remplacée ici (CREATE OR REPLACE), reprise à l'identique sauf deux conditions commentées « 654 ».
CREATE OR REPLACE FUNCTION ent_internat_attendu_calcule(etab UUID, nuit DATE)
RETURNS TABLE (eleve_id UUID, chambre_id UUID, lit_id UUID, statut TEXT, motif_code TEXT)
LANGUAGE sql STABLE AS $$
    WITH ancre AS (
        SELECT ent_internat_ancre_nuit(etab, nuit) AS t, ent_internat_fuseau(etab) AS tz,
               (((nuit + 1) + TIME '06:00') AT TIME ZONE ent_internat_fuseau(etab)) AS matin
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
           -- 654 (D-069, règle 1) : un retrait posé pour un voyage ne vaut qu'à partir du DÉPART CONSTATÉ du participant.
           AND (r.source <> 'voyage' OR ent_sortie_retrait_effectif(etab, r.id))
        UNION ALL
        SELECT ab.eleve_id, 'absence_annoncee' FROM ent_internat_absence ab, ancre
         WHERE ab.etablissement_id = etab AND ab.du_nuit <= nuit AND ab.au_nuit >= nuit
           AND (ab.statut = 'regularisee'
                -- Passe 3 : une absence dont l'auteur ou la personne remise est devenu visé APRÈS le départ ouvre un cas
                -- pour le CPE ; tant qu'il n'a pas tranché (cas ouvert), les nuits SUIVANTES ne retirent plus l'élève.
                OR (NOT EXISTS (SELECT 1 FROM ent_internat_cas cx
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
        SELECT so.eleve_id, 'sortie' FROM ent_internat_sortie so, ancre
         WHERE so.etablissement_id = etab AND so.statut IN ('partie', 'rentree')
           AND NOT EXISTS (SELECT 1 FROM ent_internat_cas cx
                            WHERE cx.etablissement_id = etab AND cx.eleve_id = so.eleve_id
                              AND cx.nature = 'interdiction_auteur' AND cx.issue IS NULL
                              AND cx.nuit < ent_internat_attendu_calcule.nuit)
           AND so.debut <= ancre.t
           AND COALESCE(so.retour_effectif, so.fin_ajustee) >= ancre.matin
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
           -- 654 (D-069, règle 1) : « prêt » n'est pas « parti » ; seul le départ constaté retire l'élève de l'attendu.
           AND p.etat IN ('parti', 'remis', 'parti_seul_autorise')
           -- 654 (M2) : une annulation du CPE (retrait « voyage » annulé hors synchronisation) est mémorisée et respectée.
           AND NOT EXISTS (SELECT 1 FROM ent_sortie_internat_retrait l
                             JOIN ent_internat_retrait rl ON rl.etablissement_id = l.etablissement_id AND rl.id = l.retrait_id
                            WHERE l.etablissement_id = so.etablissement_id AND l.sortie_id = so.id
                              AND l.participant_id = p.id AND rl.annule_le IS NOT NULL
                              AND NOT EXISTS (SELECT 1 FROM ent_sortie_internat_retrait_fin fi
                                               WHERE fi.etablissement_id = l.etablissement_id AND fi.retrait_id = l.retrait_id))
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

SELECT ent_rls_poser(ARRAY['ent_sortie_internat_retrait', 'ent_sortie_internat_retrait_fin']);
