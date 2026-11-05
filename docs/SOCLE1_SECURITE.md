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
| A4 | Anti-abus : limitation de débit (oubli de mot de passe, passkey, OIDC, réinitialisation), IP fiable derrière proxy, verrou sans déni de service, réponses indiscernables (énumération), bornes de taille | Élevée | en cours (sous-agent) |
| A5 | WebAuthn fail-closed : `rp_id` `scolia.fr` validé au démarrage, liste d'origines (sous-domaines, applis natives) | Élevée | en cours (session cloud) |
| A6 | Cookie `HttpOnly` en production pour le web (fin du jeton en `localStorage`), sur la base du prototype `ent-web-session` et de A3 | Élevée | à faire |
| A7 | OIDC : `email_verified` obligatoire, state lié au navigateur, URL de découverte contrôlée (SSRF), secret client chiffré | Moyenne | en cours (session cloud) |
| A8 | Récupération : codes de secours, délai et notification sur reset passkey par la direction, alerte si compteur de signature régresse, changement forcé du mot de passe temporaire, liste de mots de passe compromis, `needs_rehash` | Moyenne | **partiel** : récupération de passkey en libre-service par e-mail (D-018) implémentée + testée en local (migration 097, voir ci-dessous) ; reste : codes de secours, délai/notification sur le reset par la direction, compteur de signature, mot de passe temporaire, mots de passe compromis, `needs_rehash` |
| A9 | Alerte de nouvelle connexion (nouvel appareil), ré-authentification passkey pour les actions sensibles | Moyenne | à faire |

### B. Rôles et droits
| # | Lot | Gravité | Statut |
|---|---|---|---|
| B1 | Professeur limité à ses élèves (`enseigne_a`) : notes (lecture et saisie), dossier, vie scolaire | Élevée | implémenté + testé en local (règle unique `prof_suit_eleve` : notes, saisie, absences, dossier, journal, compétences, livret, retards, sorties, dispenses, alertes, conseils, infos, vœux, conventions, inscription, listes VS) |
| B2 | Messagerie : un professeur ne lit que les fils où il participe ou qui concernent ses élèves | Élevée | implémenté + testé en local (fils des élèves suivis uniquement) |
| B3 | Justification d'absence réservée à la vie scolaire (et aux responsables autorisés) | Moyenne | implémenté + testé en local (vie scolaire, responsable autorisé, élève majeur) |
| B4 | Rattachement d'un responsable : la cible doit avoir le rôle Parent | Moyenne | à faire |
| B5 | Cycle de vie des rôles : changer, retirer, désactiver, historique, révocation des sessions, contrainte sur la colonne `role` | Élevée | à faire |
| B6 | Modèle de capacités (`Capacite`) et fonction unique `autoriser(acteur, action, ressource)` en façade des `can_*` existants | Structurant | à faire |
| B7 | Lien responsable–élève : autorité parentale, dates, suspension (décision de justice), vérifiés en un point | Moyenne | à faire |
| B8 | Accès Admin/Direction aux données psy et santé : à décider avec Florian | Décision | à soumettre |

### C. Isolation des établissements
| # | Lot | Gravité | Statut |
|---|---|---|---|
| C1 | Correctifs ponctuels : filtres manquants (`periode_repo`, `competence_repo`, `update_credential`), `is_member` sur remplaçant EDT et absence RH, `user_labels` filtré par établissement, messages `NotMember`/`EmailTaken` neutres | Moyenne | partiel (filtres périodes et compétences, remplaçant et absence RH membres ; `user_labels` et messages neutres : à faire) |
| C2 | Établissement déduit de l'adresse (D-012) : table `etablissement` + domaines, middleware `Host`, session refusée si autre domaine, plus d'`etablissement_id` saisi | Élevée | en cours (session cloud) |
| C3 | Type `Tenant` obtenu seulement de la session, imposé aux accès aux données sensibles et au stockage de fichiers | Structurant | à faire |
| C4 | Row-Level Security PostgreSQL sur les tables les plus sensibles (santé, psy, sanctions, facturation), rôle applicatif non propriétaire | Défense en profondeur | à faire |
| C5 | FK composites `(etablissement_id, id)` sur les tables enfants prioritaires | Défense en profondeur | à faire |
| C6 | Matrice de séparation étendue : personne membre de deux établissements, références croisées en écriture, test statique des requêtes sans `etablissement_id` | Élevée | à faire |

### D. Documents et données
| # | Lot | Gravité | Statut |
|---|---|---|---|
| D1 | URL pré-signées courtes (HMAC en local, Signature V4 S3/Scaleway en production), émises après contrôle des droits ; service des fichiers avec en-têtes anti-exécution | Élevée | implémenté + testé en local (`7b1797d`, 4 routes `…/lien`, `GET /fichiers/{jeton}`) |
| D2 | Contrôle du type réel des fichiers déposés (signature binaire), taille, nom non fiable | Moyenne | à faire |
| D3 | Chiffrement applicatif des champs les plus sensibles (santé, IBAN) | Moyenne | à faire |

### E. Transverse
HSTS et CSP sur l'API, tests de sécurité en CI, journal (rôle de migration séparé, conservation), RGPD (droit d'accès, conservation). À planifier après A–D.

## Ordre de réalisation
A1 → B1 → (A4, D1 en parallèle) → A2 → A3 → B2–B4 → C1 → C2 → A5 → A6 → B5–B7 → C3–C6 → A7–A9, D2–D3, E.

