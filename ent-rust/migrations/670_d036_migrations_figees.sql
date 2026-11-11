-- D-036 — fichiers de migration FIGÉS. Les migrations sont rejouées à chaque démarrage ; les fichiers
-- ci-dessous lisent ou recréent les anciennes colonnes JSON retirées par 668 et 669 (reprise des
-- données, `ADD COLUMN IF NOT EXISTS`, déclencheurs) : les rejouer échouerait, ou ressusciterait une
-- colonne retirée. Le lanceur (`migrate.rs`) saute les fichiers listés ici. Sur une base neuve, ils
-- sont joués dans l'ordre (avant 668) ; sur une base existante, ils ont déjà été joués. Aucun fichier
-- n'est modifié (règle : une migration versée ne se modifie plus).
--
-- Table technique, sans établissement : classée « Technique » au registre.

CREATE TABLE IF NOT EXISTS ent_migration_figee (
    nom TEXT PRIMARY KEY CHECK (nom ~ '^[0-9]{3}_[a-z0-9_]+$' AND length(nom) <= 120),
    raison TEXT NOT NULL CHECK (length(raison) BETWEEN 1 AND 300),
    figee_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO ent_migration_figee (nom, raison) VALUES
    ('250_inscriptions_i0', 'ADD COLUMN resolutions (retirée par 668)'),
    ('275_personne_verification_preferences', 'ADD COLUMN joignable (retirée par 669)'),
    ('304_inscription_demarche_dossier', 'ADD COLUMN scolarite_actuelle (retirée par 668)'),
    ('560_delegation_droits_normalises', 'reprise lue sur ent_delegation.droits (retirée par 668)'),
    ('561_inscription_resolution', 'reprise lue sur ent_inscription_dossier.resolutions (retirée par 668)'),
    ('562_inscription_scolarite_actuelle', 'reprise lue sur scolarite_actuelle (retirée par 668)'),
    ('563_signalement_assiduite_detail', 'reprise lue sur ent_signalement_assiduite.dossier (retirée par 668)'),
    ('662_d036_profil_jours_ouvres', 'reprise lue sur jours_ouvres (retirée par 669)'),
    ('663_d036_parametre_type', 'reprise lue sur ent_parametre.valeur (retirée par 669)'),
    ('664_d036_import_ligne_typee', 'reprise lue sur ent_import_ligne.donnees (retirée par 669)'),
    ('665_d036_personne_plage_joignable', 'reprise lue sur joignable (retirée par 669)'),
    ('666_d036_eleve_fusion_normalisee', 'reprise lue sur conflits_resolus, doublons_touches et cle (retirées par 669)'),
    ('667_d036_personne_fusion_normalisee', 'reprise lue sur ent_personne_fusion.instantane (retirée par 669)')
ON CONFLICT DO NOTHING;
