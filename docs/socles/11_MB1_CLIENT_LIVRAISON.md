# Socle 11 — palier « MB1 client » : les applis Android et iOS sur ENT (livraison)

Branche `ent-socle-11-palier-mb1-client` (base `origin/ENT` = `f9c54101`), règle 39 de `PILOTAGE.md` : une branche, lots sans PR avec contre-revue par lot, une seule PR et une seule CI à la fin. Accord de Florian pour mettre les applis dans `ENT` : 2026-11-27 (la conception disait « pas sans accord », `11_APPLICATIONS_MOBILES.md` §9, MB1). Conception : `11_APPLICATIONS_MOBILES.md` §5, §9 (MB1), §10 ; `11_MOBILE.md` MO0 (DA1, DA2) ; partie serveur déjà versée : `11_MB1_SERVEUR_LIVRAISON.md`, `11_MOBILE_MB0_LIVRAISON.md`.

**Aucun fichier Rust, SQL, `Cargo.toml` ni `Cargo.lock` modifié** ; aucune migration. Le diff touche `ent-rust/apps/**`, `.github/workflows/ent-rust.yml` et des docs.

## Lots
| Lot | Contenu | Commit |
|---|---|---|
| 1 | Fusion `--no-ff` d'`origin/ent-mobile-app` (5da17f42) : appli Android unique (`:core` Kotlin pur, `:app` Compose) et appli iOS unique (`ScolIACore` + `ScolIAUI`, XcodeGen), `docs/MOBILE.md` ; **retrait des 4 prototypes** (`android-eleve`, `android-famille`, `ios-eleve`, `ios-famille`). Aucun conflit (ENT n'avait pas touché `ent-rust/apps/` depuis la base commune). | 381c40f5 |
| 2 | Contrat d'API mobile dans les deux cœurs ; correctifs d'appli de MB0 (DA1, DA2, OkHttp, DS4) | 2ce38646, e18e203d (correctifs de contre-revue) |
| 3 | CI GitHub des applis ; docs | 59b7de0e |

