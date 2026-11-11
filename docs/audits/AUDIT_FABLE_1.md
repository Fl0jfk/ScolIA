# Audit approfondi n° 1 du cœur ScolIA (D-060)

- **Date** : 2026-11-11. **Branche auditée** : `origin/ENT` à `a4bcbe0` (`MIGRATION_TAG` = `2240_assiduite_as0`).
- **Mode** : lecture seule. Aucun code modifié, aucune compilation, aucun test, aucune CI lancée, aucune base consultée.
- **Périmètre** : `ent-rust/crates/{ent-auth, ent-oidc, ent-db, ent-api, ent-object, ent-types}`, `ent-rust/web`, `ent-rust/migrations` (240 fichiers), et en complément `ent-events`, `ent-mail`, `ent-sms`, `ent-push`, `ent-brain`, `ent-bank`, `Dockerfile`, `scripts/`, workflow CI.
- **Chemins** : relatifs à `ent-rust/crates/` sauf mention contraire ; les migrations sont dans `ent-rust/migrations/`.

## 0. Lecture rapide

**Aucun constat critique.** Je n'ai trouvé ni injection SQL, ni route métier sans session, ni fuite directe de données d'un établissement vers un autre par une route existante. L'établissement vient partout de la session, jamais du client. Le cœur d'authentification (jetons hachés, Argon2id, anti-énumération, WebAuthn, garde SSRF de l'OIDC, liens de fichiers signés, validation des téléversements) est de bonne facture.

**21 constats élevés, 34 moyens, 48 faibles** (plus 16 pistes de simplification, section 6). Les plus sérieux se regroupent en cinq thèmes :

1. **L'identité partagée entre établissements (D-013) a trois portes dérobées** : le SSO configuré par un établissement (S-01), la reprise d'IBAN par code e-mail (B-03) et l'invitation famille sans session (B-05).
2. **Le cloisonnement par adresse (D-012) ne fonctionne pas à travers le relais web** (B-01), et un administrateur peut s'approprier n'importe quel nom de domaine (B-02).
3. **Les migrations sont rejouées en entier à chaque démarrage**, sans registre ni transaction (R-01 à R-04) : un redémarrage peut échouer sur des données valides ou réécrire des données corrigées.
4. **Les données de santé ont quatre fuites** : note confidentielle rendue au parent (D-01), routes du psychologue hors du verrou « module santé » (D-02), `documents.lire` qui ouvre les pièces de santé (A-02), motifs d'absence du personnel lisibles par tous (D-03).
5. **La disponibilité repose sur 5 connexions** avec des appels S3 synchrones et des envois de courriels dans des transactions ouvertes (R-05 à R-07).

### Les 10 constats les plus graves

| # | Réf. | Constat | Gravité | Effort |
|---|---|---|---|---|
| 1 | S-01 | Un administrateur d'établissement peut, par un SSO qu'il configure, prendre le compte d'un utilisateur dans **tous** ses établissements | élevée | 2–3 j |
| 2 | B-01 | Le relais web supprime `Host` : le cloisonnement par adresse est inopérant sur `/web` (ou la connexion est impossible en mode strict) | élevée | 0,5 j |
| 3 | B-02 | Tout administrateur d'établissement peut enregistrer n'importe quel nom de domaine, y compris celui d'un autre établissement | élevée | 1–2 j |
| 4 | R-01, R-02, R-03 | 240 migrations rejouées à chaque démarrage, sans registre ; un redémarrage peut échouer ou écraser des données | élevée | 2–3 j |
| 5 | A-01 | Création des comptes élèves : mot de passe temporaire renvoyé au secrétariat, sans changement forcé | élevée | 2–3 j |
| 6 | D-01 | La note confidentielle de l'infirmière est renvoyée au parent qui enregistre la fiche santé | élevée | 1–2 h |
| 7 | D-02 | Les routes du psychologue échappent au verrou « module santé désactivé » (pas de santé sans HDS) | élevée | 0,5 j puis 1–2 j |
| 8 | B-03 | IBAN d'un autre établissement lisible et réutilisable avec un simple code reçu par e-mail | élevée | 1–2 j |
| 9 | D-03, A-02 | Motifs d'absence du personnel lisibles par tout le personnel ; `documents.lire` ouvre aussi les documents de santé | élevée | 1–1,5 j |
| 10 | B-04, R-08 | Mode RLS : `off` par défaut et faute de frappe traitée comme `off` sans bruit ; `pg_dump` vide ou en échec sous `FORCE RLS` | élevée | 1 j |

### Ordre de correction proposé (recommandation)

1. **Tout de suite, petits correctifs sûrs (≈ 3 j)** : D-01, D-02 (préfixe `/psy`), D-03, A-02, A-04, A-07, B-01, B-04 (garde au démarrage), S-03, R-02.
2. **Avant toute base déployée (répétition de Noël, D-057)** : R-01 (registre des migrations), S-01, B-02, B-03, B-05, A-01, R-05 à R-07, R-08.
3. **Avant l'ouverture à un deuxième établissement** : B-06 à B-09, S-02, S-04, D-04 à D-07, test d'intrusion externe (D-060).
4. **En continu** : section 6 (simplification), au fil des lots.

### Écarts documentaires relevés en passant (faits)

- Sur `ENT`, `docs/PILOTAGE.md` s'arrête à la règle 9 et `docs/DECISIONS.md` à D-028 ; `docs/PRA_SAUVEGARDES.md` est absent. Les règles 13 à 17, D-029 à D-060 et les exigences E1–E14 n'existent que sur `ent-mc0hoe` (c'est là que je les ai lues). Une session de code lancée depuis `ENT` ne les voit pas.
- La règle 15 de PILOTAGE s'appuie sur « la vérification d'empreinte de sqlx ». **Elle n'existe pas** : `ent-db/src/migrate.rs` n'utilise pas le migrateur de sqlx et ne tient aucune empreinte (voir R-01).

### Méthode et limites

- Lecture directe par l'auditeur : `ent-auth`, `ent-oidc`, `ent-object` (liens signés, téléversements), `ent-api` (`lib.rs`, `auth.rs`, `tenant.rs`, `http_garde.rs`, `anti_abus.rs`, `client_ip.rs`, handlers d'authentification), `ent-db` (`auth_repo.rs`, parties authentification de `service.rs`, `tenant_tx.rs`, `droits_ctx.rs`, `droits_repo.rs`).
- Six relectures parallèles sur le reste (routes en deux moitiés, autres modules de l'API, migrations et RLS, `ent-db`, frontend et chaîne de livraison). Les constats élevés qui en viennent ont été **revérifiés dans le code** par l'auditeur.
- Non couvert ligne à ligne : environ 330 des 400 méthodes `ent-db` appelées par les routes (contrôlées par signature et par sondage), les 1 979 `innerHTML` de `ui.rs`, les index et la pagination (673 `fetch_all` pour 159 `LIMIT`), `pra.rs`, `reprise_bytea.rs`, les applis mobiles.
- « À confirmer » signale ce qui demande une base réelle ou une lecture complémentaire. Aucun constat n'a été vérifié par exécution.
- Cet audit ne remplace ni un test d'intrusion externe ni l'avis d'un juriste ou du DPO (D-060).

---

## 1. Sécurité : authentification, sessions, passkeys, secrets, téléversements

### S-01 — Prise de compte inter-établissements par le SSO d'un établissement — élevée
- **Où** : `ent-db/src/service.rs:2983-3010` (déclaration d'un fournisseur : rôle `Admin | Direction` seulement), `:3305-3330` (rattachement par e-mail sur l'identité **globale**, `find_user_id_by_email`), `:2260-2270` (enrôlement d'une passkey sans mot de passe si session `Enrolement` + `Sso`), `ent-db/src/securite_repo.rs:92-101` (politique « passkey obligatoire » réglée par les mêmes rôles), `ent-db/src/passkey_repo.rs:139-152` (passkeys par personne, tous établissements).
- **Scénario** : la direction de A (ou quiconque a volé son compte) déclare un fournisseur OIDC qu'elle contrôle. Elle rend la passkey obligatoire pour le rôle de la victime, membre de A et de B, sans passkey. Son fournisseur affirme `email = victime`, `email_verified = true` : l'ENT rattache l'identité, ouvre une session d'enrôlement, et l'attaquant enrôle **sa** passkey sans mot de passe. Cette passkey vaut pour la personne entière : `passkey/login/start` avec l'établissement B ouvre une session complète dans B. Le seul signal est un courriel « passkey ajoutée », envoyé au mieux.
- **Aussi dans A seul** : la direction peut se connecter comme le psychologue ou l'infirmière et lire des données couvertes par le secret professionnel.
- **Incohérence** : `cible_compte_autorisee` (`service.rs:2870-2930`) interdit justement à un tiers de toucher au compte d'une personne membre d'un autre établissement. Le SSO contourne cette règle.
- **Correction** : (a) pas de rattachement par e-mail pour un compte qui a un autre établissement, un mot de passe ou une passkey : le rattachement se fait par la personne, depuis une session ouverte ; (b) pas d'enrôlement de passkey sans preuve propre à la personne (mot de passe ou lien envoyé à son adresse) ; (c) déclaration d'un fournisseur soumise à une validation plateforme ou à une double validation avec réauthentification, domaine d'e-mail borné aux domaines vérifiés de l'établissement ; (d) `link_identity` après le contrôle d'appartenance, pas avant (`service.rs:3326` puis `:3334`).
- **Effort** : 2 à 3 jours.

### S-02 — Énumération des comptes par le défi passkey factice — moyenne
- **Où** : `ent-db/src/service.rs:2385-2440` (`passkey_challenge_factice`).
- **Fait** : le nombre d'identifiants factices est stable par adresse, mais leurs valeurs sont tirées au hasard à chaque appel (`aleatoire_32()`), alors qu'un vrai défi renvoie toujours les mêmes identifiants. La longueur est fixe (32 octets) et la forme JSON est écrite à la main, donc distincte de celle de `webauthn-rs`.
- **Scénario** : deux appels à `/auth/passkey/login/start` pour la même adresse ; identifiants identiques = compte réel, membre de cet établissement, avec passkey. On dresse la liste des parents d'un établissement.
- **Correction** : dériver les identifiants factices de `HMAC(secret, e-mail, i)` avec un secret de configuration (stable entre instances et redémarrages), et produire le factice par la même sérialisation que le vrai.
- **Effort** : 0,5 jour.

### S-03 — Aucune réauthentification récente sur les actions les plus sensibles — moyenne
- **Où** : `session.assurance` n'est qu'affichée (`ent-api/src/routes.rs:2438, 2992, 13125`). Routes concernées : export SEPA (`:11314`), mandat avec IBAN (`:11231`), politique de sécurité (`:13176`), domaines (`:13297`), fournisseurs SSO (`:2603`), exports officiels (`:7321-7649`), export OGEC (`:10973`).
- **Scénario** : un poste laissé ouvert ou un jeton volé suffit à exporter le fichier des prélèvements ou à désactiver l'obligation de passkey.
- **Correction** : garde commune « passkey présentée depuis moins de N minutes » (cohérent avec D-041).
- **Effort** : 1 à 2 jours.

### S-04 — Chiffrement des champs sensibles : pas d'identifiant de clé, clair accepté en lecture — moyenne
- **Où** : `ent-auth/src/secretbox.rs` (`SEALED_PREFIX = "enc:v1:"`, `open` rend tel quel ce qui n'a pas le préfixe), `ent-db/src/champs_chiffres.rs:8, 86-92, 148-150`.
- **Scénario** : rotation de `ENT_SECRETS_KEY` impossible sans tout rechiffrer d'un bloc (contraire à E11). Quelqu'un qui écrit en base remplace un chiffré par du clair et contourne l'authentification par AAD. Une écriture qui oublie le coffre stocke en clair sans erreur (colonnes `TEXT`, `045_fiche_sante.sql:7-13`).
- **Correction** : format `enc:v2:{id_clé}:…`, trousseau en lecture, refus du clair sur les colonnes déclarées une fois la reprise faite, `CHECK` sur le préfixe.
- **Effort** : 2 à 3 jours.

### S-05 — Aucun antivirus n'existe — moyenne
- **Où** : `ent-object/src/upload.rs:341, 461-466` (`AucuneAnalyse` par défaut ; aucune autre implémentation, `avec_analyseur` n'est appelé nulle part hors tests).
- **Scénario** : une famille dépose un document Office piégé (sans macro, mais avec un autre vecteur) ; le secrétariat l'ouvre. Les contrôles de type, de macros et de PDF actif sont bons mais heuristiques (un `/JS` dans un flux compressé passe, `upload.rs:697-700`).
- **Correction** : analyseur ClamAV (`clamd` local), fermé en cas de panne hors labo, ou risque accepté par écrit.
- **Effort** : 1 à 2 jours.

### S-06 — Image de production : seule l'ancienne interface, jeton en `localStorage` — élevée
- **Où** : `ent-rust/Dockerfile:23` (construit seulement `ent-api`), `web/crates/ent-web-server/src/main.rs:27-30` (refuse toute écoute non locale), `ent-api/src/routes.rs:131-132` (`/app` servi sans condition), `ent-api/src/ui.rs:221-222`.
- **Scénario** : le cookie `HttpOnly`, le contrôle CSRF et la CSP à nonce n'existent que dans `ent-web-server`, non livrable. Déployer l'image actuelle expose l'interface F-001, où une seule injection ou une extension de navigateur lit `ent_token`.
- **Correction** : livrer `ent-web-server` (écoute configurable derrière un proxy TLS) ; retirer `/app` des binaires sans `labo`.
- **Effort** : 2 à 3 jours.

### S-07 — Le SSO n'a pas de parcours par cookie — moyenne
- **Où** : `ent-api/src/routes.rs:157` (`/auth/oidc/callback` sert l'ancienne interface), `web/crates/ent-web-session/src/lib.rs:47-52` (le relais interdit `/auth/oidc/`).
- **Scénario** : un établissement en SSO retombe sur l'ancienne interface, jeton en `localStorage`.
- **Correction** : `/web/session/oidc/debut` et `/fin` dans le relais, page de retour Leptos.
- **Effort** : 1 à 2 jours.

### S-08 — `/labo/bootstrap` : administrateur sans authentification dès qu'un binaire `labo` est joignable — moyenne
- **Où** : `ent-api/src/routes.rs:867, 941, 1074, 10703-10710` (`ENT_ALLOW_LABO=1` ouvre même avec `ENT_ENV=prod`).
- **Scénario** : une recette exposée sur Internet rend à quiconque le mot de passe public et les adresses de tous les personas, administrateur compris.
- **Correction** : un binaire `labo` refuse de démarrer sur une adresse non locale (ou exige un secret d'amorçage) ; supprimer la dérogation quand `ENT_ENV=prod`.
- **Effort** : 0,5 jour.

### S-09 — Sauvegardes en clair, lisibles localement, jeton visible dans la liste des processus — moyenne
- **Où** : `ent-rust/scripts/sauvegarde-postgres.sh:41-50`, `scripts/pra-exercice.sh:125, 138` ; aucun `umask`.
- **Correction** : `umask 077`, chiffrement du dump (clé à l'inventaire E11), jeton passé par fichier.
- **Effort** : 0,5 jour.

### S-10 — Chaîne de livraison : références mobiles — moyenne
- **Où** : `ent-rust/Dockerfile:13, 30` (images par étiquette), `:23` (`cargo build` sans `--locked`) ; `.github/workflows/ent-rust.yml:37-38, 62, 442` (actions par étiquette, dont `dtolnay/rust-toolchain@master`), `:454` (binaryen sans empreinte).
- **Correction** : `--locked`, images par `@sha256:`, actions par SHA, `sha256sum -c`.
- **Effort** : 0,5 jour.

### S-11 — Argon2 exécuté sur les fils asynchrones — moyenne
- **Où** : `ent-db/src/auth_repo.rs:372-384` (`verify_password` appelé directement), `ent-auth/src/password.rs`. Aucun `spawn_blocking` dans `ent-db` ni `ent-api`.
- **Scénario** : 120 connexions par minute et par adresse IP sont permises (`anti_abus.rs`). Quelques adresses suffisent à occuper tous les fils et à ralentir toute l'API.
- **Correction** : `spawn_blocking` avec un sémaphore borné.
- **Effort** : 0,5 jour.

### Constats faibles (sécurité)
- **S-12 — Verrou de compte sans adresse IP** : quand l'IP est inconnue, la clé dure vaut `compte-ip:{email}|-` (`auth_repo.rs:335`). N'importe qui verrouille alors le compte d'une victime 15 minutes avec 5 essais. Dépend du déploiement (`ConnectInfo`, `ENT_TRUST_PROXY`). Refuser de démarrer hors labo si l'IP cliente n'est pas déterminable. 2 h.
- **S-13 — Passkeys liées à `scolia.fr` avec sous-domaines** (`ent-auth/src/passkey.rs`, `ENT_RP_ALLOW_SUBDOMAINS`) : tout sous-domaine de `scolia.fr` (site vitrine, iframe bureautique, domaine volé par B-02) peut demander une assertion. Liste fermée d'origines plutôt que `allow_subdomains`. 0,5 j.
- **S-14 — Jeton d'identité OIDC** (`ent-oidc/src/jwt.rs`) : pas de contrôle de `iat`/`nbf`, ni d'`azp` avec plusieurs audiences, ni de taille minimale de clé RSA. 2 h.
- **S-15 — Client HTTP de l'OIDC** (`ent-oidc/src/transport.rs`) : pas de `.no_proxy()`. Si `HTTPS_PROXY` est défini, la résolution contrôlée est contournée (le proxy résout). 15 min.
- **S-16 — Mot de passe temporaire** (`ent-auth/src/token.rs:16-24`) : biais de modulo (62 caractères sur 256 valeurs). Tirage par rejet. 15 min.
- **S-17 — HSTS absent** (`ent-api/src/lib.rs:205-216`, `web/crates/ent-web-server/src/lib.rs:136-147`). 1 h.
- **S-18 — Contrôle CSRF** : l'origine est comparée à la liste de toutes les origines autorisées, pas à l'hôte de la requête (`web/crates/ent-web-session/src/lib.rs:163-164`). Non exploitable aujourd'hui (le pré-contrôle CORS échoue). À corriger avec B-01. 1 h.
- **S-19 — Le relais pose en cookie une session mobile** (`ent-web-session/src/lib.rs:364-370`) : défi ouvert avec `client=mobile` puis terminé par le relais. Refuser si la session n'est pas `web`. 2 h.
- **S-20 — CORS** : `ENT_CORS_ORIGINS=*` donne `CorsLayer::permissive()` (`ent-api/src/lib.rs:146`). Refuser `*` hors labo. 15 min.
- **S-21 — Détails renvoyés au client** : `format!("credential invalide: {e}")` (`routes.rs:2430, 2526`) ; `/health` et `/ready` publient `git_sha` et `migration_tag` sans authentification (`:2016-2040`). 1 h.
- **S-22 — Appareils push** (`routes.rs:2934`, `ent-db/src/push_repo.rs:51-76`) : pas de désinscription, la déconnexion ne révoque pas le jeton d'appareil. Sur une tablette partagée, les notifications du précédent continuent. 0,5 j.
- **S-23 — Service worker de l'ancienne interface** (`ent-api/static/sw.js:140-149`) : met en cache toute navigation, donc `/fichiers/{jeton}` s'il est un jour activé (il échoue aujourd'hui faute d'en-tête). À supprimer. 0,5 j.
- **S-24 — Établissement désactivé** : `resolve_bearer` (`auth_repo.rs:770-800`) ne regarde pas `ent_etablissement.actif` ; les sessions d'un établissement désactivé restent valides. 1 h.

---

## 2. Cloisonnement entre établissements et RLS

### B-01 — Le relais web supprime `Host` : le cloisonnement par adresse est inopérant sur `/web` — élevée
- **Où** : `web/crates/ent-web-session/src/lib.rs:212` (`nom != header::HOST`, URI interne relative) ; `ent-api/src/tenant.rs:53-68, 113-115`.
- **Scénario** : pour toute requête relayée, `hote_brut` ne trouve aucun hôte. En mode `souple` (défaut), `accepte()` est toujours vrai et l'établissement de connexion est celui envoyé par le navigateur : sur `lpnb.scolia.fr`, on ouvre ou on utilise une session d'un autre établissement. En mode `strict`, toute requête relayée reçoit 404 : connexion impossible. Aucun test ne couvre l'hôte à travers le relais.
- **Correction** : transmettre `Host` dans `transmettre` ; test « session de B refusée sur l'hôte de A via `/web/api` », en souple et en strict ; exiger `strict` hors labo.
- **Effort** : 0,5 jour.

### B-02 — Un administrateur d'établissement s'approprie n'importe quel nom de domaine — élevée
- **Où** : `ent-api/src/routes.rs:13297-13317, 13277-13295` ; `ent-db/src/etablissement_repo.rs:197-240, 133-170`.
- **Fait** : `exiger_admin` puis `INSERT … ON CONFLICT (domaine) DO NOTHING`. Aucune preuve de possession, aucune liste réservée, premier arrivé premier servi. Le slug est aussi modifiable.
- **Scénario** : l'administrateur de A enregistre `college-b.scolia.fr` avant l'ouverture de B, ou l'hôte générique de la plateforme. Les utilisateurs de B arrivent sur une page de connexion au nom de A, qui propose le SSO de A (voir S-01) : hameçonnage sur une adresse légitime, et déni de service pour B.
- **Correction** : ajout de domaine et changement de slug réservés à un rôle plateforme ; ou suffixe imposé `{slug}.scolia.fr`, slug immuable, liste d'hôtes réservés ; domaines personnalisés sous vérification DNS.
- **Effort** : 1 à 2 jours.

### B-03 — IBAN d'un autre établissement lu et réutilisé avec un code e-mail — élevée
- **Où** : `ent-db/src/inscriptions.rs:3236-3264` (`ibans_reprenables`, filtre `l.etablissement_id <> $2`), utilisée en `:3321-3375` ; signature `:3454` ; jeton issu de `verifier_preinscription` (`:2804-2857`).
- **Scénario** : une personne qui lit la boîte mail d'un parent payeur de A obtient un jeton de préinscription dans B avec un code à 6 chiffres. Elle voit l'IBAN masqué et les initiales du titulaire, crée un mandat « reprise » sur cet IBAN et le signe. Ni passkey ni session.
- **Cloisonnement** : c'est la seule requête d'`ent-db` qui lit volontairement un autre établissement, ce qui contredit D-013. Elle tourne dans un `TenantTx` de B : en RLS stricte elle ne rendra plus rien, sans erreur.
- **Correction** : reprise d'IBAN et signature réservées à un jeton issu d'une session ; le besoin inter-établissements passe par un service d'identité explicite et journalisé ; test en mode strict.
- **Effort** : 1 à 2 jours.

### B-04 — Mode RLS ouvert par défaut, faute de frappe silencieuse — élevée
- **Où** : `ent-db/src/tenant_tx.rs:17-32` (absent ou inconnu → `Off`) ; repli `… IS NULL AND NOT ent_rls_strict()` dans toutes les politiques (`100_rls_pilote.sql:44-48`, `580_rls_eleves_responsables.sql:25-30`).
- **Scénario** : `ENT_RLS_MODE=stricte` en production : aucune barrière en base, aucun message. Un filtre `etablissement_id` oublié fait alors fuiter un autre établissement.
- **Correction** : refuser de démarrer sur une valeur inconnue ; hors labo, exiger `strict` ; journaliser le mode effectif ; après la bascule, migration qui retire le repli permissif.
- **Effort** : 0,5 jour pour la garde, 1 jour pour la migration finale.

### B-05 — Invitation famille : rattachement à un compte existant sans session — moyenne
- **Où** : `ent-db/src/inscriptions.rs:3622-3649`.
- **Scénario** : le lien reçu par e-mail suffit à ajouter l'appartenance `parent` et à activer les liens (enfants, payeur) sur un compte déjà existant, éventuellement d'un autre établissement. Le parcours du personnel exige la session du compte (`personnels_acces.rs:913-924`).
- **Correction** : aligner sur le parcours du personnel.
- **Effort** : 1 jour.

### B-06 — Tables sans RLS portant des données personnelles de tous les établissements — moyenne
- **Où** : `ent-db/src/registre_donnees.rs:365, 422, 424, 452, 455, 486, 489, 526` : `ent_metier_event` (charge utile de tous les faits métier), `ent_outbox`, `ent_inscription_envoi`, `ent_inscription_invitation`, `ent_preinscription_verification`, `ent_membership`, `ent_oidc_provider`.
- **Aggravant** : des charges utiles trop bavardes y sont écrites (`service.rs:9444-9451` titre et clé de stockage d'un document, `:9248` e-mail, `:13372` titre d'alerte CPE).
- **Correction** : RLS standard sur `ent_metier_event`, `ent_outbox`, `ent_inscription_envoi` (le distributeur passe par `etablissements_systeme`) ; identifiants et codes seulement dans les charges utiles.
- **Effort** : 2 à 3 jours.

### B-07 — Clés étrangères simples sur les tables les plus sensibles — moyenne
- **Où** : `045_fiche_sante.sql:6`, `052_pai.sql:6`, `040_infirmerie_passage.sql:6`, `041_psy_entretien.sql:6`, `051_psy_dossier.sql:6`, `024_facturation.sql:6`, `030_sepa.sql:14`, `023_messagerie.sql:6, 22`, `054_vs_commission.sql:27-28`. Environ 165 clés composées sur 617 ; `ent_eleve_tables_a_cle()` (`240_eleve_cles_composees.sql:37-47`) ne couvre ni santé, ni finances, ni messagerie. `etablissement_id` n'a presque jamais de clé vers `ent_etablissement`.
- **Exemples vérifiés côté code** : convocation en commission avec un `sanction_id` ou `incident_id` non vérifié (`service.rs:11987-12030`) ; `prof_user_id` du solveur d'emploi du temps non vérifié comme membre (`service.rs:7862-7926`, `:15608-15625`).
- **Scénario** : hors RLS stricte, une ligne de A référence une ligne de B ; l'extraction PRA de A embarque une référence étrangère ; le succès ou l'échec sert d'oracle d'existence.
- **Correction** : étendre la liste de 240 (clé composée `NOT VALID`, rapport d'orphelins, validation) ; vérifier `(etablissement_id, id)` dans les deux services cités.
- **Effort** : 2 à 3 jours.

### B-08 — Un déclencheur pose le contexte d'établissement à partir de la ligne insérée et ne le rend pas — moyenne
- **Où** : `335_structure_periodes.sql:191-193` (`ent_trg_periode_s7`).
- **Scénario** : en mode strict, un chemin sans contexte insère une période avec un `etablissement_id` quelconque ; le déclencheur adopte cet établissement avant le `WITH CHECK`, qui passe, et le reste de la transaction voit cet établissement. La migration 600 restaure la valeur, pas celle-ci.
- **Correction** : supprimer cette pose ou la restaurer en fin de déclencheur.
- **Effort** : 0,5 jour.

### B-09 — Catalogues globaux modifiables depuis un établissement — moyenne
- **Où** : `353_agents_fonctions.sql:311-316`, `355_missions.sql:422-427` (politique `FOR ALL` dont le `USING` inclut `etablissement_id IS NULL`).
- **Scénario** : un `DELETE` ou un `UPDATE … SET etablissement_id = <le mien>` mal filtré retire une fonction fournie par ScolIA pour tous les autres.
- **Correction** : politique de lecture (NULL ou contexte) et politique d'écriture limitée au contexte.
- **Effort** : 0,5 jour.

### B-10 — Vues sans `security_invoker` — moyenne (à confirmer)
- **Où** : `265_etablissement_fuseau.sql:33`, `480_msg_m0a_fuites.sql:16, 43`, `481_msg_m0b_securite_fiabilite.sql:154`, `260:38`, `276:110`, `306:74`.
- **Scénario** : si le rôle de migration est superutilisateur ou `BYPASSRLS` (fréquent en base managée ; à confirmer chez Scaleway), toute lecture à travers ces vues contourne la RLS.
- **Correction** : `WITH (security_invoker = true)` et un test.
- **Effort** : 0,5 jour.

### B-11 — Oracle d'existence d'une adresse entre établissements — moyenne
- **Où** : `ent-db/src/service.rs:5800-5812`, `:5602`, `:9230`. `provisionner_personnel_for_actor` (`:3476-3520`) ferme cet oracle, pas ces trois chemins.
- **Scénario** : un administratif de A teste une adresse ; un 409 lui apprend qu'elle a un compte ailleurs. Conséquence fonctionnelle : un parent déjà connu ailleurs ne peut pas être rattaché dans un second établissement, ce qui contredit D-013.
- **Correction** : un seul service « rattacher une identité existante par invitation acceptée », réponse uniforme.
- **Effort** : 2 à 4 jours.

### B-12 — Le rattachement d'un responsable écrase le téléphone de n'importe quel compte — moyenne
- **Où** : `ent-api/src/routes.rs:6419` ; `ent-db/src/service.rs:5794, 5821`.
- **Scénario** : un administratif envoie `POST /foyers/{id}/liens` avec l'adresse d'un parent aussi membre d'un autre établissement et un numéro de son choix. Le téléphone de l'identité globale est remplacé : les SMS de l'autre établissement partent vers ce numéro. `set_user_telephone_for_actor` (`:5511-5540`) l'interdit ; ce chemin l'ignore.
- **Correction** : n'écrire le téléphone que pour un compte créé dans l'appel ; sinon même garde, journal et notification.
- **Effort** : 0,5 jour.

### Constats faibles (cloisonnement)
- **B-13 — Garde statique RLS trop large** (`ent-db/src/garde_rls_stricte.rs:35-53`) : l'exception `("service", "&self.pool")` couvre les 20 500 lignes de `service.rs`. Compter les occurrences attendues. 0,5 j.
- **B-14 — Mises à jour par `id` seul** (`dossier_coordonnees.rs:845, 911, 978, 1200-1262`, `dossier_contact.rs:911, 1209, 1241`) : l'identifiant vient d'une lecture bornée, pas de fuite constatée, mais pas de seconde barrière tant que la RLS est `off`. 1 h.
- **B-15 — Le rôle applicatif peut poser lui-même `app.etablissement_id`, `app.rls_strict` et les drapeaux `ent.*`** qui neutralisent des déclencheurs de garde. Limite connue : la RLS ne protège pas d'une injection SQL. À écrire dans `RLS_STRICTE.md`. 0,25 j.
- **B-16 — Journal d'audit** (`091_journal_ajout_seul.sql`) : `TRUNCATE` non bloqué ; sans `ENT_DATABASE_MIGRATION_URL`, l'application est propriétaire des tables (`service.rs:974`) et peut désactiver déclencheur et RLS. Déclencheur `BEFORE TRUNCATE` ; refus de servir hors labo si le rôle courant possède les tables. 1 j. *(Gravité moyenne si l'on vise une valeur probante du journal.)*
- **B-17 — Récupération de passkey** (`routes.rs:2862`) : l'établissement vient du corps, pas de l'hôte. 1 h.
- **B-18 — Deux mécanismes de contexte** : `TenantTx` et la pose manuelle d'`outbox.rs:637-650`, qui pose la variable même en mode `off`. Un seul chemin. 2 h.

---

## 3. Autorisation côté serveur et moindre privilège

Constat d'ensemble : toutes les routes métier examinées exigent une session ; l'autorisation réelle est dans `ent-db` (`*_for_actor`). Les défauts sont surtout dans le code d'avant les socles 2 à 6, qui teste un profil (`is_staff()`) au lieu d'une action et d'un périmètre.

### A-01 — Comptes élèves créés avec un mot de passe temporaire renvoyé au secrétariat — élevée
- **Où** : `ent-api/src/routes.rs:1442, 1468` ; `ent-api/src/dto.rs:87-111` ; `ent-db/src/service.rs:9209-9260` ; `domain-eleve/src/invitation.rs:88` (adresse par défaut `…@ent.lab`), `:91-94` (`"Eleve" + 12 hex + "!"`).
- **Scénario** : `POST /eleves/comptes/provisionner` rend en JSON l'identifiant et le mot de passe de tous les élèves sans compte. Aucun changement forcé à la première connexion n'existe. Un administratif se connecte durablement comme n'importe quel élève mineur, et ce que fait le compte est attribué à l'élève. L'adresse `@ent.lab` n'est pas joignable : « mot de passe oublié » ne fonctionne pas.
- **Aggravants** : création en trois écritures non atomiques ; e-mail dans la charge utile de l'événement.
- **Correction** : aligner sur le socle 6 (`provisionner_personnel`) : secret jamais renvoyé, lien d'activation à usage unique ou fiche papier à code expirant avec changement forcé ; supprimer `@ent.lab` hors labo ; une transaction.
- **Effort** : 2 à 3 jours.

### A-02 — Le droit `documents.lire` ouvre aussi les documents de santé — élevée
- **Où** : `ent-db/src/service.rs:9924-9930` (la première branche rend `true` sans regarder la confidentialité) ; libellé contraire dans `ent-auth/src/actions.rs:627` (« hors tiroir santé »).
- **Scénario** : la direction crée un rôle « professeur principal » avec `documents.lire` sur ses classes, en se fiant au libellé. Ce rôle télécharge certificats médicaux et pièces `sante` ou `restreint`. Avec les profils par défaut la fuite n'apparaît pas (ils ont aussi `documents.lire_sante`).
- **Correction** : `documents.lire` seulement si `standard` ; `sante` → `documents.lire_sante` ; `restreint` → action dédiée ou accès accordé ; test avec un rôle paramétré.
- **Effort** : 0,5 à 1 jour.

### A-03 — La liste des documents révèle aux familles des pièces qu'elles ne peuvent pas lire — élevée
- **Où** : `ent-api/src/routes.rs:1631` ; `ent-db/src/service.rs:9762-9796`.
- **Scénario** : après une information préoccupante, le CPE dépose une pièce `restreint`. Le parent mis en cause voit une ligne « Document à accès restreint » avec `tiroir`, `confidentialite`, `code` et `source`. Pour un document `standard` hors tiroir famille (note interne), le vrai titre est rendu. L'élève mineur voit la même chose.
- **Correction** : pour parent et élève, ne rendre que les documents lisibles ; pour le personnel sans droit, masquer aussi `code` et `source`.
- **Effort** : 0,5 jour.

### A-04 — Un fichier manquant est remplacé par un faux document, en production — moyenne
- **Où** : `ent-db/src/service.rs:9545-9573`.
- **Scénario** : objet perdu ou clé non conforme : `GET /documents/{id}/fichier` répond 200 avec « Document ENT (lab) — titre… » sous le nom et le type du vrai document. La perte n'est jamais vue ; un accès est journalisé. La route `/lien` rend 404 dans le même cas.
- **Correction** : 404 et alerte ; texte de substitution sous `cfg(feature = "labo")`.
- **Effort** : 1 à 2 heures.

### A-05 — Réinscription : un document « déposé » sans fichier lève la vanne des pièces obligatoires — moyenne
- **Où** : `ent-api/src/routes.rs:10456-10495` ; `ent-db/src/service.rs:11400-11463` (clé d'objet inventée) ; `domain-eleve/src/continuum.rs:601-617`.
- **Scénario** : un parent poste `{"code":"ASSURANCE"}` pour chaque exigence et confirme sans avoir fourni aucun fichier.
- **Correction** : contenu obligatoire, plus de clé fictive, seuls les documents dont l'objet existe comptent.
- **Effort** : 3 à 4 heures.

### A-06 — Le drapeau `peut_voir_vs` n'est pas appliqué partout — moyenne
- **Où** : `ent-db/src/service.rs:13438-13465` (alertes CPE), `:12997-13060` (internat), `:13654+` (documents à signer), `:14075+` (informations complémentaires). Le journal (`:13228`) et les absences (`:5311`) le testent bien.
- **Scénario** : la direction retire le suivi de vie scolaire à un parent ; il lit toujours les alertes CPE et le suivi d'internat.
- **Correction** : une fonction unique `droits_famille(lien, domaine)`.
- **Effort** : 1 jour.

### A-07 — Toute personne du personnel peut demander l'accès à n'importe quel document — moyenne
- **Où** : `ent-api/src/routes.rs:1900` ; `ent-db/src/service.rs:9622`.
- **Scénario** : un compte d'accueil demande l'accès à un document de santé d'un élève hors de son périmètre ; la demande entre dans la file du gestionnaire (acceptation par erreur) et 201 ou 404 confirme l'existence.
- **Correction** : exiger `dossier.lire` sur l'élève ; borner `duration_days`.
- **Effort** : 2 heures.

### A-08 — Création de comptes à profil sensible sans contre-pouvoir — moyenne (à arbitrer)
- **Où** : `ent-db/src/service.rs:3446-3452`.
- **Scénario** : la direction, ou tout titulaire de `personnels.gerer`, crée un compte `psy` ou `infirmerie` sur une adresse qu'elle contrôle et lit des données couvertes par le secret professionnel. Journalisé, mais sans double validation ni notification. Même famille que S-01.
- **Correction** : double validation ou notification aux titulaires existants du profil.
- **Effort** : 1 jour.

### Constats faibles (autorisation)
- **A-09** — Emploi du temps d'un élève lisible par tout le personnel sans périmètre (`service.rs:7514-7557`). 1 h.
- **A-10** — Éditions de vie scolaire : nom, prénom, classe de tout élève pour tout le personnel, sans trace (`service.rs:15542-15606`). 1 h.
- **A-11** — `actor_can_know_absent` trop large (`service.rs:5448-5456`). 2 h.
- **A-12** — Réinscription : n'importe quel lien actif suffit, rôle de session non contrôlé (`service.rs:11292-11307, 11400-11416, 11467-11480`). À confirmer : la vue `ent_eleve_lien_actif` filtre peut-être déjà l'autorité parentale. 3 h.
- **A-13** — `annuler_dispense_repas` : le contrôle de cohérence arrive après l'effet (`routes.rs:12904-12925`). 30 min.
- **A-14** — Routes ouvertes à toute session : `get_campagne` (`routes.rs:10164`), `get_reglage_plage_calme` (`:10044`), `list_note_periodes` (`:8275`), `GET /evaluations/periodes` (`evaluations.rs:109-111`). 30 min.
- **A-15** — `GET /vs/punition-types` écrit en base avec seulement `require_staff` (`vie_scolaire.rs:377-387`). 0,25 j.
- **A-16** — Autorisation par chaîne magique `"@moteur:referentiel.gerer"` passée à la place du rôle (`ent-api/src/referentiel.rs:156-172`, `ent-db/src/referentiel_repo.rs:123-138`). Type preuve non constructible. 1 j.
- **A-17** — Session « arrivée » ouverte par préfixe `/personnels/moi/` (`ent-api/src/auth.rs:44, 271-276`) : toute future route y entre d'office. Liste fermée. 0,25 j.
- **A-18** — `bilan_campagne` : droit vérifié dans la couche HTTP, pas dans `bilan_envois_source` (`communication_s28.rs:212-231`, `notifications.rs:612`). 0,25 j.
- **A-19** — Rattachement d'un responsable : les quatre droits valent `true` s'ils sont omis, même pour un lien `autre` (`routes.rs:6437-6441`). 1 h.
- **A-20** — Absence RH : la validation écrase l'auteur de la déclaration ; un administratif peut déclarer une absence pour un parent ou un élève (`service.rs:15236, 15327`). 2 h.
- **A-21** — Deux moteurs de droits coexistent : prédicats de profil en dur (`routes.rs:4279, 4327, 8034, 12963, 11543, 12871, 13166`, `service.rs:6233`) à côté des actions D-017. Un rôle paramétré « AED : absences.lire » n'ouvre pas `/vs/absences`. Incohérence fonctionnelle. 2 à 3 j.

---

## 4. Données personnelles : minimisation, santé, mineurs, D-036

### D-01 — La note confidentielle de la fiche santé est renvoyée au parent — élevée
- **Où** : `ent-api/src/routes.rs:9403-9430` ; `ent-db/src/service.rs:16081-16143` ; `ent-db/src/infirmerie_repo.rs:165-231`.
- **Fait** : pour un parent, le service recopie la `note_confidentielle` existante dans la commande puis rend la fiche complète. `vue_fiche_sante_famille` n'est appliquée qu'en lecture (`:16171, 16185`).
- **Scénario** : un parent lié fait `PUT /eleves/{id}/fiche-sante` ; la réponse contient la note de l'infirmière.
- **Atténuation** : module santé désactivé par défaut.
- **Correction** : appliquer `vue_fiche_sante_famille` au retour ; test.
- **Effort** : 1 à 2 heures.

### D-02 — Les routes du psychologue échappent au verrou « module santé » — élevée
- **Où** : `ent-api/src/sante.rs:47-84` (`SEGMENTS_ELEVE_SANTE` sans `dossier-psy`, aucun préfixe `/psy`) ; routes `ent-api/src/routes.rs:691-706` (`/psy/entretiens`, `/psy/rdvs`, `/psy/rdvs/{id}/statut`, `/eleves/{id}/dossier-psy`, courrier d'orientation).
- **Scénario** : un établissement sans hébergement HDS laisse le module désactivé ; `/sante/*` répond 403, mais entretiens, rendez-vous et dossier psychologique se créent et se lisent.
- **Cause de fond** : le verrou filtre des chemins HTTP ; `module_sante_actif` n'est appelé nulle part dans `ent-db`. Toute nouvelle route ou tout autre module qui lit une table de santé y échappe.
- **Correction** : ajouter `/psy` et `dossier-psy` tout de suite ; puis garde dans `ent-db` (`psy_repo`, `infirmerie_repo`, `sante_acc`, `accompagnement`), avec un test qui confronte les tables « santé » du registre aux fonctions gardées.
- **Effort** : 0,5 jour, puis 1 à 2 jours.

### D-03 — Absences du personnel, motif compris, lisibles par tout le personnel — élevée
- **Où** : `ent-api/src/routes.rs:6324` ; `ent-db/src/service.rs:15363-15376` (seul contrôle : `is_staff()`) ; `ent-api/src/dto.rs:3357`.
- **Scénario** : un professeur ou l'accueil appelle `GET /rh/absences?debut=…&fin=…` et obtient les absences de tous les collègues avec le motif en texte libre, « parfois médical » selon le code lui-même.
- **Correction** : liste réservée à une action `rh.absences.lire` ; les autres ne voient que les leurs ; pas de `motif` hors RH ; motif chiffré.
- **Effort** : 0,5 jour.

### D-04 — Tout adulte lié peut écraser la fiche santé, sans historique — moyenne
- **Où** : `ent-db/src/service.rs:16096-16125` ; `infirmerie_repo.rs:167-183` (remplacement complet).
- **Scénario** : un responsable aux droits restreints envoie des champs vides ; allergies et traitements saisis par l'infirmière sont perdus sans trace de l'ancienne valeur.
- **Correction** : droit explicite sur le lien, mise à jour champ par champ, version précédente conservée, ou proposition validée par l'infirmerie.
- **Effort** : 1 jour.

### D-05 — L'espace de préinscription affiche le profil d'un compte existant sur simple code e-mail — moyenne
- **Où** : `ent-db/src/inscriptions.rs:3033-3072`.
- **Scénario** : le code à 6 chiffres suffit pour lire nom, civilité, téléphone et adresse dans `ent_profil_personnel` (table globale), même pour un compte de personnel ou d'un autre établissement. Contourne la passkey obligatoire (D-009).
- **Correction** : profil seulement pour un jeton issu d'une session.
- **Effort** : 0,5 jour.

### D-06 — Adresse e-mail servie comme libellé d'une personne — moyenne
- **Où** : `ent-db/src/auth_repo.rs:1421-1452` (`user_labels` retombe sur l'e-mail) ; utilisé par les `enrich_*` (`ent-api/src/routes.rs:11651-11766`). Aussi `GET /personnel/search` et `/personnel/{id}` (`routes.rs:1166, 1408`), qui rendent l'e-mail de connexion de tous les agents à tout le personnel.
- **Scénario** : un parent ou un élève lit un fil ou l'emploi du temps et obtient l'adresse d'un professeur ou d'un autre parent.
- **Correction** : libellé neutre ; `display_name` obligatoire ; annuaire minimisé du socle 6.
- **Effort** : 0,5 jour.

### D-07 — Journal des accès sensibles « au mieux » — moyenne
- **Où** : `ent-db/src/service.rs:3888-3944` (`audit_acces`, `tracer_acces_sensible` ; 26 appelants dont `sante_acc.rs:838, 1363, 1486`) ; lectures financières inégales (`routes.rs:11450, 11572` sans trace, `:10904` avec).
- **Scénario** : journal indisponible ; des fiches de santé sont lues sans aucune trace (contraire à E7).
- **Correction** : pour santé, psychologue et exports, trace dans la transaction de lecture, refus si elle échoue.
- **Effort** : 1 jour.

### D-08 — Injection de formules dans six exports CSV — moyenne
- **Où** : `domain-eleve/src/facturation_sepa.rs:548`, `facturation_ogec.rs:49`, `repas.rs:587`, `onde_export.rs`, `notanet_export.rs:30`, `parcoursup_export.rs:40`.
- **Scénario** : une famille saisit comme titulaire `=HYPERLINK("http://x";"Voir")` ; la valeur ressort telle quelle dans l'export des prélèvements que le comptable ouvre dans un tableur.
- **Correction** : une fonction partagée sur le modèle de `ent-db/src/voyages.rs:544`, un test par export.
- **Effort** : 0,5 jour.

### D-09 — Reçus d'envoi gardés en mémoire sans limite — moyenne
- **Où** : `ent-mail/src/lib.rs:219-221`, `ent-sms/src/lib.rs:238-240`, `ent-push/src/lib.rs:177-179`.
- **Scénario** : fuite mémoire continue en production ; adresses, numéros, sujets et corps de SMS de tous les établissements restent en mémoire du processus.
- **Correction** : reçus en test ou labo seulement, ou tampon borné.
- **Effort** : 2 heures.

### D-10 — `ON DELETE CASCADE` sur des données à valeur probante ou de protection — moyenne
- **Où** : `271_eleve_interdiction.sql:11` (interdictions de contact), `270_conflit_interet.sql:12`, `078_inscription_iban.sql:6`, `122_contact_urgence.sql:8`, `210_sante_pai_complet.sql:22` (signatures de PAI). 118 cascades au total.
- **Scénario** : la suppression d'un élève efface sans trace interdictions de contact et signatures.
- **Correction** : `RESTRICT` et clôture datée (E6).
- **Effort** : 1 jour.

### Constats faibles (données)
- **D-11 — Corps des erreurs 500 recopié dans les journaux** (`ent-api/src/http_garde.rs:64-69`, 300 caractères) : un message SQL peut contenir des valeurs de ligne (E14). Journaliser un code typé. 0,5 j.
- **D-12 — `CreerMandatSepaBody` dérive `Debug` avec l'IBAN complet** (`ent-api/src/dto.rs:1757-1765`). 15 min.
- **D-13 — Clé de stockage exposée** : `TrombiEntreeDto.photo_key` (`dto.rs:2976-2981`). 30 min.
- **D-14 — Photo servie en `image/jpeg` quel que soit le format** (`routes.rs:6918-6921`). 30 min.
- **D-15 — D-036** : une colonne JSONB encore en dette (`ent_inscription_campagne.regle_relance`), treize en « retrait » conservées ; BYTEA résiduels (`038_msg_piece.sql:11`, `053_pai_piece.sql:11`, pièces de santé comprises) jusqu'à la reprise ; `ent_eleve_orphelin` garde des copies JSONB complètes de lignes, sans rétention (`240:22-33`). 1 à 2 j.
- **D-16 — Jetons dans les URL** : invitations et récupération passent le jeton en paramètre (`inscriptions_reactions.rs:218-223`, `service.rs:2700`). Journaux d'accès du proxy et en-tête `Referer`. Fragment ou POST. 0,5 j.
- **D-17 — Mot de passe de labo en dur dans les prototypes mobiles** (`ent-rust/apps/ios-eleve/Sources/EleveApp.swift:129`, `apps/android-eleve/.../MainActivity.kt:39`). À retirer à la fusion des applis (D-003).

---

## 5. Robustesse : migrations, transactions, concurrence, fuseau

### R-01 — Aucun registre des migrations : 240 fichiers rejoués à chaque démarrage, sans transaction — élevée
- **Où** : `ent-db/src/migrate.rs:262-309` ; découpage naïf sur `;` (`:311-331`).
- **Fait** : chaque instruction part seule sur le pool (validation automatique, connexions différentes). Ni table de suivi, ni empreinte. 52 fichiers contiennent des reprises de données.
- **Scénario** : une migration échoue à mi-fichier, le schéma reste à moitié appliqué. Une migration ancienne modifiée n'est détectée par rien. Chaque redémarrage reprend des verrous exclusifs sur toutes les tables.
- **Correction** : table `ent_schema_migration(nom, sha256, appliquee_le)`, une transaction par fichier, refus si l'empreinte d'un fichier appliqué change ; bloc « rejouable » distinct pour fonctions, vues et politiques. R-02 à R-04 disparaissent avec ce correctif.
- **Effort** : 2 à 3 jours, test de rejeu compris.

### R-02 — Le rejeu recrée d'anciens index uniques : un redémarrage peut échouer sur des données valides — élevée
- **Où** : `140_inscription_campagne.sql:32` puis `302_inscription_campagne_modele.sql:92` ; `141_inscription_dossier.sql:54` puis `304_inscription_demarche_dossier.sql:324`.
- **Scénario** : un établissement ouvre deux campagnes d'admission pour deux sites, ou un élève a un dossier annulé puis un nouveau. Au redémarrage suivant, le `CREATE UNIQUE INDEX` de 140 ou 141 échoue (23505), `migrate()` échoue, l'API ne démarre plus, pour tous les établissements.
- **Correction immédiate** : retirer la création dans 140 et 141 (rien n'est déployé ; dernière tolérance de la règle 15), sinon R-01.
- **Effort** : 0,5 jour.

### R-03 — Des reprises de données s'exécutent à chaque démarrage — élevée
- **Où** : `302_inscription_campagne_modele.sql:114` (`decision_requise` remis à `false` ; à confirmer qu'il est réglable), `375_msg_fil_prive.sql:7-13` (un fil créé par un parent redevient privé), `337_structure_lieux.sql:122` (salle rattachée de nouveau à son site), `306_inscription_offre_reprise_places.sql:63`, `480_msg_m0a_fuites.sql:75` (toutes les lignes réécrites à chaque démarrage).
- **Scénario** : un personnel corrige une donnée ; elle revient à l'ancienne valeur après le déploiement suivant, sans trace au journal.
- **Correction** : exécution unique (R-01).
- **Effort** : 1 jour hors R-01.

### R-04 — Contraintes, politiques et vues détruites puis recréées à chaque démarrage — élevée
- **Où** : 53 paires `DROP CONSTRAINT` + `ADD CONSTRAINT` sans `NOT VALID` (ex. `460_eva_v0_correctifs.sql:51-53`, `335_structure_periodes.sql:165-168`) ; `DROP POLICY` puis `CREATE POLICY` en deux instructions (`280:232-233`, `281:313`, `283:449-450`, `353:310-311`, `500:54-55`, `561:97`, `563:56`) ; `DROP VIEW ent_eleve_lien_actif` (`121:41-42`, `260:21-22`).
- **Scénario** : entre les deux instructions, la table est sous RLS forcée sans politique : une autre instance lit 0 ligne sans erreur. La vue disparaît un instant, puis porte la définition en UTC de 121 jusqu'au passage de 265. Écrans vides et erreurs 500 pendant un déploiement progressif.
- **Bombe latente** : `460_eva_v0_correctifs.sql:14-45` (`INSERT … SELECT n.*`) cassera le démarrage dès qu'une colonne sera ajoutée à `ent_note`.
- **Correction** : R-01, ou garde « déjà conforme » comme `ent_rls_poser` (`580:14-20`).
- **Effort** : 2 jours, ou R-01.

### R-05 — Pool de 5 connexions figé, sans délai — élevée
- **Où** : `ent-db/src/service.rs:749-752` (`max_connections(5)`, ni `acquire_timeout` ni `statement_timeout`).
- **Coût par requête** : résolution de l'hôte, session (une requête avec trois fonctions SQL, plus une écriture par minute), droits effectifs (deux à trois transactions, refaites par `a_action_quelconque`), puis le travail métier, souvent une transaction par appel de dépôt.
- **Scénario** : une trentaine d'utilisateurs simultanés à la sonnerie pour l'appel saturent le pool ; tout attend 30 s.
- **Correction** : taille par variable d'environnement, délais d'acquisition et de requête, cache court des droits et du domaine.
- **Effort** : 1 jour.

### R-06 — Stockage objet S3 synchrone dans le code asynchrone — élevée
- **Où** : `ent-db/src/objets.rs:62, 89-93` ; client `ureq` (`ent-object/src/s3.rs:95-103`, 60 s, 3 tentatives).
- **Scénario** : S3 ralentit ; quelques téléversements simultanés bloquent chacun un fil jusqu'à 3 minutes. Toute l'API se fige, appel et connexion compris, pour tous les établissements.
- **Correction** : `spawn_blocking` dans `stocker_objet` et `lire_objet` (18 points d'appel), ou client asynchrone.
- **Effort** : 0,5 à 1 jour.

### R-07 — Envois externes dans des transactions ouvertes — élevée
- **Où** : code de vérification envoyé dans un `TenantTx` (`ent-db/src/dossier_coordonnees.rs:1138-1168`) ; jusqu'à 50 envois SMTP dans une transaction avec verrous de lignes (`inscriptions_reactions.rs:119-210`) ; transaction de lot du distributeur ouverte pendant tous les consommateurs (`outbox.rs:496-607`).
- **Scénario** : SMTP lent, cinq parents demandent un code : les cinq connexions sont prises. Si le lot échoue au 40ᵉ message, la transaction est annulée et les 39 premiers courriels sont renvoyés au passage suivant.
- **Correction** : écrire et valider, puis envoyer hors transaction ; marquer chaque envoi dans sa propre transaction.
- **Effort** : 1 à 2 jours.

### R-08 — Sauvegarde `pg_dump` sous `FORCE ROW LEVEL SECURITY` — élevée (à confirmer sur base réelle)
- **Où** : `ent-rust/scripts/sauvegarde-postgres.sh:41`, `scripts/pra-exercice.sh:138`.
- **Fait PostgreSQL** : `pg_dump` pose `row_security = off` ; un propriétaire soumis à `FORCE` reçoit une erreur. Avec `--enable-row-security` et un rôle en `app.rls_strict = on`, le dump réussit sans aucune ligne des tables d'établissement.
- **Scénario** : une sauvegarde vide que rien ne signale.
- **Correction** : rôle de sauvegarde dédié `BYPASSRLS` en lecture seule, distinct de `ent_app` et `ent_owner` ; contrôle du nombre de lignes dans le manifeste ; à inscrire dans `PRA_SAUVEGARDES.md`.
- **Effort** : 0,5 jour et un test.

### R-09 — Un établissement en erreur bloque les tâches de fond de tous les suivants — élevée
- **Où** : `ent-db/src/notifications.rs:765` ; `ent-db/src/tenant_tx.rs:189-196, 213-218` (`?` dans la boucle) ; `ent-db/src/scolarite.rs:1812-1838`.
- **Scénario** : une ligne illisible chez A fait échouer son passage ; les établissements triés après A ne reçoivent plus ni notification (e-mail, SMS, push) ni tâche quotidienne, sans autre alerte qu'un `warn`.
- **Correction** : isoler l'erreur par établissement, métrique d'échec par contexte.
- **Effort** : 0,5 jour.

### R-10 — Notifications réclamées puis perdues — moyenne
- **Où** : `ent-db/src/notifications.rs:779-822` (réclamation validée avant l'envoi ; « interrompu » passe en `echec`, jamais renvoyé).
- **Scénario** : redémarrage après la réclamation d'un lot de 100 ; un SMS d'absence ou d'urgence n'est jamais envoyé.
- **Correction** : retour en `en_attente` avec compteur de tentatives, ou file d'échecs visible et alertée.
- **Effort** : 1 jour.

### R-11 — Écritures métier et événements non atomiques dans le code hérité — moyenne
- **Où** : `ent-db/src/service.rs:4888-4900`, `:12803-12850` (affectation d'internat : comptage puis insertion sans verrou), `:11987-12030`, `:13349-13395`, `:9209-9260`, `:5919-6040` ; `ent-db/src/scolarite.rs:1747-1779` (statut validé avant la publication de l'événement) ; solveur d'emploi du temps en boucle sans transaction (`service.rs:7862-7926`).
- **Scénario** : deux affectations simultanées dépassent la capacité d'une chambre ; une panne entre l'insertion et `events.publish` perd la notification ou l'entrée de journal (E7).
- **Correction** : généraliser le modèle de `create_eleve` (une `TenantTx`, événement publié dans la transaction).
- **Effort** : 3 à 5 jours, domaine par domaine.

### R-12 — Fuseau : plusieurs horloges pour « aujourd'hui » — moyenne
- **Où (SQL)** : `current_date` au lieu de `ent_aujourdhui_etab()` dans `111_classe_id.sql:20, 44, 122, 197`, `280:155, 282, 337`, `281:213, 242, 253`, `282:86`, `284:98`, `286:108-118, 190`, `340:29, 317`, `353:187`, `098:36`. Aucun réglage du fuseau de session n'est fait par l'application.
- **Où (Rust)** : `domain-eleve/src/absence.rs:288` (`today_utc()` rend en réalité le jour de Paris ; environ 76 appels) ; majorité en UTC (`ent-db/src/inscriptions.rs:2879`) ; Paris en dur (`vie_scolaire.rs:1472`, `formulaires.rs:721`, `ent-api/src/ical.rs:83-110`) alors que le fuseau est un paramètre d'établissement.
- **Scénario** : entre 0 h et 2 h, une affectation close ou le test « année courante » prennent la veille ; le 1ᵉʳ septembre à 0 h 30, un élève est rattaché à l'année précédente. Un établissement d'outre-mer est décalé toute l'année selon le chemin de code.
- **Correction** : une seule source, le jour de l'établissement ; renommer `today_utc` ; ou retirer le paramètre de fuseau tant que la V1 est en métropole (D-052).
- **Effort** : 2 à 3 jours.

### R-13 — Tâches transverses : coût proportionnel au nombre d'établissements, quadratique en mode `off` — moyenne
- **Où** : `ent-db/src/tenant_tx.rs:181-198` (requête complète exécutée une fois par établissement, puis triée en Rust) ; `283_eleve_scolarisation.sql:431-444` (`ent_eleve_statut_recalculer()` sans paramètre) ; `notifications.rs:757-776` (deux transactions par établissement toutes les 5 s, même à vide).
- **Scénario** : à 300 établissements, 300 parcours de `ent_eleve` par passage et environ 120 transactions par seconde à vide sur un pool de 5.
- **Correction** : paramètre `p_etab` dans les fonctions de balayage ; ne parcourir que les établissements qui ont du travail.
- **Effort** : 1 à 2 jours.

### R-14 — Liens d'invitation : repli sur `http://localhost:8088` — moyenne
- **Où** : `ent-db/src/inscriptions_reactions.rs:218-223`.
- **Scénario** : `ENT_PUBLIC_BASE_URL` oubliée en production ; les familles reçoivent des liens morts en HTTP.
- **Correction** : variable obligatoire (https) hors labo, au démarrage.
- **Effort** : 2 heures.

### Constats faibles (robustesse)
- **R-15 — Événement en échec** : après 8 tentatives, tous les suivants du même agrégat attendent indéfiniment (`outbox.rs:509-571`) ; voulu pour l'ordre, mais signalé par une seule ligne `error`. Alerte sur l'âge du plus vieil événement bloqué. 0,5 j.
- **R-16 — Tâches de fond sans supervision** (`ent-api/src/main.rs:197-285`) : une panique arrête la tâche sans relance ni alerte ; pas de verrou entre instances. 0,5 j.
- **R-17 — `migrate().await.ok()?`** avale l'erreur de migration (`ent-api/src/lib.rs:344, 3585`, contexte de tests à confirmer). 15 min.
- **R-18 — Paniques atteignables par une saisie** (à confirmer) : `ent-db/src/referentiel_repo.rs:164-165`, `dossier_personnes_ports.rs:185`. 1 h.
- **R-19 — `map_db` classe les erreurs par sous-chaînes du message** (`ent-api/src/routes.rs:12476-12637`, une soixantaine de `msg.contains(…)`) : un libellé modifié change le code HTTP. Variantes typées. 2 à 3 j.
- **R-20 — SMTP** : transport reconstruit à chaque message, sans délai explicite (`ent-mail/src/smtp.rs:50-63`). 2 h.
- **R-21 — Jointure sur un libellé** : `ent_classe.code = btrim(c.classe)` (`ent-db/src/edt_masse.rs:320`) ; doublons possibles entre deux années. 0,5 j.
- **R-22 — 537 index créés sans `CONCURRENTLY`** : tout nouvel index sur une grosse table bloquera les écritures au déploiement. Avec R-01.
- **R-23 — Délégation** : une première transaction abandonnée sans `commit` quand la seconde est rouverte (`ent-db/src/delegation.rs:322, 355`). 1 h.

---

## 6. Simplification, refactorisation, performance

| Réf. | Constat | Où | Proposition | Effort |
|---|---|---|---|---|
| F-01 | `service.rs` : 20 500 lignes, façade unique ; `routes.rs` : 13 339 lignes, plus de 400 handlers presque identiques | `ent-db/src/service.rs`, `ent-api/src/routes.rs` | Découper par domaine (D-026), comme les modules récents | 3–5 j, au fil des lots |
| F-02 | Trois façons de passer l'acteur (`Acteur`, triplet dans un ordre variable, `CtxSession`) ; `EntSession.actor_user_id: String` doublonne `user_id` | partout | Un type `Acteur` construit depuis la session | 2 j |
| F-03 | Environ 300 méthodes contrôlées ne portent pas le suffixe `_for_actor` ; d'anciens points d'entrée sans acteur restent publics | `ent-db` | Convention unique ; rend la revue « route par route » mécanisable | 1–2 j |
| F-04 | Double contrôle handler + service ; motif « handler → service → `enrich_*` » copié environ 150 fois | `routes.rs` | Enrichissement dans le service | 2–3 j |
| F-05 | Droits effectifs rechargés plusieurs fois par requête | `ent-api/src/auth.rs:196-222`, `ent-db/src/service.rs:9359-9374` | `a_action_quelconque` lit le contexte de requête | 0,5 j |
| F-06 | `TenantTx::begin(...).await.map_err(db)?` répété plus de 1 000 fois ; une transaction par lecture | `ent-db` | Aide `lire(etab, |tx| …)` ; un contexte par requête HTTP | 2–3 j |
| F-07 | `ui.rs` : 2,5 Mo de JavaScript dans une constante, 1 979 `innerHTML`, impose 16 Mio de pile par fil | `ent-api/src/ui.rs`, `main.rs:41` | Retrait module par module (F-001), d'abord hors des binaires sans `labo` | continu |
| F-08 | `ent-brain` n'est pas de l'IA : réacteur dont le résultat est jeté, journal en mémoire sans borne | `ent-brain/src/runtime.rs:33`, `journal.rs:15-30` | Supprimer ou renommer | 0,5 j |
| F-09 | `ent-bank::ingest_csv` : branche « live » identique à l'essai, `if` vide | `ent-bank/src/lib.rs:101-103, 176-184` | Nettoyer | 1 h |
| F-10 | Contrats JSON recopiés à la main ; `AuthSessionDto`, `LoginResponseDto` en double ; `LoginBody` diverge | `web/crates/ent-web-api/src/lib.rs:3-6`, `routes.rs:2131, 2166`, `ent-types/src/auth.rs:31, 63` | Finir la migration vers `ent-types` (D-015) | 1–2 j |
| F-11 | 7 fonctions d'échappement CSV, 2 `code_six_chiffres` | `domain-eleve`, `ent-db/src/inscriptions.rs:150`, `dossier_coordonnees.rs:1496` | Une seule de chaque (voir D-08) | 0,5 j |
| F-12 | Export ICS : jusqu'à 63 jours, un contrôle de droits et une lecture complète par jour | `routes.rs:8510-8566` | Charger une fois, projeter | 0,5 j |
| F-13 | Requêtes en N+1 : réinscription (5 par enfant), liste des documents (1 à 3 par document), cahier (2 transactions par enfant) | `routes.rs:10374-10454`, `service.rs:9762-9796`, `cahier.rs:601-628` | Regrouper | 1 j |
| F-14 | `get_dossier` sérialise la structure interne puis retire des clés à la main | `routes.rs:1509-1629` | DTO explicite (une fuite au prochain champ ajouté, sinon) | 0,5 j |
| F-15 | 240 `include_str!` tenus à la main | `ent-db/src/migrate.rs` | Liste générée au build (avec R-01) | 0,5 j |
| F-16 | Route `decider_dossier_conseil` qui répond 410 ; `let _ = Duration::days(0);` | `routes.rs:6009`, `edt_masse.rs:340` | Supprimer | 15 min |

---

## 7. Examiné et jugé correct

- **Sessions** : jeton de 256 bits, seul le SHA-256 est stocké ; expiration et inactivité contrôlées en base ; un accès suspendu, fermé ou échu invalide les sessions ouvertes ; une session = une casquette, jamais l'union ; session d'enrôlement et session « arrivée » bornées côté serveur.
- **Mots de passe** : Argon2id, sel aléatoire, borne de taille avant hachage, hachage factice pour les comptes inconnus, verrou (compte, IP) et ralentissement progressif, réinitialisation à usage unique consommée par un `UPDATE` atomique qui révoque les sessions.
- **Passkeys** : `webauthn-rs`, vérification de l'utilisateur exigée, défi à usage unique lié au compte et à l'établissement, compteur de signature contrôlé sous verrou, plafond de 10, configuration refusée hors https en production, récupération en libre-service avec délai de sécurité et annulation (D-018).
- **OIDC** : `state`, `nonce`, PKCE, cookie de liaison `__Host-`, RS256 seulement, émetteur et audience vérifiés, garde SSRF avec résolution DNS épinglée et redirections contrôlées, secret client chiffré, réponse plafonnée.
- **Limitation de débit** : seaux en base, atomiques, clés hachées ; IP cliente prise à droite de `X-Forwarded-For` et seulement derrière un proxy déclaré.
- **Fichiers** : type déduit des octets, extensions actives refusées, macros Office et PDF actifs repérés, nom assaini, plafonds par usage ; liens signés HMAC à durée courte, clé vérifiée contre l'établissement, `nosniff`, CSP `sandbox`, `Content-Disposition` forcé pour tout type actif ; en production, stockage S3 ou refus de démarrer.
- **En-têtes et erreurs** : `no-store`, corps des 500 remplacé par un message générique, journaux par gabarit de route (jamais l'URI ni la query), identifiant de requête assaini.
- **Frontend Leptos** : cookie `__Host-`, `HttpOnly`, `Secure`, `SameSite=Lax` ; jeton retiré des réponses ; CSRF par origine et en-tête ; CSP à nonce ; aucun jeton en stockage du navigateur ; une seule fonction serveur, sans secret ; aucun accès à la base.
- **Séparation labo / production** : données de démonstration, mot de passe de labo, `/labo/bootstrap` et `/auth/register` sous `cfg(feature = "labo")` ; le Dockerfile et la CI vérifient leur absence du binaire ; `ENT_ENV` fermé par défaut.
- **SQL** : aucune injection trouvée ; les `format!` n'interpolent que des constantes ; `LIKE` échappés (sauf la recherche de personnel, D-06).
- **RLS** : forme identique partout (`USING` = `WITH CHECK`, `ENABLE` + `FORCE`), aucun `SECURITY DEFINER`, aucun `BYPASSRLS`, `set_config(…, true)` limité à la transaction ; les contextes système sont tous des tâches de fond légitimes ; registre des tables testé en CI.
- **Concurrence** : numéros de facture et d'avoir par compteur atomique, places d'inscription sous `FOR UPDATE`, réclamations de file en `SKIP LOCKED`, consommateurs d'événements idempotents, garde du dernier administrateur sous verrou.
- **Finances** : unicités (facture, quittance, RUM, un mandat actif par foyer, un encaissement par prélèvement), immuabilité des pièces émises, IBAN masqués en sortie.
- **Familles de routes sondées sans défaut** : notes et évaluations, appel, cahier de textes, carnet, sorties, stages, messagerie, voyages, délégations, rôles paramétrables, copilote, PRA, comptes du personnel.
- **IA** : aucun appel externe ni clé ; le copilote est déterministe, en lecture seule, réservé au personnel et borné par les droits de l'acteur.
- **Horodatages** : aucune colonne `TIMESTAMP` sans fuseau.
