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
| Importante (avant pilote), inchangée | Jeton de session en `localStorage` ; `/app` sans CSP |

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
- Importante : jeton de session en `localStorage` (XSS atténué par la CSP, non supprimé) ; bascule vers le cookie HttpOnly à planifier avant pilote.
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
- Les pièces de messagerie (et les pièces PAI historiques) sont stockées en base : à l'émission du lien, leur contenu est recopié (idempotent) dans le magasin d'objets sous une clé conforme. Une copie persiste donc hors base.
- `ent-object` n'envoie pas encore les fichiers vers un vrai S3 (PUT sans signature, disque local source de vérité) : le mode S3 produit des URL valides seulement une fois l'envoi/lecture S3 signés implémentés. Non testé contre Scaleway.
- Un document sans octets en magasin (ancien stub de labo) donne 404 sur la route de lien, là où le téléchargement sert un texte de substitution.
- Jeton dans l'URL : il peut apparaître dans l'historique du navigateur ou un proxy ; les journaux de l'API n'enregistrent que le gabarit de route.

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

## Récupération de passkey en libre-service (D-018, Socle 1, lot A8)

Niveau : **implémenté + testé en local** ; non validé pilote ; non prêt production. Détail du parcours : `SOCLE1_SECURITE.md` (lot A8).

- Menace traitée : perte de passkey d'une personne de plusieurs établissements (aucune direction ne peut agir, E4). Menace introduite : prise de contrôle par qui contrôle la boîte e-mail → contre-mesures : confirmation par lien, **délai de sécurité** configurable (`ENT_RECUPERATION_PASSKEY_DELAI_HEURES`, 24 h par défaut), alerte à la personne avec annulation, événement journalisé dans **chaque** établissement, annulation automatique par connexion passkey.
- Réponse publique **202 neutre** (anti-énumération), envoi différé ; limitation de débit dédiée ; trois jetons distincts 256 bits hachés, usage unique ; une seule demande active ; finalisation atomique (passkeys supprimées, toutes sessions révoquées).
- Aucune adresse e-mail en clair dans le journal ni en base de la demande (empreintes). Le dry-run de la messagerie ne conserve les corps de message qu'en `dry_run_only()` (labo/tests), jamais en production.
