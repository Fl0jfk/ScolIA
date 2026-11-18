-- Socle 2 (l'élève), lot E9 (partie 1, suites d'E7-E8) : plage 3200-3219.
--
-- Additif et idempotent. N'altère AUCUNE migration déjà versée : les objets de 299 sont remplacés
-- par de nouvelles définitions (CREATE OR REPLACE, INSERT ... ON CONFLICT), jamais réécrits.
--
-- 1. Pastilles R-E9 : `protocole` (neutre, vie scolaire), `nouvel_arrivant`, `arrive_en_cours_annee`,
--    `majorite_proche` (un booléen, jamais la date), `modalite_particuliere`. La source de vérité est
--    `domain_eleve::CATALOGUE_INDICATEURS` (un test garantit l'égalité avec la table).
-- 2. F3 : `ent_eleve.statut` n'est plus jamais NULL (reprise des lignes existantes, remplissage à
--    l'insertion ; pas de NOT NULL, voir plus bas).
-- 3. F5 : validation des contraintes NOT VALID de 298 (et du statut), après assainissement.

-- Premier passage : la pastille « protocole » n'est pas encore au catalogue. La reprise des statuts, le
-- recalcul de la projection et la validation des contraintes ne se font QUE ce jour-là (garde idempotente :
-- un rejeu de la migration ne recalcule pas tous les établissements). Drapeau local à la transaction.
SELECT set_config('ent.e9_premier_passage',
                  (NOT EXISTS (SELECT 1 FROM ent_catalogue_indicateur WHERE code = 'protocole'))::text, true);

-- ───────────────────────── 1. Catalogue des pastilles ─────────────────────────
INSERT INTO ent_catalogue_indicateur (code, domaine, libelle, icone, gravite, actions_requises, depend_du_temps) VALUES
    ('protocole', 'classe', 'Protocole : voir l''infirmerie', 'croix', 'attention', ARRAY['contraintes.lire']::TEXT[], true),
    ('nouvel_arrivant', 'classe', 'Nouvel arrivant', 'etoile', 'information', ARRAY['eleves.lire', 'contraintes.lire']::TEXT[], true),
    ('arrive_en_cours_annee', 'classe', 'Arrivé en cours d''année', 'fleche', 'information', ARRAY['eleves.lire', 'contraintes.lire']::TEXT[], true),
    ('majorite_proche', 'classe', 'Majorité dans moins de 30 jours', 'calendrier', 'attention', ARRAY['eleves.etat_civil.lire']::TEXT[], true),
    ('modalite_particuliere', 'classe', 'Modalité de scolarisation particulière', 'reglage', 'information', ARRAY['eleves.lire', 'contraintes.lire']::TEXT[], true)
ON CONFLICT (code) DO UPDATE SET
    domaine = EXCLUDED.domaine, libelle = EXCLUDED.libelle, icone = EXCLUDED.icone,
    gravite = EXCLUDED.gravite, actions_requises = EXCLUDED.actions_requises,
    depend_du_temps = EXCLUDED.depend_du_temps;

-- Pastilles attendues (remplace la définition de 299, mêmes colonnes, cinq pastilles de plus).
CREATE OR REPLACE FUNCTION ent_eleve_indicateurs_attendus(p_etab UUID, p_jour DATE, p_eleve UUID DEFAULT NULL)
RETURNS TABLE (eleve_id UUID, code TEXT, valeur_code TEXT, valeur_date DATE, du DATE)
LANGUAGE sql STABLE AS $$
    WITH j AS (SELECT coalesce(p_jour, ent_jour_etablissement(p_etab)) AS jour),
    annee AS (SELECT a.date_debut, a.date_fin FROM ent_annee_scolaire a
               WHERE a.etablissement_id = p_etab AND a.courante LIMIT 1),
    e AS (
        SELECT e.id, e.regime, e.redoublant, e.date_naissance, e.date_naissance_precision,
               e.date_naissance_statut, e.statut
          FROM ent_eleve e
         WHERE e.etablissement_id = p_etab AND e.fusionne_dans IS NULL AND (p_eleve IS NULL OR e.id = p_eleve)
    ),
    -- NULL = aucune scolarisation encore (ligne reprise) : traité comme « à venir », jamais comme « actif ».
    scolarises AS (SELECT * FROM e WHERE coalesce(e.statut, 'a_venir') IN ('scolarise', 'a_venir')),
    -- scolarisation en cours (la plus récente) : base de « nouvel arrivant » et « en cours d'année »
    en_cours AS (
        SELECT DISTINCT ON (s.eleve_id) s.eleve_id, s.date_entree, s.motif_entree
          FROM ent_eleve_scolarisation s
         WHERE s.etablissement_id = p_etab AND (p_eleve IS NULL OR s.eleve_id = p_eleve)
           AND s.date_entree <= (SELECT jour FROM j)
           AND (s.date_sortie IS NULL OR s.date_sortie > (SELECT jour FROM j))
         ORDER BY s.eleve_id, s.date_entree DESC
    )
    -- interne : régime « interne » (seul régime hébergé, R-IT1)
    SELECT s.id, 'interne'::text, s.regime, NULL::date,
           coalesce((SELECT p.date_debut FROM ent_eleve_periode p
                      WHERE p.etablissement_id = p_etab AND p.eleve_id = s.id AND p.kind = 'regime'
                        AND p.date_fin IS NULL), (SELECT jour FROM j))
      FROM scolarises s WHERE s.regime = 'interne'
    UNION ALL
    -- pai_actif : PAI actif et en cours (échéance = date de fin)
    SELECT s.id, 'pai_actif', NULL, p.date_fin, p.date_debut
      FROM scolarises s
      JOIN ent_eleve_pai_courant(p_etab, p_eleve, (SELECT jour FROM j)) p ON p.eleve_id = s.id
    UNION ALL
    -- protocole : un PAI en cours existe. NEUTRE : ni échéance, ni date, ni nature (la vie scolaire
    -- sait seulement qu'il faut s'adresser à l'infirmerie) ; `du` = jour du calcul.
    SELECT s.id, 'protocole', NULL, NULL, (SELECT jour FROM j)
      FROM scolarises s
     WHERE EXISTS (SELECT 1 FROM ent_eleve_pai_courant(p_etab, s.id, (SELECT jour FROM j)) p)
    UNION ALL
    -- sans_ine : aucun INE (déclaré, à vérifier, confirmé ou contesté) en vigueur
    SELECT s.id, 'sans_ine', NULL, NULL, (SELECT jour FROM j)
      FROM scolarises s
     WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                        WHERE i.etablissement_id = p_etab AND i.eleve_id = s.id AND i.systeme = 'INE'
                          AND i.valide_au IS NULL AND i.statut IN ('declare', 'a_verifier', 'confirme', 'conteste'))
    UNION ALL
    -- ine_conteste : un INE contesté en vigueur (souvent le signe d'un doublon)
    SELECT s.id, 'ine_conteste', NULL, NULL, (SELECT min(i.valide_du) FROM ent_eleve_identifiant i
                                              WHERE i.etablissement_id = p_etab AND i.eleve_id = s.id
                                                AND i.systeme = 'INE' AND i.valide_au IS NULL AND i.statut = 'conteste')
      FROM scolarises s
     WHERE EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                    WHERE i.etablissement_id = p_etab AND i.eleve_id = s.id AND i.systeme = 'INE'
                      AND i.valide_au IS NULL AND i.statut = 'conteste')
    UNION ALL
    -- confidentialite_renforcee : mesure en vigueur (échéance = date de fin prévue)
    SELECT e.id, 'confidentialite_renforcee', NULL, c.au, c.du
      FROM e
      JOIN LATERAL (SELECT c.du, c.au FROM ent_eleve_confidentialite c
                     WHERE c.etablissement_id = p_etab AND c.eleve_id = e.id AND c.leve_le IS NULL
                       AND c.du <= (SELECT jour FROM j) AND (c.au IS NULL OR c.au > (SELECT jour FROM j))
                     ORDER BY c.du DESC LIMIT 1) c ON true
    UNION ALL
    -- majeur : 18 ans révolus à la date de naissance retenue, jamais sur une date contestée ou imprécise
    SELECT s.id, 'majeur', NULL, (s.date_naissance + INTERVAL '18 years')::date, (s.date_naissance + INTERVAL '18 years')::date
      FROM scolarises s
     WHERE s.date_naissance IS NOT NULL AND coalesce(s.date_naissance_precision, 'jour') = 'jour'
       AND coalesce(s.date_naissance_statut, 'declaree') <> 'contestee'
       AND (s.date_naissance + INTERVAL '18 years')::date <= (SELECT jour FROM j)
    UNION ALL
    -- majorite_proche : les 18 ans tombent dans les 30 prochains jours. UN BOOLÉEN : ni `valeur_date`
    -- ni `du` ne portent la date de majorité (elle livrerait la date de naissance) ; `du` = jour du calcul.
    -- Pas de pastille pour un élève émancipé (la majorité ne change plus rien pour lui).
    SELECT s.id, 'majorite_proche', NULL, NULL, (SELECT jour FROM j)
      FROM scolarises s
     WHERE ent_eleve_majorite_le(s.date_naissance, s.date_naissance_precision, s.date_naissance_statut) > (SELECT jour FROM j)
       AND ent_eleve_majorite_le(s.date_naissance, s.date_naissance_precision, s.date_naissance_statut) <= (SELECT jour FROM j) + 30
       AND NOT EXISTS (SELECT 1 FROM ent_eleve_capacite c
                        WHERE c.etablissement_id = p_etab AND c.eleve_id = s.id AND c.nature = 'emancipation'
                          AND c.levee_le IS NULL AND c.du <= (SELECT jour FROM j)
                          AND (c.au IS NULL OR c.au > (SELECT jour FROM j)))
    UNION ALL
    -- en_mobilite : période de mobilité en cours (échéance = retour prévu)
    SELECT s.id, 'en_mobilite', m.type, m.retour_prevu, m.du
      FROM scolarises s
      JOIN LATERAL (SELECT m.type, m.retour_prevu, m.du FROM ent_eleve_mobilite m
                     WHERE m.etablissement_id = p_etab AND m.eleve_id = s.id
                       AND m.du <= (SELECT jour FROM j) AND (m.au IS NULL OR m.au > (SELECT jour FROM j))
                     ORDER BY m.du DESC LIMIT 1) m ON true
    UNION ALL
    -- redoublant
    SELECT s.id, 'redoublant', NULL, NULL, (SELECT jour FROM j)
      FROM scolarises s WHERE s.redoublant
    UNION ALL
    -- nouvel_arrivant : entré depuis moins de 30 jours, ou première année dans l'établissement
    -- (aucune scolarisation antérieure et entrée dans l'année courante). Ni un passage interne, ni
    -- une reprise de données ne sont une arrivée. `du` = date d'entrée.
    SELECT s.id, 'nouvel_arrivant', NULL, NULL, o.date_entree
      FROM scolarises s JOIN en_cours o ON o.eleve_id = s.id
     WHERE o.motif_entree NOT IN ('passage_interne', 'reprise')
       AND (o.date_entree > (SELECT jour FROM j) - 30
            OR (NOT EXISTS (SELECT 1 FROM ent_eleve_scolarisation x
                             WHERE x.etablissement_id = p_etab AND x.eleve_id = s.id AND x.date_entree < o.date_entree)
                AND o.date_entree >= (SELECT date_debut FROM annee)))
    UNION ALL
    -- arrive_en_cours_annee : entrée après les deux premières semaines de l'année courante [H : seuil
    -- de 14 jours à valider avec le secrétariat]. `du` = date d'entrée.
    SELECT s.id, 'arrive_en_cours_annee', NULL, NULL, o.date_entree
      FROM scolarises s JOIN en_cours o ON o.eleve_id = s.id, annee a
     WHERE o.motif_entree NOT IN ('passage_interne', 'reprise')
       AND o.date_entree > a.date_debut + 14 AND o.date_entree <= a.date_fin
    UNION ALL
    -- modalite_particuliere : une modalité de scolarisation en cours. SANS DÉTAIL : ni code ni dates.
    SELECT s.id, 'modalite_particuliere', NULL, NULL, m.du
      FROM scolarises s
      JOIN LATERAL (SELECT min(m.du) AS du FROM ent_eleve_modalite m
                     WHERE m.etablissement_id = p_etab AND m.eleve_id = s.id
                       AND m.du <= (SELECT jour FROM j) AND (m.au IS NULL OR m.au > (SELECT jour FROM j))
                    HAVING count(*) > 0) m ON true
$$;

-- ───────────────────────── 2. F3 : ent_eleve.statut jamais NULL ─────────────────────────
-- Reprise (premier passage, AVANT le recalcul des pastilles : un élève sorti ne garde pas `sans_ine`) : le
-- statut des lignes encore NULL est dérivé de leurs scolarisations (même fonction que la projection ; sans
-- scolarisation : « a_venir »).
DO $$
BEGIN
    IF coalesce(current_setting('ent.e9_premier_passage', true), 'false') = 'true' THEN
        PERFORM set_config('ent.scolarite_projection', 'on', true);
        UPDATE ent_eleve e SET statut = ent_eleve_statut_a(e.etablissement_id, e.id, ent_jour_etablissement(e.etablissement_id))
         WHERE e.statut IS NULL;
        PERFORM set_config('ent.scolarite_projection', 'off', true);
    END IF;
END $$;

-- À l'insertion (ou à une mise à jour qui l'effacerait), le statut est dérivé, jamais laissé NULL.
CREATE OR REPLACE FUNCTION ent_eleve_statut_rempli() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.statut IS NULL THEN
        NEW.statut := coalesce(
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.statut END,
            ent_eleve_statut_a(NEW.etablissement_id, NEW.id, ent_jour_etablissement(NEW.etablissement_id)));
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_statut_rempli ON ent_eleve;
CREATE TRIGGER ent_eleve_statut_rempli
    BEFORE INSERT OR UPDATE OF statut ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_statut_rempli();

