# Socle 25 — Paiements, prélèvements et recouvrement

**Date :** 2026-11-10 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée après le code (lot 7 et lot S2-9), comme `24_FINANCES.md`. Aucun code écrit ni modifié ; aucun accès à une base, à un secret, à un compte bancaire ni à l'application Next.js.
**Branche :** `ent-socle-finances-conception` (partie d'`origin/ENT` `5366c196`).
**Lire d'abord :** `docs/socles/24_FINANCES.md` — il porte le **cadre commun** : thèse (§1), existant et défauts **FD-01 à FD-26** (§2), acteurs (§3), année financière (§4), réglementaire général (§5), frontières (§6), payeurs et familles séparées (§9), cas limites CF-01 à CF-40 (§15), **lot F0** et tableau des lots (§25), migrations, questions Q-F. Ce document-ci ne reprend que ce qui est propre à l'argent qui entre et qui sort.
**Conventions :** celles du 24 ([F], [V], [NV], [H], [R]) ; actions **PAY-100** à **PAY-211** ; règles **R-P1** à **R-P24** ; cas limites **CF-41** à **CF-72** ; questions **Q-P1**… ; réglages **S-169**…
**Plage de migrations :** **5500–5599** si Q-B2, **repli 540–559** (vérifié libre sur toutes les branches distantes le 2026-11-10).

Sommaire : §0 En bref · §1 Thèse · §2 Existant propre au 25 · §3 Acteurs (compléments) · §4 Réglementaire · §5 Concepts · §6 Moyens de paiement · §7 SEPA de bout en bout · §8 Règlements, imputation, lettrage · §9 Banque et rapprochement · §10 Relances et recouvrement · §11 Remboursements et crédits · §12 Attestations · §13 Cas limites · §14 IA · §15 Actions PAY · §16 Règles R-P · §17 Droits et sécurité · §18 Événements · §19 Notifications · §20 Écrans · §21 Modèle de données · §22 Plan de migration · §23 Lots et migrations · §24 Tests · §25 Réglages · §26 Questions · §27 Faits et hypothèses.

---

## 0. En bref

1. **Ce qui tient** [F] : IBAN des mandats **chiffrés** (AES-GCM) et **masqués** partout ; préparation des prélèvements **par échéance** refusée à moins de 14 jours de la date (pré-notification) ; pré-notification enregistrée par (foyer, date) ; rejets avec motif normalisé, re-présentation, **retour au paiement manuel** après N rejets ; règlement manuel imputé sur les plus anciennes échéances avec trop-perçu au crédit ; lettrage ; relances **proposées par le système, envoyées après confirmation humaine**, jamais avant l'échéance ni pendant un prélèvement en cours ; export comptable équilibré.
2. **Ce qui fuit ou se trompe** : RS3-01 et RS3-02 non corrigées sur `ENT` (24 FD-01, FD-02) ; export SEPA historique en `GET`, sans pré-notification, IBAN du créancier dans l'URL (FD-03) ; mandats révoqués exportés (FD-10) ; mandat « signé » à l'instant de sa création **sans preuve de consentement**, RUM choisie par le client (RS3-08) ; **pré-notification interne seulement** : un payeur sans compte est prélevé sans être prévenu (FD-14) ; pain.008 en **version .001.02** alors que l'EPC applique la version 2019 (**.001.08**) et **interdit les adresses non structurées au 15/11/2026** [V] ; aucun retour ni remboursement après exécution ; rapprochement simulé ; aucune remise de chèques ; aucun reçu d'espèces conforme au plafond ; frais de relance facturables (interdits, FD-13) ; aucun remboursement des trop-perçus.
3. **La thèse (§1).** Un paiement est **un fait bancaire prouvé**, rattaché à **un payeur** et **imputé** sur ses pièces ; un prélèvement n'existe qu'avec **un mandat prouvé**, une **pré-notification qui a atteint la personne**, un **lot validé** ; le relevé bancaire **fait foi** et le rapprochement est automatique ; une relance est **un dialogue gradué** qui commence par comprendre ; l'argent dû **à** la famille (trop-perçu, excédent de bourse, avoir) part aussi vite que l'argent dû **par** elle.
4. **SEPA [V]** : rulebook EPC SDD Core 2025 v1.1 — mandat (mentions obligatoires, RUM unique, conservation 13 mois au moins après la fin), ICS (Banque de France), **pré-notification 14 jours** sauf accord, présentation au plus tard J-1 ouvré, **FRST facultatif**, **caducité à 36 mois** sans collecte (obligation du créancier), remboursement **inconditionnel 8 semaines** (CMF L. 133-25-1), **13 mois** si non autorisé, retours sous 5 jours ouvrés ; format **pain.008.001.08** ; adresses structurées ou hybrides obligatoires au **15/11/2026**.
5. **Recouvrement [V]** : prescription **2 ans** très probable (C. consom. L. 218-2) ; **aucun frais de recouvrement** à la charge de la famille sans titre exécutoire (CPCE L. 111-8) ; injonction de payer réformée (décret 2026-96) ; sociétés de recouvrement encadrées (R. 124-1 s.) ; référentiel CNIL des impayés (suppression 48 h après régularisation) ; **espèces plafonnées à 1 000 €** par paiement (CMF D. 112-3) ; carte : **jamais le cryptogramme**, page hébergée du prestataire (délibération CNIL 2018-303).
6. **Le plan.** 112 actions (PAY-100 à PAY-211), 32 cas limites (CF-41 à CF-72), 24 règles, 12 lots (**P1 à P12**) après le lot commun **F0** ; parcours minimal : P1–P4, P6 ; **soumis à Florian** : contrat bancaire (ICS, mode de remise des fichiers, coûts, relevés électroniques), prestataire de paiement en ligne (contrat, frais), éventuelle société de recouvrement, avis juridique sur le refus de réinscription pour impayé.

---

## 1. Thèse : l'argent qui entre est prouvé, rattaché, imputé ; l'argent qui sort l'est aussi

1. **Un paiement a un payeur, un moyen, une preuve.** Le règlement (chèque, espèces, virement, carte, prélèvement) est enregistré au **compte débiteur** qui a payé (24 §9), avec sa preuve (numéro de chèque et banque, référence de virement, transaction du prestataire, identifiant de bout en bout SEPA, ligne de relevé) ; jamais « au foyer » sans payeur.
2. **Imputer est une décision traçable.** Par défaut, sur les échéances les plus anciennes du **même** compte [F] ; la famille peut **désigner** ce qu'elle paie (« le voyage de Léa ») ; jamais sur le compte d'un autre payeur ; le trop-perçu est un **crédit** du payeur, pas un profit.
3. **Pas de prélèvement sans les trois preuves** : mandat signé et prouvé, pré-notification **arrivée** (canal et date), lot validé par une personne habilitée (N3).
4. **La banque fait foi.** Le relevé (camt.053 / 054, ou fichier de la banque) confirme l'exécution, les rejets, les retours, les remboursements ; ScolIA rapproche **automatiquement** ce qui est certain et met le reste dans une file ; rien n'est « exécuté » sur la seule date prévue sans relevé, sauf geste manuel motivé [F existant].
5. **La relance est un dialogue gradué.** Rappel courtois, relance, mise en demeure, plan d'apurement, médiation, procédure : chaque palier est **proposé**, **confirmé** par une personne, **adressé au seul payeur**, **suspendu** par un plan, un deuil, une contestation, une aide en cours d'instruction ; jamais de frais de recouvrement, jamais l'élève comme levier (R-F20).
6. **Ce qui est dû à la famille part vite.** Trop-perçu, excédent de bourse, avoir de départ, excédent de voyage : proposés au remboursement **dès qu'ils naissent**, par virement vers un IBAN vérifié du payeur.
7. **Les coordonnées bancaires sont un secret de la personne.** Chiffrées, masquées, jamais dans une URL, un journal, un événement, un export autre que le fichier bancaire, un contexte d'IA ; l'IBAN du **créancier** (l'établissement) est protégé comme une clé : son changement est le geste le plus dangereux du domaine (fraude au changement de coordonnées).
8. **Séparation des tâches.** Celui qui saisit un règlement ne valide pas seul la remise en banque d'un lot important ; celui qui change l'IBAN du créancier ne le valide pas seul ; chaque geste sensible est journalisé et visible de la direction.

---

## 2. Existant propre au 25 [F]

Le tableau général est au 24 §2.1 ; les défauts au 24 §2.2 (FD-01 à FD-26). Compléments :

| Élément | Où | Ce que ça fait | Limites |
|---|---|---|---|
| Mandats | `030_sepa.sql`, `facturation_sepa.rs:141`, `routes.rs:10958` | RUM (≤ 35), IBAN contrôlé en **format seulement** (pas de clé modulo 97), BIC 8/11, un mandat actif par foyer, historique d'IBAN (`078`) | `signe_at = now()` sans preuve (RS3-08) ; RUM du client ; pas d'avenant (`AmdmntInd` toujours faux) ; pas de caducité ; créé par le parent **même non payeur** sur `ENT` (FD-01) |
| Prélèvements (historique) | `service.rs:18904-19321` | par facture ; export CSV ou pain.008.001.02 (FRST si aucun prélèvement antérieur, sinon RCUR ; pas d'OOFF) ; exécution = encaissement + quittance | FD-02, FD-03, FD-06, FD-10 ; lignes passées « exporté » **avant** la génération du fichier |
| Prélèvements (S2-9) | `finances_sepa.rs:264-859` | préparation par échéance à ≥ 14 jours ; pré-notification par (foyer, date) ; export des seuls pré-notifiés ; MsgId `SDD-AAAAMMJJ-nnnnnn` ; exécution constatée manuellement ; rejet depuis `exporte` seulement, codes ISO en dur [NV], frais facturés et émis aussitôt, retour au manuel | pré-notification **interne seulement** (FD-14) ; un FRST rejeté repart en FRST ; aucun retour ni remboursement après exécution ; aucun lien avec le rapprochement |
| Règlements | `203`, `finances_sepa.rs:907-1020` | règlement imputé sur les plus anciennes, trop-perçu, code de lettrage `L{n}` | pas de remise de chèques, pas de caisse, pas de reçu d'espèces, pas de remboursement du trop-perçu |
| Rapprochement | `ent-bank/src/lib.rs`, `059` | CSV `e2e;centimes;settled|rejected;motif` ; mode live = bouchon | simulé ; non idempotent (rejeu → erreurs) ; montants « 80,00 » refusés |
| Relances | `204`, `finances_sepa.rs:1027-1247` ; historique `032` | paliers proposés puis confirmés ; rappels historiques J-3 et retard par POST manuel | aucun planificateur ; frais de palier (FD-13) ; double relance possible avec l'historique (FD-04) ; relance d'un parent non payeur possible par le fil historique |
| Paiement en ligne | `paiement_en_ligne.rs` | trait et implémentation désactivée | aucun prestataire |
| Espace famille | `finances.rs:3385-3481` | échéances et état, prélèvement associé, mode de paiement, mandats masqués | pas de quittance, pas de paiement, pas d'attestation sur le chemin neuf |

---

## 3. Acteurs (compléments au 24 §3)

| Acteur | Ce qui lui fait perdre du temps [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Comptable** | Pointer le relevé ligne à ligne ; saisir 300 chèques en septembre ; les avis de rejet de la banque à recopier ; refaire un fichier refusé par la banque ; rembourser par chèque | Un lot envoyé deux fois ; un lot avec le mauvais IBAN créancier ; un mandat sans preuve en litige ; un chèque perdu entre l'accueil et la banque | **Rapprochement automatique** ; saisie de chèques en série (lecteur de chèques ou saisie rapide [H]) avec **bordereau de remise** ; rejets lus depuis le relevé ; remboursements groupés par virement |
| **Direction** | Arbitrer des impayés sans historique ; signer des mises en demeure faites à la main | Une procédure contre une famille fragile ; une fraude interne | Un tableau des impayés **par ancienneté et par situation** (oubli, plan, contestation, social), des courriers prêts, un journal |
| **Accueil** | Enveloppes d'espèces sans reçu | Une accusation de vol | Reçu numéroté immédiat, coffre « à remettre », remise à la compta tracée |
| **Famille** | Remplir un mandat papier ; ne pas savoir quand elle sera prélevée ; être prélevée après un changement de banque | Un prélèvement surprise ; un rejet facturé par sa banque et par l'école ; une relance humiliante | Mandat signé en ligne en une minute ; « prochain prélèvement : 312,00 € le 05/12 » ; payer par carte en deux gestes ; plan d'apurement en ligne |
| **Parent séparé payeur** | Prélevé de la totalité | Que l'autre voie son IBAN ou ses retards | Son mandat, ses prélèvements, ses relances, rien de l'autre |
| **Famille en difficulté** | Relances sèches | La honte, l'exclusion de l'enfant | Proposer elle-même un échéancier ; être orientée discrètement vers l'assistante sociale |
| **Banque de l'établissement** | — | Fichiers non conformes, rejets massifs | Fichiers pain.008.001.08 conformes, adresses structurées, lots cohérents |
| **Prestataire de paiement** | — | — | Intégration par page hébergée, notifications signées, remboursements par API |

---

## 4. Ce qui est réglementaire (vérifié le 2026-11-10)

### 4.1 Prélèvement SEPA Core [V]
- **Texte** : EPC016-06 *2025 SDD Core Rulebook* v1.1 (05/10/2025), PDF officiel lu en entier (`europeanpaymentscouncil.eu/sites/default/files/kb/file/2025-10/EPC016-06 2025 SDD Core Rulebook version 1.1.pdf`) ; une v1.2 du 30/09/2026 serait listée par l'EPC [NV].
- **Mandat** (§4.7.2, DS-01) : intitulé contenant « SEPA » ; RUM ; nom du débiteur, code postal, ville, pays ; IBAN ; nom et **ICS** du créancier ; adresse complète du créancier ; type (récurrent / ponctuel) ; lieu et date de signature, signature ; texte légal d'autorisation et de droit au remboursement ; adresse du débiteur et BIC seulement si un PSP est hors EEE. RUM ≤ 35 caractères, unique pour le créancier [V partiel] ; **communiquée au débiteur avant le premier prélèvement** (Banque de France, FAQ « le prélèvement SEPA ») [V].
- **Conservation du mandat** : tant qu'il existe, puis selon le droit national et **au moins 13 mois** après annulation (délai des opérations non autorisées) [V].
- **ICS** : demandé par la banque à la Banque de France ; 13 caractères en France ; suppose une activité et un compte en France ; **inchangé en cas de changement de banque** (`banque-france.fr`, référentiel des identifiants créanciers SEPA) [V].
- **Pré-notification** : au plus tard **14 jours calendaires** avant l'échéance, sauf autre délai convenu ; au minimum date et montant [V]. Une pré-notification **annuelle unique** (échéancier complet) est admise si elle donne dates et montants [V par le principe ; pratique courante H].
- **Présentation** : reçue par la banque du débiteur au plus tard **J-1 ouvré interbancaire**, au plus tôt 14 jours avant ; **FRST n'est plus obligatoire** (un premier prélèvement peut partir en RCUR) depuis la v9.0 (2016) [V].
- **Caducité** : un mandat **sans collecte pendant 36 mois** doit être annulé par le créancier (le délai court depuis la dernière collecte présentée, même rejetée) [V].
- **Remboursements** : prélèvement autorisé : **8 semaines**, inconditionnel (CMF **L. 133-25** et **L. 133-25-1**, Légifrance `LEGIARTI000035430556`, `LEGIARTI000035430552`) [V] ; non autorisé (sans mandat ou mandat invalide) : **13 mois** (CMF L. 133-24) [V partiel].
- **R-transactions** (§4.3–4.4) : *reject* (avant règlement : technique, compte clos, décès, refus), *refusal* (demande du débiteur avant règlement), *return* (≤ 5 jours ouvrés après règlement), *refund* (8 semaines / 13 mois), *reversal* (à l'initiative du créancier qui a prélevé à tort, ≤ 5 jours ouvrés après l'échéance), *revocation* (annulation avant règlement) ; toujours sur le **montant exact** d'origine. Le débiteur peut **révoquer le mandat** auprès du créancier à tout moment [V].
- **Format** : ISO 20022 version 2019 depuis le 17/03/2024 ; guide EPC130-08 : **pain.008.001.08** [V] ; la version .02 peut encore être convertie par certaines banques [V partiel, à confirmer avec la banque] ; **adresses non structurées interdites à partir du 15/11/2026** (formats hybride ou structuré seulement) [V].
- **Vérification du bénéficiaire** (règlement 260/2012 art. 5 quater, inséré par le règlement (UE) 2024/886) : obligatoire depuis le 09/10/2025 pour les **virements** (pas les prélèvements) [V] : concerne les **remboursements** sortants de l'établissement et le libellé exact du titulaire du compte de l'OGEC que les familles saisissent pour un virement.

### 4.2 Espèces, chèques, cartes [V]
- **Espèces** : CMF **D. 112-3** : **1 000 €** par paiement d'une dette entre un professionnel et un particulier domicilié en France (15 000 € pour un non-résident qui le justifie, 10 000 € si le créancier n'est pas assujetti aux obligations anti-blanchiment) ; amende jusqu'à 5 % [V partiel]. **Reçu** : aucun texte propre trouvé ; reçu numéroté = bonne pratique et preuve [R].
- **Chèques** : aucune règle propre relevée ; en public, remise à l'encaissement **le lendemain** par le régisseur (décret 2019-798) [V partiel].
- **Cartes** : délibération CNIL **2018-303** : cryptogramme **jamais** conservé ; données de carte gardées **13 mois** (15 si débit différé) en archive intermédiaire pour les contestations ; conservation pour des paiements futurs seulement avec **consentement explicite** [V]. → ScolIA ne touche **aucune donnée de carte** : page de paiement **hébergée** par le prestataire, ScolIA ne reçoit qu'une référence de transaction (périmètre PCI DSS minimal [H]).

### 4.3 Recouvrement [V]
- **Prescription** : C. consom. **L. 218-2** (2 ans) ; L. 218-1 (durée non modifiable) [V partiel] ; parents traités en consommateurs (24 §5.1) ; point de départ : connaissance des faits (Cass. 3e civ., 01/03/2023, n° 21-23.176 [V partiel]) ; **interruption** par une reconnaissance de dette (plan d'apurement signé), une citation en justice, une mesure d'exécution [NV, C. civ. 2240 s.] — une simple relance **n'interrompt pas** la prescription [NV, à confirmer].
- **Frais** : CPCE **L. 111-8** : sans titre exécutoire, les frais de recouvrement restent à la charge du créancier, clause contraire réputée non écrite (sauf débiteur de mauvaise foi, décision du juge de l'exécution) [V].
- **Injonction de payer** : CPC 1405 (créance contractuelle d'un montant déterminé) ; décret **2026-96** du 16/02/2026 (signification sous 3 mois pour les ordonnances rendues depuis le 01/09/2026 [V partiel]).
- **Sociétés de recouvrement amiable** : CPCE R. 124-1 à R. 124-7 (convention écrite, assurance, compte dédié, lettre reproduisant L. 111-8) [V partiel].
- **Pénalités contractuelles** : possibles si le contrat les prévoit, révisables par le juge (clause pénale) ; **indemnité forfaitaire de 40 € interdite envers un particulier** [V partiel].
- **Refus de réinscription pour impayé** : aucune source officielle [NV] ; liberté contractuelle de ne pas renouveler un contrat probable, **sanction disciplinaire de l'élève exclue** [H] ; **avis juridique** (Q-P9).
- **CNIL** : référentiel « gestion des impayés » : suppression de l'inscription **48 h** après régularisation ; sinon 5 ans au plus [V].

### 4.4 Conservation des données bancaires [V partiel]
IBAN : durée de vie du mandat ; mandat et preuves : **13 mois au moins** après la dernière opération (rulebook, L. 133-24) ; pièces comptables 6 à 10 ans (24 §5.2) ; aucune règle CNIL propre à l'IBAN trouvée [NV] → durées **soumises au DPO** (Q-P11).

---

## 5. Concepts et vocabulaire [R]

| Terme | Définition | Table cible |
|---|---|---|
| **Règlement** | Somme reçue d'un payeur par un moyen, à une date, avec sa preuve ; peut couvrir plusieurs pièces | `ent_fin_reglement` (existant, enrichi) |
| **Imputation** | Affectation d'un règlement (ou d'un avoir, d'une aide) à une échéance ; produit le lettrage | `ent_fac_encaissement` + `ent_fin_lettrage` [F] |
| **Crédit** | Somme au profit du payeur non imputée (trop-perçu, avoir non imputé, excédent d'aide) | calculé + `ent_fin_credit_mouvement` |
| **Remise** | Lot de chèques (bordereau) ou d'espèces déposé en banque ; lot de prélèvements (fichier) ; lot de virements sortants | `ent_fin_remise` |
| **Caisse** | Point d'encaissement (accueil, comptabilité, voyage) avec son journal et son solde | `ent_fin_caisse`, `ent_fin_caisse_mouvement` |
| **Mandat** | Autorisation SEPA signée par un débiteur (payeur) pour un créancier ; RUM, version, preuve, état | `ent_fin_mandat` (remplace `ent_fac_sepa_mandat` par évolution) |
| **Pré-notification** | Information du débiteur (dates, montants) **remise** par un canal prouvé, au moins N jours avant | `ent_fin_prenotification` [F] enrichie |
| **R-transaction** | Rejet, retour, refus, remboursement, reversal, révocation d'un prélèvement | `ent_fin_r_transaction` |
| **Relevé** | Fichier bancaire (camt.053 journalier, camt.054 avis, ou format de la banque) | `ent_fin_releve`, `ent_fin_releve_ligne` |
| **Rapprochement** | Lien certain entre une ligne de relevé et un objet de ScolIA (règlement, lot, R-transaction, remboursement) | `ent_fin_rapprochement` |
| **Dossier de recouvrement** | Ensemble des échéances en retard d'un payeur, son palier, sa situation (oubli, plan, contestation, social, procédure), ses suspensions | `ent_fin_dossier_recouvrement` |
| **Plan d'apurement** | Échéancier accepté par écrit (reconnaissance de dette), suivi, révoqué si non respecté | `ent_fin_plan_apurement` |
| **Remboursement** | Virement sortant vers l'IBAN vérifié du payeur (ou chèque) | `ent_fin_remboursement` |
| **Porte-monnaie** | Crédit prépayé affecté à une catégorie (cantine), débité par consommation | `ent_fin_porte_monnaie` |

---

## 6. Moyens de paiement [R]

| Moyen | Comment | Preuve | Règles |
|---|---|---|---|
| **Prélèvement SEPA** | mandat électronique ou papier scanné, lots mensuels (§7) | identifiant de bout en bout, relevé | par défaut pour la facture annuelle ; jamais sans mandat prouvé et pré-notification remise |
| **Virement** | référence de paiement unique imprimée sur chaque facture (`F2026-000123` ou référence du compte) ; IBAN de l'OGEC affiché **avec son titulaire exact** (VoP) | ligne de relevé | rapprochement automatique par référence ; virement sans référence → file « à identifier » |
| **Carte en ligne** | page hébergée du prestataire (Q-P1), depuis le compte famille ou un lien de paiement | identifiant de transaction, notification signée | aucun stockage de carte ; paiement d'une échéance, d'un solde, d'un voyage, d'un rechargement de porte-monnaie ; frais de prestataire **non refacturés** par défaut (S-169) |
| **Chèque** | saisi au guichet ou en série ; photo facultative ; **bordereau de remise** en banque | numéro, banque, tireur (nom imprimé, peut différer du payeur : un grand-parent), remise, relevé | chèque d'un tiers accepté et imputé au compte désigné ; chèque impayé = R-transaction « chèque impayé » |
| **Espèces** | caisse, **reçu numéroté** immédiat | reçu, journal de caisse, remise en banque | **refus au-delà de 1 000 €** par paiement (D. 112-3, non-résident : réglage) ; double comptage à la clôture de caisse (S-170) |
| **Chèques-vacances** (ANCV) | pour les voyages si l'établissement est conventionné [NV] | numéros, bordereau | réglage désactivé par défaut |
| **Aide versée par un tiers** (bourse, fonds social, CE, mairie) | virement ou chèque du tiers | relevé, décision | imputée comme aide affectée (24 §10.5), pas comme un règlement de la famille |
| **Compensation** (avoir, crédit) | imputation | lettrage | jamais entre deux payeurs |

---

## 7. SEPA de bout en bout

### 7.1 Le créancier [R]
- **Paramètres du créancier** (nom, ICS, IBAN, BIC, adresse **structurée**) : saisis **une fois**, chiffrés, **jamais** passés par une URL ni affichés en clair ; changement = **N3 + second validateur** (direction) + alerte à tous les comptables et à la direction + **délai de 24 h** avant le premier lot sur le nouveau compte (R-P2) ; journalisé avec l'ancienne et la nouvelle valeur **masquées**.
- **Plusieurs créanciers** possibles (établissement et OGEC, ou deux OGEC d'un ensemble) : un par série de pièces (D-027 F).

### 7.2 Le mandat [R]
- **Création électronique** par le **payeur** seulement (R-P4, RS3-01) : depuis le compte famille, à l'inscription (D-014 : reprise de l'IBAN masqué proposée, **nouveau mandat**) ou sur invitation de la comptabilité ; formulaire conforme au §4.1 ; **RUM générée par le serveur** (`{ICS court}-{série}-{n}`, ≤ 35) ; contrôle **modulo 97** de l'IBAN ; titulaire du compte saisi (peut différer du payeur : compte joint, grand-parent — alors **le titulaire signe**) ; **signature** : passkey du payeur (preuve forte) ou code envoyé sur un canal vérifié (socle 7, preuve de signature : version du texte, empreinte, horodatage, canal masqué) ; document du mandat rendu et stocké (socle 7) ; RUM communiquée au débiteur dans le document.
- **Mandat papier** : saisi par la comptabilité, **scan obligatoire** du mandat signé (socle 7), statut `actif` seulement après contrôle par une seconde personne si l'établissement l'active (S-171).
- **États** : `en_attente_signature` → `actif` → `suspendu` (litige, décès) → `revoque` (par le débiteur ou l'établissement) / `caduc` (36 mois sans collecte, **automatique** avec avis préalable) / `remplace` (avenant).
- **Avenant** (changement d'IBAN, de titulaire, de créancier) : nouvelle **version** du mandat, même RUM si le rulebook l'admet, indicateur d'amendement dans le fichier suivant (`AmdmntInd`, informations d'origine) [V principe, détail .001.08 NV] ; l'ancien IBAN reste chiffré dans l'historique.
- **Changement de payeur** (24 §9.2) : le mandat du payeur sortant **ne sert plus** pour cet élève à la date d'effet (« le mandat de M. X ne servira plus pour Léo à partir du 01/02 ») ; jamais de transfert du mandat d'une personne à une autre.
- **Plusieurs comptes débiteurs, un mandat** : un mandat par **compte débiteur** (payeur) et par créancier ; un payeur d'une seule catégorie a son propre mandat.

### 7.3 Pré-notification [R]
- **Par défaut, une pré-notification annuelle** : l'échéancier de la facture (dates et montants) remis **avec** la facture, au moins 14 jours avant la première échéance ; **une pré-notification spécifique** si un montant ou une date change (échéancier dérogatoire, re-présentation après rejet, régularisation).
- **Remise prouvée** : in-app **et** e-mail (enveloppe + document) ; payeur sans compte : e-mail avec lien nominatif ou **courrier** ; **aucun prélèvement** préparé si aucune remise n'est prouvée (corrige FD-14) ; la date de remise compte, pas la date de création.
- **Délai** : 14 jours par défaut [F] ; un délai plus court seulement s'il est **prévu au contrat de scolarisation** (réglage S-172, borne basse 2 jours).

### 7.4 Le lot de prélèvements [R]
1. **Préparation** (SYS, chaque mois à J-10 de la date par défaut) : une ligne par échéance exigible à la date, non couverte, avec mandat actif et pré-notification remise ; écartés et expliqués : paiement manuel, mandat absent / caduc / suspendu, plan d'apurement, dossier suspendu (décès, contestation), payeur sans pré-notification remise.
2. **Contrôle** (CPT) : total, nombre, écarts avec le mois précédent, premières présentations, re-présentations, montants hors norme.
3. **Validation N3** par une personne habilitée ; **seconde validation** au-delà d'un seuil (S-173, défaut 50 000 €) ; **verrou** : un lot validé ne se modifie plus, il s'annule (révocation) et se refait.
4. **Fichier** : pain.008.001.08 CORE, adresses structurées, séquence RCUR par défaut (FRST facultatif [V] ; réglage S-174 si la banque l'exige encore), un bloc par date ; contrôle **XSD** obligatoire avant remise ; empreinte ; nom de fichier normalisé.
5. **Remise** à la banque : téléchargement par la comptabilité et dépôt sur le portail de la banque (par défaut), ou **EBICS** (Q-P2, contrat bancaire) ; statut `remis` posé par le geste de remise, pas par la génération (corrige « exporté avant génération »).
6. **Exécution** : constatée par le **relevé** (camt.054 ou ligne de relevé) ; à défaut de relevé électronique, constatation manuelle en lot à J+5 ouvrés (existant [F]) avec mention « constaté sans relevé ».
7. **R-transactions** (§7.5).

### 7.5 Rejets, retours, remboursements [R]
| Type | Quand [V] | Effet dans ScolIA |
|---|---|---|
| Reject / refusal | avant règlement | prélèvement `rejete`, échéance rouverte, motif normalisé, avis au payeur, suite (re-présentation au lot suivant avec pré-notification spécifique, ou paiement manuel) [F] |
| Return | ≤ 5 jours ouvrés après règlement | **contre-passation** de l'encaissement (nouvelle ligne négative datée, jamais suppression), échéance rouverte, quittance **annulée par une pièce d'annulation** (jamais modifiée) |
| Refund (8 semaines) | demande du débiteur à sa banque | idem return ; dossier marqué « remboursement demandé par le titulaire » ; **aucune** relance automatique pendant 10 jours (dialogue d'abord) |
| Refund non autorisé (13 mois) | contestation d'autorisation | idem + **mandat suspendu**, enquête (preuve de signature, socle 7 D65) |
| Reversal | erreur de l'établissement (montant faux, prélevé deux fois) | à l'initiative de la comptabilité, ≤ 5 jours ouvrés, N3 |
| Revocation | annulation d'un lot ou d'une ligne avant règlement | auprès de la banque ; statut `revoque` |
- **Frais de rejet** : 0 par défaut (Q-F21) ; si activés, **au coût bancaire réel**, prévus au contrat, une fois par rejet, jamais sur un rejet causé par l'établissement.
- **Seuil de retour au paiement manuel** : 2 rejets sur 365 jours [F] ou motif définitif (compte clos, décès, mandat inexistant) [F] ; un retour au manuel est **expliqué** au payeur avec les autres moyens.

---

## 8. Règlements, imputation, lettrage [R]

- **Saisie au guichet** : chercher le payeur (nom, numéro de facture, référence), voir son solde et ses échéances **sans voir les autres payeurs de l'enfant** ; saisir moyen, montant, preuve ; **imputer** (par défaut les plus anciennes [F], ou désignation de la famille) ; **reçu** immédiat (quittance si la pièce est soldée, reçu sinon) ; tout dans **une transaction** sous verrou du compte (FD-07).
- **Saisie en série** (septembre) : liste de chèques avec lecture rapide (saisie au clavier, photo, ou lecteur de chèques [H]), contrôle du total contre le bordereau, imputation automatique, exceptions dans une file.
- **Enveloppe reçue à l'accueil** : enregistrée « à remettre » (montant déclaré, moyen, photo du chèque, payeur présumé), reçu d'accueil ; la comptabilité **réceptionne** (écart → alerte) puis saisit le règlement ; la chaîne est tracée.
- **Caisse** : ouverture, mouvements, clôture avec comptage (billets et pièces), écart motivé, remise en banque (bordereau d'espèces) ; une caisse par lieu ; un voyage peut avoir sa caisse temporaire.
- **Remise de chèques** : bordereau (liste, total, nombre) généré, imprimé ou envoyé, puis **rapproché** du relevé (crédit global) ; chèque refusé par la banque → R-transaction « chèque impayé ».
- **Virement reçu** : rapproché par référence ; sinon file « à identifier » (montant, libellé, nom du donneur d'ordre) avec suggestions (montant exact d'une échéance, nom proche d'un payeur) ; **aucune** imputation automatique sans certitude.
- **Annulation d'un règlement** (erreur de saisie) : contre-passation motivée, N3, jamais suppression ; impossible après la remise en banque (alors R-transaction).
- **Lettrage** : code posé quand une pièce est soldée [F] ; délettrage motivé (contre-passation) ; **état des pièces non lettrées** par ancienneté.

---

## 9. Banque et rapprochement [R]

- **Import des relevés** : camt.053 (relevé de fin de journée) et camt.054 (avis de crédit / débit) [NV sur la disponibilité chez la banque de l'OGEC], ou CSV de la banque avec un profil d'import ; dépôt manuel par la comptabilité ou récupération EBICS (Q-P2) ; un relevé importé deux fois est détecté (empreinte, numéro de relevé) et **ignoré** (corrige la non-idempotence d'`ent-bank`).
- **Rapprochement automatique** : identifiant de bout en bout (prélèvements, R-transactions) ; référence de paiement (virements) ; montant et date du bordereau (remises de chèques et d'espèces) ; identifiant du prestataire (cartes, versements groupés du prestataire nets de frais) ; remboursements sortants.
- **File d'exceptions** : lignes non rapprochées (crédits inconnus, débits de frais bancaires, versements de tiers), avec propositions ; une ligne de frais bancaires va en dépense (socle 48), pas sur une famille.
- **Mode labo** : le simulateur actuel (`ent-bank`) reste pour le labo, **jamais** en production.

---

## 10. Relances et recouvrement

### 10.1 Le parcours gradué [R]
| Palier | Quand (défaut) | Forme | Ton | Suspendu si |
|---|---|---|---|---|
| 0 — Information | J-3 avant une échéance **non prélevée** (paiement manuel) | in-app | « échéance du 05/12 : 312,00 € » | — |
| 1 — Rappel | J+7 | in-app, e-mail enveloppe | courtois, moyens de payer, lien de paiement, « si vous rencontrez une difficulté, contactez-nous » | plan, contestation, deuil, aide en instruction, prélèvement en cours |
| 2 — Relance | J+30 | e-mail + document | rappel du montant, proposition de plan d'apurement en ligne, contact de l'assistante sociale (si l'établissement en a une) | idem |
| 3 — Mise en demeure | J+60, **décision de la direction** | courrier (recommandé selon réglage S-175) | mise en demeure de payer sous 15 jours ; mention des suites possibles **sans menace sur l'élève** | idem |
| 4 — Médiation / recouvrement | après la mise en demeure, **décision de la direction** | rendez-vous, médiation (FIN-11), société de recouvrement (Q-P8), injonction de payer | — | — |
- **Toujours proposé, jamais envoyé seul** [F] : chaque palier est une proposition dans la file ; l'envoi est confirmé par une personne (N3 pour les paliers 3 et 4).
- **Adressé au seul payeur** (socle 4 `destinataires`, nature « relance ») ; **jamais** à l'autre parent, à l'élève, à un contact.
- **Plage calme** du socle 8 ; jamais un vendredi soir ni la veille d'un jour férié pour les paliers 2 et 3 (S-176).
- **Aucun frais** de relance (FD-13, CPCE L. 111-8) ; pénalités contractuelles seulement si le contrat les prévoit (Q-F21).
- **Prescription** : alerte à 18 mois d'ancienneté d'une échéance non réglée sans acte interruptif (plan signé, reconnaissance) : « créance proche de la prescription » (R-P14).

### 10.2 Situations [R]
Chaque dossier porte une **situation** choisie par la comptabilité (jamais déduite par l'IA) : `oubli`, `contestation` (suspend), `plan` (suspend tant que respecté), `difficulte` (oriente vers l'assistante sociale et le fonds social, suspend selon décision), `deuil` (suspend, contact humain), `conflit_parental` (rappel : chaque payeur ne doit que sa part), `procedure`. La situation n'est visible que du personnel financier et de la direction ; **le motif social n'y figure pas** (renvoi au socle 26 / 36).

### 10.3 Plan d'apurement [R]
Proposé par la comptabilité **ou demandé par la famille en ligne** (montant mensuel, date) ; accepté et **signé** par le payeur (socle 7 : vaut reconnaissance de dette et interrompt la prescription [NV]) ; échéancier dérogatoire (24 FAC-163) ; prélèvements ou paiements suivis ; deux échéances manquées → proposition de révocation (décision humaine).

### 10.4 Ce que le recouvrement ne fait jamais [R, R-F20]
Priver l'élève d'un document scolaire, d'un bulletin, d'un résultat, d'un certificat ; le nommer sur une liste ; l'exclure de la cantine ou d'un voyage sans décision motivée de la direction et dialogue avec la famille (FIN-12 de la matrice : « exclusion de cantine pour impayé sans lien avec le suivi social » est un **trou volontaire** — ScolIA ne l'automatise pas) ; informer les professeurs ; relancer une famille en deuil.

---

## 11. Remboursements et crédits [R]

- **Naissance d'un crédit** : trop-perçu, avoir non imputé, excédent de bourse, excédent de voyage, caution à rendre, départ.
- **Proposition** immédiate dans la file : rembourser (virement vers un IBAN **vérifié** du payeur : celui du mandat actif, ou saisi par le payeur avec contrôle modulo 97 et signature) ou imputer sur une pièce à venir **si le payeur l'accepte** ou si le règlement financier le prévoit (Q-F14).
- **Lot de virements sortants** (pain.001 [NV version]) : validation N3, seconde validation au-delà d'un seuil, rapprochement au relevé ; ou chèque (cas rare).
- **Crédit dormant** : un crédit non réclamé après départ est proposé au remboursement, puis, après relances, traité selon les règles comptables (socle 48) [NV].
- **Remboursement d'un paiement par carte** : par l'API du prestataire, vers la même carte.

---

## 12. Attestations et documents de paiement [R, socle 7]

- **Quittance** (pièce soldée) [F, à stocker] ; **reçu** (paiement partiel, espèces) ; **attestation de paiement** pour un employeur, un comité d'entreprise, la CAF (montants payés sur une période, par élève, par catégorie, sans autre donnée) en libre-service (PAY-180) ; **attestation de frais de garde** pour les moins de 6 ans (CGI 200 quater B : garde séparée de la nourriture) (PAY-181) ; **attestation de situation de compte / quitus** (« aucune somme due au … », socle 7 R-D4) (PAY-182) ; **relevé de compte**.
- Aucune « attestation fiscale de frais de scolarité » (CGI 199 quater F ne demande pas de justificatif, 24 §5.6).

---

## 13. Cas limites (CF-41 à CF-72)

| # | Situation | Traitement |
|---|---|---|
| CF-41 | Parent non payeur tente de créer un mandat avec son IBAN | refusé (R-P4) ; journalisé ; test RS3-01 |
| CF-42 | Le payeur change de banque en cours d'année | avenant au mandat, nouvelle signature ; prélèvements du lot en cours non modifiés ; pré-notification inchangée si dates et montants inchangés |
| CF-43 | Compte joint, le mandat est signé par l'autre titulaire du compte | titulaire ≠ payeur admis ; le titulaire signe ; le payeur reste débiteur de la facture |
| CF-44 | Grand-père paie par prélèvement sur son compte la part de la mère | si le grand-père est payeur (socle 4) : son mandat ; sinon : la mère payeuse, compte d'un tiers comme titulaire (CF-43) |
| CF-45 | Prélèvement rejeté pour provision insuffisante (AM04) | rejet, échéance rouverte, re-présentation au lot suivant avec pré-notification spécifique, avis neutre [F] |
| CF-46 | Second rejet dans l'année | retour au paiement manuel [F], explication des autres moyens |
| CF-47 | Rejet « compte clos » (AC04) | motif définitif : paiement manuel ; mandat révoqué ; invitation à un nouveau mandat |
| CF-48 | Rejet « débiteur décédé » (MD07) | suspension totale du compte, aucune relance, tâche humaine ; avec CF-10 |
| CF-49 | La famille demande le remboursement (8 semaines) d'un prélèvement légitime | contre-passation, échéance rouverte, **dialogue** avant toute relance ; pas de pénalité |
| CF-50 | Contestation « je n'ai jamais signé » (13 mois) | mandat suspendu ; dossier de preuve de signature (socle 7 D65) ; décision de la direction |
| CF-51 | L'établissement a prélevé deux fois | reversal ≤ 5 jours ouvrés ou remboursement par virement, avec excuses ; test d'unicité (376) |
| CF-52 | Mandat inutilisé depuis 36 mois | caducité automatique, avis préalable 30 jours, nouveau mandat requis |
| CF-53 | Lot validé, erreur découverte avant la remise | annulation du lot (N3), nouveau lot ; le fichier annulé est conservé et marqué |
| CF-54 | Lot remis à la banque deux fois | même empreinte et même MsgId détectés : refus de remettre ; si déjà fait, revocation auprès de la banque |
| CF-55 | Fichier refusé par la banque (format) | lot `refuse_banque`, correction, nouveau fichier, même lignes |
| CF-56 | Payeur sans compte ScolIA ni e-mail | pré-notification par courrier ; sans preuve de remise : pas de prélèvement (R-P6) |
| CF-57 | Virement sans référence d'un nom inconnu | file « à identifier », suggestions ; jamais d'imputation incertaine |
| CF-58 | Virement d'un montant qui couvre deux payeurs (la mère paie aussi la part du père) | imputé sur son compte ; excédent = crédit de la mère ; **jamais** sur le compte du père sans accord écrit des deux |
| CF-59 | Chèque signé par un grand-parent pour la facture de la mère | accepté, imputé au compte de la mère, tireur noté |
| CF-60 | Chèque sans provision | R-transaction « chèque impayé », contre-passation, avis neutre, relance selon le parcours |
| CF-61 | Espèces : 1 200 € pour une facture | refus au-delà de 1 000 € (D. 112-3) ; fractionnement artificiel **interdit** (même dette) ; autre moyen proposé |
| CF-62 | Enveloppe d'espèces remise à l'accueil, montant compté différent à la comptabilité | écart signalé à l'accueil et à la direction, reçu d'accueil comme preuve |
| CF-63 | Paiement en ligne réussi chez le prestataire mais notification non reçue | réconciliation par l'API du prestataire (tâche nocturne) ; aucun double paiement proposé à la famille pendant ce temps |
| CF-64 | Paiement en ligne en double | remboursement automatique proposé (N2) |
| CF-65 | Plan d'apurement non respecté | proposition de révocation ; décision humaine ; reprise du parcours |
| CF-66 | Famille en difficulté signalée par l'assistante sociale | situation `difficulte` (sans motif), suspension proposée, orientation fonds social (26) |
| CF-67 | Impayé de 18 mois sans acte interruptif | alerte prescription (R-P14) ; décision de la direction |
| CF-68 | Créance irrécouvrable | passage en perte (socle 48) par décision motivée de la direction ; plus aucune relance ; données d'impayé supprimées selon le référentiel CNIL |
| CF-69 | Trop-perçu d'une famille partie | remboursement proposé vers l'IBAN du mandat ; sinon demande d'IBAN par lien nominatif |
| CF-70 | Changement de l'IBAN créancier (nouvelle banque de l'OGEC) | N3 + second validateur + délai 24 h ; ICS inchangé [V] ; mandats non concernés |
| CF-71 | Tentative de changement de l'IBAN créancier par un compte comptable compromis | second validateur refuse ; alerte ; journal |
| CF-72 | Élève mineur qui recharge son porte-monnaie de cantine avec la carte d'un parent | rechargement par le payeur de la catégorie seulement ; l'élève voit son solde, pas les paiements |

---

## 14. IA (contrat du socle) [R]

**Fait** : expliquer un rejet à la famille (« votre banque a refusé le prélèvement pour provision insuffisante ; voici vos moyens de payer ») ; proposer l'identification d'un virement sans référence (suggestions classées, **la personne choisit**) ; résumer la file du jour ; préparer un courrier de relance ou de réponse (brouillon, **N3** à l'envoi) ; aider une famille à composer une demande de plan d'apurement (brouillon soumis à la comptabilité).
**Ne fait jamais** : valider un lot, imputer, rembourser, envoyer une relance, choisir une situation de dossier, **noter le risque** d'une famille, lire un IBAN (le contexte n'a que des IBAN masqués), utiliser un motif social.
**Capacités** : `paiements.expliquer_rejet` (lecture, payeur ou `paiements.lire`) ; `paiements.suggerer_identification` (lecture, `paiements.encaisser`, C2 pseudonymisé) ; `paiements.preparer_relance` (brouillon, `paiements.relancer`, N3) ; `paiements.preparer_plan` (brouillon, payeur, validé par la comptabilité) ; `paiements.resume_file` (lecture, `paiements.lire`).

---

## 15. Actions métier (PAY-100 et suivantes)

Abréviations du 24 §17 ; ★ = N3.

### 15.1 Créancier, banque, paramètres
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-100 | Voir les paramètres du créancier (masqués) | CPT, DIR · `paiements.lire` | IBAN masqué | — |
| PAY-101 | Proposer un changement des paramètres du créancier ★ | CPT · `paiements.creancier` | chiffré ; en attente de second validateur | `paiements.creancier_propose` |
| PAY-102 | Valider le changement du créancier ★ | DIR (autre personne) · `paiements.creancier_valider` | délai 24 h avant usage ; alerte à tous les comptables | `paiements.creancier_modifie` |
| PAY-103 | Déclarer un second créancier (OGEC, série de pièces) ★ | DIR · `paiements.creancier` | D-027 F | idem |
| PAY-104 | Paramétrer le délai de pré-notification | CPT · `paiements.parametrer` | 14 j par défaut ; < 14 seulement si contrat (S-172) | — |
| PAY-105 | Paramétrer le calendrier des lots (jour, J-10) | CPT · `paiements.parametrer` | — | — |
| PAY-106 | Paramétrer les seuils (seconde validation, retour au manuel, frais de rejet) | DIR · `paiements.parametrer` | frais 0 par défaut | — |
| PAY-107 | Paramétrer les caisses et leurs responsables | CPT · `paiements.parametrer` | — | — |
| PAY-108 | Paramétrer les profils d'import de relevés | CPT · `paiements.parametrer` | — | — |
| PAY-109 | Paramétrer les paliers de relance (délais, textes encadrés) | CPT, DIR · `paiements.parametrer` | aucun frais ; 5 paliers au plus [F] | — |
| PAY-110 | Paramétrer le prestataire de paiement en ligne (après contrat, Q-P1) ★ | DIR · `paiements.parametrer` | clés au coffre de secrets, jamais en base en clair ni dans le frontend | — |

### 15.2 Mandats
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-111 | Signer un mandat en ligne | PAY (payeur seulement) | §7.2 ; RUM serveur ; modulo 97 ; passkey ou code | `paiements.mandat_signe` |
| PAY-112 | Reprendre l'IBAN masqué d'un autre établissement à l'inscription (D-014) | PAY | consentement ; nouveau mandat ; trace | `paiements.iban_repris` |
| PAY-113 | Inviter un payeur à signer un mandat | CPT, ADF · `paiements.mandats` | lien nominatif | `paiements.mandat_invitation` |
| PAY-114 | Saisir un mandat papier (scan obligatoire) | CPT · `paiements.mandats` | contrôle par une seconde personne si S-171 | `paiements.mandat_saisi` |
| PAY-115 | Contrôler un mandat papier | autre CPT · `paiements.mandats` | — | `paiements.mandat_controle` |
| PAY-116 | Voir mes mandats (masqués), télécharger le document | PAY | — | — |
| PAY-117 | Modifier mon IBAN (avenant) | PAY | nouvelle signature ; effet au prochain lot non validé | `paiements.mandat_avenant` |
| PAY-118 | Révoquer mon mandat | PAY | effet immédiat sur les lots non validés ; information sur les lots remis | `paiements.mandat_revoque` |
| PAY-119 | Révoquer un mandat (établissement) ★ | CPT · `paiements.mandats` | motif ; prélèvements prévus annulés (FD-10) | idem |
| PAY-120 | Suspendre un mandat (litige, décès) | CPT · `paiements.mandats` | — | `paiements.mandat_suspendu` |
| PAY-121 | Rendre caducs les mandats de 36 mois sans collecte | SYS | avis 30 j avant | `paiements.mandat_caduc` |
| PAY-122 | Voir la preuve de signature d'un mandat | CPT, DIR · `paiements.mandats` | dossier de preuve (socle 7) | — |
| PAY-123 | Voir l'historique des IBAN d'un payeur (masqués) | CPT · `paiements.mandats` | jamais en clair | — |
| PAY-124 | Lister les payeurs en prélèvement sans mandat actif | CPT · `paiements.lire` | file | — |
| PAY-125 | Choisir mon mode de paiement (prélèvement, manuel, carte) | PAY | selon les modes ouverts par l'établissement | `paiements.mode_modifie` |
| PAY-126 | Imposer le retour au paiement manuel (motif) | CPT · `paiements.mandats` | [F] | idem |

### 15.3 Pré-notifications et lots SEPA
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-127 | Émettre les pré-notifications annuelles avec les factures | SYS | remise prouvée ; ≥ 14 j | `paiements.prenotification_remise` |
| PAY-128 | Émettre une pré-notification spécifique (changement, re-présentation) | SYS / CPT | idem | idem |
| PAY-129 | Voir l'état des pré-notifications (remises, non remises) | CPT · `paiements.lire` | non remise → pas de prélèvement | — |
| PAY-130 | Préparer le lot du mois | SYS (J-10) / CPT · `paiements.prelever` | §7.4 ; écartés expliqués | `paiements.lot_prepare` |
| PAY-131 | Contrôler le lot (écarts, premières, re-présentations) | CPT · `paiements.prelever` | — | — |
| PAY-132 | Retirer une ligne du lot (motif) | CPT · `paiements.prelever` | — | — |
| PAY-133 | Valider le lot ★ | CPT · `paiements.prelever_valider` | seconde validation au-delà du seuil | `paiements.lot_valide` |
| PAY-134 | Seconde validation d'un lot ★ | DIR ou autre CPT | personne différente | idem |
| PAY-135 | Générer le fichier pain.008.001.08 (contrôle XSD, empreinte) | SYS | adresses structurées ; refus si XSD invalide | `paiements.fichier_genere` |
| PAY-136 | Télécharger le fichier pour remise à la banque | CPT · `paiements.prelever` | URL courte ; journalisé | — |
| PAY-137 | Déclarer le lot remis à la banque | CPT · `paiements.prelever` | statut `remis` | `paiements.lot_remis` |
| PAY-138 | Remettre par EBICS (si contrat, Q-P2) | SYS | — | idem |
| PAY-139 | Annuler un lot avant remise ★ | CPT · `paiements.prelever_valider` | fichier conservé et marqué | `paiements.lot_annule` |
| PAY-140 | Demander la revocation d'un lot remis | CPT ★ | auprès de la banque ; trace | `paiements.lot_revocation` |
| PAY-141 | Constater l'exécution d'un lot sans relevé (manuel, J+5) | CPT · `paiements.prelever` | [F] ; mention « sans relevé » | `paiements.lot_execute` |
| PAY-142 | Voir les prélèvements d'un payeur et leur état | CPT · `paiements.lire` ; PAY (les siens) | — | — |
| PAY-143 | Voir mon prochain prélèvement | PAY | date, montant | — |

### 15.4 R-transactions
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-144 | Enregistrer un rejet (relevé ou saisie) | SYS / CPT · `paiements.prelever` | motif normalisé ; suite [F] | `paiements.rejet` |
| PAY-145 | Enregistrer un retour (≤ 5 jours après règlement) | SYS / CPT | contre-passation, quittance annulée par pièce | `paiements.retour` |
| PAY-146 | Enregistrer un remboursement demandé par le titulaire (8 semaines) | SYS / CPT | contre-passation ; dialogue avant relance | `paiements.remboursement_titulaire` |
| PAY-147 | Enregistrer une contestation non autorisée (13 mois) | CPT | mandat suspendu ; dossier de preuve | `paiements.contestation_mandat` |
| PAY-148 | Émettre un reversal (erreur de l'établissement) ★ | CPT · `paiements.prelever_valider` | ≤ 5 jours ouvrés | `paiements.reversal` |
| PAY-149 | Choisir la suite d'un rejet (re-présentation, manuel, contact) | CPT · `paiements.prelever` | défaut [F] | — |
| PAY-150 | Voir le tableau des rejets du mois (motifs, montants) | CPT, DIR · `paiements.lire` | — | — |
| PAY-151 | Expliquer un rejet à la famille (avis neutre + détail dans le compte) | SYS | — | — |

### 15.5 Règlements, caisses, remises
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-152 | Chercher un payeur au guichet (nom, facture, référence) | ADF, CPT · `paiements.encaisser` | ne montre que le compte du payeur | — |
| PAY-153 | Enregistrer un règlement (chèque, espèces, virement, autre) | ADF, CPT · `paiements.encaisser` | transaction unique sous verrou ; reçu | `paiements.reglement` |
| PAY-154 | Désigner l'imputation (choix de la famille) | ADF, CPT | — | — |
| PAY-155 | Enregistrer des chèques en série | CPT · `paiements.encaisser` | contrôle du total | idem |
| PAY-156 | Enregistrer une enveloppe à l'accueil | ACC · `paiements.recevoir` | reçu d'accueil ; « à remettre » | `paiements.enveloppe_recue` |
| PAY-157 | Réceptionner les enveloppes de l'accueil | CPT · `paiements.encaisser` | écart → alerte | `paiements.enveloppe_receptionnee` |
| PAY-158 | Refuser un paiement en espèces au-delà du plafond | SYS | D. 112-3 | — |
| PAY-159 | Ouvrir / clôturer une caisse (comptage, écart motivé) | CPT · `paiements.caisse` | double comptage si S-170 | `paiements.caisse_close` |
| PAY-160 | Générer un bordereau de remise de chèques | CPT · `paiements.encaisser` | liste, total | `paiements.remise_cheques` |
| PAY-161 | Générer un bordereau de remise d'espèces | CPT · `paiements.caisse` | — | `paiements.remise_especes` |
| PAY-162 | Enregistrer un chèque impayé | CPT · `paiements.encaisser` | contre-passation | `paiements.cheque_impaye` |
| PAY-163 | Annuler un règlement (erreur de saisie) ★ | CPT · `paiements.annuler` | contre-passation ; impossible après remise | `paiements.reglement_annule` |
| PAY-164 | Délettrer et réimputer (motif) | CPT · `paiements.annuler` | trace | `paiements.reimputation` |
| PAY-165 | Imprimer / envoyer un reçu, une quittance | ADF, CPT | document stocké | — |
| PAY-166 | Voir le journal d'une caisse | CPT, DIR | — | — |
| PAY-167 | Enregistrer un paiement par chèques-vacances (si conventionné) | CPT | réglage | idem PAY-153 |
| PAY-168 | Enregistrer le versement d'un tiers (aide) | CPT · `paiements.encaisser` | imputé comme aide affectée (24) | `paiements.aide_recue` |

### 15.6 Paiement en ligne
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-169 | Payer une échéance, un solde, un voyage en ligne | PAY | page hébergée ; montant ≤ dû ; aucune donnée de carte chez nous | `paiements.paiement_en_ligne_initie` |
| PAY-170 | Recevoir la notification signée du prestataire | SYS | signature vérifiée ; idempotent | `paiements.paiement_en_ligne_confirme` |
| PAY-171 | Réconcilier les paiements sans notification | SYS (nuit) | CF-63 | — |
| PAY-172 | Rembourser un paiement par carte | CPT ★ | API du prestataire | `paiements.remboursement_carte` |
| PAY-173 | Envoyer un lien de paiement à un payeur sans compte | CPT · `paiements.relancer` | lien nominatif à durée limitée | — |
| PAY-174 | Rapprocher les versements groupés du prestataire (nets de frais) | SYS / CPT | frais en dépense (48) | — |

### 15.7 Rapprochement bancaire
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-175 | Importer un relevé (camt.053 / 054, CSV de la banque) | CPT · `paiements.banque` | doublon ignoré (empreinte) | `paiements.releve_importe` |
| PAY-176 | Rapprocher automatiquement | SYS | règles certaines seulement | `paiements.rapprochement` |
| PAY-177 | Traiter la file d'exceptions (identifier, affecter, frais bancaires) | CPT · `paiements.banque` | suggestions ; choix humain | idem |
| PAY-178 | Voir l'état de rapprochement d'un mois | CPT, DIR | — | — |
| PAY-179 | Défaire un rapprochement (motif) | CPT ★ | trace | `paiements.rapprochement_annule` |

### 15.8 Attestations
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-180 | Obtenir une attestation de paiement (période, élève, catégorie) | PAY (libre-service), ADF | rien d'autre que les montants payés | `paiements.attestation` |
| PAY-181 | Obtenir l'attestation annuelle de frais de garde (< 6 ans) | PAY | garde séparée de la nourriture [V partiel] | idem |
| PAY-182 | Obtenir une attestation de situation de compte (quitus) | PAY, ADF | « aucune somme due au … » | idem |
| PAY-183 | Obtenir un relevé de compte | PAY, ADF | — | — |

### 15.9 Relances et recouvrement
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-184 | Proposer les relances du jour | SYS | jamais avant l'échéance ni pendant un prélèvement [F] ; suspensions | `paiements.relances_proposees` |
| PAY-185 | Confirmer l'envoi d'une relance (paliers 1–2) | CPT · `paiements.relancer` | payeur seul ; plage calme | `paiements.relance_envoyee` |
| PAY-186 | Confirmer une mise en demeure ★ | DIR · `paiements.recouvrer` | courrier ; aucune menace sur l'élève | idem |
| PAY-187 | Annuler une relance proposée (motif) | CPT | [F] | `paiements.relance_annulee` |
| PAY-188 | Choisir la situation d'un dossier | CPT · `paiements.recouvrer` | liste fermée ; aucun motif social | `paiements.dossier_situation` |
| PAY-189 | Suspendre un dossier (deuil, contestation, aide en instruction) | CPT, DIR | — | `paiements.dossier_suspendu` |
| PAY-190 | Proposer un plan d'apurement | CPT · `paiements.recouvrer` | échéancier dérogatoire | `paiements.plan_propose` |
| PAY-191 | Demander un plan d'apurement en ligne | PAY | soumis à la comptabilité | `paiements.plan_demande` |
| PAY-192 | Signer un plan d'apurement | PAY | socle 7 ; reconnaissance de dette | `paiements.plan_signe` |
| PAY-193 | Révoquer un plan non respecté | CPT, DIR ★ | décision humaine | `paiements.plan_revoque` |
| PAY-194 | Orienter vers l'assistante sociale / le fonds social | CPT, DIR | sans transmettre de montant détaillé sans accord de la famille | `paiements.orientation_sociale` |
| PAY-195 | Ouvrir une médiation | DIR | rendez-vous (socle 19) | `paiements.mediation` |
| PAY-196 | Confier un dossier à une société de recouvrement (si contrat, Q-P8) ★ | DIR · `paiements.recouvrer` | données minimales ; convention R. 124 ; lettre L. 111-8 | `paiements.recouvrement_externe` |
| PAY-197 | Préparer un dossier d'injonction de payer ★ | DIR | pièces : contrat, factures, mise en demeure, relevé | `paiements.injonction_preparee` |
| PAY-198 | Voir les alertes de prescription (18 mois) | CPT, DIR | — | — |
| PAY-199 | Passer une créance en perte ★ | DIR · `paiements.recouvrer` | socle 48 ; plus de relance | `paiements.creance_perte` |
| PAY-200 | Tableau des impayés (ancienneté, situation, montant) | DIR, CPT, OGEC (agrégé) | nominatif selon droit | — |

### 15.10 Remboursements, crédits, porte-monnaie
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-201 | Voir les crédits nés (file) | CPT · `paiements.rembourser` | — | — |
| PAY-202 | Proposer au payeur : rembourser ou imputer | SYS / CPT | Q-F14 | `paiements.credit_propose` |
| PAY-203 | Fournir un IBAN de remboursement | PAY | modulo 97, signature | `paiements.iban_remboursement` |
| PAY-204 | Valider un lot de remboursements ★ | CPT, seconde validation au-delà du seuil | pain.001 [NV] | `paiements.remboursements_valides` |
| PAY-205 | Recharger le porte-monnaie de cantine | PAY de la catégorie | carte, virement | `paiements.porte_monnaie_recharge` |
| PAY-206 | Débiter le porte-monnaie à la consommation | SYS (40) | solde négatif borné (S-177) | `paiements.porte_monnaie_debit` |
| PAY-207 | Voir le solde du porte-monnaie | PAY, élève (solde seul) | — | — |
| PAY-208 | Alerter le payeur d'un solde bas | SYS | in-app | — |

### 15.11 Pilotage et IA
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| PAY-209 | Prévisionnel d'encaissements par mois | DIR, OGEC, CPT · `paiements.pilotage` | agrégé | — |
| PAY-210 | Produire les écritures de banque pour le socle 48 | SYS / CPT | compte par moyen (512, 530 caisse, 5112 chèques à encaisser [H, plan à valider]) | `paiements.ecritures_produites` |
| PAY-211 | Expliquer un rejet, suggérer une identification, préparer une relance (brouillon) | IA | §14 ; N3 à l'envoi | — |

---

## 16. Règles transverses du socle 25 (R-P1 et suivantes) [R]

| # | Règle | Lot |
|---|---|---|
| R-P1 | **Un règlement = un payeur, un moyen, une preuve** ; imputé sur le seul compte de ce payeur | P1 |
| R-P2 | **Paramètres du créancier** chiffrés, jamais en URL ; changement N3 + second validateur + délai 24 h + alerte | F0, P3 |
| R-P3 | Encaissement, imputation, quittance dans **une transaction**, sous verrou du compte et de la pièce | F0 |
| R-P4 | **Seul le payeur** (ou le titulaire du compte qu'il désigne) signe, modifie, révoque un mandat pour son compte | F0 |
| R-P5 | Mandat **prouvé** (signature, version du texte, empreinte) ; RUM serveur ; IBAN contrôlé modulo 97 | F0, P2 |
| R-P6 | **Aucun prélèvement sans pré-notification remise** (canal et date prouvés) au moins N jours avant | F0, P3 |
| R-P7 | **Lot validé immuable** ; annulation et réfection ; remise ≠ génération | P3 |
| R-P8 | Fichier **pain.008.001.08**, adresses structurées, contrôle XSD avant toute remise | F0 (adresses), P3 |
| R-P9 | Mandat **révoqué, caduc ou suspendu** : aucune ligne de prélèvement | F0 |
| R-P10 | Caducité **automatique** à 36 mois sans collecte, avec avis | P2 |
| R-P11 | R-transaction = **contre-passation datée**, jamais suppression ; quittance annulée par une pièce | P4 |
| R-P12 | **La banque fait foi** : exécution constatée par relevé, ou constat manuel marqué « sans relevé » | P5 |
| R-P13 | Rapprochement **automatique seulement si certain** ; sinon file humaine | P5 |
| R-P14 | **Alerte de prescription** à 18 mois sans acte interruptif | P6 |
| R-P15 | Relance **proposée, confirmée, adressée au seul payeur**, dans la plage calme, suspendue par plan, contestation, deuil, aide en instruction, prélèvement en cours | P6 |
| R-P16 | **Aucun frais de recouvrement** facturé ; pénalités seulement si le contrat les prévoit | F0 |
| R-P17 | Situation d'un dossier choisie par une personne ; **aucun score** ; aucun motif social | P6 |
| R-P18 | **Espèces ≤ 1 000 €** par paiement (résident) ; reçu numéroté immédiat | P1 |
| R-P19 | **Aucune donnée de carte** dans ScolIA ; page hébergée ; notifications signées et idempotentes | P8 |
| R-P20 | Un **crédit** est proposé au remboursement dès sa naissance ; jamais imputé sur un autre payeur | P7 |
| R-P21 | IBAN **chiffré, masqué**, jamais dans un journal, un événement, une URL, un contexte d'IA ; déchiffré seulement pour le fichier bancaire et l'affichage masqué | F0 |
| R-P22 | **Séparation des tâches** : saisie ≠ validation d'un lot au-delà du seuil ; proposition ≠ validation du créancier | P3 |
| R-P23 | Un **relevé importé deux fois** ne produit rien ; un **lot remis deux fois** est refusé | P3, P5 |
| R-P24 | L'élève **n'est jamais** destinataire d'un avis de paiement, de rejet ou de relance ; il voit au plus le solde de son porte-monnaie | F0, P9 |

---

## 17. Droits et sécurité

### 17.1 Catalogue d'actions proposé (D-017) [R]
| Droit | Contenu | Modèles par défaut |
|---|---|---|
| `paiements.lire` | voir règlements, prélèvements, lots, rejets, relances | Compta, Direction |
| `paiements.recevoir` | enregistrer une enveloppe (accueil) | Accueil, Secrétariat |
| `paiements.encaisser` | règlements, chèques, remises, chèques impayés | Compta, Secrétariat (au guichet) |
| `paiements.caisse` | ouvrir, clôturer une caisse, remise d'espèces | Compta |
| `paiements.annuler` ★ | annuler un règlement, réimputer | Compta |
| `paiements.mandats` | saisir, contrôler, révoquer, suspendre des mandats | Compta |
| `paiements.prelever` | préparer, contrôler, générer, remettre | Compta |
| `paiements.prelever_valider` ★ | valider, annuler un lot, reversal | Compta (seconde validation : Direction) |
| `paiements.banque` | relevés, rapprochement | Compta |
| `paiements.relancer` | confirmer les relances (paliers 1–2), liens de paiement | Compta |
| `paiements.recouvrer` ★ | situations, plans, mise en demeure, recouvrement externe, perte | Direction (Compta pour les plans) |
| `paiements.rembourser` ★ | lots de remboursements | Compta (seconde validation au-delà du seuil) |
| `paiements.creancier` ★ / `paiements.creancier_valider` ★ | proposer / valider un changement de créancier (**personnes différentes**) | Compta / Direction |
| `paiements.parametrer` ★ | paramètres du 25 | Direction, Compta |
| `paiements.pilotage` | tableaux agrégés | Direction, OGEC |

### 17.2 Sécurité [R]
- **Fraude au changement de coordonnées** (la plus probable [H]) : R-P2 ; affichage permanent « compte crédité : FR76 •••• 1234 (depuis le 03/09/2026) » sur chaque lot ; alerte de la direction à chaque changement.
- **Compte d'un comptable compromis** : passkey obligatoire (D-009), ré-authentification N3 sur chaque geste ★, seconde validation des lots et des remboursements au-delà des seuils, journal en ajout seul consulté par la direction.
- **Fichiers bancaires** : générés côté serveur, stockés chiffrés (socle 7, stockage objet), téléchargés par URL pré-signée courte, empreinte conservée ; jamais envoyés par e-mail.
- **Notifications du prestataire** : signature vérifiée, horodatage, idempotence par identifiant de transaction, rejeu refusé.
- **RLS stricte** avant tout pilote (RF-20) ; clés étrangères composites ; matrice de séparation sur toutes les routes du 25.
- **Données minimales vers un tiers** (société de recouvrement, prestataire) : nom du payeur, montant, pièces ; jamais le nom de l'élève au prestataire de paiement au-delà d'une référence (S-178).

---

## 18. Événements [R]

**Émis** (sans montant ni IBAN) : `paiements.mandat_signe|avenant|revoque|caduc|suspendu`, `paiements.prenotification_remise`, `paiements.lot_prepare|valide|remis|annule|execute`, `paiements.rejet|retour|remboursement_titulaire|reversal`, `paiements.reglement`, `paiements.imputation` (→ 24 : pièce soldée), `paiements.reglement_annule`, `paiements.releve_importe`, `paiements.rapprochement`, `paiements.relance_envoyee`, `paiements.plan_signe|revoque`, `paiements.dossier_suspendu`, `paiements.credit_propose`, `paiements.remboursements_valides`, `paiements.porte_monnaie_*`, `paiements.creancier_modifie` (alerte), `paiements.ecritures_produites`. Les événements historiques `fac.sepa_*`, `fac.reglement`, `fac.relance_*` [F] reçoivent un alias pendant la transition.
**Consommés** : `facturation.piece_emise`, `echeancier_*`, `avoir_emis`, `gel` (24) ; `payeurs.modifies`, `personne.decedee` (4) ; `eleve.decede` (2) ; `restauration.consommations_jour` (porte-monnaie, 40) ; `aide.*` (26).

---

## 19. Notifications (natures déclarées au socle 8) [R]

| Nature | Urgence · sensibilité | Destinataires | Canaux | Contenu externe |
|---|---|---|---|---|
| `finances.prenotification` | U1 · C2 | payeur | in-app **et** e-mail (document) ; courrier si aucun canal ; **remise prouvée** | « Information sur vos prochains prélèvements » |
| `finances.mandat_a_signer` | U1 · C1 | payeur | in-app, e-mail, lien nominatif | « Un mandat de prélèvement est à signer » |
| `finances.paiement_enregistre` | U0 · C2 | le payeur qui a payé | in-app, résumé | « Paiement reçu » |
| `finances.rejet_prelevement` | U1 · C2 | payeur | in-app, e-mail enveloppe | « Information sur un prélèvement » |
| `finances.relance` (paliers 1–3) | U1 · C2 | payeur seul | in-app, e-mail enveloppe ; courrier pour la mise en demeure | « Information de [établissement] » |
| `finances.plan_propose` | U1 · C2 | payeur | in-app, e-mail enveloppe | idem |
| `finances.remboursement` | U0 · C2 | payeur | in-app | « Remboursement en cours » |
| `finances.porte_monnaie_bas` | U0 · C1 | payeur de la catégorie | in-app, push | « Solde bas » |
| `finances.creancier_modifie` | U3 · C1 | direction, comptables | in-app, e-mail, SMS | « Modification des coordonnées bancaires de l'établissement » |
| `finances.lot_a_valider` | U1 · C1 | valideurs | in-app | — |

---

## 20. Écrans clés [R]

1. **File du jour** (CPT) : lots à préparer / valider, rejets, relevé à importer, exceptions de rapprochement, enveloppes à réceptionner, relances proposées, crédits à rembourser, alertes de prescription.
2. **Guichet** : recherche du payeur, son compte (et lui seul), saisie rapide, imputation, reçu.
3. **Chèques en série** : grille de saisie, total contre bordereau, exceptions.
4. **Lot SEPA** : assistant préparation → contrôle → validation (N3, seconde validation) → fichier → remise ; compte crédité affiché en permanence.
5. **Rapprochement** : relevé à gauche, objets ScolIA à droite, certains rapprochés en vert, exceptions avec suggestions.
6. **Dossier de recouvrement** : échéances, paliers, situation, suspensions, plan, courriers, journal.
7. **Espace famille** : prochain prélèvement, mandat, payer, plan d'apurement, attestations, porte-monnaie.
8. **Créancier** : coordonnées masquées, historique des changements, proposition et validation séparées.

---

## 21. Modèle de données cible (brique `paiements`) [R]

Mêmes principes que le 24 §23 (évolution des tables existantes, `etablissement_id`, clés composites, RLS, aucun `CASCADE`, ajout seul pour les journaux).

```
ent_fin_creancier                                -- remplace les colonnes crediteur_* de ent_fin_parametres
  id, etablissement_id, nom, ics, iban_chiffre, bic_chiffre, adresse JSONB (structurée), serie_pieces TEXT[]
  actif_du TIMESTAMPTZ, propose_par, valide_par (≠ propose_par, CHECK), version, remplace_id
ent_fin_mandat  (évolution de ent_fac_sepa_mandat)
  + compte_id → ent_fin_compte, creancier_id, type ('rcur','ooff'), titulaire_chiffre,
    etat ('en_attente_signature','actif','suspendu','revoque','caduc','remplace'),
    preuve_signature_id (socle 7), document_id, derniere_collecte_le, version, remplace_id
  -- un mandat actif par (compte, créancier)
ent_fin_prenotification (+ canal, remise_le, preuve_id, document_id, nature ('annuelle','specifique'))
ent_fin_lot
  id, etablissement_id, type ('prelevement','remboursement'), creancier_id, date_execution,
  statut ('prepare','valide','fichier_genere','remis','execute','annule','refuse_banque'),
  nb, total_centimes, valide_par, second_valideur NULL, fichier_document_id, empreinte, msg_id
ent_fac_sepa_prelevement (+ lot_id, sequence, statut + 'remis','revoque','retourne','rembourse')
ent_fin_r_transaction
  id, prelevement_id, type ('reject','refusal','return','refund','refund_non_autorise','reversal','revocation'),
  code_motif, recu_le, releve_ligne_id NULL, contre_passation_id NULL, suite
ent_fin_reglement (+ compte_id, preuve JSONB (n° chèque, banque, tireur, référence, transaction), caisse_id NULL,
                   remise_id NULL, statut ('enregistre','remis','rapproche','annule'), annule_par_id NULL)
ent_fin_caisse, ent_fin_caisse_mouvement, ent_fin_caisse_cloture (comptage JSONB, ecart, motif)
ent_fin_enveloppe                                -- accueil → comptabilité
ent_fin_remise                                   -- bordereaux de chèques et d'espèces
ent_fin_releve (empreinte UNIQUE, numero, compte, du, au, format)
ent_fin_releve_ligne (date, montant signé, libellé, référence, e2e, contrepartie_nom)
ent_fin_rapprochement (releve_ligne_id, objet_type, objet_id, mode ('auto','manuel'), par, annule_le NULL)
ent_fin_dossier_recouvrement (compte_id, situation, palier, suspendu_jusqu_au NULL, motif_suspension (liste fermée))
ent_fin_relance (existant 204 ; frais forcés à 0 ; + dossier_id, canal, remise_le)
ent_fin_plan_apurement (+ échéances via ent_fin_echeancier_version du 24 ; document signé)
ent_fin_credit_mouvement (compte_id, origine, montant, statut ('ne','propose','rembourse','impute'))
ent_fin_remboursement (compte_id, montant, iban_chiffre, lot_id, statut)
ent_fin_paiement_en_ligne (compte_id, intention_id, prestataire_ref UNIQUE, montant, statut, notifie_le, signature_ok)
ent_fin_porte_monnaie (compte_id, eleve_id, categorie, solde_centimes, seuil_bas, plancher)
ent_fin_porte_monnaie_mouvement (ajout seul)
ent_fin_attestation (type, periode, document_id)
ent_fin_paiements_journal (ajout seul)
```

---

## 22. Plan de migration depuis l'existant

| Étape | Quoi | Garantie |
|---|---|---|
| MIG-P1 (F0) | voir 24 §24 MIG-1 et MIG-2 ; plus : IBAN créancier chiffré ; exclusions de l'export ; pré-notification exigée ; RUM serveur pour les nouveaux mandats | aucune donnée monétaire modifiée par script |
| MIG-P2 (P2) | `ent_fac_sepa_mandat` enrichie en `ent_fin_mandat` (colonnes ajoutées, renommage par vue de compatibilité) ; mandats existants : `preuve_signature_id` NULL, marqués « preuve non conservée » ; **invitation** des payeurs à re-signer en ligne à la rentrée suivante (pas de révocation automatique) ; `derniere_collecte_le` calculée | aucun mandat actif perdu |
| MIG-P3 (P3) | `ent_fin_parametres.crediteur_*` → `ent_fin_creancier` (première version validée par la direction à la mise en service) | — |
| MIG-P4 (P5) | `ent_bank_rapprochement` conservée en lecture (labo) ; nouvelles tables de relevés | — |
| MIG-P5 (P6) | `ent_fac_rappel` (historique) gelée ; relances par paliers seules | plus de double relance |
| MIG-P6 (retrait) | routes historiques `/facturation/sepa/*`, `/facturation/banque/*`, `/facturation/rappels` retirées avec `ui.rs` | 558–559 (ou 5590–5599) |

---

## 23. Lots de code (P1 à P12) et migrations

Le lot commun **F0** et l'ordre d'ensemble sont au **24 §25**.

### 23.1 Lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **P1** encaissements au guichet | règlement par payeur sous verrou, imputation désignée, reçus et quittances **stockés**, chèques en série, enveloppes de l'accueil, caisses et clôture, bordereaux de remise, chèques impayés, plafond espèces, annulation par contre-passation | F0, F1, F2 | CF-57 à CF-62 ; deux guichets simultanés sans dépassement |
| **P2** mandats électroniques | signature par le payeur (passkey ou code), RUM serveur, modulo 97, document du mandat, avenants, révocation, suspension, caducité 36 mois, mandat papier contrôlé, reprise MIG-P2, reprise de l'IBAN masqué (D-014) | F0, F2 ; socle 7 (signature) | CF-41 à CF-44, CF-52 ; preuve de signature téléchargeable |
| **P3** lots SEPA | créancier versionné à double validation, pré-notification remise et prouvée (annuelle et spécifique), préparation, contrôle, validation N3 et seconde validation, pain.008.001.08 avec contrôle XSD, adresses structurées, remise (portail ou EBICS, Q-P2), annulation, refus de la banque | P2 ; socle 8 (remise prouvée) | fichier validé contre le XSD officiel ; aucun prélèvement sans pré-notification remise ; CF-53 à CF-56, CF-70, CF-71 |
| **P4** R-transactions | rejets, retours, remboursements 8 semaines et 13 mois, reversal, revocation, contre-passations, quittances annulées par pièce, suites, frais de rejet (0 par défaut) | P3 | CF-45 à CF-51 |
| **P5** relevés et rapprochement | import camt.053 / 054 et CSV de banque, doublons, rapprochement automatique certain, file d'exceptions, versements du prestataire | P1, P3 | 95 % de rapprochement automatique sur un relevé de labo réaliste ; rejeu sans effet |
| **P6** relances et recouvrement | parcours gradué, situations, suspensions, plans d'apurement signés, orientation sociale, médiation, mise en demeure, dossier d'injonction, alertes de prescription, perte, tableau des impayés, suppression des données d'impayé après régularisation | P1, F7 ; socles 7, 8, 16 | CF-65 à CF-68 ; aucune relance à un non-payeur, à l'élève, à une famille en deuil |
| **P7** remboursements et crédits | crédits proposés, IBAN de remboursement, lots de virements (pain.001), cautions, excédents de bourse et de voyage | P3 | CF-69 ; aucun crédit imputé sur un autre payeur |
| **P8** paiement en ligne | prestataire choisi (Q-P1), page hébergée, notifications signées, réconciliation, remboursements, liens de paiement | P1 ; **contrat** | CF-63, CF-64 ; aucune donnée de carte en base |
| **P9** porte-monnaie de cantine | rechargement, débit à la consommation (40), solde bas, plancher, solde visible de l'élève | P8 ou P1 ; socle 40 | CF-72 ; aucun refus de repas automatique pour solde négatif (R-F20) |
| **P10** attestations | paiement, frais de garde, quitus, relevé, en libre-service | P1 ; socle 7 | rien d'autre que les montants |
| **P11** trésorerie et pilotage | prévisionnel d'encaissements, état des rejets, des impayés, écritures de banque par moyen vers le 48 | P5, P6 | écritures équilibrées |
| **P12** IA | capacités §14 | P4, P6 ; `ent-ia` | aucun geste sans N3 ; aucun IBAN en contexte |

### 23.2 Migrations
| Plage si Q-B2 | Repli | Lot | Contenu |
|---|---|---|---|
| 5500 | 540 | P1 | règlements enrichis, caisses, enveloppes, remises |
| 5501 | 541 | P2 | mandats enrichis, versions, preuves |
| 5502 | 542 | P3 | créanciers versionnés, lots, pré-notifications enrichies |
| 5503 | 543 | P4 | R-transactions |
| 5504 | 544 | P5 | relevés, rapprochements |
| 5505 | 545 | P6 | dossiers de recouvrement, plans |
| 5506 | 546 | P7 | crédits, remboursements |
| 5507 | 547 | P8 | paiements en ligne |
| 5508 | 548 | P9 | porte-monnaie |
| 5509 | 549 | P10 | attestations |
| 5510–5589 | 550–557 | réserve | P11, P12 et suivants |
| 5590–5599 | 558–559 | retrait | RLS stricte, retrait des tables et routes historiques (`ent_bank_rapprochement`, `ent_fac_rappel`) après archive |

---

## 24. Tests et recette

- **XSD** : chaque fichier pain.008 et pain.001 généré par les tests est validé contre le schéma officiel (fichier de schéma versionné dans le dépôt de tests, licence à vérifier).
- **Concurrence** : deux guichets, deux validations de lot, rejeu de notification du prestataire, rejeu de relevé.
- **Séparation** : toutes les routes du 25 × (parent non payeur, payeur d'une autre catégorie, élève, professeur, accueil, autre établissement) ; création de mandat par un non-payeur refusée (RS3-01) ; IBAN jamais en clair dans une réponse, un journal, un événement (test de garde sur les sorties).
- **Créancier** : changement sans second validateur refusé ; délai de 24 h respecté ; alerte émise.
- **Recette par personas** : comptable (septembre : 300 chèques, premier lot, 12 rejets), parent séparé qui change de banque, parent non payeur qui **attaque** (mandat, relevé, attestation de l'autre), famille en difficulté (plan en ligne), fraudeur interne (changement d'IBAN créancier), banque (fichier refusé).

---

## 25. Réglages signalés (D-023)

| # | Réglage | Défaut | Pourquoi signalé |
|---|---|---|---|
| S-169 | Frais de paiement en ligne refacturés | non | coût pour la famille |
| S-170 | Double comptage à la clôture de caisse | oui au-delà de 500 € | petite équipe |
| S-171 | Contrôle d'un mandat papier par une seconde personne | non | — |
| S-172 | Délai de pré-notification < 14 jours (si contrat) | 14 jours | conformité |
| S-173 | Seuil de seconde validation d'un lot | 50 000 € | — |
| S-174 | Séquence FRST pour la première présentation | non (RCUR) | dépend de la banque |
| S-175 | Mise en demeure en recommandé | oui | coût |
| S-176 | Pas de relance de palier 2–3 le vendredi soir et veille de jour férié | oui | — |
| S-177 | Plancher du porte-monnaie (solde négatif autorisé) | −10 € | ne jamais refuser un repas |
| S-178 | Nom de l'élève transmis au prestataire de paiement | non (référence seule) | minimisation |

---

## 26. Questions à Florian (avec recommandations, appliquées par défaut — D-031)

| # | Question | Recommandation |
|---|---|---|
| Q-P1 | **Prestataire de paiement en ligne** (Q5) : lequel, à quel coût ? (EasyTransac est utilisé côté Next.js d'après la matrice) | **Soumis à Florian** (contrat, coûts) ; critères : page hébergée, hébergement européen, notifications signées, remboursements par API, frais par transaction ; en attendant, P8 n'est pas lancé. |
| Q-P2 | **Remise des fichiers à la banque** : téléchargement et dépôt sur le portail (défaut) ou **EBICS** (contrat, coût) ? Relevés électroniques camt.053 disponibles ? | **Portail par défaut** (appliqué) ; EBICS et camt **soumis à Florian** (contrat bancaire). |
| Q-P3 | ICS de l'OGEC : existe-t-il déjà (Charlemagne prélève aujourd'hui) ? | **Soumis à Florian** (information de la banque) ; ScolIA réutilise l'ICS existant (inchangé si changement de banque [V]). |
| Q-P4 | Mandats existants (Charlemagne) : reprendre avec leur RUM et leur date de signature, sans preuve, et inviter à re-signer en ligne à la rentrée suivante ? | **Oui** (appliqué) ; les RUM doivent être conservées pour la continuité des prélèvements (le créancier et la banque les connaissent) ; **données réelles soumises à Florian**. |
| Q-P5 | Pré-notification **annuelle unique** jointe à la facture, plus une spécifique à chaque changement ? | **Oui** (appliqué). |
| Q-P6 | Séquence : tout en RCUR (FRST facultatif) ? | **Oui** (appliqué), sauf exigence de la banque (S-174). |
| Q-P7 | Parcours de relance : J+7, J+30, mise en demeure à J+60 sur décision de la direction, aucun frais ? | **Oui** (appliqué). |
| Q-P8 | Recours à une **société de recouvrement** ? | **Non par défaut** (appliqué : médiation et injonction de payer d'abord) ; un contrat éventuel est **soumis à Florian**. |
| Q-P9 | **Refus de réinscription** pour impayé : possible ? | **Décision humaine de la direction seulement**, jamais automatique, jamais une sanction de l'élève (appliqué) ; **avis juridique** avant le premier cas (soumis : coût). |
| Q-P10 | Porte-monnaie de cantine : solde négatif autorisé jusqu'à un plancher, jamais de refus de repas automatique ? | **Oui** (appliqué, R-F20). |
| Q-P11 | Durées de conservation des IBAN, mandats, relevés, données d'impayé (§4.4) ? | **Hypothèses de travail appliquées** (mandat + 13 mois ; pièces 10 ans ; impayés 48 h après régularisation) ; **à valider par le DPO** (soumis). |
| Q-P12 | Changement de l'IBAN créancier : double validation par deux personnes différentes et délai de 24 h ? | **Oui** (appliqué) : c'est la fraude la plus coûteuse du domaine. |
| Q-P13 | Remboursement des trop-perçus par **virement** (lot pain.001) plutôt que par chèque ? | **Oui** (appliqué). |
| Q-P14 | Plafond des espèces : refuser au-delà de 1 000 € (D. 112-3), réglage pour les non-résidents ? | **Oui** (appliqué). |

---

## 27. Faits, hypothèses, contradictions

**Faits [F]** : §2 et 24 §2 (code lu sur `origin/ENT` `5366c196`).
**Vérifié [V]** : §4 (rulebook EPC 2025 v1.1 lu en entier ; Légifrance ; Banque de France ; CNIL).
**Hypothèses [H]** : pratiques des comptables (saisie de chèques en série, lecteurs de chèques) ; plan de comptes de banque (512, 530, 5112) à valider par l'expert-comptable ; disponibilité de camt.053 et d'EBICS chez la banque de l'OGEC ; usage d'EasyTransac côté Next.js (relevé par la matrice, non relu).
**Contradictions relevées** :
1. Le S2-9 constate l'exécution d'un prélèvement **sans relevé** : admis comme repli marqué (R-P12), mais la banque doit faire foi dès que les relevés électroniques sont disponibles.
2. Le S2-9 refacture les **frais de rejet** émis aussitôt et prévoit des **frais de relance** : contraires à CPCE L. 111-8 pour les seconds [V] ; à qualifier pour les premiers (24 Q-F21).
3. `RECETTE_SECURITE_3.md` RS3-08 (mandat sans preuve, RUM du client) est classée « faible » : ce document la remonte au lot **F0** pour la RUM et au lot **P2** pour la preuve, parce qu'un litige de mandat se perd sans preuve (remboursement 13 mois).
4. La version **pain.008.001.02** générée par l'existant n'est plus celle du standard EPC (version 2019, .001.08) [V] ; l'échéance du **15/11/2026** sur les adresses structurées est **dans cinq jours** : si l'établissement prélevait avec ScolIA, ce serait un bloquant ; ce n'est pas le cas aujourd'hui (le pilote prélève avec Charlemagne [H]), d'où sa place en F0 (vérification) et P3 (migration de format).
