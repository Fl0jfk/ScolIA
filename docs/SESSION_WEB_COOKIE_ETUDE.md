# Étude — session web : du jeton en `localStorage` au cookie `HttpOnly`

Date : 2026-11-02 — Statut : **étude (§1–§14) puis mise en œuvre A6 (§15, 2026-11-05)** : le relais cookie est intégré au serveur web `ent-web-server` et le frontend Leptos n'a plus de jeton en `localStorage`. `ent-auth` / `ent-api` restent inchangés (option A du §14).

## 1. Existant (vérifié dans le code)

| Élément | Constat | Preuve |
|---|---|---|
| Jeton | 32 octets aléatoires (hex, 64 caractères), **seul son SHA-256 est stocké** en base | `ent-auth/src/token.rs` |
| Durée | **14 jours fixes** (`ENT_SESSION_TTL_DAYS`), pas de prolongation à l'usage, pas de délai d'inactivité | `ent-db/src/auth_repo.rs` |
| Révocation | déconnexion (`POST /auth/logout`) ; autres sessions révoquées au changement de mot de passe ; toutes les sessions d'un compte révocables par l'administration ; purge des sessions expirées au démarrage | `ent-db/src/service.rs`, `auth_repo.rs` |
| Transport | en-tête `Authorization: Bearer` **uniquement** (extracteur `EntSession`) | `ent-api/src/auth.rs` |
| Remise au client | le jeton est renvoyé **dans le JSON** par `POST /auth/login`, `/auth/passkey/login/finish`, `/auth/oidc/complete`, `/auth/register` | `ent-api/src/routes.rs` |
| Stockage web | ancienne interface : `localStorage['ent_token']`, lu par le JavaScript ; prototype Leptos : idem | `ui.rs` l. 27–28 |
| Surface XSS de l'ancienne interface | **1 973 affectations `innerHTML`**, protégées par une fonction d'échappement maison `esc()` (≈ 3 650 appels) ; pas de `eval` | `ui.rs` |
| En-têtes | `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN` ; **pas de CSP, pas de HSTS** | `ent-api/src/lib.rs` |
| CORS | désactivé par défaut ; `ENT_CORS_ORIGINS` liste d'origines, ou `*` = tout autoriser | `ent-api/src/lib.rs` |
| Fichiers (PDF, iCal) | téléchargés par `fetch` + `Bearer` puis `Blob` (12 usages) ; aucun jeton dans les URL | `ui.rs`, `routes.rs` |
| Applis mobiles | jeton Bearer (Swift / Kotlin) | `ent-rust/apps/` |

Points positifs déjà présents : jeton long et aléatoire, stockage haché, révocation côté serveur, anti-force brute à la connexion, passkey non contournable par le mot de passe ou le SSO.

## 2. Risques de la situation actuelle

1. **Vol du jeton par XSS** : tout script exécuté dans la page lit `localStorage` et peut envoyer le jeton ailleurs. L'attaquant obtient alors **14 jours d'accès** depuis n'importe quelle machine, sans passkey (la session est déjà ouverte). Avec 1 973 insertions `innerHTML`, un seul oubli d'échappement suffit. Les extensions de navigateur malveillantes ont le même accès.
2. **Postes partagés** (salle des professeurs, CDI, accueil) : jeton persistant 14 jours dans le navigateur. Un utilisateur qui oublie de se déconnecter laisse sa session au suivant.
3. **Rendu serveur impossible avec données** : le serveur ne voit pas la session lors de la navigation (D-010).
4. Pas de CSP : rien ne limite les scripts en cas d'injection.

## 3. Cible proposée

### 3.1 Cookie de session

```
Set-Cookie: __Host-ent_session=<jeton opaque>; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=<durée>
```

| Attribut | Rôle | Remarque |
|---|---|---|
| `HttpOnly` | invisible pour JavaScript : un XSS ne peut plus **lire** ni exfiltrer le jeton | n'empêche pas un XSS d'**agir** dans la page (voir §5) |
| `Secure` | envoyé uniquement en HTTPS | en local, Chrome et Firefox acceptent `Secure` sur `http://localhost` / `127.0.0.1` ; option `ENT_COOKIE_SECURE=0` réservée au développement |
| Préfixe `__Host-` | impose `Secure`, `Path=/`, **pas de `Domain`** : le cookie est lié à l'hôte exact | isole naturellement les établissements servis sur des sous-domaines différents (multi-tenant) |
| `SameSite=Lax` | le cookie n'accompagne pas les requêtes d'un autre site, sauf la navigation GET de premier niveau (clic sur un lien) | **recommandé** : un lien reçu par e-mail ouvre l'ENT connecté ; les écritures restent protégées (§4) |
| `SameSite=Strict` | jamais envoyé depuis un autre site | plus strict, mais l'utilisateur qui arrive depuis un e-mail apparaît déconnecté au premier affichage : **à arbitrer** |
| `Max-Age` | durée côté navigateur, alignée sur la durée serveur | la vérité reste en base (expiration, révocation) |

