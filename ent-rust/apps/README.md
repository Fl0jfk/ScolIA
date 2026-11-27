# Applis mobiles ScolIA

Une seule appli par plateforme, pour tous les rôles (D-003) : élèves, familles,
professeurs, vie scolaire, direction. Version « quotidienne » allégée ; les tâches
complexes restent sur le web. L'API Rust (`ent-api`) est la seule source de vérité
et contrôle tous les droits.

| Dossier | Plateforme | Pile | Build |
|---|---|---|---|
| `android/` | Android 9+ (API 28) | Kotlin, Jetpack Compose, Credential Manager | `./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug` (SDK 36 requis) |
| `ios/` | iOS 17+ | SwiftUI, AuthenticationServices, SwiftPM + XcodeGen | `xcodegen generate` puis Xcode ; `swift test` (cœur) |

Architecture, écrans par rôle, sécurité, API manquantes : **`docs/MOBILE.md`**.

Les 4 prototypes d'origine (`ios-famille`, `ios-eleve`, `android-famille`,
`android-eleve`) ont été fusionnés dans ces deux applis puis retirés (historique git :
commit précédant la fusion sur la branche `ent-mobile`). Leurs fonctions hors du
périmètre quotidien (factures, internat, cantine, PAI, réinscription…) relèvent du web.

Labo local : build Debug uniquement, adresse `http://10.0.2.2:3001?etablissement_id=<uuid>`
(émulateur Android) ou `http://127.0.0.1:3001?etablissement_id=<uuid>` (simulateur iOS).

Sur `ENT` depuis le palier MB1 client (2026-11-27, `docs/socles/11_MB1_CLIENT_LIVRAISON.md`) : contrat d'API
mobile appliqué (`X-ScolIA-Appli`, `Idempotency-Key`, erreurs codées), cache hors ligne borné (DA1), API 36 (DA2),
OkHttp retiré. CI GitHub : jobs `mobile-android` et `mobile-ios-coeur` de `ent-rust.yml`. Interface SwiftUI non
compilée hors Mac (procédure dans `docs/MOBILE.md` §11).
