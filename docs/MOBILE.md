# Applis mobiles ScolIA — architecture commune

Date : 2026-11-06 — branche `ent-mobile`. Références : D-003 (une appli par plateforme, tous rôles, version quotidienne), D-009 (passkey obligatoire), D-011 (session mobile longue), D-012 (un établissement par adresse, rattachement par invitation / QR), D-013 (passkey liée à `scolia.fr`), D-017 (droits portés par les actions), `SOCLE1_SECURITE.md` E7/E8.

**Niveau atteint** : *implémenté* ; cœurs Kotlin et Swift *testés* (tests unitaires) ; interfaces Compose et SwiftUI **non compilées** dans le conteneur (voir « Vérifications »). Rien n'est validé sur appareil, ni prêt pilote.

## 1. Organisation du code

| | Android (`ent-rust/apps/android/`) | iOS (`ent-rust/apps/ios/`) |
|---|---|---|
| Cœur testable sans SDK | module Gradle `:core` (Kotlin JVM 17) | cible SwiftPM `ScolIACore` (Foundation, compile sous Linux) |
| Interface | module `:app` (Jetpack Compose, Material 3) | cible `ScolIAUI` (SwiftUI) + appli `App/` (XcodeGen `project.yml`) |
| Versions minimales | Android 9 (API 28), compile/target 35 | iOS 17 |
| Build | `./gradlew assembleDebug test` | `xcodegen generate` puis Xcode ; `swift test` pour le cœur |

Le cœur est **identique dans sa forme** sur les deux plateformes (mêmes noms, mêmes règles, mêmes tests) :

| Fichier (Kotlin / Swift) | Rôle |
|---|---|
| `Modeles` | DTO JSON copiés de `ent-api` (`dto.rs`, `routes.rs`, `roles.rs`, `ent-types`) : session, `/auth/me`, droits, EDT, appel, famille, absences, notes, devoirs, cahier de textes, messagerie, annonces, appareils |
| `ApiClient` | client typé, transport injectable (OkHttp / URLSession en vrai, faux serveur en test), traduction des erreurs (401 → session perdue, 403 → refus serveur, hors ligne…), encodage des chemins, aucune redirection suivie, jeton jamais journalisé |
| `Capacites` | `/auth/me` + `/roles/mes-droits` → profil et liste d'écrans |
| `Rattachement` | lecture d'un lien d'invitation, d'un QR code ou d'une saisie |
| `Passkey` | pont entre le JSON WebAuthn de `webauthn-rs` et l'API native |
| `Session` | parcours de connexion, coffre à jeton, push (interface), état de connexion |
| `HorsLigne` | cache de lecture, cloisonné par établissement et utilisateur |

La couche API est **écrite à la main** à partir des DTO (pas de génération : l'API n'a pas de schéma OpenAPI). Les tests de désérialisation reprennent le JSON réel du serveur ; un test « aller-retour » contre `ent-api` (comme `contre_backend.rs` côté web) reste à écrire (voir §9).

## 2. Écrans par rôle

Le serveur décide de tout (D-003) : la liste d'écrans n'est qu'un **affichage** ; un écran affiché à tort reçoit un 403, géré proprement.

| Profil (déduit de `role` + droits) | Écrans (ordre de la barre ; 4 onglets, le reste dans « Plus ») |
|---|---|
| Élève (`eleve`, ou `eleve_id` présent) | Accueil, EDT, Messages, Notes, Cahier de textes, Mon compte |
| Famille (`parent`) | Accueil, **Absences** (justification), Messages, Notes, EDT, Cahier de textes, Mon compte — choix de l'enfant, droits par enfant (`peut_voir_notes`, `peut_justifier_absence`) |
| Professeur (`prof`) | Accueil (cours du jour), **Appel**, Messages, EDT, Cahier de textes, Saisir des notes, Mon compte |
| Vie scolaire (`cpe`, ou rôle libre avec `appel.verifier` / `absences.lire` portée établissement, D-017) | Accueil, Vie scolaire (absences à traiter), Messages, EDT, Mon compte |
| Direction (`admin`, `direction`) | Accueil, Vie scolaire, Messages, EDT, Mon compte |
| Autre personnel | Accueil, Messages, EDT, Mon compte (+ Vie scolaire / Saisie de notes selon droits) |
| Passkey à enrôler (`portee = enrolement`) | **uniquement** l'écran d'enrôlement (D-009) |

