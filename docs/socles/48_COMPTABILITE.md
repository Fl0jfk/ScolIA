# Socle 48 — Comptabilité, budget et achats

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code de ce socle. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement** (fichiers ciblés de `origin/main` par `git show`, rien exécuté, rien construit).
**Branche :** `ent-socle-48-49-conception`, partie de `origin/ent-mc0hoe` (`37ce0d3b`).
**Passe personas :** `docs/socles/PERSONAS_48.md` (même branche). Ses ajouts sont faits **ici**, au **§33**, marqués « ajout personas » ; **en cas de contradiction, le §33 l'emporte** ; rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` : D-013 (cloisonnement), D-017 (droits portés par les actions), D-019 (public et privé), D-021, D-023 (peu de réglages), **D-024**, **D-025** (comptabilité dans l'ENT Rust), D-026 (briques), **D-027 (E)** (on compense, on ne réémet pas), D-029 (public), **D-031**, **D-034** (doublon 2027-2028, finances dans Charlemagne), **D-035** (précision 2 : la comptabilité de l'add-on est une **ébauche à refaire de zéro**), **D-036** (aucun JSON-mémoire), D-037 à D-046 (PRA). `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14). `docs/TRANSITION_T0_INVENTAIRE.md` (M15, ligne « Comptabilité de l'établissement »). `docs/SOCLES.md` §48, Q-C, T-05. Conceptions **`24_FINANCES.md`** (§5, §6, §12, F9, F13, Q-F17, Q-F18), **`25_PAIEMENTS.md`**, `26` (via 24 §10.5), **`42_SORTIES_VOYAGES.md`** §12 (budget de séjour), `06_PERSONNELS.md` (missions, R-P11), **`49_PAIE.md`** (journal de paie), `07_DOCUMENTS.md` (pièces, PDF/A-3), `08_NOTIFICATIONS.md`, `IA_COEUR.md` §6.2 (niveaux N0–N3). Modèles de forme : `42_SORTIES_VOYAGES.md`, `24_FINANCES.md`, `10_EMPLOI_DU_TEMPS.md`.

**Conventions.** **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins de `origin/main`) ; **[V]** règle lue sur une source officielle ; **[V-sec]** lue sur une source secondaire sérieuse (éditeur juridique, ordre professionnel, cabinet), l'officiel n'ayant pas été lu ; **[NV]** non vérifié ; **[H]** hypothèse de terrain ; **[R]** recommandation.

