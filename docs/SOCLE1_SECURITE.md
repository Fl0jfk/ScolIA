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
| A2 | Obligation de passkey : politique par rôle et date, session « enrôlement seulement » tant qu'aucune passkey, lien d'enrôlement initial sans mot de passe (D-009) | Critique | à faire |
| A3 | Session serveur : inactivité 2 h et maximum 12 h pour le web, longue révocable pour le mobile (D-011), niveau d'assurance (mot de passe / passkey), liste et révocation des appareils | Élevée | à faire |
| A4 | Anti-abus : limitation de débit (oubli de mot de passe, passkey, OIDC, réinitialisation), IP fiable derrière proxy, verrou sans déni de service, réponses indiscernables (énumération), bornes de taille | Élevée | en cours (sous-agent) |
| A5 | WebAuthn fail-closed : `rp_id` `scolia.fr` validé au démarrage, liste d'origines (sous-domaines, applis natives) | Élevée | implémenté + testé en local (natives iOS : voir détail) |
| A6 | Cookie `HttpOnly` en production pour le web (fin du jeton en `localStorage`), sur la base du prototype `ent-web-session` et de A3 | Élevée | à faire |
| A7 | OIDC : `email_verified` obligatoire, state lié au navigateur, URL de découverte contrôlée (SSRF), secret client chiffré | Moyenne | implémenté + testé en local |
| A8 | Récupération : codes de secours, délai et notification sur reset passkey par la direction, alerte si compteur de signature régresse, changement forcé du mot de passe temporaire, liste de mots de passe compromis, `needs_rehash` | Moyenne | partiel : compteur régressé refusé + journalisé, mise à jour atomique ; reste : codes de secours, délai/notification sur reset, mot de passe temporaire, mots de passe compromis, `needs_rehash` |
| A9 | Alerte de nouvelle connexion (nouvel appareil), ré-authentification passkey pour les actions sensibles | Moyenne | à faire |

### B. Rôles et droits
| # | Lot | Gravité | Statut |
|---|---|---|---|
| B1 | Professeur limité à ses élèves (`enseigne_a`) : notes (lecture et saisie), dossier, vie scolaire | Élevée | en cours |
| B2 | Messagerie : un professeur ne lit que les fils où il participe ou qui concernent ses élèves | Élevée | à faire |
| B3 | Justification d'absence réservée à la vie scolaire (et aux responsables autorisés) | Moyenne | à faire |
| B4 | Rattachement d'un responsable : la cible doit avoir le rôle Parent | Moyenne | à faire |
| B5 | Cycle de vie des rôles : changer, retirer, désactiver, historique, révocation des sessions, contrainte sur la colonne `role` | Élevée | à faire |
| B6 | Modèle de capacités (`Capacite`) et fonction unique `autoriser(acteur, action, ressource)` en façade des `can_*` existants | Structurant | à faire |
| B7 | Lien responsable–élève : autorité parentale, dates, suspension (décision de justice), vérifiés en un point | Moyenne | à faire |
| B8 | Accès Admin/Direction aux données psy et santé : à décider avec Florian | Décision | à soumettre |

### C. Isolation des établissements
| # | Lot | Gravité | Statut |
|---|---|---|---|
| C1 | Correctifs ponctuels : filtres manquants (`periode_repo`, `competence_repo`, `update_credential`), `is_member` sur remplaçant EDT et absence RH, `user_labels` filtré par établissement, messages `NotMember`/`EmailTaken` neutres | Moyenne | à faire |
| C2 | Établissement déduit de l'adresse (D-012) : table `etablissement` + domaines, middleware `Host`, session refusée si autre domaine, plus d'`etablissement_id` saisi | Élevée | à faire |
| C3 | Type `Tenant` obtenu seulement de la session, imposé aux accès aux données sensibles et au stockage de fichiers | Structurant | à faire |
| C4 | Row-Level Security PostgreSQL sur les tables les plus sensibles (santé, psy, sanctions, facturation), rôle applicatif non propriétaire | Défense en profondeur | à faire |
| C5 | FK composites `(etablissement_id, id)` sur les tables enfants prioritaires | Défense en profondeur | à faire |
| C6 | Matrice de séparation étendue : personne membre de deux établissements, références croisées en écriture, test statique des requêtes sans `etablissement_id` | Élevée | à faire |