Écrans livrés et API utilisées :

| Écran | API |
|---|---|
| Rattachement | `GET /auth/etablissement` (vérifie l'adresse, récupère le nom) |
| Connexion | `POST /auth/passkey/login/start` + `/finish` ; `POST /auth/login` (secours, second facteur) ; `GET /auth/me` ; `GET /roles/mes-droits` |
| Enrôlement passkey | `POST /auth/passkey/register/start` + `/finish` (mot de passe actuel exigé par le serveur) |
| Accueil | `GET /edt/mes-creneaux?jour=` (professeur), `GET /annonces` |
| Appel | `GET /edt/mes-creneaux?jour=`, `POST /appels`, `POST /appels/{id}/lignes`, `POST /appels/{id}/clore` (**confirmation** avant clôture) |
| Vie scolaire | `GET /vs/absences` (lecture ; traitement sur le web) |
| Absences (famille) | `GET /famille/enfants`, `GET /famille/enfants/{id}/resume`, `POST /absences/{id}/justifier` |
| Notes | `GET /eleves/{id}/notes` |
| Saisie de notes | `GET /note-periodes`, `GET /devoirs?periode_id=&classe=`, `GET /classes/{classe}/effectif?jour=`, `POST /eleves/{id}/notes` |
| Cahier de textes | `GET /cahier-texte[?eleve_id=]` |
| EDT | `GET /eleves/{id}/edt?jour=`, `GET /edt/mes-creneaux?jour=` |
| Messagerie | `GET /messages/fils`, `GET /messages/fils/{id}`, `POST /messages/fils/{id}/repondre` (**confirmation** avant envoi) |
| Mon compte | `GET /auth/sessions`, `DELETE /auth/sessions/{id}` (révoquer un autre appareil), `POST /auth/logout` |

Hors périmètre mobile (restent sur le web, D-003) : création de fil avec choix des destinataires, pièces jointes, signature du carnet, factures, internat, cantine, PAI, réinscription, séjours, décisions de vie scolaire. Ces fonctions existaient dans les prototypes ; elles pourront revenir au cas par cas, après arbitrage.

## 3. Navigation

- Android : activité unique, état dans `AppModele` (ViewModel), aiguillage `RacineScolIA` → coquille `NavigationBar` (4 onglets + menu « Plus »), retour système géré (`BackHandler`). Pas de bibliothèque de navigation (moins de dépendances).
- iOS : `RacineScolIA` → `TabView` (iOS regroupe au-delà de 5 onglets), `NavigationStack` par onglet.
- Aiguillage commun : pas d'établissement → rattachement ; pas de session → connexion ; session mais app verrouillée → verrou biométrique ; `portee = enrolement` → enrôlement ; sinon onglets du rôle.

## 4. Connexion, session, révocation

1. **Passkey d'abord** (D-009) : e-mail mémorisé (D-011), `passkey/login/start` avec `client: "mobile"`, signature native, `passkey/login/finish` → jeton.
   - Android : Credential Manager (`androidx.credentials`, services Play pour Android 9–13) ; le JSON WebAuthn est passé tel quel (`publicKey` extrait des options) puis normalisé vers la forme attendue par `webauthn-rs` (identique au client web `ent-web-app/src/noyau/webauthn.rs`).
   - iOS : `ASAuthorizationPlatformPublicKeyCredentialProvider` (rp `scolia.fr`) ; challenge et identifiants décodés en base64url, réponse réencodée.
   - Les deux refusent un `rpId` autre que `scolia.fr` avant d'appeler le système (défense en profondeur ; le serveur reste juge).
2. **Secours mot de passe** : `POST /auth/login` (`client: "mobile"`) ; si `mfa_required`, second facteur passkey ; aucun jeton stocké avant la fin du second facteur.
3. **Session mobile longue** (D-011, A3 : 90 jours glissants, plafond 1 an côté serveur) ; jeton opaque en `Authorization: Bearer`.
4. **Stockage du jeton** : Android → chiffré AES-256-GCM par une clé non exportable de l'Android Keystore (préférences privées, exclues des sauvegardes) ; iOS → Keychain `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` (ni iCloud, ni migration). Un jeton par établissement.
5. **Déverrouillage biométrique** : à l'ouverture et après 5 min en arrière-plan (empreinte / visage / code de l'appareil : `BiometricPrompt` `BIOMETRIC_STRONG | DEVICE_CREDENTIAL`, `LAContext.deviceOwnerAuthentication`). Pas de second déverrouillage juste après une connexion. C'est un verrou d'interface : la clé du jeton n'est pas (encore) liée à la biométrie (piste : `setUserAuthenticationParameters` / `SecAccessControl .userPresence`).
6. **Révocation** : tout 401 efface jeton **et** cache hors ligne et ramène à la connexion (téléphone perdu révoqué depuis le web ou un autre appareil). Écran « Mon compte » : liste des appareils et révocation des autres sessions. Déconnexion : `POST /auth/logout` puis effacement local, même hors ligne.

