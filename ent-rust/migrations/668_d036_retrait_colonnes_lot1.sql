-- D-036 — retrait des quatre anciennes colonnes JSON reprises par le lot 1 (migrations 560–563) :
-- `ent_delegation.droits`, `ent_inscription_dossier.resolutions`, `ent_inscription_dossier.scolarite_actuelle`,
-- `ent_signalement_assiduite.dossier`. Leur contenu vit dans les tables typées (560, 561, 562, 563) ;
-- le code ne les lit ni ne les écrit plus. Les déclencheurs de lecture seule et leurs fonctions partent
-- avec elles (un déclencheur « UPDATE OF colonne » empêche de retirer la colonne).
--
-- Rejouable (IF EXISTS). Aucune migration existante n'est modifiée : les fichiers de reprise devenus
-- caducs sont figés par la migration 670 (le lanceur ne les rejoue plus).

DROP TRIGGER IF EXISTS ent_delegation_droits_lecture_seule ON ent_delegation;
DROP FUNCTION IF EXISTS ent_delegation_droits_lecture_seule();
ALTER TABLE ent_delegation DROP COLUMN IF EXISTS droits;

ALTER TABLE ent_inscription_dossier DROP COLUMN IF EXISTS resolutions;
ALTER TABLE ent_inscription_dossier DROP COLUMN IF EXISTS scolarite_actuelle;

DROP TRIGGER IF EXISTS ent_signalement_dossier_lecture_seule ON ent_signalement_assiduite;
DROP FUNCTION IF EXISTS ent_signalement_dossier_lecture_seule();
ALTER TABLE ent_signalement_assiduite DROP COLUMN IF EXISTS dossier;