Le jeton reste **le même jeton opaque** (32 octets, haché en base) : seul son transport change. Aucune migration de données.

### 3.2 Durées de session (à décider)
14 jours fixes est long pour des postes partagés. Proposition :
- **web** : durée absolue 12 h + inactivité 2 h (prolongation glissante côté serveur, `last_seen_at`) ; option « appareil personnel » éventuelle plus tard ;
- **applis mobiles** : 30 jours (appareil personnel, déverrouillage par biométrie / passkey), inchangé dans son principe.
Cela demande une colonne `last_seen_at` et une durée par type de client (migration `ent_session`).

### 3.3 Cycle de vie
- Nouveau jeton à **chaque** connexion (déjà le cas) ; en plus : nouveau jeton après ajout d'une passkey ou changement de rôle (évite la fixation de session).
- Déconnexion : révocation en base (existe) **et** `Set-Cookie: __Host-ent_session=; Max-Age=0`.
- Changement de mot de passe : révocation des autres sessions (existe).
- Page « Mes sessions actives » avec révocation à distance (à prévoir avec D-009).

## 4. Protection CSRF

Avec un cookie, le navigateur l'ajoute automatiquement : il faut empêcher un autre site de déclencher une action (CSRF). Défense en profondeur proposée, appliquée aux méthodes qui modifient (`POST`, `PUT`, `PATCH`, `DELETE`) **quand la session vient du cookie** :

1. `SameSite=Lax` (bloque les POST inter-sites dans les navigateurs modernes) ;
2. **contrôle d'origine** : `Origin` doit valoir l'origine publique configurée (`ENT_PUBLIC_ORIGIN`) ; à défaut d'`Origin`, `Sec-Fetch-Site: same-origin` ; sinon refus 403 ;
3. **en-tête obligatoire** `X-ENT-CSRF: 1` sur les appels JSON : un formulaire d'un autre site ne peut pas l'ajouter, et un `fetch` inter-site avec cet en-tête déclenche une vérification CORS que le serveur refuse (pas de CORS avec cookies).

Règles associées :
- **aucune requête GET ne modifie de données** (à auditer route par route ; `POST /appels` est bien un POST) ;
- CORS : jamais `*` avec cookies ; garder CORS désactivé pour le web (même origine) ;
- les requêtes **Bearer** (applis mobiles) ne sont pas concernées : un jeton dans un en-tête n'est jamais ajouté automatiquement par le navigateur.

Les fonctions serveur Leptos envoient des POST du même site : le client Leptos ajoute l'en-tête `X-ENT-CSRF` ; un formulaire Leptos sans JavaScript (amélioration progressive) passe par le contrôle d'origine (point 2).

## 5. XSS : ce que le cookie change, et ce qu'il ne change pas

| | Jeton en `localStorage` | Cookie `HttpOnly` |
|---|---|---|
| Lecture et envoi du jeton vers un autre serveur | **possible** | **impossible** |
| Utilisation hors du navigateur pendant 14 jours | possible | impossible |
| Actions dans la page tant que l'onglet est ouvert | possible | **toujours possible** |

Le cookie réduit fortement l'impact d'un XSS, mais ne dispense pas de l'empêcher :
- **CSP** sur les pages Leptos : `default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'nonce-…'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'self'; base-uri 'none'; form-action 'self'`. Le script d'hydratation de Leptos est en ligne : il faut activer les **nonces** de Leptos (`leptos::nonce`) ;
- Leptos échappe le texte par défaut : **interdiction** d'utiliser `inner_html` avec des données utilisateur (règle à ajouter dans `FRONTEND_LEPTOS.md`) ;
- **HSTS** en production (`Strict-Transport-Security`) ;
- ancienne interface `ui.rs` : son JavaScript en ligne empêche une CSP stricte ; elle reste le principal risque XSS tant qu'elle est en service, ce qui plaide pour la remplacer module par module (D-010).

## 6. Cohabitation web / mobile / ancienne interface

| Client | Transport | CSRF |
|---|---|---|
| Pages Leptos `/web/…` | cookie `__Host-ent_session` | contrôle d'origine + `X-ENT-CSRF` |
| Applis iOS / Android | `Authorization: Bearer` (inchangé) | non applicable |
| Ancienne interface `/app` | `Bearer` depuis `localStorage` (inchangé pendant la transition) | non applicable |

