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
