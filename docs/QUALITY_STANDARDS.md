# Qualité et livraison

Une fonctionnalité est terminée lorsque UI, métier, API, persistance, validation, permissions, erreurs, cas limites, historique, tests, sécurité, performance et documentation sont traités selon son périmètre.

États à distinguer : découvert, proposé, en cours, implémenté, testé, validé, prêt pilote, prêt production. Un écran seul ne constitue pas une fonctionnalité terminée.

Vérifications déterministes quand l'environnement est sûr : formatage, compilation, lint, tests unitaires/intégration, permissions, multi-tenant, analyse statique, dépendances, charge et revue du diff. Ne jamais lancer de commande susceptible de toucher la prod ou des secrets.

Boucle : définir critères, utiliser des outils déterministes, corriger l'unité fautive, limiter à trois tentatives sans progrès puis arrêter et revoir le plan. Vérifier le diff final.

Tester milliers d'élèves, utilisateurs simultanés, plusieurs établissements, changements d'année, responsables multiples, transferts, historique, imports et jobs asynchrones. Avant pilote : observabilité, alertes, sauvegarde/restauration, incidents et support.
