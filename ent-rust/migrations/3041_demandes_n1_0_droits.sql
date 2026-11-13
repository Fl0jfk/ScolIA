-- Socle N1 (« Demandes »), lot N1-0 — migration 3041 : droits et projections.
-- Réf. : docs/socles/N1_DEMANDES.md §24 (D-017), R-DM16, R-DM18 ; NJ-82 (le demandeur lisait la fiche complète).
--
-- Les actions D-017 `demandes.*` et leurs modèles de rôles par défaut sont dans le CODE
-- (`ent-auth/src/actions.rs`, fonction `socle_n1_demandes`), comme pour le socle 29 : aucune ligne SQL à écrire,
-- les modèles ne vivent pas en base tant qu'un établissement n'a pas paramétré ses rôles.
--
-- Ce que la base garantit en plus, parce que c'est de la minimisation et non de la politesse :
-- DEUX PROJECTIONS séparées de la demande. L'écran ou l'API du demandeur ne lit QUE la vue « demandeur » ;
-- celle-ci n'a physiquement ni la file, ni le traitant, ni le gel, ni les dates de conservation, ni la
-- raison du routage : un oubli du code applicatif ne peut pas les faire fuir (NJ-82, R-DM18).
-- Les deux vues s'exécutent avec les droits de l'appelant (security_invoker) : la RLS d'établissement s'applique.
-- Additive et rejouable (E8).

CREATE OR REPLACE VIEW ent_dem_vue_demandeur WITH (security_invoker = true) AS
SELECT d.id,
       d.etablissement_id,
       d.numero,
       d.nature_code,
       d.nature_version,
       d.statut,
       d.issue,
       d.decision,
       d.urgence,
       d.demandeur_personne_id,
       d.deposant_personne_id,
       d.eleve_id,
       d.objet,
       d.texte,
       d.texte_scelle,
       d.depose_le,
       d.clos_le,
       d.version,
       d.cree_le,
       d.maj_le
  FROM ent_dem_demande d
 WHERE d.purgee_le IS NULL;

CREATE OR REPLACE VIEW ent_dem_vue_traitant WITH (security_invoker = true) AS
SELECT d.id,
       d.etablissement_id,
       d.numero,
       d.annee_id,
       d.nature_code,
       d.nature_version,
       d.statut,
       d.issue,
       d.decision,
       d.confidentialite,
       d.urgence,
       d.demandeur_personne_id,
       d.deposant_personne_id,
       d.canal_depot,
       d.beneficiaire_agent_id,
       d.eleve_id,
       d.lieu_id,
       d.site_id,
       d.objet,
       d.texte,
       d.texte_scelle,
       d.file_id,
       d.attribuee_a_agent_id,
       d.attribuee_le,
       d.depose_le,
       d.clos_le,
       d.gel,
       d.version,
       d.cree_le,
       d.maj_le
  FROM ent_dem_demande d
 WHERE d.purgee_le IS NULL;
