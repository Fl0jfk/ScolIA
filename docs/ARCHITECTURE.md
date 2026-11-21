# Architecture cible et points à retrouver

## Socle
Backend métier/API Rust (axum, sqlx). Frontend en Rust avec **Leptos 0.8** (`DECISIONS.md` D-008, D-010) : rendu serveur + hydratation / îlots, servi par le même serveur axum que l'API (même origine, sous `/web/…`), remplacement progressif de l'UI JavaScript transitoire de `ent-api/src/ui.rs`. Organisation en crates `ent-web-ui` (design system), `ent-web-api` (client typé), `ent-web-app` (modules), `ent-web-server` — conventions dans `FRONTEND_LEPTOS.md`. Types partagés front/back : crate `ent-types` proposée (`ENT_TYPES_PROPOSITION.md`). WASM : îlots Leptos et bureautique ArtCraft (D-006). PostgreSQL cible. Scaleway prioritaire pour hébergement et stockage objet S3-compatible. OVH prioritaire pour SMS. Email à décider après inspection. Architecture multi-tenant.

## Ne pas migrer par réflexe
Distinguer les générations du dépôt. Next.js en production est strictement intouchable. Ne pas modifier ses fichiers, scripts, pipelines, dépendances ou configuration.

## Authentification
Solution Rust maison conservée (`ent-auth`, `ent-oidc` — D-009) : Argon2id, sessions Postgres, passkeys `webauthn-rs`, SSO OIDC par établissement. Cible : cookie de session sécurisé pour le web, jeton Bearer pour les applis mobiles, **passkey obligatoire pour tous** à terme. Restent à traiter : rotation/révocation des sessions, enrôlement obligatoire, récupération de compte, repli pour les familles sans appareil compatible.

## Fichiers
Stockage privé par défaut. Métadonnées/droits en base. URL pré-signée courte générée après authentification et autorisation tenant/objet. Contrôler taille, extension, MIME et contenu réel ; traiter les noms comme non fiables ; quotas, logs, rétention et antivirus/quarantaine selon risque.

## Email/SMS
SMS : OVH prioritaire, vérifier API, coûts, conformité, suivi. Email : identifier d'abord l'existant, puis comparer délivrabilité, SPF/DKIM/DMARC, rebonds, régions, rétention et sous-traitants.

## Moteur documentaire / convt
Étudier https://github.com/opencoredev/convt comme référence architecturale, pas comme solution à appeler par API cloud ou simple wrapper de package. Objectif : moteur documentaire natif piloté en Rust ; orchestration, règles métier et API Rust. Comprendre détection des formats, moteurs et pipelines. Licence AGPL-3.0 annoncée à vérifier, ainsi que dépendances et obligations avant toute reprise. Distinguer inspiration, réimplémentation indépendante et incorporation de code. Des workers spécialisés non-Rust (PDF, bureautique, média) peuvent être justifiés s'ils sont matures et isolés, plutôt que de réécrire un moteur fiable à tout prix. Sandbox, limites CPU/RAM/temps, réseau désactivé par défaut, filesystem temporaire. But : traiter les documents dans une infrastructure maîtrisée/UE et éviter les convertisseurs publics pour les documents scolaires.

## Outils bureautiques
Étudier les projets souhaités : Wordcraft (traitement de texte), un équivalent tableur à identifier dans les notes, Artcraft (piste secondaire graphique). Ne pas supposer les dépôts exacts : retrouver liens/noms, vérifier licence, activité, maturité, dépendances, taille, sécurité, import/export, accessibilité et intégration. Vérifier si WASM est une vraie compilation/port ou seulement une app web embarquée. Priorité Word et tableur. Ne pas proposer OnlyOffice, Collabora, Nextcloud, GIMP, Krita ou autres alternatives par défaut ; demander accord avant d'élargir.

## Observabilité
Logs structurés sans données personnelles superflues, métriques, traces, alertes, suivi jobs, sauvegardes/restauration testée, procédures incident et gestion des secrets. Aucun transfert de données élèves vers un tiers sans validation.