Extracteur `EntSession` : accepte d'abord l'en-tête `Authorization`, sinon le cookie ; il note la provenance (`via_cookie`) pour appliquer le contrôle CSRF. Les deux interfaces web coexistent sans se gêner ; l'ancienne pourra être basculée plus tard ou simplement retirée avec ses modules.

Remise du cookie : les routes de connexion (`/auth/login`, `/auth/passkey/login/finish`, `/auth/oidc/complete`) posent le cookie **et n'incluent pas le jeton dans le corps** quand le client le demande (en-tête `X-ENT-Client: web`, ou routes dédiées `/web/auth/…`) ; sans cette demande, comportement actuel inchangé pour les applis.

## 7. Bénéfice pour Leptos

Avec le cookie, le rendu serveur reçoit la session (`leptos_axum::extract`) :
- pages rendues **avec les données** dès le premier affichage (plus de squelette puis chargement) ;
- fonctions serveur Leptos qui appellent directement les services `ent-db` avec les mêmes contrôles de droits ;
- garde de route côté serveur (redirection vers la connexion sans afficher la page).

## 8. Conception proposée dans le code (à valider)

| Endroit | Changement |
|---|---|
| `ent-api/src/auth.rs` | `EntSession` lit `Authorization` puis le cookie ; champ `via_cookie` |
| `ent-api` (nouveau middleware) | garde CSRF pour les méthodes non sûres quand `via_cookie` |
| routes de connexion / déconnexion | `Set-Cookie` à la connexion (client web), effacement à la déconnexion |
| `ent-api/src/lib.rs` | en-têtes CSP (pages Leptos, avec nonce), HSTS (production), `Referrer-Policy` |
| configuration | `ENT_PUBLIC_ORIGIN`, `ENT_COOKIE_SECURE`, durées web / mobile |
| `ent_session` (migration) | `last_seen_at`, type de client ; durées distinctes |
| `ent-web-app` / `ent-web-api` | plus de `localStorage` ; ajout de `X-ENT-CSRF` ; rendu serveur avec données |

## 9. Tests à écrire

- Attributs du cookie posé (`HttpOnly`, `Secure`, `SameSite`, `__Host-`, `Max-Age`) ; jeton absent du corps de réponse en mode web.
- Requête modifiante avec cookie **sans** `Origin` valide ou sans `X-ENT-CSRF` → 403 ; avec → succès.
- Bearer seul (appli mobile) → fonctionne comme avant, sans contrôle CSRF.
- Déconnexion : session révoquée en base et cookie effacé ; ancien cookie refusé ensuite.
- Expiration par inactivité et durée absolue ; révocation des autres sessions au changement de mot de passe.
- Rendu serveur Leptos avec cookie → données présentes ; sans cookie → redirection vers la connexion.
- En-tête CSP présent, script d'hydratation avec nonce, aucune violation CSP dans la console.
- Non-régression : tests existants d'`ent-api` (22 tests d'intégration) et du prototype.

## 10. Plan proposé

1. **Décision** sur les points du §11.
2. Lot A : extracteur double (en-tête + cookie) + garde CSRF + routes de connexion web + effacement à la déconnexion + tests (backend).
3. Lot B : prototype Leptos basculé sur le cookie, rendu serveur avec données, en-tête CSRF, CSP avec nonce.
4. Lot C : durées de session web / mobile (migration `ent_session`), page des sessions actives.
5. Lot D (plus tard) : bascule ou retrait de l'ancienne interface.

## 11. Décisions

1. `SameSite=Lax` (recommandé) ou `Strict` ? — **à arbitrer** (Lax par défaut).
2. Durées : **décidé (D-011)** — web 2 h d'inactivité / 12 h maximum, à condition d'un parcours de reconnexion rapide ; mobile longue durée avec biométrie.
3. Ancienne interface `/app` : **plus tard** ; elle reste sur `localStorage` d'ici là.
4. Origine publique : **décidé (D-012)** — un sous-domaine par établissement (ex. `lpnb.scolia.fr`) ; `ENT_PUBLIC_ORIGIN` devient une liste d'origines déduite de la table des établissements.

## 12. Compléments (révocation, mobile, passkeys, coexistence)

### 12.1 Expiration et révocation : comportement cible