**Numérotation propre au socle 48** (plages vérifiées libres sur les 134 branches distantes le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Défauts de l'existant (Rust et Next.js) | **KD-01**… |
| Actions cibles | **CGA-100 à CGA-299** |
| Règles | **R-CG1**… |
| Cas limites | **CCG-01**… |
| Questions | **Q-CG1**… |
| Réglages signalés | **S-4800 à S-4849** |
| Lots | **CG0 à CG14** |
| Migrations | **2960–2979** (plage **imposée par le pilote**, §28.2) |

Sommaire : §0 En bref · §1 Thèse · §2 Existant · **§3 Intégrer ou refaire ?** · §4 Acteurs · §5 L'année · §6 Réglementaire · §7 Branchements · §8 Concepts · §9 Moteur d'écritures · §10 Budget · §11 Achats · §12 Fournisseurs · §13 Factures électroniques reçues · §14 Trésorerie · §15 Immobilisations · §16 Analytique · §17 Clôture · §18 Public (EPLE) · §19 Cas limites · §20 IA · §21 Actions · §22 Règles · §23 Droits, minimisation, conservation · §24 Événements et notifications · §25 Écrans · §26 Modèle de données · §27 Reprise · §28 Lots et migrations · §29 Tests · §30 Réglages · §31 Décisions et questions · §32 Faits, hypothèses, contradictions · §33 Ajout personas · §34 Sources.

---

## 0. En bref

1. **Ce qui existe.** Côté Rust [F] : un **export d'écritures de ventes et de règlements** (`scolia_ecritures_v1`, 18 colonnes calquées sur le FEC, `ent-db/src/finances_sepa.rs:1388-1615`, migration 205), équilibré et à empreinte, et un vieil **export « OGEC »** d'encaissements en CSV (`domain-eleve/src/facturation_ogec.rs`). Côté Next.js [F] : une **ébauche** (« pas de plan comptable, pas de partie double », `drizzle/0053_compta_etablissement.sql:1-2`) — dépenses, comptes caisse/banque, mouvements de trésorerie, export CSV « cabinet ». D-035 : on ne la reprend pas ; ses données sont **vides probables** (T0, M15).
2. **L'export Rust a dix défauts** (KD-01 à KD-10, §2.3), dont deux **importants** : le **fichier exporté n'est pas conservé** alors que ses pièces sont marquées « exportées » (un téléchargement perdu = des écritures qui ne ressortiront jamais) et les **rejets de prélèvement ne sont pas exportés** (la banque de la comptabilité ne correspond plus au relevé). Ils forment le **lot CG0** (correctifs, à faire avec le lot F9 du socle 24).
3. **La question structurante (§3) : intégrer un logiciel comptable existant ou tout refaire ?** Recommandation **[R]** : **ScolIA fait tout ce qui naît dans l'établissement** — budget, achats de la demande au paiement, fournisseurs, réception des factures, trésorerie, analytique, inventaire des biens, **et toutes les écritures en partie double**, numérotées, prouvées, reliées à leur pièce — puis **les remet au logiciel comptable de l'OGEC ou de son expert-comptable** (FEC et formats d'import), qui garde la **tenue légale** : grand livre officiel, révision, clôture, comptes annuels ANC 2018-06, liasse, rapport du commissaire aux comptes. Raisons : la valeur pour l'établissement est **en amont** (90 % des gestes : commander, réceptionner, payer, suivre un budget) ; la tenue légale est déjà externalisée dans la plupart des OGEC [H] ; refaire un logiciel comptable complet ajoute un risque (un bilan faux) sans gain d'usage. Une **balance et un grand livre de contrôle** sont quand même calculés dans ScolIA (CG9) ; la **tenue complète** (CG13, clôture et états) reste un lot **optionnel**, déclenché si un client le demande. Cela **précise D-025** sans le contredire frontalement, mais change son ambition : **soumis à Florian (Q-CG1)**, non appliqué par défaut.
4. **Thèse (§1)** : une écriture **naît d'un fait** d'un domaine (facture émise, encaissement, rejet, aide, bon à payer, paie, amortissement), par un **schéma comptable** déclaré, dans la **même transaction** que le fait ; elle n'est **jamais modifiée** une fois validée, seulement **contrepassée** ; chaque euro dit d'où il vient.
5. **Réglementaire (§6)** — points qui commandent le logiciel : plan comptable des associations **ANC 2018-06** (modifié par 2020-08) [V-sec] ; pièces comptables **10 ans** (C. com. L123-22) et 6 ans fiscaux (LPF L102 B) [V par 24 §5] ; **FEC** obligatoire pour les contribuables soumis à l'IS ou à l'IR en BIC/BNC/BA, **non établi** pour un OGEC non fiscalisé mais **format de fait** des cabinets [V-sec] ; **réception des factures électroniques** obligatoire au **1er septembre 2026** pour tout assujetti à la TVA, **même exonéré** (cas d'un OGEC), via une **plateforme agréée** [V-sec, FAQ DGFiP citée] ; **commissaire aux comptes** au-delà de **153 000 €** de subventions publiques en numéraire (C. com. L612-4, D612-5) [V-sec], le forfait d'externat étant de qualification discutée (§6.3) ; public : **M9.6-Op@le du 19 janvier 2026** (BO n° 7 du 12-2-2026) [V-sec], ScolIA **n'est pas** le comptable public ; **commande publique** (EPLE) : dispense de procédure à **60 000 € HT** (fournitures, services) depuis le 1er avril 2026 et **100 000 € HT** (travaux) [V-sec, décret du 29-12-2025], interdiction du fractionnement.
6. **Ce que ScolIA apporte de plus [H]** : le budget **vivant** (voté, engagé, réalisé, disponible en temps réel, par pôle, discipline, séjour) ; la demande d'achat d'un professeur **en trente secondes** sur son téléphone, validée par son responsable, sans papier ; le **rapprochement à trois voies** commande-réception-facture fait par la machine ; la **fraude au changement d'IBAN** bloquée par construction ; la facture électronique reçue **déjà rapprochée** de sa commande ; la trésorerie **prévue** à 6 mois avec les encaissements des familles (24–25) et la paie (49) ; un **dossier de révision** prêt pour l'expert-comptable ; une IA qui **lit les factures, propose l'imputation, repère les doublons**, et ne paie rien.
7. **Le plan** : **199 actions** (CGA-100 à CGA-298, CGA-299 réservé), **42 règles** (R-CG1 à R-CG42), **49 cas limites** (CCG-01 à CCG-60, numérotation par blocs), **15 lots** (CG0 à CG14), migrations **2960–2979**. **Parcours minimal (doublon 2027-2028)** : CG0, CG1 (moteur d'écritures), CG2 (budget), CG3–CG6 (demandes, commandes, fournisseurs, factures et bon à payer), CG7 (paiements et trésorerie), CG8 (export). Pendant le doublon, la comptabilité officielle reste dans l'outil actuel de l'OGEC : ScolIA **produit** et **exporte**, l'expert-comptable **compare** (R-CG35).
8. **Ajout personas (§33)** : quatre bloquants tranchés par défaut — (A) pendant le doublon, achats et budget en **réel** avec des journaux propres (`HA-S`, `BQ-S`), ventes dans Charlemagne, réalisé des produits lu dans la balance du cabinet ; (B) export **centralisé ou détaillé** et correspondance de comptes avec le cabinet ; (C) chaîne de suppléance des valideurs et mode « structure réduite » avec revue a posteriori par le trésorier ; (D) **collision d'IBAN** détectée par empreinte entre fournisseurs, salariés et payeurs ; plus le marché de denrées de la cuisine, les devises, la déclaration d'intérêts ; 30 actions (CGA-300 à CGA-329), 12 règles, 18 cas limites, lot CG8b.
9. **Soumis à Florian** : Q-CG1 (périmètre : production d'écritures et export, ou tenue légale complète — contredit l'ambition de D-025) ; logiciel comptable et expert-comptable de l'OGEC (formats, plan de comptes) ; plateforme agréée de réception des factures (contrat, coût) ; banque (virements fournisseurs, relevés) ; usage en réel pendant le doublon (achats) ; durées de conservation (DPO).

---

## 1. Thèse : chaque euro dit d'où il vient

1. **La comptabilité ne se saisit pas, elle se constate.** Neuf écritures sur dix d'un établissement scolaire viennent d'un fait déjà enregistré ailleurs : une facture aux familles (24), un encaissement ou un rejet (25), une aide (26), un bon à payer fournisseur (11), une paie (49), une dotation aux amortissements (15). Ces faits produisent leurs écritures **par schéma comptable déclaré**, jamais par une ressaisie. La saisie manuelle (opérations diverses) reste possible, avec pièce obligatoire et double contrôle au-delà d'un seuil.
2. **Une écriture validée est immuable.** On ne la modifie pas, on ne la supprime pas : on la **contrepasse** par une écriture datée du jour, reliée à l'originale (D-027 E, même principe que les avoirs). La période close ne reçoit plus rien.
3. **Une écriture porte toujours sa preuve.** Pièce (facture, relevé, bulletin, décision) stockée au socle 7, empreinte, lien en un clic : « d'où vient cette ligne ? » est une question à une seule réponse.
4. **Le budget précède la dépense.** Une commande **engage** un crédit avant d'être passée ; le disponible est connu au moment où l'on demande, pas en fin d'année. Le dépassement n'est pas interdit, il est **décidé** (par qui, pourquoi, tracé).
5. **Séparer les tâches.** Celui qui demande n'est pas celui qui valide ; celui qui crée un fournisseur ou modifie son IBAN n'est pas celui qui paie ; celui qui saisit n'est pas celui qui valide l'écriture au-delà d'un seuil. Le logiciel l'impose et l'explique.
6. **L'établissement produit, le comptable tient.** ScolIA est l'atelier où naissent les faits et leurs écritures ; la tenue légale (révision, clôture, comptes annuels, liasse) se fait dans le logiciel de l'expert-comptable, alimenté par un export **complet, continu, à empreinte**, jamais en double (§3).
7. **Public et privé, un modèle, deux régimes.** Dans le privé, l'OGEC est une association : partie double, ANC 2018-06. Dans le public, l'EPLE a un ordonnateur et un **agent comptable** sur Op@le : ScolIA prépare (demandes, engagements internes, régie, pièces), Op@le comptabilise. Rien du privé n'est codé en dur dans le public, et inversement (R-CG3).
8. **L'IA lit et propose ; elle ne valide, ne paie, ni n'écrit une écriture définitive.** Lecture des factures, imputation proposée, anomalies, prévisions : toujours relues par un humain ; les écritures et les paiements sont des commandes N3 (`IA_COEUR.md` §6.2).

---

## 2. L'existant

### 2.1 ENT Rust [F]

| Élément | Où | Ce qu'il fait |
|---|---|---|
| Export d'écritures `scolia_ecritures_v1` | `ent-db/src/finances_sepa.rs:1388-1615` ; rendu `finances_calcul.rs:323-363` ; migration `205_fin_export_comptable.sql` | Sur une période passée (≤ 366 j), lit les factures et avoirs émis et les encaissements non encore exportés ; produit les écritures (411 auxiliaire / 706 ou compte de produit de l'article / 709 réductions / 441 bourses ; 512 / 411 pour les règlements) ; contrôle l'équilibre ; empreinte SHA-256 ; numéro d'export `EXPAAAA-NNNN` ; marque chaque pièce exportée (`ent_fin_export_piece`, unique par pièce) ; publie `FAC_EXPORT_COMPTABLE` |
| Paramètres comptables | `200_fin_parametres_tarifs.sql:29-31` | `compte_clients` 411000, `compte_banque` 512000, `journal_ventes` VT, journal de banque ; `ent_fin_article.compte_produit` |
| Export « OGEC » historique | `domain-eleve/src/facturation_ogec.rs` (141 l.), appelé par `ent-db/src/service.rs:19413-19418` | CSV `;` UTF-8 BOM des **encaissements** d'une plage : date, pièce, libellé, `foyer_id`, quittance, moyen, référence, montant, nature. « Pas un connecteur Sage » (en-tête du fichier) |
| Budget de séjour | socle 42 V0a (migrations 640–642, `voyages*.rs`) | budget et lignes d'un séjour ; analytique « voyage » prévue vers le 48 (42 §12.1, §12.6) |
| Droits | `ent-auth/src/actions.rs:100-103` | `facturation.gerer`, `facturation.lire` ; **aucun droit comptable propre** |
| Rôle | `ent-auth` `Role::Compta` | rôle « Compta » prédéfini (portée établissement) |

Aucune table de plan comptable, de journal, d'écriture persistée, de budget, de fournisseur, de commande, de facture fournisseur ni de trésorerie [F] (`grep` des migrations, 231 fichiers, le 2026-11-11).

### 2.2 Next.js (ébauche, D-035 précision 2) [F, lecture seule]

| Élément | Où | Constat |
|---|---|---|
| Schéma | `drizzle/0053_compta_etablissement.sql` (117 l.) | `tresorerie_compte` (caisse / banque), `depense` (date, libellé, **fournisseur en texte**, portée cantine/internat/voyage/scolarité/autre, montant `numeric(12,2)`, statut prévue/payée, `travel_id`), `mouvement_tresorerie` (entrée/sortie, liens encaissement/dépense/facture), `facture_echeance`. En-tête : « Pas de plan comptable. Pas de partie double. » |
| Logique | `app/lib/compta-etablissement-db.ts` (646 l.) | créer une dépense, la payer, créer des comptes par défaut, importer les encaissements dans le livre, export CSV « cabinet » |
| API | `app/api/compta/route.ts`, `app/api/compta/export/route.ts` | protégées par `requireModule("compta-etablissement")` seulement (accès au **module**, pas d'action fine) |
| Voyages | `app/api/travels/compta-sheet*`, `compta-apel-summary*` | fiche budget de séjour et synthèse annuelle pour l'APEL (**fiables**, reprises par le socle 42, NX du 42) |

**Ce qu'on retient de l'ébauche** (idées, pas code) : la **portée** d'une dépense (cantine, internat, voyage) = un axe analytique ; le lien dépense ↔ séjour ; le fichier pour le cabinet. **Ce qu'on ne reproduit pas** : fournisseur en texte libre, montants décimaux, absence de partie double, suppression en cascade depuis l'établissement (`ON DELETE cascade`, `0053` lignes 64-75), droit au niveau du module.

### 2.3 Défauts de l'existant Rust (lot CG0)

| # | Gravité | Défaut | Preuve [F] | Correctif |
|---|---|---|---|---|
| **KD-01** | **Important** | Le **contenu de l'export n'est pas conservé** : seule son empreinte l'est ; les pièces sont marquées exportées dans la même transaction. Un téléchargement interrompu, un fichier perdu chez le cabinet = des écritures qui ne ressortent plus (« aucune pièce à exporter ») | `205` : pas de colonne de contenu ; `finances_sepa.rs:1548-1584` | Le fichier rendu est un **objet immuable** du socle 7 (E5 : `cle_objet`, `sha256`, `taille`) relié à l'export ; re-téléchargement à l'identique ; un nouvel export d'une pièce exige une **annulation tracée** de l'export (R-CG14) |
| **KD-02** | **Important** | Les **rejets de prélèvement** ne sont pas exportés (le type `rejet` existe dans la contrainte de `ent_fin_export_piece` mais aucune requête ne lit `ent_fin_rejet`) : le règlement exporté reste en 512, la banque de la comptabilité dépasse le relevé | `205` l. 28 ; requêtes l. 1405-1440 | Schéma « rejet » : 411 (aux.) au débit / 512 au crédit, plus frais bancaires si facturés à l'OGEC (627) ; test de bout en bout encaissement → rejet → export |
| KD-03 | Moyenne | **Numéro d'écriture recommencé à 1 à chaque export** ; le FEC attend un `EcritureNum` unique et continu par journal sur l'exercice [V-sec] | `finances_sepa.rs:1441-1442` (`let mut num = 0`) | Numéro attribué **à la validation**, compteur par (entité, journal, exercice) (E3), persisté |
| KD-04 | Moyenne | Tous les règlements au **512** quel que soit le moyen ; chèques et espèces passent par 5112 (chèques à encaisser) et 53 (caisse) avant la remise en banque | l. 1517-1523 | Comptes par moyen (paramètres), écriture de **remise** au dépôt du bordereau (25) |
| KD-05 | Moyenne | **Compte auxiliaire** = « F » + 8 premiers caractères hexadécimaux de l'UUID du **foyer** : collision possible (32 bits), illisible pour le comptable, et par foyer alors que le débiteur est le **compte payeur** (24 §7, `ent_fin_compte`) | l. 1406-1407 | Numéro de tiers lisible par compteur (`C000123`), porté par le compte débiteur ; table de correspondance exportée |
| KD-06 | Moyenne | Dates en **UTC** (`emise_at::DATE`, `date_naive()`) : un encaissement à 0 h 30 tombe la veille (déjà FD-24 du 24) | l. 1409, 1443 | Dates en Europe/Paris (fuseau de l'établissement, migration 265) |
| KD-07 | Moyenne | L'export exige `facturation.gerer` : tout le secrétariat qui gère la facturation peut exporter ; pas de séparation | l. 1395-1396 | Droit `comptabilite.exporter` (§23.1) |
| KD-08 | Faible | « Période close » = fin antérieure à aujourd'hui, pas une **période clôturée** : une pièce datée dans la période (encaissement saisi avec une date passée) arrive après l'export | l. 1400-1402 | S'appuyer sur la clôture des périodes (24 F9) ; écriture tardive datée dans la première période ouverte |
| KD-09 | Faible | `CompteLib` faux : tout compte qui n'est ni 411 ni 512 est « Produits » (441 bourses, 709 réductions) ; `DateLet` toujours vide | `finances_calcul.rs:343-357` | Libellés tirés du plan de comptes ; date de lettrage |
| KD-10 | Faible | `ent_fin_export_piece` en `ON DELETE CASCADE` depuis l'export ; l'export historique « OGEC » expose `foyer_id` et ne marque rien (exportable sans limite, sans trace) | `205` l. 24 ; `facturation_ogec.rs:74` | `RESTRICT` ; export historique **retiré** au profit du FEC (après avis du cabinet), ou tracé et limité à `comptabilite.exporter` |

**KD-11 (Next.js, signalé, Next.js intouchable)** : `paie-etablissement-db.ts:228-247` crée un **agent de démonstration** (« Alice DEMO ») dans la table `personnel` réelle si l'établissement n'en a pas (socle 49 §2.2) ; à vérifier qu'aucun établissement réel n'en garde une trace avant la reprise.

### 2.4 Ce qu'il ne faut pas perdre

- Le **contrôle d'équilibre en base** et l'**unicité d'export d'une pièce** (205) : principes repris tels quels.
- La **fiche budget de séjour** et la **synthèse APEL** de Next.js (fiables) : elles restent au socle 42 ; le 48 leur fournit le **réalisé** (dépenses payées imputées au séjour).
- Le **fichier pour le cabinet** : l'expert-comptable de l'OGEC l'utilise peut-être déjà [H] : à remplacer par un format qu'il accepte (Q-CG3).

### 2.5 Verdict

On **garde** le moteur d'export du 24 en le **déplaçant** dans une brique `comptabilite` qui possède désormais les écritures (le 24 ne fait plus que les demander par schéma) ; on corrige KD-01 à KD-10 en CG0 ; on **construit** tout le reste. Rien n'est repris de Next.js.

---

## 3. Intégrer un logiciel spécialisé ou tout refaire ?

La mission demande de trancher, en le justifiant. D-025 (2026-11-06) a décidé que la comptabilité d'organisme est « construite dans l'ENT Rust » et fait partie de la cible « équivalent Charlemagne » (qui a un module Comptabilité, distinct, `Charlemagne Comptabilité` [V-sec : paquets logiciels référencés]). La conception doit donc dire **ce qui est construit** et **ce qui est remis à un autre logiciel**.

### 3.1 Les trois couches d'une comptabilité d'établissement

| Couche | Contenu | Qui la fait aujourd'hui [H] | Valeur pour l'établissement |
|---|---|---|---|
| **A. Gestion** (amont) | budget, demandes d'achat, devis, commandes, réceptions, factures fournisseurs, bons à payer, paiements, fournisseurs, caisse, trésorerie prévisionnelle, analytique, inventaire | tableurs, papier, mails, parfois le module Comptabilité de Charlemagne ou un logiciel de gestion | **très forte** : c'est là que se perdent le temps et l'argent (factures égarées, doublons, budget découvert en juin, IBAN frauduleux) |
| **B. Production d'écritures** | traduire chaque fait en écritures équilibrées, numérotées, à pièce | souvent ressaisie par le comptable ou le cabinet à partir de relevés et de pièces | forte : supprime une ressaisie, garantit l'exhaustivité |
| **C. Tenue légale** | grand livre officiel, révision, écritures d'inventaire, clôture, bilan, compte de résultat, annexe ANC 2018-06, liasse, approbation, publication (CAC) | **l'expert-comptable** dans son logiciel (Cegid/Quadra, Sage, ACD, EBP… [V-sec pour les formats d'import usuels]), ou le comptable salarié sur Charlemagne Comptabilité ou Sage | faible pour l'établissement (il la sous-traite), **risque élevé** si elle est fausse |

### 3.2 Les options

| Option | Description | Pour | Contre |
|---|---|---|---|
| **1. Tout dans ScolIA** (A + B + C) | logiciel comptable complet, états de synthèse, révision | parité totale avec Charlemagne ; un seul outil | des années de travail pour une couche que le client sous-traite ; responsabilité sur les comptes annuels ; l'expert-comptable **ne quittera pas son logiciel** (il tient 100 dossiers) ; il faudrait quand même exporter vers lui |
| **2. A + B dans ScolIA, C par export** | ScolIA produit des écritures complètes et prouvées ; FEC et formats d'import vers le logiciel du cabinet ; ScolIA calcule une balance et un grand livre **de contrôle** | la valeur est en A et B ; zéro ressaisie ; le cabinet garde son outil ; risque maîtrisé | pas de comptes annuels dans ScolIA ; dépendance à des formats d'import d'éditeurs (atténuée par le FEC, que tous les logiciels importent [V-sec]) |
| **3. Intégration d'un logiciel tiers** (connecteur API vers un logiciel comptable en ligne, ou embarquer un logiciel libre) | déléguer A ou B à un tiers | rapide | données d'élèves et de familles chez un tiers (minimisation, contrat) ; dépendance cloud refusée par défaut (CLAUDE.md) ; logiciels libres comptables (Dolibarr, Odoo communautaire…) : licences **GPL/AGPL/LGPL** à analyser, architecture étrangère (PHP, Python), **pas de reprise de code** sans analyse (CLAUDE.md) |

### 3.3 Recommandation [R] — option 2, avec une porte ouverte vers C

- **Construire A et B entièrement** (lots CG1 à CG12), au niveau « masterclass » : c'est là que ScolIA dépasse Charlemagne.
- **Remettre C au logiciel du cabinet** par un export **FEC** (format de référence, LPF A47 A-1) et un ou deux formats d'éditeur choisis avec l'expert-comptable de l'OGEC pilote (Q-CG3) ; **réimporter** ses écritures d'inventaire et de clôture (amortissements validés, provisions, PCA, à-nouveaux) pour que la balance de contrôle de ScolIA reste juste (CGA-232).
- **Calculer dans ScolIA** balance, grand livre et journal **de contrôle** (CG9), utiles au quotidien (« combien sur le 606 ? ») et pour vérifier que le cabinet a tout reçu (rapprochement de balances, CGA-235).
- **Garder un lot optionnel CG13 « tenue complète »** (écritures d'inventaire, clôture, à-nouveaux, états de synthèse ANC 2018-06) déclenché seulement si un client sans expert-comptable le demande ; il s'appuie sur le même moteur, rien n'est jeté.
- **Ne jamais embarquer** un logiciel comptable tiers ni envoyer de données d'élèves à un service comptable en ligne : l'export ne contient **aucun nom d'élève** (tiers « familles » par numéro, table de correspondance séparée et minimale, R-CG21).

**Pourquoi c'est conforme à l'esprit de D-025.** D-025 visait à ne pas laisser la comptabilité hors de l'ENT. Avec l'option 2, **tout ce qui se passe dans l'établissement** est dans ScolIA, et la comptabilité générale reçoit des écritures déjà faites. Ce qui n'est pas fait, c'est **le travail du cabinet**. Mais l'option 2 **réduit l'ambition** littérale de D-025 (« un équivalent du module Comptabilité de Charlemagne ») : **Q-CG1, soumise à Florian** (contredit une décision existante, D-031 ne s'applique pas). En attendant sa réponse, la conception et les lots CG0 à CG12 valent pour les deux réponses ; seul CG13 en dépend.

### 3.4 Public (EPLE)

Pas de débat : la comptabilité de l'EPLE est tenue par l'**agent comptable** dans **Op@le** (M9.6-Op@le du 19-1-2026) [V-sec]. ScolIA ne la remplace pas (R-CG3). Il outille ce qu'Op@le ne fait pas ou fait mal pour les utilisateurs de l'établissement [H] : demandes d'achat des enseignants, suivi des crédits par discipline, préparation des engagements, régie (24 F13), pièces justificatives classées, prévision. Les **interfaces d'import d'Op@le** ne sont pas publiques [NV] : par défaut, exports CSV pour le gestionnaire (CG12), à affiner avec une académie après D-029 (Q-CG9).

---

## 4. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Comptable / économe de l'OGEC** (salarié, souvent seul) | saisit ou fait saisir, paie les fournisseurs, rapproche la banque, prépare le budget, tient la caisse, prépare la clôture avec le cabinet | ressaisir factures et relevés ; chercher une facture ; relancer les professeurs pour un bon de livraison ; pointer la banque ; trois outils (Charlemagne, Excel, banque en ligne) | payer deux fois ; payer un faux fournisseur ; découvrir un trou de trésorerie ; être seul à savoir | une **file de travail** : « 6 factures reçues dont 4 rapprochées, 2 à valider ; 3 bons à payer ; banque pointée à 97 % ; 1 alerte doublon » ; un export qui part sans retouche au cabinet | quotidienne |
| **Chef d'établissement** (ordonnateur de fait dans le privé, ordonnateur légal dans le public) | arbitre le budget, valide les grosses dépenses, signe | demander « où en est-on ? » et attendre un tableau ; signer des piles de bons | un dépassement découvert tard ; une dépense engagée sans lui | un tableau **voté / engagé / payé / disponible** par pôle ; valider sur mobile avec la pièce sous les yeux ; une prévision de fin d'année | hebdomadaire |
| **Président et trésorier de l'OGEC** (bénévoles) | votent le budget et les comptes ; signent des chèques ou valident des virements | chiffres reçus tard, en PDF, différents de ceux du comptable | une fraude ; un déficit ; la responsabilité personnelle | un accès **en lecture** clair ; la double validation des virements au-delà d'un seuil ; l'historique du budget | mensuelle |
| **Expert-comptable / commissaire aux comptes** | tient ou révise les comptes, certifie | pièces manquantes ; écritures sans origine ; relevés non pointés ; balances qui ne concordent pas | un export incomplet ou modifié | FEC continu, empreinte, chaque écriture avec sa pièce **téléchargeable** ; dossier de révision (états de rapprochement, listes d'immobilisations, charges à payer) | mensuelle à annuelle |
| **Professeur / coordonnateur de discipline** | demande du matériel, des manuels, une sortie | formulaire papier, ne sait pas ce qui reste ; commande perdue | qu'on lui reproche une dépense | « il reste 412 € sur les SVT » ; demande en 30 s sur téléphone avec photo du devis ; suivi « commandé, livré » | quelques fois par an |
| **Responsable de pôle** (directeur adjoint, CPE pour la vie scolaire, chef de travaux, documentaliste, chef cuisinier) | valide les demandes de son pôle, gère son enveloppe | ne voit pas les engagements des autres membres | dépasser sans le savoir | une enveloppe à jour, des validations groupées | hebdomadaire |
| **Secrétariat / accueil** | reçoit des colis, des factures papier, des fournisseurs au téléphone | « à qui est ce colis ? » ; factures papier à scanner | perdre une facture | **réception** en un geste (scan du bon de livraison, rapprochement de la commande) ; facture papier photographiée et classée | quotidienne |
| **Agent d'entretien / chef cuisinier** | commande des denrées, des produits, des réparations | commandes par téléphone non tracées | rupture ; facture contestée | commandes récurrentes, réception avec écarts (quantité, qualité), traçabilité des denrées (lien socle 40) | quotidienne |
| **Organisateur de séjour** (socle 42) | engage des dépenses de séjour | ne sait pas ce qui est payé | budget dépassé | le **réalisé** du séjour en direct | ponctuelle |
| **Fournisseur** | livre, facture, se fait payer | relances, factures rejetées pour une référence manquante | payé en retard | une référence de commande claire, une date de paiement annoncée | ponctuelle |
| **Agent comptable d'EPLE** (public) | tient Op@le, paie, recouvre | pièces incomplètes envoyées par les services | une dépense irrégulière | des pièces complètes et des demandes conformes avant engagement | quotidienne |
| **Gestionnaire d'EPLE / secrétaire général** (public) | prépare le budget, engage dans Op@le, suit les crédits | les demandes des professeurs par mail | dépasser un chapitre ; fractionner sans le vouloir | demandes groupées par fournisseur et par nature, **compteur de seuils** de la commande publique (R-CG32) | quotidienne |
| **Conseil d'administration de l'OGEC / de l'EPLE** | vote budget et décisions modificatives | documents illisibles | voter sans comprendre | une présentation par grands postes, comparée à N-1, en PDF propre | 2 à 4 fois par an |
| **Familles** | (indirect) | — | que leur argent serve mal | (rien de nominatif ici) ; transparence possible du budget d'un séjour (42) | — |
| **Élèves** | (indirect ; éco-délégués, foyer socio-éducatif) | — | — | budget d'un projet d'élèves lisible (lien 46) | ponctuelle |

**Ce que montrent ces portraits [R].** (1) Le besoin dominant est **amont** (demander, commander, réceptionner, payer) et **budgétaire** (savoir ce qui reste) ; la tenue légale vient après. (2) Le risque principal est **la fraude et le double paiement**, pas le calcul. (3) Le comptable est **souvent seul** : la file de travail, la continuité (absence, départ) et la documentation de ses gestes valent autant que les fonctions. (4) Les bénévoles de l'OGEC ont besoin de **lire** sans pouvoir modifier.

---

## 5. L'année vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui se passe | Ce que fait ScolIA |
|---|---|---|
| Janvier–mars | préparation du budget N+1 (l'exercice de l'OGEC court souvent de septembre à août [H]) ; simulation des tarifs (24) | budget en **versions** (proposé, voté), pré-rempli par le réalisé N-1 et l'engagé ; recettes prévues lues au 24 (simulateur) et charges de personnel au 49 (masse salariale prévue) |
| Avril–juin | vote par le CA de l'OGEC ; commandes de manuels et de fournitures pour la rentrée ; séjours de fin d'année | budget **voté** figé (version), enveloppes ouvertes au 1er septembre ; commandes anticipées sur N+1 (engagement sur le budget de l'exercice suivant, CGA-152) |
| Juillet–août | travaux d'été ; factures de fin d'exercice ; inventaire | charges à payer (réceptions sans facture), produits constatés d'avance (24), inventaire des biens (15), **arrêté** et export définitif |
| Septembre | rentrée : facturation des familles (24), afflux de factures fournisseurs ; clôture N avec le cabinet | file de factures, rapprochements, à-nouveaux reçus du cabinet (CGA-232) |
| Octobre–décembre | décision modificative éventuelle ; subventions des collectivités (investissement) ; taxe sur les salaires (49) | budget rectificatif (version) ; suivi des subventions et fonds dédiés |
| Chaque mois | rapprochement bancaire, paie (49), clôture de période des ventes (24) | période close ; export ; tableau de bord |

---

## 6. Ce qui est réglementaire (vérifié le 2026-11-11)

### 6.1 Privé : l'OGEC, association loi 1901

| # | Règle | Référence | Statut | Conséquence pour le logiciel |
|---|---|---|---|---|
| RC-01 | Comptes annuels des personnes morales de droit privé à but non lucratif : **règlement ANC n° 2018-06** du 5-12-2018 (homologué, JO du 30-12-2018), abrogeant le CRC 99-01, **modifié par le règlement 2020-08** ; fonds propres avec et sans droit de reprise, **fonds dédiés** (y compris « sur investissements »), contributions volontaires en nature | ANC 2018-06, 2020-08 | [V-sec] (Associathèque, LPA, Fiducial ; texte ANC non lu directement) | plan de comptes **paramétrable** pré-chargé ANC 2018-06 (classes 1 à 8), comptes 19 (fonds dédiés) et 86/87 (contributions en nature) présents ; aucun numéro codé en dur hors schémas par défaut (R-CG6) |
| RC-02 | Tenue : livre-journal, grand livre, inventaire ; écritures chronologiques, pièce justificative, **intangibilité** après validation | C. com. L123-12 s. et R123-172 s. (par analogie pour une association tenue de produire des comptes) | [NV] (texte non relu ; principe constant) | validation irréversible, contrepassation seule (R-CG10, R-CG11) |
| RC-03 | **Conservation** : pièces et documents comptables **10 ans** (C. com. L123-22) ; 6 ans au titre fiscal (LPF L102 B) | | [V] (lu par la conception du 24, §5) | 10 ans après la clôture de l'exercice (§23.4) |
| RC-04 | **FEC** : obligation de remise lors d'une vérification pour les entreprises tenant une comptabilité informatisée soumises à l'IS ou à l'IR (BIC, BNC, BA) ; format A47 A-1 (18 champs, numérotation continue, date de validation) ; amende CGI 1729 D | LPF L47 A-I, A47 A-1 ; BOI-CF-IOR-60-40 | [V-sec] ; **application à un OGEC non fiscalisé non établie** (aucune source explicite) | FEC produit quand même : **c'est le format d'échange** que tout logiciel comptable importe ; contrôles de conformité du format (CGA-221) |
| RC-05 | **Commissaire aux comptes** et publication des comptes (site de la DILA) pour l'association recevant **153 000 €** ou plus de subventions publiques en numéraire sur l'exercice (État, collectivités, établissements publics ; hors fonds européens) | C. com. L612-4, D612-5 | [V-sec] (« au moins » ou « plus de » : à lire dans D612-5) | compteur des **subventions publiques** de l'exercice et alerte au seuil (CGA-262) ; la qualification du forfait d'externat est au §6.3 |
| RC-06 | **TVA** : enseignement exonéré (CGI 261-4-4°) ; l'OGEC est assujetti mais réalise surtout des opérations exonérées ; opérations taxables possibles (locations, ventes) | CGI 261-4-4° | [V partiel] (24 §5.3) | régime de TVA par article (24) et par ligne de facture fournisseur ; **TVA non récupérable** incluse dans la charge par défaut ; prorata éventuel paramétré avec le cabinet (Q-CG6) |
| RC-07 | **Réception des factures électroniques** obligatoire au **1er septembre 2026** pour toutes les entreprises assujetties, **y compris celles qui ne réalisent que des opérations exonérées** (exemple de la FAQ DGFiP : un médecin) ; par une **plateforme agréée** ; émission obligatoire au 1-9-2026 (grandes et ETI) ou 1-9-2027 (PME, micro) — hors champ pour les opérations exonérées ; non assujetti : rien | CGI 289 bis ; loi de finances 2024 art. 91 | [V-sec] (sources concordantes citant la FAQ DGFiP ; FAQ non lue directement) | l'OGEC doit avoir **choisi une plateforme agréée** ; ScolIA **n'est pas** une plateforme agréée (agrément de l'administration, lourd) et s'y **connecte** pour récupérer les factures reçues (§13, Q-CG4) ; **à confirmer d'urgence avec l'expert-comptable : l'échéance est passée** (CCG-60) |
| RC-08 | **Délais de paiement** entre professionnels : 60 jours date de facture ou 45 jours fin de mois ; pénalités et indemnité forfaitaire de 40 € | C. com. L441-10 | [NV] (non relu ce jour) | échéance calculée par fournisseur ; alerte avant dépassement ; indemnité proposée si un fournisseur la facture |
| RC-09 | **Taxe sur les salaires** due par les employeurs non assujettis à la TVA sur au moins 90 % de leur chiffre d'affaires ; **abattement des associations** (CGI 1679 A) : **24 256 €** pour les salaires versés en 2026 | CGI 231, 1679 A ; formulaire 2502-SD 2026 | [V] (formulaire officiel impots.gouv.fr cité) | calculée par le logiciel de paie (49) ; le 48 en reçoit l'écriture (631) et la **prévoit** en trésorerie |
| RC-10 | **Contribution des familles** limitée (enseignement religieux, immobilier et matériel, redevances de demi-pension et d'internat) | C. éduc. R442-48 | [V] (24 §5.1) | l'analytique suit la **nature légale** des produits (24) pour justifier l'emploi des contributions (CGA-244) |
| RC-11 | **Forfait d'externat** (part « personnels » versée par l'État, part « matériel » par la commune, le département ou la région) : couvre notamment les personnels non enseignants | C. éduc. L442-9 | [V-sec] (questions parlementaires, enseignement-catholique.fr) | produit suivi par financeur et par part ; qualification comptable (subvention ou contribution obligatoire) **à trancher avec l'expert-comptable** (Q-CG7), elle change le compteur RC-05 |
| RC-12 | **Subventions d'investissement** des collectivités au privé : plafonnées (loi Falloux : 10 % des dépenses annuelles hors contrat de l'établissement pour le second degré général, C. éduc. L151-4) | | [NV] | suivi des subventions d'investissement, quote-part virée au résultat au rythme de l'amortissement (CG10) |

### 6.2 Public : l'EPLE

| # | Règle | Référence | Statut | Conséquence |
|---|---|---|---|---|
| RC-20 | Cadre budgétaire et comptable des EPLE connectés à Op@le : **instruction codificatrice M9.6-Op@le du 19 janvier 2026**, BO n° 7 du 12-2-2026, applicable à partir du compte financier 2026 ; les EPLE encore sur GFC restent sous la M9.6 du 27-4-2015 ; généralisation d'Op@le engagée depuis janvier 2024, dernière vague annoncée en 2026 | BO n° 7 du 12-2-2026 (MENF2600269J) ; IH2EF | [V-sec] (page du BO repérée, non relue en entier) | ScolIA **ne tient pas** la comptabilité publique (R-CG3) ; vocabulaire public dans les écrans (ordonnateur, engagement, service fait, mandatement… [NV]) |
| RC-21 | Budget voté par le **conseil d'administration** ; actes budgétaires transmis | C. éduc. R421-20 (compétences du CA) | [V] (42 §5.3) ; détail M9.6 [NV] | budget en versions avec date de vote (CG2) ; ScolIA prépare, Op@le exécute |
| RC-22 | **Régies** d'avances et de recettes : décret n° 2020-542 du 7-5-2020, arrêté du 13-8-2020 | | [V] (24 §5.10) | régie de recettes au 24 (F13) ; régie d'avances (petites dépenses d'un séjour, CGA-170) au 48 |
| RC-23 | **Commande publique** : dispense de publicité et de mise en concurrence en dessous de **60 000 € HT** pour les fournitures et services (depuis le 1-4-2026, 40 000 € avant) et **100 000 € HT** pour les travaux ; au-delà, procédure adaptée jusqu'aux seuils européens (216 000 € HT pour les fournitures et services des pouvoirs adjudicateurs non centraux) ; **interdiction du fractionnement** ; besoin apprécié globalement | CCP R2122-8 ; décret du 29-12-2025 (numéro 2025-1368 ou 2025-1386 : discordance des sources) | [V-sec] | **compteur de besoin** par famille homogène et par exercice ; alerte avant seuil ; aucune décision automatique (R-CG32) ; seuils **paramètres de plateforme** datés, jamais codés (R-CG33) |
| RC-24 | Un OGEC n'est en principe **pas** soumis au code de la commande publique ; un organisme de droit privé financé majoritairement par des fonds publics **peut** être pouvoir adjudicateur (CCP L1211-1) | | [NV] (position pour l'enseignement privé sous contrat non trouvée) | compteur de seuils **activable** pour le privé (S-4812), désactivé par défaut ; **question au juriste** (Q-CG8) |

### 6.3 Points ouverts du réglementaire [NV]

- Qualification comptable du **forfait d'externat** (subvention publique au sens de L612-4, ou contribution obligatoire) : les sources se contredisent (Q-CG7).
- **FEC** pour une association non fiscalisée (RC-04) : obligation non établie ; on le produit de toute façon.
- **Nomenclature comptable de l'enseignement catholique** : la FNOGEC et les UROGEC diffusent des recommandations de plan de comptes et d'analytique sectorielle [V-sec : documents FNOGEC cités par Aplim et un support de formation de 2016 « comptabilité analytique et analyse sectorielle »] ; non publics ou non lus : à obtenir de l'expert-comptable ou de l'UROGEC (Q-CG3).
- Format et **API des plateformes agréées** : normes Factur-X, UBL, CII [V-sec] ; raccordement à définir avec la plateforme choisie (Q-CG4).
- Interfaces d'**Op@le** (Q-CG9).

---

## 7. Branchements sur le cœur

### 7.1 Qui possède quoi

| Donnée | Propriétaire | Le 48 |
|---|---|---|
| Factures aux familles, avoirs, lignes, articles et leur compte de produit | 24 | reçoit les **faits** et produit les écritures par schéma ; ne recalcule jamais une facture (24 §6.5) |
| Encaissements, remises, prélèvements, rejets, remboursements | 25 | idem |
| Aides affectées, bourses, fonds sociaux (décision anonymisée) | 26 | idem ; jamais le motif |
| Paie : bulletins, journal de paie, charges | 49 (ou logiciel de paie, §3 du 49) | reçoit le **journal de paie** (import ou port) et produit les écritures de salaires et de charges ; jamais un salaire individuel visible hors droit (R-CG22) |
| Séjours, budget de séjour | 42 | fournit le **réalisé** (dépenses imputées) et l'analytique |
| Biens, inventaire matériel, maintenance | 44 | partage la **fiche du bien** : le 44 possède l'objet physique (localisation, état, prêt), le 48 sa **valeur** (acquisition, amortissement, sortie) |
| Restauration (commandes de denrées) | 40 | les commandes de denrées passent par les **achats du 48** ; le 40 en lit le statut |
| Agents, fonctions, pôles (responsables) | 6 | lit qui valide quoi (responsable de pôle = fonction datée) |
| Organismes (fournisseurs ?) | 4 (`ent_organisme`) | **non** : un fournisseur est un **tiers comptable** du 48 (données bancaires, SIRET, conditions), distinct d'un organisme payeur ou partenaire du 4 ; lien optionnel quand le même organisme est les deux (ex. mairie) |
| Pièces, PDF, factures reçues | 7 | dépose et lit ; le 7 garde l'objet immuable et son empreinte |
| Instances (vote du budget) | 46 | lit la délibération (date, référence) ; à défaut, saisie de la date de vote |

### 7.2 Ports consommés

| Port | Socle | Usage |
|---|---|---|
| `facturation::faits_comptables(periode)` (remplace `facturation::ecritures`) | 24 | factures, avoirs, lignes par compte de produit, axe analytique |
| `paiements::faits_comptables(periode)` | 25 | encaissements par moyen, remises, rejets, remboursements, frais |
| `aides::faits_comptables(periode)` | 26 | aides affectées et reversements |
| `paie::journal(periode)` | 49 | totaux par rubrique comptable et par analytique (jamais par salarié hors droit) |
| `sorties::budget(sejour)` | 42 | budget prévu, pour le comparer au réalisé |
| `personnels::responsables(pole, date)` | 6 | circuit de validation |
| `documents::deposer / lire` | 7 | pièces |
| `notifications::avis` | 8 | avis de validation, d'anomalie |

### 7.3 Ports exposés

| Port | Consommateurs | Contenu |
|---|---|---|
| `comptabilite::comptabiliser(fait, schema)` | 24, 25, 26, 49, 15 interne | commande idempotente (clé du fait) ; écrit l'écriture en **brouillard** dans la transaction du fait |
| `budget::disponible(ligne, date)` | 42, 40, 44, 6 (missions), 30 (IA) | voté, engagé, réalisé, disponible |
| `budget::engager(source, montant)` / `liberer` | 42 (séjour), 40 (denrées), 44 (maintenance) | engagement idempotent |
| `achats::statut_commande(id)` | 40, 42, 44, demandeur | statut sans montant pour qui n'a pas le droit |
| `comptabilite::realise(axe, valeur, periode)` | 42 (réalisé du séjour), 47 (pilotage) | montants agrégés |
| `immobilisations::valeur(bien)` | 44 | valeur nette, pour la sortie d'inventaire |

### 7.4 Répercussions automatiques

- Une **facture aux familles émise** (24) → écriture de ventes en brouillard, validée à la clôture de la période du 24.
- Un **rejet** (25) → écriture de rejet (corrige KD-02).
- Un **séjour annulé** (42) → les engagements non commandés sont **libérés** ; les commandes passées restent (à annuler chez le fournisseur, tâche).
- Un **agent qui part** (6) → ses demandes d'achat en cours sont **réaffectées** à son responsable ; ses délégations de validation tombent à la date de fin.
- Une **fonction de responsable de pôle** qui change (6) → le circuit des demandes non validées suit le nouveau responsable ; les validations passées restent signées par l'ancien.
- Un **bien sorti de l'inventaire** (44 : perdu, cassé, donné) → proposition d'écriture de sortie d'immobilisation (valeur nette).
- Un **fournisseur** dont l'IBAN change → tous les paiements en préparation vers lui sont **suspendus** jusqu'à vérification (R-CG26).

---

## 8. Concepts et vocabulaire [R]

| Terme | Définition | Table |
|---|---|---|
| **Entité comptable** | la personne morale qui tient les comptes (OGEC, association, EPLE) ; reprend `ent_fin_entite_facturation` du 24 (§9.6), qui devient `ent_cg_entite` | `ent_cg_entite` |
| **Exercice** | période comptable de l'entité (souvent 1-9 → 31-8 dans le privé [H], année civile dans le public) ; états : ouvert, en clôture, clos | `ent_cg_exercice` |
| **Période** | mois de l'exercice ; close = plus aucune écriture datée dedans | `ent_cg_periode` |
| **Plan de comptes** | comptes généraux de l'entité (pré-chargés ANC 2018-06 ; M9.6 non pré-chargée : public sans comptabilité générale dans ScolIA) | `ent_cg_compte` |
| **Tiers** | fournisseur, client non famille (location de salle), salarié (49, compte 421 global par défaut), organisme financeur ; numéro lisible | `ent_cg_tiers` |
| **Journal** | ventes (VT), achats (HA), banque (BQ1…), caisse (CA), opérations diverses (OD), paie (PA), à-nouveaux (AN) | `ent_cg_journal` |
| **Écriture** | ensemble équilibré de lignes, une date, une pièce, un journal ; états : brouillard → validée (numérotée) ; contrepassée | `ent_cg_ecriture`, `ent_cg_ecriture_ligne` |
| **Schéma comptable** | règle déclarée qui traduit un type de fait en écriture (comptes, sens, analytique) ; versionné | `ent_cg_schema`, `ent_cg_schema_ligne` |
| **Axe analytique** | dimension de répartition : activité (enseignement, restauration, internat, pastorale, séjours, périscolaire…), pôle, discipline, séjour, projet, site, niveau | `ent_cg_axe`, `ent_cg_section` |
| **Budget** | prévision par ligne budgétaire (compte ou regroupement × section), en **versions** (proposé, voté, rectificatif) | `ent_cg_budget`, `ent_cg_budget_version`, `ent_cg_budget_ligne` |
| **Enveloppe** | part de budget confiée à un responsable (pôle, discipline, séjour) avec un plafond | `ent_cg_enveloppe` |
| **Engagement** | réservation d'un montant sur une ligne budgétaire par une commande ou une source (séjour, mission) | `ent_cg_engagement` |
| **Demande d'achat** | besoin exprimé par un personnel, validé selon un circuit | `ent_cg_demande`, `ent_cg_demande_ligne` |
| **Commande** | engagement envers un fournisseur, numérotée | `ent_cg_commande`, `ent_cg_commande_ligne` |
| **Réception** | constat de livraison ou de service fait, total ou partiel, avec écarts | `ent_cg_reception`, `ent_cg_reception_ligne` |
| **Facture fournisseur** | pièce reçue (électronique, PDF, papier scanné) ; états : reçue → rapprochée → validée (bon à payer) → payée ; litige | `ent_cg_facture_four`, `…_ligne` |
| **Bon à payer** | décision humaine de payer une facture rapprochée | état de la facture + `ent_cg_validation` |
| **Paiement** | virement (lot SEPA), chèque, prélèvement fournisseur, carte, espèces de caisse | `ent_cg_paiement`, `ent_cg_lot_virement` |
| **Compte de trésorerie** | banque, caisse, régie, carte | `ent_cg_tresorerie` |
| **Relevé** | relevé bancaire importé (CAMT.053, CFONB 120, OFX ou CSV [NV sur la banque]) | `ent_cg_releve`, `ent_cg_releve_ligne` |
| **Rapprochement** | lien entre une ligne de relevé et une ou des écritures | `ent_cg_rapprochement` |
| **Bien (immobilisation)** | élément d'actif suivi en valeur, lié à la fiche du socle 44 | `ent_cg_immobilisation` |
| **Export** | lot d'écritures validées remis au logiciel comptable, avec son fichier (objet) | `ent_cg_export` (remplace `ent_fin_export`) |

**Distinctions qui manquent aujourd'hui** : fournisseur ≠ organisme (§7.1) ; engagement ≠ dépense ; réception ≠ facture ; brouillard ≠ validée ; budget voté ≠ budget de travail.

---

## 9. Le moteur d'écritures (cœur du socle)

### 9.1 Naissance d'une écriture

1. Un domaine produit un **fait** (facture émise, encaissement, rejet, bon à payer, paiement, journal de paie, dotation).
2. Dans **la même transaction**, il appelle `comptabilite::comptabiliser(fait, schema)` (clé d'idempotence = type + identifiant du fait + version) : une écriture en **brouillard** est créée selon le **schéma** actif à la date du fait.
3. Le brouillard est visible, modifiable **seulement** par recalcul (changement de paramètre avant validation) ; jamais à la main pour une écriture d'origine automatique (R-CG9).
4. **Validation** : à la clôture de la période du domaine (24 F9), ou par le comptable (lot, N3) ; elle attribue le **numéro continu** du journal sur l'exercice et la date de validation (FEC) ; l'écriture devient **immuable** (R-CG10).
5. Une erreur se corrige par **contrepassation** (écriture inverse, datée dans la première période ouverte, liée) puis écriture juste (R-CG11).

### 9.2 Schémas comptables par défaut (privé, ANC 2018-06) [R, à valider par l'expert-comptable]

| Fait | Débit | Crédit | Analytique |
|---|---|---|---|
| Facture aux familles | 411 (tiers compte payeur) | 706x (compte de produit de l'article, par nature légale : contribution, demi-pension, internat, séjour…) ; 709 réductions (au débit) | activité de l'article ; séjour |
| Avoir | inverse | | |
| Aide affectée (bourse avec procuration) | 4417 / 441 (organisme) [NV] | 411 | — |
| Encaissement chèque | 5112 | 411 | — |
| Remise de chèques en banque | 512 | 5112 | — |
| Encaissement espèces | 53 | 411 | — |
| Prélèvement exécuté / virement reçu / carte | 512 (ou 58 virements internes, selon la date de valeur) | 411 | — |
| Rejet de prélèvement | 411 | 512 ; frais bancaires 627 / 512 | — |
| Remboursement à une famille | 411 | 512 | — |
| Facture fournisseur (charge) | 60x / 61x / 62x (selon la catégorie d'achat) + 44566 si TVA récupérable | 401 (tiers fournisseur) | section(s) de la demande |
| Facture fournisseur (immobilisation) | 21x (ou 23x en cours) | 404 | |
| Paiement fournisseur | 401 | 512 | |
| Réception sans facture à la clôture | charge | 408 (factures non parvenues) | |
| Journal de paie (49) | 641, 645, 647… | 421, 431, 437, 4421 (prélèvement à la source)… | répartition analytique des salaires (49) |
| Taxe sur les salaires | 631 | 4471 [NV] | |
| Dotation aux amortissements | 6811 | 28x | section du bien |
| Subvention d'investissement reçue | 512 | 131 | |
| Quote-part de subvention virée au résultat | 139 | 777 | |
| Forfait d'externat | 512 / 44x | 74x (selon Q-CG7) | financeur, part |
| Régie d'avances : avance consentie | 543 [NV] | 512 | |

### 9.3 Garanties

- **Équilibre** contrôlé en base (contrainte différée sur l'écriture validée, comme en 205).
- **Continuité** : aucun trou dans la numérotation d'un journal ; un contrôle nocturne compare le compteur et les numéros (CGA-226).
- **Exhaustivité** : chaque fait comptabilisable a son écriture ou une anomalie « fait sans écriture » (contrôle nocturne, CGA-227).
- **Chaînage** : chaque écriture validée porte l'empreinte de la précédente du même journal (empreinte chaînée, à la manière d'un journal de caisse sécurisé) ; une modification en base est détectable (CGA-228).
- **Fuseau** : dates comptables en Europe/Paris (KD-06).

### 9.4 Saisie manuelle (opérations diverses)

Réservée au comptable (`comptabilite.saisir`), pièce obligatoire (sauf écritures d'à-nouveaux), libellé, comptes du plan actif, contrôle d'équilibre en direct ; **double validation** au-delà d'un seuil (S-4803) ou sur certains comptes (fonds propres, 1x ; 4x de tiers sensibles) ; modèles d'écritures récurrentes (abonnements, loyers) ; **jamais** sur un compte 411 d'une famille (passe par un avoir du 24 ou une opération du 25, R-CG12).

---

## 10. Budget

### 10.1 Construction [R]

- **Structure** : ligne budgétaire = (compte de charge ou de produit, ou regroupement « poste ») × section analytique principale (activité ou pôle) ; regroupements de présentation pour le CA (grands postes : personnel, fonctionnement pédagogique, restauration, immobilier, entretien, administration…).
- **Versions** : brouillon de travail → proposé (direction) → **voté** (CA de l'OGEC / de l'EPLE, date et référence de la délibération, socle 46) → rectificatif(s) (décision modificative, même circuit) ; une version votée est **figée** ; le « budget courant » = dernière version votée.
- **Pré-remplissage** : réalisé N-1, budget N, tendance à la date, **recettes prévues** lues au simulateur du 24 (effectifs × tarifs), **masse salariale** prévue lue au 49 (effectifs, revalorisations de la CCN, ancienneté), amortissements prévus (15), engagements pluriannuels (contrats, CG4).
- **Simulation** : scénarios (« hausse de 2 % des tarifs, deux surveillants de plus ») comparés côte à côte, sans effet sur les versions (CGA-127).
- **Présentation au CA** : document du socle 7 (trame D-023), comparaison N-1, commentaires par poste, graphique simple ; jamais de donnée nominative (ni élève, ni salaire individuel).

### 10.2 Enveloppes et crédits par discipline

- La direction confie des **enveloppes** : par pôle (vie scolaire, CDI, restauration, entretien, administration), par **discipline** (crédits pédagogiques), par **séjour** (budget du 42), par projet (46, foyer, éco-délégués), avec un **responsable** (fonction du socle 6) et des **membres** autorisés à demander.
- **Répartition des crédits pédagogiques** : outil de calcul (au prorata des heures d'enseignement du socle 10 ou des élèves, avec un socle fixe) **proposé**, ajusté à la main, voté ou arrêté par la direction (CGA-131).
- **Disponible** en temps réel : voté − engagé − réalisé non engagé ; l'engagé se libère quand la facture est validée (le réalisé prend le relais) ou quand la commande est annulée.
- **Dépassement** : interdit par défaut au demandeur ; le responsable de l'enveloppe peut **virer** entre lignes de son enveloppe (trace) ; au-delà, demande à la direction (CGA-137) ; la direction peut autoriser un dépassement motivé (trace, R-CG17).

### 10.3 Suivi

- Tableau **voté / engagé / réalisé / disponible / taux** par ligne, enveloppe, section ; **prévision de fin d'exercice** (réalisé + engagé + tendance des récurrents) ; alertes à 80 % et 100 % (S-4806).
- **Comparaison** N-1 à date ; dérive par poste expliquée (les factures qui pèsent).
- **Budget d'un séjour** : prévu (42) vs engagé et payé (48) ; écart signalé à l'organisateur sans montant individuel des familles (42 R-SV…).

---

## 11. Achats, de la demande au paiement

### 11.1 Demande d'achat [R]

- Créée par tout personnel membre d'une enveloppe (web ou **mobile**) : objet, quantité, prix estimé, fournisseur suggéré (annuaire ou nouveau, en texte en attente de création), **devis** joint (photo, PDF, lien), date souhaitée, lieu de livraison, enveloppe et section (pré-remplies).
- **Catalogue** (option) : articles courants avec prix et fournisseur habituels (papeterie, produits d'entretien, denrées) ; commandes **récurrentes** (lait le lundi).
- Contrôle immédiat : **disponible** de l'enveloppe ; doublon probable (même objet, même montant, 30 jours) ; seuil de devis (au-delà de S-4808 : deux devis demandés) ; compteur commande publique (public, R-CG32).
- **Circuit** (moteur de règles, comme 42 R-SV4) : responsable de l'enveloppe → direction au-delà d'un seuil (S-4809) → comptable (contrôle d'imputation) ; délégations datées (socle 6) ; personne ne valide **sa propre** demande (R-CG23).
- États : brouillon → soumise → validée / refusée (motif) / à compléter → commandée → (close).

### 11.2 Commande

- Une ou plusieurs demandes validées → **bon de commande** numéroté (compteur par entité, E3), fournisseur, lignes, conditions, adresse de livraison et de facturation, **référence à rappeler sur la facture** (indispensable au rapprochement automatique).
- **Engagement** budgétaire au montant TTC (ou HT si TVA récupérable) à la création ; écart de prix à la facture → l'engagement s'ajuste, alerte au-delà de S-4810.
- Envoi au fournisseur : PDF par courriel (envoi **N3**, aperçu), ou commande passée sur le site du fournisseur (la commande ScolIA est alors un **engagement interne** avec la référence du site).
- Commande **sans demande** (comptable, urgence) : permise avec motif, visible au responsable de l'enveloppe.
- **Contrats et abonnements** (photocopieurs, logiciels, maintenance, assurances, énergie) : échéancier pluriannuel, engagements générés à chaque échéance, alerte de **reconduction tacite** avant la date limite de résiliation (CGA-163).

### 11.3 Réception et service fait

- Réception **totale ou partielle** par le demandeur, l'accueil ou l'intendance : quantités reçues, écarts (manquant, abîmé, non conforme), photo du bon de livraison ; **service fait** pour une prestation (intervention, transport, formation).
- Réception partielle → reliquat suivi, relance proposée au fournisseur.
- **Denrées** (socle 40) : réception avec contrôle des températures et des lots [NV, PMS du 40] ; le 48 ne garde que le constat et l'écart, la traçabilité sanitaire est au 40.

### 11.4 Facture fournisseur et rapprochement

- Arrivée : **plateforme agréée** (électronique structurée, §13), courriel dédié (PDF), papier scanné (accueil) ; dépôt au socle 7, empreinte ; **lecture** par l'IA (fournisseur, numéro, date, échéance, montants HT, TVA, TTC, référence de commande, IBAN imprimé) **proposée**, relue (R-CG40).
- **Rapprochement à trois voies** commande ↔ réception ↔ facture : automatique si la référence de commande est présente et les écarts sous tolérance (S-4811) ; sinon file « à rapprocher » ; facture **sans commande** (énergie, eau, téléphone : contrats) rattachée au contrat.
- **Doublon** : même fournisseur + même numéro, ou même montant + même date (± 3 jours) : blocage avec explication (R-CG25).
- **IBAN imprimé différent** de l'IBAN connu → alerte fraude, facture bloquée (R-CG26).
- **Bon à payer** : par le comptable (contrôle de forme et d'imputation) et, au-delà de S-4809, par la direction ; l'écriture d'achat passe du brouillard à « à valider ».
- **Litige** : facture contestée (motif, échanges, avoir attendu) ; elle sort des échéances à payer ; la relance du fournisseur n'est pas une raison de payer.
- **Avoir fournisseur** : rapproché de la facture d'origine.

### 11.5 Paiement

- Échéancier des factures validées ; **proposition de lot** (factures à échéance dans les N jours, S-4813) ; le comptable compose, la **double validation** est exigée au-delà d'un montant de lot (S-4814 : comptable + président, trésorier ou directeur, selon la délégation de l'OGEC) ; **réauthentification par passkey** à la validation (D-041 par analogie) ; fichier **SEPA pain.001** [NV : version acceptée par la banque] remis à la banque (téléversement manuel par défaut ; EBICS hors périmètre initial, Q-CG5).
- Autres moyens : chèque (numéro, remise au fournisseur), prélèvement fournisseur (mandat **reçu** : l'OGEC est débiteur ; contrôle à réception du relevé), carte (justificatif photographié dans les 48 h, CGA-187), espèces de caisse (petites dépenses, plafond S-4815).
- **Rapprochement** avec le relevé (§14).

---

## 12. Fournisseurs

- **Fiche** : raison sociale, SIREN/SIRET, TVA intracommunautaire, adresse, contacts, conditions de paiement, catégorie d'achat par défaut, **coordonnées bancaires** (IBAN, BIC, titulaire) **chiffrées** (E11, comme les IBAN des familles, D-014), attestation de vigilance URSSAF pour les sous-traitants au-delà de 5 000 € HT [NV, C. trav. L8222-1 et D8222-5] (alerte d'échéance semestrielle).
- **Création et modification de l'IBAN** : séparation des tâches — la personne qui crée ou change un IBAN ne valide pas le paiement suivant vers ce fournisseur ; **contre-appel** obligatoire à un numéro **déjà connu** (pas celui du courrier qui annonce le changement), consigné (qui, quand, numéro appelé) ; RIB signé joint ; délai de carence (S-4816) ; avis au responsable comptable et à la direction (R-CG26). C'est la fraude la plus fréquente envers les associations [H].
- **Vérification** du SIREN (répertoire Sirene, API publique de l'INSEE [NV]) à la création, sans transmettre d'autre donnée (CGA-196).
- **Fusion** de doublons de fournisseurs (même SIREN), historique conservé.
- **Fournisseur bloqué** (litige, fraude suspectée, liquidation) : plus de commande ni de paiement.
- **Évaluation** légère (délais, conformité, litiges) visible des seuls acheteurs, jamais exportée.

---

## 13. Factures électroniques reçues (réforme 2026)

1. **Fait réglementaire** (RC-07) : l'OGEC, assujetti à la TVA même pour des opérations exonérées, doit **pouvoir recevoir** des factures électroniques depuis le **1er septembre 2026**, par une **plateforme agréée** qu'il a choisie [V-sec].
2. **Position [R]** : ScolIA **ne devient pas** plateforme agréée (agrément de l'administration fiscale, obligations d'exploitation hors de proportion). ScolIA **se raccorde** à la plateforme choisie par l'OGEC (souvent celle de l'expert-comptable ou de sa banque [H]) par son API ou par un dépôt de fichiers, et **récupère** les factures reçues (Factur-X, UBL, CII), **lit les données structurées** (plus fiables que la lecture d'un PDF), rapproche, et **renvoie les statuts** de cycle de vie (« approuvée », « en litige », « payée ») si la plateforme le permet [NV sur le détail des statuts].
3. **Factures émises** aux familles : opérations exonérées, **hors champ** de l'émission électronique (24 §5.4) ; les éventuelles opérations taxables de l'OGEC envers des professionnels (location de salle à une entreprise) entreront dans l'émission au 1-9-2027 (PME) [V-sec] : le 24 les signale, le 48 vérifie que leur volume est connu (CGA-209).
4. **Transition** : tant que le raccordement n'existe pas, dépôt manuel du fichier ou du PDF reçu (CGA-201) ; aucune facture électronique n'est **imprimée puis rescannée**.
5. **Urgence** : l'échéance du 1-9-2026 est **passée** ; si l'OGEC pilote n'a pas choisi de plateforme, c'est une non-conformité actuelle, **hors ScolIA**, à signaler à Florian et à l'expert-comptable (CCG-60, Q-CG4).

---

## 14. Trésorerie

- **Comptes** : banque(s), livret, caisse(s), régie(s), carte(s) ; solde comptable et solde bancaire.
- **Relevés** : import CAMT.053 / CFONB 120 / OFX / CSV [NV sur la banque de l'OGEC] ; **jamais** de connexion par identifiants bancaires de l'utilisateur (pas d'agrégateur par écran) ; l'accès par l'API d'une banque (DSP2) est une option de contrat (Q-CG5).
- **Rapprochement** : automatique par référence (lot de prélèvements, numéro de virement, montant + date + tiers), proposé par l'IA pour le reste, validé ; lignes bancaires sans écriture → écriture proposée (frais bancaires, intérêts, virement reçu non identifié en 471 « à identifier ») ; **état de rapprochement** mensuel signé (document du 7) (CGA-215).
- **Caisse** : fond de caisse, entrées (espèces des familles, 25) et sorties (petites dépenses), **comptage** périodique (billets et pièces), écart constaté et validé, jamais corrigé en silence (R-CG29).
- **Prévisionnel** sur 6 à 12 mois : encaissements prévus (échéanciers du 24–25, taux de rejet historique), forfait d'externat (calendrier des versements), subventions, paie (49 : nets, charges, prélèvement à la source, taxe sur les salaires), factures et contrats fournisseurs, emprunts ; **alerte** de point bas (S-4817) ; scénarios.
- **Virements internes** entre comptes (58).

---

## 15. Immobilisations

- **Fiche de valeur** liée à la fiche du bien du socle 44 (un vidéoprojecteur, un minibus, des travaux de toiture) ou propre (travaux, logiciels) : date de mise en service, coût d'acquisition (depuis la facture fournisseur), mode et durée d'amortissement (**proposés** par catégorie, choisis par le comptable avec le cabinet), section analytique, subvention d'investissement associée.
- **Seuil** d'immobilisation (S-4818 ; 500 € HT est une pratique fiscale courante [NV] pour une charge immédiate des petits biens).
- **Plan d'amortissement** calculé, dotations **proposées** à la clôture (écriture en brouillard) ou reçues du cabinet (§3.3).
- **Sortie** (cession, mise au rebut, perte) : depuis le 44 ou ici ; valeur nette ; écriture proposée.
- **Inventaire physique** : le 44 le fait ; le 48 compare la liste des biens en valeur et la liste des biens présents (écarts).
- **Biens en commodat ou mis à disposition** (bâtiments du diocèse, de l'association propriétaire, de la collectivité) : fiche sans valeur, pour mémoire [H].

---

## 16. Analytique

- **Axes par défaut** (S-4819 : ajout libre d'axes limité à 5) : **activité** (enseignement, restauration, internat, pastorale, séjours, périscolaire, location, administration générale), **pôle**, **discipline**, **séjour** (un séjour = une section, créée par le 42), **projet**, **site** (si plusieurs bâtiments), **niveau** (école, collège, lycée, post-bac, pour la répartition demandée par certains financeurs [H]).
- Chaque ligne de charge ou de produit porte au moins la section **activité** ; les autres axes sont optionnels.
- **Clés de répartition** (charges communes : énergie, entretien, administration) par surface, effectifs, heures (socle 10) ; calcul **proposé** à la clôture.
- **Coût d'un élève**, **coût d'un repas**, **coût d'une nuitée d'internat** : indicateurs calculés (47) — utiles pour les tarifs (24) et pour justifier l'emploi des contributions (RC-10).
- **Synthèse APEL / financeurs** : rapport par séjour ou par projet (reprend la synthèse APEL de Next.js, NX du 42) avec le réalisé.

---

## 17. Clôture et relation avec l'expert-comptable

- **Clôture de période** (mensuelle) : contrôles (factures non rapprochées, banque non pointée, brouillards en attente, caisse non comptée) ; validation en lot ; export.
- **Arrêté d'exercice** (option 2) : liste des **réceptions sans facture** (charges à payer, 408), **factures reçues d'avance** (charges constatées d'avance, 486), **produits constatés d'avance** (24), **dotations** proposées, **engagements hors bilan** (contrats) ; **dossier de révision** pour le cabinet (états de rapprochement signés, balances âgées clients et fournisseurs, liste des immobilisations, caisse, subventions et fonds dédiés) ; **export définitif** de l'exercice.
- **Retour du cabinet** : import de ses écritures d'inventaire et de clôture et des **à-nouveaux** (FEC du cabinet ou CSV) dans un journal dédié, marquées « origine cabinet » ; rapprochement des balances (ScolIA / cabinet) avec écarts listés (CGA-235).
- **Lot optionnel CG13** (tenue complète) : écritures d'inventaire faites dans ScolIA, clôture, à-nouveaux automatiques, bilan, compte de résultat, annexe ANC 2018-06 ; **non engagé** sans Q-CG1.

---

## 18. Public (EPLE)

- **Rôle de ScolIA** : outil des **services** de l'établissement, jamais du comptable public (R-CG3).
  - Demandes d'achat des enseignants et des services, validées par le gestionnaire et l'ordonnateur, **avant** l'engagement dans Op@le.
  - **Suivi des crédits** par discipline et par service, au plus près du terrain (Op@le suit les crédits par service budgétaire et domaine [NV]).
  - **Compteur commande publique** (RC-23) par famille homogène (nomenclature de l'établissement, S-4812).
  - Préparation des **pièces** (devis, bons de commande signés, constat de service fait) déposées au socle 7 et jointes à l'engagement Op@le.
  - **Régie** de recettes (24 F13) et régie d'avances (petites dépenses de séjour).
  - **Export** CSV des engagements préparés pour saisie ou import dans Op@le (format à obtenir, Q-CG9).
- **Pas** de plan de comptes M9.6 dans ScolIA, pas d'écriture comptable publique, pas de mandatement.
- Vocabulaire et circuit propres au public (ordonnateur, adjoint gestionnaire, agent comptable) dans les rôles par défaut du profil public.

---

## 19. Cas limites (catalogue)

### 19.1 Écritures et exports

| # | Cas | Traitement |
|---|---|---|
| CCG-01 | Une facture aux familles est émise le 31 août à 23 h 50 (heure de Paris) | date comptable 31/08 en Europe/Paris (KD-06) ; exercice N |
| CCG-02 | Le téléchargement d'un export échoue | le fichier est un objet conservé ; re-téléchargement identique, même empreinte (KD-01, CGA-234) |
| CCG-03 | Le cabinet a perdu l'export de mars | re-téléchargement ; aucun nouvel export des mêmes pièces sans annulation tracée (R-CG14) |
| CCG-04 | Un export a été importé deux fois chez le cabinet | la balance de rapprochement (CGA-235) montre le doublement ; ScolIA ne corrige pas chez le cabinet, il le signale |
| CCG-05 | Prélèvement exporté puis rejeté le mois suivant | écriture de rejet dans la période du rejet, lettrée avec l'encaissement (KD-02) |
| CCG-06 | Le schéma comptable d'un article change en cours d'exercice | nouvelle version datée ; les faits antérieurs gardent l'ancien schéma ; les brouillards non validés sont recalculés (CGA-106) |
| CCG-07 | Une écriture validée est fausse | contrepassation + écriture juste, liées (R-CG11) ; jamais de modification |
| CCG-08 | Une période est close mais une facture fournisseur datée de cette période arrive | comptabilisée à la date de réception dans la première période ouverte ; la date de la pièce reste sur la ligne (`PieceDate`) |
| CCG-09 | Un fait (facture du 24) n'a pas produit d'écriture (panne) | contrôle d'exhaustivité nocturne (CGA-227) : anomalie, rejeu idempotent |
| CCG-10 | Le compteur de numérotation et les numéros divergent (restauration partielle) | contrôle de continuité (CGA-226) bloque la validation, tâche bloquante pour l'administrateur ; jamais de renumérotation |
| CCG-11 | Écriture déséquilibrée d'un centime (arrondis de TVA) | ligne d'écart d'arrondi sur compte paramétré (658 / 758) sous un plafond (1 € par écriture) ; au-delà, refus |
| CCG-12 | Restauration de la base à J-1 (PITR) | les exports de J sont dans le registre des objets mais plus en base : le registre E12 les marque « à réconcilier » ; les écritures sont rejouées par l'exhaustivité ; aucun numéro réattribué sans contrôle (§29) |
| CCG-13 | Deux comptables valident en même temps le même lot | verrou sur la période et le journal ; le second reçoit « déjà validé » |

### 19.2 Budget et achats

| # | Cas | Traitement |
|---|---|---|
| CCG-20 | Un professeur demande un achat sur une enveloppe épuisée | la demande est possible, marquée « dépassement » ; le responsable décide (CGA-137) |
| CCG-21 | Le responsable de l'enveloppe est le demandeur | la validation remonte au niveau supérieur (R-CG23) |
| CCG-22 | Le responsable est absent trois semaines | délégation datée (socle 6) ; à défaut, la demande remonte à la direction après S-4807 jours |
| CCG-23 | Une commande est passée en juin pour la rentrée | engagement sur le **budget N+1** s'il est voté, sinon sur une enveloppe provisoire (CGA-152) |
| CCG-24 | Le budget N+1 n'est pas voté au 1er septembre | le budget courant est la dernière version **proposée**, signalée « non votée » ; enveloppes plafonnées à un pourcentage (S-4820) |
| CCG-25 | Facture supérieure de 12 % à la commande | rapprochement bloqué au-delà de la tolérance ; ajustement de l'engagement validé par le responsable |
| CCG-26 | Livraison partielle puis facture totale | rapprochement à trois voies : facture partiellement payable (quantités reçues) ou en attente du reliquat (choix du comptable) |
| CCG-27 | Facture sans référence de commande | file « à rapprocher » ; l'IA propose les commandes ouvertes du fournisseur (R-CG40) |
| CCG-28 | Même facture reçue par la plateforme et par courriel | doublon détecté par (fournisseur, numéro) ; la version structurée prime (R-CG25) |
| CCG-29 | Un séjour est annulé après des commandes passées | engagements non commandés libérés ; commandes passées listées pour annulation chez le fournisseur ; avoirs attendus suivis (42) |
| CCG-30 | Achat urgent payé par carte par un personnel, sans demande | régularisation : justificatif dans 48 h, imputation, validation a posteriori (CGA-187) ; signalé au responsable |
| CCG-31 | Un personnel avance de l'argent de sa poche (note de frais) | note de frais : pièces, validation, remboursement par virement (tiers « personnel », hors paie) ; jamais une prime déguisée (49) |
| CCG-32 | Public : six commandes de 9 000 € HT de mobilier sur l'exercice chez des fournisseurs différents | le compteur de la famille « mobilier » dépasse le seuil de dispense : alerte au gestionnaire (R-CG32), aucune commande bloquée sans décision |
| CCG-33 | Abonnement à reconduction tacite dont la date de préavis passe pendant les vacances | alerte à J-60 et J-30 de la date limite de résiliation (CGA-163) |
| CCG-34 | Un agent quitte l'établissement avec des demandes en cours | réaffectées à son responsable (§7.4) |
| CCG-35 | Commande de denrées pour la cantine (socle 40) refusée faute de disponible | la cantine ne peut pas manquer de denrées : l'enveloppe « restauration » autorise par défaut un dépassement signalé (S-4821) |

### 19.3 Fournisseurs, paiements, fraude

| # | Cas | Traitement |
|---|---|---|
| CCG-40 | Courriel « nous changeons de banque, voici notre nouveau RIB » | IBAN en attente ; contre-appel au numéro **déjà connu** ; second valideur ; carence (R-CG26) ; tous les paiements vers ce fournisseur suspendus d'ici là |
| CCG-41 | L'IBAN imprimé sur une facture diffère de celui de la fiche | facture bloquée, alerte fraude (R-CG26) |
| CCG-42 | Le comptable crée un fournisseur et prépare le paiement | séparation des tâches : le paiement vers un fournisseur créé ou modifié dans les 30 derniers jours exige un second valideur, quel que soit le montant (R-CG27) |
| CCG-43 | Lot de virements validé, fichier non transmis à la banque | lot « généré, non remis » ; aucune facture marquée payée tant que le relevé n'a pas confirmé, ou que le comptable n'a pas attesté la remise (R-CG28) |
| CCG-44 | Fichier SEPA remis deux fois à la banque | identifiant unique de message et de lot ; le relevé montre le double débit : rapprochement en anomalie, procédure de rappel |
| CCG-45 | Fournisseur en liquidation judiciaire | fournisseur bloqué ; factures à payer au mandataire selon les instructions [NV] (saisie manuelle, pièce) |
| CCG-46 | Prélèvement fournisseur (énergie) d'un montant anormal | rapprochement avec la facture du contrat ; écart > S-4811 : anomalie, contestation possible (délai SEPA de 8 semaines pour un prélèvement autorisé [NV]) |
| CCG-47 | Écart de caisse de 20 € | constaté, motivé, validé par la direction ; jamais compensé en silence (R-CG29) |
| CCG-48 | Le président de l'OGEC, deuxième valideur, n'a pas de passkey | il doit en enregistrer une (D-009) ; à défaut, la délégation de validation désigne un autre membre du bureau |

### 19.4 Entités, public, transition

| # | Cas | Traitement |
|---|---|---|
| CCG-50 | Un OGEC gère deux établissements dans ScolIA | une entité, des sections « site » ; tant que Q13 (groupes) n'est pas décidée : **deux dossiers** dans ScolIA, consolidation chez le cabinet (R-CG4) |
| CCG-51 | Un établissement passe du privé au public (ou change d'OGEC) | nouvelle entité, nouvel exercice ; l'ancienne entité reste consultable en lecture ; aucune écriture transférée automatiquement |
| CCG-52 | Public : un enseignant demande un achat déjà couvert par un marché de la collectivité | rattachement au marché (référence) ; le compteur de seuil ne l'additionne pas (S-4812) |
| CCG-53 | Doublon 2027-2028 : la comptabilité officielle reste dans l'outil actuel | ScolIA en **mode parallèle** : écritures validées en interne, exports « à blanc » pour comparaison, jamais importés chez le cabinet sans décision (R-CG35) |
| CCG-54 | Fin du doublon, bascule au 1er septembre 2028 | à-nouveaux importés du cabinet (CGA-287) ; première période réelle ; l'historique parallèle reste marqué « parallèle » |
| CCG-55 | Commissaire aux comptes demande l'accès | accès **temporaire en lecture**, journalisé, daté de fin (CGA-267) |
| CCG-56 | Le comptable salarié est absent longtemps | suppléant (cabinet ou direction) avec un rôle prédéfini ; file de travail visible ; documentation des procédures (CGA-293) |
| CCG-57 | Subvention reçue pour un projet pluriannuel | fonds dédiés : suivi de l'emploi par exercice (CGA-251) |
| CCG-58 | Location ponctuelle d'une salle à une entreprise (opération taxable) | facture émise au 24 avec TVA ; produit 752 / 706 ; TVA collectée 44571 ; signalée pour l'émission électronique au 1-9-2027 (CGA-209) |
| CCG-59 | Le forfait d'externat arrive avec trois mois de retard | prévisionnel de trésorerie : versement attendu à la date habituelle, retard signalé ; aucune écriture avant réception (créance selon Q-CG7) |
| CCG-60 | L'OGEC n'a pas choisi de plateforme agréée (échéance du 1-9-2026 passée) | non-conformité actuelle hors ScolIA : **tâche bloquante pour la direction** et signal à Florian (§13) |

---

## 20. Intelligence artificielle (contrat IA du socle)

### 20.1 Ce que l'IA fait [R]
- **Lit** une facture (PDF, photo) : fournisseur, numéro, dates, montants, TVA, référence de commande, IBAN imprimé — **proposé**, champ par champ avec son niveau de confiance, relu avant enregistrement (R-CG40). Pour une facture structurée (Factur-X, UBL), la lecture est déterministe, sans modèle de langage.
- **Propose** une imputation (compte, section) à partir de l'historique du fournisseur et de la demande ; jamais imposée.
- **Rapproche** : propose la commande d'une facture sans référence, la ligne de relevé d'un paiement.
- **Détecte** : doublons probables, IBAN changé, montant inhabituel pour un fournisseur, dérive d'un poste budgétaire, abonnement oublié, facture en retard de paiement, fractionnement probable (public).
- **Explique** : « pourquoi le 606 dépasse » (les cinq factures qui pèsent), « que reste-t-il sur les SVT ».
- **Prépare** : brouillon du budget N+1 (tendances), brouillon de relance d'un fournisseur, commentaire de présentation au CA.

### 20.2 Ce que l'IA ne fait pas
- Ne valide aucune écriture, aucun bon à payer, aucun lot de paiement, aucune modification d'IBAN, aucun vote ; n'envoie rien à un fournisseur ni à une banque.
- Ne voit pas les salaires individuels (journal de paie agrégé seulement, R-CG22), ni les données des familles au-delà du numéro de tiers.
- Ne transmet pas de pièce à un fournisseur d'IA hors du cadre validé (`IA_COEUR.md` ; Mistral à l'étude) ; les factures lues peuvent contenir des données personnelles de tiers (noms de salariés d'un fournisseur) : minimisation de l'envoi (zones utiles), aucune conservation chez le fournisseur d'IA.

### 20.3 Capacités déclarées (registre `ent-ia`)

| Capacité | Type | Droit | Niveau |
|---|---|---|---|
| `budget.disponible` | lecture | `budget.lire` (ou membre de l'enveloppe) | N0 |
| `achats.demande_brouillon` | commande | `achats.demander` | N1 (brouillon, l'humain soumet) |
| `factures_fournisseurs.lire_piece` | lecture | `factures_fournisseurs.gerer` | N0 |
| `factures_fournisseurs.proposer_saisie` | commande | idem | N2 (aperçu, confirmation) |
| `comptabilite.proposer_imputation` | commande | `comptabilite.saisir` | N2 |
| `tresorerie.proposer_rapprochement` | commande | `tresorerie.gerer` | N2 |
| `comptabilite.anomalies` | lecture | `comptabilite.lire` | N0 |
| `budget.preparer_brouillon` | commande | `budget.preparer` | N2 |
| Validation d'écritures, bon à payer, lots, IBAN | — | — | **non exposées à l'IA** |

### 20.4 Cadre
Règlement européen sur l'IA : usages comptables non classés « à haut risque » [NV] ; étude juridique commune avant mise en service des fonctions à modèle de langage (comme D-032). Jeu d'évaluation : 200 factures fictives variées (scans de travers, multi-pages, avoirs), mesure de l'exactitude champ par champ.

---

## 21. Actions métier (CGA-100 à CGA-299)

Acteurs : **COMPTA** comptable de l'entité · **DIR** direction · **RESP** responsable d'enveloppe · **PERS** personnel membre d'une enveloppe · **ACC** accueil / intendance · **OGEC** président, trésorier (lecture et seconde validation) · **CAB** cabinet ou commissaire aux comptes (accès temporaire) · **GEST** gestionnaire d'EPLE · **SYS** système · **IA** assistant. ★ = confirmation renforcée (N3, réauthentification au-delà des seuils).

### 21.1 Paramétrage (CG1)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-100 | Créer ou compléter l'**entité comptable** (raison sociale, SIREN, régime privé/public, exercice type) — reprise de `ent_fin_entite_facturation` | DIR, COMPTA · `comptabilite.parametrer` | une entité par établissement tant que Q13 n'est pas décidée (R-CG4) |
| CGA-101 | Ouvrir un exercice (dates, périodes mensuelles générées) | COMPTA · `comptabilite.parametrer` | chevauchement interdit |
| CGA-102 | Charger le **plan de comptes par défaut** ANC 2018-06 | COMPTA · idem | modifiable ; avis du cabinet (Q-CG3) |
| CGA-103 | Ajouter, renommer, désactiver un compte | COMPTA · idem | un compte mouvementé ne se supprime pas |
| CGA-104 | Paramétrer les journaux (codes, libellés, compte de contrepartie) | COMPTA · idem | codes alignés sur le cabinet |
| CGA-105 | Consulter les **schémas comptables** actifs et leur historique | COMPTA, CAB · `comptabilite.lire` | |
| CGA-106 | Modifier un schéma (nouvelle version datée) ★ | COMPTA · `comptabilite.parametrer` + second valideur | recalcul des brouillards non validés ; jamais rétroactif sur une écriture validée |
| CGA-107 | **Simuler** un schéma sur une période passée (aucune écriture) | COMPTA · idem | aperçu des écarts |
| CGA-108 | Paramétrer les comptes par moyen de paiement (5112, 53, 512…) | COMPTA · idem | corrige KD-04 |
| CGA-109 | Gérer les axes et sections analytiques | COMPTA, DIR · idem | 5 axes au plus (S-4819) |
| CGA-110 | Gérer les **clés de répartition** | COMPTA · idem | |
| CGA-111 | Régler les seuils (validation direction, double validation, devis, tolérances) | DIR · `comptabilite.parametrer` | S-4803 à S-4816 |
| CGA-112 | Déclarer les **délégations de validation** (lu au socle 6) | DIR · `personnels.gerer` (socle 6) | datées, expirent seules |
| CGA-113 | Générer la table de correspondance tiers « familles » (numéro ↔ compte payeur) | SYS | jamais exportée avec l'écriture (R-CG21) |
| CGA-114 | Créer un tiers non famille (client, financeur, personnel pour notes de frais) | COMPTA · `comptabilite.saisir` | |
| CGA-115 | Paramétrer le régime de TVA de l'entité (non récupérable, prorata) | COMPTA · `comptabilite.parametrer` | avis du cabinet (Q-CG6) |
| CGA-116 | Choisir le profil de l'entité (privé, public) : écrans, vocabulaire, fonctions | DIR · idem | public : pas de comptabilité générale (R-CG3) |
| CGA-117 | Rattacher plusieurs établissements à une entité | administrateur plateforme | **désactivé** (Q13, R-CG4) |
| CGA-118 | Gérer des **modèles d'écritures récurrentes** (loyers, abonnements) | COMPTA · `comptabilite.saisir` | génèrent des brouillards |
| CGA-119 | Consulter le journal des paramètres (qui a changé quoi) | COMPTA, DIR, CAB · `comptabilite.lire` | |

### 21.2 Budget et enveloppes (CG2)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-120 | Créer le budget d'un exercice (brouillon) | COMPTA, DIR · `budget.preparer` | |
| CGA-121 | Pré-remplir depuis le réalisé N-1, le budget N et les engagements pluriannuels | COMPTA · idem | |
| CGA-122 | Saisir et ajuster les lignes (tableau dense) | COMPTA · idem | |
| CGA-123 | Importer les **recettes prévues** du simulateur du 24 | COMPTA · idem | port `facturation::simuler` |
| CGA-124 | Importer la **masse salariale prévue** du 49 | COMPTA · idem + `paie.lire_agrege` | agrégée, jamais par salarié |
| CGA-125 | Commenter un poste | COMPTA, DIR · idem | |
| CGA-126 | Passer en « proposé » | DIR · `budget.preparer` | |
| CGA-127 | **Simuler** des scénarios et les comparer | DIR, COMPTA · idem | sans effet sur les versions |
| CGA-128 | Comparer deux versions | DIR, COMPTA, OGEC · `budget.lire` | |
| CGA-129 | Enregistrer le **vote** (date, instance, délibération du socle 46) ★ | DIR · `budget.voter` | fige la version |
| CGA-130 | Produire le document de présentation au CA | DIR, COMPTA · `budget.lire` | trame du socle 7, sans donnée nominative |
| CGA-131 | **Répartir les crédits pédagogiques** (calcul proposé : heures, élèves, socle fixe) | DIR · `budget.enveloppe.gerer` | ajusté à la main |
| CGA-132 | Créer une enveloppe (pôle, discipline, séjour, projet) | DIR · idem | |
| CGA-133 | Désigner le responsable et les membres d'une enveloppe | DIR · idem | par fonction du socle 6 de préférence |
| CGA-134 | Virer entre lignes d'une enveloppe | RESP · `budget.enveloppe.virer` | tracé |
| CGA-135 | Préparer une **décision modificative** (nouvelle version) | DIR, COMPTA · `budget.preparer` | même circuit que le vote |
| CGA-136 | Consulter le disponible de ses enveloppes | PERS, RESP · membre | sans le détail des autres enveloppes |
| CGA-137 | Demander un dépassement | RESP · membre | motif |
| CGA-138 | Autoriser un dépassement motivé | DIR · `budget.voter` ou délégation | R-CG17 |
| CGA-139 | Saisir un engagement manuel (source externe : devis signé hors ScolIA) | COMPTA · `budget.engager` | pièce |
| CGA-140 | Libérer un engagement | COMPTA, SYS | à l'annulation ou à la facture |
| CGA-141 | Tableau **voté / engagé / réalisé / disponible** | DIR, COMPTA, OGEC, RESP (le sien) · `budget.lire` | |
| CGA-142 | Prévision de fin d'exercice | DIR, COMPTA · idem | |
| CGA-143 | Alertes de consommation (80 %, 100 %) | SYS → RESP, DIR | S-4806 |
| CGA-144 | Comparaison N-1 à date | DIR, COMPTA · idem | |
| CGA-145 | Lier une section « séjour » au budget du séjour (42) | SYS | port `sorties::budget` |
| CGA-146 | Exporter le budget en tableur | DIR, COMPTA · `budget.lire` | |
| CGA-147 | Plan pluriannuel d'investissement (travaux, équipements) | DIR, COMPTA · `budget.preparer` | lien 15, 44 |
| CGA-148 | Reporter des crédits non consommés sur N+1 (décision) | DIR · `budget.voter` | privé : selon l'OGEC ; public : selon M9.6 [NV] |
| CGA-149 | Clore le budget d'un exercice (lecture seule) | COMPTA · `budget.preparer` | |

### 21.3 Demandes, commandes, contrats, réceptions (CG3, CG4)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-150 | Créer une **demande d'achat** (web, mobile) | PERS · `achats.demander` | 30 secondes ; pré-remplie |
| CGA-151 | Joindre un devis, une photo, un lien | PERS · idem | socle 7 |
| CGA-152 | Demander sur l'exercice suivant (commande anticipée) | PERS · idem | CCG-23 |
| CGA-153 | Demander depuis le **catalogue** | PERS · idem | |
| CGA-154 | Gérer le catalogue (articles, prix, fournisseur habituel) | COMPTA · `achats.commander` | |
| CGA-155 | Soumettre la demande | PERS · idem | contrôles immédiats (§11.1) |
| CGA-156 | Valider une demande (responsable) | RESP · `achats.valider` | jamais la sienne (R-CG23) ; mobile |
| CGA-157 | Refuser ou renvoyer « à compléter » (motif) | RESP · idem | avis au demandeur |
| CGA-158 | Valider au-delà du seuil (direction) | DIR · idem | S-4809 |
| CGA-159 | Contrôler l'imputation (compte, section) | COMPTA · `achats.commander` | proposition IA |
| CGA-160 | Regrouper des demandes en une commande par fournisseur | COMPTA · idem | |
| CGA-161 | Créer une commande (avec ou sans demande, motif) | COMPTA · idem | engagement à la création |
| CGA-162 | Envoyer la commande au fournisseur ★ | COMPTA · idem | N3, aperçu, PDF du socle 7 |
| CGA-163 | Gérer un **contrat ou abonnement** (échéancier, préavis, alerte de reconduction) | COMPTA · idem | CCG-33 |
| CGA-164 | Préparer la résiliation d'un contrat (courrier) ★ | COMPTA, DIR · idem | envoi N3 |
| CGA-165 | Modifier (avenant) ou annuler une commande | COMPTA · idem | engagement ajusté ou libéré |
| CGA-166 | Relancer un fournisseur en retard de livraison (brouillon, envoi ★) | COMPTA · idem | |
| CGA-167 | **Réceptionner** une commande (totale, partielle) | ACC, PERS (demandeur), COMPTA · `achats.receptionner` | photo du bon de livraison |
| CGA-168 | Signaler un écart ou une non-conformité | idem | tâche au comptable |
| CGA-169 | Constater le **service fait** d'une prestation | PERS (demandeur), RESP · idem | |
| CGA-170 | **Régie d'avances** : avance, justification des dépenses, reconstitution | COMPTA, régisseur désigné · `tresorerie.gerer` | séjours, petites dépenses (RC-22 pour le public) |
| CGA-171 | Programmer une commande récurrente | COMPTA · `achats.commander` | |
| CGA-172 | Suivre ses demandes (« commandée, livrée ») | PERS · demandeur | sans prix si non autorisé (S-4822) |
| CGA-173 | File des commandes en attente de réception | COMPTA, ACC · `achats.receptionner` | |
| CGA-174 | Enregistrer un retour au fournisseur | COMPTA · `achats.commander` | avoir attendu |
| CGA-175 | Exiger deux devis au-delà d'un seuil | SYS | S-4808 |
| CGA-176 | Consulter le **compteur de commande publique** par famille | GEST, COMPTA · `achats.commander` | public ; privé si activé |
| CGA-177 | Alerte de fractionnement probable | SYS → GEST | R-CG32 |
| CGA-178 | Recevoir une commande de denrées du socle 40 | SYS | port `budget::engager` |
| CGA-179 | Recevoir un engagement de séjour du socle 42 | SYS | idem |

### 21.4 Factures fournisseurs et paiements (CG6, CG7)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-180 | Recevoir une facture (plateforme, courriel dédié, scan de l'accueil) | SYS, ACC · `factures_fournisseurs.deposer` | socle 7 |
| CGA-181 | Lecture de la facture proposée par l'IA | IA | R-CG40 |
| CGA-182 | Relire et corriger la saisie | COMPTA · `factures_fournisseurs.gerer` | |
| CGA-183 | Rapprochement **automatique** commande–réception–facture | SYS | tolérances S-4811 |
| CGA-184 | Rapprocher à la main | COMPTA · idem | |
| CGA-185 | Contrôle de **doublon** (blocage expliqué) | SYS | R-CG25 |
| CGA-186 | Donner le **bon à payer** ★ (au-delà du seuil : direction) | COMPTA, DIR · `factures_fournisseurs.valider` | |
| CGA-187 | Déposer le justificatif d'un achat par carte (48 h) | PERS porteur de carte · `achats.demander` | relance automatique |
| CGA-188 | Ouvrir un **litige** (motif, échanges) | COMPTA · `factures_fournisseurs.gerer` | sort de l'échéancier |
| CGA-189 | Rapprocher un **avoir** de la facture d'origine | COMPTA · idem | |
| CGA-190 | Échéancier des factures à payer | COMPTA, DIR · idem | retards signalés (RC-08) |
| CGA-191 | Composer un **lot de paiement** | COMPTA · `paiements_fournisseurs.preparer` | proposé (S-4813) |
| CGA-192 | Valider un lot ★ (double validation au-delà de S-4814, passkey) | DIR, OGEC · `paiements_fournisseurs.valider` | R-CG27 |
| CGA-193 | Générer le fichier **SEPA pain.001** | SYS | identifiants uniques (CCG-44) |
| CGA-194 | Enregistrer un paiement par chèque, prélèvement fournisseur, espèces | COMPTA · `paiements_fournisseurs.preparer` | |

### 21.5 Fournisseurs et factures électroniques (CG5, CG14)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-195 | Créer un fournisseur | COMPTA · `fournisseurs.gerer` | |
| CGA-196 | Vérifier le SIREN au répertoire public | SYS | seul le SIREN est transmis [NV sur l'API] |
| CGA-197 | Demander la modification d'un **IBAN** (RIB, contre-appel consigné) | COMPTA · `fournisseurs.gerer` | R-CG26 |
| CGA-198 | Valider la modification d'IBAN ★ (autre personne) | DIR · `fournisseurs.iban.valider` | carence S-4816 |
| CGA-199 | Bloquer, débloquer un fournisseur | COMPTA, DIR · `fournisseurs.gerer` | |
| CGA-200 | Fusionner deux fournisseurs (même SIREN) | COMPTA · idem | historique gardé |
| CGA-201 | Déposer à la main une facture électronique reçue | COMPTA · `factures_fournisseurs.deposer` | transition |
| CGA-202 | Paramétrer le raccordement à la **plateforme agréée** | administrateur · `comptabilite.parametrer` | Q-CG4 |
| CGA-203 | Récupérer les factures de la plateforme (tâche périodique) | SYS | file à neutraliser à la restauration (E9) |
| CGA-204 | Renvoyer un statut de cycle de vie (approuvée, litige, payée) | SYS | si la plateforme le permet [NV] |
| CGA-205 | Suivre les **attestations de vigilance** (sous-traitants) | COMPTA · `fournisseurs.gerer` | alerte semestrielle [NV] |
| CGA-206 | Noter un fournisseur (délais, conformité) | COMPTA, RESP · `achats.commander` | interne |
| CGA-207 | Consulter l'annuaire des fournisseurs | PERS · `achats.demander` | sans IBAN |
| CGA-208 | Exporter la liste des fournisseurs (sans IBAN) | COMPTA · `fournisseurs.gerer` | |
| CGA-209 | Contrôler le volume d'opérations **taxables** émises (préparation de l'émission électronique 2027) | COMPTA · `comptabilite.lire` | lien 24 |

### 21.6 Trésorerie (CG7)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-210 | Créer un compte de trésorerie (banque, caisse, régie, carte) | COMPTA · `tresorerie.gerer` | IBAN chiffré |
| CGA-211 | Importer un **relevé** (CAMT.053, CFONB 120, OFX, CSV) | COMPTA · idem | doublon de relevé détecté |
| CGA-212 | Rapprochement automatique | SYS | |
| CGA-213 | Rapprochement proposé (IA) ou manuel | COMPTA · idem | |
| CGA-214 | Créer l'écriture d'une ligne bancaire (frais, intérêts, 471 à identifier) | COMPTA · `comptabilite.saisir` | |
| CGA-215 | **État de rapprochement** mensuel signé | COMPTA · `tresorerie.gerer` ; DIR signe | document du 7 |
| CGA-216 | Compter la **caisse** ; constater un écart ★ | COMPTA · idem ; DIR valide | R-CG29 |
| CGA-217 | Remise de chèques en banque (bordereau, lien 25) | COMPTA · idem | écriture 512 / 5112 |
| CGA-218 | **Prévisionnel** de trésorerie (6–12 mois, scénarios) | COMPTA, DIR, OGEC · `tresorerie.lire` | 24, 25, 49, contrats |
| CGA-219 | Virement interne entre comptes | COMPTA · `tresorerie.gerer` | 58 |

### 21.7 Écritures, contrôles, exports, clôture (CG1, CG8, CG9)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-220 | Consulter le **brouillard** (écritures non validées) | COMPTA, CAB · `comptabilite.lire` | |
| CGA-221 | Contrôler la **conformité FEC** d'un export (champs, numérotation, équilibre, dates) | SYS, COMPTA | avant remise |
| CGA-222 | **Valider** des écritures (lot) ★ | COMPTA · `comptabilite.valider` | numéro continu, date de validation |
| CGA-223 | Saisir une **opération diverse** | COMPTA · `comptabilite.saisir` | pièce obligatoire ; jamais sur un 411 famille (R-CG12) |
| CGA-224 | Valider une OD au-delà du seuil ou sur compte sensible (autre personne) ★ | DIR, CAB · `comptabilite.valider` | S-4803 |
| CGA-225 | **Contrepasser** une écriture | COMPTA · `comptabilite.valider` | R-CG11 |
| CGA-226 | Contrôle de **continuité** de la numérotation (nocturne) | SYS | CCG-10 |
| CGA-227 | Contrôle d'**exhaustivité** (fait sans écriture) | SYS | CCG-09 |
| CGA-228 | Contrôle du **chaînage** des empreintes | SYS | |
| CGA-229 | **Lettrer** les comptes de tiers (auto et manuel) | COMPTA · `comptabilite.saisir` | date de lettrage (KD-09) |
| CGA-230 | **Clôturer une période** (contrôles, validation, verrou) ★ | COMPTA · `comptabilite.valider` | |
| CGA-231 | **Exporter** (FEC, format d'éditeur) ★ | COMPTA · `comptabilite.exporter` | fichier conservé (KD-01) |
| CGA-232 | Importer les écritures du cabinet et les **à-nouveaux** | COMPTA · `comptabilite.saisir` | journal « origine cabinet » |
| CGA-233 | Annuler un export (motif, tracé) ★ | COMPTA + DIR · `comptabilite.exporter` | pièces exportables à nouveau (R-CG14) |
| CGA-234 | Re-télécharger un export à l'identique | COMPTA, CAB · `comptabilite.exporter` | |
| CGA-235 | **Rapprocher les balances** ScolIA / cabinet | COMPTA · `comptabilite.lire` | écarts listés |
| CGA-236 | Balance, grand livre, journal **de contrôle** | COMPTA, CAB, DIR · `comptabilite.lire` | |
| CGA-237 | Balances âgées clients et fournisseurs | COMPTA, DIR · idem | clients : par numéro de tiers ; nom visible avec `facturation.lire` |
| CGA-238 | **Arrêté** d'exercice : charges à payer, charges et produits constatés d'avance | COMPTA · `comptabilite.saisir` | brouillards proposés |
| CGA-239 | Produire le **dossier de révision** | COMPTA · `comptabilite.lire` | §17 |

### 21.8 Analytique et immobilisations (CG10, CG11)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-240 | Imputer ou corriger l'analytique d'une ligne (avant validation) | COMPTA · `comptabilite.saisir` | |
| CGA-241 | Répartir les charges communes (clés) | COMPTA · idem | proposé |
| CGA-242 | Coûts de revient : élève, repas, nuitée | COMPTA, DIR · `comptabilite.lire` | 47 |
| CGA-243 | Synthèse d'un séjour, d'un projet, pour un financeur ou l'APEL | COMPTA, DIR, organisateur (le sien) · idem | reprend la synthèse APEL |
| CGA-244 | État de l'**emploi des contributions des familles** (R442-48) | COMPTA, DIR · idem | |
| CGA-245 | Créer une immobilisation depuis une facture | COMPTA · `immobilisations.gerer` | lien 44 |
| CGA-246 | Calculer le plan d'amortissement | SYS | mode et durée choisis |
| CGA-247 | Proposer les dotations de l'exercice | SYS → COMPTA | brouillard |
| CGA-248 | Sortir une immobilisation (cession, rebut, perte) | COMPTA · idem | depuis le 44 |
| CGA-249 | Comparer inventaire en valeur et inventaire physique (44) | COMPTA · idem | |
| CGA-250 | Suivre une **subvention d'investissement** et sa quote-part | COMPTA · idem | |
| CGA-251 | Suivre les **fonds dédiés** | COMPTA · `comptabilite.saisir` | |
| CGA-252 | Tenir la liste des biens mis à disposition (pour mémoire) | COMPTA · `immobilisations.gerer` | |
| CGA-253 | Suivre les emprunts (échéancier, intérêts, capital) | COMPTA · `tresorerie.gerer` | |
| CGA-254 | Valoriser les **contributions volontaires en nature** (bénévolat) | COMPTA · `comptabilite.saisir` | 86/87, déclaratif |

### 21.9 Pilotage, rapports, public (CG11, CG12)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-255 | Tableau de bord de la direction (tuiles : disponible, trésorerie, factures en retard, anomalies) | DIR · `comptabilite.lire` | |
| CGA-256 | Espace **en lecture** du bureau de l'OGEC | OGEC · rôle prédéfini « Bureau OGEC » | aucune donnée nominative |
| CGA-257 | Rapport mensuel préparé pour le bureau (envoi ★) | COMPTA | N3 |
| CGA-258 | Exposer les indicateurs au pilotage (47) | SYS | |
| CGA-259 | Export pour le CA (budget, réalisé) | DIR · `budget.lire` | |
| CGA-260 | Public : demande validée → **pré-engagement** à saisir dans Op@le | GEST · `achats.commander` | |
| CGA-261 | Public : export CSV des pré-engagements | GEST · idem | format Q-CG9 |
| CGA-262 | Compteur des **subventions publiques** de l'exercice (seuil du CAC) | SYS → DIR, COMPTA | RC-05, Q-CG7 |
| CGA-263 | Suivre les versements du **forfait d'externat** (financeur, part, calendrier) | COMPTA · `tresorerie.gerer` | |
| CGA-264 | Prévoir la **taxe sur les salaires** (port 49) | COMPTA · `paie.lire_agrege` | RC-09 |
| CGA-265 | Public : suivi des crédits par service et domaine | GEST · `budget.lire` | |
| CGA-266 | Public : régie d'avances de séjour | régisseur · `tresorerie.gerer` | RC-22 |
| CGA-267 | Ouvrir un **accès temporaire en lecture** (cabinet, CAC, auditeur) ★ | DIR · `comptabilite.parametrer` | date de fin, journal |
| CGA-268 | Rapport des **délais de paiement** fournisseurs | COMPTA, DIR · `comptabilite.lire` | |
| CGA-269 | Justifier l'emploi d'une aide publique (compte rendu financier) | COMPTA · idem | analytique « projet » |

### 21.10 IA et système

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-270 | Lire une facture | IA (droit de l'utilisateur) | N0 / N2 |
| CGA-271 | Proposer une imputation | IA | N2 |
| CGA-272 | Détecter les anomalies (doublons, IBAN, montants, dérives, fractionnement) | IA, SYS | |
| CGA-273 | Expliquer un écart budgétaire | IA | lecture |
| CGA-274 | Préparer un brouillon de budget N+1 | IA | N2 |
| CGA-275 | Proposer des rapprochements bancaires | IA | N2 |
| CGA-276 | Répondre « combien reste-t-il sur… » | IA | lecture, portée de l'utilisateur |
| CGA-277 | Rédiger un brouillon de relance fournisseur | IA | l'envoi reste humain ★ |
| CGA-278 | Comptabiliser un fait (port, dans la transaction du fait) | SYS | idempotent |
| CGA-279 | Contrôles nocturnes (continuité, exhaustivité, chaînage, caisse non comptée, banque non pointée) | SYS | |
| CGA-280 | Archiver et purger selon les durées (§23.4) | SYS | après avis du DPO |
| CGA-281 | Journaliser les consultations sensibles (IBAN, balance âgée nominative, notes de frais) | SYS | E7 |
| CGA-282 | **File de travail** du comptable | COMPTA | tâches typées, socle 29 |
| CGA-283 | Notifications (validation attendue, anomalie, échéance) | SYS | natures `comptabilite.*`, `achats.*` (§24) |
| CGA-284 | Aide intégrée (glossaire, « pourquoi ce compte ? ») | tous | |

### 21.11 Reprise, doublon, sécurité (CG0, CG8)

| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| CGA-285 | Importer le plan de comptes du cabinet | COMPTA · `comptabilite.parametrer` | correspondance avec les schémas |
| CGA-286 | Importer des fournisseurs (CSV) | COMPTA · `fournisseurs.gerer` | IBAN importés **en attente** de validation (R-CG26) |
| CGA-287 | Importer les soldes d'ouverture (à-nouveaux) | COMPTA · `comptabilite.saisir` | équilibre exigé |
| CGA-288 | Importer un budget voté (tableur) | COMPTA · `budget.preparer` | |
| CGA-289 | Activer le **mode parallèle** (doublon) | DIR · `comptabilite.parametrer` | R-CG35 |
| CGA-290 | Comparer avec la balance de l'outil officiel | COMPTA · `comptabilite.lire` | |
| CGA-291 | **Passer en réel** (fin du parallèle) ★ | DIR · idem | décision de Florian (§31) |
| CGA-292 | Export de portabilité du domaine (D-046) | administrateur | double validation (D-041) |
| CGA-293 | Désigner un **suppléant** comptable ; procédures documentées | DIR · `personnels.gerer` | CCG-56 |
| CGA-294 | Revue annuelle des accès comptables | DIR | liste des droits par personne |
| CGA-295 | **Geler** les paiements (fraude suspectée) ★ | DIR · `paiements_fournisseurs.valider` | tout lot suspendu |
| CGA-296 | Neutraliser à la restauration les lots de virement et les envois en file | SYS | E9 |
| CGA-297 | Vérifier, après restauration, les exports par leur empreinte | SYS | §29 |
| CGA-298 | Retirer l'export historique « OGEC » (après avis du cabinet) | administrateur | KD-10 |
| CGA-299 | (réservé) | | |

---

## 22. Règles transverses (R-CG1 à R-CG42) [R]

| N° | Règle |
|---|---|
| R-CG1 | Une écriture naît d'un fait par un schéma déclaré, dans la transaction du fait ; idempotente par clé de fait. |
| R-CG2 | Le 48 ne recalcule jamais une pièce d'un autre socle ; une correction passe par le propriétaire (avoir au 24, opération au 25). |
| R-CG3 | Public : ScolIA ne tient pas la comptabilité de l'EPLE et ne produit aucune écriture publique ; il prépare et documente. |
| R-CG4 | Une entité comptable par établissement tant que les groupes (Q13) ne sont pas décidés ; aucune donnée d'un établissement visible d'un autre. |
| R-CG5 | Montants en centimes (`BIGINT`), jamais en flottant ; arrondi de TVA par ligne puis par taux, écart d'arrondi plafonné (CCG-11). |
| R-CG6 | Aucun numéro de compte codé en dur hors des schémas par défaut ; tout vient du plan de comptes de l'entité. |
| R-CG7 | Dates comptables en Europe/Paris. |
| R-CG8 | Exercice et périodes : aucune écriture dans une période close ; une pièce tardive se date dans la première période ouverte, sa date d'origine conservée. |
| R-CG9 | Une écriture d'origine automatique n'est jamais modifiée à la main ; elle se recalcule (avant validation) ou se contrepasse. |
| R-CG10 | Une écriture validée est immuable (numéro continu par journal et par exercice, date de validation, empreinte chaînée). |
| R-CG11 | Correction = contrepassation datée du jour + écriture juste, reliées. |
| R-CG12 | Aucune opération diverse sur un compte de tiers « famille ». |
| R-CG13 | Chaque écriture a une pièce (sauf à-nouveaux et contrepassations, qui pointent l'écriture d'origine). |
| R-CG14 | Une écriture n'est exportée qu'une fois ; un nouvel export exige l'annulation tracée du précédent ; le fichier exporté est conservé à l'identique. |
| R-CG15 | Le budget voté est figé ; tout changement est une nouvelle version votée. |
| R-CG16 | Une commande engage un crédit à sa création ; l'engagement se libère à la facture validée ou à l'annulation. |
| R-CG17 | Un dépassement de crédit n'est pas interdit, il est décidé : par qui, motif, date. |
| R-CG18 | Le disponible affiché à un demandeur se limite à ses enveloppes. |
| R-CG19 | Une demande, une commande, une facture validée ne se suppriment pas : elles s'annulent (état daté, motif). |
| R-CG20 | Le demandeur voit le statut de sa demande ; le prix seulement si le réglage le permet (S-4822). |
| R-CG21 | Aucun nom d'élève ni de famille dans un export comptable : tiers « familles » par numéro ; table de correspondance séparée, consultable avec `facturation.lire`. |
| R-CG22 | Aucun salaire individuel dans le 48 : le journal de paie arrive agrégé par rubrique et section ; le détail reste au 49 ou au logiciel de paie. |
| R-CG23 | Personne ne valide sa propre demande, sa propre OD au-delà du seuil, ni un paiement vers un fournisseur dont il a modifié l'IBAN. |
| R-CG24 | Tolérances de rapprochement explicites ; au-delà, décision humaine. |
| R-CG25 | Doublon de facture (fournisseur + numéro, ou montant + date proches) : blocage expliqué, levée motivée. |
| R-CG26 | Modification d'IBAN fournisseur : RIB, contre-appel à un numéro déjà connu, consigné, second valideur, carence ; paiements suspendus d'ici là ; IBAN imprimé différent sur une facture = blocage. |
| R-CG27 | Double validation d'un lot au-delà du seuil, et de tout paiement vers un fournisseur créé ou modifié dans les 30 jours ; réauthentification par passkey. |
| R-CG28 | Une facture n'est « payée » qu'au relevé ou sur attestation de remise. |
| R-CG29 | Un écart de caisse se constate et se valide ; jamais de compensation silencieuse. |
| R-CG30 | La lecture d'un relevé ne se fait jamais avec les identifiants bancaires d'un utilisateur. |
| R-CG31 | Coordonnées bancaires chiffrées (E11), affichées masquées, consultation journalisée. |
| R-CG32 | Public (et privé si activé) : compteur de besoin par famille homogène et par exercice ; alerte avant seuil ; jamais de blocage ni de décision automatique. |
| R-CG33 | Seuils réglementaires (commande publique, CAC, taxe sur les salaires) = paramètres de plateforme datés, avec leur source, jamais codés. |
| R-CG34 | Les dépenses d'un séjour ne mentionnent jamais un élève ou une famille. |
| R-CG35 | Mode parallèle (doublon) : écritures marquées « parallèle », exports « à blanc », jamais remis au cabinet comme définitifs sans décision. |
| R-CG36 | Accès d'un tiers (cabinet, CAC) : temporaire, en lecture, journalisé. |
| R-CG37 | Toute action comptable sensible est journalisée (`ent_journal_modif`, E7). |
| R-CG38 | Aucune donnée comptable nominative dans les journaux applicatifs (E14). |
| R-CG39 | Les factures fournisseurs peuvent contenir des données personnelles (salariés du fournisseur, élèves cités) : accès limité aux droits d'achat et de comptabilité. |
| R-CG40 | L'IA propose ; une lecture de facture n'est enregistrée qu'après relecture ; aucune validation par l'IA. |
| R-CG41 | Un engagement ou une écriture ne référence jamais une donnée de santé ou sociale (aide : numéro de décision anonymisé, 24 §10.5). |
| R-CG42 | Les fichiers de virement et les relevés sont des objets (E5), jamais des `BYTEA`, conservés selon §23.4. |

---

## 23. Droits et minimisation

### 23.1 Catalogue d'actions proposé (D-017)

| Droit | Contenu | Rôles par défaut |
|---|---|---|
| `comptabilite.lire` | brouillard, écritures, balances, journaux, tableaux | Compta, Direction |
| `comptabilite.saisir` | OD, imputations, lettrage, imports | Compta |
| `comptabilite.valider` ★ | valider, contrepasser, clôturer | Compta |
| `comptabilite.exporter` ★ | exports, annulation, re-téléchargement | Compta |
| `comptabilite.parametrer` | entité, plan, journaux, schémas, seuils, accès tiers | Compta, Direction (seuils) |
| `budget.lire` | budget, versions, réalisé | Direction, Compta, Bureau OGEC |
| `budget.preparer` | brouillons, versions, simulations | Direction, Compta |
| `budget.voter` ★ | enregistrer un vote, autoriser un dépassement | Direction |
| `budget.enveloppe.gerer` / `budget.enveloppe.virer` | enveloppes, membres / virements internes | Direction / responsable |
| `achats.demander` | créer et suivre ses demandes | tout personnel membre d'une enveloppe |
| `achats.valider` | valider les demandes de ses enveloppes | responsables, Direction |
| `achats.commander` | commandes, contrats, catalogue | Compta, gestionnaire |
| `achats.receptionner` | réceptions, service fait | Accueil, intendance, demandeur (les siennes) |
| `fournisseurs.gerer` / `fournisseurs.iban.valider` ★ | fiches / validation d'IBAN | Compta / Direction |
| `factures_fournisseurs.deposer` / `.gerer` / `.valider` ★ | dépôt / saisie, litiges / bon à payer | Accueil / Compta / Compta, Direction |
| `paiements_fournisseurs.preparer` / `.valider` ★ | lots / validation | Compta / Direction, Bureau OGEC |
| `tresorerie.lire` / `tresorerie.gerer` | prévisionnel / comptes, relevés, caisse | Direction, Bureau / Compta |
| `immobilisations.gerer` | biens en valeur | Compta |

**Retrait** de l'export comptable du droit `facturation.gerer` (KD-07). Le rôle « Administratif » (secrétariat) ne reçoit **aucun** droit comptable par défaut, sauf `factures_fournisseurs.deposer` et `achats.receptionner` (FD-27 du 24 par analogie).

### 23.2 Ce que voit chacun

- **Professeur** : ses demandes, le disponible de ses enveloppes, le statut de ses commandes ; jamais un IBAN, une balance, une autre enveloppe.
- **Responsable de pôle** : son enveloppe, les demandes de ses membres.
- **Bureau de l'OGEC** : budget, réalisé, trésorerie, lots à valider ; jamais de nom de famille, de salaire individuel.
- **Cabinet, CAC** : écritures, pièces, balances, en lecture, pour la durée de l'accès ; la table de correspondance des tiers familles seulement si nécessaire à un contrôle (sur décision de la direction, journalisé).
- **Organisateur de séjour** : le réalisé de son séjour.

### 23.3 Pièges à tester
Énumération des factures par identifiant ; disponible d'une autre enveloppe ; IBAN par l'API d'export ; lot validé par son préparateur ; validation après expiration d'une délégation ; export d'un autre établissement ; IA qui lit un journal de paie détaillé ; accès cabinet après sa date de fin ; table de correspondance tiers dans un export.

### 23.4 Conservation [R, durées à valider par le DPO]

| Donnée | Durée proposée | Fondement |
|---|---|---|
| Écritures, pièces comptables, exports, relevés, états de rapprochement | **10 ans** après la clôture de l'exercice | C. com. L123-22 [V] |
| Budgets et versions | 10 ans | pièces de gestion liées aux comptes |
| Demandes d'achat sans suite | 2 ans | [R] |
| Fiches fournisseurs sans mouvement | archivées 2 ans après le dernier mouvement ; coordonnées bancaires effacées à l'archivage si aucune pièce n'en dépend | minimisation |
| Notes de frais d'un personnel | 10 ans (pièces comptables) ; données personnelles minimales | |
| Journaux de consultation | 1 an | CNIL, recommandation journalisation (06 §… [V]) |

---

## 24. Événements et notifications

**Émis** (aucun montant nominatif, aucune donnée personnelle hors identifiants) : `comptabilite.ecriture_validee`, `.periode_close`, `.export_produit`, `.export_annule`, `.anomalie_detectee` ; `budget.version_votee`, `.seuil_atteint`, `.depassement_autorise` ; `achats.demande_soumise|validee|refusee`, `.commande_emise|annulee`, `.reception_constatee`, `.litige_ouvert` ; `fournisseurs.iban_demande|valide` ; `paiements_fournisseurs.lot_valide|remis` ; `tresorerie.point_bas_prevu`.
**Consommés** : `facturation.piece_emise`, `.avoir_emis`, `paiements.encaissement|remise|rejet|remboursement`, `aides.affectee|reversement`, `paie.journal_arrete` (49), `sorties.annulee|budget_modifie` (42), `personnel.engagement_clos`, `fonction.modifiee` (6), `bien.sorti` (44).
**Natures de notifications** (socle 8) : demande à valider (responsable), demande traitée (demandeur), commande livrée (demandeur), facture bloquée (comptable), IBAN à valider (direction), lot à valider (direction, bureau), seuil budgétaire (responsable, direction), point bas de trésorerie (direction), échéance de préavis d'un contrat (comptable). Aucune ne contient de montant sur l'écran verrouillé d'un téléphone (aperçu masqué).

---

## 25. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)

- **Comptable** : « Aujourd'hui » (file de travail typée : factures à relire, à rapprocher, bons à payer, lot à composer, banque à pointer, anomalies) ; tableaux denses (brouillard, écritures, factures, relevés) avec raccourcis clavier ; fiche facture en deux volets (pièce à gauche, saisie à droite, champs proposés surlignés) ; rapprochement bancaire en deux colonnes.
- **Direction** : tuiles Bento (disponible global, trésorerie à 90 jours, factures en retard, budget par pôle, anomalies, lots à valider) ; budget en tableau avec versions.
- **Professeur, responsable** : « Mes demandes », « Nouvelle demande » (formulaire court), « Mon enveloppe » ; **mobile** : créer une demande avec photo, valider les demandes de son pôle, réceptionner (scan du bon), consulter le disponible.
- **Bureau de l'OGEC** : lecture et validation de lots sur mobile, avec la liste des factures et leurs pièces.
- **Cabinet** : espace de lecture avec téléchargement des exports et des pièces.
- Vocabulaire du public dans le profil public.

---

## 26. Modèle de données cible (brique `comptabilite`) [R]

Règles : toutes les tables portent `etablissement_id UUID NOT NULL` (E1), FK composites `(etablissement_id, x_id)` (E2), UUID applicatifs, numéros lisibles par compteur par établissement (E3), `cree_le`/`modifie_le` (E6), états datés plutôt que `DELETE`, journal (E7), **aucune colonne JSON, aucun tableau** (E10, D-036), fichiers en objets (E5), RLS stricte (E13). Montants `BIGINT` en centimes. Pseudo-DDL abrégé.

```
ent_cg_entite (id, etablissement_id, raison_sociale, siren, regime CHECK IN ('prive_sous_contrat','prive_hors_contrat','public_eple','autre'),
               tva_regime CHECK IN ('non_recuperable','prorata','recuperable'), tva_prorata_pct NUMERIC(5,2) NULL,
               entite_facturation_id NULL -- reprise de 24 ent_fin_entite_facturation, puis fusion)
ent_cg_exercice (id, etablissement_id, entite_id, du, au, etat CHECK IN ('ouvert','en_cloture','clos'), clos_le, clos_par) -- EXCLUDE chevauchement
ent_cg_periode (id, etablissement_id, exercice_id, mois DATE, etat CHECK IN ('ouverte','close'), close_le, close_par)
ent_cg_compte (id, etablissement_id, entite_id, numero TEXT, libelle, classe SMALLINT, nature CHECK IN ('general','tiers_collectif'),
               actif BOOLEAN, sensible BOOLEAN) UNIQUE (etablissement_id, entite_id, numero)
ent_cg_journal (id, etablissement_id, entite_id, code, libelle, type CHECK IN ('ventes','achats','banque','caisse','od','paie','a_nouveaux','cabinet'),
                tresorerie_id NULL)
ent_cg_tiers (id, etablissement_id, entite_id, numero TEXT, type CHECK IN ('famille','fournisseur','client','personnel','financeur','autre'),
              compte_collectif_id, fin_compte_id NULL -- 24 ent_fin_compte (famille), fournisseur_id NULL, agent_id NULL, organisme_id NULL)
              -- CHECK : exactement une référence selon le type
ent_cg_compteur (etablissement_id, entite_id, cle TEXT, valeur BIGINT) PK (etablissement_id, entite_id, cle) -- journal×exercice, commandes, tiers
ent_cg_schema (id, etablissement_id, entite_id, type_fait TEXT, version INT, du DATE, au DATE NULL, valide_par, second_valideur)
ent_cg_schema_ligne (id, etablissement_id, schema_id, ordre, role_montant TEXT, sens CHECK IN ('D','C'), compte_id NULL, source_compte CHECK IN ('fixe','article','categorie_achat','tresorerie','tiers'), axe_id NULL)
ent_cg_ecriture (id, etablissement_id, entite_id, exercice_id, journal_id, etat CHECK IN ('brouillard','validee'),
                 numero BIGINT NULL, date_ecriture DATE, date_piece DATE, piece_ref TEXT, piece_objet_id NULL,
                 libelle, origine_type TEXT, origine_id UUID NULL, origine_version INT NULL, schema_id NULL,
                 contrepasse_ecriture_id NULL, parallele BOOLEAN, valide_le, valide_par, empreinte TEXT NULL, empreinte_precedente TEXT NULL)
                 UNIQUE (etablissement_id, journal_id, exercice_id, numero) ; UNIQUE (etablissement_id, origine_type, origine_id, origine_version)
ent_cg_ecriture_ligne (id, etablissement_id, ecriture_id, ordre, compte_id, tiers_id NULL, libelle, debit_centimes BIGINT, credit_centimes BIGINT,
                       lettrage_code NULL, lettre_le NULL, echeance DATE NULL, CHECK (debit=0 OR credit=0), CHECK (debit>=0 AND credit>=0))
ent_cg_ecriture_analytique (etablissement_id, ligne_id, section_id, part_pct NUMERIC(5,2) ou montant_centimes) -- une ligne par section
ent_cg_axe (id, etablissement_id, code, libelle, obligatoire BOOLEAN) ; ent_cg_section (id, etablissement_id, axe_id, code, libelle, source_type NULL, source_id NULL, actif)
ent_cg_cle_repartition (id, etablissement_id, libelle, base CHECK IN ('surface','effectif','heures','manuelle')) ; ent_cg_cle_part (cle_id, section_id, part)
ent_cg_budget (id, etablissement_id, entite_id, exercice_id) ; ent_cg_budget_version (id, budget_id, numero, nature CHECK IN ('travail','propose','vote','rectificatif','simulation'),
               vote_le, instance_ref, deliberation_id NULL, fige BOOLEAN)
ent_cg_budget_ligne (id, etablissement_id, version_id, compte_id NULL, poste_code NULL, section_id NULL, montant_centimes, commentaire)
ent_cg_enveloppe (id, etablissement_id, exercice_id, libelle, section_id, responsable_fonction_id NULL, responsable_agent_id NULL, plafond_centimes, depassement CHECK IN ('interdit','signale'))
ent_cg_enveloppe_membre (etablissement_id, enveloppe_id, agent_id NULL, fonction_id NULL, du, au)
ent_cg_engagement (id, etablissement_id, enveloppe_id, budget_ligne_id NULL, source_type, source_id, montant_centimes, etat CHECK IN ('actif','libere','consomme'), du_le)
ent_cg_virement_credit (id, etablissement_id, enveloppe_id, de_ligne_id, vers_ligne_id, montant_centimes, motif, par)
ent_cg_depassement (id, etablissement_id, enveloppe_id, demande_id NULL, montant_centimes, motif, autorise_par, autorise_le)
ent_cg_fournisseur (id, etablissement_id, entite_id, numero, raison_sociale, siren NULL, siret NULL, tva_intra NULL, conditions_jours SMALLINT, fin_de_mois BOOLEAN,
                    categorie_achat_id NULL, etat CHECK IN ('actif','bloque','archive'), fusionne_dans_id NULL)
ent_cg_fournisseur_contact (id, etablissement_id, fournisseur_id, nom, fonction, email, telephone, telephone_verifie_le NULL)
ent_cg_fournisseur_banque (id, etablissement_id, fournisseur_id, iban_chiffre, iban_cle_id, iban_masque, bic, titulaire, rib_objet_id,
                           etat CHECK IN ('en_attente','valide','remplace','refuse'), demande_par, contre_appel_numero, contre_appel_le, contre_appel_par,
                           valide_par NULL, valide_le NULL, utilisable_a_partir TIMESTAMPTZ)
ent_cg_fournisseur_vigilance (id, etablissement_id, fournisseur_id, objet_id, delivree_le, expire_le)
ent_cg_categorie_achat (id, etablissement_id, code, libelle, compte_charge_id, famille_ccp_code NULL, immobilisable BOOLEAN)
ent_cg_catalogue_article (id, etablissement_id, libelle, fournisseur_id, prix_centimes, categorie_achat_id, actif)
ent_cg_demande (id, etablissement_id, numero, demandeur_agent_id, enveloppe_id, exercice_id, objet, date_souhaitee, lieu_livraison,
                etat CHECK IN ('brouillon','soumise','a_completer','validee','refusee','commandee','close','annulee'), motif_refus NULL, depassement BOOLEAN)
ent_cg_demande_ligne (id, etablissement_id, demande_id, libelle, quantite NUMERIC(10,3), prix_estime_centimes, fournisseur_id NULL, fournisseur_suggere TEXT NULL, catalogue_article_id NULL)
ent_cg_validation (id, etablissement_id, objet_type, objet_id, niveau CHECK IN ('responsable','direction','comptable','second'), decision CHECK IN ('valide','refuse','a_completer'),
                   par_agent_id, par_delegation_id NULL, le, motif NULL)
ent_cg_commande (id, etablissement_id, numero, fournisseur_id, etat CHECK IN ('preparee','emise','partiellement_recue','recue','soldee','annulee'), emise_le, document_objet_id, reference_site NULL, contrat_id NULL)
ent_cg_commande_ligne (id, etablissement_id, commande_id, demande_ligne_id NULL, libelle, quantite, prix_unitaire_centimes, tva_taux_pm INT, compte_id, section_id)
ent_cg_contrat (id, etablissement_id, fournisseur_id, objet, du, au NULL, reconduction_tacite BOOLEAN, preavis_jours SMALLINT, montant_periodique_centimes, periodicite CHECK IN ('mensuelle','trimestrielle','annuelle'), resilie_le NULL)
ent_cg_reception (id, etablissement_id, commande_id, recue_le, par_agent_id, bon_livraison_objet_id NULL, nature CHECK IN ('livraison','service_fait'))
ent_cg_reception_ligne (id, etablissement_id, reception_id, commande_ligne_id, quantite_recue, ecart CHECK IN ('aucun','manquant','abime','non_conforme'), commentaire NULL)
ent_cg_facture_four (id, etablissement_id, fournisseur_id, numero_fournisseur, date_facture, echeance, total_ht_centimes, total_tva_centimes, total_ttc_centimes,
                     source CHECK IN ('plateforme','courriel','scan','manuel'), format CHECK IN ('facturx','ubl','cii','pdf','papier'), objet_id, iban_imprime_masque NULL,
                     etat CHECK IN ('recue','a_relire','a_rapprocher','rapprochee','bon_a_payer','en_lot','payee','litige','annulee'), avoir_de_id NULL)
                     UNIQUE (etablissement_id, fournisseur_id, numero_fournisseur) -- doublon (R-CG25)
ent_cg_facture_four_ligne (id, etablissement_id, facture_id, libelle, quantite, prix_unitaire_centimes, tva_taux_pm INT, compte_id NULL, section_id NULL, commande_ligne_id NULL)
ent_cg_rapprochement_achat (etablissement_id, facture_ligne_id, commande_ligne_id, reception_ligne_id NULL, ecart_centimes, accepte_par NULL)
ent_cg_litige (id, etablissement_id, facture_id, motif CHECK IN (...), ouvert_le, clos_le NULL, issue NULL)
ent_cg_note_frais (id, etablissement_id, agent_id, etat, total_centimes) ; ent_cg_note_frais_ligne (…, objet_id)
ent_cg_lot_virement (id, etablissement_id, numero, message_id TEXT UNIQUE, etat CHECK IN ('compose','valide','genere','remis','rapproche','annule'), fichier_objet_id, valide_par, second_valideur, remis_le)
ent_cg_paiement (id, etablissement_id, facture_id NULL, note_frais_id NULL, lot_id NULL, moyen CHECK IN ('virement','cheque','prelevement','carte','especes'), montant_centimes, date, reference)
ent_cg_tresorerie (id, etablissement_id, entite_id, type CHECK IN ('banque','caisse','regie','carte','livret'), libelle, iban_chiffre NULL, iban_cle_id NULL, compte_id)
ent_cg_releve (id, etablissement_id, tresorerie_id, du, au, solde_debut_centimes, solde_fin_centimes, format, objet_id, empreinte UNIQUE)
ent_cg_releve_ligne (id, etablissement_id, releve_id, date_operation, date_valeur, libelle, reference, montant_centimes)
ent_cg_rapprochement_banque (etablissement_id, releve_ligne_id, ecriture_ligne_id, par, le)
ent_cg_caisse_comptage (id, etablissement_id, tresorerie_id, le, par, total_compte_centimes, solde_theorique_centimes, ecart_centimes, valide_par NULL)
ent_cg_caisse_comptage_ligne (etablissement_id, comptage_id, valeur_faciale_centimes, nombre)
ent_cg_immobilisation (id, etablissement_id, entite_id, numero, libelle, bien_id NULL -- socle 44, categorie, mise_en_service DATE, cout_centimes, mode CHECK IN ('lineaire','degressif','non_amorti'),
                       duree_mois, section_id, facture_ligne_id NULL, subvention_id NULL, sortie_le NULL, sortie_motif NULL)
ent_cg_amortissement (id, etablissement_id, immobilisation_id, exercice_id, dotation_centimes, ecriture_id NULL)
ent_cg_subvention (id, etablissement_id, financeur_tiers_id, objet, montant_centimes, nature CHECK IN ('fonctionnement','investissement'), publique BOOLEAN, notifiee_le, recue_centimes)
ent_cg_fonds_dedie (id, etablissement_id, subvention_id, exercice_id, report_centimes, emploi_centimes)
ent_cg_export (id, etablissement_id, entite_id, numero, du, au, format CHECK IN ('fec','scolia_v1','editeur_x'), nb_ecritures, total_debit, total_credit, empreinte, fichier_objet_id,
               parallele BOOLEAN, annule_le NULL, annule_par NULL, annule_motif NULL) -- remplace ent_fin_export
ent_cg_export_ecriture (etablissement_id, export_id, ecriture_id) UNIQUE (etablissement_id, ecriture_id) WHERE export non annulé
ent_cg_import_cabinet (id, etablissement_id, fichier_objet_id, nature CHECK IN ('a_nouveaux','inventaire','plan','autre'), nb_ecritures, importe_par, empreinte)
ent_cg_acces_tiers (id, etablissement_id, user_id, organisme, du, au, accorde_par)
ent_cg_compteur_ccp (etablissement_id, exercice_id, famille_code, montant_ht_centimes) -- calculé, reconstructible
```

**Registre (E12)** : toutes les tables `ent_cg_*` en portée `etablissement` ; catégorie d'export **double** (données financières : double validation de l'export complet, D-041) ; `ent_cg_lot_virement` en état « généré » et la tâche de récupération auprès de la plateforme (CGA-203) **neutralisées à la restauration** (E9) ; `ent_cg_compteur_ccp` reconstructible. **Clés** (E11) : `iban_chiffre` des fournisseurs et des comptes de trésorerie avec `iban_cle_id`, inscrites à l'inventaire des clés. **Exceptions D-036 : aucune.**

---

## 27. Reprise des données

- **Next.js (M15, ébauche)** : **rien n'est repris** par défaut (T0 : « vide probable ») ; si Florian signale des dépenses réelles de séjour dans `depense` (`travel_id`), elles sont reprises comme **réalisé** du séjour (42) avec la pièce, pas comme écritures (Q-CG10).
- **ENT Rust** : `ent_fin_export` et `ent_fin_export_piece` → `ent_cg_export` (format `scolia_v1`, `fichier_objet_id` absent pour l'historique, marqué « fichier non conservé ») ; les pièces déjà exportées gardent leur marque (aucune réexportation) ; `ent_fin_parametres` (comptes, journaux) → plan et journaux de l'entité.
- **Outil comptable actuel de l'OGEC** (Charlemagne Comptabilité, Sage ou cabinet) : plan de comptes (CGA-285), fournisseurs (CGA-286, IBAN **en attente**), soldes d'ouverture (CGA-287), budget voté (CGA-288) ; **jamais** l'historique des écritures (il reste dans l'outil d'origine, consultable par le cabinet).
- Tests sur données **fictives** uniquement.

---

## 28. Lots et migrations

### 28.1 Lots (équipe Sonnet)

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **CG0** correctifs de l'existant | KD-01 à KD-10 : fichier d'export en objet et re-téléchargement ; export des rejets ; numérotation continue persistée ; comptes par moyen ; tiers lisibles ; fuseau ; droit `comptabilite.exporter` ; libellés ; `RESTRICT` ; export historique tracé | 24 F0 | tests : export → rejet → export ; re-téléchargement identique ; continuité sur deux exports ; encaissement 0 h 30 |
| **CG1** moteur d'écritures | entité, exercice, périodes, plan ANC, journaux, tiers, schémas versionnés, brouillard, validation, contrepassation, OD, contrôles nocturnes, port `comptabiliser` ; le 24 et le 25 y branchent leurs faits | CG0 | équilibre, continuité, exhaustivité, idempotence, immuabilité (test d'`UPDATE` refusé) |
| **CG2** budget | budget, versions, vote, enveloppes, engagements, disponible, alertes, simulation, présentation CA | CG1 ; 6 (fonctions) | disponible juste sous concurrence ; version votée immuable |
| **CG3** demandes | demandes, catalogue, circuit, délégations, mobile | CG2 ; 6 | jamais sa propre demande ; délégation expirée refusée |
| **CG4** commandes et réceptions | commandes, contrats, reconduction, réceptions, service fait, régie d'avances | CG3 ; 7 | engagement ajusté ; reliquats |
| **CG5** fournisseurs | fiches, IBAN avec contre-appel et second valideur, vigilance, blocage, fusion | CG1 | piège « préparateur = valideur » ; carence |
| **CG6** factures fournisseurs | dépôt, lecture IA, rapprochement 3 voies, doublons, litiges, avoirs, bon à payer, notes de frais | CG4, CG5 ; 7 ; IA_COEUR | 200 factures fictives ; doublons |
| **CG7** paiements et trésorerie | lots, pain.001, double validation, relevés, rapprochement, caisse, prévisionnel | CG6 | lot non remis jamais « payé » ; rapprochement 95 % sur jeu fictif |
| **CG8** exports et cabinet | FEC (contrôles A47 A-1), formats d'éditeur (Q-CG3), import cabinet et à-nouveaux, annulation d'export, mode parallèle | CG1 | FEC validé par l'outil de contrôle de la DGFiP « Test Compta Demat » [NV : disponibilité] |
| **CG9** états de contrôle | balance, grand livre, journal, lettrage, balances âgées, rapprochement de balances | CG8 | |
| **CG10** immobilisations | biens, plans, dotations proposées, sorties, subventions d'investissement, fonds dédiés | CG1 ; 44 | |
| **CG11** analytique et rapports | clés, coûts de revient, synthèses séjour et APEL, emploi des contributions, tableaux, 47 | CG2, CG10 | |
| **CG12** public | pré-engagements, export Op@le, compteur commande publique, crédits par service, régie d'avances | CG3, CG4 | aucune écriture publique |
| **CG13** tenue complète (optionnel) | inventaire, clôture, à-nouveaux, états ANC 2018-06 | CG9, CG10 ; **Q-CG1** | non engagé |
| **CG14** plateforme agréée | raccordement, récupération, statuts | CG6 ; **Q-CG4** (contrat) | |

**Parcours minimal pour l'année de doublon** : CG0 → CG8 (dont CG6 et CG7), en **mode parallèle** pour les écritures ; **en réel** possible pour les demandes d'achat, commandes et réceptions si Florian le décide (Q-CG11 : elles ne sont pas dans Charlemagne, ou mal [H]).

### 28.2 Migrations (plage imposée : **2960–2979**)

| N° | Contenu | Lot |
|---|---|---|
| 2960 | CG0 : `ent_fin_export.fichier_objet_id`, `annule_*`, compteur persisté, tiers lisibles, `RESTRICT` sur `ent_fin_export_piece` | CG0 |
| 2961 | entité, exercices, périodes | CG1 |
| 2962 | plan de comptes, journaux, tiers, compteurs | CG1 |
| 2963 | schémas et lignes | CG1 |
| 2964 | écritures, lignes, analytique des lignes, garde d'immuabilité (déclencheur refusant `UPDATE`/`DELETE` d'une écriture validée) | CG1 |
| 2965 | axes, sections, clés de répartition | CG1, CG11 |
| 2966 | budgets, versions, lignes, enveloppes, membres, engagements, virements, dépassements | CG2 |
| 2967 | fournisseurs, contacts, banque, vigilance, catégories d'achat | CG5 |
| 2968 | demandes, lignes, validations, catalogue | CG3 |
| 2969 | commandes, lignes, contrats | CG4 |
| 2970 | réceptions | CG4 |
| 2971 | factures fournisseurs, lignes, rapprochements, litiges, notes de frais | CG6 |
| 2972 | lots de virement, paiements | CG7 |
| 2973 | trésorerie, relevés, rapprochements, caisse | CG7 |
| 2974 | immobilisations, amortissements, subventions, fonds dédiés | CG10 |
| 2975 | exports `ent_cg_export*`, imports cabinet, accès tiers ; reprise de `ent_fin_export` | CG8 |
| 2976 | public : compteur commande publique, régie d'avances | CG12 |
| 2977 | RLS stricte de toutes les tables `ent_cg_*`, registre E12 | CG1 → CG12 (complétée à chaque lot) |
| 2978 | réserve | |
| 2979 | retrait de `ent_fin_export(_piece)` après reprise et durée d'archive (**sur décision**, E8) | — |

Note : 2960–2979 tombe dans le « noyau 1000–2999 » de la proposition Q-B2 d'`ARCHITECTURE_BRIQUES.md` (non adoptée) ; plage imposée par le pilote, consignée dans `MIGRATIONS.md`.

---

## 29. Tests et recette

- **Propriétés** : toute écriture validée est équilibrée ; la numérotation d'un journal est continue ; un fait produit une et une seule écriture (rejeu) ; une écriture validée ne change pas (tentative d'`UPDATE` refusée par la base) ; aucune pièce exportée deux fois sans annulation.
- **Isolation** : chaque route de la brique dans la matrice d'isolation entre établissements ; RLS stricte.
- **Séparation des tâches** : matrice « qui prépare ne valide pas » sur demandes, IBAN, lots, OD.
- **Restauration** : PITR à J-1 puis démarrage : contrôles de continuité et d'exhaustivité verts ou anomalies listées ; lots « générés » neutralisés ; exports retrouvés par empreinte.
- **FEC** : contrôle de format par l'outil public de la DGFiP si disponible [NV], sinon par un validateur maison des règles A47 A-1.
- **Recette avec l'expert-comptable** de l'OGEC pilote : un mois fictif complet importé dans son logiciel sans retouche.
- **Jeu IA** : 200 factures fictives ; exactitude champ par champ ; aucune action non autorisée.

---

## 30. Réglages signalés (D-023) — S-4800 à S-4849

| N° | Réglage | Défaut |
|---|---|---|
| S-4800 | Début d'exercice | 1er septembre (privé), 1er janvier (public) |
| S-4801 | Plan de comptes de départ | ANC 2018-06 par défaut ; import du cabinet |
| S-4802 | Format d'export | FEC + un format d'éditeur (Q-CG3) |
| S-4803 | Seuil de double validation d'une OD | 1 500 € |
| S-4804 | Validation des écritures automatiques | à la clôture de période du domaine |
| S-4805 | Écart d'arrondi maximal | 1 € par écriture |
| S-4806 | Alertes de consommation budgétaire | 80 % et 100 % |
| S-4807 | Délai avant remontée d'une demande non validée | 5 jours ouvrés |
| S-4808 | Seuil « deux devis » | 2 000 € TTC (privé) ; selon la politique d'achat (public) |
| S-4809 | Seuil de validation par la direction | 1 000 € TTC |
| S-4810 | Écart de prix toléré commande / facture | 5 % ou 20 € |
| S-4811 | Tolérance de rapprochement automatique | 1 % ou 5 € |
| S-4812 | Compteur commande publique | activé (public), désactivé (privé) |
| S-4813 | Horizon de proposition des lots | 7 jours |
| S-4814 | Seuil de double validation d'un lot | 5 000 € |
| S-4815 | Plafond d'une dépense en espèces | 150 € [NV] |
| S-4816 | Carence après changement d'IBAN | 3 jours ouvrés |
| S-4817 | Point bas de trésorerie | 1 mois de masse salariale |
| S-4818 | Seuil d'immobilisation | 500 € HT [NV] |
| S-4819 | Nombre maximal d'axes analytiques | 5 |
| S-4820 | Plafond des enveloppes sans budget voté | 25 % de l'année précédente |
| S-4821 | Dépassement de l'enveloppe restauration | signalé, non bloquant |
| S-4822 | Prix visible du demandeur | oui |

---

## 31. Décisions appliquées par défaut (D-031) et questions à Florian

### 31.1 Appliquées par défaut (D-031)

| # | Question | Décision appliquée |
|---|---|---|
| Q-CG2 | Le 48 possède-t-il les écritures du 24 et du 25 ? | **Oui** : le 24 et le 25 émettent des faits ; le 48 les comptabilise par schéma ; l'export du 24 (F9) est déplacé dans le 48 (CG1, CG8). |
| Q-CG12 | Fournisseur = `ent_organisme` du 4 ? | **Non**, tiers comptable propre, lien optionnel. |
| Q-CG13 | Validation des écritures automatiques | À la clôture de période du domaine (S-4804). |
| Q-CG14 | Plusieurs établissements d'un même OGEC | Une entité par établissement jusqu'à Q13 (R-CG4). |
| Q-CG15 | Lecture des relevés | Import de fichiers ; jamais d'identifiants bancaires d'utilisateur (R-CG30). |
| Q-CG16 | Anti-fraude IBAN | Contre-appel consigné, second valideur, carence, suspension (R-CG26). |
| Q-CG17 | Export historique « OGEC » | Tracé tout de suite (CG0), retiré après avis du cabinet. |
| Q-CG18 | Public | Pas de comptabilité publique dans ScolIA (R-CG3). |
| Q-CG19 | IA | Lecture et propositions seulement ; aucune validation (R-CG40). |
| Q-CG20 | Mode parallèle pendant le doublon | Oui (R-CG35). |

### 31.2 Soumis à Florian (engagements extérieurs, données réelles, contradiction de décision)

| # | Question | Recommandation |
|---|---|---|
| **Q-CG1** | **Périmètre** : option 2 (ScolIA fait la gestion et les écritures, le logiciel du cabinet tient les comptes) ou option 1 (tenue légale complète, CG13) ? **Touche D-025.** | **Option 2**, CG13 en option (§3.3). |
| Q-CG3 | Quel **logiciel** et quel **expert-comptable** pour l'OGEC pilote ? Plan de comptes, format d'import, nomenclature UROGEC | à demander au cabinet ; FEC par défaut |
| Q-CG4 | **Plateforme agréée** de réception des factures (choix, contrat, coût, API) — l'échéance du 1-9-2026 est passée | à vérifier avec l'expert-comptable **cette semaine** (CCG-60) |
| Q-CG5 | **Banque** : format des relevés, pain.001, éventuel accès par API ou EBICS | fichiers d'abord |
| Q-CG6 | TVA de l'OGEC (non récupérable, prorata) | avis du cabinet |
| Q-CG7 | Qualification du **forfait d'externat** (subvention publique ou non : seuil du CAC) | avis du cabinet ou du CAC |
| Q-CG8 | L'OGEC est-il **pouvoir adjudicateur** (CCP L1211-1) ? | avis d'un juriste ; compteur désactivé d'ici là |
| Q-CG9 | Public : format d'import Op@le, académie partenaire | après D-029 |
| Q-CG10 | Des dépenses **réelles** sont-elles dans l'ébauche Next.js (`depense`) ? | sinon rien n'est repris |
| Q-CG11 | Pendant le doublon, les **achats** (demandes, commandes) peuvent-ils être en **réel** dans ScolIA ? | oui si Charlemagne ne les fait pas ou mal ; à confirmer |
| Q-CG21 | Durées de conservation (§23.4) | DPO |

---

## 32. Faits, hypothèses, contradictions

**Faits [F]** : code Rust relu le 2026-11-11 sur `ent-socle-48-49-conception` (`37ce0d3b`) : `finances_sepa.rs:1388-1615`, `finances_calcul.rs:323-363`, `205_fin_export_comptable.sql`, `200_fin_parametres_tarifs.sql`, `facturation_ogec.rs`, `service.rs:19413-19418`, `ent-auth/src/actions.rs`. Next.js lu sur `origin/main` : `drizzle/0053_compta_etablissement.sql`, `app/lib/compta-etablissement-db.ts` (en-têtes), `app/api/compta/route.ts`, `app/api/compta/export/route.ts`, `app/lib/paie-etablissement-db.ts:225-247`. Rien n'a été compilé ni exécuté.
**Hypothèses [H]** : pratiques des OGEC (exercice sept.–août, externalisation de la tenue au cabinet, comptable seul), fréquence de la fraude à l'IBAN, usage de Charlemagne Comptabilité dans l'établissement pilote.
**Contradictions signalées**
- **D-025** dit « comptabilité construite dans l'ENT Rust » ; la recommandation (§3) construit la gestion et les écritures et **remet la tenue légale** au logiciel du cabinet : Q-CG1.
- Le 24 (§6.5, F9) se dit producteur des écritures de ventes ; ici le 48 les **possède** (Q-CG2, appliqué) : le lot F9 du 24 devient « émettre les faits » et son export passe au CG8.
- `docs/ent-metier-checklist.md` classe encore « Compta OGEC » en « DROP » (déjà signalé par `SOCLES.md` §9) : à mettre à jour (non modifié ici).
- La plage 2960–2979 est dans le « noyau » de la proposition Q-B2 (non adoptée) : imposée par le pilote.

---

## 33. Ajout personas (2026-11-11, `PERSONAS_48.md`) — l'emporte en cas de contradiction

### 33.1 Régimes pendant l'année de doublon (bloquant A, R-CG43)

| Fonction | Régime 2027-2028 | Détail |
|---|---|---|
| Budget, enveloppes, demandes, commandes, réceptions | **réel** (après trois mois de preuves, Q-CG11) | engagements et disponible faits dans ScolIA |
| Factures fournisseurs, bons à payer, paiements fournisseurs | **réel** | écritures exportées au cabinet dans les journaux **`HA-S`** et **`BQ-S`** |
| Facturation et encaissements des familles | hors ScolIA (Charlemagne, D-034) | 24 F14 ; le 48 n'a pas de ventes |
| Réalisé des produits dans le budget | **lu** dans la balance mensuelle importée du cabinet | CGA-300 |
| Rapprochement bancaire, caisse | **parallèle** (l'outil actuel fait foi) | ScolIA rapproche ses seuls paiements |
| Tenue légale | cabinet | inchangé |

Le régime est un **tableau signé** par l'établissement (document du 7), affiché dans l'écran d'administration du domaine ; changer de régime est une décision de la direction (CGA-301), jamais automatique.

### 33.2 Règles ajoutées (R-CG43 à R-CG54)

| N° | Règle |
|---|---|
| R-CG43 | Pendant le doublon, les écritures produites en réel par ScolIA vont dans des journaux propres (`HA-S`, `BQ-S`) ; jamais dans un journal tenu aussi par l'outil actuel. |
| R-CG44 | L'export peut être **détaillé** ou **centralisé** (jour ou mois, par compte et section) par journal ; la pièce reste consultable dans ScolIA. |
| R-CG45 | À la mise en service : au moins deux valideurs de lots et d'IBAN (direction, président, trésorier) ; sinon le domaine paiements reste désactivé. |
| R-CG46 | Les demandes, dépenses et notes de frais du chef d'établissement sont validées par le président ou le trésorier. |
| R-CG47 | Empreinte HMAC des IBAN (fournisseurs, salariés, payeurs) ; une collision entre catégories déclenche une alerte à la direction et au trésorier sans révéler l'IBAN ni la personne. |
| R-CG48 | Reprise : un IBAN fournisseur payé sans incident dans les 12 derniers mois par l'outil d'origine peut être validé par attestation signée de la direction ; les autres suivent R-CG26. |
| R-CG49 | Pas de validation au-delà du seuil direction sans affichage de la pièce. |
| R-CG50 | Les écritures importées du cabinet ne sont jamais réexportées. |
| R-CG51 | Une facture récapitulative se rapproche de plusieurs réceptions ; l'écart total reste soumis aux tolérances. |
| R-CG52 | Montants en devise : montant d'origine, devise, taux et source du taux conservés ; écart de change en 666 / 766. |
| R-CG53 | Déclaration d'intérêts des administrateurs et dirigeants ; une commande à un fournisseur déclaré est signalée et listée pour le CAC. |
| R-CG54 | Les notes de frais ne sont visibles que du comptable et du valideur ; la direction voit des totaux par personne. |

### 33.3 Actions ajoutées (CGA-300 à CGA-329)

| N° | Action | Acteur · droit | Lot |
|---|---|---|---|
| CGA-300 | Importer la **balance mensuelle** du cabinet pour le réalisé du budget | COMPTA · `comptabilite.saisir` | CG8b |
| CGA-301 | Déclarer le **régime** d'une fonction (réel, parallèle, hors ScolIA) ★ | DIR · `comptabilite.parametrer` | CG8b |
| CGA-302 | Paramétrer la **correspondance de comptes et de journaux** ScolIA ↔ cabinet | COMPTA · idem | CG8 |
| CGA-303 | Choisir l'export détaillé ou centralisé par journal | COMPTA · idem | CG8 |
| CGA-304 | Déclarer la **chaîne de suppléance** des valideurs | DIR · `comptabilite.parametrer` | CG5 |
| CGA-305 | **Revue a posteriori** mensuelle (structure réduite), liste signée par le trésorier ★ | OGEC | CG7 |
| CGA-306 | Valider une demande du chef d'établissement | OGEC · `achats.valider` | CG3 |
| CGA-307 | Calculer les empreintes HMAC des IBAN | SYS | CG5 |
| CGA-308 | Alerte de **collision d'IBAN** | SYS → DIR, OGEC | CG5 |
| CGA-309 | Lever une alerte de collision (motif) ★ | DIR + OGEC | CG5 |
| CGA-310 | **Reprise attestée** des IBAN fournisseurs | DIR | CG5 |
| CGA-311 | Alerte « facture attendue non reçue » (contrat) | SYS → COMPTA | CG4 |
| CGA-312 | Carnet de procédures lié aux tâches | COMPTA | CG1 |
| CGA-313 | Archive des pièces d'une période pour le cabinet | COMPTA, CAB | CG8 |
| CGA-314 | **Demande partagée** d'une discipline | PERS, coordinateur | CG3 |
| CGA-315 | Importer un panier ou un devis en lignes de demande | PERS ; IA | CG3 |
| CGA-316 | Réception en deux temps (colis par l'accueil, conformité par le demandeur) | ACC, PERS | CG4 |
| CGA-317 | **Marché de denrées** : produits, mercuriale, commande quotidienne, facture récapitulative | chef cuisinier, COMPTA | CG4, CG6 |
| CGA-318 | Colis non identifié (photo, file) | ACC | CG4 |
| CGA-319 | Cote de l'original papier d'une facture | ACC, COMPTA | CG6 |
| CGA-320 | Saisir une dépense ou une avance **en devise** | régisseur, COMPTA | CG7 |
| CGA-321 | Imputation publique (codes Op@le locaux) sur une demande | GEST | CG12 |
| CGA-322 | PDF du constat de service fait pour l'engagement Op@le | GEST | CG12 |
| CGA-323 | Tenir la **déclaration d'intérêts** et la liste pour le CAC | OGEC, DIR | CG5 |
| CGA-324 | **Avis de paiement** au fournisseur | SYS | CG7 |
| CGA-325 | **Rapport des validations** et des exceptions | CAB, DIR | CG9 |
| CGA-326 | Fiche de traitement RGPD du domaine | DPO, administrateur | CG1 |
| CGA-327 | Bandeau anti-fraude sur création de fournisseur ou lot hors calendrier | SYS | CG5 |
| CGA-328 | Quarantaine des factures d'expéditeurs inconnus | SYS → COMPTA | CG6 |
| CGA-329 | Avis à tous les valideurs à chaque validation de lot | SYS | CG7 |

### 33.4 Cas limites ajoutés (CCG-61 à CCG-78)

| # | Cas | Traitement |
|---|---|---|
| CCG-61 | Écriture tardive après le verrou du cabinet | datée en N+1, alerte au comptable et au cabinet |
| CCG-62 | Le cabinet change de plan de comptes | nouvelle version de la correspondance, datée ; exports antérieurs inchangés |
| CCG-63 | Centralisation : une facture est corrigée après l'export centralisé du jour | contrepassation dans la centralisation suivante |
| CCG-64 | Le trésorier est le seul valideur disponible et c'est lui qui a préparé | interdit ; lot reporté ; en structure réduite : validation simple + revue a posteriori |
| CCG-65 | IBAN d'un fournisseur identique à celui d'une famille payeuse (parent artisan) | collision signalée ; levée motivée (cas légitime) |
| CCG-66 | 200 fournisseurs repris dont 30 sans paiement depuis 12 mois | 170 par attestation, 30 par contre-appel (R-CG48) |
| CCG-67 | Livraison de denrées refusée à la porte | réception avec écart « refusé » ; avoir attendu sur la récapitulative |
| CCG-68 | Prix de la mercuriale changé en cours de semaine | prix de la commande du jour ; écart signalé sur la récapitulative |
| CCG-69 | Taux de change de la carte bancaire différent du taux de l'avance | taux réel du relevé ; écart de change |
| CCG-70 | Le fournisseur est l'entreprise du conjoint d'un administrateur | déclaré : signalement, liste CAC ; non déclaré : rien à détecter automatiquement (limite connue) |
| CCG-71 | Facture reçue d'une adresse inconnue mais vraie | quarantaine ; validation de l'expéditeur par le comptable, ajout aux expéditeurs connus |
| CCG-72 | Lot généré, restauration de la base | lot « à vérifier », jamais « à remettre » |
| CCG-73 | Demande partagée dont un membre quitte l'établissement | ses lignes restent, attribuées à la coordinatrice |
| CCG-74 | Le chef d'établissement est aussi régisseur d'un séjour | ses dépenses validées par le président (R-CG46) |
| CCG-75 | Un administrateur démissionne | sa déclaration d'intérêts est close à la date, l'historique gardé |
| CCG-76 | Balance du cabinet importée avec un écart d'arrondi | écart affiché, pas de correction automatique |
| CCG-77 | Pendant le doublon, un paiement fournisseur de ScolIA apparaît dans le rapprochement de l'outil actuel sans écriture | le comptable l'y rapproche avec la référence du journal `BQ-S` (la référence est imprimée dans le libellé du virement) |
| CCG-78 | Valideur en voyage sans réseau le jour du lot | le suivant de la chaîne de suppléance est sollicité après S-4828 heures |

### 33.5 Réglages ajoutés (S-4823 à S-4832)

| N° | Réglage | Défaut |
|---|---|---|
| S-4823 | Export des ventes et encaissements familles | centralisé par jour |
| S-4824 | Auxiliaires familles à l'export | non (411 collectif), table de correspondance disponible |
| S-4825 | Mode structure réduite | désactivé ; activable avec revue a posteriori |
| S-4826 | Calendrier des lots | 2 par mois |
| S-4827 | Avis de paiement aux fournisseurs | oui |
| S-4828 | Délai avant sollicitation du valideur suivant | 24 h |
| S-4829 | Expéditeurs de factures acceptés | contacts connus des fournisseurs |
| S-4830 | Écart de change | au taux réel du relevé |
| S-4831 | Seuil d'affichage obligatoire de la pièce avant validation | seuil direction (S-4809) |
| S-4832 | Revue a posteriori | mensuelle |

### 33.6 Lots révisés
- **CG8b** (nouveau, entre CG8 et CG9) : régimes du doublon, journaux `HA-S` et `BQ-S`, import de la balance du cabinet (CGA-300, CGA-301).
- **CG4** intègre le marché de denrées (CGA-317) avant l'usage par la restauration (socle 40).
- **CG5** intègre les empreintes d'IBAN, la chaîne de suppléance, la reprise attestée, la déclaration d'intérêts ; dépend désormais du lot RH1 du 49 pour l'empreinte des IBAN de salariés (sinon la collision ne compare que fournisseurs et payeurs).
- **Parcours minimal du doublon** : CG0 → CG8b.
- Migrations : 2978 (réserve) reçoit les tables `ent_cg_iban_empreinte`, `ent_cg_regime`, `ent_cg_correspondance_compte`, `ent_cg_interet_declare`, `ent_cg_marche_denrees(_produit, _prix)` et les colonnes de devise (`montant_devise_centimes`, `devise`, `taux`, `taux_source`) ; **aucune colonne JSON**.

### 33.7 Questions ajoutées
Q-CG22 (originaux papier), Q-CG23 (contrat du cabinet) : soumises à Florian ; Q-CG24 à Q-CG27 : appliquées (`PERSONAS_48.md` §6).

---

## 34. Sources (consultées le 2026-11-11)

Recherche par moteur ; les pages officielles marquées [V] ont été lues dans l'extrait du résultat, les autres sont des sources secondaires [V-sec]. Légifrance renvoie 403 aux accès automatiques depuis la session.
- Taxe sur les salaires, abattement 2026 (24 256 €) : https://www.impots.gouv.fr/sites/default/files/formulaires/2502-sd/2026/2502-sd_5581.pdf ; https://www.impots.gouv.fr/professionnel/questions/mon-association-peut-elle-etre-redevable-de-la-taxe-sur-les-salaires
- ANC 2018-06 et 2020-08 : https://www.associatheque.fr/fr/fichiers/actualite/reglement-2018-06.pdf ; https://www.associatheque.fr/fr/fichiers/actualite/reglement-2020-08-modifiant-ANC-2018-06-comptes-annuels.pdf ; https://www.fiducial.fr/Audit-entreprise/Conseils/La-reforme-du-plan-comptable-des-associations-evolution-ou-revolution
- Facturation électronique et associations : https://www.fiducial.fr/facturation-electronique/actualites-conseils/associations-facturation-electronique ; https://www.rydge.fr/hubfs/ESS/Facturation%20%C3%A9lectronique%20et%20acteurs%20ESS%20D%C3%A9cembre%202025.pdf ; https://www.filiz.io/blog/facturation-electronique-cfa-organisme-formation (cite la FAQ DGFiP)
- FEC : https://www.legalstart.fr/fiches-pratiques/comptabilite-entreprise/fichier-ecritures-comptables-fec/ ; https://hayot-expertise.fr/blog/fec-fichier-des-ecritures-comptables
- Commissaire aux comptes des associations : https://www.justice.fr/fiche/cas-association-recourir-commissaire-comptes ; https://h2a-france.org/wp-content/uploads/2025/09/Position-commune-H2A-CNCC-AGP-1.pdf
- M9.6-Op@le : https://www.education.gouv.fr/bo/2026/Hebdo7/MENF2600269J ; https://www.ih2ef.gouv.fr/cadre-budgetaire-et-comptable-pour-les-eple-connectes-au-systeme-dinformation-comptable-ople
- Seuils de la commande publique 2026 : https://www.collectivites-locales.gouv.fr/index.php/animer-les-territoires/commande-publique/les-seuils-et-taux-applicables-aux-contrats-de-la-commande-publique ; https://www.weka.fr/actualite/?p=211682 ; https://www.vie-publique.fr/fiches/20268-quelles-sont-les-conditions-de-passation-dun-marche-public
- Forfait d'externat : https://enseignement-catholique.fr/1-forfaits-communaux ; https://www.letudiant.fr/educpros/enquetes/letat-principal-financeur-des-etablissements-scolaires-prives.html
- Charlemagne Comptabilité et Paie (Aplim) : https://wapt.tranquil.it/store/fr/details/tis-charlemagne-comptabilite_6.9.0.0-10_windows_10.0_PROD.wapt ; https://aplim.fr/ressources/communication/infomag/infomagaplim_202106.pdf ; https://www.crowe.com/fr/groupe-fideliance/industries/ogec
- Échecs : Légifrance (403 sur accès direct) ; FAQ DGFiP sur la facturation électronique non lue directement ; texte ANC non lu sur anc.gouv.fr.
