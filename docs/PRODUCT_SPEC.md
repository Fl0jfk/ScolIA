# Spécification produit — périmètre cible

Ce document décrit la cible, pas l'état implémenté. Voir `PROJECT_STATUS.md` pour les faits constatés.

## Phase 1 — socle commercial
- Administration : établissements/sites, années scolaires, périodes, calendriers, utilisateurs, rôles, personnels, enseignants, classes, groupes, options.
- Admissions : demandes, candidatures, préinscriptions, inscriptions/réinscriptions, pièces, responsables, relances et historique.
- Dossier élève : identité, responsables, scolarité, historique, documents, autorisations, vue consolidée et actions contextualisées.
- Pédagogie : matières, services, EDT, salles, évaluations, notes, compétences, moyennes, bulletins, conseils, appréciations, imports/exports.
- Vie scolaire : absences, retards, justificatifs, appels, incidents, sanctions/mesures éducatives, suivi et notifications.
- Communication : messagerie, campagnes ciblées, modèles, documents, publipostage, suivi de distribution et pièces jointes privées.
- Finance : facturation, échéanciers, remises/aides, paiements, rapprochements, impayés, comptabilité/interfaces selon périmètre validé.
- Services : internat, restauration, transports, sorties, séjours, voyages et autorisations.
- Portails : parents, élèves, enseignants et personnels ; dashboards et statistiques selon permissions.

## Applications mobiles — incluses au MVP
Les applications Android et iOS déjà commencées sont obligatoires dans le MVP. Retrouver d'abord code, technologie, documentation et décisions. Fonctions ciblées selon rôle : par exemple appel rapide pour enseignants/surveillants, accès élèves et parents, démarches et suivi adaptés. Ce ne sont pas des copies complètes de l'ENT web. Ne pas redéfinir leur périmètre sans audit et arbitrage.

## Phase 2 — différenciation
IA native, automatisations avancées, tableaux de bord augmentés, moteur documentaire et outils bureautiques intégrés.

Chaque domaine doit préciser acteurs, règles métier, modèle/historique, permissions, validations, erreurs, notifications, imports/exports, tests et critères d'acceptation.

« Terminé » ne signifie pas qu'un écran existe : UI, métier, API, persistance, validations, permissions, erreurs, cas limites, historique, tests, sécurité et documentation doivent être traités.