| Événement | Web (cookie) | Mobile (Bearer) | Existe déjà ? |
|---|---|---|---|
| Inactivité | 2 h → session refusée, reconnexion par-dessus la page | non (session longue) | **non** : ajouter `last_seen_at` |
| Durée maximale | 12 h après la connexion | plusieurs mois, prolongée à l'usage | **partiel** : 14 jours fixes pour tous |
| Déconnexion | révocation en base + cookie effacé | révocation en base | **oui** (révocation) ; effacement du cookie à ajouter |
| Changement de mot de passe | autres sessions révoquées | idem | **oui** |
| Ajout / suppression de passkey, changement de rôle | nouveau jeton pour la session courante, autres sessions révoquées | idem | **non** |
| Compte désactivé / départ | toutes les sessions révoquées | idem | **oui** (`revoke_all_for_user`) |
| Téléphone perdu | — | révocation depuis le web (« Mes appareils ») | **non** : liste des sessions par appareil |
| Purge | sessions expirées supprimées au démarrage | idem | **oui** ; prévoir aussi une purge périodique |

La vérification reste **côté serveur à chaque requête** (`resolve_bearer` lit `ent_session`) : une révocation est effective immédiatement, sans attendre l'expiration du cookie.

### 12.2 Applications mobiles
- Transport inchangé : `Authorization: Bearer`, jeton stocké dans le **Trousseau iOS** / **Keystore Android** (jamais en clair).
- Session longue (D-011) avec **type de client** enregistré (`web` / `mobile`) et nom d'appareil, pour la liste « Mes appareils » et la révocation à distance.
- Déverrouillage biométrique **local** à l'ouverture de l'appli (protège l'accès au jeton sur le téléphone) ; la passkey sert à la connexion initiale et aux actions sensibles.
- Aucune protection CSRF nécessaire (pas de cookie). Le contrôle d'origine du §4 ne s'applique qu'aux sessions venant d'un cookie.
- Passkeys natives : déclarer le domaine `scolia.fr` dans les applis (`webcredentials:scolia.fr` côté iOS, `assetlinks.json` côté Android, publiés sur `https://scolia.fr/.well-known/`). Avec D-013 (passkey liée à `scolia.fr`), **un seul domaine** à déclarer pour tous les établissements.

### 12.3 Passkeys WebAuthn

| Point | Existant | Proposition |
|---|---|---|
| Domaine (`rp_id`) | `ENT_RP_ID` (défaut `localhost`) | `scolia.fr` (D-013) |
| Origines acceptées | **une seule** (`ENT_RP_ORIGIN`) | liste des sous-domaines d'établissements (`WebauthnBuilder::append_allowed_origin`, alimentée par la table des établissements de D-012) ; jamais « tous les sous-domaines » sans liste |
| Rattachement | `ent_passkey_credential (etablissement_id, user_id)` | lier à la **personne** (`user_id`) seulement : une passkey pour tous ses établissements |
| État du défi | stocké en base (JSONB `ent_passkey_challenge.state`), usage unique, expiration courte, jamais envoyé au navigateur ; option `danger-allow-state-serialisation` utilisée correctement | conserver ; ajouter une purge périodique |
| Vérification de l'utilisateur | exigée (`UserVerificationRequired`) | conserver (empreinte / visage / code de l'appareil) |
| Connexion | e-mail d'abord, puis passkey (ou passkey en second facteur) | ajouter la **connexion en un geste** : passkey proposée directement par le navigateur dans le champ e-mail (identifiants « découvrables » + médiation conditionnelle, `start_discoverable_authentication` de `webauthn-rs`) |
| Poste partagé | — | passkey du téléphone par QR code (fonction standard des navigateurs, « autre appareil ») |
| Fin du parcours | jeton renvoyé en JSON | client web : **cookie posé**, jeton absent du corps ; client mobile : inchangé |
| Reconnexion après expiration | — | fenêtre de reconnexion par-dessus la page (passkey), puis reprise de la requête interrompue |

### 12.4 Coexistence avec l'authentification actuelle

| Phase | Ancienne interface `/app` | Pages Leptos `/web` | Applis | Risque |
|---|---|---|---|---|
| Aujourd'hui | Bearer `localStorage` | Bearer `localStorage` (prototype) | Bearer | XSS = vol de jeton |
| Lot A | inchangée | inchangées | inchangées | aucun : le cookie est seulement **accepté** en plus par le serveur |
| Lot B | inchangée | **cookie** + CSRF + CSP | inchangées | les deux interfaces web n'ont plus la même session : passer de l'une à l'autre demande une connexion (acceptable pendant la transition) |
| Lot C | inchangée | durées web | durées mobile | sessions web plus courtes (reconnexion rapide indispensable) |
| Retrait de `/app` | supprimée module par module | seules restantes | inchangées | — |

