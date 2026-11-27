# Mission — Sauvegardes, réplication multi-cloud et plan de reprise d'activité (PRA)

Commande de Florian, 2026-11-10, reproduite telle quelle. **Mission de réflexion, d'audit et de préparation documentaire : aucune implémentation, aucun déploiement, aucune ressource cloud, aucun identifiant de production.**

## 1. Contexte et objectif
ScolIA (ENT) est une plateforme SaaS multi-établissements scolaires. Objectif à long terme : une architecture de sauvegarde et de reprise d'activité de niveau professionnel, adaptée à plusieurs dizaines puis plusieurs centaines d'établissements.
À protéger : les bases PostgreSQL ; les documents et fichiers des établissements ; les données applicatives et métadonnées nécessaires à la reconstruction du service ; les configurations et éléments indispensables à une reprise d'activité.
Priorités : fiabilité, sécurité, réversibilité, maîtrise des coûts, indépendance vis-à-vis d'un fournisseur unique.

## 2. Architecture générale souhaitée
La production pourra d'abord fonctionner sur un seul fournisseur (Scaleway). Les sauvegardes sont répliquées vers **deux fournisseurs indépendants, Scaleway et OVHcloud, en parallèle et en permanence** : pas de primaire/secours.
- les sauvegardes partent vers les deux fournisseurs ;
- chaque destination est indépendante ; une indisponibilité ne bloque pas l'autre ;
- les opérations manquantes sont reprises automatiquement au retour du fournisseur ;
- chaque destination permet seule de restaurer.
Ce n'est **pas** une base PostgreSQL multi-maître ni deux productions acceptant des écritures. Pas de doublement immédiat de l'infrastructure applicative.

## 3. Réplication et indisponibilités
Réplication asynchrone fiable : file de tâches persistante ; identifiants uniques et stables par sauvegarde ; état indépendant par destination ; nouvelles tentatives avec temporisation progressive ; transferts reprenables ; opérations idempotentes ; détection des éléments manquants ; réconciliation automatique après interruption ; contrôles d'intégrité après transfert.
La file ne doit pas dépendre exclusivement du fournisseur dont on veut tolérer la panne ; étudier sa persistance y compris en cas de panne complète de l'infrastructure principale.
États explicites : sauvegarde produite ; copie Scaleway confirmée ; copie OVH confirmée ; copie unique disponible ; double copie confirmée ; réplication en retard ; vérification d'intégrité échouée ; restauration possible ; restauration vérifiée.
Jamais « double sauvegarde confirmée » sans deux copies écrites et vérifiées. En cas de panne d'un fournisseur : continuer sur l'autre, état de protection dégradée clairement visible ; au retour, reprendre les manquants sans recréer l'existant.
Scénarios : panne partielle, panne totale, reprise, doublons, transfert interrompu, divergence entre manifestes.

## 4. PostgreSQL : sauvegardes cohérentes et PITR
Sauvegardes complètes cohérentes périodiques ; archivage continu des WAL ; restauration à un instant précis (PITR) ; réplication des sauvegardes et des WAL vers les deux fournisseurs ; vérification de la continuité des chaînes ; détection des WAL manquants ou corrompus. Une copie de fichiers d'une base en fonctionnement n'est **jamais** une sauvegarde cohérente.
Paramètres à documenter : fréquence des complètes, `archive_timeout`, volume des WAL, rétention, compression, chiffrement, délais de transfert, objectifs de restauration. Restauration possible indépendamment depuis chaque destination.

## 5. Documents et objets S3
Réplication depuis le stockage principal vers deux stockages indépendants : identifiants d'objets stables ; manifestes ; empreintes cryptographiques ; vérification après transfert ; versioning ; gestion des suppressions ; reprise des transferts incomplets ; rétention adaptée ; mécanismes empêchant qu'une suppression ou corruption soit propagée immédiatement à toutes les copies.
Étudier les capacités **réelles** de chaque fournisseur (versioning, immutabilité, Object Lock ou équivalent) ; ne pas supposer que S3 est identique partout.

## 6. Ransomwares et erreurs humaines
Deux copies synchronisées ne suffisent pas. Sauvegardes historiques immuables avec rétentions distinctes. Étudier : immutabilité ; protection contre les suppressions ; comptes et permissions séparés ; identifiants de service à privilèges minimaux ; isolation des environnements ; protection contre la compromission du compte de production ; corruption logique ; chiffrement malveillant ; conservation de générations saines antérieures à un incident.
Évaluer l'intérêt d'une troisième copie indépendante face à son coût ; proposer une évolution progressive.

