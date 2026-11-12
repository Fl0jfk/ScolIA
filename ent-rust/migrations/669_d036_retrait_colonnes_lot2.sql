-- D-036 — retrait des neuf anciennes colonnes JSON reprises par le lot 2 (migrations 662–667) :
--   ent_etablissement_profil.jours_ouvres      → ent_etablissement_jour_ouvre
--   ent_parametre.valeur                       → type_param / valeur_entier / valeur_booleen / valeur_heure
--   ent_import_ligne.donnees                   → dix-sept colonnes typées
--   ent_personne_preference.joignable          → ent_personne_plage_joignable
--   ent_personne_preference_version.joignable  → ent_personne_preference_version_plage
--   ent_eleve_fusion.conflits_resolus          → ent_eleve_fusion_conflit(_cle)
--   ent_eleve_fusion.doublons_touches          → ent_eleve_fusion_doublon
--   ent_eleve_fusion_ligne.cle                 → ent_eleve_fusion_ligne_cle
--   ent_personne_fusion.instantane             → colonnes typées + ent_personne_fusion_objet / _lien_supprime / _contact_supprime
-- Les déclencheurs de lecture seule et leurs fonctions partent avec les colonnes.
--
-- Rejouable (IF EXISTS). Ne supprime ni `ent_eleve_fusion.resume` ni les autres exceptions D-036.

DROP TRIGGER IF EXISTS ent_profil_jours_ouvres_lecture_seule ON ent_etablissement_profil;
DROP FUNCTION IF EXISTS ent_profil_jours_ouvres_lecture_seule();
ALTER TABLE ent_etablissement_profil DROP COLUMN IF EXISTS jours_ouvres;

DROP TRIGGER IF EXISTS ent_parametre_valeur_lecture_seule ON ent_parametre;
DROP FUNCTION IF EXISTS ent_parametre_valeur_lecture_seule();
ALTER TABLE ent_parametre DROP COLUMN IF EXISTS valeur;

DROP TRIGGER IF EXISTS ent_import_ligne_donnees_lecture_seule ON ent_import_ligne;
DROP FUNCTION IF EXISTS ent_import_ligne_donnees_lecture_seule();
ALTER TABLE ent_import_ligne DROP COLUMN IF EXISTS donnees;

DROP TRIGGER IF EXISTS ent_personne_preference_joignable_lecture_seule ON ent_personne_preference;
DROP TRIGGER IF EXISTS ent_personne_preference_version_joignable_lecture_seule ON ent_personne_preference_version;
DROP FUNCTION IF EXISTS ent_personne_preference_joignable_lecture_seule();
ALTER TABLE ent_personne_preference DROP COLUMN IF EXISTS joignable;
ALTER TABLE ent_personne_preference_version DROP COLUMN IF EXISTS joignable;

DROP TRIGGER IF EXISTS ent_eleve_fusion_ligne_cle_lecture_seule ON ent_eleve_fusion_ligne;
DROP TRIGGER IF EXISTS ent_eleve_fusion_json_lecture_seule ON ent_eleve_fusion;
DROP FUNCTION IF EXISTS ent_eleve_fusion_json_lecture_seule();
ALTER TABLE ent_eleve_fusion DROP COLUMN IF EXISTS conflits_resolus;
ALTER TABLE ent_eleve_fusion DROP COLUMN IF EXISTS doublons_touches;
ALTER TABLE ent_eleve_fusion_ligne DROP COLUMN IF EXISTS cle;

DROP TRIGGER IF EXISTS ent_personne_fusion_instantane_lecture_seule ON ent_personne_fusion;
DROP FUNCTION IF EXISTS ent_personne_fusion_instantane_lecture_seule();
ALTER TABLE ent_personne_fusion DROP COLUMN IF EXISTS instantane;
