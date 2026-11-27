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
| A11 | **Durées de conservation proposées au DPO avant l'année de transition**, avec leurs sources (`VERIF_REGLEMENTAIRE.md` R41-R46) : journaux d'accès 6 mois à 1 an (CNIL, délibération 2021-122) ; instruction de tri des archives des EPLE (BO n° 24 du 16-6-2005) ; sanctions effacées selon R511-13 IV ; mots d'excuse et justificatifs d'absence 1 an. Aucune durée appliquée sans validation | 0 | à faire |
| A12 | **Revue annuelle des habilitations** et restauration des sauvegardes testée régulièrement (CNIL, guide de la sécurité des données personnelles 2024) | 0 | à faire |
| A13 | **Procédure des demandes d'exercice des droits** : réponse sous un mois, prolongeable de deux mois avec information dans le premier mois (RGPD art. 12.3) ; registre tenu | 0 | à faire |
| A14 | **Avis d'absence aux familles opérationnel dès le premier jour** : « toute absence est immédiatement signalée aux personnes responsables » (C. éduc. R131-5, public et privé) ; une panne d'envoi est un incident à traiter le jour même | 0 | règle |
| A15 | AIPD : très probablement requise (données de mineurs, personnes vulnérables, catégories particulières) — liste CNIL des traitements soumis à AIPD ; à confirmer par le DPO (précise A8) | 0 | à faire |

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
| B9 | **HDS, précisions** (complète B1) : depuis le 16-5-2026, seuls les certificats HDS au référentiel v2 (arrêté du 26-4-2024, stockage dans l'EEE) sont valides : vérifier la version du certificat Scaleway. Héberger des données de santé sans certificat est sanctionné (C. santé publ. L1115-1). L'application de L1111-8 à l'infirmerie, aux PAI, PAP et PPS n'est tranchée par aucune source : avis du juriste et du DPO avant d'ouvrir le module | juridique | A7 reste la règle |
| B10 | **Archives** : dans le public, toute élimination exige le visa du service d'archives (C. patr. R212-14) ; le privé sous contrat peut aussi détenir des archives publiques (L211-4 2°) : avis du juriste avant la première élimination | juridique | |
| B11 | **Facturation électronique** : un organisme exonéré de TVA (enseignement, CGI 261-4-4°) doit pouvoir **recevoir** des factures électroniques depuis le 1-9-2026 ; cantine, internat et activités taxables à qualifier par l'expert-comptable | EC | |
| B12 | **Signature électronique** : faire valider par un juriste la qualification de la signature par passkey (S2, socle 7) au regard de l'art. 26 d'eIDAS modifié, avant de l'utiliser pour un contrat de scolarisation | juridique | |
| B13 | **Hors contrat** : transmission annuelle à l'autorité académique des noms, identité, âge, nationalité et titres des personnels (C. éduc. L442-2 II) : décider avec le DPO si ces pièces sont tenues dans ScolIA | — | |
| B14 | **Veille** : texte sur l'honorabilité des personnels en milieu scolaire (proposition de loi en navette au 2026-11-09) ; qualification SecNumCloud de Scaleway (en cours, non obtenue d'après les sources trouvées) | — | à surveiller |

## Sources
- Vérification réglementaire du 2026-11-09 (textes, URL, avis à demander) : `docs/VERIF_REGLEMENTAIRE.md`
- CNIL, recommandation sur la journalisation : https://www.cnil.fr/fr/la-cnil-publie-une-recommandation-relative-aux-mesures-de-journalisation
- CNIL, guide de la sécurité des données personnelles (liste de contrôle 2024) : https://www.cnil.fr/sites/cnil/files/2024-03/guide_securite_donnees_personnelles_checklist.pdf
- CNIL, liste des traitements soumis à AIPD : https://www.cnil.fr/sites/default/files/atoms/files/liste-traitements-avec-aipd-requise-v2.pdf
- Instruction de tri des archives des EPLE (BO n° 24 du 16-6-2005) : https://www.education.gouv.fr/bo/BoAnnexes/2005/24/tableaux_encart24.pdf
- DGFiP, fiche « association » de la facturation électronique : https://www.impots.gouv.fr/sites/default/files/media/1_metier/2_professionnel/EV/2_gestion/290_facturation_electronique/fiches_reforme/fiche-association.pdf
- Scaleway HDS : https://www.scaleway.com/fr/security-and-compliance/hds/
- Scaleway SecNumCloud : https://www.scaleway.com/fr/news/scaleway-annonce-son-entree-dans-le-processus-de-qualification-secnumcloud
- Plans de support : https://www.scaleway.com/en/docs/account/support/understanding-support-plans/
- Chiffrement au repos des bases managées : https://www.scaleway.com/en/docs/managed-databases-for-postgresql-and-mysql/api-cli/setting-up-encryption-at-rest/
- Concepts des bases managées (accès réseau par défaut) : https://www.scaleway.com/en/docs/managed-databases/postgresql-and-mysql/concepts/
- Chiffrement Object Storage : https://www.scaleway.com/fr/blog/choosing-the-best-strategy-for-object-storage-encryption/