## 7. Chiffrement et clés
En transit ; au repos ; avant transfert si utile ; séparation des responsabilités et des permissions. Gestion, rotation, sauvegarde et récupération des clés. Les clés nécessaires à la restauration ne dépendent pas exclusivement de l'infrastructure principale. Scénario : Scaleway totalement inaccessible → récupérer les clés et restaurer depuis OVHcloud. Jamais de mécanisme rendant les sauvegardes irrécupérables si un compte ou un fournisseur est perdu.

## 8. Restauration globale et par établissement (point stratégique)
**Globale** : reconstruction de PostgreSQL, documents, configurations, reprise des services, vérification de cohérence, retour en production.
**Sélective d'un établissement** (base mutualisée) : restaurer un seul établissement à un instant donné sans écraser les autres — environnement isolé, extraction, relations et dépendances conservées, contraintes vérifiées, fichiers associés contrôlés, validation avant réintégration, journal complet. Ne pas supposer qu'une restauration PostgreSQL classique le permet ; analyser le modèle de données réel.

## 9. Tests automatiques de restauration
Une sauvegarde n'est pas fiable parce qu'un objet existe. Tests réguliers : empreintes ; téléchargement depuis chaque fournisseur ; restauration PostgreSQL isolée ; cohérence de la base ; documents ; restauration globale ; restauration d'un établissement ; mesure du temps réel. Jamais d'écrasement de la production ni d'exposition des données restaurées. Rapport : date, source, durée, résultat, anomalies.

## 10. Supervision et tableau de bord
Voir : dernière complète ; dernier WAL archivé ; état de chaque destination ; retard de réplication ; volumes ; erreurs ; vérifications d'intégrité ; derniers tests de restauration ; statut global. Alertes : sauvegarde échouée ; absence de WAL ; retard excessif ; perte d'une destination ; corruption ; test de restauration échoué ; expiration proche d'une rétention ou d'un certificat. Distinguer protection dégradée et double protection confirmée.

## 11. Objectifs de reprise
Indicatifs, à étudier et valider techniquement (pas des engagements commerciaux) : **RPO 15 min, RTO 4 h**. Évaluer séparément : perte de la base ; perte du stockage documentaire ; panne d'un fournisseur ; panne totale de l'infrastructure principale ; corruption logique ; compromission des accès ; restauration d'un établissement. Dire ce qui est possible avec une production sur un seul fournisseur et deux destinations ; ne pas prétendre que la double sauvegarde maintient l'ENT accessible pendant une panne complète du fournisseur de production.

## 12. Coûts et dimensionnement
Jeune SaaS : coûts maîtrisés. Scénarios : une production et deux destinations ; ajout d'une troisième copie immuable ; ajout ultérieur d'une infrastructure de reprise ; politiques de rétention ; fréquences ; volumes. Coûts : stockage, requêtes S3, transferts entrants et sortants, frais d'extraction, calcul des sauvegardes, workers de réplication, tests de restauration, stockage historique. Pas deux serveurs applicatifs permanents si un worker suffit. Architecture progressive, sans dette technique bloquante.

## 13. Architecture Rust et abstractions
Respecter l'architecture du projet. Étudier une abstraction de fournisseur de stockage (ex. `BackupStorageProvider`, `ScalewayS3Provider`, `OvhS3Provider`, `GenericS3Provider` — noms non imposés). Peu de dépendances ; composants testables ; interfaces explicites ; erreurs robustes ; idempotence ; logique de sauvegarde indépendante du fournisseur.

## 14. Documentation attendue
Architecture générale ; décisions et alternatives rejetées ; réplication ; indisponibilités ; restauration ; restauration d'un établissement ; rétention ; clés ; PRA ; supervision et alertes ; coûts ; plan de tests ; procédures opérationnelles. Respecter `CLAUDE.md`, `DECISIONS.md`, `DECISIONS_AND_FAILURES.md` ; pas de documentation parallèle contradictoire.