### 12.5 Conséquences précises (si approuvé)

| Domaine | Changement | Fichiers / objets |
|---|---|---|
| Base | `ent_session` : `last_seen_at`, `client_kind`, `device_label` ; `ent_passkey_credential` : rattachement à la personne ; table des établissements avec domaines (D-012) | nouvelles migrations dans `ent-rust/migrations/` (numéros 091+) |
| Auth | durées par type de client, expiration d'inactivité, nouveau jeton après ajout de passkey / changement de rôle | `ent-auth`, `ent-db/src/auth_repo.rs`, `service.rs` |
| API | extracteur `EntSession` (en-tête puis cookie, `via_cookie`), garde CSRF, routes de connexion web posant le cookie, effacement à la déconnexion, liste « Mes appareils » | `ent-api/src/auth.rs`, `routes.rs`, nouveau middleware |
| WebAuthn | `rp_id = scolia.fr`, liste d'origines, authentification découvrable | `ent-auth/src/passkey.rs`, configuration |
| En-têtes | CSP avec nonce sur `/web`, HSTS en production, `Referrer-Policy` | `ent-api/src/lib.rs`, `ent-web-server` |
| Configuration | `ENT_PUBLIC_ORIGINS` (liste), `ENT_COOKIE_SECURE`, durées web / mobile | variables de déploiement (aucune variable globale de la machine) |
| Frontend | suppression de `localStorage`, en-tête `X-ENT-CSRF`, rendu serveur avec données, fenêtre de reconnexion | `ent-web-app`, `ent-web-api` |
| Applis | inchangées au lot A ; « Mes appareils » et passkeys natives plus tard | `ent-rust/apps/` |
| Tests | voir §9 | `ent-api` (intégration), `ent-web-server/tests` |

## 13. Points complémentaires identifiés (2026-11-03)

Toujours au stade de l'étude : aucune modification d'`ent-auth` ni d'`ent-api`.

| Point | Risque | Proposition |
|---|---|---|
| **Cache du navigateur** sur postes partagés | après déconnexion, le bouton « Précédent » réaffiche une page déjà chargée avec des données d'élèves | `Cache-Control: no-store` sur toutes les réponses authentifiées (API et pages `/web` rendues avec données) ; `Vary: Cookie` |
| **Déconnexion dans tous les onglets** | un onglet resté ouvert continue d'afficher des données | côté serveur la session est révoquée (requêtes suivantes refusées) ; côté navigateur, prévenir les autres onglets (`BroadcastChannel`) et afficher la reconnexion |
| **CSRF à la connexion** (« login CSRF ») | un site tiers connecte la victime à un compte de l'attaquant | le contrôle d'origine du §4 s'applique **aussi** à `POST /auth/login`, aux routes passkey et à `/auth/oidc/complete` |
| **CSRF sur la déconnexion** | déconnexion forcée (gêne) | `POST` uniquement + contrôle d'origine |
| **Clickjacking** | page de l'ENT intégrée dans un cadre malveillant | `frame-ancestors 'self'` dans la CSP (en plus de `X-Frame-Options: SAMEORIGIN` déjà présent) |
| **Fuite du jeton dans les journaux** | jeton écrit dans des logs (en-têtes, corps de réponse) | ne jamais journaliser `Authorization`, `Cookie`, ni le corps des réponses de connexion ; `ent-types` masque déjà jeton et mot de passe dans `Debug` |
| **Sessions sur plusieurs établissements** (D-012, D-013) | confusion de session entre sous-domaines | cookie `__Host-` par sous-domaine ; vérifier à chaque requête que `ent_session.etablissement_id` correspond au domaine appelé |
| **Journal des sessions** | impossible de savoir qui s'est connecté d'où | tracer connexion, déconnexion, expiration, révocation (sans jeton), consultable dans « Mes appareils » et par l'administration |
| **Durée du cookie vs durée serveur** | cookie conservé plus longtemps que la session | `Max-Age` ≤ durée absolue (12 h) ; la vérité reste en base |

Tests à ajouter au §9 : `Cache-Control: no-store` présent sur les réponses authentifiées ; refus d'une connexion venant d'une autre origine ; session d'un établissement refusée sur le sous-domaine d'un autre.

## 14. Prototype isolé (2026-11-03, D-016)

**Aucune modification d'`ent-auth` ni d'`ent-api`.** Crate séparée `ent-rust/web/crates/ent-web-session`. Statut au 2026-11-03 : prototype testé, non branché. **Depuis le 2026-11-05 (§15) : intégré à `ent-web-server`, limites n° 1 (suivi en mémoire) et E3 levées** ; les autres limites restent valables.

