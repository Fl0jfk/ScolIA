# Socle 1 — Sécurité : plan et suivi

Ouvert le 2026-11-05 (feuille de route : `docs/ROADMAP.md`). Base : trois audits en lecture seule du 2026-11-05 (authentification, rôles et droits, isolation des établissements) et les vagues précédentes (`docs/SECURITY.md`).
Statuts : **à faire** / **en cours** / **implémenté + testé en local** / **CI verte** / **validé**.

## Critère de sortie
Pouvoir affirmer, tests à l'appui, que :
1. aucun compte ne peut être utilisé sans passkey une fois l'obligation activée, et qu'aucun parcours (mot de passe, SSO, autre établissement) ne la contourne ;
2. chaque rôle n'accède qu'à son périmètre (professeur → ses classes, responsable → ses enfants selon ses droits, élève → lui-même) ;
3. aucune donnée ne traverse les établissements, même en cas d'oubli dans une requête (défense en profondeur) ;
4. les fichiers ne sont accessibles que par des liens signés courts émis après contrôle des droits ;
5. les abus (force brute, bombardement, énumération) sont bornés.

## État des lieux (faits vérifiés le 2026-11-05)
Solide et conservé : Argon2id ; jetons 256 bits stockés hachés ; verrou anti-force brute en base ; challenges passkey à usage unique, liés au compte, état jamais exposé ; plafond de 10 passkeys ; 2FA appliqué au SSO ; OIDC avec state + PKCE + nonce ; session revérifiée à chaque requête avec l'appartenance ; matrice de séparation (30 tests) ; clés de stockage jamais fournies par le client ; journal en ajout seul ; erreurs internes génériques.
Aucune fuite inter-établissements trouvée dans l'échantillon audité — mais l'isolation ne repose que sur la discipline du code (pas de RLS, pas de FK composites, pas de type dédié).

## Lots

