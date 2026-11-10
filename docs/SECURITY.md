# Sécurité

Les données élèves/familles/personnels, résultats, absences, incidents, documents et paiements exigent un haut niveau de protection.

Contrôles : authentification robuste, sessions protégées, révocation, récupération de compte, autorisation côté serveur à chaque opération, isolation multi-tenant, moindre privilège, validation des entrées, encodage des sorties, protection CSRF/XSS/SSRF/injection selon surfaces, rate limiting, secrets centralisés, chiffrement en transit/au repos, logs minimisés, dépendances suivies, sauvegardes chiffrées et restauration testée.

Tester les accès inter-tenant sur API, identifiants, recherche, exports, jobs, fichiers, URLs signées, notifications, caches et IA. Le filtrage frontend ne suffit pas.

Fichiers : bucket privé, autorisation avant URL signée courte, taille/MIME/type réel, noms non fiables, quotas, scan/quarantaine selon risques, aucun accès public permanent implicite.

Dev : données synthétiques, base locale séparée, pas de secrets production, sandbox moteurs conversion (non privilégié, limites CPU/RAM/temps, filesystem temporaire, réseau désactivé par défaut).

Référence audit : https://github.com/cloudflare/security-audit-skill. Vérifier licence, provenance, compatibilité et comportement avant installation. Approche : reconnaissance, analyse multi-angles, validation contradictoire, structuration, vérification indépendante, rapport. L'IA n'est pas une certification. Audits ciblés après domaines sensibles, complets avant pilote/prod/changements majeurs, complétés par SAST, dépendances, secrets, fuzzing et tests d'autorisation.

Chaque constat indique gravité, exploitabilité, actif exposé, scénario, preuve, impact, recommandation et confiance. Aucun test destructif, exploitation réelle ou scan externe sans autorisation.

## Vague 1 du socle de confiance (2026-11-04) — tranche 2

Niveau : **implémenté + testé en local** ; non commitée à la date de rédaction (commit local prévu sur `ENT`, non poussé) ; non validé pilote ; non prêt production ; aucun audit complet.

**Constats corrigés**

