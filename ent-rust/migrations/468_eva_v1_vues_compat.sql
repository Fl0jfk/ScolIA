-- Socle 21, lot V1 (suite) — 468 : vues de COMPATIBILITÉ pour les anciens lecteurs (M2). Elles rendent,
-- depuis `eva_*`, la forme exacte des anciennes tables `ent_devoir`, `ent_note`, `ent_note_periode`,
-- `ent_note_appreciation` : un ancien lecteur (UI JavaScript, applis mobiles, anciens écrans) peut être
-- repointé sur `<ancienne table>_v` sans changer sa requête, puis migré vers les routes nouvelles (V12).
--
-- Elles ne REMPLACENT pas encore les anciennes tables : celles-ci restent la source des anciens
-- écrivains jusqu'à M3 (V2–V8), où l'on renomme l'ancienne table `_ancien` et où la vue prend son nom.
-- Lecture seule (aucune règle INSTEAD OF), `security_invoker` : la RLS de l'appelant s'applique.
--
-- Écarts de forme assumés (documentés dans 21_EVALUATIONS_V1_V2_LIVRAISON.md) :
--   * `valeur` rend 0 pour un statut non numérique (l'ancienne colonne était NOT NULL) ;
--   * `non_rendu` et `annule` se lisent `non_note` (l'ancienne liste de statuts ne les connaît pas) ;
--   * `publication_explicite` vaut toujours vrai et `publie_le` porte la publication (une évaluation
--     historique « publique à la création » a `publie_le` = sa création) : la règle ancienne
--     « visible si pas explicite, ou publié » reste exacte ;
--   * `ent_note_periode_v.statut` rend le statut le MOINS avancé des classes de la période.

CREATE OR REPLACE VIEW ent_note_periode_v WITH (security_invoker = true) AS
SELECT np.id, np.etablissement_id, np.code, np.libelle, np.date_debut, np.date_fin,
       COALESCE(ag.statut, np.statut) AS statut, np.created_at, np.periode_id
  FROM ent_note_periode np
  LEFT JOIN LATERAL (
        SELECT CASE min(eva_etat_rang(e.etat))
                    WHEN 0 THEN 'ouverte' WHEN 1 THEN 'partielle_appreciations' ELSE 'fermee' END AS statut
          FROM eva_periode_etat e
         WHERE e.etablissement_id = np.etablissement_id AND e.periode_id = np.periode_id) ag ON true;

CREATE OR REPLACE VIEW ent_devoir_v WITH (security_invoker = true) AS
SELECT ev.id, ev.etablissement_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ev.etablissement_id AND np.periode_id = ev.periode_id
         ORDER BY np.created_at, np.id LIMIT 1) AS periode_id,
       co.matiere_id,
       COALESCE(cl.code, g.code, '') AS classe,
       ev.titre,
       ev.date_eval AS date_devoir,
       ev.coef::double precision AS coef,
       ev.bareme::double precision AS sur,
       ev.cree_par AS created_by_user_id,
       ev.cree_le AS created_at,
       co.classe_id,
       (SELECT s.id FROM ent_service_enseignement s
         WHERE s.etablissement_id = ev.etablissement_id AND s.cours_id = ev.cours_id
         ORDER BY s.created_at, s.id LIMIT 1) AS service_id,
       co.groupe_id,
       true AS publication_explicite,
       ev.publiee_le AS publie_le,
       ev.publiee_par AS publie_par_user_id
  FROM eva_evaluation ev
  JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
  LEFT JOIN ent_classe cl ON cl.id = co.classe_id AND cl.etablissement_id = co.etablissement_id
  LEFT JOIN ent_groupe g ON g.id = co.groupe_id AND g.etablissement_id = co.etablissement_id
 WHERE ev.mode = 'note' AND ev.etat = 'ouverte';

CREATE OR REPLACE VIEW ent_note_v WITH (security_invoker = true) AS
SELECT r.id, r.etablissement_id, r.eleve_id, co.matiere_id,
       COALESCE(r.valeur, 0)::double precision AS valeur,
       ev.bareme::double precision AS sur,
       ev.date_eval AS date_note,
       r.saisi_par AS auteur_user_id,
       NULL::text AS libelle,
       r.cree_le AS created_at,
       r.evaluation_id AS devoir_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ev.etablissement_id AND np.periode_id = ev.periode_id
         ORDER BY np.created_at, np.id LIMIT 1) AS periode_id,
       CASE WHEN r.statut IN ('non_rendu', 'annule') THEN 'non_note' ELSE r.statut END AS statut,
       r.saisi_le AS updated_at,
       r.saisi_par AS updated_by_user_id
  FROM eva_resultat r
  JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
  JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
 WHERE ev.mode = 'note' AND ev.etat = 'ouverte';

CREATE OR REPLACE VIEW ent_note_appreciation_v WITH (security_invoker = true) AS
SELECT ap.id, ap.etablissement_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ap.etablissement_id AND np.periode_id = ap.periode_id
         ORDER BY np.created_at, np.id LIMIT 1) AS periode_id,
       ap.eleve_id,
       co.matiere_id,
       ap.texte,
       ap.auteur AS auteur_user_id,
       ap.cree_le AS created_at,
       ap.maj_le AS updated_at
  FROM eva_appreciation ap
  LEFT JOIN ent_cours co ON co.id = ap.cours_id AND co.etablissement_id = ap.etablissement_id
 WHERE ap.etat = 'validee' AND ap.type IN ('cours', 'generale');
