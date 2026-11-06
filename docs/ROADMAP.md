# Feuille de route par socles

Fixée par Florian le 2026-11-05, carte des socles ajoutée le 2026-11-06 (`docs/SOCLES.md`). Remplace la feuille de route « indicative » de l'audit initial, dont les idées utiles sont reprises ci-dessous.
Méthode pour chaque lot : **1. réflexion** (état des lieux, recherche, conception) → **2. action** (code) → **3. test** (automatisé, puis CI). Point d'avancement environ toutes les demi-journées.
Les listes ci-dessous ne sont pas exhaustives : chaque socle doit aller au-delà d'un ENT classique dès que c'est utile (sécurité, fonctionnalités, ergonomie).

Branche : `ENT` (D-001), tout en Rust (D-002). Next.js, `main`, production : intouchables.

## Socle 1 — Sécurité « carrée »

Statut : **fait** selon Florian (2026-11-06) ; restes éventuels suivis dans `docs/SOCLE1_SECURITE.md`.
Critère de sortie : on peut affirmer, preuves et tests à l'appui, que l'authentification, les rôles, l'isolation des établissements et l'accès aux données sont solides.
Plan détaillé et suivi : `docs/SOCLE1_SECURITE.md`.

- **Authentification** : passkey obligatoire pour tous (enrôlement guidé, plusieurs passkeys, récupération), session web en cookie `HttpOnly` (D-011), jeton mobile révocable, anti-force brute et limitation de débit, ré-authentification pour les actions sensibles, liste et révocation des appareils connectés, alerte de nouvelle connexion, SSO durci.
- **Rôles et droits** : modèle de capacités explicite (rôle → actions), une fonction d'autorisation unique, droits relationnels (professeur → ses classes via `enseigne_a`, responsable → ses enfants selon l'autorité légale, élève → lui-même), délégations et remplacements datés.
- **Isolation des établissements** : établissement déduit de l'adresse (D-012) et de la session, jamais du client ; type Rust dédié obligatoire dans les accès aux données ; défense en profondeur PostgreSQL (RLS) ; matrice de tests de séparation étendue à chaque domaine.
- **Documents et données** : stockage privé, **URL pré-signées courtes** émises après contrôle des droits, contrôle du type réel des fichiers, chiffrement des données les plus sensibles (santé, coordonnées bancaires), journal d'accès en ajout seul.
- **Transverse** : CSP et en-têtes partout, secrets hors du code, journaux minimisés, tests de sécurité en CI, RGPD (conservation, droit d'accès).

## Socles 2 à 51 — la carte

Carte complète, périmètres, dépendances, existant et justification de l'ordre : **`docs/SOCLES.md`**. Socles 1 à 4 fixés par Florian (2026-11-06) ; socles 5 à 51 proposés, à valider (`SOCLES.md` §8).
Chaque socle est un domaine entier conçu en profondeur avant d'être codé (document `docs/SOCLE_NN_<nom>.md`), puis livré complet (serveur, droits, événements, écrans web, mobile si usage quotidien, tests, documentation). Ce qui n'est pas encore refait reste servi par l'existant (ancienne interface figée, D-010). Public et privé ensemble à chaque socle (D-019).

| Étage | Socles | Jalon atteint à la fin |
|---|---|---|
| A · Cœur | 2 L'élève · 3 Inscriptions et réinscriptions · 4 Dossier administratif (foyers, responsables, coordonnées, identifiants officiels) | J1 « Cœur » (avec 5) |
| B · Cadre commun | 5 Structure et année scolaire · 6 Personnels, services et équipes · 7 Documents, pièces et signatures · 8 Notifications et diffusion · 9 Interopérabilité nationale et identité fédérée | |
| C · Journée | 10 Emploi du temps · 11 Applications mobiles · 12 Appel et assiduité · 13 Discipline et suivi éducatif · 14 Accueil, sorties et sûreté · 15 Absences des personnels et remplacements | J2 « Journée » |
| D · Échanger | 16 Messagerie et conversations · 17 Annonces et campagnes · 18 Formulaires, autorisations et consentements · 19 Agenda, réunions et rendez-vous | |
| E · Apprendre et évaluer | 20 Cahier de textes et travail · 21 Évaluations et notes · 22 Compétences et livrets · 23 Bulletins, conseils et passage | |
| F · Argent des familles | 24 Facturation · 25 Paiements et recouvrement · 26 Bourses et aides | |
| G · Premier établissement | 27 RGPD, conservation et archives · 28 Mise en service, reprise de données et support · 29 Portails et à-faire unifié | **J3 « Pilote »** |
| H · Au-dessus des domaines | 30 Assistant IA et « chat first » · 31 Automatisations · 32 Espaces de travail et bureautique · 33 Visio et continuité | J4 « Différence » |
| I · Chaque élève | 34 Santé · 35 Besoins particuliers · 36 Suivi social, psy, protection de l'enfance · 37 Orientation · 38 Examens · 39 Stages | |
| J · Services | 40 Restauration · 41 Internat · 42 Sorties et voyages · 43 Périscolaire et transports · 44 Ressources et prêts · 45 CDI · 46 Instances · 47 Pilotage | J5 « ENT complet » |
| K · Organisme | 48 Comptabilité, budget et achats · 49 Paie | |
| L · Extensions | 50 Premier degré · 51 Post-bac et apprentissage | J6 |

**Ancien « socle 3 — fonctions différenciantes »** : devient l'étage H (socles 30 à 33), placé juste après le jalon pilote, parce que l'IA vaut ce que valent les actions et les données qu'elle peut toucher. Ses points d'accroche sont posés dès le socle 2 : chaque socle décrit ses actions (identifiants stables) pour l'assistant, alimente la chronologie typée et passe ses actions sensibles par une confirmation humaine (D-AI). Il peut remonter juste après la messagerie (16) si la priorité commerciale l'exige.

**Ancien « socle 2 — domaines »** : ses 26 domaines et ses lots S2-0 à S2-15 sont répartis dans les socles (correspondance : `SOCLES.md` §6). `docs/SOCLE2_DOMAINES.md` reste la référence des actions métier (DOS-, VS-, FIN-…) et de la toile des événements. Lots déjà réalisés : S2-0 (outbox), S2-1 (personnes), S2-2 (référentiel), S2-4 (vie scolaire), fusionnés sur `ENT` ; S2-3 (inscriptions), S2-6 (cahier de textes), finances et mobile sur branches non fusionnées.

## Repris de la feuille de route initiale

Préparation pilote (charge, RGPD, restauration testée, documentation, support) → socles 27 à 29, plus les domaines nécessaires au pilote choisi (la restauration, par exemple, peut être servie par l'existant ou avancée). Innovations : moteur documentaire et conversion → socle 7 ; bureautique ArtCraft → socle 32 ; automatisations → socle 31.