| Constat | Gravité | Correction | Preuve |
|---|---|---|---|
| `storage_key` fourni par le client et lecture de fichiers hors de leur document (lot L1) | Critique | Clé jamais fournie par le client (retirée de `DeposerDocumentBody`, forcée à `None` dans le dépôt public). Lecture seulement si la clé appartient au document et à son établissement (`document_storage_key_is_owned`, `photo_storage_key_is_owned`, `pai_storage_key_is_owned`). `ent-object` `assert_safe_key` refuse `:`, `\`, `..`, segments vides ou `.`, NUL, chemins absolus, plus de 512 caractères. `NotFound` sans clé dans le message. Clés retirées des réponses (dossier ; trombinoscope : marqueur) | Suite backend du 2026-11-04. 22 documents de test à clé hostile neutralisés (sans modification de données) |
| Accès aux données personnelles de l'annuaire (lot L2) | À qualifier | `/eleves/search` et `/eleves/labels` réservés au personnel. `/eleves/{id}/absent` : personnel, parent lié, élève lui-même. Identité par niveau en un point (`PgEnt::niveau_identite`) : Complet (Admin, Direction, Administratif, parent pour ses enfants, élève pour lui-même), Sante (Infirmerie : sexe et date de naissance), Reduit (Prof, Compta, Cpe, Psy), appliqué à la recherche, à la fiche élève, à l'effectif de classe et au dossier (continuum mef/régime/redoublement masqué hors Complet ; `redoublant` à null). Recherche par badge réservée au niveau Complet. Jokers LIKE échappés. Foyers réservés à Admin, Direction, Administratif, Compta, Cpe. Journal et points de vie scolaire : `peut_voir_vs` exigé pour un parent. Dossier de la Compta réduit (identité, scolarité, famille, repas, bourses) | Matrice L8 ; tests ent-api |
| Labo et inscription libre accessibles hors production (lot L4, reprise de F-005) | Élevée | Feature Cargo `labo` désactivée par défaut (ent-db, ent-api, ent-lab, ent-web-server). Sans elle, `/labo/bootstrap` et `/auth/register` répondent 404 ; aucune constante ni aide de labo. Binaire de production vérifié : sans mot de passe de développement du labo, sans `/labo/bootstrap`, sans `seed_labo_cast`, sans `/auth/register` (contrôle automatique en CI et dans le Dockerfile). Rattachement d'un parent sans mot de passe : secret aléatoire inconnu (plus de mot de passe public). Provisionnement du personnel via `POST /etablissement/personnel` | Contrôle CI et Dockerfile ; tests avec et sans `labo` |
| Erreurs internes, en-têtes et journaux (lot L6) | À qualifier | `DomainError::Infrastructure` et `Reessayer`. SQLSTATE classe 22 → 400, classe 40 → 503, reste → 500 générique « Erreur interne. Référence : <request id> » (détail au journal seulement). `Cache-Control: no-store` et `Pragma` sur l'API et `/web` (pas sur les statiques). Referrer-Policy et Permissions-Policy. Journal : span INFO avec méthode, route gabarit (MatchedPath, jamais la query), request id assaini. Traçabilité des consultations santé et psy (fiche santé, PAI, pièces PAI, passages infirmerie, entretiens, dossier psy), des téléchargements et des exports réglementaires. Échecs de connexion et verrous journalisés seulement pour un compte membre de l'établissement, au plus une fois par 5 min, avec empreinte SHA-256 tronquée de l'e-mail. Journal `ent_journal_modif` en ajout seul (migration 091, trigger anti UPDATE/DELETE). `Debug` masqué pour les types à secrets | Suite backend du 2026-11-04 |
| Séparation inter-établissements non couverte par des tests systématiques (lot L8) | — | Matrice `ent-rust/crates/ent-api/tests/separation.rs` : 27 tests, plus de 1 700 cas rôles × ressources × établissements ; invariants 401 sur toutes les routes non publiques ; `etablissement_id` ignoré ; oracle de fuite | Matrice L8 |

**Validation (2026-11-04, base de test locale, conditions du job CI backend `ENT_ENV=test`)**
- Backend : `cargo test --workspace --features ent-api/labo,ent-lab/labo` : 344 réussis, 0 échec. Sans `labo`, sous-ensemble `ent-api` + `ent-db` + `ent-object` : 51 réussis (les tests qui exigent le labo sont exclus par construction ; la suite complète est celle avec `labo`).
- Web : 89 réussis ; WASM OK ; clippy 0 erreur.
- Ancienne interface vérifiée au navigateur : aucune régression bloquante.
- Revues Sonnet : 1 bloquant trouvé et corrigé ; plus aucun bloquant à la date de rédaction.
- Non fait : exécution de la CI GitHub sur ces changements ; test pilote.

**Limites restantes** (gravité indicative)

| Gravité | Limite |
|---|---|
| Élevée | L3 non traité : e-mail de l'administrateur modifiable par un administratif ; réinitialisation du mot de passe d'un parent par la direction ; champ `password` du rattachement. Deux cas de test L3 restent ignorés |
| Moyenne | Journal : l'application peut encore désactiver le trigger (pas de rôle de migration séparé) ; `TRUNCATE` non bloqué ; durée de conservation non définie ; e-mail en clair pour les connexions réussies (`audit_acces`) |
| Faible | Empreinte e-mail non salée (HMAC à décider) ; libellés « Seed labo » dans l'ancienne UI ; mots de passe préremplis dans les applis mobiles ; date de début des périodes de classe visible aux rôles réduits |
| Importante (avant pilote) | Jeton de session en `localStorage` **uniquement dans l'ancienne UI `/app`** (sans CSP, figée) ; le frontend Leptos utilise un cookie `HttpOnly` depuis A6 (2026-11-05) |

**Non fait** : audit complet ; L3 ; durée de conservation du journal ; aucun changement documenté sur la limitation de débit.

**Hypothèse / recommandation** : la matrice L8 couvre les routes qu'elle énumère ; elle ne prouve pas l'absence de défaut de séparation ailleurs. Étendre la matrice à chaque domaine avant pilote. Trancher les décisions en attente (rôle de migration, durée de conservation, HMAC de l'empreinte) listées dans `PROJECT_STATUS.md`.

## État vérifié (2026-11-03) — tranche 1 du frontend

Niveau : **implémenté + testé en local** ; aucun audit complet, non validé pilote, non prêt production. Rien poussé.

**Constats corrigés**

| Constat | Gravité | Correction | Preuve |
|---|---|---|---|
| Porte du labo (`POST /labo/bootstrap`) et inscription libre ouvertes si `ENT_ENV` absent (fail-open) | Élevée | `gate_open` fail-closed : ouverte seulement si `ENT_ENV` appartient à `ENVS_NON_PROD` (lab/labo/dev/local/test) ou dérogation `=1` ; bandeau labo aligné ; test de parité de la constante | tests `gate_*` ; `ent-rust-prod-runbook.md`, `ent-rust/README.md`, `ent-rust-quality.md` cohérents |
| Accès horizontal à l'appel : lire, pointer, clore permis à tout professeur de l'établissement connaissant l'`appel_id` | Élevée | `autoriser_acces_appel` (titulaire / remplaçant / supervision) appliqué partout, 403 sinon | tests ent-db, `appel_d_un_collegue_inaccessible` |
| Garde « base locale » des tests par sous-chaîne | Moyenne | `ent_db::assert_local_postgres_url` ne contrôle que l’hôte loopback (`url_guard.rs:17-29`) ; le contrôle du NOM de base (test/proto/lab) n’existe que dans les tests `contre_backend.rs` et `session_cookie.rs` ; les autres consommateurs (ent-lab, scénarios ent-db) n’ont que la garde d’hôte ; `seed_labo_web` : hôte exact | `contre_backend.rs`, `session_cookie.rs` |
| Pas de CSP sur le frontend Leptos | Moyenne | CSP stricte à nonce par requête sur `/web` et `/pkg` (`script-src 'self' 'nonce-…' 'wasm-unsafe-eval'`, `style-src 'self'`, `object-src 'none'`, `frame-ancestors 'self'`…) | `entetes_securite` (2 tests) |
| Redirections ouvertes après connexion, envois répétés après 401/403/429, panique sur `?semaine=`, fuite par `Debug`, messages serveur non bornés | Moyenne / faible | `retour_sur` durci, arrêt des envois, bornage, `Debug` masqué | tests ent-web-app |

**Limites restantes** (liste complète : `PROJECT_STATUS.md`)
- Importante : jeton de session en `localStorage` **limité à `/app`** (figée) ; frontend Leptos passé au cookie `HttpOnly` (A6, 2026-11-05, essai navigateur réel à faire).
- Importante : `/app` (ancienne UI JS) partage l'origine et le jeton, sans CSP.
- Moyenne : consultation d'une feuille d'appel via `POST` (écriture) ; `config_connexion` expose `ENT_WEB_ETABLISSEMENT_ID` (D-012) ; passkey non testée au navigateur.
- Non fait : audit complet, test inter-établissement sur toutes les routes de l'appel, limitation de débit vérifiée.

**Hypothèse / recommandation** : d'autres routes d'`ent-db` peuvent avoir le même défaut de contrôle « horizontal » que l'appel ; revue ciblée par domaine à prévoir.

## Liens courts d'accès aux fichiers privés (socle 1 sécurité)

Niveau : **implémenté + testé en local** (sans réseau) ; non validé pilote ; non prêt production. Exigence : « URL pré-signées courtes émises après authentification et contrôle des droits ».

**Faits**
- Quatre routes authentifiées émettent un lien : `POST /documents/{id}/lien`, `POST /eleves/{id}/photo/lien`, `POST /pai/pieces/{id}/lien`, `POST /messages/pieces/{id}/lien`. Réponse : `{ "url", "expires_at" }` (secondes Unix). Chacune réutilise la fonction de contrôle d'accès **partagée** avec la route de téléchargement correspondante (extraite, non dupliquée) ; le client ne fournit jamais de clé de stockage. Les routes de téléchargement existantes (`…/fichier`, `/photo`, `/pai/pieces/{id}`, `/messages/pieces/{id}`) sont inchangées.
- L'émission est journalisée (qui, quoi, sans jeton ni clé objet) : `lien_document`, `lien_pai_piece`, `lien_piece_message` ; la photo n'était pas tracée, elle ne l'est pas non plus.
- Mode local (`ENT_OBJECT_PRESIGN_MODE=local`, défaut) : `GET /fichiers/{jeton}`, route publique. Jeton = charge + HMAC-SHA256 (domaine séparé, comparaison en temps constant) couvrant clé objet, expiration, user_id, établissement, disposition, nom affiché, type MIME. Réponses : 410 (expiré), 403 (invalide/altéré/autre secret), 404 (désactivé/absent). L'objet doit appartenir à l'établissement signé (`{etab}/…`, `reinscription/{etab}/…`, `lab/{8 hex}/…`), sinon refus (l'émission refuse aussi de signer une clé d'un autre établissement ou une clé hostile).
- En-têtes servis : `Content-Type` et `Content-Disposition` signés, `X-Content-Type-Options: nosniff`, `Cache-Control: private, no-store`, `Referrer-Policy: no-referrer`, `Cross-Origin-Resource-Policy: same-site`, CSP `default-src 'none'; sandbox`. Exception : un PDF affiché en ligne reçoit `default-src 'none'; frame-ancestors 'self'` (le lecteur PDF des navigateurs est bloqué par `sandbox`). `inline` n'est accordé qu'à une liste blanche (pdf, png, jpeg, gif, webp, text/plain) ; html, svg, xml et toute extension active sont forcés en `attachment`, aussi à la lecture du jeton.
- Durée : 60 s par défaut (`ENT_OBJECT_LINK_TTL_SECS`), plafonnée à 900 s.
- **Fail-closed** : sans `ENT_OBJECT_SIGNING_KEY` d'au moins 32 octets, aucun lien n'est émis (HTTP 503) sauf si `ENT_ENV` vaut `lab`, `labo`, `dev`, `local` ou `test` (clé éphémère aléatoire propre au processus). Aucun secret dans `Debug` ni dans les journaux.
- Mode S3 (`ENT_OBJECT_PRESIGN_MODE=s3`) : URL pré-signée AWS Signature V4 (`X-Amz-Expires`, `response-content-disposition`, `response-content-type`), path-style ou virtual-host, endpoint par défaut `https://s3.{région}.scw.cloud`. Implémentée à la main (`hmac` + `sha2`, déjà dans l'arbre de dépendances) et vérifiée sur le vecteur officiel AWS « GET object » ; aucun appel réseau dans les tests.

