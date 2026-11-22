-- Socle 21, lot V3d — 476 : revue de sécurité de V4 (F-1).
--
-- * `eva_dispense_port` : `BEFORE TRUNCATE` (la garde de 475 couvrait UPDATE et DELETE, pas le vidage de la table : une
--   dispense retirée ou en vigueur ne disparaît pas sans trace). Même convention que 461 (déclencheur d'instruction,
--   aucune dérogation par réglage de session).
-- * `eva_aesh_indisponibilite_port` : ajout seul (UPDATE, DELETE, TRUNCATE refusés). Une indisponibilité saisie par erreur
--   se compense par une nouvelle ligne, jamais par une réécriture : l'alerte « l'aménagement ne pourra pas être assuré »
--   s'appuie sur ce registre.
-- 461–475 intactes (règle 25). Additive et rejouable.

DROP TRIGGER IF EXISTS eva_dispense_vidage ON eva_dispense_port;
CREATE TRIGGER eva_dispense_vidage BEFORE TRUNCATE ON eva_dispense_port
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

DROP TRIGGER IF EXISTS eva_aesh_indispo_ajout_seul ON eva_aesh_indisponibilite_port;
CREATE TRIGGER eva_aesh_indispo_ajout_seul BEFORE UPDATE OR DELETE ON eva_aesh_indisponibilite_port
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_aesh_indispo_vidage ON eva_aesh_indisponibilite_port;
CREATE TRIGGER eva_aesh_indispo_vidage BEFORE TRUNCATE ON eva_aesh_indisponibilite_port
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- F-5 (CE-27 sans faux positif) : une absence JUSTIFIÉE ce jour-là (séance ou demi-journée) du socle 12, quelle que soit la
-- séance. Sert à l'évaluation « hors séance » (aucune séance du cours ce jour-là : `eva_absence_a_la_seance` rend alors
-- NULL) : l'élève absent toute la matinée avec un justificatif valable n'est pas « absent sans justification ». Lecture
-- seule du socle 12 ; même critère de justification que `eva_absence_a_la_seance`.
CREATE OR REPLACE FUNCTION eva_absence_justifiee_ce_jour(p_etab uuid, p_eleve uuid, p_jour date)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ent_asd_absence_seance sa
          JOIN ent_asd_absence a ON a.etablissement_id = sa.etablissement_id AND a.id = sa.absence_id AND a.etat <> 'retire'
         WHERE sa.etablissement_id = p_etab AND sa.eleve_id = p_eleve AND sa.jour = p_jour AND sa.retiree_le IS NULL
           AND a.etat = 'qualifie' AND a.qualification IN ('reputee_legitime', 'excuse_valable'))
$$;

