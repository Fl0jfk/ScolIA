# ScolIA — instructions Claude Code

## Langue
Toujours répondre à Florian **en français**, y compris pour les points d'étape et les sujets très techniques (décision de Florian, 2026-11-09).

**Contradiction bienvenue (Florian, 2026-11-11)** : ne pas hésiter à contredire Florian quand une idée paraît mauvaise ou mal placée, en expliquant pourquoi et en proposant mieux. Ses idées sont notées dans `docs/idees/CARNET_IDEES.md` avant expertise.

## Mission
Tu travailles sur la nouvelle ENT ScolIA, dont le cœur cible est en Rust. Le dépôt contient plusieurs générations et une application Next.js en production. Comprends l'existant ; ne repars pas de zéro.

## Interdictions absolues
- **Next.js en production est intouchable** : aucune modification, formatage, renommage, déplacement, suppression, génération, dépendance, script, pipeline, configuration, documentation opérationnelle, build ou déploiement. Lecture ciblée seulement si nécessaire pour comprendre les frontières. **Seule exception : le chantier D-053** (correction des failles critiques et élevées, `docs/missions/MISSION_NEXTJS_CORRECTIFS.md`), avec ses garde-fous ; seul le pilote pousse sur `main`.
- **DB de production : aucun accès, même en lecture**. Aucune connexion, requête, migration, seed, import/export, sauvegarde/restauration ou test.
- **Secrets/env de production : ne jamais consulter, afficher, copier ou utiliser.** Ne demande aucun credential.
- Aucun déploiement ou release en production.
- Aucun commit/push/merge/PR vers main/master ou branche de production sans autorisation explicite.
- **Branche Rust autorisée : `ENT`** (confirmée le 2026-11-02, D-001). `main` = production Next.js, `dev` = intégration/labo Next.js : ne pas y développer l'ENT Rust.
Si une action risque de franchir une limite, arrête-toi et demande validation.

## Décisions de référence
L'audit initial (`docs/INITIAL_MISSION.md`) est terminé (2026-11-02). Les décisions validées sont dans `docs/DECISIONS.md` (D-001 à D-007) et font foi ; les erreurs passées dans `docs/DECISIONS_AND_FAILURES.md`. Lis-les avant toute décision d'architecture.

## Orientations techniques
- **Tout l'ENT en Rust, frontend compris** (D-002). Backend : axum + sqlx (existant dans `ent-rust/`). Frontend : **Leptos 0.8** (D-008, D-010 : socle validé par le prototype d'appel, `ent-rust/web/`), rendu serveur + hydratation / îlots, intégré au serveur axum. Versions épinglées : `leptos` 0.8.23, `leptos_router` 0.8.17, `leptos_meta` 0.8.7, `leptos_axum` 0.8.10, `cargo-leptos` 0.3.11, `wasm-bindgen` 0.2.130 ; ne pas produire de code d'anciennes API Leptos. **Lire `docs/FRONTEND_LEPTOS.md` (conventions et pièges) avant tout développement frontend.** Pas de migration générale : un module à la fois, après les préalables de D-010. L'UI JavaScript actuelle (`ent-rust/crates/ent-api/src/ui.rs`) est une erreur de départ (F-001) : ne pas l'étendre, la remplacer module par module, sans migration massive.
- WebAssembly : bureautique ArtCraft en iframe (D-006) et îlots interactifs Leptos.
- PostgreSQL comme base centrale cible.
- Scaleway prioritaire pour hébergement et stockage objet S3-compatible. **SMS : OVH** (D-004).
- Email : existant Next = Scaleway TEM + IMAP OVH ; choix pour l'ENT Rust encore ouvert.
- Auth : conserver la solution Rust maison (`ent-auth`, `ent-oidc` : Argon2id, sessions Postgres, passkeys via `webauthn-rs`, SSO OIDC) ; pas de bibliothèque tierce de type `better-auth` (D-009). Web : cookie de session sécurisé ; applis mobiles : jeton Bearer. **Passkey obligatoire pour tous les comptes à terme** (D-009).

