# Sécurité

Les données élèves/familles/personnels, résultats, absences, incidents, documents et paiements exigent un haut niveau de protection.

Contrôles : authentification robuste, sessions protégées, révocation, récupération de compte, autorisation côté serveur à chaque opération, isolation multi-tenant, moindre privilège, validation des entrées, encodage des sorties, protection CSRF/XSS/SSRF/injection selon surfaces, rate limiting, secrets centralisés, chiffrement en transit/au repos, logs minimisés, dépendances suivies, sauvegardes chiffrées et restauration testée.

Tester les accès inter-tenant sur API, identifiants, recherche, exports, jobs, fichiers, URLs signées, notifications, caches et IA. Le filtrage frontend ne suffit pas.

Fichiers : bucket privé, autorisation avant URL signée courte, taille/MIME/type réel, noms non fiables, quotas, scan/quarantaine selon risques, aucun accès public permanent implicite.

Dev : données synthétiques, base locale séparée, pas de secrets production, sandbox moteurs conversion (non privilégié, limites CPU/RAM/temps, filesystem temporaire, réseau désactivé par défaut).

Référence audit : https://github.com/cloudflare/security-audit-skill. Vérifier licence, provenance, compatibilité et comportement avant installation. Approche : reconnaissance, analyse multi-angles, validation contradictoire, structuration, vérification indépendante, rapport. L'IA n'est pas une certification. Audits ciblés après domaines sensibles, complets avant pilote/prod/changements majeurs, complétés par SAST, dépendances, secrets, fuzzing et tests d'autorisation.

Chaque constat indique gravité, exploitabilité, actif exposé, scénario, preuve, impact, recommandation et confiance. Aucun test destructif, exploitation réelle ou scan externe sans autorisation.