### A. Authentification
| # | Lot | Gravité | Statut |
|---|---|---|---|
| A1 | Passkey rattachée à la personne : le 2ᵉ facteur n'était exigé que dans l'établissement d'enrôlement (contournement par un autre établissement ou son SSO) | Critique | implémenté + testé en local (`f80c634`) |
| A2 | Obligation de passkey : politique par rôle et date, session « enrôlement seulement » tant qu'aucune passkey, lien d'enrôlement initial sans mot de passe (D-009) | Critique | implémenté + testé en local (session « enrôlement », politique par rôle ; mot de passe redemandé sauf SSO ; sessions sans passkey fermées à l'activation ; écrans : en cours) |
| A3 | Session serveur : inactivité 2 h et maximum 12 h pour le web, longue révocable pour le mobile (D-011), niveau d'assurance (mot de passe / passkey), liste et révocation des appareils | Élevée | implémenté + testé en local (web 2 h / 12 h, mobile 90 j glissants plafond 1 an, assurance, appareils connectés) |
| A2 | Obligation de passkey : politique par rôle et date, session « enrôlement seulement » tant qu'aucune passkey, lien d'enrôlement initial sans mot de passe (D-009) | Critique | à faire (écrans web : implémentés + testés en local, voir « Écrans frontend ») |
| A3 | Session serveur : inactivité 2 h et maximum 12 h pour le web, longue révocable pour le mobile (D-011), niveau d'assurance (mot de passe / passkey), liste et révocation des appareils | Élevée | à faire (écran « mes appareils » : implémenté + testé en local) |
| A4 | Anti-abus : limitation de débit (oubli de mot de passe, passkey, OIDC, réinitialisation), IP fiable derrière proxy, verrou sans déni de service, réponses indiscernables (énumération), bornes de taille | Élevée | en cours (sous-agent) |
| A5 | WebAuthn fail-closed : `rp_id` `scolia.fr` validé au démarrage, liste d'origines (sous-domaines, applis natives) | Élevée | en cours (session cloud) |
| A6 | Cookie `HttpOnly` en production pour le web (fin du jeton en `localStorage`), sur la base du prototype `ent-web-session` et de A3 | Élevée | à faire |
| A7 | OIDC : `email_verified` obligatoire, state lié au navigateur, URL de découverte contrôlée (SSRF), secret client chiffré | Moyenne | en cours (session cloud) |
| A8 | Récupération : codes de secours, délai et notification sur reset passkey par la direction, alerte si compteur de signature régresse, changement forcé du mot de passe temporaire, liste de mots de passe compromis, `needs_rehash` | Moyenne | à faire |
| A5 | WebAuthn fail-closed : `rp_id` `scolia.fr` validé au démarrage, liste d'origines (sous-domaines, applis natives) | Élevée | implémenté + testé en local (natives iOS : voir détail) |
| A6 | Cookie `HttpOnly` en production pour le web (fin du jeton en `localStorage`), sur la base du prototype `ent-web-session` et de A3 | Élevée | à faire |
| A7 | OIDC : `email_verified` obligatoire, state lié au navigateur, URL de découverte contrôlée (SSRF), secret client chiffré | Moyenne | implémenté + testé en local |
| A8 | Récupération : codes de secours, délai et notification sur reset passkey par la direction, alerte si compteur de signature régresse, changement forcé du mot de passe temporaire, liste de mots de passe compromis, `needs_rehash` | Moyenne | partiel : compteur régressé refusé + journalisé, mise à jour atomique ; reste : codes de secours, délai/notification sur reset, mot de passe temporaire, mots de passe compromis, `needs_rehash` |
| A9 | Alerte de nouvelle connexion (nouvel appareil), ré-authentification passkey pour les actions sensibles | Moyenne | à faire |

### B. Rôles et droits
| # | Lot | Gravité | Statut |
|---|---|---|---|
| B1 | Professeur limité à ses élèves (`enseigne_a`) : notes (lecture et saisie), dossier, vie scolaire | Élevée | implémenté + testé en local (règle unique `prof_suit_eleve` : notes, saisie, absences, dossier, journal, compétences, livret, retards, sorties, dispenses, alertes, conseils, infos, vœux, conventions, inscription, listes VS) |
| B2 | Messagerie : un professeur ne lit que les fils où il participe ou qui concernent ses élèves | Élevée | implémenté + testé en local (fils des élèves suivis uniquement) |
| B3 | Justification d'absence réservée à la vie scolaire (et aux responsables autorisés) | Moyenne | implémenté + testé en local (vie scolaire, responsable autorisé, élève majeur) |
| B4 | Rattachement d'un responsable : la cible doit avoir le rôle Parent | Moyenne | à faire |
| B5 | Cycle de vie des rôles : changer, retirer, désactiver, historique, révocation des sessions, contrainte sur la colonne `role` | Élevée | R1 : attribution et retrait datés de rôles paramétrables, journalisés (D-017) ; désactivation de compte et contrainte sur `role` : à faire |
| B6 | Modèle de capacités (`Capacite`) et fonction unique `autoriser(acteur, action, ressource)` en façade des `can_*` existants | Structurant | R1 implémenté + testé en local : catalogue `ent_auth::Action`/`Portee`, rôles par établissement, cumul, anti-escalade, moteur `autoriser_eleve` / `eleves_autorises` branché sur notes, saisie, compétences, absences, dossier, journal, messagerie, suivi, listes VS (D-017, `docs/ROLES_ET_DROITS.md`) ; R2 (autres domaines, écrans) : à faire |
| B7 | Lien responsable–élève : autorité parentale, dates, suspension (décision de justice), vérifiés en un point | Moyenne | à faire |
| B8 | Accès Admin/Direction aux données psy et santé : à décider avec Florian | Décision | à soumettre |

### C. Isolation des établissements
| # | Lot | Gravité | Statut |
|---|---|---|---|
| C1 | Correctifs ponctuels : filtres manquants (`periode_repo`, `competence_repo`, `update_credential`), `is_member` sur remplaçant EDT et absence RH, `user_labels` filtré par établissement, messages `NotMember`/`EmailTaken` neutres | Moyenne | implémenté + testé en local (détail « C1 / C6 » ci-dessous ; reste : invitation de rattachement, parcours parent, `update_credential`) |
| C2 | Établissement déduit de l'adresse (D-012) : table `etablissement` + domaines, middleware `Host`, session refusée si autre domaine, plus d'`etablissement_id` saisi | Élevée | en cours (session cloud) |
| C2 | Établissement déduit de l'adresse (D-012) : table `etablissement` + domaines, middleware `Host`, session refusée si autre domaine, plus d'`etablissement_id` saisi | Élevée | implémenté + testé en local (détail ci-dessous) |
| C3 | Type `Tenant` obtenu seulement de la session, imposé aux accès aux données sensibles et au stockage de fichiers | Structurant | à faire |
| C4 | Row-Level Security PostgreSQL sur les tables les plus sensibles (santé, psy, sanctions, facturation), rôle applicatif non propriétaire | Défense en profondeur | à faire |
| C5 | FK composites `(etablissement_id, id)` sur les tables enfants prioritaires | Défense en profondeur | à faire |
| C6 | Matrice de séparation étendue : personne membre de deux établissements, références croisées en écriture, test statique des requêtes sans `etablissement_id` | Élevée | implémenté + testé en local (matrice : 40 tests, 2292 cas ; garde statique ; détail ci-dessous) |

#### C2 — détail (2026-11-05, branche `ent-socle1-tenant-host`)
**Faits (implémenté + testé en local) :**
- migration `096_etablissement_domaines.sql` : `ent_etablissement (id, slug unique, nom, actif, created_at)` et `ent_etablissement_domaine (domaine PK en minuscules, etablissement_id FK)` ; additive, aucune FK posée sur les tables existantes ;
- `ent-db/src/etablissement_repo.rs` : `normaliser_domaine` (minuscules, sans port ni point final, refus des valeurs invalides : IPv6, `_`, accents non punycode, étiquettes vides ou > 63) ; `resoudre_domaine` (établissements actifs) ; administration **Admin seulement, limitée à l'établissement de la session**, journalisée (`etablissement_declare`, `etablissement_domaine_ajoute`, `etablissement_domaine_retire`) ; un domaine d'un autre établissement n'est jamais réattribué (409) ;
- `ent-api/src/tenant.rs` : middleware + extracteur `TenantHote` ; `Host`, ou `X-Forwarded-Host` seulement si `ENT_TRUST_PROXY=1` ; `ENT_TENANT_PAR_HOTE=strict|souple` (défaut `souple`) ;
- appliqué aux 5 routes publiques (`/auth/login`, `/auth/passkey/login/start`, `/auth/oidc/public`, `/auth/oidc/{slug}/start`, `/auth/register` labo) — `etablissement_id` devient facultatif et ignoré si l'hôte est connu —, à l'extracteur `EntSession` (session d'un autre établissement → 401) et aux fins de parcours `/auth/passkey/login/finish` et `/auth/oidc/complete` (aucun jeton remis si le parcours a commencé sur une autre adresse) ;
- nouvelles routes : `GET /auth/etablissement` (public), `PUT /etablissement/fiche`, `GET|POST /etablissement/domaines`, `DELETE /etablissement/domaines/{domaine}` ;
- connexion d'un non-membre : même réponse (401, même message) que des identifiants invalides ; l'échec reste journalisé (`connexion_echec`) ;
- tests : `ent-api/tests/tenant_hote.rs` (8 tests : établissement imposé malgré le corps, hôte inconnu → 404 en strict avec sondes exemptées, session de A sur l'hôte de B → 401 en strict et en souple, souple = comportement antérieur, normalisation, `X-Forwarded-Host` ignoré sans proxy de confiance, non-membre, administration) ; tests unitaires de normalisation dans `ent-db`.

**Limites / reste à faire :**
- en `souple` (défaut), un hôte inconnu garde l'ancien comportement : passer en `strict` en production après déclaration des domaines ;
- une résolution en base par requête (clé primaire, sans cache) ;
- l'attribution d'un domaine est faite par l'Admin de l'établissement : pas encore de rôle « plateforme » ni de liste de suffixes autorisés (ex. `*.scolia.fr`) ; un Admin pourrait réserver un domaine non encore déclaré par un autre établissement (refus seulement s'il est déjà pris) — à encadrer avant la production ;
- `/auth/logout`, `/auth/forgot-password`, `/auth/reset-password` ne sont pas encore liés à l'hôte ; le cookie `__Host-` (A6) et la liste d'origines WebAuthn (A5) restent à faire ;
- l'interface JavaScript existante envoie encore `etablissement_id` (accepté, ignoré sur une adresse connue) : à retirer lors du remplacement Leptos.

#### C1 (reste) et C6 — détail (2026-11-05, branche `ent-socle1-matrice`)
**Faits (implémenté + testé en local) :**
- `user_labels(etablissement_id, ids)` (`ent-db`, `PgEnt`) : jointure `ent_membership` ; l'établissement de la session est passé aux 17 appels (`routes.rs`, `referentiel.rs`) ; un identifiant d'un autre établissement n'a plus de libellé (test `c1_user_labels_filtre_par_etablissement`).
- `provisionner_personnel_for_actor` : adresse déjà membre **de l'établissement** → 409 (inchangé) ; adresse connue **ailleurs** → réponse indiscernable d'un succès (201, même forme, même coût Argon2id), **aucun compte créé ni modifié**, entrée de journal `personnel_provisionne_adresse_connue` (sans l'adresse) ; le mot de passe renvoyé n'est rattaché à rien. Le rattachement réel par **invitation acceptée par la personne** reste **à concevoir** (non implémenté).
- SSO : une personne authentifiée par l'IdP mais non membre répond comme un compte inconnu (`OidcCompteInconnu`, 401), plus `Forbidden`.
- Connexion d'un non-membre et mot de passe oublié : vérifiés déjà indiscernables (L5/A4) ; passkey `login/start` : défi factice indiscernable.
- **Défauts trouvés par la matrice et corrigés** : (1) `POST /edt/creneaux/{id}/affectation` acceptait un `prof_user_id` d'un autre établissement (200) → membre exigé ; (2) `POST /facturation/factures` acceptait une ligne dont `eleve_id` est d'un autre établissement (201) → élève de l'établissement exigé ; (3) les traces de consultation sensible (`fiche_sante`, `pai`, `dossier_psy`, `entretiens_psy`, `passages_infirmerie`) et toute entrée de journal visant un `eleve_id` d'un autre établissement étaient écrites dans le journal de l'établissement de la session (et listées par `GET /eleves/{id}/journal`) → `ent_journal_modif` n'insère plus (ni événement) qu'un élève de l'établissement.
- Inventaire des routes (`tests/separation/routes_src.rs`) : il ne lisait que `routes.rs` ; `referentiel.rs` et `roles.rs` (donc `/roles/*`, `/referentiel/groupes/{id}/responsables`) échappaient au test « 401 sans session ». Les trois fichiers sont lus et un test échoue si un autre fichier de `src/` déclare des routes.

**Matrice étendue (`ent-api/tests/separation.rs`) :** 40 tests (30 auparavant), **2292 cas exécutés** (dont 519 dans les cinq tests C6 : double membre 304, références croisées 13, `*_user_id` 13, routes du socle 1 189 ; l'inventaire « 401 » passe de 2 × ~250 à 2 × 348 routes, `referentiel.rs` et `roles.rs` inclus) ; énumération : 12 comparaisons.
- double membre : 8 comptes neufs (admin, direction, administratif, CPE, professeur, compta, infirmerie, psy) membres de A **et** de B, connectés sur A ; toutes les spécifications de la matrice rejouées avec des identifiants de B (chemin, query, corps) ; listes de A sans identifiant ni e-mail de B ; sessions, politique et domaines de A seulement ;
- références croisées en écriture (session A, parent de B) : note (matière, période, devoir), devoir (matière, période), appel (élève, créneau), message (fil, réponse), facture (foyer, ligne), document (dépôt, décision de grant) → refus 4xx, rien d'écrit dans B ;
- colonnes `*_user_id` du client : remplaçant EDT, affectation de créneau, contrainte EDT, absence RH, service d'enseignement, responsable de groupe, attribution / retrait / modification de rôle (dans les deux sens) ; la route de demande d'accès à un document n'accepte aucun bénéficiaire du client (le bénéficiaire est la session) ;
- énumération : connexion (bon mot de passe d'un compte de B ≡ compte inconnu ≡ mauvais mot de passe), passkey, mot de passe oublié, provisionnement ;
- nouvelles routes : `/roles/*`, `/auth/sessions*`, `/etablissement/securite|domaines|fiche`, les 4 routes `…/lien`, `/fichiers/{jeton}` (jeton inconnu ou altéré) : 401 sans session, refus des identifiants de B pour tout le personnel de A et les doubles membres, session de B non révocable depuis A. Le tenant par hôte reste couvert par `tests/tenant_hote.rs` (C2).

**Garde statique (`ent-db/tests/isolation_sql_statique.rs`) :** déduit des migrations 124 tables portant `etablissement_id` et contrôle les littéraux SQL de `ent-db/src/*.rs` (hors `tests_*`) : tout `SELECT/UPDATE/DELETE … FROM|UPDATE|JOIN ent_x` sur ces tables doit contenir `etablissement_id` dans le même littéral. 430 requêtes contrôlées au premier passage, 26 sans filtre : **aucune n'était une fuite**, toutes justifiées une à une dans `EXCEPTIONS` (sessions, passkeys, états OIDC : clé = personne ou secret global ; purges et métriques globales ; chiffrement des secrets au démarrage). Une exception inutilisée fait échouer le test (pas de dette silencieuse).

**Limites / reste à faire :**
- le garde est syntaxique : il ne voit pas une requête construite par concaténation hors littéral, ni un `etablissement_id` présent dans le littéral mais seulement dans un `SET` ; il ne remplace ni la RLS (C4) ni les FK composites (C5) ;
- parcours de rattachement d'un parent (`POST /foyers/{id}/liens`) et provisionnement en lot : une adresse déjà utilisée dans un autre établissement donne encore un 409 distinct (oracle d'existence) ; même traitement neutre à prévoir avec l'invitation ;
- `update_credential` (C1) : non re-audité dans ce passage ;
- les comptes parents / élèves n'ont pas été rejoués comme doubles membres (ils reçoivent un secret aléatoire, sans mot de passe de test) ; leurs routes exigent un lien par élève, refusé hors établissement dans la matrice standard ;
- le temps de réponse du provisionnement d'une adresse connue ailleurs n'est égalé qu'au coût d'Argon2id, non mesuré.

### D. Documents et données
| # | Lot | Gravité | Statut |
|---|---|---|---|
| D1 | URL pré-signées courtes (HMAC en local, Signature V4 S3/Scaleway en production), émises après contrôle des droits ; service des fichiers avec en-têtes anti-exécution | Élevée | implémenté + testé en local (`7b1797d`, 4 routes `…/lien`, `GET /fichiers/{jeton}`) |
| D2 | Contrôle du type réel des fichiers déposés (signature binaire), taille, nom non fiable | Moyenne | à faire |
| D3 | Chiffrement applicatif des champs les plus sensibles (santé, IBAN) | Moyenne | à faire |

### E. Transverse
HSTS et CSP sur l'API, tests de sécurité en CI, journal (rôle de migration séparé, conservation), RGPD (droit d'accès, conservation). À planifier après A–D.

### Détail A5, A7, A8 (implémenté le 2026-11-05)
**A5 — WebAuthn fail-closed** (`ent-auth/src/passkey.rs`, `RpConfig`). Hors labo/test (`ENT_ENV` ∉ `ENVS_NON_PROD`) : `ENT_RP_ID` obligatoire (domaine public, ni `localhost`, ni IP, ni sans point), au moins une origine `https`, toutes les origines web en `https`, sous le `rp_id`, sans chemin/requête ; sinon `PgEnt::connect` échoue au démarrage avec un message explicite (plus de repli silencieux sur `localhost`, plus de `.ok()`). `ENT_RP_ORIGIN` + `ENT_RP_ORIGINS` (virgules) → `append_allowed_origin`. `ENT_RP_ALLOW_SUBDOMAINS=1` → `allow_subdomains` de webauthn-rs 0.5.5 (désactivé par défaut). Origines natives Android `android:apk-key-hash:…` acceptées (comparaison exacte par webauthn-rs). iOS : les associated domains utilisent l'origine `https` du `rp_id` — rien à configurer côté serveur ; le fichier `apple-app-site-association` (webcredentials) est à servir par le frontend/proxy (**non fait**). Version webauthn-rs inchangée (0.5.5).
**A7 — OIDC.** (1) `email_verified == true` exigé pour le rattachement par e-mail ; un compte déjà rattaché par `sub` se connecte sans cette condition. (2) Cookie `__Host-ent_oidc` (`Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/`, 10 min, `no-store`) posé par `POST /auth/oidc/{slug}/start`, hash SHA-256 stocké avec le state (migration 096), exigé et comparé en temps constant par `POST /auth/oidc/complete`, effacé ensuite ; le state est consommé même en cas de refus. (3) Garde SSRF (`ent-oidc/src/ssrf.rs`, transport) : `https` obligatoire, pas d'identifiants dans l'URL, toutes les adresses résolues contrôlées (loopback, privées, link-local dont 169.254.169.254, CGNAT, multicast, IPv6 ULA/mappées/NAT64/6to4…), adresse validée épinglée (anti DNS rebinding), redirections suivies à la main (3 max, revalidées, pas de downgrade, aucune sur POST), réponse bornée (1 Mio) et délai ; vérifié aussi à l'enregistrement du fournisseur (issuer, endpoints, `redirect_uri` en `https`). Assouplie seulement si `ENT_ENV` est un environnement non productif. (4) `client_secret` chiffré AES-256-GCM (OpenSSL, déjà présent — aucune nouvelle dépendance) au format `enc:v1:…`, AAD liée à l'usage, clé `ENT_SECRETS_KEY` obligatoire hors labo/test (démarrage refusé sinon) ; valeurs héritées en clair lues puis rechiffrées (à la connexion et au démarrage) ; jamais dans les journaux, `Debug` ni DTO.
**A8 (partiel).** webauthn-rs 0.5.5 refuse déjà un compteur non croissant (`CredentialPossibleCompromise`) — vérifié par test avec SoftPasskey. Désormais : erreur distincte `PossibleCompromise` (401, message « authentification par passkey refusée… »), trace `warn` et entrée de journal `passkey_compteur_regresse`, et mise à jour du compteur **atomique** (verrou `FOR UPDATE`, relecture, refus si régression, jamais d'abaissement). `apply_counter` n'avale plus l'erreur. Pas de blocage automatique de la clé ni de notification à l'utilisateur (reste à faire).
## Écrans frontend du socle 1 (2026-11-05)
Branche `ent-socle1-ecrans-securite`, module `ent-rust/web/crates/ent-web-app/src/modules/securite/`. Backend **non modifié** : les écrans s'appuient sur les routes décrites ci-dessous.

| Écran | Route web | Routes `ent-api` utilisées | Statut |
|---|---|---|---|
| Enrôlement obligatoire d'une passkey (écran plein, hors coquille, seule sortie : « Se déconnecter ») | `/web/securite/enrolement` | `GET /auth/me` (`portee`), `POST /auth/passkey/register/start` et `/finish` (mot de passe vide accepté en session « enrôlement »), `POST /auth/logout` | implémenté + testé en local (unitaires, rendu serveur, contre le backend, Chrome avec authentificateur WebAuthn virtuel à 375 px : redirection, contournement refusé, création de passkey, retour à l'accueil) |
| Sécurité du compte : niveau de la session, mes passkeys (liste, ajout et suppression avec mot de passe actuel), mes appareils (liste, « cet appareil », déconnexion confirmée) | `/web/securite` | `GET /auth/me`, `GET`/`DELETE /auth/passkeys`, `GET`/`DELETE /auth/sessions` | implémenté + testé en local (idem ; ajout, suppression avec mauvais puis bon mot de passe, déconnexion d'un autre appareil vérifiés au navigateur) |
| Politique de sécurité (cases par rôle, enregistrement confirmé) | `/web/direction/securite` (lien depuis « Compte » pour Admin et Direction) | `GET`/`PUT /etablissement/securite` | implémenté + testé en local (idem ; confirmation, enregistrement, refus à un professeur vérifiés au navigateur) |

Comportement de redirection (`noyau/portee.rs`, testé) : session « enrôlement » → toute page de la coquille redirige vers l'écran d'enrôlement et ne charge aucune donnée ; un 403 `passkey_enrolement_requis` reçu par un écran déjà ouvert redirige de même ; session complète sur l'écran d'enrôlement → accueil. Le serveur reste l'autorité : contourner l'écran ne donne accès à rien.

Constats (2026-11-05) :
- le serveur calcule la portée **à l'ouverture de la session** : supprimer la dernière passkey d'un rôle obligatoire ne bascule pas la session en cours en « enrôlement » ; l'obligation s'applique à la prochaine connexion (l'écran l'annonce ainsi) ;
- **API manquante / divergence** : `ent_types::auth::MeDto` n'a pas `portee`, `assurance`, `client`, `passkey_obligatoire` alors que `GET /auth/me` les renvoie ; le test `contre_backend::contrat_json_ent_types_lot1` échoue donc sur cette route (ancien test de contrat, non lié aux écrans). Correction côté `ent-types` (hors périmètre de cette branche). Le frontend décode ces champs dans `ent_web_api::MoiSecurite` en attendant ;
- aucune route de renommage de passkey.

À faire avant « CI verte » / « validé » : CI complète, axe-core et lecteur d'écran, essais Firefox / Safari (passkey), appareils mobiles réels.

## Ordre de réalisation
A1 → B1 → (A4, D1 en parallèle) → A2 → A3 → B2–B4 → C1 → C2 → A5 → A6 → B5–B7 → C3–C6 → A7–A9, D2–D3, E.

## Revue adverse du 2026-11-05 (lecture seule, sous-agent indépendant)
Vérifié non exploitable : contournement de la session d'enrôlement par variante de chemin ou relais `/web/api` ; session complète sans passkey par SSO, mot de passe, passkey, réinitialisation ou labo ; prolongation d'une session expirée ; révocation de la session d'autrui ; injection SQL (paramètres liés).
Corrigé : E1 périmètre professeur incomplet (≈ 12 routes) ; E2 enrôlement sans mot de passe abusable avec un jeton d'enrôlement volé (mot de passe redemandé sauf SSO, e-mail d'alerte à chaque nouvelle passkey) ; E5 politique imposée sans moteur WebAuthn et sessions existantes non fermées ; E6 accès aux fils acquis par participation ; E10 tests (fenêtre de remplacement, variantes de chemin, prolongation mobile).
Restent (décisions ou suites) :
- **E3** le cookie du prototype `ent-web-session` garde la durée de la session d'enrôlement (15 min) : à traiter lors du passage du cookie en production (A6).
- **E4** récupération d'une personne membre de deux établissements qui perd sa passkey : aucune direction ne peut agir (isolation) → **décision à prendre** avant d'activer l'obligation (support plateforme journalisé, ou récupération en libre-service avec délai et notification).
- **E7** le type de client `mobile` est déclaré par le client (90 j) : à lier à l'appli (attestation d'appareil) avec le chantier mobile.
- **E8** appareil et IP non conservés pour une connexion par passkey ; libellé d'appareil (User-Agent) à échapper à l'affichage.
- **E9** créneaux EDT historiques sans date de fin (accès conservé tant que le créneau existe) ; comparaison de classe sensible à la casse ; jour UTC.
- Sessions antérieures à la migration 095 sans délai d'inactivité : à purger au déploiement.