## 5. Rattachement à l'établissement (D-012)

- **Lien d'invitation** (principal) : `https://<etab>.scolia.fr/rejoindre?invitation=<code>` ouvert par App Links (Android, `autoVerify`) / Universal Links (iOS, `applinks:*.scolia.fr`).
- **QR code** : même URL, lue par zxing (Android, Apache-2.0, sans service Google) ou AVFoundation (iOS).
- **Saisie** : `lpnb` ou `lpnb.scolia.fr`.
- **Recherche par nom ou ville** : **non livrée**, API absente (voir §9).
- Règles de lecture (testées) : `https` seulement ; hôte sous `.scolia.fr` ; pas d'identifiants, pas de port ; sous-domaines techniques refusés (`www`, `api`…) ; schéma personnalisé refusé (détournable) ; code d'invitation borné (`[A-Za-z0-9_-]{8,128}`), sinon ignoré. L'adresse est ensuite vérifiée par `GET /auth/etablissement`.
- Plusieurs établissements mémorisés (parent d'enfants dans deux établissements) ; une session par établissement.
- Labo local : uniquement en build Debug, `http://10.0.2.2:3001?etablissement_id=…` (Android) / `http://127.0.0.1:3001?…` (iOS).
- Le code d'invitation est lu mais **pas encore consommé** : l'API d'acceptation d'invitation n'existe pas (§9).

## 6. Notifications push

Interface prête (`FournisseurPush`), implémentation par défaut inactive. À la connexion, si un jeton d'appareil est disponible, `POST /devices/push` (`platform` `android`/`ios`, `app_id`). Fournisseur **non choisi** (FCM, APNs direct, relais, UnifiedPush…) : à arbitrer (RGPD, dépendance Google/Apple). Le contenu des notifications ne doit pas porter de données sensibles (titre générique, détail après ouverture authentifiée).

## 7. Hors ligne (minimal, lecture seule)

- Lectures mises en cache : EDT, cahier de textes, notes, enfants, fils de messages. Pas d'écriture différée (appel, justification, messages exigent le réseau).
- Réseau d'abord ; cache en secours **uniquement** si le réseau ou le serveur est indisponible ; bandeau « Hors ligne — données du … ».
- Clé = établissement + utilisateur + chemin (pas de lecture du cache d'un autre compte, testé).
- Android : fichiers chiffrés (clé Keystore dédiée), dossier `noBackupFilesDir`, noms hachés. iOS : protection `complete`, exclus de la sauvegarde, noms hachés.
- Effacé à la déconnexion et à tout 401.

## 8. Sécurité et accessibilité

- Pas de sauvegarde cloud ni transfert d'appareil (Android `allowBackup=false` + règles d'extraction ; iOS `ThisDeviceOnly`).
- HTTPS seul, autorités système seulement (pas de CA utilisateur) ; texte clair autorisé seulement pour `10.0.2.2`/`localhost` en Debug.
- Android `FLAG_SECURE` (pas de capture ni d'aperçu) ; iOS : contenu masqué hors premier plan.
- Pas de cookies, pas de cache HTTP disque (iOS `ephemeral`), aucune redirection suivie (le jeton ne part pas ailleurs).
- `toString` / `description` des types sensibles masquent jeton et mot de passe (testé).
- Libellé d'appareil (User-Agent) affiché en texte brut tronqué (E8).
- Confirmation humaine avant clôture d'appel et envoi de message.
- Accessibilité : tailles de texte du système respectées (Dynamic Type / `sp`), cibles ≥ 48 dp, libellés et états pour les boutons d'appel (`contentDescription`/`stateDescription`), titres marqués (`heading` / `.isHeader`), erreurs annoncées (`liveRegion` / annonce VoiceOver), éléments de liste combinés. **Non vérifié** avec TalkBack / VoiceOver réels.

## 9. API manquantes ou à adapter pour le mobile

| # | Besoin | État | Proposition |
|---|---|---|---|
| M1 | Associations de domaine pour les passkeys natives (`rp_id = scolia.fr`) | absent | servir `https://scolia.fr/.well-known/assetlinks.json` (paquet `fr.scolia.mobile` + empreinte SHA-256 du certificat de signature) et `/.well-known/apple-app-site-association` (`webcredentials` + `applinks` pour `/rejoindre`) ; ajouter l'origine `android:apk-key-hash:<…>` à `ENT_RP_ORIGINS` (déjà supporté par `ent-auth`) |
| M2 | Acceptation d'une invitation (lien / QR) | absent | `GET /auth/invitation/{code}` (public, limité en débit : établissement, rôle attendu, expiration) puis rattachement / création de compte (D-013) |
| M3 | Recherche d'établissement par nom ou ville | absent | annuaire public minimal (`GET /etablissements?q=`) au niveau ENT, sans donnée personnelle |
| M4 | Attestation de l'appli (E7 : `client = mobile` déclaré par le client) | à faire | Play Integrity / App Attest à la création de session mobile ; sinon durée web |
| M5 | Désinscription push | absent | `DELETE /devices/push/{id}` à la déconnexion ; purge des jetons à la révocation de session |
| M6 | Absences de l'enfant (historique, pas seulement « à justifier ») | partiel | liste paginée famille, ou exposer `/eleves/{id}/demi-journees` dans le résumé |
| M7 | Marquer une notification comme lue | absent (aucune route `/notifications/{id}/…` ; pour les messages, l'ouverture d'un fil semble marquer lu, à vérifier) | route dédiée ; compteur de non-lus pour l'accueil |
| M8 | Accueil mobile agrégé | optionnel | `GET /mobile/accueil` (cours du jour, non-lus, absences à justifier) : 1 appel au lieu de 3–4 |
| M9 | Pagination et `If-None-Match` | absent | pagination de `/messages/fils`, ETag sur les lectures mises en cache |
| M10 | Saisie de notes en lot | absent | `POST /devoirs/{id}/notes` (liste), au lieu d'un appel par élève |
| M11 | Test de contrat mobile | à écrire | rejouer les fixtures JSON des tests mobiles contre `ent-api` (comme `contre_backend.rs`) |
| M12 | Signature du carnet, pièces jointes, nouveau fil | existe | non porté (web) ; à réévaluer |

Aucune modification du backend n'a été faite dans ce chantier.

## 10. Vérifications (2026-11-06, conteneur Linux)

| Élément | Résultat |
|---|---|
| Android `:core` | `./gradlew -Pscolia.coreSeulement=true :core:test` : **32 tests verts** (Kotlin 2.2.21, JDK 21, cible 17) |
| Android `:app` | **non compilé** : `dl.google.com` / `maven.google.com` refusés par le réseau du conteneur (SDK, AGP, androidx, Compose introuvables, miroirs aussi bloqués). Analyse par `kotlinc` contre `android.jar` (API 35) : aucune erreur de syntaxe ; erreurs restantes toutes dues aux bibliothèques androidx absentes |
| iOS `ScolIACore` | image Docker `swift:6.1-noble` : `swift build` OK, **28 tests verts** |
| iOS `ScolIAUI` + `App` | **non compilé** (pas de SDK iOS sous Linux) ; `swiftc -parse` : syntaxe valide |
| Sur appareil | rien (passkeys, biométrie, QR, App Links non essayés) |

Une revue indépendante (sans compilation) du code Compose et SwiftUI a été faite ; ses constats sont traités au §12.

Pour compiler ailleurs : Android Studio / SDK 35 → `cd ent-rust/apps/android && ./gradlew assembleDebug test` ; Mac → `brew install xcodegen && cd ent-rust/apps/ios && xcodegen generate && open ScolIA.xcodeproj`.

## 11. Suites proposées

1. Compiler `:app` et `ScolIAUI` sur une machine équipée, corriger, lancer sur émulateur / simulateur contre le labo.
2. M1 (associations de domaine) : préalable à toute passkey native.
3. M2 + M4, puis choix du fournisseur push.
4. Tests d'interface (Compose UI tests, XCUITest) et essais TalkBack / VoiceOver.
5. Liaison de la clé du jeton à l'authentification de l'appareil.

## 12. Revue indépendante (2026-11-06) — corrigé / restant

Corrigé après revue :
- un mauvais mot de passe (401 sans jeton) n'est plus traité comme une session perdue : message « identifiants », jeton existant conservé (tests ajoutés des deux côtés) ;
- verrou Android : `BIOMETRIC_WEAK | DEVICE_CREDENTIAL` sur Android 9–10 ; déverrouillage automatique **seulement** si aucun verrou n'est configuré (sinon on reste verrouillé) ; même règle sur iOS (`passcodeNotSet`) ;
- recréation de l'activité Android : plus de reverrouillage ni de retraitement du lien d'invitation ;
- boutons masqués par une liste (validation de l'appel, saisie de notes) ; marges système et clavier (edge-to-edge) ;
- saisie de notes : saisies remises à zéro au changement de devoir, confirmation récapitulative, pas de double envoi, valeurs non finies refusées ;
- pas de double envoi de message / de justification ; confirmation avant révocation d'un appareil (iOS : seul le bouton est actif) ; 401 pendant une révocation → session perdue ;
- annulation (« Réessayer », changement d'enfant) : plus de faux message d'erreur ni de bascule « hors ligne » ; données de l'enfant précédent masquées pendant le rechargement ;
- iOS : `ASAuthorizationController` retenu pendant la requête ; permission caméra demandée, aperçu redimensionné, message si caméra indisponible ; jetons Keychain purgés au premier lancement (ils survivent à la désinstallation) ; boîtes de dialogue posées sur la liste ; état d'appel inconnu affiché sans sélection ;
- retrait d'un établissement : confirmation, révocation serveur au mieux, cache et e-mail mémorisé effacés ;
- changement d'établissement : la reprise en cours est annulée (pas d'état d'un autre établissement) ; démarrage hors ligne : message explicite ;
- `toString` / `description` de la requête et de la réponse de connexion sans jeton ; OkHttp exposé au module `app` ; `POST_NOTIFICATIONS` et `aps-environment` retirés tant que le push n'est pas choisi ; orientations iPad.

Restant (non bloquant, à traiter à la première compilation) :
- démarrage à froid hors ligne : le cache n'est pas accessible tant que `/auth/me` ne répond pas (prévoir un `Me` mis en cache) ;
- Xcode 16 (SDK iOS 18) recommandé (isolation `@MainActor` des vues) ;
- build Debug Android (`fr.scolia.mobile.debug`) : à déclarer aussi dans `assetlinks.json` pour tester passkeys et App Links ;
- `FLAG_SECURE` non appliqué aux boîtes de dialogue Compose (`DialogProperties(securePolicy = SecureOn)`) ;
- une seule instance de transport HTTP à partager (OkHttp / URLSession) ;
- iOS au-delà de 5 onglets (famille : 7) : onglet « Plus » système, à remplacer éventuellement par un menu maison comme sur Android ;
- clé du jeton non liée à la biométrie (§4).
