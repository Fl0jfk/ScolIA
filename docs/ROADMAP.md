# Feuille de route par socles

Fixée par Florian le 2026-11-05. Remplace la feuille de route « indicative » de l'audit initial, dont les idées utiles sont reprises ci-dessous.
Méthode pour chaque lot : **1. réflexion** (état des lieux, recherche, conception) → **2. action** (code) → **3. test** (automatisé, puis CI). Point d'avancement environ toutes les demi-journées.
Les listes ci-dessous ne sont pas exhaustives : chaque socle doit aller au-delà d'un ENT classique dès que c'est utile (sécurité, fonctionnalités, ergonomie).

Branche : `ENT` (D-001), tout en Rust (D-002). Next.js, `main`, production : intouchables.

## Socle 1 — Sécurité « carrée »

Critère de sortie : on peut affirmer, preuves et tests à l'appui, que l'authentification, les rôles, l'isolation des établissements et l'accès aux données sont solides.
Plan détaillé et suivi : `docs/SOCLE1_SECURITE.md`.

- **Authentification** : passkey obligatoire pour tous (enrôlement guidé, plusieurs passkeys, récupération), session web en cookie `HttpOnly` (D-011), jeton mobile révocable, anti-force brute et limitation de débit, ré-authentification pour les actions sensibles, liste et révocation des appareils connectés, alerte de nouvelle connexion, SSO durci.
- **Rôles et droits** : modèle de capacités explicite (rôle → actions), une fonction d'autorisation unique, droits relationnels (professeur → ses classes via `enseigne_a`, responsable → ses enfants selon l'autorité légale, élève → lui-même), délégations et remplacements datés.
- **Isolation des établissements** : établissement déduit de l'adresse (D-012) et de la session, jamais du client ; type Rust dédié obligatoire dans les accès aux données ; défense en profondeur PostgreSQL (RLS) ; matrice de tests de séparation étendue à chaque domaine.
- **Documents et données** : stockage privé, **URL pré-signées courtes** émises après contrôle des droits, contrôle du type réel des fichiers, chiffrement des données les plus sensibles (santé, coordonnées bancaires), journal d'accès en ajout seul.
- **Transverse** : CSP et en-têtes partout, secrets hors du code, journaux minimisés, tests de sécurité en CI, RGPD (conservation, droit d'accès).

## Socle 2 — Domaines et actions métier (ENT complet, commercialisable)

Critère de sortie : un ENT au niveau de Pronote / École Directe / Charlemagne / École Futée, prêt à commercialiser.

- **Dossier élève au centre** : toutes les actions métier passent par lui ; modèle relationnel normalisé.
- **Domaines à identifier et créer** (base : `docs/CARTOGRAPHIE_ENT.md`, 16 domaines) : administratif (dossiers élèves, responsables, foyers, inscriptions), vie scolaire (appel, vérification de l'appel, absences déclarées à l'accueil, retards, sanctions), pédagogie (EDT, cahier de textes, notes, compétences, bulletins), facturation (reliée à l'administratif), communication (messagerie, annonces, notifications), santé, restauration, internat, séjours, etc.
- **Actions métier** dans chaque domaine, et **communication entre domaines** (« toile d'araignée ») : événements typés, outbox, réactions d'un domaine à l'autre (ex. inscription validée → facturation ; absence → notification famille).
- Applications iOS/Android (D-003) pour l'usage quotidien, sur la même API.

## Socle 3 — Fonctions différenciantes (apportées par Florian)

Ce qui fait de ScolIA plus qu'« un ENT de plus » : par exemple le « chat first », une IA au-dessus de tout qui comprend tout (jamais plus de droits que l'utilisateur, D-AI). Liste à définir avec Florian une fois le socle 2 livré.

## Repris de la feuille de route initiale

Préparation pilote (charge, RGPD, restauration testée, documentation, support) et innovations (moteur documentaire, bureautique ArtCraft, automatisations) restent à placer dans les socles 2 et 3.