### Conception : une passerelle devant l'API existante
| Route du prototype | Rôle |
|---|---|
| `POST /web/session/connexion` | appelle `/auth/login` existant ; si la session est ouverte : **cookie posé, jeton retiré du corps** ; si une passkey est exigée : défi renvoyé, aucun cookie |
| `POST /web/session/passkey/debut` · `/fin` | connexion par passkey sans mot de passe (et fin du second facteur) ; `/fin` pose le cookie |
| `POST /web/session/deconnexion` | `/auth/logout` existant (révocation en base) + effacement du cookie |
| `/web/api/…` | relais vers l'API existante : cookie → `Authorization: Bearer` ; refuse les routes qui émettent un jeton (`/auth/login`, `/auth/register`, `/auth/passkey/login/…`, `/auth/oidc/…`) |
| `/web/_session` | page d'essai, CSP stricte, sans script en ligne |

- Cookie : `__Host-ent_session=…; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age` ≤ 12 h et ≤ expiration réelle en base.
- CSRF (toutes les requêtes qui modifient, **connexion et déconnexion comprises**) : `Origin` dans la liste autorisée (ou, sans `Origin`, `Sec-Fetch-Site: same-origin`) **et** `X-ENT-CSRF: 1`.
- Durées D-011 : inactivité 2 h et maximum 12 h, contrôlés par la passerelle. Une session dépassée est **aussi révoquée en base**.
- Réponses relayées : `Cache-Control: no-store`, `Vary: Cookie`. L'adresse du client est transmise à `ent-api`, ce qui conserve son anti-force brute.
- Le cookie n'est lu **que** sous `/web/…`. Les routes existantes, `/app` et les applis mobiles l'ignorent : leur comportement est inchangé par construction.

### Résultats
**9 tests d'intégration** (`tests/session_cookie.rs`, contre le vrai `ent-api` et PostgreSQL local, sur base existante **et** sur base vierge), tous verts :

| Exigence | Vérifié |
|---|---|
| Connexion mot de passe | cookie aux bons attributs, pas de `Domain`, `Max-Age` ≤ 12 h, **pas de jeton dans le corps**, `no-store` ; mauvais mot de passe → 401 sans cookie |
| Connexion passkey | second facteur (mot de passe → défi, aucun cookie → passkey → cookie) **et** connexion sans mot de passe ; enrôlement par le relais |
| CSRF | connexion refusée sans origine, avec une origine étrangère ou sans en-tête (« login CSRF ») ; écriture relayée refusée dans les mêmes cas ; lecture (GET) acceptée ; fin de passkey sans en-tête refusée |
| Révocation | déconnexion : cookie effacé, ancien cookie refusé **et** jeton refusé en base ; déconnexion forcée depuis un autre site refusée ; changement de mot de passe : autres sessions révoquées, session courante conservée |
| Expiration | inactivité dépassée → 401, cookie effacé, session révoquée en base ; activité régulière refusée après la durée maximale, jamais avant |
| Coexistence Bearer (mobile) | `/auth/login` renvoie toujours le jeton, sans cookie et sans contrôle CSRF ; Bearer accepté |
| Coexistence `/app` / `/web` | `/app` servie comme avant ; le cookie n'ouvre pas les routes existantes ; fermer la session web ne ferme pas celle de `/app`, et inversement |
| Page | CSP stricte, aucun accès à `localStorage`, `sessionStorage` ou `document.cookie` |

**Essai dans un vrai Chrome** (sans fenêtre, 2 onglets, authentificateur WebAuthn virtuel), `_outils-scolia/test-session-navigateur.json` :
- `document.cookie` vide, `localStorage` vide ; cookie `HttpOnly`, `Secure`, `SameSite=Lax`, 12 h (accepté par Chrome sur `http://localhost`) ;
- connexion dans l'onglet A, B est connecté aussitôt ; **déconnexion dans B, A est prévenu en moins de 1 ms** (`BroadcastChannel`) et l'API lui répond 401 ; plus aucun cookie ;
- enrôlement d'une passkey, connexion sans mot de passe, puis mot de passe + passkey : OK ;
- `fetch` forgé sans `X-ENT-CSRF` : 403 ; avec : 200 ;
- aucune exception JavaScript, aucune violation CSP (seules les réponses 401, 403 et 404 attendues).