### D. Documents et données
| # | Lot | Gravité | Statut |
|---|---|---|---|
| D1 | URL pré-signées courtes (HMAC en local, Signature V4 S3/Scaleway en production), émises après contrôle des droits ; service des fichiers avec en-têtes anti-exécution | Élevée | en cours (sous-agent) |
| D2 | Contrôle du type réel des fichiers déposés (signature binaire), taille, nom non fiable | Moyenne | à faire |
| D3 | Chiffrement applicatif des champs les plus sensibles (santé, IBAN) | Moyenne | à faire |

### E. Transverse
HSTS et CSP sur l'API, tests de sécurité en CI, journal (rôle de migration séparé, conservation), RGPD (droit d'accès, conservation). À planifier après A–D.

### Détail A5, A7, A8 (implémenté le 2026-11-05)
**A5 — WebAuthn fail-closed** (`ent-auth/src/passkey.rs`, `RpConfig`). Hors labo/test (`ENT_ENV` ∉ `ENVS_NON_PROD`) : `ENT_RP_ID` obligatoire (domaine public, ni `localhost`, ni IP, ni sans point), au moins une origine `https`, toutes les origines web en `https`, sous le `rp_id`, sans chemin/requête ; sinon `PgEnt::connect` échoue au démarrage avec un message explicite (plus de repli silencieux sur `localhost`, plus de `.ok()`). `ENT_RP_ORIGIN` + `ENT_RP_ORIGINS` (virgules) → `append_allowed_origin`. `ENT_RP_ALLOW_SUBDOMAINS=1` → `allow_subdomains` de webauthn-rs 0.5.5 (désactivé par défaut). Origines natives Android `android:apk-key-hash:…` acceptées (comparaison exacte par webauthn-rs). iOS : les associated domains utilisent l'origine `https` du `rp_id` — rien à configurer côté serveur ; le fichier `apple-app-site-association` (webcredentials) est à servir par le frontend/proxy (**non fait**). Version webauthn-rs inchangée (0.5.5).
**A7 — OIDC.** (1) `email_verified == true` exigé pour le rattachement par e-mail ; un compte déjà rattaché par `sub` se connecte sans cette condition. (2) Cookie `__Host-ent_oidc` (`Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/`, 10 min, `no-store`) posé par `POST /auth/oidc/{slug}/start`, hash SHA-256 stocké avec le state (migration 096), exigé et comparé en temps constant par `POST /auth/oidc/complete`, effacé ensuite ; le state est consommé même en cas de refus. (3) Garde SSRF (`ent-oidc/src/ssrf.rs`, transport) : `https` obligatoire, pas d'identifiants dans l'URL, toutes les adresses résolues contrôlées (loopback, privées, link-local dont 169.254.169.254, CGNAT, multicast, IPv6 ULA/mappées/NAT64/6to4…), adresse validée épinglée (anti DNS rebinding), redirections suivies à la main (3 max, revalidées, pas de downgrade, aucune sur POST), réponse bornée (1 Mio) et délai ; vérifié aussi à l'enregistrement du fournisseur (issuer, endpoints, `redirect_uri` en `https`). Assouplie seulement si `ENT_ENV` est un environnement non productif. (4) `client_secret` chiffré AES-256-GCM (OpenSSL, déjà présent — aucune nouvelle dépendance) au format `enc:v1:…`, AAD liée à l'usage, clé `ENT_SECRETS_KEY` obligatoire hors labo/test (démarrage refusé sinon) ; valeurs héritées en clair lues puis rechiffrées (à la connexion et au démarrage) ; jamais dans les journaux, `Debug` ni DTO.
**A8 (partiel).** webauthn-rs 0.5.5 refuse déjà un compteur non croissant (`CredentialPossibleCompromise`) — vérifié par test avec SoftPasskey. Désormais : erreur distincte `PossibleCompromise` (401, message « authentification par passkey refusée… »), trace `warn` et entrée de journal `passkey_compteur_regresse`, et mise à jour du compteur **atomique** (verrou `FOR UPDATE`, relecture, refus si régression, jamais d'abaissement). `apply_counter` n'avale plus l'erreur. Pas de blocage automatique de la clé ni de notification à l'utilisateur (reste à faire).

## Ordre de réalisation
A1 → B1 → (A4, D1 en parallèle) → A2 → A3 → B2–B4 → C1 → C2 → A5 → A6 → B5–B7 → C3–C6 → A7–A9, D2–D3, E.