-- F-9 : une dispense (port 34 simulé) SUIT l'élève à la fusion de fiches (stratégie « Deplacer ») : sans cela, l'élève conservé
-- perdait sa dispense, et l'évaluation lui proposait un « absent » ou le laissait noter à tort. Seul le déplacement de
-- l'élève, pendant la fusion (`ent.fusion_eleve = 'on'`, posé par la fusion seule, dans sa transaction), est admis ; toute
-- autre modification reste refusée (la garde de 475 est conservée à l'identique).
CREATE OR REPLACE FUNCTION ent_trg_eva_dispense_port() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_dispense_port : suppression physique interdite, retirer la dispense' USING ERRCODE = '23514';
    END IF;
    IF current_setting('ent.fusion_eleve', true) = 'on' AND NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       AND NEW.matiere_id IS NOT DISTINCT FROM OLD.matiere_id AND NEW.nature IS NOT DISTINCT FROM OLD.nature
       AND NEW.du IS NOT DISTINCT FROM OLD.du AND NEW.au IS NOT DISTINCT FROM OLD.au
       AND NEW.saisi_par IS NOT DISTINCT FROM OLD.saisi_par
       AND NEW.retiree_le IS NOT DISTINCT FROM OLD.retiree_le AND NEW.retiree_par IS NOT DISTINCT FROM OLD.retiree_par THEN
        RETURN NEW;
    END IF;
    IF OLD.retiree_le IS NOT NULL THEN
        RAISE EXCEPTION 'eva_dispense_port : dispense déjà retirée' USING ERRCODE = '23514';
    END IF;
    IF NEW.eleve_id IS DISTINCT FROM OLD.eleve_id OR NEW.matiere_id IS DISTINCT FROM OLD.matiere_id
       OR NEW.nature IS DISTINCT FROM OLD.nature OR NEW.du IS DISTINCT FROM OLD.du OR NEW.au IS DISTINCT FROM OLD.au
       OR NEW.saisi_par IS DISTINCT FROM OLD.saisi_par THEN
        RAISE EXCEPTION 'eva_dispense_port : seul le retrait modifie une dispense' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

-- Tests web et base « moyenne vide » (`periode_complete_avec_le_client_du_frontend`, CI 463 d'ENT ; cinq tests de
-- `tests_evaluations*` qui échouent de façon déterministe dès que la fin de la période de notes dépasse celle du trimestre
-- du labo) : CAUSE. Les périodes de la périodisation « historique » ne sont pas protégées contre le chevauchement (les
-- périodes de notes de l'ancien modèle y sont projetées : `W…` recouvre `T1`). Deux fonctions devaient alors dire de quelle
-- période relève un jour, et ne s'accordaient pas :
--   * `eva_periode_eleve_jour` (période d'un élève un jour donné) joignait DEUX périodes et ne départageait que l'affectation :
--     le `LIMIT 1` rendait l'une ou l'autre selon le plan d'exécution ;
--   * `ent_periode_de_note_pour_classe` (période de notes -> période de la classe) prenait le plus grand recouvrement,
--     puis la plus ancienne.
-- Une période de notes qui dépasse d'un jour le trimestre recouvre davantage sa propre projection que le trimestre : la
-- première fonction rangeait un résultat dans `T1`, la seconde cherchait `W…` : moyenne vide. Règle commune, TOTALE, dans les
-- deux fonctions : à recouvrement égal, la période la PLUS PRÉCISE (la plus courte), puis la plus ancienne (début, fin,
-- identifiant). Aucune donnée sans recouvrement n'est touchée (résultat inchangé). Mêmes signatures et colonnes qu'en 463 et 472.
CREATE OR REPLACE FUNCTION eva_periode_eleve_jour(p_etab uuid, p_eleve uuid, p_jour date)
RETURNS TABLE (classe_id uuid, periode_id uuid, etat text, date_arret_effective date)
LANGUAGE sql STABLE AS $$
    SELECT a.classe_id, p.id,
           COALESCE(e.etat, 'ouverte'),
           COALESCE(e.date_arret_effective, p.date_arret_notes)
      FROM ent_affectation a
      JOIN LATERAL (SELECT ent_periodisation_pour_classe(a.etablissement_id, a.classe_id) AS pz) z ON true
      JOIN ent_periode p
        ON p.etablissement_id = a.etablissement_id AND p.periodisation_id = z.pz
       AND p.date_debut <= p_jour AND p.date_fin >= p_jour
      LEFT JOIN eva_periode_etat e
        ON e.etablissement_id = p.etablissement_id AND e.periode_id = p.id AND e.classe_id = a.classe_id
     WHERE a.etablissement_id = p_etab AND a.eleve_id = p_eleve AND a.classe_id IS NOT NULL
       AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
     ORDER BY a.date_debut DESC, a.id, (p.date_fin - p.date_debut), p.date_debut, p.date_fin, p.id
     LIMIT 1
$$;

CREATE OR REPLACE FUNCTION ent_periode_de_note_pour_classe(p_etab uuid, p_note_periode uuid, p_classe uuid)
RETURNS uuid LANGUAGE sql STABLE AS $$
    SELECT p.id
      FROM ent_note_periode np
      JOIN ent_classe c ON c.etablissement_id = np.etablissement_id AND c.id = p_classe
      JOIN ent_periode p ON p.etablissement_id = np.etablissement_id AND p.annee_id = c.annee_id
                        AND p.periodisation_id = ent_periodisation_pour_classe(np.etablissement_id, c.id)
     WHERE np.etablissement_id = p_etab AND np.id = p_note_periode
       AND p.date_debut <= np.date_fin AND p.date_fin >= np.date_debut
     ORDER BY (least(p.date_fin, np.date_fin) - greatest(p.date_debut, np.date_debut)) DESC,
              (p.date_fin - p.date_debut), p.date_debut, p.date_fin, p.id
     LIMIT 1
$$;