### Risques et limites
| # | Point | Gravité | Piste |
|---|---|---|---|
| 1 | Suivi d'inactivité **en mémoire** : perdu au redémarrage (tout le monde est déconnecté), et faux avec plusieurs serveurs | bloquant pour la production | colonnes `last_seen_at` / type de client dans `ent_session` (lot C) |
| 2 | Liste des routes **interdites au relais** à tenir à jour : une nouvelle route qui renvoie un jeton serait exposée au JavaScript | élevé | relais par **liste autorisée**, ou contrôle dans `ent-api` (lot A) ; test qui échoue si une réponse relayée contient `token` de session |
| 3 | Une session expirée n'est révoquée en base qu'à la requête suivante ; sinon elle reste valable 14 jours pour un client Bearer qui connaîtrait le jeton | moyen (le jeton n'est jamais exposé au JS) | purge périodique + durée serveur alignée (lot C) |
| 4 | XSS : le cookie empêche le **vol** du jeton, pas les **actions** dans la page ouverte | inhérent | CSP sur les pages Leptos (nonce), règle « pas d'`inner_html` » |
| 5 | Pendant la transition, `/app` et `/web` ont deux sessions distinctes : reconnexion pour passer de l'une à l'autre | faible, attendu (§12.4) | retrait de `/app` module par module |
| 6 | Testé uniquement sous Chrome, sur `localhost`, sans HTTPS réel ni sous-domaines par établissement (D-012) ni `rp_id = scolia.fr` (D-013) | à couvrir | essais Firefox / Safari ; environnement HTTPS de recette avec deux sous-domaines |
| 7 | Rendu serveur Leptos avec données grâce au cookie : non démontré | — | lot B |

### Deux façons de passer en production (décision à prendre)
- **A. Passerelle** (ce prototype, intégrée à `ent-web-server`) : `ent-auth` et `ent-api` restent inchangés ; risque n° 2 à traiter par une liste autorisée ; les durées demandent tout de même une migration (risque n° 1).
- **B. Dans `ent-api`** (§8, lot A) : l'extracteur `EntSession` lit aussi le cookie ; un seul endroit pour les règles, mais il faut modifier l'authentification existante et tester tous ses clients.
- Recommandation : **A pour le transport et le CSRF** (le risque est cantonné à `/web`), **plus** une migration `ent_session` pour les durées. Rien n'est engagé sans ta décision.

## 15. Mise en œuvre A6 (2026-11-05) — cookie `HttpOnly` dans le serveur web et le frontend Leptos

Statut : **implémenté et testé en local** (PostgreSQL jetable, données de labo). Pas « prêt pilote » : voir les limites.

### Ce qui change
| Élément | Avant | Après |
|---|---|---|
| Session du frontend Leptos | jeton Bearer en `localStorage['ent_token']` | cookie `__Host-ent_session` `HttpOnly; Secure; SameSite=Lax`, posé par le relais ; **le WebAssembly n'a jamais le jeton** |
| Appels API du navigateur | `Authorization: Bearer` direct sur `/auth/…` | `/web/api/…` (relais cookie → Bearer), en-tête `X-ENT-CSRF: 1` ; connexion/déconnexion par `/web/session/…` |
| Inactivité / durée maximale | suivi **en mémoire** du relais (perdu au redémarrage, faux en multi-instance) | **la base seule** : `ent_session.last_seen_at`, `inactivite_max_secs`, `expires_at` appliqués par `resolve_bearer`. Le relais n'a plus aucun état. Un 401 efface le cookie |
| Durée du cookie après enrôlement passkey (E3) | figée à celle de la connexion (15 min) | `POST /auth/passkey/register/finish` réussi par le relais → cookie **ré-émis** avec l'expiration lue en base (`GET /auth/sessions`, session `courante`) ≈ 12 h |
| Type de session | le navigateur pouvait réclamer `client: "mobile"` | le relais **impose `client = web`** à la connexion et au début de passkey |
| Montage | prototype autonome | `ent_web_server::assembler` : relais + pages Leptos + `ent_api::app` tel quel (`routeur_complet`) |

Routes du relais : `/web/session/connexion`, `/web/session/passkey/debut`, `/web/session/passkey/fin`, `/web/session/deconnexion`, `/web/session/etat` (nouvelle : `{expire_dans, inactivite_max, client}`, lu en base), `/web/api/{*}`. Le cookie n'est lu que sous `/web/api` et `/web/session` : `/auth/…`, `/app`, les applis mobiles l'ignorent.

Configuration du serveur (`ent_web_server::config_session`) : `ENT_WEB_ORIGINES` (origines publiques, défaut `http://localhost:<port>` et `127.0.0.1`), `ENT_WEB_COOKIE_SECURE=0` (essai local seulement, refusé hors écoute locale), `ENT_SESSION_WEB_MAX_HEURES` / `ENT_SESSION_WEB_INACTIVITE_MINUTES` (**mêmes variables que la base** : plafond du cookie et valeur annoncée ; la base reste seule juge).

### Frontend (Leptos)
- `noyau/session.rs` : `client()` = `ApiClient::relais()` ; plus de `jeton()`, `enregistrer()`, `effacer()`, événement `storage`. Synchro entre onglets par `BroadcastChannel("ent-session")` : messages **sans secret** (`fin`, `ouverte:<id>`, `activite:<t>`).
- **Migration douce** : `purger_ancien_jeton()` (au chargement de la coquille, de la connexion et de l'enrôlement) efface `localStorage['ent_token']` s'il existe et le révoque au mieux côté serveur (`/auth/logout`). C'est le **seul** accès au stockage du navigateur, en lecture/suppression de l'ancienne clé.
- **Reconnexion rapide (D-011)** : `noyau/expiration.rs` (logique pure testée : avertissement à 5 min, marge de prudence de 60 s car le serveur n'écrit l'activité qu'une fois par minute) ; `noyau/reconnexion.rs` : bandeau « votre session expire dans X min — Rester connecté » et **fenêtre de reconnexion** (`<dialog>` modal, mot de passe ou passkey) par-dessus la page quand un appel renvoie 401 ou que l'échéance est dépassée. La page, la saisie et l'URL sont conservées ; après reconnexion les chargements en échec se relancent (`ErreurChargement`). Une action qui a échoué n'est **pas rejouée automatiquement** (appel, message, passkey…) : la personne la refait. Autre compte reconnecté → rechargement complet.
- Les 401 au **premier chargement** (aucun cookie) redirigent toujours vers `/web/connexion?retour=…`.

### Applis mobiles et ancienne UI `/app`
- Mobile : inchangé (Bearer, `/auth/login` renvoie le jeton, pas de CSRF) — testé (`bearer_mobile_et_ancienne_interface_inchanges`).
- `/app` (`ent-api/src/ui.rs`, **figée, non modifiée**) garde son propre jeton dans `localStorage['ent_token']`, qu'elle partageait jusqu'ici avec Leptos. Conséquences : (1) se connecter sur `/web` ne connecte plus `/app` et inversement (déjà le cas du cookie, §12.4) ; (2) **ouvrir une page `/web` efface et révoque le jeton `/app` du même navigateur** (migration douce) — la personne se reconnecte sur `/app`. `/app` conserve donc le risque XSS/`localStorage` décrit en §2 jusqu'à son retrait module par module.

### Tests (état au 2026-11-05)
Voir « Résultats des vérifications » dans `SOCLE1_SECURITE.md` (A6). Couverture : cookie et attributs, CSRF, expiration par la base (inactivité, maximum, **autre instance du relais**), ré-émission après enrôlement (+ pas de cookie sur échec), déconnexion, `client=web` imposé, `etat`, intégration dans le serveur complet, absence de jeton côté frontend (test qui parcourt les sources `ent-web-app` et `ent-web-api` : aucun `local_storage`, `session_storage`, `set_item`, `document.cookie`…), échéances et diffusion entre onglets (tests purs).

### Limites et suite
1. **Non vérifié dans un navigateur réel** dans cette session : la fenêtre de reconnexion (`<dialog>`, passkey), le `BroadcastChannel` du frontend Leptos et l'alerte n'ont que leurs parties pures testées en natif et la compilation wasm. Essai Chrome/Firefox/Safari à faire avant tout pilote (le prototype avait été essayé sous Chrome, §14).
2. `GET /web/session/etat` compte comme une activité (toute requête authentifiée passe par `resolve_bearer`) : le navigateur ne l'appelle qu'au chargement et sur « Rester connecté ». L'inactivité côté page est une **estimation** (dernières réponses reconnues, onglets compris) ; en cas d'écart la base tranche (401 → fenêtre de reconnexion).
3. La session d'**enrôlement** (15 min) ne peut pas lire `/auth/sessions` (403) : pas d'alerte avant expiration pendant l'enrôlement ; l'enrôlement réussi ré-émet bien le cookie.
4. Risque n° 2 du §14 (liste des routes émettrices **interdites** au relais, à tenir à jour) : inchangé ; un test de non-régression « aucune réponse relayée ne contient `token` de session » reste à écrire.
5. Les sous-domaines par établissement (D-012), HTTPS réel et l'adresse WebAuthn de production ne sont pas couverts ici (origines configurables par `ENT_WEB_ORIGINES`).
6. La purge des sessions antérieures à la migration 095 (sans délai d'inactivité) reste à faire au déploiement (voir `SOCLE1_SECURITE.md`).