-- Sens de NULL : `statut` est un cache dérivé des scolarisations ; NULL = « aucune scolarisation encore » (ligne
-- reprise), jamais « actif ». Les lecteurs le traitent comme « à venir ». NOT NULL n'est PAS posée : la reprise
-- d'anciennes données (`ent_eleve_scolarisation_reprendre`) et ses tests insèrent volontairement des lignes
-- sans statut, hors déclencheurs. En fonctionnement normal, le déclencheur ci-dessus n'en laisse aucune.

-- Recalcul COMPLET de la projection (premier passage, après la reprise des statuts), par établissement et sous
-- son contexte de cloisonnement : les cinq nouvelles pastilles, et toute pastille que le statut repris change.
DO $$
DECLARE etab UUID;
BEGIN
    IF coalesce(current_setting('ent.e9_premier_passage', true), 'false') = 'true' THEN
        FOR etab IN SELECT DISTINCT etablissement_id FROM ent_eleve LOOP
            PERFORM set_config('app.etablissement_id', etab::text, true);
            PERFORM ent_eleve_indicateurs_recalculer(etab, NULL, NULL, NULL, NULL);
        END LOOP;
        PERFORM set_config('app.etablissement_id', '', true);
    END IF;
END $$;

-- ───────────────────────── 3. F5 : contraintes NOT VALID ─────────────────────────
-- Validation au premier passage, SANS réécrire la table d'audit (aucune ligne de
-- `ent_eleve_confidentialite` n'est modifiée ici : une mesure est immuable). Si des lignes sont hors règle, la
-- contrainte reste NOT VALID (elle protège les écritures à venir) ; le démarrage le signale par un journal
-- d'erreur qui nomme la contrainte (`migrate::signaler_contraintes_non_validees`), jamais un simple avertissement
-- de migration.
DO $$
BEGIN
    IF coalesce(current_setting('ent.e9_premier_passage', true), 'false') = 'true' THEN
        BEGIN
            ALTER TABLE ent_eleve_confidentialite VALIDATE CONSTRAINT ent_eleve_confidentialite_regles_chk;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'ent_eleve_confidentialite_regles_chk : lignes hors règle, contrainte laissée NOT VALID';
        END;
        BEGIN
            ALTER TABLE ent_eleve VALIDATE CONSTRAINT ent_eleve_statut_chk;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'ent_eleve_statut_chk : lignes hors règle, contrainte laissée NOT VALID';
        END;
    END IF;
END $$;