## 15. Déroulement
1. Auditer l'architecture Rust et les choix faits. 2. Examiner le modèle PostgreSQL et le stockage documentaire. 3. Identifier ce qui est déjà compatible. 4. Manques et risques. 5. Comparer les solutions réalistes. 6. Coûts et complexité. 7. Architecture cible progressive. 8. Décisions à prendre maintenant / plus tard. 9. Prérequis avant implémentation. 10. Feuille de route future.
Interdits : modifier la production ; créer des ressources cloud ; demander ou utiliser de vrais identifiants ; migration ou changement destructif ; étendre l'ancienne interface JavaScript.

## 16. Livrable final
Rapport clair répondant notamment à : l'architecture actuelle permet-elle la cible ? composants à prévoir dès maintenant ? ce qui peut attendre ? coût à petite, moyenne, grande échelle ? protection réelle si Scaleway tombe ? si OVHcloud est indisponible ? si les deux le sont temporairement ? comment restaurer toute la plateforme ? un établissement ? comment garantir qu'une sauvegarde est restaurable ? risques résiduels ?
Recommandation finale distinguant : le minimum indispensable à concevoir dès maintenant ; la première version de sauvegarde en production ; les protections avancées ensuite ; les fonctions coûteuses non justifiées au lancement.

## 17. Portabilité, exports autonomes et gouvernance des données par établissement (ajout de Florian, 2026-11-10)
Chaque établissement peut demander et récupérer **de manière autonome** une copie complète de ses propres données. C'est distinct des sauvegardes techniques : mécanisme de portabilité, de réversibilité et de restitution.

**A. Export autonome par établissement** — demandé depuis l'interface d'administration. Contenu selon les modules utilisés : données administratives ; élèves et responsables légaux ; données pédagogiques ; données financières et comptables ; documents et pièces jointes ; historiques et informations nécessaires à la compréhension ; paramètres propres à l'établissement ; métadonnées et relations. Format structuré et documenté, exploitable sans l'ENT (étudier CSV, JSON, fichiers documentaires originaux), avec un **manifeste** décrivant le contenu et permettant d'en vérifier l'intégrité. Ce n'est pas une archive PostgreSQL : compréhensible, exploitable et **réimportable** dans des conditions documentées. (Note du pilote : le JSON est ici un format d'échange, compatible avec D-036.)

**B. Validation à deux personnes** pour les exports complets ou particulièrement sensibles : (1) une personne habilitée initie ; (2) une seconde personne, d'habilitation compatible, approuve explicitement ; (3) chacune avec son propre compte et sa propre authentification forte ; (4) une même personne ne peut pas satisfaire les deux validations avec plusieurs sessions ou appareils. Fonctions autorisées définies par une politique de rôles et permissions configurable (direction, chef d'établissement, responsable administratif, autres fonctions expressément habilitées). Étudier délégation, absence, révocation, remplacement des approbateurs. Un administrateur technique ne peut pas contourner cette validation.

**C. Authentification et sécurité** — s'appuyer sur les passkeys/WebAuthn du projet ; réauthentification récente pour initier et pour approuver. Prévoir : expiration des demandes ; révocation avant exécution ; notification aux personnes habilitées ; journal d'audit non modifiable par les utilisateurs ordinaires ; traçabilité de l'initiateur, de l'approbateur, de la date, du périmètre et du résultat ; protection contre les demandes simultanées et les approbations répétées ; chiffrement des archives ; liens de téléchargement à durée limitée ; contrôle strict du périmètre de l'établissement ; aucun accès aux données d'autres établissements.

**D. Prévention des abus** — risques d'exfiltration de données personnelles ; limites de fréquence ; alertes sur comportement inhabituel ; rétention des archives générées et suppression automatique à expiration, sous réserve des obligations de conservation. Respect des droits des personnes, de la confidentialité, des obligations contractuelles et du RGPD.

**E. Réversibilité et restitution** — un établissement doit pouvoir quitter la plateforme sans intervention manuelle de l'éditeur. Étudier un export périodique planifié, soumis aux mêmes règles d'habilitation si nécessaire. Documenter contenu, formats, limites et modalités de réimportation.

**F. Travail demandé** — ne pas développer. Auditer rôles, permissions, passkeys, journalisation et séparation des établissements. Proposer une architecture cohérente avec les mécanismes existants. Classer les catégories d'exports : approbation simple, double approbation, procédure particulière. Présenter les décisions techniques et réglementaires à prendre avant l'implémentation.