## Fait
| Réf. | Livrable | Où |
|---|---|---|
| §5.2 | `X-ScolIA-Appli: android/<version> (build <n>)` / `ios/<version> (build <n>)` sur **toute** requête, aucun identifiant d'appareil ; le serveur sert alors l'enveloppe codée | `ApiClient.kt`, `ApiClient.swift` ; `AppModele.kt` (`BuildConfig`), `AppModele.swift` (`Info.plist`) |
| §5.2, §5.5 | `Idempotency-Key` (UUID v4) sur toute écriture authentifiée hors `/auth/*` (le serveur les tient hors registre) ; une clé par intention ; paramètre `cle` sur chaque méthode d'écriture pour qu'un nouvel essai rejoue au lieu d'exécuter deux fois | idem |
| §5.3 | Lecture de `{ "erreur": { code, message, champs, reessayer_apres_s, trace } }` : 401 → session perdue (purge), sauf route sans jeton (connexion) ou qui redemande le mot de passe (`/auth/passkey/register/finish`, `/auth/password`) → refus ; 403/404 message neutre ; 426 `AppliTropAncienne` ; 429 `DebitDepasse` (délai) ; autres 4xx → refus avec `code`, `champs`, `reessayer_apres_s`, `trace` ; repli sur le texte court des routes anciennes ; jamais de JSON ni de HTML brut affiché | idem |
| DA1 | Cache hors ligne borné : date de fin sur chaque entrée (minuit, fuseau de l'appareil, pour l'EDT, les créneaux et l'appel ; 24 h au plus sinon), plafond revérifié à la lecture (horloge reculée comprise) ; entrée échue jamais servie et effacée ; purge au démarrage, à la reprise et au retour au premier plan ; ancien format (sans date de fin) effacé ; écriture Android atomique | `HorsLigne.kt`, `HorsLigne.swift`, `Securite.kt` (`CacheChiffre` v2), `Securite.swift` (`CacheFichiers`), `MainActivity.kt`, `Racine.swift` |
| DA2 | Android `compileSdk`/`targetSdk` **36** ; bord à bord (déjà actif) et retour prédictif déclarés | `app/build.gradle.kts`, `AndroidManifest.xml` |
| D-062 | **OkHttp retiré** : transport `HttpsURLConnection` (10 s / 15 s, aucune redirection suivie, HTTPS seul hors labo debug, pas de mode « streaming » qui changerait un 401 en erreur réseau) ; règles R8 d'OkHttp retirées. iOS : garde HTTPS en code en plus d'ATS, 15 s | `Session.kt` (`TransportHttps`), `proguard-rules.pro`, `ApiClient.swift` |
| DS4 | Identifiant d'inscription push (`POST /devices/push` → `{id, platform}`) gardé par établissement ; `DELETE /devices/push/{id}` avant `POST /auth/logout` ; effacé avec la session et à l'oubli de l'établissement | `Session.kt`, `Session.swift`, `Securite.kt` (`MemoirePushPrefs`), `Securite.swift` (`MemoirePushDefauts`) |
| MB1 (CI) | Jobs `mobile-android` (validation du wrapper Gradle, `:core:test`, `:app:testDebugUnitTest` Robolectric, `:app:lintDebug`, `:app:assembleRelease`, SDK 36) et `mobile-ios-coeur` (`swift test` dans `swift:6.1-noble`), requis sur ENT et sur toute branche qui touche `ent-rust/apps/**` ou le workflow (sortie `mobile` de `portee`) ; résultats dans les commentaires de veille | `.github/workflows/ent-rust.yml` |
| Docs | `MOBILE.md` (§6, §7, §8 bis contrat, §9 M5/M11, §10 CI), `apps/README.md`, ligne de `CLAUDE.md` et de `CARTOGRAPHIE_ENT.md` sur les prototypes | — |

## Règles non tranchées → valeur par défaut prudente (D-069)
1. **Fuseau du cache « du jour »** : celui de l'appareil (l'appli ne connaît pas encore le fuseau de l'établissement, qui viendra avec `/mobile/contexte`, MB5) ; jamais plus de 24 h après l'écriture.
2. **401 avec jeton sur `/auth/*`** : le serveur pose `auth.refusee` sur tout 401 de `/auth/*`, y compris un jeton mort sur `/auth/me` ; l'appli le traite comme une session perdue, sauf sur les routes qui redemandent le mot de passe actuel. Correction propre côté serveur proposée en file : l'extracteur `EntSession` pose explicitement `session.expiree` / `session.revoquee`.
3. **Clé d'idempotence** : tirée par envoi tant que l'écran ne la garde pas ; les écrans de l'appel et des notes la garderont pour leurs nouveaux essais avec les opérations différées (MB7).

## Non fait (et pourquoi)
- **Interface SwiftUI** et appli iOS : non compilées hors Mac (passe Mac, `MOBILE.md` §11). Les changements de `ScolIAUI` sont limités (paramètres ajoutés, purge, mémoire push).
- **`:app` Android non compilé en local** : le SDK Android et le dépôt Google Maven (`dl.google.com`) sont inaccessibles depuis le conteneur ; la CI GitHub du palier est la première compilation de `:app` depuis le 2026-11-06.
- Écran de mise à jour (426) et attente du 429 : les erreurs existent, l'affichage dédié viendra avec MB14.
- Fixtures dorées et banc de parité (MB2) ; clé d'appareil et présence (MB4) ; SBOM (MB14) ; `distributionSha256Sum` du wrapper (la CI valide le jar contre les sommes officielles).

## Tests (local, 2026-11-27)
- Cœur Kotlin : **58/58** (`./gradlew -Pscolia.coreSeulement=true :core:test`, JDK 21) ; 24 nouveaux dans `ContratMb1Test` (en-tête, clé, hors registre, enveloppe, jeton mort sur `/auth/me`, mot de passe actuel refusé, champs, 409 en cours, 426, 429, 503, expiration à minuit et à 24 h, horloge reculée, purge, désinscription push, transport réel : texte clair refusé, redirection non suivie, 401 sur POST lu, en-têtes et corps envoyés).
- Cœur Swift : **52/52** (`swift test`, image `swift:6.1-noble`) ; 21 dans `ContratMb1Tests`, miroirs des cas Kotlin.
- Backend : rien à rejouer (aucun fichier Rust modifié ; arbre Rust identique à `f9c54101`, CI 741 verte).

## Contre-revues
- Lot 1 : 4 élevés, tous attendus et du ressort du lot 2 (contrat MB1 absent, DA1, DA2, OkHttp) ; moyens M1 (réponse de `/devices/push`, fait), M2–M5 (invitation non consommée, session courte sans passkey non signalée, passkey du build debug, verrou absent) en file.
- Lot 2 : 1 élevé (jeton mort sur `/auth/*` non détecté) **corrigé** ; M1 (streaming et 401) et M2 (garde HTTPS iOS) **corrigés** ; M3 (clé gardée par les écrans) en file (MB7) ; F1 (plafond revérifié), F3 (push oublié avec l'établissement), F5 **corrigés** ; F2, F4, F6 en file.
- Lot 3 : voir la PR.