## Revue adverse du 2026-11-05 (lecture seule, sous-agent indépendant)
Vérifié non exploitable : contournement de la session d'enrôlement par variante de chemin ou relais `/web/api` ; session complète sans passkey par SSO, mot de passe, passkey, réinitialisation ou labo ; prolongation d'une session expirée ; révocation de la session d'autrui ; injection SQL (paramètres liés).
Corrigé : E1 périmètre professeur incomplet (≈ 12 routes) ; E2 enrôlement sans mot de passe abusable avec un jeton d'enrôlement volé (mot de passe redemandé sauf SSO, e-mail d'alerte à chaque nouvelle passkey) ; E5 politique imposée sans moteur WebAuthn et sessions existantes non fermées ; E6 accès aux fils acquis par participation ; E10 tests (fenêtre de remplacement, variantes de chemin, prolongation mobile).
Restent (décisions ou suites) :
- **E3** le cookie du prototype `ent-web-session` garde la durée de la session d'enrôlement (15 min) : à traiter lors du passage du cookie en production (A6).
- **E4** récupération d'une personne membre de deux établissements qui perd sa passkey : aucune direction ne peut agir (isolation) → **traité** par D-018 (récupération en libre-service avec délai et alertes, lot A8, voir « Lot A8 — récupération de passkey »). Implémenté + testé en local ; non validé pilote.
- **E7** le type de client `mobile` est déclaré par le client (90 j) : à lier à l'appli (attestation d'appareil) avec le chantier mobile.
- **E8** appareil et IP non conservés pour une connexion par passkey ; libellé d'appareil (User-Agent) à échapper à l'affichage.
- **E9** créneaux EDT historiques sans date de fin (accès conservé tant que le créneau existe) ; comparaison de classe sensible à la casse ; jour UTC.
- Sessions antérieures à la migration 095 sans délai d'inactivité : à purger au déploiement.

## Lot A8 (partie D-018) — récupération de passkey en libre-service

Niveau : **implémenté + testé en local** (`ent-api` `tests_recuperation`, `ent-db`) ; non validé pilote ; non prêt production. Pages web (liens des e-mails) : à écrire en Leptos, l'API est prête.

**Parcours** (une demande active par personne ; table volatile `ent_recuperation_passkey`, migration 097) :
1. `POST /auth/passkey/recuperation` `{email, etablissement_id}` — public, **202 toujours** (compte inconnu, sans passkey, hors établissement ou valide : indiscernable ; traitement et envoi différés). Si éligible : demande `demandee` + e-mail avec lien de **confirmation** (jeton 256 bits, haché, usage unique, 60 min). Une demande non confirmée est remplacée par la suivante ; une demande déjà « en attente » n'est jamais écrasée.
2. `POST /auth/passkey/recuperation/confirmer` `{token}` — la demande passe `en_attente`, **délai de sécurité** `ENT_RECUPERATION_PASSKEY_DELAI_HEURES` (défaut 24, borné 1–168). E-mail d'alerte à la personne avec un lien d'**annulation** et un lien de **finalisation** (deux jetons distincts, hachés). Événement `passkey_recuperation_en_attente` journalisé **dans chaque établissement** de la personne, sans mention des autres (D-013) ni e-mail en clair (empreinte).
3. Annulation : `POST …/annuler` `{token}` (lien d'annulation) **ou** connexion réussie par passkey pendant le délai (`passkey_login_finish`) → `annulee`, événement `passkey_recuperation_annulee` dans chaque établissement.
4. `POST …/finaliser` `{token}` — vérification **paresseuse** (pas de tâche planifiée) : avant la fin du délai → **409** (jeton non consommé) ; après, et dans les 72 h : en une transaction, suppression de **toutes** les passkeys, des challenges, révocation de **toutes** les sessions, invalidation des liens de mot de passe actifs, `terminee`, événement `passkey_recuperation_finalisee`. Au-delà de 72 h : `expiree`.
5. Ensuite la personne se reconnecte **par mot de passe** (l'obligation de passkey de son rôle la place en session d'enrôlement existante : elle réenrôle une passkey). Sans mot de passe connu : « mot de passe oublié » (`/auth/forgot-password`), puis réenrôlement.

**Sécurité** : jetons 256 bits (hex, 64 caractères), seuls les SHA-256 en base, usage unique atomique (`UPDATE … WHERE statut = … RETURNING`), expirations ; jeton > 256 caractères ou vide → 400 avant tout traitement ; erreurs de jeton uniformes (400). Limitation de débit dédiée (routes ajoutées à `anti_abus.rs`) : demande `5/900` par IP et `2/3600` par e-mail ; confirmer, annuler, finaliser `20/900` par IP et `10/900` par jeton. Journal sans e-mail en clair.

**Limites connues / hypothèses**
- Qui contrôle la boîte e-mail de la personne obtient les deux liens : le délai et l'alerte ne protègent que si la personne lit ses e-mails pendant le délai (compromis choisi par D-018). Un changement d'adresse e-mail pendant le délai n'annule pas la demande (à décider : annuler les demandes actives au changement d'e-mail).
- La récupération par la direction (un seul établissement) reste inchangée et sans délai (suite du lot A8).
- Une demande exige que le message de confirmation parte : messagerie non prête ou URL publique absente → rien n'est créé, neutralité conservée (journal serveur uniquement).
