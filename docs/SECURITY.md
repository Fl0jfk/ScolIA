# Sécurité

Les données élèves/familles/personnels, résultats, absences, incidents, documents et paiements exigent un haut niveau de protection.

Contrôles : authentification robuste, sessions protégées, révocation, récupération de compte, autorisation côté serveur à chaque opération, isolation multi-tenant, moindre privilège, validation des entrées, encodage des sorties, protection CSRF/XSS/SSRF/injection selon surfaces, rate limiting, secrets centralisés, chiffrement en transit/au repos, logs minimisés, dépendances suivies, sauvegardes chiffrées et restauration testée.

Tester les accès inter-tenant sur API, identifiants, recherche, exports, jobs, fichiers, URLs signées, notifications, caches et IA. Le filtrage frontend ne suffit pas.

Fichiers : bucket privé, autorisation avant URL signée courte, taille/MIME/type réel, noms non fiables, quotas, scan/quarantaine selon risques, aucun accès public permanent implicite.

Dev : données synthétiques, base locale séparée, pas de secrets production, sandbox moteurs conversion (non privilégié, limites CPU/RAM/temps, filesystem temporaire, réseau désactivé par défaut).

Référence audit : https://github.com/cloudflare/security-audit-skill. Vérifier licence, provenance, compatibilité et comportement avant installation. Approche : reconnaissance, analyse multi-angles, validation contradictoire, structuration, vérification indépendante, rapport. L'IA n'est pas une certification. Audits ciblés après domaines sensibles, complets avant pilote/prod/changements majeurs, complétés par SAST, dépendances, secrets, fuzzing et tests d'autorisation.

Chaque constat indique gravité, exploitabilité, actif exposé, scénario, preuve, impact, recommandation et confiance. Aucun test destructif, exploitation réelle ou scan externe sans autorisation.

## État vérifié (2026-11-03) — tranche 1 du frontend

Niveau : **implémenté + testé en local** ; aucun audit complet, non validé pilote, non prêt production. Rien poussé.

**Constats corrigés**

| Constat | Gravité | Correction | Preuve |
|---|---|---|---|
| Porte du labo (`POST /labo/bootstrap`) et inscription libre ouvertes si `ENT_ENV` absent (fail-open) | Élevée | `gate_open` fail-closed : ouverte seulement si `ENT_ENV` appartient à `ENVS_NON_PROD` (lab/labo/dev/local/test) ou dérogation `=1` ; bandeau labo aligné ; test de parité de la constante | tests `gate_*` ; `ent-rust-prod-runbook.md`, `ent-rust/README.md`, `ent-rust-quality.md` cohérents |
| Accès horizontal à l'appel : lire, pointer, clore permis à tout professeur de l'établissement connaissant l'`appel_id` | Élevée | `autoriser_acces_appel` (titulaire / remplaçant / supervision) appliqué partout, 403 sinon | tests ent-db, `appel_d_un_collegue_inaccessible` |
| Garde « base locale » des tests par sous-chaîne | Moyenne | `ent_db::assert_local_postgres_url` + nom de base test/proto/lab ; `seed_labo_web` : hôte exact | `contre_backend.rs`, `session_cookie.rs` |
| Pas de CSP sur le frontend Leptos | Moyenne | CSP stricte à nonce par requête sur `/web` et `/pkg` (`script-src 'self' 'nonce-…' 'wasm-unsafe-eval'`, `style-src 'self'`, `object-src 'none'`, `frame-ancestors 'self'`…) | `entetes_securite` (2 tests) |
| Redirections ouvertes après connexion, envois répétés après 401/403/429, panique sur `?semaine=`, fuite par `Debug`, messages serveur non bornés | Moyenne / faible | `retour_sur` durci, arrêt des envois, bornage, `Debug` masqué | tests ent-web-app |

**Limites restantes** (liste complète : `PROJECT_STATUS.md`)
- Importante : jeton de session en `localStorage` (XSS atténué par la CSP, non supprimé) ; bascule vers le cookie HttpOnly à planifier avant pilote.
- Importante : `/app` (ancienne UI JS) partage l'origine et le jeton, sans CSP.
- Moyenne : consultation d'une feuille d'appel via `POST` (écriture) ; `config_connexion` expose `ENT_WEB_ETABLISSEMENT_ID` (D-012) ; passkey non testée au navigateur.
- Non fait : audit complet, test inter-établissement sur toutes les routes de l'appel, limitation de débit vérifiée.

**Hypothèse / recommandation** : d'autres routes d'`ent-db` peuvent avoir le même défaut de contrôle « horizontal » que l'appel ; revue ciblée par domaine à prévoir.
