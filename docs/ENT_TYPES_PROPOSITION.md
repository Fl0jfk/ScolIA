# Proposition — crate `ent-types` (types partagés frontend / backend)

Date : 2026-11-02 (mise à jour 2026-11-03) — Statut : **lot 1 préparé** : crate créée, utilisée par le frontend, contrat vérifié contre le vrai backend ; **`ent-api` pas encore migré** (aucune modification du backend).
Contexte : D-008, D-010 ; prototype Leptos (`ent-rust/web/`) qui recopie aujourd'hui les types JSON d'`ent-api`.

## 1. Constat (vérifié dans le code)

| Fait | Détail |
|---|---|
| Emplacement | `ent-api/src/dto.rs` : 3 886 lignes, **247 types** ; plus **85 types** `…Dto/Body/Query` définis directement dans `routes.rs` (`MeDto`, `LoginBody`, `LoginResponseDto`, labo, personnel…) |
| Visibilité | `mod dto;` est **privé** : aucun autre crate ne peut réutiliser ces types |
| Sens | 123 types `Serialize` seul (réponses), 123 `Deserialize` seul (corps de requête), 1 les deux. Le client a besoin du sens inverse : il faudra dériver les deux |
| Champs | **Aucun champ n'utilise un type serveur** : uniquement `String`, nombres, `bool`, `Uuid`, `NaiveDate`, `chrono::DateTime<Utc>`, `serde_json::Value` et d'autres DTO. Le contenu est donc extractible tel quel |
| Couplage serveur | uniquement via les imports (`domain_eleve::{Absence, AttributKind, Eleve, ElevePeriode}`, `ent_db::{AbsenceMotif, IbanHisto, MailOutboxRow, Matiere, Note, SmsOutboxRow}`) et **102 conversions `impl From<&ent_db::X / domain_eleve::X> for XDto`**, utilisées ~295 fois dans `routes.rs` |
| Attributs serde | 59 `skip_serializing_if`, 26 `default`, 5 `default = "…"`, 1 `flatten` : ils définissent le contrat JSON, à conserver **à l'identique** |
| États métier | transmis en `String` (`"present" / "absent" / "retard"`, `"ouvert" / "clos"`…) |
| Consommateurs | interface `ui.rs` (JavaScript), applis iOS/Android (Swift/Kotlin, hors Rust), prototype Leptos (copie manuelle dans `ent-web-api`) |

## 2. Objectifs et règles

- `ent-types` ne dépend **ni de sqlx, ni d'`ent-db`, ni de `domain-eleve`, ni d'axum** : uniquement `serde`, `serde_json`, `uuid` (avec `serde`), `chrono` (avec `serde`).
- Compile pour le serveur **et** pour le navigateur (`wasm32-unknown-unknown`).
- **Le JSON ne change pas** : mêmes noms de champs, mêmes attributs serde, mêmes représentations. Les applis mobiles et `ui.rs` ne voient aucune différence.
- Une seule définition par type : `ent-api` et `ent-web-api` utilisent la même.
- Migration **par lots**, chacun vérifiable et réversible.

## 3. Conception

```
ent-rust/crates/ent-types/
├── Cargo.toml          serde, serde_json, uuid (serde, SANS v4), chrono (serde, SANS default-features)
└── src/
    ├── lib.rs          modules par domaine, pas de logique
    ├── auth.rs         Moi, LoginBody, LoginResponse, AuthSession…
    ├── appel.rs        Appel, AppelLigne, OuvrirAppelBody, MarquerAppelLigneBody
    ├── edt.rs          EdtCreneau…
    └── eleve.rs        Eleve, Effectif… (lots suivants)
```