## Produit
Voir `docs/VISION.md` et `docs/PRODUCT_SPEC.md`. L'élève est le centre de navigation métier, mais le modèle reste relationnel et normalisé. Les apps mobiles sont obligatoires dans le MVP (D-003) : **une seule appli par plateforme** (SwiftUI pour iOS, Kotlin/Jetpack Compose pour Android, pas Expo) **pour tous les rôles** (élèves, familles, professeurs, administratifs, direction), avec écrans et actions selon les droits. C'est une version « quotidienne » allégée (appel, notes, messagerie, visio…) ; les tâches complexes (séjours scolaires…) restent sur le web. Les 4 prototypes de `ent-rust/apps/` sont à fusionner.

## Sécurité, données et IA
Données d'élèves/familles : sécurité par conception, cloisonnement établissement, autorisation côté serveur, moindre privilège, audit et minimisation. Fichiers privés : URL pré-signées courtes émises après authentification et contrôle des droits. L'IA n'a jamais plus de droits que l'utilisateur et passe par les services métier. Actions sensibles et envois de messages soumis à confirmation humaine. Mistral prioritaire à étudier sous réserve de garanties contractuelles réelles. Pas de clé dans le frontend ni de fallback externe non autorisé.

## Open source et moteurs
- **Conversion de fichiers (D-005)** : moteur maison en Rust, briques libres non-AGPL, exécuté sur nos serveurs. convt (AGPL-3.0) sert d'inspiration uniquement : **ne jamais copier son code**.
- **Bureautique (D-006)** : suite ArtCraft (Wordcraft, GridCraft, DeckCraft — MIT/Apache-2.0) intégrée en WebAssembly ; version figée, marques et logos ArtCraft non utilisés.
Pour toute autre brique : vérifier identité exacte, activité, licence, dépendances, sécurité et réalité du port WASM. Ne pas adopter une dépendance/API cloud comme solution finale par défaut. Toute reprise de code nécessite analyse de licence et validation. Ne propose pas d'alternatives non demandées sans expliquer et demander accord.

## Agents et modèles
Sonnet = agent principal par défaut pour le développement courant. Haiku = exploration, recherche ciblée, petites tâches déterministes/doc légère. Opus = seulement architecture difficile, décisions structurantes ou audits approfondis, jamais par défaut. Déléguer seulement si le gain est réel.
- `explore-haiku` : lecture seule.
- `chores-haiku` : tâche très limitée, max. deux fichiers, jamais sécurité/DB/migrations.
- `review-sonnet` : revue indépendante ciblée selon le risque.
Pas d'orchestration complexe permanente. Éviter de relire tout le dépôt inutilement. `/compact` et `/clear` gèrent le contexte, pas le quota.

## Qualité
**Exigence « masterclass » (Florian, 2026-11-06)** : chaque domaine métier est pensé de A à Z, en autonomie et avec bon sens, jamais un écran minimal. Florian ne liste pas les actions : c'est à nous de les déduire des besoins réels de chaque acteur (établissement, direction, secrétariat, profs, CPE/vie scolaire, accueil, familles, élèves mineurs et majeurs) — ce qui leur fait perdre du temps, ce qu'ils redoutent, ce qui les ravirait — jusqu'à des dizaines ou centaines d'actions utiles, avec cas limites et liens vers les autres domaines. Ce n'est pas une exhaustivité mécanique. Méthode (D-021) : on avance **socle par socle** (carte `docs/SOCLES.md` ; élève, puis inscriptions, puis dossier administratif…), **conception sur Opus** (`docs/socles/`) avant tout code, **code sur Sonnet**, puis relecture de Florian.
Une fonctionnalité terminée couvre, selon le périmètre : UI, logique métier, API, DB, validation, permissions, erreurs/cas limites, historique, tests, sécurité et documentation. Distinguer découvert / proposé / implémenté / testé / validé / prêt pilote / prêt production. Vérifications déterministes (format, build, tests, lint, diff) quand elles sont sûres. Après trois tentatives infructueuses, arrêter et revoir le plan.

## Documentation
Lire les docs existantes avant de les modifier. Préserver les idées utiles, fusionner les doublons, signaler les contradictions. Maintenir `PROJECT_STATUS.md`, `DECISIONS.md` et `DECISIONS_AND_FAILURES.md`. Faits, hypothèses, recommandations et questions doivent être séparés.