**Limites**
- Le jeton local est un **porteur** : quiconque le détient pendant sa courte durée de vie lit le fichier (le user_id signé sert à l'audit, pas à un contrôle d'identité). Le copier-coller d'un lien dans un canal non sûr reste un risque, borné par l'expiration.
- La clé objet figure **en clair** dans la charge du jeton (base64url n'est pas un chiffrement) ; elle n'est que `{etab}/{ns}/{uuid}.{ext}`, non devinable, sans donnée personnelle.
- Pas de révocation individuelle avant expiration ; pas de limitation de débit sur `GET /fichiers/{jeton}` ; pas de requêtes `Range`.
- **Stockage (F-013)** : en production le magasin est S3 (SigV4), jamais le disque local ; objets immuables (clé `{etab}/{espace}/{uuid}.{ext}`), SHA-256 et taille enregistrés dans `ent_objet` (migration 500), vérifiés à l'écriture et à chaque lecture par l'application (en mode S3 pré-signé, le téléchargement direct est protégé par la signature et le contrôle du fournisseur ; l'empreinte reste vérifiable hors ligne). Aucun échec de stockage n'est ignoré : erreur HTTP 503/500, journal et métrique `ent_objet_operations`.
- Les pièces de messagerie (et les pièces PAI historiques) sont stockées en base : à l'émission du lien, leur contenu est recopié (idempotent) dans le magasin d'objets sous une clé conforme. Une copie persiste donc hors base.
- `ent-object` n'envoie pas encore les fichiers vers un vrai S3 (PUT sans signature, disque local source de vérité) : le mode S3 produit des URL valides seulement une fois l'envoi/lecture S3 signés implémentés. Non testé contre Scaleway.
- Un document sans octets en magasin (ancien stub de labo) donne 404 sur la route de lien, là où le téléchargement sert un texte de substitution.
- Jeton dans l'URL : il peut apparaître dans l'historique du navigateur ou un proxy ; les journaux de l'API n'enregistrent que le gabarit de route.

## Fichiers déposés et champs sensibles (Socle 1, D2 et D3 — 2026-11-05)
**Fichiers déposés.** Tout contenu fourni par un client (documents, justificatifs, photos, pièces PAI, pièces de messagerie) est contrôlé **avant écriture** : type réel par signature binaire (le type déclaré et l'extension ne sont jamais une preuve), liste blanche par usage (photo = images seulement ; PAI = PDF/PNG/JPEG), taille maximale par usage (`ENT_UPLOAD_MAX_DOCUMENT|PHOTO|MESSAGE|PAI`), nom assaini (`sanitize_filename`) sans segment d'extension actif, refus des fichiers actifs (HTML, SVG, XML, exécutables, scripts, PDF avec JavaScript/lancement en clair, Office/ODF avec macros ou ActiveX, ZIP quelconques). Le type MIME **stocké et servi** est celui des octets. Point d'accroche antivirus : trait `AnalyseurAntivirus` ; implémentation par défaut `AucuneAnalyse` = **pas d'analyse antivirus** (un analyseur indisponible refuse le dépôt). Le service des liens courts (`GET /fichiers/{jeton}`) garde ses en-têtes anti-exécution (voir plus haut). Détail et tests : `docs/SOCLE1_SECURITE.md` (D2).
**Champs sensibles.** IBAN/BIC/titulaire (mandats SEPA, historique), fiche santé et dossier psy (champs libres) sont chiffrés au repos en AES-256-GCM, nonce aléatoire, AAD = table + colonne + id de ligne, clé dérivée de `ENT_SECRETS_KEY` (obligatoire hors labo/test). Valeurs héritées en clair lues puis chiffrées à la première écriture ou par `rechiffrer_champs_sensibles` (au démarrage). Réponses API et journal d'événements : IBAN masqué (`FR76 •••• 1234`). Détail et limites (pas de rotation de clé, pas de KMS, journaux d'événements antérieurs, autres champs de santé non couverts) : `docs/SOCLE1_SECURITE.md` (D3).

## Anti-abus de l'authentification (Socle 1, lot L5)

Niveau : **implémenté + testé en local** (ent-api, ent-db, ent-auth) ; non validé pilote ; non prêt production ; aucun audit complet. Configuration : `ent-rust-prod-runbook.md`.

**Constats corrigés (faits vérifiés par les tests `abus_*`, `client_ip::tests`, `ent-auth`)**

| Constat | Correction |
|---|---|
| E5 : aucune limitation de débit sur `forgot-password`, `reset-password`, `passkey/login/start|finish`, `oidc/{slug}/start`, `login` | Seaux à jetons Postgres (`ent_rate_limit`, migration 094), une requête atomique par contrôle, clés (route, IP) et (route, e-mail ou jeton normalisé) ; 429 + `Retry-After`, message neutre ; quotas par variables `ENT_RATE_<ROUTE>_IP/_ID`. Atomicité vérifiée sous 30 appels concurrents (exactement le quota) |
| M2 : `X-Forwarded-For` falsifiable (première valeur), garde par IP désactivée sans `ENT_TRUST_PROXY` | IP de la socket par défaut (`ConnectInfo`, serveur lancé avec `into_make_service_with_connect_info`) ; derrière proxy de confiance (`ENT_TRUST_PROXY` = nombre ou CIDR) entrée de **droite** ; entrée invalide ou chaîne trop courte → socket ; `X-Real-IP` ignoré ; clé IPv6 en `/64` |
| M1 : 5 essais faux bloquent la victime 15 min | Verrou dur par couple (compte, IP) ; ralentissement progressif par compte toutes IP confondues (200 ms doublé, plafond 3 s) ; pas de seau de débit par compte sur la connexion ni le démarrage passkey |
| M3 : contrôle du verrou et enregistrement de l'échec non atomiques | Incrément atomique (`INSERT … ON CONFLICT … RETURNING`) **avant** la vérification du mot de passe, rendu en cas de succès ; 20 essais concurrents : exactement 5 mots de passe vérifiés. Même mécanisme pour la ré-authentification (`reauth:<id>`) |
| M4 : énumération par `passkey/login/start`, par le temps de `forgot-password`, par `NotMember` | `passkey/login/start` renvoie toujours 200 avec un challenge (factice, non persisté, de même forme que le réel, vérifié) ; `forgot-password` : traitement différé (`tokio::spawn`), 202 immédiat ; `NotMember` → 401 générique identique à des identifiants invalides, cause journalisée (niveau info, identifiants internes seulement) |
| F4 : pas de borne de taille | E-mail ≤ 254 octets, mot de passe ≤ 256 caractères, jeton de réinitialisation ≤ 256 : refus 400 avant tout hachage (`ent-auth`, appliqué à la connexion, l'inscription, le changement et la réinitialisation) |

**Changements de comportement** : `passkey/login/start` ne renvoie plus 401 « aucune passkey » ; `NotMember` n'est plus un 403 distinct à la connexion ; `forgot-password` répond avant l'envoi (une erreur d'envoi n'est que journalisée).

**Limites restantes (faits)**
- Verrou (compte, IP) : un attaquant disposant de nombreuses IP obtient 5 essais par IP et par fenêtre ; au niveau du compte seul joue le ralentissement (par connexion, contournable en parallèle). Pas de plafond global par compte : choix délibéré contre le déni de service ; compensé par la politique de mot de passe et la passkey. Les IPv6 sont groupées par /64.
- La victime qui se connecte depuis l'IP de l'attaquant (NAT partagé) est bloquée avec lui.
- Le seau par e-mail de `forgot-password` (3/h) permet à un tiers d'empêcher une victime de demander un lien pendant une heure (le personnel peut réinitialiser) ; c'est le prix de l'anti-bombardement.
- `ENT_TRUST_PROXY` mal réglé : trop petit, les quotas sont partagés entre clients ; trop grand, usurpation possible. À valider sur l'infrastructure réelle (Scaleway), non vérifié ici.
- La limitation s'ouvre si la base répond en erreur. Le temps du challenge factice (`passkey/login/start`) n'est pas strictement égalé à celui du réel (quelques requêtes de moins) : écart non mesuré.
- Les verrous de connexion renvoient 429 sans `Retry-After` ; seuls les seaux de débit l'émettent.
- `oidc/complete`, `register`, `change_password` et le reste de l'API authentifiée n'ont pas de seau de débit (hors périmètre du lot).
## Socle 1 — A5, A7, A8 partiel (2026-11-05)

Niveau : **implémenté + testé en local** ; non validé pilote ; non prêt production ; aucun audit externe.

**Faits**
- WebAuthn : hors labo/test, démarrage refusé sans `ENT_RP_ID` (domaine public) ni origine `https` ; origines multiples (`ENT_RP_ORIGINS`), sous-domaines optionnels, origines Android natives. Détail : `docs/SOCLE1_SECURITE.md`.
- SSO OIDC : rattachement par e-mail seulement si `email_verified == true` ; retour lié au navigateur par cookie `__Host-ent_oidc` ; URL d'IdP en `https`, cibles internes refusées après résolution DNS (adresse épinglée), redirections et tailles bornées ; `client_secret` chiffré AES-256-GCM (`ENT_SECRETS_KEY`).
- Passkey : compteur de signature régressé → connexion refusée, journalisée (`passkey_compteur_regresse`), compteur mis à jour atomiquement.

**Limites connues**
- `ENT_SECRETS_KEY` : pas de rotation ni de KMS ; perdre la clé rend les secrets clients illisibles ; la clé de dev (labo/test) est publique.
- Cookie `__Host-` : exige HTTPS (Safari refuse `Secure` sur `http://localhost`) et que `start` et `complete` partent du même hôte que `redirect_uri` ; les clients mobiles doivent conserver le cookie pendant le parcours SSO.
- IdP sans claim `email_verified` (ex. Microsoft Entra par défaut) : plus de rattachement par e-mail ; pas encore de rattachement administrateur par `sub`.
- SSRF : contrôle au niveau applicatif (pas de pare-feu de sortie) ; un proxy sortant ou un DNS interne intercepteur n'est pas couvert.
- `ENT_RP_ALLOW_SUBDOMAINS=1` : un sous-domaine détourné pourrait solliciter des passkeys du `rp_id`.
- Régression de compteur : pas de désactivation automatique de la clé ni d'alerte à l'utilisateur ; les passkeys synchronisées (compteur 0) ne sont pas concernées.
- Fichier `apple-app-site-association` (iOS) non fourni.

## Récupération de passkey en libre-service (D-018, Socle 1, lot A8)

Niveau : **implémenté + testé en local** ; non validé pilote ; non prêt production. Détail du parcours : `SOCLE1_SECURITE.md` (lot A8).

- Menace traitée : perte de passkey d'une personne de plusieurs établissements (aucune direction ne peut agir, E4). Menace introduite : prise de contrôle par qui contrôle la boîte e-mail → contre-mesures : confirmation par lien, **délai de sécurité** configurable (`ENT_RECUPERATION_PASSKEY_DELAI_HEURES`, 24 h par défaut), alerte à la personne avec annulation, événement journalisé dans **chaque** établissement, annulation automatique par connexion passkey.
- Réponse publique **202 neutre** (anti-énumération), envoi différé ; limitation de débit dédiée ; trois jetons distincts 256 bits hachés, usage unique ; une seule demande active ; finalisation atomique (passkeys supprimées, toutes sessions révoquées).
- Aucune adresse e-mail en clair dans le journal ni en base de la demande (empreintes). Le dry-run de la messagerie ne conserve les corps de message qu'en `dry_run_only()` (labo/tests), jamais en production.

## Socle 1 — C1 (reste) et C6 : matrice d'isolation étendue (2026-11-05)

Niveau : **implémenté + testé en local** ; non validé pilote ; non prêt production ; aucun audit externe. Détail : `docs/SOCLE1_SECURITE.md` (C1, C6).

**Constats corrigés (trouvés par la matrice étendue, faits vérifiés par test)**
| Constat | Gravité | Correction |
|---|---|---|
| `user_labels` renvoyait le libellé (nom ou e-mail) de n'importe quel utilisateur par UUID, même d'un autre établissement | Moyenne | jointure `ent_membership`, établissement de la session passé aux 17 appels |
| `POST /edt/creneaux/{id}/affectation` acceptait un `prof_user_id` d'un autre établissement (200) | Moyenne | membre de l'établissement exigé |
| `POST /facturation/factures` acceptait une ligne dont `eleve_id` est d'un autre établissement (201) | Moyenne | élève de l'établissement exigé |
| Une consultation sensible d'un `eleve_id` d'un autre établissement laissait une trace dans le journal de l'acteur, listée par `GET /eleves/{id}/journal` | Faible | l'insertion au journal (et l'événement) exige un élève de l'établissement |
| Provisionnement d'une adresse connue dans un autre établissement : 409 « déjà utilisé » (oracle d'existence inter-établissements) | Moyenne | réponse indiscernable d'un succès, aucun compte créé ni modifié, journalisée sans l'adresse |
| SSO : non-membre (403) distinct d'un compte inconnu (401) | Faible | même réponse (401) |
| Le test « 401 sans session » ne couvrait que `routes.rs` : `/roles/*` et `/referentiel/groupes/{id}/responsables` hors inventaire | Faible (test) | inventaire sur 3 fichiers + test de couverture |

**Limites connues**
- Provisionnement neutre : le mot de passe renvoyé pour une adresse connue ailleurs ne sert à rien ; l'administrateur ne voit pas d'échec et la personne n'est pas rattachée tant que l'invitation (acceptée par la personne) n'est pas conçue. Un administrateur peut donc croire un compte créé : à traiter par l'invitation et un message d'interface.
- Le rattachement d'un parent (`POST /foyers/{id}/liens`) et le provisionnement en lot gardent un 409 distinct pour une adresse existant ailleurs.
- Garde statique SQL : syntaxique (littéraux seulement) ; les exceptions sont nominatives et justifiées dans le test (19 à la création de la garde, 32 après l'intégration du lot 3 : +13 pour la récupération de passkey D-018, parcours de la personne tous établissements ; les requêtes de l'outbox et du rechiffrement santé/psy signalées ont été corrigées par un filtre `etablissement_id`) ; sans RLS (C4) ni FK composites (C5), l'isolation repose toujours sur la discipline du code.
- Les doubles membres de la matrice sont du personnel ; parents et élèves n'y sont pas rejoués.

## Socle 1 — C4 : RLS PostgreSQL, pilote (2026-11-05)
Défense en profondeur de l'isolation des établissements : Row-Level Security (activée et forcée) sur 22 tables de santé, psy, sanctions/commissions et facturation, politique `etablissement_id = app.etablissement_id` (variable posée par transaction via `TenantTx`). **Implémenté + testé en local ; désactivé par défaut (`ENT_RLS_MODE`) ; non validé, non prêt production.**
- Rôles : migrations par un rôle propriétaire (`ENT_DATABASE_MIGRATION_URL`, facultatif), application par un rôle non propriétaire, sans `BYPASSRLS`, sans droit de désactiver le trigger du journal (script d'exemple dans `docs/RLS_ISOLATION.md`).
- Limites : la RLS protège des oublis de filtre, pas d'une injection SQL ni d'un code qui contrôle le SQL ; tables d'identité, sessions, passkeys et limitation de débit exclues (lues avant de connaître l'établissement) ; pas encore de test sous rôle non propriétaire pour toute la suite.
- Coexistence avec le chiffrement applicatif (D3) : dans les dépôts santé, psy et facturation, la lecture de l'id existant, le scellement, l'écriture et la reprise en cas de course passent dans **la même** transaction `TenantTx`. Le rechiffrement au démarrage (`rechiffrer_champs_sensibles`) n'ouvre pas de `TenantTx` : il fonctionne en mode `off`/`permissif` ; en mode strict il ne verrait aucune ligne des tables RLS (voir `docs/RLS_ISOLATION.md`, 2.7).
- Conception, plan d'extension et performance : `docs/RLS_ISOLATION.md`.
