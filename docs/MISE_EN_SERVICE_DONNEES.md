# Mise en service : avant les premières vraies données, avant la commercialisation

Fiche de suivi des obligations hors code (hébergement Scaleway, RGPD, sécurité opérationnelle). Faits vérifiés le 2026-11-06 sur les pages Scaleway citées en bas ; à revérifier au moment d'agir.

## Plan retenu (Florian, 2026-11-06)
1. Développement avec des données fictives (maintenant).
2. **Année de transition** dans l'établissement de Florian (aujourd'hui sous Charlemagne), avec de vraies données.
3. Si l'essai est concluant : commercialisation, et alors seulement les investissements lourds (HDS, test d'intrusion par un cabinet, contrats avec d'autres établissements).

## Déjà en place dans le code
- IBAN, santé, suivi psy chiffrés champ par champ par l'application (clé hors base).
- Fichiers privés servis uniquement par URL pré-signées courtes après contrôle des droits.
- Cloisonnement par établissement (application + RLS), journal des accès.

## Déjà en place côté Scaleway (déclaré par Florian)
- Région France, base PostgreSQL managée, sauvegardes automatiques, bucket Object Storage.

## A. Avant les premières vraies données (année de transition) — peu ou pas de coût
| # | Action | Coût | État |
|---|---|---|---|
| A1 | Fermer l'accès réseau de la base (réseau privé ou liste d'IP limitée au serveur ; par défaut une base managée est ouverte à 0.0.0.0/0) | 0 | à faire |
| A2 | Chiffrement au repos de la base activé (irréversible ; l'activer après coup copie toutes les données) | 0 | à vérifier |
| A3 | Bucket privé, accès public bloqué, chiffrement SSE-ONE au minimum (SSE-KMS avec Key Manager de préférence), versionnage | 0 à faible | à vérifier |
| A4 | Clé maîtresse du chiffrement applicatif dans Secret Manager ; jamais dans un fichier ni dans Git | faible | à faire |
| A5 | Comptes nominatifs avec double authentification, moindre privilège, projets séparés production / labo, Audit Trail activé | 0 | à vérifier |
| A6 | Une restauration de sauvegarde réellement testée ; sauvegardes gardées en France (fr-par, autre zone de disponibilité) | 0 | à faire |
| A7 | **Pas de données de santé** (infirmerie, PAI/PAP/PPS, psy) tant qu'il n'y a pas d'hébergement HDS : module santé désactivé, l'infirmerie garde son outil | 0 | règle |
| A8 | Accord écrit de l'établissement (responsable du traitement) : direction, avis du DPO s'il existe, analyse d'impact (AIPD) courte, information des familles ; préciser par écrit la propriété du logiciel et le rôle de Florian | 0 | à faire |
| A9 | Revue de sécurité adverse complète de l'équipe + outils de scan gratuits juste avant l'ouverture | 0 | à faire |
| A10 | Aucune donnée réelle dans le projet labo | 0 | règle |

## B. Avant la commercialisation
| # | Action | Coût indicatif | Note |
|---|---|---|---|
| B1 | Hébergement HDS pour ouvrir le module santé : plan de support Business Scaleway (le plus élevé de 250 € HT/mois ou 10 % de la facture) + contrat HDS signé avec leur équipe commerciale | ≥ 250 € HT/mois | données HDS limitées aux centres français |
| B2 | Test d'intrusion par un cabinet indépendant (idéalement qualifié PASSI) | devis | |
| B3 | Contrat de sous-traitance (RGPD art. 28) type pour chaque établissement client ; contrat de traitement Scaleway signé | juridique | |
| B4 | DPO (externe possible), registre des traitements, AIPD complète, procédure de notification de violation sous 72 h, durées de conservation | juridique | |
| B5 | Assurance cyber | devis | |
| B6 | SecNumCloud : Scaleway n'est pas qualifié (en cours depuis janvier 2025). Si un client public l'exige, migration possible vers un hébergeur qualifié (PostgreSQL + stockage S3 standards) | — | à surveiller |
| B7 | Paiement par carte : uniquement via un prestataire de paiement agréé, jamais de carte stockée chez nous | — | plus tard |
| B8 | Prélèvements SEPA : identifiant créancier (ICS) fourni par chaque établissement | — | les établissements l'ont |

## Sources
- Scaleway HDS : https://www.scaleway.com/fr/security-and-compliance/hds/
- Scaleway SecNumCloud : https://www.scaleway.com/fr/news/scaleway-annonce-son-entree-dans-le-processus-de-qualification-secnumcloud
- Plans de support : https://www.scaleway.com/en/docs/account/support/understanding-support-plans/
- Chiffrement au repos des bases managées : https://www.scaleway.com/en/docs/managed-databases-for-postgresql-and-mysql/api-cli/setting-up-encryption-at-rest/
- Concepts des bases managées (accès réseau par défaut) : https://www.scaleway.com/en/docs/managed-databases/postgresql-and-mysql/concepts/
- Chiffrement Object Storage : https://www.scaleway.com/fr/blog/choosing-the-best-strategy-for-object-storage-encryption/