### Deux pièges de compilation WebAssembly à éviter
- `uuid` : le workspace active `v4` (génération aléatoire). En WebAssembly, cela tire `getrandom` qui exige une configuration spéciale. `ent-types` déclare `uuid = { version = "1", features = ["serde"] }` **sans** `v4` (les types ne génèrent jamais d'identifiant).
- `chrono` : la fonctionnalité par défaut `clock` demande `wasmbind` dans le navigateur. `ent-types` déclare `chrono = { version = "0.4", default-features = false, features = ["serde"] }`.

### Les conversions : règle d'orphelin de Rust
Une fois `AppelDto` déplacé dans `ent-types`, `ent-api` **ne peut plus écrire** `impl From<&ent_db::Appel> for ent_types::AppelDto` (ni le trait `From` ni les deux types ne lui appartiennent : erreur E0117).

| Option | Avis |
|---|---|
| A. Mettre les conversions dans `ent-types` | **refusée** : réintroduit la dépendance à `ent-db` |
| B. Mettre les conversions dans `ent-db` / `domain-eleve` | possible, mais la couche métier dépendrait des formats d'API (inversion de couches) |
| **C. Trait local à `ent-api`** : `trait VersDto { type Dto; fn vers_dto(&self) -> Self::Dto; }` implémenté pour les types `ent_db` / `domain_eleve` | **recommandée** : autorisé par Rust (trait local), conversions restent dans `ent-api`, appel uniforme `appel.vers_dto()` |

Changement mécanique dans `routes.rs` : `EdtCreneauDto::from(&c)` → `c.vers_dto()`, uniquement pour les types du lot migré.

### Compatibilité des noms
Dans `ent-api/src/dto.rs`, réexporter les types migrés (`pub use ent_types::appel::{AppelDto, …};`) : le reste de `routes.rs` ne change pas de noms. Les suffixes `Dto/Body` sont conservés tant que des appels en dépendent.

### Sens de sérialisation
Les types migrés dérivent `Serialize` **et** `Deserialize`. Points de vigilance :
- `skip_serializing_if = "Option::is_none"` : sans effet à la lecture (un `Option` absent vaut `None`) ;
- `skip_serializing_if` sur un `Vec` ou un `bool` : ajouter `#[serde(default)]` pour que le client accepte le champ absent ;
- `flatten` (réponse de connexion) : vérifier l'aller-retour par un test.

## 4. Plan de migration progressive

| Lot | Contenu | Pourquoi d'abord |
|---|---|---|
| **0** | Crate vide `ent-types` dans le workspace + **tests de contrat JSON** sur les routes du lot 1 (réponse attendue figée), écrits **avant** tout déplacement | filet de sécurité |
| **1** | Appel + EDT + session : `AppelDto`, `AppelLigneDto`, `OuvrirAppelBody`, `MarquerAppelLigneBody`, `EdtCreneauDto`, `MeDto`, `LoginBody`, `LoginResponseDto`, `AuthSessionDto` | déjà consommés par le prototype ; petit lot (~9 types, 2 conversions) |
| 2 | Élève, effectif, dossier, foyer (`EleveDto`, `EffectifDto`, …) | utilisés par les applis mobiles et les futurs écrans |
| 3… | Un domaine à la fois, **au moment où un écran Leptos en a besoin** | pas de migration « pour rien » |

Les types de `routes.rs` (labo, personnel, auth) suivent le même chemin dans leur lot. Les types purement internes au serveur restent dans `ent-api`.

### Déroulé d'un lot (vérifiable)
1. Écrire / compléter les tests de contrat JSON des routes du lot (exemple : `POST /appels`, `GET /edt/mes-creneaux`, `GET /auth/me`, `POST /auth/login`) — ils doivent passer **avant** le déplacement.
2. Déplacer les types dans `ent-types`, ajouter `Deserialize`, garder les attributs serde.
3. Remplacer les `impl From` du lot par `VersDto` ; réexporter les noms dans `dto.rs`.
4. Vérifier : `cargo build` du workspace, **tests de contrat JSON identiques**, tests existants d'`ent-api` (dont les 22 tests d'intégration) et d'`ent-db`, `cargo build --target wasm32-unknown-unknown -p ent-types`.
5. Côté prototype : remplacer les copies de `ent-web-api` par `ent-types` ; relancer les tests contre le vrai backend.
6. Un commit par lot (réversible).

### Ce qui n'est pas fait dans un premier temps
- Pas de typage fort des états (`etat: String` → `enum`) : cela change le contrat pour les clients non Rust. À décider plus tard, lot par lot, avec tests.
- Pas de génération automatique de types Swift/Kotlin (piste possible plus tard : schémas JSON générés depuis `ent-types`).

## 5. Risques

| Risque | Parade |
|---|---|
| Changement involontaire du JSON (renommage, attribut serde oublié) | tests de contrat écrits avant ; relecture des attributs ; réexport des noms |
| Conflit de fonctionnalités `uuid` / `chrono` entre serveur et navigateur | dépendances minimales dans `ent-types` (§3) ; build wasm dans la vérification |
| Migration « en grand » qui s'enlise | lots petits, déclenchés par un besoin d'écran |
| `ent-rust/Cargo.lock` non versionné : versions différentes selon les machines | versionner le lockfile avant le lot 0 (recommandé dans D-010) |

## 6. Estimation

Lot 0 + lot 1 : environ une demi-journée à une journée (création de la crate, ~4 tests de contrat, ~9 types, 2 conversions, adaptation du prototype). Modification du backend : **validation nécessaire** avant de commencer.

## 7. État au 2026-11-03 (lot 1 préparé)

| Élément | État |
|---|---|
| Crate `ent-rust/crates/ent-types` | créée, membre du workspace principal ; dépendances : `serde`, `serde_json`, `uuid` (sans `v4`), `chrono` (sans `clock`) |
| Types du lot 1 | `appel` (`AppelDto`, `AppelLigneDto`, `OuvrirAppelBody`, `MarquerAppelLigneBody`), `edt` (`EdtCreneauDto`), `auth` (`LoginBody`, `LoginResponseDto`, `AuthSessionDto`, `PasskeyChallengeDto`, `MeDto`) ; `Debug` masque le mot de passe et le jeton |
| Règle « aucune dépendance serveur » | vérifiée (`cargo tree` : ni sqlx, ni ent-db, ni axum, ni tokio, ni openssl) ; contrôlée en CI |
| WebAssembly | compile pour `wasm32-unknown-unknown` (local + CI) |
| Tests | 4 tests d'aller-retour JSON ; **test de contrat contre le vrai backend** (`contrat_json_ent_types_lot1`) : 5 routes (`/auth/login`, `/auth/me`, `/edt/mes-creneaux`, `POST /appels`, `/appels/{id}/clore`), JSON relu puis re-sérialisé **identique** |
| Frontend | `ent-web-api` utilise `ent-types` (alias des anciens noms) : copies manuelles supprimées |
| `ent-api` | **non modifié** ; la bascule (réexport dans `dto.rs`, trait `VersDto`) attend l'approbation |

Prochaine étape (à approuver) : migration du lot 1 dans `ent-api` suivant le déroulé du §4, le test de contrat servant de filet de sécurité.
