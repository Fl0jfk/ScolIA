# Socle 24 — Facturation (et cadre commun des finances des familles, avec le socle 25)

**Date :** 2026-11-10 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code (lot 7 : chemin historique `/facturation/*` des migrations 024–033 ; lot S2-9 : chemin `/finances/*` des migrations 200–206, présent sur `ENT` depuis `15a8045a`) pour le rattraper (`docs/PILOTAGE.md` §4, « rattrapage des domaines codés avant la méthode »). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à l'application Next.js.
**Branche :** `ent-socle-finances-conception`, partie d'`origin/ENT` (`5366c196`). `ent-recette-securite-3` **lue sans être fusionnée** (voir §2.2 : ses correctifs ne sont pas sur `ENT`).
**Document jumeau :** `docs/socles/25_PAIEMENTS.md` (paiements, prélèvements SEPA, rapprochement, relances et recouvrement). La carte (`docs/SOCLES.md` §2, socles 24 et 25) sépare les deux socles ; ce document-ci porte **le cadre commun** (thèse, existant, acteurs, réglementaire, cas limites des familles, lot 0, lots, migrations, questions) et **la facturation** ; le 25 porte l'argent qui rentre (et qui ressort). Les deux se lisent ensemble.
**À lire avec :** `CLAUDE.md` (données des familles, sécurité par conception, l'IA n'a jamais plus de droits que l'utilisateur, envois soumis à confirmation humaine) ; `docs/DECISIONS.md` (D-013, **D-014** IBAN masqué et nouveau mandat, D-017, D-019, D-021, **D-023**, D-024, **D-025** comptabilité OGEC dans l'ENT, D-026, **D-027 (E)** avoir plutôt que réémission, D-028, **D-029** ouverture au public, **D-031**) ; `docs/SOCLES.md` (socles 24, 25, 26, 40, 41, 42, 43, 48) ; `docs/MATRICE_CHARLEMAGNE.md` §3.11 (FIN-01 à FIN-18) ; `docs/SOCLE2_DOMAINES.md` §D10 et §6.5 (S2-9 réalisé) ; `docs/socles/04_ADMINISTRATIF.md` §6.2, **§8.9 payeurs**, §8.12 destinataires ; `07_DOCUMENTS.md` (natures financières, coffre, avoir R-D2) ; `08_NOTIFICATIONS.md` (natures `finances.*`, CN-07) ; `09_INTEROP.md` ; `IA_COEUR.md` (§6.2 niveaux de confirmation : **la facturation est N3**) ; `docs/recette/RECETTE_SECURITE_3.md` (branche `ent-recette-securite-3`) ; modèles de forme : `16_MESSAGERIE.md`, `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée, §5) ; **[NV]** règle non vérifiée sur une source officielle ; **[H]** hypothèse (pratique du terrain, marché) ; **[R]** recommandation (non décidée). Numérotation propre : actions **FAC-100** et suivantes (socle 24) et **PAY-100** et suivantes (socle 25 ; FIN-01 à FIN-18 sont pris par la matrice et `SOCLE2_DOMAINES.md`) ; défauts **FD-01**… ; règles **R-F1**… (24) et **R-P1**… (25) ; cas limites **CF-01**… ; questions **Q-F1**… ; réglages signalés **S-150**… (après S-148 du socle 16).
**Plages de migrations :** socle 24 **5400–5499** si la règle « 3000 + 100 × numéro de socle » (Q-B2 d'`ARCHITECTURE_BRIQUES.md`) est adoptée, **repli 520–539** sinon ; socle 25 **5500–5599**, repli **540–559** ; socle 26 (bourses) **5600–5699**, repli **560–579**, réservé pour éviter une collision. Vérifié libre sur **toutes les branches distantes** le 2026-11-10 (plus haute migration trouvée : 480, socle 16 M0a ; plus haute réservation : 500–519, socle 17). Q-B2 n'étant pas adoptée à ce jour (le socle 21 a pris son repli 460), **le lot F0 part en 520**. Registre : `MIGRATIONS.md`.
**Méthode.** Lecture des décisions, de la carte, de la matrice, des conceptions 04, 07, 08, 16, de la recette de sécurité 3 ; lecture directe des migrations 024, 025, 026, 030 et 200 à 205 ; deux audits indépendants du code en lecture seule (chemin historique ~40 lectures, chemin S2-9 ~50 lectures), dont **les défauts FD-01, FD-02, FD-05, FD-06 et FD-08 ont été relus ligne à ligne** par le concepteur (`service.rs:18188-18201`, `:18612`, `routes.rs:11019-11038`, `fac_repo.rs:918-945`, `024_facturation.sql`) ; recherche réglementaire (sous-agent, ~55 lectures) sur Légifrance, service-public.fr, impots.gouv.fr, BOFiP, banque-france.fr, le rulebook de l'EPC (PDF officiel lu en entier), cnil.fr, EUR-Lex ; les pages d'economie.gouv.fr, de la liste des rulebooks de l'EPC et du BO 2025–2026 sur les bourses ont renvoyé 403 (points marqués [NV]).

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année financière vue du terrain · §5 Réglementaire · §6 Frontières · §7 Concepts · §8 De la scolarité à la facture · §9 Payeurs, débiteurs, familles séparées · §10 Tarifs, réductions, aides · §11 Consommations et services · §12 Sorties et voyages · §13 Pièces, numérotation, avoirs, documents · §14 Compte famille · §15 Cas limites · §16 IA · §17 Actions FAC · §18 Règles R-F · §19 Droits et minimisation · §20 Événements · §21 Notifications · §22 Écrans · §23 Modèle de données · §24 Plan de migration · §25 Lots (24 et 25) et migrations · §26 Tests et recette · §27 Réglages signalés · §28 Questions · §29 Faits, hypothèses, contradictions · §30 Les cinq décisions prioritaires.

---

## 0. En bref

1. **Deux chemins sur les mêmes tables.** Le dépôt contient **deux logiques de facturation** qui écrivent dans les mêmes lignes `ent_fac_*` : le chemin historique `/facturation/*` (lot 7, UI JavaScript `ui.rs`) et le chemin S2-9 `/finances/*` (centimes, prorata, multi-payeurs, sources idempotentes, avoirs, numérotation continue, pré-notification, rejets, lettrage, relances confirmées, export d'écritures équilibré, RLS). Le second est **bien conçu** ; le premier est **dangereux** et le contourne : numéro de facture saisi par le client, aucun avoir, encaissements concurrents non verrouillés, facture « soldée » avec un prélèvement encore ouvert, prélèvements par facture invisibles du chemin neuf (**double prélèvement possible**), export SEPA en `GET` qui change l'état et reçoit l'IBAN du créancier dans l'URL, mandats révoqués encore exportés, `ON DELETE CASCADE` qui effacerait encaissements et quittances avec une facture.
2. **Les failles de la recette 3 ne sont pas corrigées sur `ENT`.** RS3-01 (un parent **non payeur** lit factures, IBAN et mandats, et **peut détourner le prélèvement** en remplaçant le mandat par le sien) et RS3-02 (**double encaissement** SEPA) sont corrigées sur `ent-recette-securite-3`, **non fusionnée** : `foyer_ids_parent` ne filtre toujours pas `est_payeur` sur `ENT` (`service.rs:18188-18201` [F]). Un **lot F0** les rapatrie et ferme le reste (FD-01 à FD-16) **avant toute autre évolution**.
3. **La thèse (§1).** L'argent des familles est **une suite de faits datés et prouvés** : un fait de scolarité (inscription, régime, repas, sortie) ouvre un **droit à facturer** expliqué ; une **pièce** émise ne change plus jamais (on la compense par un avoir) ; chaque centime a une origine en un clic (« d'où vient cette ligne ? ») ; **qui paie** est un fait du socle 4, daté et par catégorie, jamais une case cochée sur un parent ; **personne ne reçoit une facture qui ne le concerne pas**, et un parent séparé ne voit **rien** du compte de l'autre.
4. **Le réglementaire tranche sept points [V]** (§5) : (a) dans le privé sous contrat d'association, **l'externat simple est gratuit** ; la contribution des familles n'est possible que pour l'enseignement religieux et l'immobilier/le matériel, plus les redevances de demi-pension et d'internat (C. éduc. R. 442-48) : chaque article porte sa **nature légale** pour répondre à un contrôle ; (b) facture ou note obligatoire au-delà de 25 € TTC envers un particulier, **numérotation continue**, mentions légales, **avoir** pour corriger ; (c) enseignement, cantine, internat, fournitures indispensables **exonérés de TVA** (CGI 261-4-4°), avec mention du motif ; (d) ces opérations exonérées sont **hors facturation électronique et e-reporting** (FAQ DGFiP 2026), mais l'OGEC doit **recevoir** des factures électroniques depuis le 1er septembre 2026 (socle 48) ; (e) **prescription de 2 ans** très probable des créances sur les familles (C. consom. L. 218-2 ; la Cour de cassation traite les parents en consommateurs) ; (f) **frais de recouvrement interdits** sans titre exécutoire (CPCE L. 111-8) ; (g) **bourses** déduites seulement des frais de demi-pension et de pension, avec procuration, et pas si le boursier n'est pas le payeur [V partiel, NV sur la version consolidée].
5. **Parents séparés et payeurs tiers (§9).** Un **compte débiteur** par payeur (personne ou organisme), alimenté par la désignation datée du socle 4 (`payeurs(eleve, catégorie, date)`), jamais par le foyer de résidence ; répartition par catégorie (« la mère paie la scolarité, le père la cantine ») ; **chaque payeur reçoit sa propre facture de sa part**, avec son propre tarif au quotient, ses propres réductions ; changement de payeur **à une date**, sans toucher aux pièces émises ; élève majeur payeur, grand-parent, ASE, employeur, comité d'entreprise : tous possibles sans en faire des « responsables ».
6. **Ce que ScolIA apporte de plus que Charlemagne [H]** : une rentrée **facturée sans ressaisie** depuis les inscriptions validées (avec simulation chiffrée avant émission) ; « d'où vient cette ligne ? » ; les consommations (cantine, internat, études, garderie) facturées depuis les faits du jour ; la famille voit **solde, échéances, documents**, paie en deux gestes, comprend chaque ligne ; une relance n'atteint **jamais** le mauvais parent ni une famille en deuil ; la comptabilité reçoit des **écritures équilibrées, numérotées, prouvées** ; l'IA explique, détecte et prépare, **ne décide et n'envoie rien**.
7. **Le plan.** **227 actions** (FAC-100 à FAC-214 pour le 24, 115 ; PAY-100 à PAY-211 pour le 25, 112), **72 cas limites** (CF-01 à CF-72), **52 règles** (R-F1 à R-F28, R-P1 à R-P24), **29 réglages signalés** (S-150 à S-178), **26 lots** : **F0** (correctifs urgents, commun), **F1 à F13** (facturation ; F13 = établissements publics), **P1 à P12** (paiements). **Parcours minimal pour un pilote réel** : F0, F1–F5, F7, P1–P4, P6 ; **bascule depuis Charlemagne au 1er septembre seulement**, après une **période de double tenue** comparée au centime (Q-F2, soumis à Florian : données réelles).
8. **Décisions appliquées par défaut (D-031)** : 39 questions (Q-F1 à Q-F25 ici, Q-P1 à Q-P14 au 25) tranchées selon la recommandation ; **soumis à Florian** : contrat bancaire (ICS, mode de remise, coûts), prestataire de paiement en ligne et ses frais, logiciel et plan comptable de l'OGEC (avec l'expert-comptable), clauses du contrat de scolarisation (avec un juriste), recours à une société de recouvrement, durées de conservation (DPO), données réelles et double tenue.

---

## 1. Thèse : l'argent des familles est une suite de faits datés, prouvés, adressés à la bonne personne

1. **Une pièce émise ne change jamais.** Une facture, un avoir, une quittance émis sont figés (contenu, montant, numéro, document rendu, empreinte) ; une erreur se **compense** par un avoir puis, s'il le faut, une nouvelle facture (D-027 E, R-D2 du socle 7). Rien ne se supprime ; la base le garantit (déclencheurs), pas seulement le code.
2. **Chaque ligne dit d'où elle vient.** Une ligne n'est jamais tapée « à la main » sans motif : elle naît d'un **fait** (inscription validée, régime au 1er octobre, 12 repas en novembre, voyage à Rome) ou d'un geste motivé du personnel (frais exceptionnel). Son **explication** (tarif appliqué, tranche de quotient, prorata, réduction, part du payeur) est conservée et lisible par la famille dans des mots simples.
3. **Qui paie est un fait, pas une case.** La désignation des payeurs appartient au socle 4 (§8.9 : par catégorie de frais, datée, personne ou organisme, somme = 100 %). La facturation **lit** ce fait à la date de la prestation ; elle ne déduit jamais le payeur du foyer de résidence ni du « responsable 1 ».
4. **Un payeur, un compte, sa part.** Chaque payeur a son **compte débiteur** dans l'établissement ; il reçoit sa facture de **sa** part, calculée sur **son** quotient ; il ne voit rien du compte d'un autre payeur, même pour le même enfant.
5. **On calcule d'abord, on émet ensuite, on prévient enfin.** Génération = **simulation chiffrée** (combien, pour qui, avec quels écarts par rapport à l'an dernier) ; émission = geste humain **N3** sur un lot vérifié ; envoi aux familles = après validation, dans la plage calme (socle 8), avec un document lisible.
6. **Centimes entiers, arrondis en faveur de la famille, sommes exactes.** Montants en `i64` centimes ; prorata au mois commencé ; arrondi à l'inférieur pour la famille ; répartition par le plus fort reste (la somme des parts est exacte au centime) [F, `finances_calcul.rs`].
7. **La nature légale de chaque euro est connue.** Contribution des familles (au titre de R. 442-48 : enseignement religieux, immobilier et matériel), demi-pension, internat, prestations facultatives, voyages, fournitures ; régime de TVA (exonéré 261-4-4° ou taxable) ; compte de produit. Un contrôle de l'inspection ou du commissaire aux comptes trouve sa réponse sans tableur.
8. **Ce qui est dû à la famille l'est aussi.** Remises d'ordre (absence longue, stage, fermeture), avoirs de départ, trop-perçus **remboursés** ou affectés avec accord, bourses déduites quand la loi le dit : le logiciel **propose** les sommes dues aux familles avec la même rigueur que celles qu'elles doivent.
9. **Le social n'est pas une donnée comptable.** Une difficulté de paiement ouvre un dialogue (plan d'apurement, renvoi vers l'assistante sociale, fonds social au socle 26), jamais une sanction de l'élève, jamais une étiquette visible des professeurs ; la décision d'aide est **anonymisée** dans la comptabilité.
10. **L'IA explique, contrôle, prépare ; elle ne facture pas.** Elle explique une ligne à une famille, repère une fratrie oubliée ou un tarif aberrant, simule un barème, prépare un courrier ; toute émission, tout envoi, toute remise est **N3, confirmé par une personne** (`IA_COEUR.md` §6.2).

---

## 2. Évaluation franche de l'existant

Code lu sur `origin/ENT` (`5366c196`), chemins depuis `ent-rust/`. Deux audits indépendants en lecture seule ; **FD-01, FD-02, FD-05, FD-06 et FD-08 relus ligne à ligne**.

### 2.1 Ce qu'il y a [F]

| Élément | Où | Ce que ça fait |
|---|---|---|
| Pièces | `024_facturation.sql`, `025`, `029`, `201` | `ent_fac_facture` (débiteur = **`foyer_id`**, `numero` unique par établissement, `statut` brouillon / émise / soldée, `type_piece` facture / avoir, `facture_origine_id`, `origine`, `nature`, `annee_scolaire`) ; `ent_fac_ligne` (`eleve_id`, quantité, montant unitaire **signé**, `source_cle` **unique** par établissement, `article_id`, `categorie`, période, `ligne_origine_id`) |
| Échéances | `026`, `033` | `ent_fac_echeance` (rang, montant net, `bourse_centimes` ventilé, date) |
| Encaissements, quittances | `025`, `027`, `203` | `ent_fac_encaissement` (moyens espèces, chèque, virement, CB, prélèvement, autre ; `reglement_id`) ; `ent_fac_quittance` 1:1 ; `ent_fin_reglement` (règlement manuel imputé sur les plus anciennes, trop-perçu) ; `ent_fin_lettrage` |
| SEPA | `030`, `031`, `202`, `078`, `376` (branche) | `ent_fac_sepa_mandat` (RUM unique, IBAN/BIC/titulaire **chiffrés AES-GCM** `champs_chiffres.rs:27-29`, un mandat actif par foyer) ; `ent_fac_sepa_prelevement` (échéance, pré-notification, rang de présentation, `end_to_end_id` unique) ; `ent_fin_prenotification` ; `ent_fin_rejet` (motif, frais, suite) ; `ent_foyer_iban_histo` ; détails : `25_PAIEMENTS.md` §2 |
| Paramètres, tarifs | `200` | `ent_fin_parametres` (mois facturables, jour d'échéance, **14 jours** de pré-notification, frais de rejet, seuil de remise d'ordre, créancier SEPA, comptes 411/512, journaux) ; `ent_fin_article` (catégorie, compte de produit) ; `ent_fin_tarif` (montant **annuel** par niveau, régime, tranche de quotient) ; `ent_fin_reduction` (fratrie au rang N, personnel, autre ; taux) ; `ent_fin_bourse_bareme` ; `ent_fin_echeancier_type` ; `ent_fin_compteur` |
| Calculs | `ent-db/src/finances_calcul.rs` | prorata au mois commencé, arrondi inférieur, plus fort reste, choix du tarif le plus spécifique, catégorie due selon le régime ; 9 tests unitaires |
| Génération et événements | `ent-db/src/finances.rs`, `finances_reactions.rs` | génération annuelle depuis les dossiers d'inscription **validés** avec **aperçu sans écriture** ; consommateur `d10.facturation` de `inscription.validee`, `eleve.regime_changed`, `eleve.sortie`, `eleve.absence_justified` (remise d'ordre) ; brouillon unique par (foyer, origine, année) ; idempotence par `source_cle` |
| Multi-payeurs | `finances.rs:781-842`, `:974-1077` | parts de `ent_eleve_lien.est_payeur` / `part_paiement` ; **chaque foyer payeur a son propre tarif au quotient**, puis sa part ; rang de fratrie par foyer payeur ; meilleure réduction fratrie + réductions individuelles ; bourse en ligne négative répartie |
| Émission | `finances.rs:3097-3253` | numéro continu `F{année}-nnnnnn` / `A{année}-nnnnnn` (séries par **année civile**) ; échéancier à l'émission (calendrier type ou jour du mois) ; avoir imputé par lettrage |
| Relances | `204`, `finances_sepa.rs:1027-1247` | paliers (7, 30, 60 jours par défaut), **proposées** par le système, **envoyées après confirmation humaine**, jamais avant l'échéance ni pendant un prélèvement en cours |
| Export comptable | `205`, `finances_sepa.rs:1251-1478` | `scolia_ecritures_v1` (18 colonnes inspirées du FEC [NV]), période close, équilibre contrôlé en base, empreinte SHA-256, **une pièce n'est exportée qu'une fois** ; export historique « OGEC » = journal d'encaissements CSV |
| Paiement en ligne | `paiement_en_ligne.rs` | trait `PrestatairePaiement`, implémentation `PaiementDesactive` (route 409) |
| Espace famille (neuf) | `finances.rs:2826-2852`, `:3385-3481` | payeur seulement (`est_payeur`) : pièces émises, échéances et état, solde, crédit, mode de paiement, mandats masqués ; **ni lignes, ni PDF, ni quittance** |
| Droits | `ent-auth/src/actions.rs:996-1001` | `facturation.lire`, `facturation.gerer`, `bourses.gerer` (D-017) ; rôles Admin, Direction, Administratif, Compta ; portée établissement |
| RLS | `100`, `206` | ENABLE + FORCE sur `ent_fac_*` et les 17 `ent_fin_*` (mode `off` par défaut, RF-20) |
| Tests | `ent-db/src/tests_finances.rs`, `ent-api/src/tests_finances.rs` | 6 scénarios Postgres (rentrée multi-payeurs 60/40 au centime, prorata, fratrie, quotient, bourse, avoirs de régime, sortie et absence, SEPA de bout en bout, relances, export, droits, RLS) + 1 test HTTP |
| Chemin historique | `domain-eleve/src/facturation*.rs`, `fac_repo.rs`, `service.rs:18179-19455`, `routes.rs:747-808`, `ui.rs` (48 appels) | créer (numéro du client), émettre, encaisser, échéancier libre ou en parts, quittances PDF, mandats, prélèvements par facture, export CSV / pain.008, exécution, rejet, rapprochement CSV simulé (`ent-bank`), rappels J-3 et retard, quotient, TVA, cantine au badge, acompte de réinscription, export « OGEC » |

### 2.2 Défauts trouvés (à corriger avant toute extension)

| # | Gravité | Défaut | Conséquence | Où [F] | Lot |
|---|---|---|---|---|---|
| **FD-01** | **Critique** | **RS3-01 non corrigée sur `ENT`** : `foyer_ids_parent` renvoie tous les foyers d'un lien actif sans filtrer `est_payeur` ; toutes les lectures et écritures familiales du chemin historique en dépendent | un parent séparé non payeur ou un tiers lié lit factures, quittances, historique IBAN, mandats ; **révoque le mandat et en crée un avec son IBAN** : le prélèvement est détourné, ou la mère payeuse est prélevée sur un mandat signé par le père | `service.rs:18188-18201` ; correctif sur `ent-recette-securite-3` (non fusionnée) | F0 |
| **FD-02** | **Critique** | **RS3-02 non corrigée sur `ENT`** : exécution d'un prélèvement = encaissement **avant** contrôle de statut, hors transaction commune ; relevé rejoué idem | la même échéance encaissée deux fois ; soldes faux ; quittance en double | `service.rs:19212`, `:19301-19321` ; correctif + migration 376 sur la branche | F0 |
| **FD-03** | Élevée | **RS3-03 partiellement ouverte** : `GET /facturation/sepa/export` exporte sans exiger la pré-notification, **change l'état en `GET`**, reçoit **IBAN, BIC et ICS du créancier en paramètres d'URL** | prélèvement sans pré-notification (non conforme au rulebook) ; coordonnées bancaires de l'établissement dans les journaux de proxys ; un préchargement de lien suffit à marquer des lignes « exportées » | `routes.rs:11019-11038`, `service.rs:18904` | F0 |
| **FD-04** | Élevée | **Deux logiques sur les mêmes lignes** : l'encaissement historique ignore avoirs et lettrage ; le prélèvement historique est **par facture** (`echeance_id` NULL), invisible de la préparation par échéance ; l'export historique emporte les prélèvements du chemin neuf ; les rappels historiques doublent les relances | **double prélèvement** d'une même échéance ; sur-encaissement d'une facture qui a un avoir ; deux relances le même jour | audit S2-9 §7 ; `fac_repo.rs:918-945` | F0, F1 |
| **FD-05** | Élevée | **Numéro de facture fourni par le client** (`ui.rs` le fabrique avec `Date.now().toString(36)`) et attribué dès le brouillon | numérotation ni continue ni chronologique (exigence légale, §5) ; deux factures « cantine » le même jour entrent en collision (`CM-AAAAMMJJ`) | `routes.rs:10573`, `domain-eleve/src/facturation.rs:176` | F0 |
| **FD-06** | Élevée | **Facture « soldée » avec un prélèvement encore ouvert** : l'encaissement manuel compte comme « déjà encaissé » l'encaissé **plus** les prélèvements ouverts | la facture passe soldée trop tôt ; le prélèvement ne peut plus s'exécuter (`FactureDejaSoldee`) ; l'argent arrive quand même à la banque | `service.rs:18612-18613` | F0 |
| **FD-07** | Élevée | **Encaissements concurrents non verrouillés** ; encaissement et quittance dans deux transactions, rang de quittance par `count + 1` | deux guichets encaissent au-delà du total ; encaissement sans quittance | `service.rs:18626-18635` | F0 |
| **FD-08** | Élevée | **`ON DELETE CASCADE`** de la facture vers lignes, encaissements, quittances, échéances, prélèvements, rappels | une suppression de facture (script, erreur d'intégration) **efface des pièces comptables et des preuves de paiement** sans trace | `024`, `025`, `026`, `027`, `030`, `032` | F0 |
| **FD-09** | Élevée | **Immutabilité seulement applicative** : aucun déclencheur ne protège une pièce émise ; le chemin historique peut poser un échéancier sur une facture du chemin neuf | une facture émise modifiable par un chemin oublié ou un import | audit S2-9 §6 | F0 |
| **FD-10** | Moyenne | **Mandats révoqués encore exportés** ; la révocation n'annule pas les prélèvements prévus ; statut `annule` jamais écrit | prélèvement sur un mandat révoqué = opération **non autorisée** (remboursable 13 mois, §5) | `fac_repo.rs:938` | F0 |
| **FD-11** | Moyenne | **IBAN du créancier en clair** dans `ent_fin_parametres` ; changement des paramètres (dont IBAN créancier) **sans événement ni confirmation renforcée** ; aucune séparation des tâches | une seule personne (ou un compte volé) change le compte crédité et l'export suivant **verse l'argent des familles ailleurs** sans alerte | `finances.rs:3008` ; audit S2-9 §4 | F0 |
| **FD-12** | Moyenne | **Bourse imputée sur toute la facture** (ligne négative répartie entre tous les payeurs, toutes catégories) | contraire à la règle de déduction sur la seule demi-pension / pension, au profit du payeur boursier [V partiel, §5.7] ; l'autre parent profite de la bourse du premier | `finances.rs:1038-1077` | F8 (règle provisoire en F0) |
| **FD-13** | Moyenne | **Frais de relance facturables** (`ent_fin_palier_relance.frais_centimes`) et frais de rejet refacturés émis aussitôt | **frais de recouvrement à la charge du débiteur interdits** sans titre exécutoire, clause réputée non écrite (CPCE L. 111-8 [V]) ; frais de rejet : à qualifier par un juriste | `204`, `finances_sepa.rs:142-194` | F0 (frais de relance désactivés) |
| **FD-14** | Moyenne | **Payeurs sans compte : aucune pré-notification ni relance** (notification interne seulement) | prélèvement sans pré-notification pour un parent sans compte (non conforme) ; relance qui n'arrive jamais | audit S2-9 §3 | F0 (garde), P3 |
| **FD-15** | Moyenne | **Élève majeur payeur sans accès** à son compte (sa fiche payeur n'a pas de compte, le filtre cherche `user_id`) | l'élève majeur paie sans voir ses factures | `dossier_personnes.rs:1402-1403` | F2 |
| **FD-16** | Moyenne | **Montants de consommation de −1 M€ à +1 M€** acceptés ; réductions empilées sans plancher par ligne | une consommation négative devient un avoir déguisé, sans pièce ni motif | `finances.rs:2502` | F0 |
| FD-17 | Moyenne | **Aucune TVA ni motif d'exonération** sur les pièces du chemin neuf ; la table `ent_taux_tva` du chemin historique ne sert à rien | facture sans la mention obligatoire « motif d'exonération » (§5.2) ; prestations taxables (location, vente annexe) impossibles | `075`, audit S2-9 §6 | F3 |
| FD-18 | Moyenne | **PDF « sans valeur fiscale »** fait main (Latin-1, une page, « ? » à la place des caractères), sans mentions légales ; **aucun PDF** sur le chemin neuf ; documents non stockés (régénérés à chaque appel) | la famille ne reçoit pas de facture conforme ; impossible de prouver ce qui a été remis (socle 7 §2 point 12) | `facturation_pdf.rs:148` | F5 |
| FD-19 | Moyenne | **Paramètres et réglages sans versions** (tarifs « actifs » modifiables, réductions supprimables en cascade `ent_fin_reduction_eleve ON DELETE CASCADE`) | impossible de dire quel tarif était en vigueur au moment d'une facture contestée | `200` | F3 |
| FD-20 | Faible | **Séries de numérotation par année civile** alors que tout le reste est par année scolaire | lisible mais déroutant ; conforme (continuité par série) | `finances.rs:3106-3112` | F5 (choix Q-F12) |
| FD-21 | Faible | **Remise d'ordre non recalculée** si les dates de l'absence sont corrigées (`source_cle` figée) | avoir faux après correction d'une absence | audit S2-9 §6 | F6 |
| FD-22 | Faible | **Rang de fratrie par date de naissance dans le foyer payeur**, pas par règle de l'établissement (le plus jeune, l'inscrit le plus ancien, les seuls enfants de l'établissement ou de l'ensemble scolaire) | réduction contestée dans les familles recomposées | `finances.rs:818-842` | F8 |
| FD-23 | Faible | **Limites codées en dur sans pagination** (80 factures, 80 mandats **filtrés ensuite en mémoire**) | un parent ne voit pas son propre mandat dans un gros établissement | `service.rs:18860` | F0 |
| FD-24 | Faible | **Fuseau** : export historique et dates comptables en UTC | un encaissement de 0 h 30 tombe la veille | `fac_repo.rs:556` | F0 |
| FD-25 | Faible | **Aucun consommateur** des événements `fac.*` ; `actor_user_id` absent de certains événements ; `tracer_acces` limité aux exports | chronologie de l'élève et journal incomplets | audits | F1 |
| FD-26 | Faible | **`/facturation/famille` vérifie `session.role == "parent"` en dur** et affiche les avoirs comme des factures négatives | contournement de D-017 ; affichage trompeur | `routes.rs:10538` | F0 (lecture), F7 |

### 2.3 Verdict : garder, corriger, remplacer

- **Garder** le chemin S2-9 comme **base unique** : calculs, sources idempotentes, avoirs, numérotation transactionnelle, multi-payeurs par foyer, pré-notification, rejets, lettrage, relances confirmées, export équilibré, RLS, tests. C'est le meilleur domaine financier du dépôt.
- **Corriger** immédiatement (F0) les failles et les incohérences entre chemins ; poser en base l'immutabilité et l'interdiction de suppression.
- **Remplacer** le chemin historique module par module (F1) : ses routes deviennent des **adaptateurs** du service unique (pour ne pas casser `ui.rs` ni le labo), puis disparaissent avec `ui.rs` (F-001) ; aucune fonction nouvelle ne lui est ajoutée.
- **Changer de débiteur** (F2) : du `foyer_id` (lieu de vie) au **compte débiteur** du payeur (personne ou organisme), en gardant `foyer_id` par compatibilité comme le prévoit le socle 4 (§6.2, Q-A1).

### 2.4 Ce que fait l'application Next.js de production (sans lecture nouvelle)
D'après la matrice (`MATRICE_CHARLEMAGNE.md`, lignes 461 et 466 [F, lecture déjà faite par une autre session]) : facturation `app/api/facturation/`, paiement en ligne `app/api/billing/` avec **EasyTransac**, facturation des passages de cantine (`app/lib/passages-facturation-shared.ts`). **Aucune lecture nouvelle** n'a été faite pour ce document. Le choix d'un prestataire de paiement pour l'ENT Rust reste ouvert (Q5, `25_PAIEMENTS.md` Q-P1) : l'existence d'un contrat EasyTransac côté Next.js est un **fait à confirmer par Florian**, pas une décision.

---

## 3. Acteurs, besoins réels, irritants

Pratiques des logiciels du marché (Charlemagne, École Directe, Pronote n'a pas de facturation) **de mémoire et par la matrice** : **[H]** partout, à confirmer à la passe personas.

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Chef d'établissement / directeur** | Propose les tarifs au conseil d'administration de l'OGEC ; arbitre les remises, les impayés délicats, les départs ; signe les courriers | Des tableaux Excel pour simuler les tarifs ; ne pas savoir combien rapporte une hausse de 2 % ; découvrir un impayé de 3 000 € en juin | Exclure un enfant pour une dette de parents ; un contrôle qui trouve une contribution illégale (R. 442-48) ; un litige perdu faute de preuve | Une **simulation** « tarifs 2027 » chiffrée par niveau et régime, comparée à l'an dernier ; un tableau des impayés qui distingue « oubli », « difficulté », « conflit » ; aucune décision sociale mélangée à la comptabilité | mensuelle, pics en mai-juin |
| **Président / trésorier de l'OGEC** | Vote les tarifs ; suit la trésorerie et le budget ; répond du commissaire aux comptes | Des chiffres qui changent selon qui les sort ; pas de prévision d'encaissements | Un trou de trésorerie en décembre ; une fraude interne (changement d'IBAN créancier) | Un prévisionnel d'encaissements par mois ; des chiffres signés (empreinte) ; une piste d'audit | trimestrielle |
| **Comptable / économe / intendant** | Paramètre articles, tarifs, échéancier ; génère et émet ; encaisse ; prépare les prélèvements ; traite les rejets ; relance ; exporte vers la comptabilité ; rembourse | Ressaisir la rentrée (régimes, fratries, départs) ; refaire une facture parce qu'un parent s'est séparé ; pointer les relevés à la main ; les rejets SEPA par courrier bancaire ; les remises d'ordre calculées à la calculette ; trois écrans pour un chèque | Prélever deux fois ; prélever le mauvais parent ; perdre un chèque ; une erreur de lot de 400 familles ; être seul à savoir faire | **Une rentrée en une demi-journée** (simulation, contrôle, émission) ; rejets lus automatiquement ; rapprochement à 95 % sans geste ; une **file de travail** : « 12 rejets, 4 chèques à remettre, 7 relances proposées, 3 remboursements » | quotidienne |
| **Secrétariat** | Informe les familles (tarifs, réductions, attestations) ; reçoit des chèques ; enregistre les changements de régime et de payeur (socle 4) ; reçoit les plaintes | « Combien je dois ? » au téléphone ; attestations pour employeurs et CAF faites à la main ; changements de régime notés sur papier et oubliés de la compta | Donner à un parent le solde de l'autre ; dire un montant faux | **Attestation en libre-service** pour la famille ; changement de régime qui recalcule seul (avec aperçu de l'avoir) ; réponse « votre solde » sans accès au détail de l'autre payeur | quotidienne |
| **Accueil** | Reçoit des enveloppes (chèques, espèces) | Enveloppes sans nom ; espèces non tracées | Être accusé d'avoir perdu de l'argent | **Reçu immédiat** numéroté, enveloppe enregistrée « à remettre à la compta » avec photo du chèque | quotidienne en septembre |
| **CPE / vie scolaire** | Signale absences longues, stages, exclusions (remises d'ordre, repas non pris) | On lui demande « il était absent combien de jours ? » | Que l'impayé d'une famille soit connu de la vie scolaire ou serve de sanction | **Rien à faire** : les remises d'ordre partent des absences justifiées, sans que la vie scolaire voie l'argent | rare |
| **Responsable de restauration / internat** | Fournit les repas pris, les nuitées, les régimes | Listes de passages exportées, recopiées | Facturer un repas non pris ; un PAI mal géré | Les **faits du jour** (passages, réservations, nuitées) deviennent des consommations facturables seules (socles 40, 41) | quotidienne |
| **Organisateur de voyage (professeur)** | Budget, acomptes, désistements, aides | Collecter des chèques en classe ; tenir un tableau des payés | Partir avec des impayés ; un élève exclu du voyage faute d'argent | Un **voyage** avec budget, échéancier, aides, état des paiements **agrégé** (« 23 à jour / 2 en retard », sans détail social) | par voyage |
| **Famille unie** (deux parents, un foyer) | Reçoit une facture annuelle, choisit le prélèvement, paie la cantine et les sorties | Courriers papier, factures incompréhensibles (« Régularisation 2/10 »), trois paiements pour trois enfants | Un prélèvement surprise ; une erreur jamais corrigée | **Un solde, des échéances, des documents, une explication par ligne** ; payer en deux gestes ; l'attestation pour l'employeur en un geste | mensuelle |
| **Parent séparé payeur** | Paie sa part, ou une catégorie | Reçoit la facture totale et doit « s'arranger » ; l'autre parent voit ce qu'il a payé | Être prélevé de la part de l'autre ; que l'autre voie son adresse ou son IBAN | **Sa facture de sa part**, son propre mandat, aucune visibilité de l'autre compte, la phrase « la part de l'autre responsable lui est facturée séparément » | mensuelle |
| **Parent séparé non payeur** | Ne paie rien | Reçoit des relances qui ne le concernent pas (ou rien, sans savoir) | Être poursuivi pour la dette de l'autre ; ne rien savoir des frais de son enfant | **Rien de financier** (sauf choix « copie aux titulaires » activé, R-F8) ; il reste informé de la scolarité par les autres socles | — |
| **Famille en difficulté** | Paie en retard ou partiellement | Relances automatiques sèches ; honte au guichet | Que l'enfant soit privé de cantine ou de voyage ; que les professeurs sachent | Un **plan d'apurement** proposé en ligne ; l'orientation discrète vers l'assistante sociale et le fonds social ; jamais d'impact visible sur l'enfant | — |
| **Famille recomposée** | Le beau-parent paie, n'a aucune autorité | Le logiciel exige de le déclarer « responsable » pour qu'il paie | Que le beau-parent voie les notes | **Payeur sans autorité** : il voit ses factures et rien d'autre (socle 4 §8.9) | — |
| **Payeur tiers** (grand-parent, employeur, comité d'entreprise, ASE, ambassade, mairie) | Paie une catégorie ou un montant | Il faut une adresse, un SIRET, parfois **Chorus Pro** pour un organisme public [NV] | Recevoir une facture avec les notes de l'enfant | Une facture **à son nom**, sans aucune donnée scolaire, avec le seul nom de l'élève et la prestation | annuelle |
| **Élève majeur** | Peut être son propre payeur (études post-bac, autonomie) ; souvent ses parents restent payeurs | Le logiciel continue de tout envoyer aux parents ou ne lui montre rien | Que ses parents voient ses dépenses s'il paie seul | Voir et payer ses factures s'il est payeur ; ne rien voir de financier sinon (Q-F9) | rare |
| **Élève mineur** | Ne paie pas ; utilise le badge de cantine, le porte-monnaie (socle 25) | — | **Être humilié** : badge refusé devant les autres, nom sur une liste d'impayés | **Ne jamais être un canal de recouvrement** (R-F20) | quotidienne (cantine) |
| **Commissaire aux comptes / expert-comptable** | Contrôle les comptes de l'OGEC | Extractions incomplètes, pièces sans origine | Une écriture sans pièce ; une pièce modifiée | **Export d'écritures** numérotées, équilibrées, à empreinte, chaque écriture reliée à sa pièce ; FEC si exigé (socle 48) | annuelle |
| **Inspecteur / rectorat** (contrôle des contributions R. 442-17 [NV]) | Vérifie la conformité des contributions au contrat | — | — | Un **état des contributions par nature légale** en un geste (FAC-196) | rare |
| **Agent comptable d'un EPLE** (public, D-029) | Régies de recettes, demi-pension et hébergement, tarifs votés par la collectivité, bourses déduites | [H] Logiciel national (Op@le [NV]) | Une recette encaissée hors régie | ScolIA comme **outil de la régie** (§5.9, F13), interfacé au logiciel comptable public, sans le remplacer | quotidienne |
| **Support ScolIA** | Diagnostique | — | Accéder à des montants ou à des IBAN | **Aucun accès** aux données financières des familles ; diagnostics sur métadonnées techniques | — |
| **Assistant IA** (au nom d'une personne) | Explique, contrôle, simule, prépare | — | — | **Jamais plus de droits que la personne** ; aucune émission, aucun envoi, aucune remise sans geste humain N3 | — |

**Ce que montrent ces portraits [R].** (1) Le danger principal n'est pas le calcul (il est juste) mais **l'adressage** : facturer, relancer, prélever ou montrer à la **mauvaise personne** ; le payeur doit être un fait daté du socle 4, appliqué partout. (2) La comptabilité veut une **file de travail** et des automatismes vérifiables, pas des écrans de saisie. (3) Les familles veulent **comprendre** (explication par ligne) autant que payer. (4) L'élève doit être **hors du circuit de l'argent** : aucun refus visible, aucune liste, aucun message.

---

## 4. L'année financière vue du terrain (lycée privé sous contrat) [H, à confirmer avec Florian]

| Moment | Ce qui se passe | Ce que fait ScolIA |
|---|---|---|
| **Février–avril** | Préparation du budget de l'OGEC ; simulation des tarifs N+1 | Simulation sur les effectifs prévisionnels (socle 3) et les tarifs proposés ; comparaison N / N+1 par niveau, régime, tranche ; produit attendu (FAC-120 à FAC-124) |
| **Mai–juin** | Vote des tarifs par le conseil d'administration ; réinscriptions ; **acompte** de réinscription | Version de tarifs « votée » figée ; acompte facturé à la validation de la réinscription (socle 3) ; reprise de l'IBAN masqué et nouveau mandat (D-014, socle 25) |
| **Juillet** | Nouvelles inscriptions ; régimes déclarés ; mandats signés | Engagements financiers créés au fil des validations (§8) ; contrôle des fratries et des payeurs (alertes de qualité, socle 4) |
| **Fin août** | Génération de la facture annuelle | **Simulation** de la rentrée, revue des écarts, émission **N3** par lots, envoi après validation (plage calme) ; échéancier (10 mois ou 3 trimestres) ; pré-notification SEPA annuelle unique |
| **Septembre** | Changements de régime, arrivées tardives, départs, séparations, bourses | Avoirs et compléments **au prorata** proposés automatiquement depuis les faits ; contrôle quotidien (« 8 lignes à valider ») |
| **Octobre → juin** | Prélèvements mensuels (5 du mois par défaut [F]) ; cantine au réel ; sorties ; voyages | Lots SEPA, rejets, relances, consommations du mois, voyages (§11, §12, socle 25) |
| **Trimestres** | Bourses versées à terme échu [V partiel] ; remises d'ordre | Déduction sur demi-pension / pension du payeur boursier (§10.5) ; remises d'ordre des absences longues |
| **Juin–juillet** | Fin d'année ; départs ; soldes ; relances de fin d'année ; attestations | Solde des départs (avoir ou facture de régularisation) ; remboursement des trop-perçus (socle 25) ; quitus ; clôture des voyages |
| **Septembre N+1** | Clôture comptable N (exercice de l'OGEC, souvent sept–août [H]) | Clôture des périodes, export définitif (socle 48), report des soldes ; contrôle « créances de plus de 18 mois » (prescription, socle 25) |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-10)

Sources lues par la recherche réglementaire (sous-agent) ; certitude indiquée. Le détail SEPA, espèces, recouvrement est au **§4 de `25_PAIEMENTS.md`**.

### 5.1 Contribution des familles dans le privé sous contrat [V]
- **C. éduc. R. 442-48** (Légifrance, section `LEGISCTA000018378386`) : sous contrat d'association, **l'externat simple est gratuit** ; une contribution ne peut être demandée aux familles que pour (1°) l'**enseignement religieux et le culte**, (2°) les **annuités d'amortissement des bâtiments**, le **matériel** scientifique, scolaire ou sportif et la **provision pour grosses réparations** ; le contrat précise les **redevances** des externes surveillés, demi-pensionnaires et internes. Les services de l'État vérifient la conformité des contributions au contrat (R. 442-17 [NV sur le détail]).
- **Conséquence [R]** : chaque article porte une **nature légale** fermée (`contribution_r442_48_1`, `contribution_r442_48_2`, `demi_pension`, `internat`, `externat_surveille`, `prestation_facultative` (études, garderie, activités), `sortie_voyage`, `fournitures`, `frais_dossier`, `autre_motivee`) ; un **état des contributions par nature** répond à un contrôle (FAC-196) ; le libellé « frais de scolarité » reste possible pour la famille, la nature reste exacte en dessous.
- **Forfait d'externat** (C. éduc. L. 442-9) : versé par l'État et les collectivités à l'établissement, pas par les familles ; **hors socle 24** (socle 48) [V partiel].
- **Contrat de scolarisation** : aucun texte spécifique ; droit commun des contrats et de la consommation ; la Cour de cassation contrôle les **clauses abusives** des contrats de scolarité (Cass. 1re civ., 12/10/2016, n° 15-25.468 ; 02/04/2009, n° 08-11.596 [V]) : une clause « l'année entière est due » n'est valable que si elle permet aux parents de résilier pour **motif légitime et sérieux** [V par la jurisprudence citée]. ScolIA ne rédige pas le contrat (avis juridique, Q-F20) mais **en applique les règles déclarées** (frais de résiliation, préavis, acompte non remboursable) sous forme de réglages bornés.

### 5.2 Facture ou note, mentions, numérotation, avoir [V]
- **Mentions** (C. com. L. 441-9 ; CGI ann. II art. 242 nonies A ; fiche `entreprendre.service-public.gouv.fr/vosdroits/F31808`, vérifiée le 11/08/2026) : date et **numéro unique fondé sur une séquence chronologique continue**, date de la prestation, identité et adresse du vendeur (SIREN), nom et adresse de l'acheteur, désignation, quantité, prix unitaire HT, **taux de TVA ou motif d'exonération**, totaux HT et TTC, réductions, date de règlement, conditions d'escompte, pénalités de retard (l'indemnité forfaitaire de 40 € ne concerne **que** les clients professionnels).
- **Envers un particulier** : arrêté n° 83-50/A du 3 octobre 1983 (Légifrance `JORFTEXT000000494187`) : **note obligatoire** pour toute prestation de service d'au moins **25 € TTC**, remise sur demande en dessous ; double conservé 2 ans. Application aux frais de scolarité : **interprétation [V texte, NV application]** ; ScolIA émet de toute façon une facture conforme pour toute pièce (plus simple, plus sûr).
- **Rectification** : une facture émise ne se modifie pas ; on émet un **avoir** qui renvoie à la facture d'origine [NV sur le BOFiP, pratique constante ; D-027 E].
- **Conservation** : LPF L. 102 B, **6 ans** dans la forme d'origine (électronique si née électronique) [V] ; C. com. L. 123-22, **10 ans** pour les pièces comptables (vise les commerçants ; durée prudente pour une association) [V texte, application par prudence].

### 5.3 TVA [V]
- **CGI 261-4-4° a** ; BOFiP **BOI-TVA-CHAMP-30-10-20-50** (16/10/2019) : exonération de l'enseignement scolaire des établissements publics et des établissements privés relevant du code de l'éducation, et des **prestations étroitement liées** : logement et nourriture des internes et demi-pensionnaires, cantines, fournitures indispensables (livres, cahiers), soutien scolaire, matériel pédagogique vendu aux élèves ; **pas** les services achetés à des tiers. Voyages scolaires et périscolaire : **non traités** par cette page [NV].
- L'OGEC est **assujetti** (activité économique) mais réalise surtout des opérations **exonérées sans droit à déduction** [V partiel] ; il peut avoir des opérations **taxables** (location de salles, ventes annexes, prestations à des tiers).
- **Conséquence [R]** : chaque article porte un **régime de TVA** (`exonere_261_4_4`, `taxable` avec taux, `hors_champ`) ; la mention d'exonération est imprimée sur la pièce ; les prestations taxables sont possibles mais **signalées** au paramétrage (avis de l'expert-comptable, Q-F18).

### 5.4 Facturation électronique (réforme 2026–2027) [V]
- FAQ DGFiP du 01/09/2026 (`impots.gouv.fr/foire-aux-questions-je-decouvre-la-facturation-electronique`) : **réception** obligatoire pour toutes les entreprises depuis le **01/09/2026** (plateforme agréée choisie) ; **émission et e-reporting** pour les PME et micro-entreprises au **01/09/2027** ; ventes aux particuliers : e-reporting de totaux journaliers (CGI art. 290 I) ; **les opérations exonérées au titre des articles 261 à 261 E sont exclues** (FAQ 1.2).
- **Conséquence [R]** : frais de scolarité, cantine, internat exonérés → **hors réforme** ; les éventuelles opérations taxables de l'OGEC entreront dans l'e-reporting au 01/09/2027 [V partiel, à qualifier par l'expert-comptable] ; la **réception** des factures fournisseurs est l'affaire du socle 48 ; une facture à un **organisme professionnel** payeur (employeur, comité d'entreprise) d'une prestation exonérée reste hors réforme, mais peut être produite en PDF/A-3 structuré (socle 7 R-D20) si l'organisme le demande ; **organisme public payeur** (département pour l'ASE, mairie) : dépôt sur **Chorus Pro** probablement exigé [NV, ordonnance 2014-697] (FAC-170).

### 5.5 Prescription et recouvrement (détail : `25_PAIEMENTS.md` §4) [V/NV]
- **C. consom. L. 218-2** : « l'action des professionnels, pour les biens ou les services qu'ils fournissent aux consommateurs, se prescrit par deux ans » [V] ; parents traités en consommateurs par la Cour de cassation (§5.1) → **2 ans très probables** [V partiel] ; aucun arrêt trouvé sur les frais de scolarité [NV]. **Frais de recouvrement** à la charge du créancier sans titre exécutoire (**CPCE L. 111-8**, toute clause contraire réputée non écrite) [V]. **Injonction de payer** (CPC 1405 s., réformée par le décret 2026-96) [V partiel].
- **Conséquence pour le 24** : la pièce porte sa **date d'exigibilité** (point de départ) ; aucun « frais de relance » ne s'ajoute à une facture (FD-13) ; les pénalités contractuelles éventuelles ne sont facturées que si le contrat de scolarisation les prévoit (réglage, Q-F21).

### 5.6 Réduction d'impôt et attestations [V]
- **CGI 199 quater F** (fiche service-public F9) : 61 € collège, 153 € lycée, 183 € supérieur, moitié en résidence alternée ; **aucun justificatif à joindre** [V montant, V partiel justificatif]. ScolIA ne fabrique pas d'« attestation fiscale de frais de scolarité » trompeuse ; le **certificat de scolarité** (socle 7) suffit.
- **CGI 200 quater B** (frais de garde des enfants de moins de 6 ans au 1er janvier) : garderie et périscolaire éligibles, **pas la cantine** ; l'établissement fournit une attestation qui **sépare garde et nourriture** [V partiel] → utile pour l'école maternelle d'un ensemble scolaire (socle 50) : article de nature `garde_moins_6_ans` et attestation annuelle (PAY-181).

### 5.7 Bourses (détail au socle 26) [V partiel]
- C. éduc. **R. 531-13 à D. 531-36** (lycée) et **D. 531-1 s.** (collège) : bourse payée aux familles ; avec **procuration** au représentant légal de l'établissement, elle passe par lui **après déduction des frais de pension ou de demi-pension** ; prime d'équipement **non déductible** ; pas de déduction si la restauration est facturée au ticket ou si le bénéficiaire n'est pas celui qui paie [V partiel, version consolidée non lue ; circulaires 2025 et 2026 inaccessibles, NV].
- **Conséquence [R]** : la déduction (FD-12) ne s'impute que sur les lignes `demi_pension` / `internat` **du payeur bénéficiaire**, à hauteur de ces lignes ; l'excédent est **reversé à la famille** (remboursement, socle 25), jamais imputé sur la contribution des familles sans accord écrit ; tant que le socle 26 n'est pas codé, F0 restreint l'imputation actuelle aux catégories restauration et internat (règle provisoire).
- **Fonds sociaux** (circulaire 2017-122 [V, actualité NV]) : dans le privé sous contrat, fonds social collégien / lycéen versé **directement aux familles** ; fonds social pour les cantines = subvention à l'établissement réservée à la restauration → au 24, ce sont des **aides affectées** (§10.6), saisies par décision anonymisée du socle 26.

### 5.8 Données personnelles [V/NV]
- **Référentiels CNIL « gestion commerciale » et « gestion des impayés »** (non contraignants) [V] : données du contrat pendant la relation, puis archive le temps de la prescription ou de l'obligation légale ; fichier d'impayés supprimé **sous 48 h après régularisation**, sinon **5 ans** au plus. → La qualité « en impayé » n'est pas une donnée durable de la personne : c'est un **état calculé** du compte, qui disparaît avec le paiement (R-F21).
- Cartes et IBAN : `25_PAIEMENTS.md` §4.

### 5.9 Établissements publics (D-029) [V partiel]
- **C. éduc. R. 531-52** : tarifs de restauration fixés par la **collectivité de rattachement**, plafonnés au coût par usager [V] ; service de restauration et d'hébergement = service annexe, instruction **M9.6** [NV] ; recettes par **régie** (décret 2019-798 : versements au comptable au moins mensuels, chèques remis à l'encaissement le lendemain, régisseur personnellement responsable) [V partiel].
- **Conséquence [R]** : en public, ScolIA n'est **pas** le comptable : il est l'outil du **régisseur** et de l'ordonnateur (tarifs importés de la délibération, facturation des familles, encaissements en régie, bordereaux de versement au comptable, déductions de bourses), interfacé avec le logiciel comptable public (**Op@le** [NV]) ; lot **F13** après le pilote privé ; aucune hypothèse du privé (contribution R. 442-48, OGEC) n'est codée en dur.

---

## 6. Frontières

### 6.1 Qui possède quoi

| Sujet | 2 élève | 3 inscriptions | 4 administratif | **24 facturation** | **25 paiements** | 26 aides | 40–43 services | 48 comptabilité |
|---|---|---|---|---|---|---|---|---|
| Régime, scolarité à la date | **possède** | propose | — | lit | — | lit | lit | — |
| Inscription validée, départ | lit | **possède** | — | lit (déclencheur) | — | — | — | — |
| Payeurs (qui, quelle part, quelle catégorie, quand) | — | propose | **possède** | lit | lit | — | — | — |
| Compte débiteur, pièces, lignes, avoirs, échéanciers | — | demande (acompte) | — | **possède** | lit | demande (déduction) | demande (consommation) | lit |
| Tarifs, articles, réductions, natures légales | — | lit (simulateur) | — | **possède** | — | lit | lit | lit (comptes) |
| IBAN, mandats, prélèvements, règlements, lettrage, relances, remboursements | — | demande (reprise IBAN D-014) | — | lit (solde) | **possède** | — | — | lit |
| Bourses, fonds sociaux, quotient familial (calcul) | — | — | — | lit | — | **possède** | — | lit |
| Quotient familial : pièce (avis CAF / impôt) | — | — | — | — | — | **possède** (avec 7) | — | — |
| Repas, nuitées, études, garderie, transports | — | — | — | facture | — | — | **possèdent** (faits) | — |
| Voyages (organisation, budget, participants) | — | — | — | facture | encaisse | aide | **42 possède** | analytique |
| Écritures, journaux, grand livre, clôture | — | — | — | produit les écritures de ventes | produit les écritures de banque | produit les écritures d'aides | — | **possède** |
| Documents (PDF, coffre, trames) | — | — | — | demande | demande | demande | — | — |

### 6.2 Règles de frontière [R]
1. **Personne n'écrit dans les tables du 24** hors de ses commandes : les socles 3, 26, 40–43 **demandent** (commande `facturation::facturer_source`, idempotente par clé de source) ; le 25 **impute** par la commande `facturation::imputer` ; aucun `INSERT`/`UPDATE`/`DELETE` d'un autre module sur `ent_fac_*`, `ent_fin_*` (garde de code, comme `03_INSCRIPTIONS.md` §5.2 point 5).
2. **Le 24 n'écrit jamais dans le 4** : il lit `personnes::payeurs(eleve, categorie, date)` et réagit à `payeurs.modifies` ; s'il manque un payeur, il crée une **tâche** pour le secrétariat, il ne désigne personne.
3. **Le 24 ne connaît pas l'IBAN** : il voit « mode de paiement : prélèvement (mandat actif) » ; le 25 seul déchiffre.
4. **Les services (40–43) disent ce qui a été consommé, le 24 dit combien ça coûte** : un passage de cantine ne porte jamais de prix.
5. **La comptabilité (48) lit des écritures**, elle ne recalcule pas une facture ; une correction comptable passe par une pièce du 24 (avoir) ou une opération du 25.

### 6.3 Ports exposés par le socle 24 [R]
| Port | Pour | Nature |
|---|---|---|
| `facturation::facturer_source(source)` | 3 (acompte, frais de dossier), 26 (déduction, aide affectée), 40–43 (consommations, voyages) | commande idempotente (clé de source) ; renvoie la ou les lignes en attente |
| `facturation::annuler_source(cle, motif)` | idem | commande ; produit un avoir si la ligne est émise |
| `facturation::simuler(params)` | 3 (simulateur public de tarifs, T-18), direction | lecture sans écriture |
| `facturation::compte(debiteur)` / `solde(debiteur)` | 25, espace famille, 30 (IA) | lecture filtrée par droits |
| `facturation::explication(ligne)` | familles, IA | lecture (« d'où vient cette ligne ») |
| `facturation::imputer(piece, montant, origine)` | 25 | commande (lettrage) |
| `facturation::etat_contributions(annee)` | direction, inspection | lecture agrégée |
| `facturation::ecritures(periode)` | 48 | lecture des écritures de ventes |

### 6.4 Ports consommés [R]
`eleve::scolarite_a(eleve, date)` (2) ; `inscriptions::validee` et `scolarite.depart_*` (3) ; `personnes::payeurs`, `personnes::destinataires(eleve, nature, date)`, adresse du débiteur (4) ; `structure::annee`, `niveaux`, `calendrier` (5) ; `documents::emettre(nature, trame, données)` et coffre (7) ; `notifications::avis(nature)` (8) ; faits de consommation (40–43) ; décisions d'aide (26) ; `ent-ia` (registre de capacités).

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]
| Terme | Définition | Table cible |
|---|---|---|
| **Compte débiteur** | Le compte d'un payeur (personne ou organisme) dans l'établissement ; reçoit pièces, échéances, règlements ; remplace le `foyer_id` comme destinataire comptable (le `foyer_id` reste pour la compatibilité, socle 4 Q-A1) | `ent_fin_compte` |
| **Payeur** | Fait du socle 4 : qui paie quelle part de quelle catégorie de frais pour quel élève, sur quelle période | `ent_eleve_payeur` (socle 4) |
| **Engagement financier** | Pour un élève et une année : formule (régime, options payantes, internat), période, contrat signé, conditions particulières ; c'est la « fiche financière de l'élève » | `ent_fin_engagement` |
| **Article** | Ce qui est facturé (catégorie, **nature légale**, régime de TVA, compte de produit) | `ent_fin_article` |
| **Tarif** | Prix d'un article pour une année, dans une **version de grille** (brouillon, votée, en vigueur, close), par niveau, régime, tranche de quotient | `ent_fin_tarif`, `ent_fin_grille_version` |
| **Source** | Le fait qui fonde une ligne (inscription, régime, consommation, voyage, aide, geste motivé), avec sa **clé** unique | `ent_fin_source` |
| **Ligne calculée** | Ligne à facturer, non encore sur une pièce, avec son **explication** | `ent_fin_ligne_attente` |
| **Pièce** | Facture ou avoir émis, numéroté, figé, rendu en document | `ent_fac_facture` enrichie |
| **Échéancier** | Le découpage d'une pièce en échéances datées | `ent_fac_echeance` |
| **Solde** | Total des pièces émises − imputations (règlements, avoirs) ; un **crédit** est un solde négatif | calculé |
| **Exigibilité** | Date à laquelle une échéance est due (point de départ du retard et de la prescription) | `ent_fac_echeance.date_echeance` |
| **Remise d'ordre** | Réduction due à la famille pour une période non servie (absence longue justifiée, stage, fermeture) | source `remise_ordre` |
| **Aide affectée** | Somme d'un tiers (bourse avec procuration, fonds social, aide locale, CE) imputée sur des lignes précises | source `aide` (socle 26) |
| **Période comptable** | Mois ou exercice ; **close** = plus aucune pièce datée dedans | `ent_fin_periode` |

### 7.2 Quatre distinctions qui manquent aujourd'hui [R]
1. **Foyer ≠ débiteur** : le foyer est un lieu de vie (socle 4) ; le débiteur est un compte de payeur.
2. **Calculé ≠ émis** : une ligne calculée se recalcule ; une ligne émise ne bouge plus.
3. **Libellé ≠ nature** : « frais de scolarité » pour la famille, `contribution_r442_48_2` pour le contrôle.
4. **Dû ≠ exigible ≠ en retard** : une facture annuelle est due, ses échéances deviennent exigibles mois par mois, seul l'exigible non réglé est en retard (et seul lui peut être relancé).

---

## 8. De la scolarité à la facture

### 8.1 La chaîne [R]
```
faits (2, 3, 4, 26, 40–43)            ex. inscription validée, régime DP au 01/09, 12 repas en novembre
   │  événement outbox (même transaction que le fait)
   ▼
source (clé unique, empreinte des données utiles)      ent_fin_source
   │  calcul pur (tarif de la version en vigueur, prorata, réductions, aides, part du payeur)
   ▼
lignes en attente, par compte débiteur, avec explication   ent_fin_ligne_attente
   │  contrôle (anomalies, écarts, seuils) — file de travail de la comptabilité
   ▼
pièce brouillon (regroupement par compte, origine, période)
   │  émission N3 (geste humain, lot vérifié) — numéro continu, document rendu, empreinte
   ▼
pièce émise (figée) + échéancier + avis au payeur (socle 8) + dépôt au coffre (socle 7)
   │  25 : prélèvement, règlement, rejet, relance ; imputation par lettrage
   ▼
pièce soldée / compensée par avoir                          écritures → socle 48
```
- **Recalcul** : tant qu'une ligne est **en attente**, un fait corrigé (régime antidaté, absence modifiée) la **recalcule** (même clé de source, nouvelle empreinte) ; une fois **émise**, la correction produit un **avoir** (différence) et, s'il le faut, une ligne complémentaire. Corrige FD-21.
- **Regroupement** : par défaut, une facture annuelle par compte débiteur (tous les enfants du payeur, une section par enfant) ; une facture mensuelle de consommations ; une facture par voyage (ou intégrée à la mensuelle, réglage S-152).
- **Seuil d'émission** : une ligne de consommation inférieure à un seuil (0,50 € par défaut, réglage) attend la facture suivante.

### 8.2 Calcul [F pour l'existant, R pour les ajouts]
1. **Tarif** : version de grille **en vigueur à la date de la prestation** ; le plus spécifique (niveau × régime × tranche) gagne [F] ; un foyer sans quotient prend le tarif sans borne haute [F] — **corrigé [R]** : prend le **tarif plein** déclaré comme tel (`tarif_par_defaut`), pas « le premier sans borne », et l'explication dit « quotient non fourni ».
2. **Prorata** au mois commencé, arrondi à l'inférieur pour la famille [F] ; option « à la semaine » ou « au jour » pour les remises d'ordre [F jours calendaires] ; **mois de départ** dû si commencé (réglage S-153 : dû / non dû si départ avant le 15).
3. **Réductions** : en taux ou en montant, sur des catégories, **avec règle de cumul** (meilleure seule, cumul plafonné, cumul libre) ; plancher : une ligne ne descend jamais sous 0 (FD-16) ; les réductions ne s'appliquent jamais à une nature où la loi l'interdit [R] (aucune aujourd'hui).
4. **Part du payeur** : chaque compte reçoit `part × montant calculé sur son propre quotient` [F, choix documenté] ; répartition au plus fort reste quand un même quotient s'applique ; l'explication montre « votre part : 40 % (désignation du 02/09) ».
5. **Aides** : imputées **après** les réductions, sur les lignes et le payeur que l'aide vise (§10.5) ; jamais au-delà de ces lignes.
6. **Arrondi d'échéancier** : parts égales, le reste en centimes sur la **première** échéance [F] (conservé : l'usage est que la famille paie le reste au début).
7. **Aucune ligne n'est saisie sans source** : une ligne manuelle est une source `geste_motive` (motif obligatoire, article obligatoire, montant borné, double contrôle au-delà d'un seuil, S-154).

### 8.3 Simulation et contrôle avant émission [R]
- **Aperçu de génération** [F, transaction abandonnée] enrichi : totaux par niveau, régime, catégorie, nature légale ; **écarts** avec l'année précédente (famille par famille : « +312 € : passage en internat ») ; familles **sans payeur**, **parts ≠ 100 %**, **payeur sans adresse**, **mandat absent** pour un mode prélèvement ; fratries **incohérentes** (deux foyers payeurs différents pour des frères) ; **tarif plein** appliqué faute de quotient ; élèves facturés **sans engagement signé**.
- **Anomalies** déterministes (règles écrites, pas un modèle) : montant hors de l'intervalle habituel de son niveau, ligne en double par article et période, consommation sans régime, remise supérieure à la ligne ; l'IA peut **expliquer** une anomalie (§16), jamais la lever.
- **Émission par lots** : la comptabilité émet « 412 factures, 381 207,50 € » après relecture de l'aperçu, d'un échantillon de 5 documents tirés au hasard et de la liste des anomalies ; N3 ; possibilité d'émettre en excluant les familles en anomalie.

### 8.4 Quand les avis partent [R]
Émission ≠ envoi. Les avis (`finances.facture_emise`) suivent le socle 8 : plage calme 20 h–7 h et week-end [V, socle 16 §5], **enveloppe neutre** (« un document financier est disponible », pas le montant) par e-mail, SMS, push ; document lu authentifié ou par **lien nominatif** pour un payeur sans compte (socle 7 R-D8) ; **courrier** si aucun canal (avec coût, socle 8).

---

## 9. Payeurs, débiteurs, familles séparées

### 9.1 Le compte débiteur [R]
- Un **compte** par payeur **et par établissement** (D-013 : aucune visibilité entre établissements) : `personne_id` **ou** `organisme_id` (socle 4) ; `foyer_id` historique conservé (compatibilité de toutes les tables `ent_fac_*`, Q-A1 du socle 4) ; adresse de facturation = adresse du foyer du payeur (socle 4) ou adresse de l'organisme ; canaux d'avis = ceux de la personne (usage `facturation`, socle 4 §8.5) ; mode de paiement (prélèvement / manuel / en ligne), géré par le 25.
- Un compte peut couvrir **plusieurs élèves** (fratrie, élèves d'un organisme) et **plusieurs catégories**.
- **Reprise** (F2) : chaque `ent_foyer` qui a des pièces devient un compte (payeur = personnes du foyer avec `est_payeur`), sans renuméroter les pièces ; un foyer qui réunit deux payeurs **séparés** (cas existant possible) est signalé, pas éclaté automatiquement (CF-09).

### 9.2 Désignation et changements (lecture du socle 4) [R]
- La facturation lit `personnes::payeurs(eleve, categorie, date)` **à la date de chaque prestation** : une facture annuelle couvrant septembre–juin est **découpée** si la désignation change en cours d'année (part avant / après la date d'effet).
- Sur `payeurs.modifies` : **aucune pièce émise n'est touchée** (socle 4 §8.9) ; le 24 calcule les **conséquences proposées** : avoir au payeur sortant pour les périodes futures déjà facturées, facture au payeur entrant, report d'échéances ; la comptabilité valide (N3) ; le payeur sortant voit « vous n'êtes plus payeur de la restauration de Léo à partir du 01/02 : avoir de 312,00 € ».
- **Payeur manquant** : un élève dont une catégorie n'a pas 100 % de payeurs n'est **pas** facturé pour cette catégorie ; une tâche part au secrétariat (« Léo : restauration sans payeur depuis le 01/02 ») ; rien n'est facturé « par défaut au responsable 1 ».
- **Séparation en cours d'année** (C22 du socle 4) : l'assistant du socle 4 redéfinit les payeurs ; le 24 présente l'effet financier **avant** validation (« le père sera facturé de 50 % de la scolarité à partir du 01/12 : 1 040,00 € sur 7 mois ») ; nouveau mandat pour le nouveau payeur (25).

### 9.3 Ce que voit chacun [R]
| Personne | Voit | Ne voit jamais |
|---|---|---|
| Payeur (personne avec compte) | ses pièces, ses lignes **pour les élèves et catégories dont il est payeur**, son solde, ses échéances, ses documents, son mode de paiement, son mandat masqué | le compte d'un autre payeur, même pour le même enfant ; les lignes d'un enfant du foyer dont il n'est pas payeur (corrige le [S] de l'audit S2-9 §3) ; l'adresse d'un autre payeur |
| Parent titulaire non payeur | **rien de financier** par défaut ; s'il est activé (R-F8, S-155), un **relevé informatif** des frais de l'enfant (sans montants payés, sans payeur, sans adresse) | factures, soldes, relances, IBAN de l'autre |
| Élève majeur payeur | comme un payeur | — |
| Élève majeur non payeur / élève mineur | **rien** (Q-F9) ; le solde de son porte-monnaie de cantine (25) | factures de ses parents |
| Payeur sans compte | ses pièces par **lien nominatif** à usage limité (socle 7 R-D8), par courrier sinon | — |
| Organisme payeur | ses factures (adresse et référence de l'organisme, nom de l'élève, prestation, période) | toute donnée scolaire |
| Personnel financier (droit `facturation.lire`) | tous les comptes de l'établissement | IBAN en clair (sauf 25, droit spécifique) ; motifs sociaux des aides (anonymisés, socle 26) |
| Autres personnels (professeur, CPE, vie scolaire) | **rien** (pas même « impayé ») ; l'organisateur d'un voyage voit un état **agrégé** (§12) | tout le reste |

### 9.4 Payeurs particuliers [R]
- **Organisme** (ASE, mairie, employeur, comité d'entreprise, ambassade) : facture à son nom, référence de commande ou de prise en charge, SIRET ; plafond éventuel (« prise en charge 50 % dans la limite de 800 € ») ; part non couverte → payeur suivant désigné par le socle 4 (ou tâche) ; **organisme public** → dépôt Chorus Pro [NV] (F11).
- **Prise en charge partielle par l'employeur d'un parent** : ce n'est pas un payeur, c'est une **attestation de paiement** (25, PAY-180) ; sauf si l'employeur paie directement (alors payeur).
- **Élève majeur** (socle 2 `eleve.majeur`) : ses parents **restent** payeurs s'ils le sont (socle 4 C12) ; l'élève peut être désigné payeur (fiche personne) ; F2 rattache sa fiche payeur à **son compte élève** (FD-15).
- **Personnel de l'établissement parent d'élève** : réduction « personnel » (nature `personnel`) attachée à la **personne** (agent du socle 6), pas à l'élève ; elle tombe à la fin de l'engagement de l'agent (événement du socle 6) avec proposition d'effet daté.
- **Correspondant d'un interne dont la famille est à l'étranger** (socle 4 C36) : n'est **pas** payeur sauf désignation.

---

## 10. Tarifs, réductions, aides

### 10.1 Catalogue d'articles [R]
- Articles fournis par ScolIA comme **modèles** (D-023 : prêts à l'emploi), copiés et renommés par l'établissement (D-022 : données libres) : contribution des familles (scolarité), cotisation (APEL : **encaissement pour compte de tiers** [H], nature `pour_compte_tiers`, hors chiffre d'affaires), demi-pension (forfait ou au repas), internat, externat surveillé, études surveillées, garderie, activités, sorties, voyages, fournitures, frais de dossier, frais d'examen, assurance scolaire (pour compte de tiers), caution (badge, casier, matériel ; **dépôt** remboursable, pas une vente), carte de cantine perdue, manuels.
- Chaque article : catégorie (pour les payeurs, socle 4), **nature légale** (§5.1), **régime de TVA** (§5.3), compte de produit (socle 48), unité (année, mois, repas, nuit, séance, unité), mode (forfait / réel), remboursable ou non, soumis aux réductions ou non, soumis aux remises d'ordre ou non.

### 10.2 Grilles et versions [R]
- **Version de grille** par année : `brouillon` → `proposée` (au conseil d'administration) → `votée` (date et référence de la délibération, pièce au socle 7) → `en vigueur` → `close`. Une version votée **ne se modifie plus** ; une correction = nouvelle version datée (FD-19).
- Dimensions : niveau (socle 5), régime, tranche de quotient (bornes [min, max[ [F]), ordre de l'enfant (si l'établissement facture « par rang » plutôt que par réduction), statut (interne de semaine / week-end), site.
- **Simulateur** : appliquer une hausse (% ou montant) par article, arrondir (à l'euro, à 5 €), voir le produit attendu sur les effectifs prévus (socle 3) et l'effet par famille type ; **comparer** deux scénarios ; exporter le tableau pour le conseil d'administration (FAC-120 à FAC-124).
- **Publication** : grille publique (site, socle 17) et simulateur public de tarifs (T-18) **sans donnée personnelle** (quotient saisi par le visiteur, rien n'est enregistré).

### 10.3 Réductions [R]
- **Fratrie** : règle de rang choisie par l'établissement (à partir du 2e / 3e ; ordre : du plus âgé, du plus jeune, de l'inscription la plus ancienne) ; **périmètre** : enfants du **même payeur** dans l'établissement (par défaut) ; ensemble scolaire seulement en « groupe » explicite et consenti (`SOCLES.md` Q13, D-013) ; la réduction s'applique à l'enfant de rang N (pas à toute la fratrie) ; explication : « 3e enfant de M. X dans l'établissement : −20 % sur la contribution ».
- **Personnel**, **ancien élève**, **association partenaire**, **geste commercial** (motif) : accordées à un élève ou une personne, datées, plafonnées, avec **décideur** ; au-delà d'un seuil, double validation (S-156).
- **Cumul** : règle par réduction (exclusive, cumulable, cumul plafonné à X %).
- **Remise d'ordre** : déclenchée par une absence justifiée longue (seuil 15 jours [F]), un stage, une fermeture du service (socle 5 : fermeture exceptionnelle), une exclusion (le règlement financier dit si elle donne droit à remise, réglage S-157) ; au prorata des jours **de service** (pas calendaires, R-F12) ; proposée, validée par la comptabilité (N2) ; jamais une information sur le motif de l'absence (« remise d'ordre : 18 jours » sans « maladie »).

### 10.4 Quotient familial [R]
- Le **calcul** et la pièce justificative sont au socle 26 (avec le 7) ; le 24 lit `quotient(compte, annee) -> tranche` et **ne stocke pas** le revenu ; l'explication dit « tranche C (quotient fourni le 12/07) », jamais le revenu.
- Sans quotient : tarif plein déclaré (§8.2) ; un quotient fourni en cours d'année s'applique **à partir du mois suivant** (réglage S-158 : rétroactif ou non) avec avoir proposé.
- Deux payeurs séparés : **chacun son quotient** [F, conservé].

### 10.5 Bourses et aides affectées [R, §5.7]
- La bourse est une **aide affectée** venant du socle 26 : (bénéficiaire = payeur qui a donné procuration, montant, période, lignes cibles `demi_pension` / `internat`) ; imputée à hauteur de ces lignes ; excédent → **reversement** à la famille (25, PAY-202 et PAY-204) ; prime d'équipement → toujours reversée.
- Bourse au **trimestre échu** [V partiel] : la déduction est **prévue** dès l'échéancier (la famille ne paie que le net, réglage S-159 par défaut « déduction anticipée ») et **constatée** au versement de l'État ; un retrait de bourse produit une facture de régularisation **proposée**, jamais émise sans geste.
- Fonds social, aide de la région, du département, de la mairie, du CE : même mécanique (aide affectée, lignes cibles, payeur) ; la **décision** et son motif restent au socle 26 (anonymisés côté comptable : « aide n° 2026-014 »).

---

## 11. Consommations et services (socles 40, 41, 43)

- **Restauration** (40) : forfait (par régime, au mois) ou **réel** (passages / réservations × tarif du repas) [F FIN-18] ; le socle 40 publie chaque jour le **volume facturable** par élève (`restauration.consommations_jour`) ; le 24 agrège au mois ; un repas **réservé non pris** est facturé ou non selon le règlement (réglage S-160, avec délai de prévenance du socle 40) ; un repas **non réservé pris** au tarif majoré si prévu ; **PAI panier repas** : tarif spécifique (article dédié), la nature médicale n'est **jamais** visible (« tarif panier repas ») ; **porte-monnaie prépayé** : débit au passage côté 25 (PAY-206), la facture mensuelle n'est alors qu'un relevé.
- **Internat** (41) : forfait au trimestre ou au mois ; nuitées supplémentaires ; week-ends ; remise d'ordre pour fermeture.
- **Études, garderie, activités, transports** (43) : inscription à une activité = engagement daté ; facturation au forfait ou à la séance (appel du socle 12) ; transport scolaire souvent régional (import, pas facturation [H]).
- **Règle de transmission** : un service transmet des **quantités** (et le régime, la date), jamais un montant ; il peut annuler une consommation (même clé) avant émission ; après émission, l'annulation produit un **avoir** de la ligne.
- **Contrôle du mois** : avant la facture mensuelle, la comptabilité voit les variations (« 37 familles : +20 % de repas »), les élèves avec consommations sans régime, les consommations d'élèves partis.

---

## 12. Sorties et voyages (socle 42 possède l'organisation)

- Le socle 42 crée le **voyage** (budget, participants, dates, coût par élève, financements : établissement, APEL, collectivité, ventes, aides) et décide de la **part familles** ; le 24 facture cette part par participant (source `voyage:{id}:{eleve}`) avec un **échéancier propre** (acompte à l'inscription, soldes) ; le 25 encaisse (chèques en plusieurs fois, chèques-vacances si acceptés [NV sur l'éligibilité des séjours scolaires], carte).
- **Désistement** : règle du voyage (avant / après engagement des dépenses, motif légitime — maladie, deuil — selon la jurisprudence des clauses abusives, §5.1) → avoir total, partiel ou nul **proposé**, validé par l'organisateur et la comptabilité.
- **Aides** : caisse de solidarité, fonds social (26), APEL → aides affectées sur la ligne du voyage ; l'organisateur voit le **coût résiduel agrégé**, jamais qui est aidé (R-F19).
- **État de paiement pour l'organisateur** : « 23 à jour, 2 en retard, 1 plan en cours » ; **aucun nom** n'apparaît associé à un retard pour un professeur (réglage S-161 : la direction peut ouvrir la liste nominative au seul responsable du voyage, journalisé) ; **jamais d'exclusion d'un élève d'un voyage pour impayé sans décision de la direction** et dialogue avec la famille (R-F20).
- **Clôture du voyage** : bilan réel vs budget ; excédent par élève remboursé ou non selon la règle annoncée (réglage S-162) ; analytique « voyage » vers le socle 48.

---

## 13. Pièces, numérotation, avoirs, documents

### 13.1 Numérotation [R, §5.2]
- **Séries** fixées par ScolIA (une par type et par établissement) : factures `F`, avoirs `A`, plus la série de quittances du 25 ; continuité **dans chaque série** garantie par le compteur transactionnel [F] et une contrainte de **non-trou** vérifiée chaque nuit (FAC-200) ; numéro attribué **à l'émission**, jamais au brouillon (corrige FD-05).
- **Format** : `F2026-000123` (année de la série) ; série **par exercice** de l'OGEC ou **par année civile** (Q-F12 : par défaut année civile [F], aligné sur la pratique comptable ; réglage S-163) ; la série `FIN` du socle 7 (`07_DOCUMENTS.md` ligne 547) réutilise ces numéros.
- Les pièces historiques à numéro libre sont **conservées telles quelles** et marquées `serie = 'historique'` (aucune renumérotation d'une pièce émise).

### 13.2 Immutabilité [R]
- Déclencheurs en base (F0) : sur une pièce `emise` ou `soldee`, aucun `UPDATE` de montant, de lignes, de débiteur, de numéro, de date d'émission ; seules des colonnes de **suivi** changent (statut de solde, date de solde) ; aucun `DELETE` sur `ent_fac_*`, `ent_fin_*` financiers ; `ON DELETE CASCADE` remplacés par `RESTRICT` (FD-08, FD-09).
- **Brouillon** : modifiable et supprimable (il n'a pas de numéro) ; sa suppression est journalisée.
- **Annulation d'une facture** = avoir total (même date ou date du jour), la facture reste visible avec la mention « compensée par l'avoir A2026-000045 ».

### 13.3 Avoirs [F + R]
- Total, partiel au prorata des lignes [F], ou **par ligne** [R] ; motif obligatoire (liste fermée + texte) ; imputé sur la facture d'origine si elle est due, sinon **crédit** du compte (25 : remboursement ou imputation sur les factures suivantes **avec l'accord du payeur ou selon le règlement financier**, Q-F14) ; un avoir ne dépasse jamais la facture d'origine (contrôle) ; un avoir ne se compense **jamais** sur le compte d'un autre payeur.

### 13.4 Documents [R, socle 7]
- Chaque pièce émise produit un **document** (trame D-023, 3 ou 4 styles, identité de l'émetteur = établissement ou OGEC, D-027 F) stocké au socle 7 (PDF/A, empreinte), **déposé au coffre du payeur** ; jamais régénéré : le document remis est le document conservé (FD-18).
- **Contenu** : mentions du §5.2 ; une **section par élève** ; pour chaque ligne : libellé, période, quantité, prix, réduction, aide, **part du payeur** ; motif d'exonération de TVA ; échéancier ; modes de paiement ; **référence de paiement** (pour virement, 25) ; contact de la comptabilité ; QR de vérification (socle 7) ; mention « la part de l'autre responsable lui est facturée séparément » si répartition.
- **Ce qui n'y figure jamais** : motif d'une absence, d'une exclusion, d'une aide, régime médical (PAI), l'autre payeur, son adresse, sa part en euros.
- **Relevé de compte** (situation à une date), **échéancier seul**, **duplicata** marqué « duplicata » (même document, même numéro), **attestation de situation** (« quitus », socle 7 R-D4, PAY-182).

### 13.5 Périodes et clôture [R]
- **Période mensuelle** close par la comptabilité (N3) après contrôle : plus aucune pièce ne peut être datée dans une période close ; une correction tardive est datée du jour (avoir) ; les écritures de la période deviennent **définitives** (export « définitif » vers le 48 avec numéro d'export continu et empreinte [F]).
- **Exercice** de l'OGEC (souvent septembre–août [H]) : clôture annuelle au socle 48 ; le 24 fournit l'état des **produits constatés d'avance** (factures annuelles émises en août pour septembre–juin) si l'expert-comptable le demande (FAC-198) [H].

---

## 14. Compte famille (web et appli mobile)

- **Accueil financier** du payeur : « vous devez 312,00 € au 05/12 (prélèvement prévu) » ; **solde**, **prochaines échéances**, **retards** (ton neutre), **mode de paiement** et mandat masqué, **documents** (factures, avoirs, quittances, attestations), **payer** (25 : en ligne, virement avec référence, instructions chèque).
- **« D'où vient cette ligne ? »** : explication en langage clair générée depuis l'explication enregistrée (pas par un modèle de langage : phrases fixes D-023) — « Demi-pension de Léo, octobre : forfait 4 jours, tranche C (quotient du 12/07), votre part 50 % → 38,40 € » ; bouton **« contester cette ligne »** qui ouvre un fil vers la boîte « comptabilité » (socle 16) avec la ligne en carte.
- **Par enfant** et **pour moi** ; un parent avec des enfants dans deux établissements voit **deux comptes séparés** (un par établissement, D-013), sans totalisation entre établissements.
- **Appli mobile** (D-003, version quotidienne) : solde, échéances, documents, paiement en ligne, attestation en un geste ; **pas** de paramétrage ni de contestation complexe (lien web).
- **Langues** : documents et explications dans les langues du socle 8 (familles allophones) ; montants et dates au format français dans le document officiel.
- **Accessibilité** : documents PDF/A accessibles (socle 7), montants lus correctement par les lecteurs d'écran.

---

## 15. Cas limites (catalogue)

Chaque cas a une règle (R-F / R-P) et un test. Les cas propres aux paiements (rejets, chèques, espèces, prélèvements) sont au §13 de `25_PAIEMENTS.md` (CF-41 à CF-72).

### 15.1 Familles et payeurs
| # | Situation | Traitement |
|---|---|---|
| CF-01 | Parents séparés, 50/50 sur tout | deux comptes, deux factures de 50 %, chacun son quotient, sa fratrie, son mandat ; aucun ne voit l'autre |
| CF-02 | La mère paie la scolarité, le père la cantine | catégories distinctes (socle 4 §8.9 : précis > `toutes`) ; deux factures ; consommations de cantine au seul père |
| CF-03 | Séparation au 01/12, factures annuelles émises en août | aucune pièce touchée ; conséquences proposées : avoir à la mère sur décembre–juin pour 50 %, facture au père ; validation N3 ; nouveau mandat du père (25) |
| CF-04 | Le père, non payeur, demande « combien coûte la scolarité » | relevé informatif seulement si R-F8 activé ; sinon la grille publique ; jamais le compte de la mère |
| CF-05 | Grand-mère payeuse sans lien de responsabilité | compte à son nom (foyer `personne_seule`, socle 4) ; factures et lien nominatif ; aucune donnée scolaire |
| CF-06 | ASE paie l'internat, les parents la contribution | organisme payeur de la catégorie internat ; facture à l'organisme (Chorus Pro [NV]) ; adresse d'accueil protégée **jamais** imprimée (socle 4 §12.3) |
| CF-07 | Beau-père payeur, sans autorité | payeur : il voit ses factures, rien d'autre (aucune note, absence) |
| CF-08 | Élève majeur qui paie lui-même ses frais | compte rattaché à son compte élève (F2) ; ses parents ne voient plus ces factures |
| CF-09 | Foyer historique qui réunit deux parents désormais séparés | reprise : signalé « à vérifier » ; aucune réaffectation automatique des pièces passées |
| CF-10 | Parent décédé payeur | socle 4 ferme sa désignation à la date ; **toutes les relances et prélèvements programmés sont suspendus** (avec 25) ; tâche « reprise des paiements » à la comptabilité ; aucun envoi automatique ; avoir et succession traités humainement |
| CF-11 | Décès de l'élève | gel immédiat de toute facturation future, de toute relance (socle 8 CN-13) ; avoir des périodes futures proposé ; contact humain uniquement |
| CF-12 | Parent visé par une interdiction de communication, payeur | il reste débiteur (fait juridique) ; il reçoit ses **seules** pièces financières par courrier ou lien nominatif, aucune donnée scolaire ; avis juridique (Q-F22) |
| CF-13 | Parent « à ne pas solliciter » (socle 8 Q-N9) | factures = information due : selon la décision de la direction (coffre / courrier) ; relances = sollicitation → **tâche humaine** au lieu d'un envoi automatique |
| CF-14 | Payeur sans adresse ni canal | pièce émise, document au coffre, **tâche** « adresse de facturation manquante » ; pas de prélèvement possible sans pré-notification (25) |
| CF-15 | Payeur à l'étranger | adresse étrangère structurée ; paiement par virement SEPA ou international ; pas de prélèvement hors zone SEPA |
| CF-16 | Parts 70/30 mais quotients différents | chaque part sur son propre tarif [F] ; explication claire ; la somme ne correspond à aucun tarif unique : affiché à la comptabilité dans l'aperçu |
| CF-17 | Fratrie : deux enfants chez la mère payeuse, un chez le père payeur | rang par payeur [F] : la mère a 2 enfants, le père 1 ; réduction fratrie de la mère seulement (règle par défaut, réglage S-164) |
| CF-18 | Frère dans le collège de l'ensemble scolaire, autre établissement ScolIA | aucune visibilité entre établissements (D-013) ; réduction fratrie inter-établissements **seulement** par groupe consenti (Q13) ou par déclaration de la famille avec pièce |
| CF-19 | Famille d'accueil (assistant familial) | n'est pas payeur sauf désignation ; reçoit les informations du quotidien, pas les factures |
| CF-20 | Personnel de l'établissement qui part en cours d'année | la réduction « personnel » tombe à la date de fin d'engagement (socle 6) ; complément proposé, non émis automatiquement |

### 15.2 Scolarité et facturation
| # | Situation | Traitement |
|---|---|---|
| CF-21 | Arrivée le 18 novembre | prorata : novembre dû (mois commencé) → 8/10 [F] |
| CF-22 | Départ le 10 mars | avoir des mois après la sortie [F] ; mois de mars selon S-153 ; solde final et quitus |
| CF-23 | Passage externe → demi-pensionnaire au 1er octobre | ligne de demi-pension à partir d'octobre ; si déjà émise en forfait annuel : complément ; inverse : avoir [F] |
| CF-24 | Changement de régime antidaté (saisie tardive) | lignes en attente recalculées ; pièces émises : avoir / complément datés du jour, période d'origine dans le libellé |
| CF-25 | Absence justifiée de 20 jours | remise d'ordre au prorata des jours de service (R-F12) ; aucun motif |
| CF-26 | Absence corrigée après émission de la remise d'ordre | nouveau calcul : avoir complémentaire ou facture de régularisation proposés (corrige FD-21) |
| CF-27 | Stage en entreprise de 4 semaines (lycée professionnel) | remise d'ordre restauration / internat si le règlement le prévoit (S-157) |
| CF-28 | Fermeture du lycée une semaine (crise) | fermeture du socle 5 → remise d'ordre proposée à **tous** les demi-pensionnaires et internes, en un lot |
| CF-29 | Exclusion temporaire de 8 jours | remise d'ordre selon règlement (S-157) ; jamais « exclusion » dans le libellé |
| CF-30 | Élève inscrit sans engagement signé | facturé seulement si le réglage le permet (S-165, défaut : bloqué avec tâche) |
| CF-31 | Inscription annulée après émission de la facture annuelle | avoir total ; acompte : conservé ou remboursé selon le contrat (S-166) |
| CF-32 | Réinscription refusée par la famille après versement de l'acompte | règle du contrat (acompte non remboursable sauf motif légitime) ; décision humaine motivée |
| CF-33 | Quotient fourni en janvier | à partir de février (S-158) ; avoir proposé si rétroactif |
| CF-34 | Bourse accordée en décembre, rétroactive au 1er septembre | aide affectée sur la demi-pension de sept–déc du payeur bénéficiaire ; excédent reversé |
| CF-35 | Bourse, mais c'est l'autre parent qui paie la demi-pension | pas de déduction (bénéficiaire ≠ payeur) [V partiel] ; la bourse est versée à la famille bénéficiaire (25) |
| CF-36 | Facture contestée : tarif erroné | avoir + nouvelle facture (D-027 E, socle 7 D75) |
| CF-37 | Erreur sur 400 factures (mauvaise grille) | avoir en lot + réémission en lot, N3, avis unique et explication aux familles (« correction de tarif ») ; **aucune** suppression |
| CF-38 | Facture d'une période close | refusée ; la correction se fait par un avoir daté du jour |
| CF-39 | Élève qui change d'établissement dans l'ensemble scolaire | solde ici, nouvelle facturation là-bas ; aucun transfert de solde entre établissements sans accord écrit de la famille |
| CF-40 | Repas PAI panier | article « panier repas », aucune mention médicale |

---

## 16. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §6.2, §8)

### 16.1 Ce que l'IA fait [R]
- **Expliquer** une ligne, une facture, un solde à la famille ou au secrétariat (à partir de l'explication enregistrée et des phrases fixes ; le modèle reformule, il n'invente aucun chiffre : tout montant affiché vient du service, vérifié à la sortie).
- **Contrôler** : résumer les anomalies déterministes de l'aperçu de génération (§8.3), proposer l'ordre de traitement, signaler une fratrie probablement oubliée (deux élèves au même payeur avec un rang incohérent) ; **les règles d'anomalie restent déterministes**, l'IA les commente.
- **Simuler** : « que rapporte +2 % sur la contribution et +0,10 € sur le repas ? », en appelant le simulateur (lecture).
- **Préparer** : un lot d'avoirs après une erreur de grille (brouillon de commande, **N3** à confirmer), un courrier de réponse à une contestation (brouillon, N3 si envoi), une note au conseil d'administration sur les tarifs (texte).
- **Répondre** à « combien je dois ? », « pourquoi ce montant ? » dans l'espace famille, avec les droits de la personne (jamais le compte d'un autre payeur).

### 16.2 Ce que l'IA ne fait pas [R]
- **Jamais** émettre, annuler, accorder une réduction, une remise ou une aide, changer un payeur, modifier un tarif, clore une période : elle prépare, une personne confirme (N3, passkey récente).
- **Jamais** de **score de risque** de paiement ni de profilage d'une famille (« mauvais payeur ») ; jamais de proposition fondée sur la situation sociale, la nationalité, le quartier.
- **Jamais** de données sociales (motif d'aide, situation familiale) ni d'IBAN dans le contexte ; montants et identités **pseudonymisés** pour les tâches de contrôle en lot (C2, `IA_COEUR.md` §7).

### 16.3 Capacités déclarées (registre `ent-ia`) [R]
| Capacité | Type | Droit | Sensibilité | Confirmation |
|---|---|---|---|---|
| `facturation.expliquer_ligne` | lecture | payeur ou `facturation.lire` | C2 | — |
| `facturation.solde` | lecture | payeur ou `facturation.lire` | C2 | — |
| `facturation.simuler_grille` | lecture | `facturation.tarifs` | C1 (agrégé) | — |
| `facturation.anomalies_generation` | lecture | `facturation.gerer` | C2 pseudonymisé | — |
| `facturation.preparer_avoirs_lot` | commande (brouillon) | `facturation.emettre` | C2 | N3 |
| `facturation.preparer_reponse_contestation` | commande (brouillon de message) | `facturation.gerer` | C2 | N3 à l'envoi |
| `facturation.etat_contributions` | lecture | `facturation.lire` | C1 | — |

### 16.4 Cadre
Les fonctions à modèle de langage attendent l'étude juridique (règlement IA, déjà demandée au socle 21, Q-EV8) et le choix du fournisseur (Mistral prioritaire, garanties contractuelles) ; les fonctions déterministes (anomalies, explication par phrases fixes) n'en dépendent pas et sont livrées d'abord.

---

## 17. Actions métier (FAC-100 et suivantes)

Abréviations : DIR (direction), OGEC (président, trésorier), CPT (comptable, économat, intendance), ADF (secrétariat), ACC (accueil), VS, CPE, RES (responsable restauration / internat), ORG (organisateur de voyage), PAY (payeur), PNP (parent non payeur), ELM (élève majeur), ORGA (organisme payeur), SYS (système), IA (assistant, toujours au nom d'une personne). ★ = confirmation renforcée N3 (passkey récente, aperçu complet). Droits : §19.1.

### 17.1 Paramétrage et catalogue
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-100 | Voir les paramètres de facturation | CPT, DIR · `facturation.lire` | — | — |
| FAC-101 | Modifier les paramètres (mois facturables, jour d'échéance, seuils) ★ | CPT · `facturation.parametrer` | versionné, motif ; effet futur seulement | `facturation.parametres_modifies` |
| FAC-102 | Choisir l'identité émettrice (établissement / OGEC, SIREN, adresse, mentions) ★ | DIR · `facturation.parametrer` | D-027 F ; contrôle SIREN ; mentions imprimées sur les nouvelles pièces | idem |
| FAC-103 | Créer un article depuis un modèle | CPT · `facturation.tarifs` | catégorie, nature légale, TVA, compte, unité, mode obligatoires | `facturation.article_cree` |
| FAC-104 | Modifier un article (libellé, compte) | CPT · `facturation.tarifs` | nature et TVA non modifiables si l'article a des lignes émises (nouvel article) | `facturation.article_modifie` |
| FAC-105 | Désactiver un article | CPT · `facturation.tarifs` | jamais supprimé ; reste sur les pièces | idem |
| FAC-106 | Voir l'usage d'un article (lignes, montants, années) | CPT · `facturation.lire` | — | — |
| FAC-107 | Définir les catégories de frais de l'établissement (sous-catégories des catégories logicielles) | CPT · `facturation.parametrer` | liste logicielle fermée + libellés libres ; lien avec les payeurs du socle 4 | — |
| FAC-108 | Paramétrer le calendrier d'échéances type de l'année (10 mensualités, 3 trimestres, comptant) | CPT · `facturation.parametrer` | dates dans l'année scolaire ; jour ≤ 28 [F] | `facturation.calendrier_modifie` |
| FAC-109 | Paramétrer les règles de prorata (mois commencé, quinzaine, au jour) | CPT · `facturation.parametrer` | bornes ; par catégorie | — |
| FAC-110 | Paramétrer les règles de remise d'ordre (seuil, motifs ouvrant droit, catégories) | CPT, DIR · `facturation.parametrer` | S-157 ; jamais de motif sur les pièces | — |
| FAC-111 | Paramétrer les séries et le format de numérotation ★ | CPT · `facturation.parametrer` | une fois par exercice ; jamais en cours de série | — |
| FAC-112 | Choisir la trame des documents financiers (D-023) | DIR, CPT · `facturation.parametrer` | 3–4 trames, identité seule | — |
| FAC-113 | Rédiger les textes libres encadrés (conditions de paiement, contact) | CPT · `facturation.parametrer` | longueur bornée ; aperçu | — |
| FAC-114 | Paramétrer le seuil d'émission des petites consommations | CPT · `facturation.parametrer` | défaut 0,50 € | — |
| FAC-115 | Paramétrer l'option « copie aux titulaires non payeurs » (relevé informatif) | DIR · `facturation.parametrer` | R-F8 ; défaut non | — |

### 17.2 Tarifs, simulation, vote
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-116 | Créer la grille de l'année N+1 en copiant N | CPT · `facturation.tarifs` | version brouillon | `facturation.grille_creee` |
| FAC-117 | Saisir / modifier un tarif (niveau, régime, tranche, montant) | CPT · `facturation.tarifs` | seulement en brouillon ; bornes [F] | — |
| FAC-118 | Définir les tranches de quotient de l'année | CPT · `facturation.tarifs` | bornes continues, sans trou ni chevauchement | — |
| FAC-119 | Désigner le tarif par défaut (sans quotient) de chaque article | CPT · `facturation.tarifs` | obligatoire si l'article a des tranches | — |
| FAC-120 | Appliquer une hausse en % ou en montant, avec arrondi | CPT, DIR · `facturation.tarifs` | sur brouillon ; aperçu avant/après | — |
| FAC-121 | Simuler le produit attendu sur les effectifs prévus | DIR, CPT, OGEC · `facturation.simuler` | effectifs du socle 3 ; aucune donnée nominative | — |
| FAC-122 | Comparer deux scénarios de grille | DIR, OGEC · `facturation.simuler` | tableau, écarts par famille type | — |
| FAC-123 | Exporter la proposition pour le conseil d'administration | DIR · `facturation.simuler` | tableau et note (trame) ; aucune donnée personnelle | — |
| FAC-124 | Voir l'effet d'une grille sur une famille type (2 enfants, DP, tranche C) | DIR, ADF · `facturation.simuler` | familles types déclarées, fictives | — |
| FAC-125 | Proposer la grille (gel du brouillon) | DIR · `facturation.tarifs` | plus modifiable sauf retour en brouillon motivé | `facturation.grille_proposee` |
| FAC-126 | Enregistrer le vote (date, référence, pièce de la délibération) ★ | DIR · `facturation.voter` | version figée ; pièce au socle 7 | `facturation.grille_votee` |
| FAC-127 | Mettre en vigueur une grille à une date | CPT · `facturation.tarifs` | une seule en vigueur par période ; prestations datées | `facturation.grille_en_vigueur` |
| FAC-128 | Corriger une grille votée par une nouvelle version | DIR · `facturation.voter` ★ | nouvelle version datée ; motif ; effet futur ou rétroactif proposé (avoirs / compléments) | idem |
| FAC-129 | Publier la grille et le simulateur public | DIR · `facturation.publier` | socle 17 ; sans donnée personnelle | `facturation.grille_publiee` |
| FAC-130 | Consulter l'historique des grilles et des votes | DIR, CPT, OGEC · `facturation.lire` | — | — |

### 17.3 Réductions et aides
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-131 | Créer une réduction (fratrie, personnel, ancien élève, partenaire, geste) | CPT · `facturation.tarifs` | taux ou montant, catégories, cumul, plafond | `facturation.reduction_creee` |
| FAC-132 | Définir la règle de rang de fratrie et son périmètre | CPT, DIR · `facturation.parametrer` | S-164 ; payeur par défaut | — |
| FAC-133 | Accorder une réduction à un élève ou une personne | CPT · `facturation.reductions` | datée, motif, décideur ; double validation au-delà du seuil (S-156) | `facturation.reduction_accordee` |
| FAC-134 | Retirer une réduction (à une date) | CPT · `facturation.reductions` | jamais rétroactif sans avoir/complément proposé | `facturation.reduction_retiree` |
| FAC-135 | Voir les réductions accordées et leur coût | DIR, CPT · `facturation.lire` | agrégé et nominatif selon droit | — |
| FAC-136 | Recevoir une aide affectée (bourse, fonds social, aide locale) du socle 26 | SYS | lignes cibles, payeur bénéficiaire, période | `facturation.aide_imputee` |
| FAC-137 | Voir l'imputation d'une aide (lignes, excédent à reverser) | CPT · `facturation.lire` | motif social absent (anonymisé) | — |
| FAC-138 | Proposer la régularisation d'une aide retirée | SYS → CPT | proposition seulement ; émission N3 | `facturation.regularisation_proposee` |

### 17.4 Engagements et génération
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-139 | Créer l'engagement financier d'un élève à la validation de l'inscription | SYS (sur `inscription.validee`) | formule, régime, options payantes, contrat ; payeurs lus au socle 4 | `facturation.engagement_cree` |
| FAC-140 | Voir la fiche financière d'un élève (engagement, payeurs, lignes, pièces par payeur) | CPT, ADF · `facturation.lire` | la vue sépare les payeurs | — |
| FAC-141 | Modifier un engagement (option payante, internat de semaine) | CPT, ADF · `facturation.engagements` | datée ; effet calculé et affiché avant validation | `facturation.engagement_modifie` |
| FAC-142 | Enregistrer les conditions particulières (accord de la direction, échéancier dérogatoire) | DIR · `facturation.engagements` | motif ; visible du comptable ; jamais de motif social en clair | idem |
| FAC-143 | Lancer la **simulation** de la rentrée (aperçu sans écriture) | CPT · `facturation.gerer` | [F] enrichi §8.3 | — |
| FAC-144 | Voir les écarts avec l'année précédente, famille par famille | CPT · `facturation.gerer` | tri par écart ; explication | — |
| FAC-145 | Voir et traiter la liste des anomalies de génération | CPT · `facturation.gerer` | sans payeur, parts ≠ 100 %, sans adresse, sans mandat, fratrie, tarif plein, sans engagement | — |
| FAC-146 | Générer les lignes de l'année (écriture des lignes en attente) | CPT · `facturation.gerer` | idempotent (clés de source) [F] | `facturation.generation_annuelle` |
| FAC-147 | Regénérer après correction (recalcul des lignes en attente) | CPT · `facturation.gerer` | seulement les lignes non émises | idem |
| FAC-148 | Exclure temporairement un compte de la génération (en attente de régularisation) | CPT · `facturation.gerer` | motif ; tâche de suivi | — |
| FAC-149 | Facturer l'acompte de (ré)inscription à la validation | SYS (socle 3) | source `acompte:{inscription}` ; imputé sur la facture annuelle | `facturation.acompte_facture` |
| FAC-150 | Facturer les frais de dossier d'une candidature | SYS (socle 3) | réglage du socle 3 ; payeur = déclarant | idem |
| FAC-151 | Recevoir une demande de facturation d'un autre socle (source) | SYS | idempotente ; refus si article inconnu, payeur manquant (tâche) | `facturation.source_recue` |
| FAC-152 | Annuler une source avant émission | SYS / CPT | ligne en attente retirée, trace conservée | `facturation.source_annulee` |

### 17.5 Lignes, pièces, émission
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-153 | Voir la file des lignes en attente (par compte, origine, période) | CPT · `facturation.gerer` | filtres ; totaux | — |
| FAC-154 | Ajouter une ligne motivée (geste, frais exceptionnel) | CPT · `facturation.gerer` | article, motif, montant borné ; double contrôle au-delà du seuil (S-154) | `facturation.ligne_ajoutee` |
| FAC-155 | Retirer une ligne en attente | CPT · `facturation.gerer` | motif ; trace | `facturation.ligne_retiree` |
| FAC-156 | Composer les brouillons (regroupement par compte) | SYS / CPT | règles §8.1 | `facturation.brouillon_compose` |
| FAC-157 | Prévisualiser une pièce telle que la famille la recevra | CPT · `facturation.gerer` | document réel (trame), mention « brouillon » | — |
| FAC-158 | Prévisualiser un échantillon de 5 pièces tirées au hasard d'un lot | CPT · `facturation.emettre` | obligatoire avant un lot > 20 pièces | — |
| FAC-159 | **Émettre** une pièce ★ | CPT · `facturation.emettre` | numéro continu, document rendu et stocké, échéancier, dépôt au coffre, avis différé | `facturation.piece_emise` |
| FAC-160 | **Émettre un lot** ★ | CPT · `facturation.emettre` | aperçu (nombre, total, anomalies exclues), N3 ; reprise sur incident sans doublon | `facturation.lot_emis` |
| FAC-161 | Choisir le moment d'envoi des avis du lot | CPT · `facturation.emettre` | plage calme du socle 8 ; défaut : prochain jour ouvré 9 h | — |
| FAC-162 | Poser ou modifier l'échéancier d'une pièce avant l'envoi | CPT · `facturation.gerer` | somme exacte ; jamais après un prélèvement préparé | `facturation.echeancier_pose` |
| FAC-163 | Accorder un échéancier dérogatoire (après émission) | CPT · `facturation.echeanciers` | nouvel échéancier **versionné**, ancien conservé ; avis au payeur ; prélèvements futurs recalculés (25) | `facturation.echeancier_modifie` |
| FAC-164 | Voir une pièce, ses lignes, son explication, son document, son historique | CPT, ADF · `facturation.lire` | — | — |
| FAC-165 | Télécharger le document d'une pièce (original ou duplicata) | CPT, ADF · `facturation.lire` | URL pré-signée courte (CLAUDE.md) ; journalisé | — |
| FAC-166 | Renvoyer l'avis d'une pièce au payeur | CPT, ADF · `facturation.gerer` | mêmes destinataires (payeurs) ; plage calme | `facturation.avis_renvoye` |
| FAC-167 | Rechercher une pièce (numéro, élève, payeur, montant, période) | CPT, ADF · `facturation.lire` | — | — |
| FAC-168 | Voir la séquence de numérotation et ses contrôles | CPT · `facturation.lire` | aucun trou ; brouillons sans numéro | — |
| FAC-169 | Émettre une facture à un organisme (référence de commande, SIRET) | CPT · `facturation.emettre` ★ | mentions B2B ; pénalités et indemnité de 40 € seulement envers un professionnel | idem FAC-159 |
| FAC-170 | Déposer une facture sur Chorus Pro (organisme public) | CPT · `facturation.emettre` | lot F11 [NV] ; suivi du statut | `facturation.depot_chorus` |

### 17.6 Corrections et avoirs
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-171 | Émettre un avoir total ★ | CPT · `facturation.avoirs` | motif ; mention sur la facture d'origine | `facturation.avoir_emis` |
| FAC-172 | Émettre un avoir partiel (prorata des lignes) ★ | CPT · `facturation.avoirs` | [F] | idem |
| FAC-173 | Émettre un avoir par ligne ★ | CPT · `facturation.avoirs` | ≤ ligne d'origine | idem |
| FAC-174 | Corriger une facture (avoir + nouvelle facture en un geste) ★ | CPT · `facturation.avoirs` + `emettre` | deux pièces liées, un seul avis expliquant la correction | `facturation.correction_emise` |
| FAC-175 | Corriger un lot (erreur de grille) ★ | CPT · `facturation.avoirs` | aperçu des deux lots, échantillon, N3 ; avis unique | idem |
| FAC-176 | Accepter les conséquences proposées d'un changement de payeur ★ | CPT · `facturation.avoirs` | §9.2 | `facturation.changement_payeur_applique` |
| FAC-177 | Accepter les conséquences proposées d'un changement de régime, d'une sortie | CPT · `facturation.gerer` | [F] avoirs / compléments ; N3 si émis | idem |
| FAC-178 | Valider les remises d'ordre proposées (unitairement ou en lot) | CPT · `facturation.gerer` | jamais de motif ; N2 | `facturation.remise_ordre_validee` |
| FAC-179 | Refuser une remise d'ordre proposée (motif) | CPT · `facturation.gerer` | motif ; trace | — |
| FAC-180 | Annuler un brouillon | CPT · `facturation.gerer` | journalisé | `facturation.brouillon_annule` |

### 17.7 Consommations, services, voyages
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-181 | Recevoir les consommations du jour (repas, nuitées, études, garderie) | SYS (40, 41, 43) | quantités, jamais de prix | `facturation.consommations_recues` |
| FAC-182 | Contrôler le mois avant la facture de consommations | CPT · `facturation.gerer` | variations, sans régime, élèves partis | — |
| FAC-183 | Émettre les factures mensuelles de consommations ★ | CPT · `facturation.emettre` | lot (FAC-160) | idem |
| FAC-184 | Facturer une perte de badge, une caution | ADF, CPT · `facturation.gerer` | article dédié ; caution = dépôt remboursable | — |
| FAC-185 | Créer l'échéancier familles d'un voyage (à la demande du socle 42) | SYS / ORG → CPT | acompte, soldes ; source par participant | `facturation.voyage_facture` |
| FAC-186 | Traiter un désistement de voyage (avoir proposé) | ORG propose, CPT valide | règle du voyage ; motif légitime | `facturation.voyage_desistement` |
| FAC-187 | Voir l'état agrégé des paiements d'un voyage | ORG · `voyages.gerer` (42) | agrégé ; nominatif seulement si S-161 | — |
| FAC-188 | Clôturer financièrement un voyage (bilan, excédent) | CPT, ORG | S-162 | `facturation.voyage_cloture` |

### 17.8 Familles et payeurs
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-189 | Voir mon compte (solde, échéances, documents) | PAY, ELM payeur · payeur | §14 ; un compte par établissement | — |
| FAC-190 | Voir le détail et l'explication d'une ligne | PAY · payeur | phrases fixes ; seulement mes parts | — |
| FAC-191 | Télécharger mes documents (factures, avoirs, échéanciers, relevé) | PAY · payeur | coffre ; URL courte | — |
| FAC-192 | Contester une ligne (ouvre un fil vers la comptabilité) | PAY · payeur | carte de la ligne dans le fil (socle 16) | `facturation.contestation_ouverte` |
| FAC-193 | Voir le relevé informatif des frais de mon enfant (si activé) | PNP | R-F8 ; sans montants payés ni payeur | — |
| FAC-194 | Consulter mes factures par lien nominatif (payeur sans compte) | PAY sans compte | socle 7 R-D8 ; durée limitée | — |
| FAC-195 | Voir ses factures (organisme) | ORGA | lien nominatif ou portail ; aucune donnée scolaire | — |

### 17.9 Pilotage, contrôle, clôture
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| FAC-196 | État des contributions par nature légale (R. 442-48) | DIR, CPT · `facturation.lire` | agrégé ; exportable pour l'inspection | — |
| FAC-197 | Tableau du produit facturé (par mois, catégorie, niveau, régime) | DIR, OGEC, CPT · `facturation.pilotage` | agrégé | — |
| FAC-198 | État des produits constatés d'avance à une date | CPT · `facturation.lire` | [H] pour l'expert-comptable | — |
| FAC-199 | Clore une période mensuelle ★ | CPT · `facturation.cloturer` | contrôles (non-trou, lignes en attente datées, brouillons) ; irréversible sauf réouverture N3 motivée par DIR | `facturation.periode_close` |
| FAC-200 | Contrôle nocturne d'intégrité (séquences, sommes, empreintes des documents) | SYS | anomalie → alerte CPT et DIR | `facturation.integrite_anomalie` |
| FAC-201 | Produire les écritures de ventes d'une période pour le socle 48 | SYS / CPT · `facturation.exporter` | [F] équilibre, unicité ; numéro d'export continu ; dates en heure de Paris | `facturation.ecritures_produites` |
| FAC-202 | Exporter les écritures (format du logiciel comptable de l'OGEC) ★ | CPT · `facturation.exporter` | formats : FEC [NV], `scolia_ecritures_v1` [F], logiciel choisi (Q-F17) ; empreinte | `facturation.export_comptable` |
| FAC-203 | Voir le journal des actions financières (qui, quoi, quand) | DIR, CPT · `facturation.journal` | ajout seul | — |
| FAC-204 | Exporter les données d'un compte (droit d'accès RGPD) | CPT, DIR · `facturation.gerer` | pour le payeur seulement ; socle 27 | — |
| FAC-205 | Préparer la reprise depuis Charlemagne (soldes d'ouverture par payeur) | CPT · `facturation.reprise` | F-03 anonymisé d'abord ; pièce « solde d'ouverture » par compte ; contrôle au centime | `facturation.reprise_soldes` |
| FAC-206 | Tenir la **double tenue** (comparaison ScolIA / Charlemagne sur une période) | CPT · `facturation.reprise` | rapport d'écarts par famille, au centime | — |

### 17.10 Système (automatismes)
| ID | Action | Déclencheur | Règles | Événement |
|---|---|---|---|---|
| FAC-207 | Recalculer les lignes en attente d'un élève | `eleve.regime_changed`, `scolarite.*`, `payeurs.modifies`, quotient | idempotent ; aucune pièce émise touchée | `facturation.lignes_recalculees` |
| FAC-208 | Proposer les conséquences d'un fait sur des pièces émises | idem | proposition dans la file ; jamais d'émission automatique | `facturation.consequences_proposees` |
| FAC-209 | Geler la facturation d'un élève décédé ou d'un payeur décédé | `eleve.decede`, `personne.decedee` | annule les lignes en attente, suspend avec 25 | `facturation.gel` |
| FAC-210 | Créer une tâche « payeur manquant » | catégorie < 100 % | au secrétariat | `facturation.tache_payeur` |
| FAC-211 | Déposer les documents émis au coffre du payeur | `facturation.piece_emise` | socle 7 | — |
| FAC-212 | Émettre les avis différés (plage calme) | file du socle 8 | enveloppe neutre | — |

### 17.11 IA
| ID | Action | Qui | Règles | Événement |
|---|---|---|---|---|
| FAC-213 | Expliquer une ligne, un solde, une facture | IA au nom de PAY, ADF, CPT | §16 ; chiffres du service seulement | — |
| FAC-214 | Résumer les anomalies d'une génération et préparer un lot de corrections (brouillon) | IA au nom de CPT | N3 pour toute émission | — |

---

## 18. Règles transverses du socle 24 (R-F1 et suivantes) [R]

| # | Règle | Lot |
|---|---|---|
| R-F1 | **Un seul service de facturation** : toute route (historique, `/finances/*`, mobile, IA) passe par les mêmes commandes ; test de garde qui échoue si un module écrit dans `ent_fac_*` / `ent_fin_*` hors du service | F0, F1 |
| R-F2 | **Pièce émise immuable** : déclencheurs en base ; aucun `DELETE` financier ; `RESTRICT` partout | F0 |
| R-F3 | **Numéro attribué à l'émission**, par le serveur, série continue ; contrôle nocturne de non-trou | F0 |
| R-F4 | **Payeur lu au socle 4 à la date de la prestation** ; jamais déduit du foyer ou du rang du responsable ; catégorie sans payeur = non facturée + tâche | F2 |
| R-F5 | **Un compte par payeur et par établissement** ; un payeur ne voit que ses parts ; un avoir ne se compense jamais sur un autre compte | F2 |
| R-F6 | **Chaque ligne a une source** (clé unique) et une **explication** conservée | F4 |
| R-F7 | **Calcul en centimes entiers**, arrondi à l'inférieur pour la famille, répartition au plus fort reste | [F] |
| R-F8 | Un parent **non payeur** ne reçoit rien de financier, sauf « copie aux titulaires » activée (relevé informatif sans montants payés, sans payeur, sans adresse) | F0, F7 |
| R-F9 | **Nature légale et régime de TVA obligatoires** sur chaque article ; motif d'exonération imprimé | F3 |
| R-F10 | **Grille votée figée** ; correction par nouvelle version datée | F3 |
| R-F11 | **Simulation avant toute génération** ; émission N3 par lot après aperçu et échantillon | F4 |
| R-F12 | Remise d'ordre au prorata des **jours de service** ; jamais de motif sur une pièce | F6 |
| R-F13 | Une ligne ne descend jamais sous 0 ; une consommation est une quantité ≥ 0 ; les montants négatifs ne viennent que des avoirs, réductions et aides typées | F0 |
| R-F14 | **Aide affectée** imputée seulement sur ses lignes cibles et son payeur bénéficiaire ; excédent reversé | F0 (provisoire), F8 |
| R-F15 | **Période close** : aucune pièce datée dedans ; correction datée du jour | F9 |
| R-F16 | Documents **rendus une fois**, stockés, empreinte, déposés au coffre ; duplicata = même document | F5 |
| R-F17 | **Aucune donnée scolaire, médicale, disciplinaire ou sociale** sur une pièce, un avis, un export comptable | F5 |
| R-F18 | Avis financiers : **enveloppe neutre**, plage calme, destinataires = payeurs de la catégorie (socle 4 `destinataires`) | F5, F7 |
| R-F19 | Les personnels hors finances (professeurs, vie scolaire, organisateurs) ne voient **aucune** information de paiement nominative ; l'organisateur de voyage voit un état agrégé | F0, F10 |
| R-F20 | **L'élève n'est jamais un canal ni un levier de recouvrement** : aucune privation de cantine, de voyage, de document scolaire, de bulletin, de résultat pour impayé sans décision motivée de la direction (et jamais pour le bulletin, le certificat, les notes) | F0, P6 |
| R-F21 | « **En impayé** » est un **état calculé**, pas une étiquette de la personne ; il disparaît avec le paiement | F7, P6 |
| R-F22 | Le 24 **ne voit jamais l'IBAN** ; il connaît le mode de paiement | F0 |
| R-F23 | **Séparation des tâches** : paramétrer la grille ≠ voter ; émettre un lot > seuil demande un second regard si l'établissement l'active (S-167) ; personne ne modifie seul l'identité émettrice et les paramètres du créancier (25) | F3, P3 |
| R-F24 | **Dates comptables en heure de l'établissement** (fuseau du socle 5, migration 265) | F0 |
| R-F25 | Événements financiers **sans montant ni donnée personnelle** au-delà des identifiants ; consommateurs déclarés | F1 |
| R-F26 | **Aucune visibilité entre établissements** : un compte, une grille, une fratrie par établissement, sauf groupe consenti | F2 |
| R-F27 | Tout geste financier est **journalisé** (acteur, casquette, motif) en ajout seul | F1 |
| R-F28 | Les automatismes **proposent**, les personnes **émettent** : aucune pièce n'est émise sans geste humain, sauf les pièces déclarées « émission automatique » par l'établissement (consommations mensuelles sous un seuil, S-168), toujours avec un aperçu quotidien | F4, F6 |

---

## 19. Droits et minimisation

### 19.1 Catalogue d'actions proposé (D-017) [R]
L'existant a trois droits (`facturation.lire`, `facturation.gerer`, `bourses.gerer`) [F] : trop grossier pour séparer les tâches. Proposé (portée : établissement) :

| Droit | Contenu | Modèles par défaut |
|---|---|---|
| `facturation.lire` | voir comptes, pièces, lignes, documents | Compta, Direction, Secrétariat (lecture) |
| `facturation.gerer` | lignes, brouillons, sources manuelles, échéanciers avant envoi, remises d'ordre | Compta |
| `facturation.emettre` ★ | émettre pièces et lots | Compta |
| `facturation.avoirs` ★ | avoirs, corrections | Compta (Direction au-delà d'un seuil, S-156) |
| `facturation.echeanciers` | échéanciers dérogatoires | Compta, Direction |
| `facturation.reductions` | accorder / retirer des réductions | Compta, Direction |
| `facturation.tarifs` | articles, grilles brouillon, réductions | Compta |
| `facturation.voter` ★ | enregistrer le vote, corriger une grille votée | Direction |
| `facturation.simuler` | simulateur, scénarios | Direction, Compta, OGEC |
| `facturation.parametrer` ★ | paramètres, identité, séries, trames | Direction, Compta |
| `facturation.engagements` | engagements financiers des élèves | Compta, Secrétariat |
| `facturation.cloturer` ★ | clôture de période | Compta (réouverture : Direction) |
| `facturation.exporter` ★ | écritures, exports comptables | Compta |
| `facturation.pilotage` | tableaux agrégés | Direction, OGEC |
| `facturation.journal` | journal financier | Direction |
| `facturation.reprise` | reprise et double tenue | Compta (temporaire) |
| `facturation.publier` | grille publique | Direction |

Le droit du **payeur** n'est pas un rôle : il découle du fait « payeur » du socle 4 (`ent_eleve_lien_droit` `recevoir_factures`, `payer_en_ligne` [socle 4 §8.6]) ; le **professeur** n'a aucun droit financier.

### 19.2 Pièges de minimisation à tester [R]
1. Parent non payeur : aucune route financière ne lui répond (matrice de séparation, RS3-01 généralisée à toutes les routes 24 et 25).
2. Payeur d'une seule catégorie : ne voit pas les lignes des autres catégories du même enfant.
3. Payeur d'un enfant d'un foyer recomposé : ne voit pas les lignes des autres enfants du foyer dont il n'est pas payeur.
4. Organisme : aucune donnée scolaire dans ses documents ni ses avis.
5. Professeur organisateur : aucun nom associé à un retard (sauf S-161 ouvert, journalisé).
6. Explication d'une remise d'ordre : aucun motif d'absence.
7. Panier repas : aucune mention médicale.
8. Avis : aucun montant dans un SMS, un push, un objet d'e-mail.
9. Événements outbox : aucun montant, aucun nom.
10. Support ScolIA : aucun accès aux montants ni aux comptes.
11. IA : aucun IBAN ni motif social dans un contexte ; une question d'un parent sur « l'autre parent » reçoit « je ne peux pas vous renseigner sur un autre compte ».

### 19.3 Conservation [R, durées soumises au DPO]
Pièces et écritures : **10 ans** (C. com. L. 123-22 par prudence ; 6 ans fiscaux LPF L. 102 B) [V] ; explications et sources : comme la pièce ; lignes en attente jamais émises : 2 ans ; journal financier : 10 ans ; données de compte d'un payeur sans pièce depuis 2 ans : archivées (référentiel CNIL [V]) ; « en impayé » : état calculé, aucune liste conservée après régularisation (48 h, référentiel CNIL [V]).

---

## 20. Événements (outbox, dans la transaction du fait) [R]

**Émis par le 24** (aucun montant, aucune donnée personnelle hors identifiants) : `facturation.engagement_cree|modifie`, `facturation.grille_votee|en_vigueur|publiee`, `facturation.source_recue|annulee`, `facturation.lignes_recalculees`, `facturation.consequences_proposees`, `facturation.piece_emise` (type, compte, élèves), `facturation.lot_emis`, `facturation.avoir_emis`, `facturation.echeancier_pose|modifie`, `facturation.piece_soldee` (sur imputation du 25), `facturation.periode_close`, `facturation.ecritures_produites`, `facturation.gel`, `facturation.tache_payeur`, `facturation.integrite_anomalie`. Les événements historiques `fac.*` [F] sont **renommés** avec un alias de lecture pendant la transition (F1).

**Consommateurs déclarés** : 25 (`piece_emise`, `echeancier_*`, `avoir_emis` → prélèvements, crédits) ; 7 (dépôt au coffre) ; 8 (avis) ; 2 (chronologie de l'élève **côté personnel financier seulement** : « facture annuelle émise », jamais le montant pour les autres personnels) ; 48 (écritures) ; 30 (IA, lecture).

**Consommés par le 24** : `inscription.validee`, `inscription.annulee`, `scolarite.depart_*` (3) ; `eleve.regime_changed`, `eleve.sortie`, `eleve.decede`, `eleve.majeur` (2) ; `payeurs.modifies`, `personne.decedee`, `personne.coordonnees_modifiees` (adresse de facturation) (4) ; `vie_scolaire.absence_justifiee` (remise d'ordre) ; `structure.fermeture` (5) ; `agent.engagement_termine` (6) ; `restauration.consommations_jour`, `internat.*`, `activites.*` (40–43) ; `voyage.*` (42) ; `aide.accordee|retiree` (26) ; `paiements.imputation` (25).

---

## 21. Notifications (natures déclarées au socle 8) [R]

| Nature | Urgence · sensibilité | Destinataires | Canaux | Contenu externe |
|---|---|---|---|---|
| `finances.facture_emise` | U1 · C2 | payeurs de la pièce | in-app, e-mail enveloppe, push ; lien nominatif sans compte ; courrier si aucun canal | « Un document financier de [établissement] est disponible » |
| `finances.avoir_emis` | U0 · C2 | payeur | in-app, résumé | idem |
| `finances.echeancier_modifie` | U1 · C2 | payeur | in-app, e-mail enveloppe | idem |
| `finances.correction_tarif` (lot) | U1 · C2 | payeurs concernés | in-app, e-mail enveloppe ; un seul avis | « Une correction a été apportée à votre facture » |
| `finances.consequences_changement_payeur` | U1 · C2 | payeur sortant et entrant, chacun pour soi | in-app, e-mail enveloppe | idem |
| `finances.contestation_reponse` | U1 · C2 | payeur | messagerie (socle 16) | — |
| `finances.tache_comptable` | U0 · C2 | personnel financier | in-app, résumé | — |
| `finances.integrite_anomalie` | U2 · C1 | CPT, DIR | in-app, e-mail | « Anomalie de contrôle financier » |

Les natures du 25 (pré-notification, rejet, relance, paiement reçu, remboursement) sont au `25_PAIEMENTS.md` §19.

---

## 22. Écrans clés [R]

1. **Tableau de bord financier** (CPT) : file de travail (lignes à contrôler, anomalies, brouillons, conséquences proposées, remises d'ordre, contestations, tâches payeur) ; chiffres du mois ; raccourcis.
2. **Grilles** : tableau niveau × régime × tranche, version, statut, simulation à droite (produit attendu, écart N-1), familles types.
3. **Rentrée** : assistant en quatre étapes — simulation, anomalies, échantillon, émission du lot (N3) — avec barre de progression et reprise.
4. **Fiche financière de l'élève** (dans le dossier élève, onglet visible du seul personnel financier) : engagement, payeurs par catégorie et période (lus au 4), lignes par payeur, pièces, explication.
5. **Compte débiteur** : pièces, échéances, imputations, solde, documents, journal.
6. **Pièce** : document, lignes avec explication, échéancier, imputations, avoirs liés, historique.
7. **Espace famille** (web, mobile) : solde, échéances, documents, explication par ligne, payer, contester.
8. **Clôture** : contrôles de la période, bouton de clôture N3, état des exports.
9. **Contributions** (DIR) : état par nature légale, par niveau.

---

## 23. Modèle de données cible (brique `facturation`) [R]

Principe : **faire évoluer** les tables existantes (`ent_fac_*`, `ent_fin_*`) plutôt que de les remplacer ; aucune pièce émise n'est réécrite ; toutes les tables portent `etablissement_id`, clés étrangères **composites** `(etablissement_id, id)`, RLS ENABLE + FORCE, aucun `ON DELETE CASCADE` sur une donnée financière.

```
ent_fin_compte                                   -- compte débiteur (§9.1)
  id, etablissement_id
  personne_id UUID NULL, organisme_id UUID NULL   -- exactement un des deux (socle 4)
  foyer_id UUID NULL → ent_foyer                  -- compatibilité (Q-A1 du socle 4)
  libelle_facturation TEXT                        -- nom imprimé (« M. Paul Durand », « Conseil départemental — ASE »)
  siret TEXT NULL, reference_client TEXT NULL     -- organisme
  chorus_pro BOOLEAN NOT NULL DEFAULT false
  etat TEXT ('actif','archive')
  UNIQUE (etablissement_id, personne_id) WHERE personne_id IS NOT NULL
  UNIQUE (etablissement_id, organisme_id) WHERE organisme_id IS NOT NULL

ent_fin_engagement                               -- fiche financière d'un élève pour une année (§7.1)
  id, etablissement_id, eleve_id, annee_scolaire
  du DATE, au DATE NULL, regime TEXT, formule JSONB (options payantes, internat de semaine…)
  contrat_document_id UUID NULL (socle 7), conditions_particulieres TEXT NULL, decide_par
  version INT, remplace_id NULL                   -- versions (jamais d'UPDATE du contenu)

ent_fin_article  (+ colonnes)                    -- existant 200
  nature_legale TEXT NOT NULL                     -- liste fermée §5.1
  regime_tva TEXT NOT NULL, taux_tva_bps INT NULL  -- 'exonere_261_4_4','taxable','hors_champ','pour_compte_tiers'
  unite TEXT, mode TEXT ('forfait','reel'), remboursable, soumis_reductions, soumis_remise_ordre

ent_fin_grille_version                           -- §10.2
  id, etablissement_id, annee_scolaire, numero, statut ('brouillon','proposee','votee','en_vigueur','close')
  vote_le DATE NULL, vote_reference TEXT NULL, vote_document_id NULL, en_vigueur_du DATE NULL
ent_fin_tarif (+ grille_version_id, tarif_par_defaut BOOLEAN)   -- existant 200 ; immuable si version ≠ brouillon (déclencheur)

ent_fin_reduction (+ mode 'taux'|'montant', cumul, plafond, perimetre_fratrie, regle_rang)
ent_fin_reduction_attribution                    -- remplace ent_fin_reduction_eleve (CASCADE → RESTRICT)
  id, reduction_id, eleve_id NULL, personne_id NULL, du, au NULL, motif, decide_par, valide_par NULL

ent_fin_source                                   -- §8.1
  id, etablissement_id, cle TEXT UNIQUE (par établissement), origine TEXT, eleve_id NULL,
  empreinte TEXT, donnees JSONB (quantités, période, référence du fait), recue_le, annulee_le NULL
ent_fin_ligne_attente
  id, etablissement_id, source_id, compte_id, eleve_id, article_id, periode_debut, periode_fin,
  quantite, montant_unitaire_centimes, montant_centimes, part_bps, explication JSONB, piece_id NULL
ent_fac_facture (+ colonnes)                     -- existant ; « pièce »
  compte_id UUID NULL → ent_fin_compte            -- obligatoire pour les nouvelles pièces (CHECK sur émise après F2)
  serie TEXT, date_piece DATE, exigible_le DATE NULL, document_id UUID NULL, empreinte TEXT NULL
  emetteur JSONB (identité figée à l'émission), destinataire JSONB (nom, adresse figés)
  numero NULL tant que brouillon (corrige FD-05)
ent_fac_ligne (+ colonnes)                       -- existant
  nature_legale, regime_tva, taux_tva_bps, montant_ht_centimes, montant_tva_centimes,
  part_bps, explication JSONB, aide_id NULL, reduction_id NULL
ent_fin_echeancier_version                       -- échéancier dérogatoire versionné (FAC-163)
ent_fin_periode                                  -- §13.5 : (etablissement_id, mois) statut 'ouverte','close', close_par, close_le
ent_fin_journal                                  -- ajout seul : acteur, casquette, action, objet, motif, avant/après (sans IBAN)
ent_fin_aide_imputation                          -- socle 26 → lignes cibles (aide_id, ligne_id, montant)
ent_fin_simulation                               -- scénarios de grille enregistrés (sans données nominatives)
```
**Déclencheurs** (F0) : `ent_fac_facture`, `ent_fac_ligne`, `ent_fac_echeance` (version courante), `ent_fac_quittance`, `ent_fac_encaissement` : refus d'`UPDATE` des colonnes figées et de tout `DELETE` quand la pièce n'est pas brouillon ; `ent_fin_tarif` figé hors brouillon ; `ent_fin_journal` en ajout seul.

---

## 24. Plan de migration depuis l'existant (sans casser l'UI historique, le mobile ni le labo)

| Étape | Quoi | Garantie |
|---|---|---|
| MIG-1 (F0) | Rapatrier les correctifs RS3-01 à RS3-04 et la migration 376 (et 377) ; **contrôler l'absence de doublons** avant l'index unique (procédure du §2.2 de la recette 3, **sans correction automatique** : rapport à la comptabilité) | aucune donnée monétaire modifiée par un script |
| MIG-2 (F0) | `CASCADE` → `RESTRICT` ; déclencheurs d'immutabilité ; colonnes `serie`, `numero` nullable pour les brouillons ; pièces historiques marquées `serie='historique'` | aucune pièce émise réécrite |
| MIG-3 (F1) | Les routes `/facturation/*` deviennent des **adaptateurs** des commandes du service unique (encaisser → règlement imputé du 25 ; prélever → préparation par échéance ; rappels → relances par paliers ; export → `POST` de lot pré-notifié) ; mêmes réponses JSON pour `ui.rs` | test d'or : chaque route historique rend le même JSON qu'avant sur les scénarios du labo, sauf les corrections listées |
| MIG-4 (F2) | Comptes débiteurs créés depuis les foyers qui ont des pièces ; `compte_id` renseigné sur les pièces existantes ; foyers à deux payeurs séparés **signalés** | rapport de reprise ; aucune réaffectation |
| MIG-5 (F2) | Lecture des payeurs par le port du socle 4 (`ent_eleve_payeur`) dès qu'il existe ; d'ici là adaptateur sur `ent_eleve_lien.est_payeur` / `part_paiement` [F] | mêmes résultats sur les tests S2-9 |
| MIG-6 (F3) | Nature légale et TVA des articles : **valeurs proposées** par l'IA ou par règles (catégorie → nature), **validées** par la comptabilité ; impossible d'émettre une nouvelle pièce avec un article non qualifié | — |
| MIG-7 (F5) | Documents : les pièces historiques gardent leur PDF régénéré marqué « reconstitution » ; les nouvelles pièces ont leur document stocké | — |
| MIG-8 (F1 → retrait) | Retrait des routes historiques et des tables mortes (`ent_fac_rappel`, `ent_tarif_conso`, `ent_taux_tva` remplacée par l'article) avec `ui.rs` (F-001), après la durée d'archive | migrations de retrait 538–539 (ou 5490–5499) |

---

## 25. Lots de code (équipe Sonnet) — socles 24 et 25

### 25.1 Tableau des lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **F0** correctifs urgents (24 et 25) | **FD-01** à **FD-03** : rapatrier RS3-01 (filtre `est_payeur` dans `foyer_ids_parent` et **toutes** les routes historiques financières), RS3-02 (`executer_sepa_atomique`, migration 376 après contrôle des doublons), RS3-04 (377) ; export SEPA historique : `POST`, pré-notification exigée, paramètres du créancier **lus côté serveur** (plus de `cdtr_*` en URL) ; **FD-04** : prélèvement historique refusé sur une facture qui a des échéances, export historique limité à ses propres prélèvements pré-notifiés, rappels historiques désactivés sur les pièces du chemin neuf, encaissement historique routé vers le règlement imputé ; **FD-05** numéro attribué par le serveur à l'émission ; **FD-06** calcul du solde sur l'encaissé réel + avoirs ; **FD-07** verrou `FOR UPDATE` de la facture, encaissement et quittance dans la même transaction, rang de quittance par compteur ; **FD-08/09** `RESTRICT` et déclencheurs d'immutabilité ; **FD-10** mandat révoqué exclu de l'export, prélèvements prévus annulés à la révocation ; **FD-11** IBAN du créancier chiffré, changement des paramètres du créancier N3 + événement + alerte à la direction ; **FD-13** frais de relance désactivés (palier `frais_centimes` forcé à 0 tant que Q-F21 n'est pas tranchée) ; **FD-14** aucune préparation de prélèvement pour un payeur sans canal joignable (tâche) ; **FD-16** quantités ≥ 0, montants unitaires ≥ 0 hors articles de réduction ; **FD-12** règle provisoire : bourse imputée seulement sur restauration et internat du payeur ; **FD-23** pagination des mandats côté SQL ; **FD-24** dates en heure de Paris ; **FD-26** contrôle par droit et non par `role == "parent"` ; contrôle de la clé IBAN (modulo 97) ; RUM générée par le serveur ; vérifier que le pain.008 n'émet **aucune adresse non structurée** (exigence EPC au **15/11/2026**, `25_PAIEMENTS.md` §4.1) | — | un test par défaut corrigé (dont les tests RS3-01 à RS3-04 de la recette 3) ; matrice de séparation des routes financières étendue (parent non payeur, payeur d'une catégorie, autre établissement) ; aucune régression des 6 scénarios S2-9 ; `MIGRATION_TAG` à jour |
| **F1** service unique | routes historiques en adaptateurs (MIG-3) ; événements `facturation.*` avec alias `fac.*` ; journal financier ; garde de code R-F1 ; droits fins (§19.1) et modèles de rôles | F0 | test d'or des routes historiques ; aucune écriture hors service (garde) ; journal complet |
| **F2** comptes débiteurs et payeurs | `ent_fin_compte`, reprise MIG-4, port `payeurs` du socle 4 (ou adaptateur), découpage par date de désignation, conséquences d'un changement de payeur, tâches « payeur manquant », élève majeur payeur (FD-15), organismes payeurs | F1 ; socle 4 A-lot payeurs (ou adaptateur) | CF-01 à CF-09, CF-16, CF-17 ; matrice « payeur d'une catégorie » |
| **F3** catalogue, grilles, nature légale, TVA | articles enrichis, versions de grille et vote, tarif par défaut, simulateur et scénarios, familles types, état des contributions | F1 | grille votée immuable (test) ; simulation = génération au centime ; FAC-196 |
| **F4** engagements et génération | `ent_fin_engagement`, sources et lignes en attente avec explication, recalcul avant émission, aperçu enrichi et anomalies, émission par lot avec échantillon, reprise sur incident | F2, F3 | rentrée de 400 familles fictives simulée puis émise sans écart ; rejeu sans doublon |
| **F5** pièces et documents | numérotation (séries, non-trou), identité émettrice figée, trames D-023 et PDF/A via le socle 7, coffre, duplicata, relevé, mentions légales, motif d'exonération, avoirs par ligne, correction en un geste | F4 ; socle 7 (émission de documents) | document conforme à la liste §5.2 ; empreinte vérifiée ; aucun document régénéré |
| **F6** facturation par événements | consommations 40/41/43 (quantités), remises d'ordre au jour de service, recalcul des absences corrigées (FD-21), fermetures, stages, seuil d'émission, factures mensuelles | F4 ; socles 40, 41, 43 (ou API de consommation existante) | CF-21 à CF-29, CF-40 |
| **F7** espace famille | compte web et mobile, explication par ligne (phrases fixes), contestation vers la boîte comptabilité, relevé informatif R-F8, lien nominatif sans compte, langues | F5 ; socles 8, 16 | pièges §19.2 ; un payeur voit ses seules parts |
| **F8** réductions et aides | réductions en montant, cumul, plafond, règle de fratrie, attributions datées, aides affectées du socle 26, excédent à reverser (25) | F4 ; socle 26 | CF-17, CF-18, CF-33 à CF-35 |
| **F9** périodes, clôture, écritures | périodes, clôture N3, contrôle nocturne d'intégrité, écritures de ventes, export FEC et formats (Q-F17), produits constatés d'avance | F5 ; socle 48 (ou export seul) | export équilibré, continu, unique ; non-trou ; dates en heure locale |
| **F10** voyages et sorties | échéancier de voyage, désistement, aides, état agrégé, clôture et excédent | F4 ; socle 42 | R-F19 testé |
| **F11** organismes et Chorus Pro | factures B2B, mentions professionnelles, dépôt Chorus Pro [NV], suivi | F5 | facture à un organisme public déposée en recette Chorus |
| **F12** IA | capacités §16.3 ; explication, anomalies, simulation, brouillons N3 | F4, F7 ; `ent-ia` | aucune émission sans geste ; aucune donnée sociale ni IBAN dans un contexte |
| **F13** établissements publics (D-029) | régie de recettes, tarifs importés de la délibération de la collectivité, bordereaux au comptable, interface Op@le [NV] | F9 ; socle 9 | après le pilote privé |
| **P1 à P12** | voir `25_PAIEMENTS.md` §23 : P1 encaissements au guichet et quittances ; P2 mandats électroniques ; P3 lots SEPA pain.008.001.08 et pré-notification ; P4 R-transactions ; P5 relevés bancaires et rapprochement ; P6 relances et recouvrement ; P7 remboursements et trop-perçus ; P8 paiement en ligne ; P9 porte-monnaie ; P10 attestations ; P11 trésorerie et tableaux ; P12 IA | — | — |

**Parcours minimal pour un pilote réel** : F0, F1–F5, F7, P1–P4, P6 (puis F6 et P5 avant la première facture de consommations). **Bascule** : au 1er septembre seulement, après une **double tenue** sur au moins une période (FAC-206), comparée au centime (Q-F2).

### 25.2 Migrations (registre `MIGRATIONS.md`)
| Plage si Q-B2 | Repli | Lot | Contenu |
|---|---|---|---|
| 5400 | **520** | F0 | `fin_f0_correctifs` : `RESTRICT`, déclencheurs d'immutabilité, numéro nullable des brouillons, `serie`, chiffrement de l'IBAN du créancier (colonne déclarée chiffrée), palier de relance sans frais, contrainte quantités ≥ 0 (+ rapatriement 376, 377 sous leur numéro d'origine) |
| 5401 | 521 | F1 | journal financier, droits fins |
| 5402 | 522 | F2 | comptes débiteurs, `compte_id` sur les pièces, reprise |
| 5403 | 523 | F3 | articles enrichis, versions de grille, attributions de réductions |
| 5404 | 524 | F4 | engagements, sources, lignes en attente |
| 5405 | 525 | F5 | colonnes de pièce (document, empreinte, identité figée, TVA par ligne) |
| 5406 | 526 | F6 | sources de consommation, recalcul |
| 5407 | 527 | F8 | aides affectées |
| 5408 | 528 | F9 | périodes, clôture |
| 5409 | 529 | F10 | voyages |
| 5410 | 530 | F11 | organismes, Chorus Pro |
| 5411–5489 | 531–537 | réserve | F12, F13 et suivants |
| 5490–5499 | 538–539 | retrait | RLS stricte et retrait des tables historiques |
Socle 25 : `25_PAIEMENTS.md` §23.2 (5500–5599, repli 540–559). Socle 26 : 5600–5699, repli 560–579, réservé.

---

## 26. Tests et recette

- **Unitaires** : calculs (existants [F] + cumul de réductions, plancher, aides affectées, jours de service, découpage par désignation).
- **Postgres** : les 6 scénarios S2-9 [F] gardés comme non-régression ; nouveaux : séparation en cours d'année (CF-03), payeur d'une catégorie, élève majeur payeur, organisme, grille corrigée après vote, lot de 400 corrections, période close, déclencheurs d'immutabilité (tentative d'`UPDATE` et de `DELETE` refusée), non-trou.
- **Séparation** (matrice déclarative de `separation.rs`) : toutes les routes 24 et 25 × (parent non payeur, payeur d'une autre catégorie, payeur d'un autre enfant du foyer, élève mineur, élève majeur non payeur, professeur, autre établissement, organisme).
- **Concurrence** : deux émissions simultanées d'un même lot, deux encaissements simultanés, rejeu d'événements.
- **Recette par personas** (D-021 étape 4) : comptable (rentrée complète, séparation au 1er décembre, erreur de grille), parent séparé (« je vois la facture de mon ex ? »), parent non payeur qui **attaque** (routes financières par identifiant), élève qui tente de voir les factures, professeur organisateur de voyage, inspecteur (état des contributions), expert-comptable (export).
- **Double tenue** (FAC-206) avant toute bascule réelle.

---

## 27. Réglages signalés « usine à gaz » (D-023) — signalés, non supprimés

| # | Réglage | Défaut | Pourquoi signalé |
|---|---|---|---|
| S-150 | Regroupement des pièces (annuelle unique / par enfant / par catégorie) | une annuelle par compte, une section par enfant | chaque variante complique l'explication |
| S-151 | Factures mensuelles plutôt qu'annuelles pour la contribution | annuelle + échéancier | la mensuelle multiplie les pièces |
| S-152 | Voyage sur facture propre ou intégrée à la mensuelle | propre | — |
| S-153 | Mois de départ dû / non dû avant le 15 | dû | source de contestations |
| S-154 | Seuil de double contrôle d'une ligne manuelle | 500 € | — |
| S-155 | Copie aux titulaires non payeurs | non | risque de fuite entre foyers |
| S-156 | Seuil de double validation d'une réduction ou d'un avoir | 300 € | — |
| S-157 | Motifs ouvrant droit à remise d'ordre (absence, stage, exclusion, fermeture) | absence > 15 jours, stage, fermeture | l'exclusion est délicate |
| S-158 | Quotient fourni en cours d'année : rétroactif ou mois suivant | mois suivant | — |
| S-159 | Bourse : déduction anticipée ou à réception | anticipée | trésorerie de l'OGEC |
| S-160 | Repas réservé non pris facturé | oui après le délai de prévenance du socle 40 | — |
| S-161 | Liste nominative des retards pour l'organisateur d'un voyage | fermée | minimisation |
| S-162 | Excédent de voyage remboursé | oui au-delà de 10 € par élève | — |
| S-163 | Série de numérotation par année civile ou par exercice | année civile [F] | — |
| S-164 | Fratrie : périmètre (payeur / foyer de résidence) et ordre | payeur, du plus âgé | familles recomposées |
| S-165 | Facturation sans engagement signé | bloquée avec tâche | — |
| S-166 | Acompte remboursable en cas d'annulation | non, sauf motif légitime | dépend du contrat |
| S-167 | Second regard obligatoire avant un lot > N pièces | non (alerte seulement) | petite équipe comptable |
| S-168 | Émission automatique des consommations mensuelles sous un seuil | non | contredit R-F28 si activé sans aperçu |

---

## 28. Questions à Florian (avec recommandations, appliquées par défaut — D-031)

| # | Question | Recommandation |
|---|---|---|
| Q-F1 | Lancer **F0** tout de suite, avant toute autre évolution des finances, en rapatriant les correctifs de la recette 3 ? | **Oui** (appliqué par défaut, D-031) : faille élevée de détournement de prélèvement **présente sur `ENT`**. |
| Q-F2 | **Bascule depuis Charlemagne** : au 1er septembre seulement, après une double tenue sur au moins une période, comparée au centime ? Quelle année ? | **Oui sur le principe** (appliqué) ; **l'année et l'usage de données réelles sont soumis à Florian** (données réelles, D-031). Recommandation : finances dans Charlemagne l'an 1 du pilote, double tenue au printemps de l'an 1, bascule en septembre de l'an 2. |
| Q-F3 | Un **service unique** et retrait progressif du chemin historique (pas de nouvelle fonction dessus) ? | **Oui** (appliqué). |
| Q-F4 | **Compte débiteur par payeur** (personne ou organisme) plutôt que le foyer ? | **Oui** (appliqué), avec le `foyer_id` conservé (Q-A1 du socle 4). |
| Q-F5 | Chaque payeur séparé reçoit **sa propre facture de sa part** calculée sur **son** quotient ? | **Oui** (appliqué, existant [F]). |
| Q-F6 | Parent non payeur : rien de financier, option « relevé informatif » désactivée par défaut ? | **Oui** (appliqué). |
| Q-F7 | Payeur manquant : ne pas facturer la catégorie et créer une tâche, jamais « le responsable 1 par défaut » ? | **Oui** (appliqué). |
| Q-F8 | **Nature légale R. 442-48** obligatoire sur chaque article, état des contributions pour l'inspection ? | **Oui** (appliqué). La **liste des natures** est à relire avec l'expert-comptable de l'OGEC (engagement extérieur : soumis à Florian). |
| Q-F9 | Élève majeur : ne voit ses factures que s'il est payeur ; rien sinon ? | **Oui** (appliqué). |
| Q-F10 | Grille **votée figée**, correction par nouvelle version ? | **Oui** (appliqué). |
| Q-F11 | Émission **toujours humaine** (N3) par lot, sauf réglage S-168 explicitement activé ? | **Oui** (appliqué). |
| Q-F12 | Séries de numérotation par année civile (existant) ou par exercice de l'OGEC ? | **Année civile** (appliqué ; existant, conforme) ; à revoir si l'expert-comptable préfère l'exercice. |
| Q-F13 | Bourse : imputation seulement sur demi-pension / pension du payeur bénéficiaire, excédent reversé (corrige FD-12) ? | **Oui** (appliqué ; règle provisoire en F0) ; version consolidée du code de l'éducation à relire au socle 26. |
| Q-F14 | Crédit d'un compte (avoir, trop-perçu) : imputé sur les factures suivantes seulement avec l'accord du payeur ou si le règlement financier le prévoit ; sinon remboursé ? | **Oui** (appliqué) ; clause à prévoir dans le règlement financier (juriste). |
| Q-F15 | Remise d'ordre au prorata des **jours de service** plutôt que des jours calendaires (existant) ? | **Oui** (appliqué) : plus juste pour les familles ; jours calendaires gardés en réglage. |
| Q-F16 | Reprise : soldes d'ouverture par payeur depuis Charlemagne (F-03), sans reprendre l'historique des pièces ? | **Oui** (appliqué) ; l'historique reste consultable dans Charlemagne (archive) ; **données réelles soumises à Florian**. |
| Q-F17 | Format d'export comptable : quel **logiciel comptable** l'OGEC utilise-t-il ? | **Soumis à Florian** (engagement extérieur) ; par défaut FEC [NV] + format générique existant. |
| Q-F18 | Prestations **taxables** (location de salles, ventes) dans ScolIA ou hors ScolIA ? | **Possibles mais signalées** (appliqué) ; qualification par l'expert-comptable (soumis). |
| Q-F19 | Voyages : état agrégé seulement pour l'organisateur, liste nominative fermée par défaut ? | **Oui** (appliqué). |
| Q-F20 | Contrat de scolarisation : ScolIA applique des règles déclarées (préavis, acompte, résiliation pour motif légitime) mais ne rédige pas le contrat ? | **Oui** (appliqué) ; **relecture du contrat par un juriste** (soumis : coût). |
| Q-F21 | **Frais de relance** et **frais de rejet** refacturés aux familles ? | **Non pour les frais de relance** (CPCE L. 111-8 [V], appliqué : désactivés en F0) ; **frais de rejet : 0 par défaut** (appliqué), activables seulement si le contrat les prévoit et au coût bancaire réel, après avis juridique (soumis). |
| Q-F22 | Payeur visé par une interdiction de communication : reste débiteur, reçoit ses seules pièces par courrier ou lien nominatif ? | **Oui** (appliqué) ; avis juridique au premier cas réel. |
| Q-F23 | Plages de migrations 5400–5499 / 5500–5599 / 5600–5699 (Q-B2) ou repli 520–539 / 540–559 / 560–579 ? | **Réservées** (appliqué) ; F0 part en **520**. |
| Q-F24 | **Fratrie entre établissements** d'un ensemble scolaire ? | **Seulement par groupe consenti** (Q13 de `SOCLES.md`) ou déclaration de la famille avec pièce (appliqué). |
| Q-F25 | Simulateur public de tarifs (T-18) au socle 24 ? | **Oui**, sans donnée personnelle (appliqué). |

---

## 29. Faits, hypothèses, contradictions

**Faits [F]** : §2 (code et migrations lus sur `origin/ENT` `5366c196`, défauts relus ligne à ligne pour FD-01, FD-02, FD-05, FD-06, FD-08) ; présence des migrations 200–206 sur `ENT` (commit `15a8045a`).
**Vérifié [V]** : §5 (sources citées) ; certitude « partielle » signalée quand seuls des extraits ont été lus.
**Hypothèses [H]** : pratiques de Charlemagne et des autres logiciels ; calendrier de l'année financière (§4) ; exercice de l'OGEC septembre–août ; APEL et assurance en encaissement pour compte de tiers ; produits constatés d'avance.
**Contradictions relevées** :
1. `SOCLE2_DOMAINES.md` §6.5 dit S2-9 « non fusionné » : **contredit** par la présence des migrations 200–206 sur `ENT` (commit `15a8045a`) ; la mention est périmée.
2. `docs/ent-metier-checklist.md` (finances) décrit encore le chemin historique et marque l'export OGEC « DROP ENT » alors que S2-9 livre un export d'écritures : à mettre à jour (signalé, non modifié ici).
3. `RECETTE_SECURITE_3.md` présente RS3-01 et RS3-02 comme **corrigées** : vrai sur sa branche, **faux sur `ENT`** tant qu'elle n'est pas fusionnée (FD-01, FD-02).
4. `04_ADMINISTRATIF.md` §8.9 place le **débiteur** comme un `ent_foyer` ; ce document introduit un **compte débiteur** qui garde `foyer_id` : compatible (Q-A1), mais le socle 4 doit savoir que le 24 ne crée plus de foyer « personne seule » pour facturer un organisme (l'organisme a son compte).
5. Les paliers de relance du S2-9 prévoient des **frais** (`ent_fin_palier_relance.frais_centimes`) : contraires à CPCE L. 111-8 [V] ; désactivés par F0.
6. Le S2-9 répartit la **bourse** entre tous les payeurs et toutes les catégories : contraire à la règle de déduction [V partiel] ; FD-12.

---

## 30. Les cinq décisions à prendre en priorité

1. **Lancer F0 maintenant** (Q-F1) : un parent non payeur peut détourner le prélèvement de l'autre parent et une échéance peut être encaissée deux fois **sur `ENT` aujourd'hui** ; les deux chemins de facturation se contredisent (double prélèvement possible). **Recommandation : oui, appliquée par défaut.**
2. **Un seul service, un compte par payeur** (Q-F3, Q-F4, Q-F5) : le chemin S2-9 devient la base unique ; le débiteur devient le payeur (personne ou organisme), plus le foyer. **Recommandation : oui, appliquée.**
3. **Ce que l'argent peut et ne peut pas faire à l'élève** (R-F20) : aucune privation (cantine, voyage, document scolaire) sans décision motivée de la direction, jamais sur le bulletin, le certificat ou les notes ; « en impayé » n'est pas une étiquette. **Recommandation : oui, appliquée ; avis juridique sur le refus de réinscription pour impayé (`25_PAIEMENTS.md` Q-P9).**
4. **Conformité des contributions et des pièces** (Q-F8, Q-F21) : nature légale R. 442-48 sur chaque article, factures conformes et numérotées par le serveur, pas de frais de relance. **Recommandation : oui, appliquée ; liste des natures et clauses à relire avec l'expert-comptable et un juriste (soumis à Florian).**
5. **Calendrier réel** (Q-F2, Q-F17) : finances dans Charlemagne l'an 1 du pilote, double tenue, bascule un 1er septembre ; logiciel comptable de l'OGEC à connaître pour l'export. **Soumis à Florian** (données réelles, engagement extérieur).
