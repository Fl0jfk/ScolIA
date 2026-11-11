# Socle 49 — Paie et administration du personnel

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée avant tout code de ce socle. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un environnement de production ; Next.js **lu seulement** (fichiers ciblés de `origin/main` par `git show`).
**Branche :** `ent-socle-48-49-conception`, partie de `origin/ent-mc0hoe` (`37ce0d3b`).
> **Révision D-047 (2026-11-11, branche `ent-socle-48-49-revision`)** : Florian confirme D-025 — le **calcul des bulletins et la DSN** sont construits dans ScolIA. **Le §34 l'emporte sur tout le document, §31 compris** ; Q-RH1 est close ; la recommandation du §3.3 (calcul et DSN au logiciel de paie) est abandonnée ; RH11 et RH12 deviennent obligatoires et sont redécoupés (RH11 à RH22). Rien n'a été supprimé.
> **Révision D-048 (même jour)** : périmètre « light » limité à un établissement scolaire ; **le §35 l'emporte sur le §34** (et donc sur tout le document).

**Passe personas :** `docs/socles/PERSONAS_49.md` ; ajouts au **§31**, marqués « ajout personas » ; **en cas de contradiction, le §31 l'emporte**.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-009, D-013, D-017, D-019, D-021, D-023, **D-024**, **D-025** (paie dans l'ENT Rust), D-026, **D-027 (A, H)** (dossiers des personnels domaine restreint ; pièces RH exigées par le droit du travail), D-029, **D-031**, **D-034**, **D-035** (précision 2 : la paie et la RH de l'add-on sont des **ébauches à refaire de zéro**), **D-036**, D-041. `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14). `docs/TRANSITION_T0_INVENTAIRE.md` (M05, M06, ligne « Paie »). `docs/SOCLES.md` §49, Q-B, T-04, T-06. Conceptions **`06_PERSONNELS.md`** (engagements, missions R-P11, service fait PRS-145, P26, P47, P57), **`15_ABSENCES_PERSONNELS.md`** (§14.3–14.4, R-15, R-16, Q-AP11), **`48_COMPTABILITE.md`** (journal de paie, R-CG22), `07_DOCUMENTS.md` (coffre de l'agent, signature), `08_NOTIFICATIONS.md`, `IA_COEUR.md`.

**Conventions.** [F] fait vérifié dans le code ; [V] lu sur source officielle ; [V-sec] lu sur source secondaire sérieuse ; [NV] non vérifié ; [H] hypothèse ; [R] recommandation.

**Numérotation propre au socle 49** (vérifiée libre sur les 134 branches distantes, 2026-11-11) : défauts **HD-01**… ; actions **RHP-100 à RHP-279** ; règles **R-RH1**… ; cas limites **CRH-01**… ; questions **Q-RH1**… ; réglages **S-4900 à S-4949** ; lots **RH0 à RH12** ; migrations **2980–2999** (imposée par le pilote).

Sommaire : §0 En bref · §1 Thèse · §2 Existant · **§3 Construire un moteur de paie ou s'interfacer ?** · §4 Qui est payé par qui · §5 Acteurs · §6 L'année · §7 Réglementaire · §8 Branchements · §9 Concepts · §10 Contrats et dossier du salarié · §11 Temps, congés et éléments variables · §12 Préparation de la paie et échange avec le logiciel de paie · §13 Bulletins, journal, DSN · §14 Santé et sécurité au travail · §15 Entretiens et formation · §16 Obligations collectives · §17 Public (EPLE) · §18 Cas limites · §19 IA · §20 Actions · §21 Règles · §22 Droits, minimisation, conservation · §23 Événements · §24 Écrans · §25 Modèle de données · §26 Reprise · §27 Lots et migrations · §28 Tests · §29 Réglages · §30 Décisions et questions · §31 Ajout personas · §32 Faits, hypothèses, contradictions · §33 Sources · **§34 Révision D-047 (l'emporte)**.

---

## 0. En bref

1. **Ce qui existe.** Côté Rust [F] : **rien** pour la paie ; le socle 6 tient les **engagements** (`ent_agent_engagement`, migration 352 : nature `salarie_organisme`, employeur `organisme_gestion`, quotité, heures dues) et les missions (355, « rémunération indicative » sans montant, R-P11) ; le socle 15 tient des absences très pauvres (080, motif en texte libre, déjà corrigé par son lot A0). Côté Next.js [F] : une **ébauche** de paie (`paie_periode` brouillon/figée, `paie_element` absence/heure/prime/retenue, `paie-rh-spec.ts` « spec_ready_impl_blocked », « pas un Sage Paie ») et une RH ébauche (M06) ; D-035 : refaites de zéro, données perdables (T0).
2. **Trois défauts Next.js à signaler** (Next.js intouchable) : le **libellé d'absence** (qui peut contenir le motif médical) est recopié en clair dans `paie_element.libelle` (AD-03 du socle 15) ; un **agent fictif** « Alice DEMO » est créé dans la table réelle `personnel` si l'établissement n'en a pas (`paie-etablissement-db.ts:228-247`) ; l'accès est contrôlé au **module**, pas à l'action (HD-01 à HD-03). Aucun correctif Rust urgent : le **lot RH0** est un lot de **garde-fous** (aucune donnée de paie hors brique, aucune colonne de salaire dans le socle 6, nature d'absence codée).
3. **La question structurante (§3) : construire un moteur de paie et de DSN, ou s'interfacer avec un logiciel de paie ?** Recommandation **[R]** : **ne pas construire le calcul de la paie ni la DSN.** ScolIA fait **tout ce qui précède et tout ce qui suit le calcul** : contrats et avenants, dossier du salarié, registre unique du personnel, temps et congés, **éléments variables préparés, contrôlés et validés** depuis les faits de l'établissement (absences du 15, service fait et heures du 6 et du 15, missions, primes décidées), échéances légales (DPAE, fin de période d'essai, fin de CDD, visites médicales, entretiens de parcours, titres de séjour, signalements DSN à J+5), santé et sécurité au travail, formation ; puis il **remet** les variables au logiciel de paie de l'OGEC ou de son cabinet (Silae, Sage, EBP, Charlemagne Paie…) dans **son format d'import**, et **reprend** en retour les **bulletins** (déposés au coffre de chaque salarié) et le **journal de paie** (écritures du socle 48). Raisons : la paie est **la seule fonction de l'ENT où une erreur coûte immédiatement de l'argent à une personne** ; la DSN suit une **norme annuelle** (NEODeS, mise en production obligatoire au 1-1-2027 pour la version 2027 [V-sec]) et une **charte d'éditeur** avec le GIP-MDS [V-sec] ; les taux, plafonds et règles de la CCN EPNL changent chaque année ; un OGEC paie **10 à 60 salariés** [H] ; les logiciels de paie et les cabinets amortissent ce coût sur des milliers de dossiers. Cela **contredit D-025** (« paie construite dans l'ENT Rust ») : **soumis à Florian (Q-RH1)**. Les lots RH1 à RH10 valent quelle que soit la réponse ; un **moteur de paie** (lots RH11–RH12, chiffrés au §3.4) n'est engagé qu'après Q-RH1.
4. **Qui est payé par qui (§4)** : les **enseignants du privé sous contrat** sont des agents publics **payés par l'État** (C. éduc. L914-1, maîtres contractuels et délégués) [V-sec] — ScolIA ne les paie jamais, il produit les **états de service** du rectorat (15) ; seuls les **personnels de droit privé de l'OGEC** (vie scolaire, administration, comptabilité, entretien, restauration, internat, et les heures de surveillance ou d'étude payées par l'OGEC à un enseignant, P47 du 6) passent par le 49. Dans le **public**, les agents sont payés par l'État ou la collectivité ; les AED et AESH le sont souvent par un **EPLE mutualisateur** [V-sec, académies] : ScolIA ne fait que transmettre les **états** à ce payeur.
5. **Réglementaire (§7)** : CCN de l'**enseignement privé non lucratif (EPNL), IDCC 3218**, texte unique signé le 11-4-2022, appliqué depuis le 1-9-2022 [V-sec] ; DSN mensuelle et signalements (arrêt de travail **dans les 5 jours**) [V-sec] ; bulletin : mentions de R3243-1, **montant net social** depuis le 1-7-2023, double conservé **5 ans** (L3243-4), bulletin électronique disponible **50 ans ou jusqu'à 6 ans après la retraite** (D3243-8) [V-sec] ; **entretien de parcours professionnel** remplaçant l'entretien professionnel (loi n° 2025-989 du 24-10-2025) : dans l'année de l'embauche puis **tous les 4 ans**, bilan à **8 ans** [V-sec] ; **référentiel CNIL de durées de conservation RH du 2-4-2026** [V-sec] ; registre unique du personnel conservé **5 ans** après le départ ; transposition de la **directive transparence salariale** en cours (projet de loi présenté le 10-9-2026, non voté) [V-sec] ; taxe sur les salaires, abattement associations **24 256 €** en 2026 [V].
6. **Ce que ScolIA apporte de plus [H]** : la paie de l'OGEC reçoit **chaque mois un fichier de variables juste et complet**, construit à partir des faits du quotidien (une surveillance remplacée, une absence validée, une heure d'étude assurée), contrôlé (écarts avec le mois précédent, absences sans pièce, heures sans validation), **validé** par la direction ; plus aucune feuille Excel ni note sur post-it ; le salarié voit **son** dossier, ses compteurs, ses bulletins, ses échéances (entretien, visite) ; la direction voit les **échéances légales** avant qu'elles ne tombent ; aucune donnée de santé n'est lisible au-delà de ceux qui en ont strictement besoin.
7. **Le plan** : **163 actions** (RHP-100 à RHP-262 ; RHP-263 à RHP-279 réservés), **40 règles**, **45 cas limites**, **13 lots** (RH0 à RH12, dont RH11–RH12 conditionnés à Q-RH1), migrations **2980–2999**. **Parcours minimal (doublon 2027-2028, en usage réel possible : l'add-on Next.js fait déjà de la RH)** : RH0, RH1 (dossier et contrats), RH3 (temps et congés), RH4 (variables), RH5 (export et retour), RH6 (échéances).
8. **Ajout personas (§31)** : quatre bloquants tranchés par défaut — (A) la gestionnaire de paie et le directeur ne gèrent jamais leur propre rémunération ; (B) clé de sécurité physique, bulletin papier et borne pour les salariés sans smartphone ; (C) périodes de protection et autorisation de cumul des agents publics qui **bloquent** les gestes concernés ; (D) trois mois de parallèle outillé puis réel ; 17 actions (RHP-263 à RHP-279), 10 règles, 15 cas limites.
9. **Soumis à Florian** : **Q-RH1** (moteur de paie ou interface, contredit D-025) ; logiciel de paie et cabinet de l'OGEC pilote (format d'import, coût) ; usage réel de la RH pendant le doublon ; signature électronique des contrats ; durées de conservation (DPO) ; réponses de la CCN à faire confirmer par un juriste en droit social.

---

## 1. Thèse : la paie se prépare dans l'établissement, se calcule chez le spécialiste

1. **Les faits de la paie naissent dans l'établissement.** Une absence (15), une séance d'étude assurée (10, 12, 6 PRS-145), une surveillance supplémentaire, une mission, une prime décidée par la direction, un congé posé, une entrée, une sortie : ce sont des faits déjà enregistrés dans ScolIA. Le 49 **les rassemble par salarié et par mois**, sans ressaisie.
2. **Un élément variable n'existe que s'il est prouvé et validé.** Chaque ligne du fichier de paie dit d'où elle vient (fait, pièce, décision) et qui l'a validée ; rien n'est envoyé sans l'**arrêté** de la direction (N3).
3. **Le calcul et la déclaration appartiennent au logiciel de paie** (option recommandée, §3) ; ScolIA ne prétend pas faire ce qu'un éditeur certifie chaque année. Il reprend ce qui en sort (bulletins, journal) pour que le salarié, la direction et la comptabilité aient tout au même endroit.
4. **La santé ne circule pas.** Une absence arrive avec une **nature codée** (maladie, accident du travail, maternité…), jamais avec un diagnostic ni un libellé libre ; un arrêt de travail est un fait daté ; un avis d'aptitude est « apte / apte avec aménagements / inapte », sans motif médical (R-RH6).
5. **Le dossier du salarié est un domaine restreint** (D-027 A), distinct de la fiche d'établissement du socle 6 : un collègue, un parent, un élève n'en voient rien ; la direction voit ce que sa fonction exige ; le salarié voit **tout ce qui le concerne**.
6. **Les échéances légales sont des tâches, pas des souvenirs.** Fin de période d'essai, fin de CDD, visite d'information, entretien de parcours, titre de séjour, attestation de formation obligatoire, signalement DSN à J+5 : chaque échéance a son responsable et apparaît avant de tomber.
7. **Public et privé, un modèle, des employeurs.** L'employeur est porté par l'engagement (socle 6) ; le 49 ne prépare de paie que pour les engagements dont l'employeur est **l'organisme de gestion** (ou l'EPLE employeur d'AED/AESH quand il est son propre payeur) ; tout autre agent produit au plus un **état** pour son payeur (rectorat, mutualisateur).
8. **L'IA aide à contrôler, jamais à décider ni à lire la santé.** Elle repère des écarts, prépare des brouillons de contrats à partir de modèles, explique un compteur ; elle ne valide aucun élément variable, ne lit aucun motif d'absence (il n'y en a pas), ne propose aucune sanction.

---

## 2. L'existant

### 2.1 ENT Rust [F]

| Élément | Où | Rapport au 49 |
|---|---|---|
| Agents et engagements | `migrations/352_agents.sql` (natures dont `salarie_organisme`, `vacataire`, `aed`, `aesh`, `apprenti` ; employeurs `etat`, `collectivite`, `organisme_gestion`… ; quotité ; heures dues hebdomadaires ; motifs de fin codés dont `rupture_conventionnelle`, `licenciement`) | **source** du 49 : un contrat du 49 se rattache à un engagement ; le 6 ne porte aucune donnée de paie (06 §… point 4) |
| Missions | `355_missions.sql` | « rémunération indicative » (nature, volume, jamais de montant, R-P11) lue par le 49 |
| Absences du personnel | `080_absence_rh.sql` (motif `TEXT` libre, 3 statuts) | refondu par le socle 15 (lot A0, conception `15_ABSENCES_PERSONNELS.md`) ; le 49 lit `absences::absences_validees` avec nature codée |
| Champs chiffrés | `champs_chiffres.rs` (cité par 06 §… [F par 06]) | réutilisé pour le NIR et l'IBAN des salariés (E11) |
| Rôle « Compta » | `ent-auth` | aucun droit RH ou paie aujourd'hui |

Aucune table de contrat de travail, de variable de paie, de bulletin, de compteur de congés ni de visite médicale [F] (`grep` des 231 migrations).

### 2.2 Next.js (ébauches, D-035 précision 2) [F, lecture seule]

| Élément | Où | Constat |
|---|---|---|
| Spécification figée | `app/lib/paie-rh-spec.ts` (54 l.) | « Spec Paie RH basique… Pas un Sage Paie » ; périmètre : variables depuis les fiches, bulletins PDF, journal ; hors périmètre : « DSN complète », « remplacement d'un ERP paie généraliste (Sage, Silae…) » ; statut **`spec_ready_impl_blocked`**, bloqué par « brief comptable établissement (codes cotisations, conventions retenues) » |
| Paie | `app/lib/paie-etablissement-db.ts` (261 l.), `app/api/paie/route.ts` (64 l.), pages `(admin)/paie`, `(admin)/rh/paie` | périodes (`brouillon` → `figee`) ; éléments (`absence`, `heure`, `prime`, `retenue`, libellé ≤ 200 car., quantité, montant) ; création idempotente d'un élément depuis une absence validée (`absences-paie-sync.ts`) |
| RH (M06) | `personnel` (+ `payload` JSONB), `personnel_attr`, `rh__*` | ébauche (Florian), « perdable / vide probable » (T0) ; tables filles déclarées jamais utilisées |

**Idées retenues** : une **période de paie** qui se fige ; des **éléments variables** typés et liés à leur source (l'absence) de façon idempotente ; la conscience, dès la spécification, que **la paie complète et la DSN ne se refont pas**. **Ce qu'on ne reproduit pas** : libellés libres (fuite du motif), montants décimaux, JSON, données de démonstration en table réelle, contrôle au module.

### 2.3 Défauts relevés

| # | Gravité | Défaut | Preuve [F] | Traitement |
|---|---|---|---|---|
| **HD-01** | **Critique** (Next.js, signalé) | Le libellé de l'absence est recopié dans `paie_element.libelle` sans purge : le motif médical peut s'y trouver (AD-03 du socle 15) | `app/lib/absences-paie-sync.ts:60` (cité par 15 §2.3) ; `paie-etablissement-db.ts:69-83` | ScolIA : nature codée seulement (R-RH6) ; **à signaler à Florian** : purge côté Next.js à décider par lui |
| HD-02 | Important (Next.js, signalé) | `ensurePersonnelDemo` crée un agent fictif dans la table `personnel` réelle | `paie-etablissement-db.ts:228-247` | reprise : ignorer les identifiants `paie-demo-*` (§26) ; à signaler |
| HD-03 | Moyenne (Next.js, signalé) | Accès contrôlé par `requireModule` (module), pas par action | `app/api/paie/route.ts`, `app/api/compta/route.ts:2,15` | droits fins (§22.1) |
| HD-04 | Moyenne (Rust, socle 15) | `ent_absence_rh.motif` en texte libre | `080_absence_rh.sql:8` | déjà traité par le lot A0 du 15 ; **préalable** au lot RH4 |

### 2.4 Ce qu'il ne faut pas perdre
- La synchronisation **absence validée → élément variable**, idempotente : reprise comme principe (R-RH14).
- Les **HSE** et heures demandées dans Next.js (M08, « alimente la paie ») : conservées pour l'année en cours (T0) et reprises comme heures validées (§26).

### 2.5 Verdict
Rien à reprendre en code. Le 49 est conçu de zéro, en s'appuyant sur les engagements du 6 et les faits du 15.

---

## 3. Construire un moteur de paie ou s'interfacer avec un logiciel de paie ?

### 3.1 Ce que « faire la paie » veut dire
1. **Calculer** chaque mois, pour chaque salarié : salaire de base selon la classification de la CCN EPNL, ancienneté, primes conventionnelles, heures, absences (retenues, maintien de salaire, subrogation des indemnités journalières), cotisations (dizaines de rubriques : maladie, vieillesse plafonnée et déplafonnée, AGIRC-ARRCO par tranches, chômage, AGS, prévoyance et mutuelle conventionnelles, formation, taxe d'apprentissage, contributions diverses), réductions générales, exonérations des heures supplémentaires, **prélèvement à la source** (taux transmis par l'administration via la DSN), montant net social, net à payer, régularisations.
2. **Déclarer** : DSN mensuelle (norme **NEODeS**, cahier technique annuel, contrôles DSN-Val [V-sec : net-entreprises]), signalements d'événements (arrêt de travail, reprise anticipée, fin de contrat), retours des organismes (comptes rendus, taux de prélèvement), attestations.
3. **Payer** : virements des nets, paiement des organismes.
4. **Maintenir** : nouvelle norme chaque année (mise en production obligatoire de la version 2027 au 1-1-2027 [V-sec]), plafond de la sécurité sociale, taux, SMIC, avenants de salaires de la branche (NAO), réformes (montant net social, transparence salariale).
5. **Répondre** des erreurs : un salaire faux, une cotisation omise, une DSN rejetée ont des effets immédiats pour une personne et pour l'employeur. Les éditeurs signent une **charte** avec le GIP-MDS (qualité et complétude des logiciels) [V-sec].

### 3.2 Options

| Option | Description | Coût et risque [H] | Valeur pour l'établissement |
|---|---|---|---|
| **1. Moteur de paie et DSN dans ScolIA** | tout le §3.1 | construire : de l'ordre de **plusieurs années-personne** avant une première DSN fiable, puis une **équipe dédiée** chaque année (norme, taux, CCN) ; **responsabilité d'éditeur de paie** ; tests DSN-Val et relation avec le GIP-MDS | parité littérale avec « Charlemagne Paie » ; un seul outil |
| **2. Administration du personnel et préparation de la paie dans ScolIA ; calcul et DSN par le logiciel de paie de l'OGEC ou de son cabinet** | §10 à §16 dans ScolIA ; échange de fichiers (variables sortantes, bulletins et journal entrants) | **modéré** : formats d'import des éditeurs à suivre (un adaptateur par logiciel) | **la plus grande** : supprime la ressaisie, sécurise les variables, garde le salarié et la direction dans ScolIA ; zéro risque de calcul |
| **3. Intégrer un service de paie en ligne par API** | variables et retour par API d'un éditeur en ligne | dépendance cloud, données de salariés chez un tiers (contrat, sous-traitance RGPD), refusée par défaut (CLAUDE.md) | proche de 2 |

### 3.3 Recommandation [R] : option 2

- **Construire** tout ce qui entoure le calcul, au niveau « masterclass » (lots RH1 à RH10).
- **Échanger** avec le logiciel de paie par un **format pivot** ScolIA (CSV documenté : matricule, période, rubrique ScolIA, quantité, unité, base, date de début et de fin, référence du fait) et un **adaptateur par logiciel** (correspondance des rubriques ScolIA ↔ rubriques ou constantes de l'éditeur, paramétrée avec le gestionnaire de paie). Les éditeurs du marché importent des variables par fichiers (Sage : constantes, rubriques, événements ; Silae : « import de données variables » ; formats propres) [V-sec].
- **Reprendre** du logiciel de paie : les **bulletins** (PDF, un par salarié) déposés au coffre du salarié (socle 7), le **journal de paie** (totaux par rubrique comptable → écritures au 48, R-CG22), les **coûts** par salarié (agrégés pour le budget et l'analytique, jamais visibles hors droit), la **date de dépôt de la DSN** et ses anomalies (attestées par le gestionnaire de paie).
- **Ne pas** produire de DSN ; **suivre** les échéances (signalement d'arrêt à J+5, DSN mensuelle) comme tâches.
- **Garder la porte ouverte** : lots RH11 (calcul de contrôle « brut → net » simplifié pour vérifier les bulletins, sans valeur légale) et RH12 (moteur complet, DSN) **non engagés sans Q-RH1**.

### 3.4 Ce que coûterait l'option 1 (pour éclairer Q-RH1) [H]
- Paramétrage de la **CCN EPNL** (classifications, grilles, ancienneté, primes, annualisation, congés) et de ses avenants annuels ; régimes de prévoyance de branche.
- Environ **une centaine de rubriques** de cotisations et leur maintenance ; taux AT/MP ; réductions générales.
- **NEODeS** : génération, contrôles, dépôt, retours, changement de norme annuel ; signalements ; tests avec le GIP-MDS.
- Prélèvement à la source (taux, CRM, régularisations), indemnités journalières et subrogation, soldes de tout compte, attestations France Travail.
- Ordre de grandeur : une équipe de **2 à 4 personnes à temps plein** dans la durée, une responsabilité juridique d'éditeur ; à comparer au coût d'un logiciel de paie ou d'un cabinet pour 10 à 60 bulletins par mois [H, à chiffrer par Florian avec le cabinet].

---

## 4. Qui est payé par qui

| Personne | Statut | Payeur | Ce que fait ScolIA |
|---|---|---|---|
| Maître contractuel ou agréé (privé sous contrat d'association) | agent public de l'État (C. éduc. L914-1) [V-sec] | **État** (rectorat) | états de service fait, HSE et absences au rectorat (15, 6) ; **aucune paie** |
| Maître délégué (suppléant) | contractuel de l'État (C. éduc. R914-58 renvoie aux règles des contractuels enseignants) [V-sec] | **État** | idem |
| Maître agréé du privé **sous contrat simple** (premier degré) | salarié de l'organisme de gestion (rupture conventionnelle du code du travail applicable) [V-sec, analyse d'avocats] | **OGEC** [NV au détail] | paie préparée (Q-RH8) |
| Enseignant qui fait des surveillances ou des études **payées par l'OGEC** | double engagement (P47 du 6) | **OGEC** pour l'engagement de droit privé | variables de l'engagement OGEC seulement |
| Personnel de vie scolaire, administratif, comptable, entretien, restauration, internat, documentaliste de droit privé, animateur en pastorale | salarié de droit privé, **CCN EPNL IDCC 3218** [V-sec] | **OGEC** | tout le 49 |
| Vacataire, intervenant payé à l'heure | selon contrat | OGEC | service fait (6 PRS-145) → variables |
| Apprenti, stagiaire gratifié | contrat d'apprentissage / convention | OGEC | contrat, échéances ; gratification de stage [NV] |
| AED, AESH (public) | contrats de droit public de l'EPLE ou du rectorat | **EPLE mutualisateur** ou rectorat [V-sec] | états (heures, absences) transmis au payeur ; contrat suivi si l'établissement est employeur |
| Agents de la collectivité (public) | fonctionnaires territoriaux | collectivité | aucun |
| Bénévoles, service civique | pas de salaire | — | aucun (sauf notes de frais au 48) |

---

## 5. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Gestionnaire de paie / comptable de l'OGEC** (ou cabinet) | collecte les variables, saisit, contrôle, lance la paie, dépose la DSN, envoie les bulletins | courir après les absences, les heures, les pièces ; ressaisir ; recoder des libellés ; deviner si une absence est « maladie » | une DSN d'arrêt hors délai ; une retenue injustifiée ; une erreur de salaire ; un salarié oublié | **un fichier de variables arrêté le 20 du mois**, complet, à importer sans retouche ; la file « arrêts à signaler » avec J+5 | mensuelle, quotidienne pour les signalements |
| **Chef d'établissement** (employeur par délégation du président de l'OGEC [H]) | recrute, signe, décide des primes, des sanctions, des ruptures ; arrête les variables | dossiers papier ; échéances oubliées (fin de CDD requalifiée) ; ne pas savoir qui a passé sa visite | un contentieux prud'homal ; une requalification ; un salarié sans visite qui se blesse | un **tableau des échéances** ; des contrats générés depuis des modèles validés par le juriste ; l'arrêté des variables sur mobile en deux minutes | hebdomadaire |
| **Président de l'OGEC** (employeur légal) | signe les contrats des cadres, délègue | n'a pas la vue | sa responsabilité pénale (sécurité au travail) | registre et DUERP à jour, preuve à la main | trimestrielle |
| **Salarié de l'OGEC** (surveillante, secrétaire, agent d'entretien, cuisinier, éducateur d'internat) | travaille, pose ses congés, reçoit son bulletin | demander son solde de congés ; papiers perdus ; bulletins papier | une erreur sur sa paie ; que ses collègues connaissent son salaire ou sa santé | son **espace** : bulletins, compteurs, contrat, entretiens, formations, demandes, en un endroit, sur son téléphone | mensuelle |
| **Enseignant avec heures OGEC** (P47) | surveille, encadre des études | ne sait pas ce qui sera payé | heures oubliées | ses heures **constatées** (appel fait) visibles avant l'arrêté | mensuelle |
| **Responsable d'équipe** (CPE pour les surveillants, chef de cuisine, responsable d'entretien) | plannings, congés de l'équipe, premier niveau de validation | conflits de congés ; plannings changés à la main | une équipe vide un jour de rentrée | validation des congés avec la **vue de l'équipe** et des besoins (planning de service du 6) | hebdomadaire |
| **Secrétariat de direction** | prépare les contrats, les attestations, les dossiers d'embauche | resaisir l'identité ; pièces manquantes | perdre un titre de séjour, une pièce d'identité | dossier d'embauche par **lien** (le futur salarié dépose ses pièces), contrat pré-rempli | à chaque arrivée |
| **Expert-comptable / cabinet de paie** | calcule, déclare, conseille | variables arrivées tard, par mail, incomplètes | la responsabilité d'une erreur sur des données fausses | le format d'import, arrêté, signé, à date fixe | mensuelle |
| **Médecine du travail** (service de prévention et de santé au travail) | visites, avis | listes d'effectifs fausses | — | liste des salariés à convoquer, à jour (sans donnée médicale de retour hormis l'avis) | annuelle |
| **CSE / représentants du personnel** (≥ 11 salariés) | consultation, réclamations | données éparses | — | les documents dus (BDESE si ≥ 50 [NV]) | ponctuelle |
| **Inspection du travail, URSSAF** (contrôle) | contrôle registre, DUERP, affichages, paie | — | — | registre et pièces produits en un clic | rare |
| **Gestionnaire d'EPLE** (public) | transmet les états des AED/AESH au mutualisateur | tableaux manuels | erreurs d'heures | l'état mensuel exporté | mensuelle |

**Ce que montrent ces portraits [R].** (1) Le besoin dominant est la **justesse des variables** et le **respect des échéances**, pas le calcul. (2) Le salarié veut un **espace personnel** complet et confidentiel. (3) La confidentialité (santé, salaire) est la condition de confiance.

---

## 6. L'année vue du terrain [H]

| Période | Ce qui se passe | ScolIA |
|---|---|---|
| Juin–août | recrutements de rentrée, fins de CDD, renouvellements, congés d'été, travaux d'été (agents) | contrats générés, échéances de fin de CDD, dossiers d'embauche par lien, DPAE à faire (tâche), soldes de congés |
| Septembre | rentrée : nouveaux salariés, plannings, visites d'information et de prévention | liste « visites à demander » ; planning de service (6) |
| Chaque mois | absences, heures, primes ; arrêté le N du mois (S-4902) ; paie ; DSN (5 ou 15 du mois suivant [V-sec]) | file des variables, contrôle, arrêté, export, retour des bulletins et du journal |
| Octobre–décembre | entretiens annuels (si pratiqués), plan de formation, NAO de branche | entretiens et formations |
| Janvier | revalorisations (SMIC, grille de la CCN), taux PAS, nouvelle norme DSN | gérées par le logiciel de paie ; ScolIA met à jour la grille de référence pour le budget (48) |
| Février–mars | budget N+1 : masse salariale prévue (48) | projection depuis les contrats et le coût réel des derniers mois |
| Juin | acquisition des congés (période de référence légale du 1-6 au 31-5 [NV], à adapter à la CCN) | compteurs |

---

## 7. Ce qui est réglementaire (vérifié le 2026-11-11)

| # | Règle | Référence | Statut | Conséquence |
|---|---|---|---|---|
| RR-01 | Maîtres du privé sous contrat d'association : agents publics de l'État, rémunérés par l'État, principe de parité | C. éduc. L914-1 | [V-sec] (questions parlementaires, analyses) | jamais de paie ScolIA pour eux (§4) |
| RR-02 | Maîtres délégués du privé sous contrat d'association soumis aux règles des contractuels enseignants du public | C. éduc. R914-58 | [V-sec] | idem |
| RR-03 | **CCN de l'enseignement privé non lucratif (EPNL), IDCC 3218** : texte unique du 11-4-2022 appliqué depuis le 1-9-2022, remplaçant les anciennes conventions du champ ; avenants (salaires, déconnexion du 12-12-2022, classification des CQP) | CCN IDCC 3218 | [V-sec] (Legisocial, Tissot, AG2R, Tripalio) | référentiel « CCN » paramétré (classifications, catégories) pour le **budget** et les contrats ; le calcul reste au logiciel de paie ; **juriste à consulter** pour les modèles de contrats (Q-RH5) |
| RR-04 | OPCO des salariés de l'IDCC 3218 : **AKTO** | | [V-sec, source secondaire unique] | formation : financeur par défaut, à confirmer (Q-RH6) |
| RR-05 | DSN obligatoire, mensuelle ; signalement d'arrêt de travail dans les **5 jours** de la connaissance (maladie non professionnelle), obligatoire aussi pour AT/MP même court ; prolongations par la DSN mensuelle | CSS L133-5-3 ; fiches net-entreprises | [V-sec] (15 R-16, fiches spécialisées) | tâche « signalement à faire avant J+5 » (R-RH20) ; ScolIA ne dépose pas |
| RR-06 | Norme **NEODeS** : version 2027 en production obligatoire au **1-1-2027** (régime général) ; charte de partenariat GIP-MDS / éditeurs (mars 2024) | net-entreprises.fr | [V-sec] | argument du §3 ; aucune DSN dans ScolIA |
| RR-07 | Bulletin de paie : mentions de **R3243-1**, structure en zones ; **montant net social** obligatoire depuis le 1-7-2023 (projet de décret modifiant la rubrique signalé en octobre 2026 [NV]) | C. trav. R3243-1 | [V-sec] | ScolIA **ne génère pas** de bulletin (option 2) ; contrôle de présence des pages reçues |
| RR-08 | Remise **électronique** du bulletin sauf **opposition** du salarié ; employeur : double ou bulletin électronique conservé **5 ans** ; bulletin électronique disponible **50 ans** à compter de l'émission **ou jusqu'à 6 ans après la retraite** du salarié (D3243-8) ; service-public.fr | C. trav. L3243-2, L3243-4, D3243-8 | [V-sec] (service-public.fr repéré, non relu) | coffre du salarié (socle 7) ; **droit d'opposition** enregistré (RHP-160) ; disponibilité au-delà du départ : **accès sans compte actif** par lien nominatif (D-027 C, G) ou remise d'archive au départ (Q-RH9) |
| RR-09 | **Registre unique du personnel** (mentions D1221-23), conservé **5 ans** après le départ | C. trav. L1221-13, D1221-23, R1221-26 | [V-sec] | généré depuis les contrats (RHP-131), jamais saisi deux fois |
| RR-10 | **DPAE** avant l'embauche | C. trav. L1221-10 | [NV] (non relu) | tâche bloquante avant la date d'entrée ; faite dans le logiciel de paie ou sur net-entreprises, **attestée** dans ScolIA |
| RR-11 | **CDD** : mentions obligatoires, motif, terme | C. trav. L1242-12 | [NV] | modèles de contrats ; contrôle du motif ; alerte de fin |
| RR-12 | **Visite d'information et de prévention** après l'embauche (délais selon le poste) ; suivi renforcé selon les risques | C. trav. R4624-10 s. | [NV] | échéances (RHP-200) |
| RR-13 | **DUERP** : mise à jour, conservation **40 ans** | C. trav. R4121-1 s. ; loi du 2-8-2021 | [NV] | document du socle 7, versions conservées 40 ans (§22.4) |
| RR-14 | **Entretien de parcours professionnel** (loi n° 2025-989 du 24-10-2025, dite « seniors ») : dans l'année suivant l'embauche puis **tous les 4 ans** (accord pouvant raccourcir), **bilan à 8 ans** ; accords non conformes inapplicables depuis le 1-10-2026 | C. trav. L6315-1 | [V-sec] (Lefebvre Dalloz, Tissot, Legisocial, AKTO) | échéances calculées (RHP-211) ; compte rendu au dossier |
| RR-15 | **Index de l'égalité** F/H à partir de 50 salariés ; **directive (UE) 2023/970** sur la transparence des rémunérations : transposition due au 7-6-2026, **projet de loi présenté en Conseil des ministres le 10-9-2026, non voté** | C. trav. L1142-8 ; directive 2023/970 | [V-sec] | aucun calcul tant que la loi n'est pas votée ; modèle prêt à recevoir des **catégories d'emploi** et des bandes (§16) |
| RR-16 | **Référentiel CNIL des durées de conservation en gestion des ressources humaines** (publié le 2-4-2026), pour les employeurs de droit privé et public dont les salariés relèvent du droit du travail (hors dossier individuel des agents publics) ; durées imposées (en bleu) et repères (en gris) | cnil.fr | [V-sec] (page CNIL repérée, PDF non lu) | §22.4 aligné sur ce référentiel ; **à relire par le DPO** |
| RR-17 | Taxe sur les salaires : abattement associations **24 256 €** (salaires 2026) | CGI 231, 1679 A ; 2502-SD | [V] | calculée par le logiciel de paie ; prévision au 48 |
| RR-18 | Contrôle d'honorabilité des personnes au contact de mineurs [NV au détail pour les salariés de l'OGEC] | (06, 42) | [NV] | échéance de renouvellement si l'établissement le pratique (S-4920) |
| RR-19 | Données sensibles : santé (RGPD art. 9), appartenance syndicale ; NIR : usage encadré | RGPD ; décret-cadre NIR [NV] | [V principe] | NIR chiffré, jamais affiché en clair, transmis seulement au logiciel de paie (R-RH8) |

**Non vérifié et à faire confirmer par un juriste en droit social (Q-RH5)** : annualisation du temps de travail dans la CCN EPNL, régime des congés (conventionnels, ancienneté), temps partiel modulé des personnels d'éducation, primes conventionnelles, durées des périodes d'essai.

---

## 8. Branchements sur le cœur

### 8.1 Qui possède quoi

| Donnée | Propriétaire | Le 49 |
|---|---|---|
| Agent, engagements, fonctions, missions, service fait | 6 | lit ; un contrat du 49 **référence** l'engagement `salarie_organisme` ; jamais l'inverse |
| Absences, natures codées, heures constatées, rattrapages, retenues | 15 | lit (port `absences::absences_validees`, nature codée) ; transforme en variables |
| Séances assurées (appel fait) | 10, 12 | via le service fait du 6 |
| Planning de service du personnel non enseignant | 6 (R-P10) | lit pour les heures planifiées et les jours travaillés |
| Pièces (contrats signés, pièces d'identité, diplômes, titres de séjour, bulletins, avis d'aptitude) | 7 | dépose dans le **domaine restreint RH** (D-027 A) |
| Écritures de paie | 48 | lui transmet le journal de paie agrégé |
| Budget (masse salariale) | 48 | lui transmet la projection agrégée |
| Habilitations utiles à la journée (secourisme, encadrement) | 6 | les formations du 49 **mettent à jour** l'échéance de l'habilitation du 6 (commande du 6) |
| Instances (CSE) | 46 | lit |

### 8.2 Ports consommés
`personnels::engagements(agent, periode)`, `personnels::service_fait(agent, mois)`, `personnels::missions_remunerees(agent, mois)`, `personnels::planning_service(agent, mois)` (6) ; `absences::absences_validees(agent?, plage)`, `absences::heures_constatees(agent, mois)`, `absences::arrets_a_signaler()` (15) ; `documents::deposer/lire` (7) ; `notifications::avis` (8).

### 8.3 Ports exposés
| Port | Consommateurs | Contenu |
|---|---|---|
| `paie::journal(periode)` | 48 | totaux par rubrique comptable et section analytique ; jamais nominatif |
| `paie::masse_salariale_prevue(exercice)` | 48 | agrégée par catégorie et section |
| `rh::contrat_actif(agent, date)` | 6 (alerte de fin d'engagement), 15 (circuit) | existence, type, terme ; jamais la rémunération |
| `rh::compteurs_conges(agent)` | 15 (demande de congé), le salarié | soldes |
| `rh::echeances(scope)` | 29 (à-faire), 47 | tâches datées |

### 8.4 Répercussions automatiques
- **Engagement clos** au 6 (départ) → tâches de sortie (solde de tout compte à faire par la paie, attestation France Travail, certificat de travail, restitution du matériel (44), fermeture des accès (1), archivage du dossier) ; **le contrat n'est pas clos par le 6** : la fin du contrat est une décision du 49 qui **commande** la fin de l'engagement (ou l'inverse selon l'ordre, cohérence vérifiée, CRH-05).
- **Absence validée, modifiée, requalifiée** (15) → variable recalculée tant que le mois n'est pas arrêté ; après arrêté : **régularisation** sur le mois suivant, jamais de réécriture (R-RH15).
- **Arrêt de travail validé** (15) → tâche « signalement DSN avant J+5 » pour le gestionnaire de paie (R-RH20).
- **Séance d'étude assurée par un enseignant à engagement OGEC** (12 via 6) → heure constatée → variable.
- **Formation obligatoire réalisée** (49) → mise à jour de l'habilitation au 6.

---

## 9. Concepts et vocabulaire [R]

| Terme | Définition | Table |
|---|---|---|
| **Employeur** | l'organisme qui paie (OGEC, association, EPLE employeur) : SIRET, CCN, établissement d'affectation, logiciel de paie | `ent_rh_employeur` |
| **Salarié** | l'agent du 6 vu par son employeur : matricule, NIR chiffré, coordonnées bancaires chiffrées, situation nécessaire à la paie | `ent_rh_salarie` |
| **Contrat** | CDI, CDD (motif, terme), apprentissage, contrat aidé, vacation ; quotité, horaire, classification CCN, date d'ancienneté, période d'essai | `ent_rh_contrat` |
| **Avenant** | modification datée du contrat (quotité, poste, classification, rémunération de base) | `ent_rh_avenant` |
| **Rémunération de référence** | salaire de base contractuel et éléments fixes **tels que portés au contrat** (pas un calcul) ; vue restreinte | `ent_rh_remuneration` |
| **Rubrique ScolIA** | code de variable du format pivot (heures supplémentaires à 25 %, heures d'étude, absence maladie en jours, prime exceptionnelle…) | `ent_rh_rubrique` |
| **Correspondance** | rubrique ScolIA ↔ code du logiciel de paie | `ent_rh_rubrique_editeur` |
| **Élément variable** | une ligne (salarié, mois, rubrique, quantité ou montant décidé, dates, source, état) | `ent_rh_variable` |
| **Période de paie** | le mois : ouverte → **arrêtée** (direction) → exportée → retour reçu → close | `ent_rh_periode` |
| **Retour de paie** | bulletins, journal, coûts, attestations de dépôt | `ent_rh_retour`, `ent_rh_bulletin` |
| **Compteur** | congés payés, RTT/repos, heures, CET [NV] : acquisitions et prises datées | `ent_rh_compteur_mvt` |
| **Échéance** | tâche légale datée (fin d'essai, fin de CDD, visite, entretien, titre de séjour, formation obligatoire) | `ent_rh_echeance` |
| **Visite** | visite de santé au travail : type, date, **avis** (apte, apte avec aménagements, inapte), jamais de motif | `ent_rh_visite` |
| **Entretien** | entretien de parcours, entretien annuel, bilan à 8 ans | `ent_rh_entretien` |
| **Formation** | action, dates, heures, financeur, obligatoire ou non, attestation | `ent_rh_formation` |

---

## 10. Contrats et dossier du salarié

- **Recrutement** (lien avec le 6) : poste ouvert, candidats (option, minimale : nom, coordonnées, CV en pièce, décision ; conservation courte des candidatures non retenues, CNIL [V-sec, 2 ans au plus en repère]), choix, **promesse d'embauche** (modèle).
- **Dossier d'embauche par lien** (D-027 C) : le futur salarié dépose ses pièces (pièce d'identité, carte Vitale ou attestation, RIB, diplômes, titre de séjour le cas échéant, justificatif de domicile si nécessaire [NV]) et ses informations (NIR, coordonnées, personne à prévenir) ; contrôle de complétude ; rien n'est demandé au-delà de la liste de la nature de contrat (minimisation).
- **Contrat** généré depuis **3 ou 4 modèles** validés par un juriste (D-023) : CDI, CDD (motif obligatoire parmi une liste, terme ou durée minimale), temps partiel, apprentissage ; clauses de la CCN ; **signature** électronique (socle 7) par le salarié et l'employeur (délégation du président) ; une version signée est **immuable**.
- **DPAE** : tâche bloquante « DPAE à faire » avant la date d'entrée, attestée (numéro d'accusé, date) ; aucun accès à l'ENT ouvert avant l'attestation si le réglage l'exige (S-4910).
- **Période d'essai** : échéance de fin, alerte à J-15, renouvellement si prévu par le contrat et la CCN [NV], rupture (modèle de courrier, délai de prévenance [NV]).
- **CDD** : alerte avant le terme (J-30, J-15), renouvellement ou succession dans les limites légales [NV : nombre de renouvellements, délai de carence], **requalification** évitée (alerte si un CDD dépasse les limites).
- **Avenants** : quotité, poste, classification, salaire de base ; générés, signés, immuables.
- **Titres de séjour** : date d'expiration suivie, alerte J-90 et J-30 [NV : obligation de vérification de l'autorisation de travail].
- **Sortie** : démission, rupture conventionnelle (procédure, délais, homologation [NV]), licenciement (procédure, entretien préalable, notification : **modèles seulement, avis juridique obligatoire**, Q-RH5), fin de CDD, retraite, décès ; tâches de sortie (§8.4) ; documents de fin de contrat produits par le logiciel de paie et déposés au coffre.
- **Dossier du salarié** : pièces classées par nature (contrat, avenants, identité, diplômes, formations, entretiens, avis d'aptitude, sanctions disciplinaires du droit du travail, courriers) ; **sanctions** : procédure et prescription propres au droit du travail [NV : amnistie, délai de 3 ans pour invoquer une sanction antérieure] ; consultable par le salarié (tout ce qui le concerne, droit d'accès RGPD) ; **jamais** visible par un collègue ou un responsable d'équipe au-delà de ce que sa fonction exige.

---

## 11. Temps, congés et éléments variables

### 11.1 Temps de travail
- Durée contractuelle (hebdomadaire ou **annualisée** selon la CCN et les fonctions [NV]) ; **planning de service** (6 R-P10) ; heures faites constatées ; heures complémentaires (temps partiel) et supplémentaires **décidées** (demande, validation, motif) ou **constatées** (séance d'étude assurée, remplacement de surveillance).
- **Compteur annuel** pour les annualisés : dû, planifié, fait, écart en temps réel, régularisation proposée en fin de période.

### 11.2 Congés
- **Acquisition** (règle de l'employeur paramétrée, avec la CCN [NV]) et **prise** (demandes posées au socle 15, nature `conge_paye`, validées) → mouvements de compteur ; soldes visibles du salarié ; **périodes imposées** (fermeture de l'établissement l'été, calendrier du 5) ; congés pour événements familiaux (nature du 15) ; **acquisition pendant les arrêts maladie** (droit européen et loi du 22-4-2024 [NV]) : règle paramétrée par le gestionnaire de paie, jamais codée.
- **La paie fait foi** pour les soldes valorisés : chaque mois, le retour de paie peut **recaler** les compteurs (RHP-186) ; écart affiché.

### 11.3 Éléments variables
| Source | Variable | Validation |
|---|---|---|
| Absence validée (15) | jours ou heures par **nature codée** (maladie, AT/MP, maternité, paternité, enfant malade, sans solde, grève, absence injustifiée, formation, événement familial…) | validée au 15 ; reprise automatiquement |
| Heures constatées (15, 6) | heures de remplacement, d'étude, de surveillance, complémentaires, supplémentaires (taux) | validées au 15 ou par le responsable |
| Missions rémunérées (6) | nature et volume | décision de la direction |
| Primes et indemnités | prime exceptionnelle, indemnité de fonction, astreinte (internat) [NV], panier, transport (forfait mobilités), avantage en nature repas (cantine, 40) | décision de la direction, montant saisi par un droit restreint |
| Retenues | acomptes, avances, saisies sur salaire (avis du juge transmis par le gestionnaire de paie) | gestionnaire de paie |
| Entrées, sorties, changements | date d'entrée, de sortie, avenant | 49 |
| Notes de frais | (au 48, **hors paie**) | — |

- **Contrôles** avant l'arrêté (RHP-176) : absence validée sans pièce attendue ; heures non validées ; salarié sans variable alors qu'il a eu des absences ; écart de plus de S-4904 % avec le mois précédent ; variable sur un salarié sorti ; double source (même absence importée deux fois) ; heures au-delà des limites légales [NV] ; prime sans décision.
- **Arrêté** par la direction (N3, mobile possible), puis **export**.

---

## 12. Préparation de la paie et échange avec le logiciel de paie

1. **Ouverture** de la période (automatique au 1er du mois).
2. **Collecte** continue (§11.3) : chaque fait produit ou met à jour une variable en brouillon, idempotente par clé de fait (R-RH14).
3. **Contrôle** (RHP-176) et corrections par les propriétaires des faits (le 15 pour une absence, le responsable pour des heures).
4. **Arrêté** à la date S-4902 (le 20 par défaut [H]) par la direction ★ ; tout fait postérieur va au mois suivant (régularisation).
5. **Export** au format du logiciel de paie (adaptateur) ★ : fichier objet (E5), empreinte, numéro ; **remise** au gestionnaire (téléchargement, ou dépôt sur un espace convenu ; jamais par courriel en pièce jointe, D-027 G) ; accusé de prise en compte.
6. **Retour** : import des **bulletins** (PDF par salarié, rapprochés par matricule, contrôle « un bulletin par salarié payé ») déposés au coffre (sauf opposition : remise papier, R-RH12), du **journal de paie** (CSV ou FEC partiel du logiciel → 48), des **coûts** par salarié (agrégés pour le 48), des **compteurs** (recalage) et de l'**attestation de dépôt de la DSN** (date, statut) ★.
7. **Clôture** de la période ; anomalies du retour listées (bulletin manquant, salarié inconnu, total du journal différent des variables attendues).
8. **Régularisations** : sur le mois suivant, jamais par réécriture (R-RH15).

**Pendant le doublon (D-034)** : la paie de l'OGEC est faite comme aujourd'hui (le gestionnaire actuel) ; ScolIA peut **produire le fichier de variables en parallèle** et le gestionnaire compare, ou être en **usage réel** si Florian le décide (Q-RH3, R-RH35) : la RH de l'add-on Next.js existe déjà (ébauche), Charlemagne a un module Paie [V-sec].

---

## 13. Bulletins, journal, DSN

- **Bulletins** : reçus du logiciel de paie ; stockés au coffre du salarié (socle 7, domaine RH) ; **jamais** consultables par la direction en masse (accès d'un bulletin individuel journalisé, motif) ; disponibles **50 ans ou jusqu'à 6 ans après la retraite** (RR-08) : remise de l'archive des bulletins au départ (fichier) et **accès après le départ** par lien nominatif renouvelable (Q-RH9).
- **Journal de paie** : agrégé par rubrique comptable et par section (R-CG22) ; écritures produites au 48 (journal PA) ; écart signalé si les totaux ne concordent pas avec les variables arrêtées.
- **Virements des nets** : faits par le logiciel de paie ou par l'OGEC (fichier pain.001 du logiciel de paie) ; si l'OGEC veut les passer par le 48 (lot de virement), import du fichier des nets **sans** nom ni montant visibles hors droit `paie.payer` (Q-RH10).
- **DSN** : jamais produite (option 2) ; **tâches** : signalement d'arrêt à J+5 (RR-05), DSN mensuelle (échéance S-4905), fin de contrat ; attestation de dépôt et anomalies retournées saisies par le gestionnaire.

---

## 14. Santé et sécurité au travail (trou T-06 de la matrice)

- **DUERP** : document du socle 7, versions datées conservées 40 ans (RR-13), plan d'actions de prévention (tâches), consultation du CSE ; lien avec les registres de sécurité de l'établissement (socle 14, 44) pour les risques liés aux bâtiments.
- **Visites** de santé au travail : liste des salariés à convoquer (embauche, périodique, reprise après arrêt de 60 jours [NV], pré-reprise, suivi renforcé) transmise au service de santé au travail (effectifs, postes, sans donnée médicale) ; **avis** enregistré (type, date, conclusion) sans motif (R-RH6) ; aménagements de poste : description fonctionnelle seulement (« pas de port de charges ») visible du responsable d'équipe, jamais la cause.
- **Accidents du travail** : déclaration (délai de 48 h de l'employeur [NV]) préparée, **signalement DSN** (tâche), registre des accidents bénins [NV] ; jamais de diagnostic.
- **Habilitations et formations obligatoires** : SST, électricité, manipulation de produits, hygiène alimentaire (cuisine, PMS du 40), échéances de recyclage ; lien avec les habilitations du 6.
- **Équipements de protection** remis (traçabilité, 44).
- **Affichages obligatoires** et documents (règlement intérieur, si ≥ 50 [NV]) : liste de contrôle.

---

## 15. Entretiens et formation

- **Entretien de parcours professionnel** (RR-14) : échéance dans l'année de l'embauche puis tous les 4 ans (ou moins par accord, S-4912), **bilan à 8 ans** ; trame du socle 7, compte rendu signé des deux, copie au salarié ; retour de longue absence (congé maternité, parental, arrêt long) : entretien proposé [V-sec : Tissot] .
- **Entretien annuel d'évaluation** (facultatif) : trame distincte, objectifs ; jamais de note visible d'autrui.
- **Formation** : demandes (salarié, responsable), **plan de développement des compétences** (budget, priorités), inscriptions, attestations, heures ; financeur (OPCO AKTO par défaut, RR-04), CPF (information, pas de gestion [NV]) ; formations obligatoires (§14).
- **Pour les enseignants agents de l'État**, la formation continue relève de l'employeur public (Formiris dans l'enseignement catholique [H]) : **hors 49** ; le 6 garde les qualifications utiles.

---

## 16. Obligations collectives

- **Effectifs** : calcul des seuils (11, 50 salariés [NV au mode de calcul : moyenne sur 12 mois, temps partiels au prorata]) → obligations qui s'allument (CSE, index de l'égalité, règlement intérieur, BDESE) ; affichage du calcul.
- **CSE** : calendrier des élections (lien 46) [NV], consultations, documents.
- **Index de l'égalité** (≥ 50) : calcul fait **par le logiciel de paie** ou sur Egapro [NV] ; ScolIA tient les **catégories d'emploi** (CCN) prêtes pour la transparence salariale (RR-15).
- **Taxe sur les salaires**, **contributions** : logiciel de paie ; prévision au 48.
- **Bilan social** (si exigé [NV]), **rapports** à la direction : effectifs par catégorie, entrées et sorties, absentéisme **agrégé** (jamais par nature médicale en dessous de 5 personnes, R-RH25), heures supplémentaires, formation.

---

## 17. Public (EPLE)

- Pas de paie locale de droit commun (RR : agents de l'État et de la collectivité).
- **AED, AESH** : si l'EPLE est **employeur** (le chef d'établissement signe le contrat) avec un **mutualisateur** qui paie [V-sec] : ScolIA suit le contrat (dates, quotité, durée annuelle de référence 1 607 h pour les AED [V-sec, académie de Versailles]), le service fait, les absences, et **exporte l'état mensuel** au mutualisateur (format à obtenir, Q-RH11).
- **Contrats aidés**, **vacations** : suivi et états.
- **HSE des enseignants** : état mensuel au rectorat (socle 15) — hors 49.

---

## 18. Cas limites (catalogue)

| # | Cas | Traitement |
|---|---|---|
| CRH-01 | Un professeur (État) fait aussi des surveillances payées par l'OGEC | seul l'engagement OGEC produit des variables (P47) ; ses absences d'enseignant ne retiennent rien sur sa paie OGEC sauf si elles portent sur une surveillance (15 distingue la séance) |
| CRH-02 | Absence d'un salarié validée après l'arrêté du mois | régularisation sur le mois suivant (R-RH15) |
| CRH-03 | Absence requalifiée « maladie » → « accident du travail » après export | régularisation ; tâche « signalement AT » ; aucune mention médicale |
| CRH-04 | Salarié en arrêt depuis 4 mois : la DSN mensuelle porte les prolongations | rien à signaler en événementiel pour une prolongation [V-sec] ; tâche seulement pour l'arrêt initial |
| CRH-05 | Le 6 clôt l'engagement mais le contrat court encore (préavis) | incohérence signalée ; la fin de contrat (49) fait foi pour la paie ; l'accès aux outils suit l'engagement (6) |
| CRH-06 | Embauche le lundi, DPAE non attestée le vendredi | tâche bloquante, alerte à la direction ; ouverture des accès retenue si S-4910 |
| CRH-07 | CDD de remplacement dont le salarié remplacé revient plus tôt | terme réel = retour du remplacé (motif de remplacement) [NV] ; alerte à la direction |
| CRH-08 | Troisième CDD successif sur le même poste | alerte de requalification probable ; décision humaine avec avis juridique |
| CRH-09 | Salarié qui s'oppose au bulletin électronique | bulletin papier remis ; opposition enregistrée et réversible (R-RH12) |
| CRH-10 | Salarié parti depuis 3 ans qui demande un bulletin de 2026 | lien nominatif vers son archive (Q-RH9) |
| CRH-11 | Un responsable d'équipe veut savoir pourquoi une surveillante est absente | il voit « absente du … au … », jamais la nature (R-RH6, 15) |
| CRH-12 | Inaptitude prononcée par le médecin du travail | avis « inapte » enregistré ; procédure de reclassement : modèles et avis juridique ; aucun motif médical |
| CRH-13 | Saisie sur salaire notifiée par un huissier | gestionnaire de paie seul ; montant hors ScolIA ; ScolIA garde l'existence de la procédure en accès restreint ou rien (S-4914) |
| CRH-14 | Salarié de l'OGEC affecté à deux établissements du même OGEC | un contrat, un employeur ; deux engagements (un par établissement) ; le contrat est porté par l'établissement **gestionnaire** du contrat (R-RH4) ; l'autre ne voit que l'engagement |
| CRH-15 | Le logiciel de paie change en cours d'année | nouvel adaptateur ; correspondances des rubriques reprises ; mois de bascule doublé (contrôle croisé) |
| CRH-16 | Bulletin reçu pour un salarié inconnu de ScolIA | anomalie, bulletin en quarantaine, jamais déposé à un coffre au hasard |
| CRH-17 | Deux salariés homonymes | rapprochement par matricule, jamais par nom |
| CRH-18 | Un salarié est aussi parent d'élève | deux casquettes (06) ; son dossier RH n'est jamais visible depuis sa casquette parent et inversement |
| CRH-19 | Le directeur est salarié de l'OGEC | son dossier, ses variables et ses bulletins sont gérés par le président ou le gestionnaire de paie ; il n'arrête **pas** ses propres variables (R-RH22) |
| CRH-20 | Le comptable de l'OGEC est le gestionnaire de paie : il voit son propre salaire et celui des collègues | séparation : ses propres variables sont arrêtées par la direction ; consultations journalisées ; revue annuelle des accès |
| CRH-21 | Prime exceptionnelle décidée à l'oral | aucune variable sans décision tracée (R-RH16) |
| CRH-22 | Heures d'étude assurées mais appel non fait | heure non constatée ; le salarié peut attester, son responsable valide (15) |
| CRH-23 | Grève | nature `greve` codée ; retenue proportionnelle (privé) calculée par la paie ; aucune liste de grévistes diffusée (R-RH26) |
| CRH-24 | Un salarié demande l'accès à tout son dossier (RGPD) | export de son dossier et de ses bulletins (RHP-245), sous 1 mois ; notes internes non communicables : il n'en existe pas (R-RH27) |
| CRH-25 | Candidat non retenu | données supprimées après S-4915 (2 ans au plus en repère CNIL [V-sec]) sauf accord |
| CRH-26 | Décès d'un salarié | tâches : solde, documents aux ayants droit (procédure), fermeture des accès, aucune notification automatique à l'équipe (décision de la direction) |
| CRH-27 | Congé maternité puis parental | natures codées ; entretien de reprise ; compteurs selon la règle paramétrée |
| CRH-28 | Apprenti mineur | échéances spécifiques (travaux réglementés [NV]) ; tuteur ; contrat signé par le représentant légal |
| CRH-29 | Titre de séjour expiré | alerte, tâche bloquante à la direction ; **aucune** action automatique sur le contrat |
| CRH-30 | Avenant de baisse de quotité signé en cours de mois | prorata au logiciel de paie ; variable « changement de quotité » datée |
| CRH-31 | Le fichier d'export est importé deux fois dans le logiciel de paie | identifiant de lot et empreinte ; le retour (journal) le révèle ; tâche de vérification |
| CRH-32 | Restauration de la base après l'export d'un mois | export retrouvé par empreinte (objet) ; état « exporté » réconcilié ; aucune réexportation automatique (E9) |
| CRH-33 | Directeur absent le jour de l'arrêté | délégation datée (6) ; à défaut, le président |
| CRH-34 | Salarié en télétravail (secrétariat) | indemnité éventuelle par accord ; variable décidée [NV] |
| CRH-35 | Effectif qui franchit 50 salariés | obligations listées (§16), tâches à la direction |
| CRH-36 | AED (public) employé par l'EPLE, payé par le mutualisateur, absent | état mensuel au mutualisateur avec nature codée ; aucune paie locale |
| CRH-37 | Salarié qui travaille pour l'OGEC et pour l'APEL | deux employeurs ; ScolIA ne gère que l'OGEC (l'APEL n'est pas un employeur paramétré, S-4916) |
| CRH-38 | Formation obligatoire non recyclée | habilitation expirée au 6 → le salarié n'est plus proposé pour la tâche concernée (encadrement de sortie, cuisine) |
| CRH-39 | Accident du travail d'un salarié dans la cour | déclaration préparée, signalement DSN (tâche) ; si un élève est impliqué : lien avec le 14 **sans** partager le dossier RH |
| CRH-40 | Le gestionnaire de paie externe a besoin d'un accès | accès temporaire au seul espace d'échange (exports, retours), journalisé (R-RH30) |
| CRH-41 | La direction veut comparer les salaires de deux surveillantes | `paie.lire_remunerations` (restreint) ; journalisé ; préparation de la transparence salariale (§16) |
| CRH-42 | Une absence de professeur (État) est saisie par erreur dans les variables OGEC | impossible : la variable exige un engagement OGEC à la date (R-RH3) |
| CRH-43 | Demande de congé qui fait passer un compteur sous zéro | permise si la règle de l'employeur l'autorise (anticipation) ; signalée |
| CRH-44 | Sanction disciplinaire du droit du travail | modèles et procédure (entretien préalable, notification) avec avis juridique ; pièce au dossier ; jamais au dossier d'établissement du 6 ni au 13 (discipline des élèves) |
| CRH-45 | Le salarié conteste une variable (heures oubliées) | réclamation dans son espace ; correction par le propriétaire du fait ; régularisation |

---

## 19. Intelligence artificielle

**Fait [R]** : contrôle des variables (écarts inhabituels, oublis probables, doubles sources) ; brouillons de contrats et d'avenants depuis les **modèles validés** (jamais de clause inventée) ; explication d'un compteur (« pourquoi il me reste 12 jours ») ; préparation de la liste des échéances du mois ; brouillon de trame d'entretien ; lecture d'un bulletin reçu pour le rapprocher du salarié (matricule, période) **sans** extraire de montant pour l'IA.
**Ne fait pas** : aucune validation de variable, aucun arrêté, aucune signature, aucun envoi ; **aucune lecture de donnée de santé** (il n'y en a pas, et l'IA n'a pas accès aux avis d'aptitude) ; aucun conseil juridique sur une rupture ou une sanction (renvoi au juriste) ; aucune lecture des rémunérations individuelles sauf pour un utilisateur qui a `paie.lire_remunerations`, et jamais vers un fournisseur d'IA hors cadre.
**Capacités** : `rh.mes_compteurs` (N0, le salarié), `rh.echeances` (N0, direction), `paie.controle_variables` (N0, gestionnaire), `rh.brouillon_contrat` (N2, secrétariat), `rh.brouillon_trame_entretien` (N2) ; arrêté, export, signature, rupture : **non exposés**.

---

## 20. Actions métier (RHP-100 à RHP-279)

Acteurs : **DIR** direction (employeur délégué) · **PRES** président de l'OGEC · **GP** gestionnaire de paie (comptable ou cabinet) · **SEC** secrétariat RH · **RESP** responsable d'équipe · **SAL** salarié · **CAND** candidat ou futur salarié (lien) · **SYS** · **IA**. ★ = N3.

### 20.1 Paramétrage (RH1)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-100 | Déclarer l'**employeur** (OGEC : SIRET, CCN, logiciel de paie, établissement gestionnaire) | DIR · `rh.parametrer` | |
| RHP-101 | Charger le référentiel **CCN EPNL** (catégories, classifications) | GP · idem | pour contrats et budget ; pas de calcul |
| RHP-102 | Définir les **rubriques ScolIA** actives | GP · idem | |
| RHP-103 | Paramétrer l'**adaptateur** du logiciel de paie (correspondances) | GP · idem | Q-RH2 |
| RHP-104 | Paramétrer la date d'arrêté mensuel | DIR · idem | S-4902 |
| RHP-105 | Paramétrer les règles d'acquisition des congés | GP · idem | avec la CCN [NV] |
| RHP-106 | Paramétrer les modèles de contrats, avenants, courriers | DIR, SEC · idem | 3 ou 4 trames validées par un juriste |
| RHP-107 | Paramétrer les listes de pièces par nature de contrat | DIR · idem | minimisation |
| RHP-108 | Définir les **délégations** (arrêté, signature) | PRES, DIR · `personnels.gerer` (6) | datées |
| RHP-109 | Déclarer le service de santé au travail et l'OPCO | SEC · `rh.parametrer` | |
| RHP-110 | Consulter le journal des paramètres | DIR, GP | |

### 20.2 Recrutement, embauche, contrat (RH1, RH2)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-111 | Ouvrir un poste à pourvoir | DIR · `rh.recruter` | lien 6 (engagement prévu) |
| RHP-112 | Enregistrer un candidat (minimal) | SEC · idem | |
| RHP-113 | Décider (retenu, non retenu) | DIR · idem | avis au candidat ★ |
| RHP-114 | Purger les candidatures non retenues | SYS | S-4915 |
| RHP-115 | Envoyer le **lien de dossier d'embauche** ★ | SEC · `rh.dossier.gerer` | D-027 C |
| RHP-116 | Déposer ses pièces et informations | CAND (lien) | NIR, IBAN chiffrés |
| RHP-117 | Contrôler la complétude du dossier | SEC · idem | |
| RHP-118 | Générer le **contrat** depuis un modèle | SEC · `rh.contrat.preparer` | IA brouillon possible |
| RHP-119 | Contrôler le motif et le terme d'un CDD | SYS | RR-11 |
| RHP-120 | Faire signer le contrat (salarié, employeur) ★ | DIR · `rh.contrat.signer` | socle 7 |
| RHP-121 | Attester la **DPAE** (numéro, date) | GP, SEC · `rh.dossier.gerer` | tâche bloquante |
| RHP-122 | Créer l'engagement au 6 (commande) depuis le contrat | SYS | cohérence |
| RHP-123 | Enregistrer la rémunération de référence | GP, DIR · `paie.remunerations.gerer` | restreint |
| RHP-124 | Générer un **avenant** | SEC · `rh.contrat.preparer` | |
| RHP-125 | Signer un avenant ★ | DIR · `rh.contrat.signer` | |
| RHP-126 | Suivre la **période d'essai** (fin, renouvellement, rupture) | DIR · `rh.dossier.gerer` | alerte J-15 |
| RHP-127 | Suivre le terme d'un **CDD** (renouvellement, succession) | DIR · idem | alerte J-30, J-15 |
| RHP-128 | Alerter sur un risque de **requalification** | SYS → DIR | CRH-08 |
| RHP-129 | Suivre un **titre de séjour** | SEC · idem | J-90, J-30 |
| RHP-130 | Enregistrer un apprenti (tuteur, représentant légal) | SEC · idem | CRH-28 |
| RHP-131 | Produire le **registre unique du personnel** | SYS ; DIR · `rh.registre.lire` | RR-09 |
| RHP-132 | Consulter un contrat et ses avenants | DIR, GP · `rh.dossier.lire` ; SAL (le sien) | |

### 20.3 Dossier du salarié (RH1)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-133 | Consulter son **dossier** complet | SAL | droit d'accès |
| RHP-134 | Mettre à jour ses coordonnées, personne à prévenir | SAL | socle 4/6 pour l'identité |
| RHP-135 | Demander la modification de son **IBAN** (pièce, confirmation par passkey) | SAL | R-RH9 |
| RHP-136 | Valider un changement d'IBAN ★ | GP · `paie.iban.valider` | anti-fraude |
| RHP-137 | Déposer une pièce (diplôme, attestation) | SAL, SEC | |
| RHP-138 | Classer, consulter une pièce | SEC · `rh.dossier.gerer` | journalisé |
| RHP-139 | Enregistrer une **sanction** du droit du travail (procédure) ★ | DIR · `rh.disciplinaire` | avis juridique (Q-RH5) |
| RHP-140 | Produire une **attestation employeur** | SEC · `rh.dossier.gerer` ; SAL (demande) | modèles |
| RHP-141 | Exporter son dossier (RGPD) | SAL ; DIR | CRH-24 |
| RHP-142 | Fusionner deux dossiers (doublon) | GP · `rh.parametrer` | via le 6 |

### 20.4 Temps et congés (RH3)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-143 | Définir la durée contractuelle et l'annualisation | GP · `rh.temps.gerer` | |
| RHP-144 | Lire le planning de service (6) | RESP, SAL | |
| RHP-145 | Demander des heures complémentaires ou supplémentaires | RESP | motif |
| RHP-146 | Valider des heures ★ | DIR · `rh.temps.valider` | |
| RHP-147 | Attester une heure non constatée (appel non fait) | SAL ; RESP valide | CRH-22 |
| RHP-148 | Compteur annuel (dû, planifié, fait) | SAL, RESP, GP | |
| RHP-149 | Régularisation de fin de période d'annualisation | GP · idem | proposée |
| RHP-150 | Mouvements de **compteurs de congés** (acquisition) | SYS | règle paramétrée |
| RHP-151 | Poser un congé (port vers le 15) | SAL | |
| RHP-152 | Valider un congé (vue de l'équipe) | RESP · 15 | |
| RHP-153 | Déclarer des congés imposés (fermeture) | DIR · `rh.temps.gerer` | calendrier du 5 |
| RHP-154 | Corriger un compteur (motif) | GP · idem | |
| RHP-155 | Recaler les compteurs sur le retour de paie | SYS | RHP-186 |
| RHP-156 | Consulter ses compteurs | SAL | mobile |
| RHP-157 | Expliquer un compteur (IA) | IA | N0 |
| RHP-158 | Alerter sur des congés non pris avant la fin de période | SYS → SAL, RESP | |
| RHP-159 | Compte épargne-temps (si accord) [NV] | GP | |
| RHP-160 | Enregistrer l'**opposition** au bulletin électronique | SAL | RR-08 |

### 20.5 Variables et arrêté (RH4)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-161 | Ouvrir la période de paie | SYS | |
| RHP-162 | Importer les absences validées (15) en variables | SYS | idempotent |
| RHP-163 | Importer les heures constatées (15, 6) | SYS | |
| RHP-164 | Importer les missions rémunérées (6) | SYS | |
| RHP-165 | Saisir une **prime** ou indemnité (décision) | DIR · `paie.variables.decider` | R-RH16 |
| RHP-166 | Saisir une retenue (acompte, avance) | GP · `paie.variables.gerer` | |
| RHP-167 | Saisir un avantage en nature (repas, 40) | SYS / GP | |
| RHP-168 | Saisir une indemnité de transport | GP | |
| RHP-169 | Enregistrer une entrée, une sortie, un changement en variable | SYS | |
| RHP-170 | Consulter les variables d'un salarié | GP · `paie.variables.gerer` ; SAL (les siennes, sans montant de prime non encore décidée) | |
| RHP-171 | Consulter ses heures et absences du mois avant l'arrêté | SAL | contester |
| RHP-172 | Contester une variable | SAL | CRH-45 |
| RHP-173 | Corriger une variable (par le propriétaire du fait) | 15, RESP, GP | |
| RHP-174 | Ajouter une variable manuelle (motif, pièce) | GP · idem | |
| RHP-175 | Supprimer une variable : non, **annuler** (motif) | GP · idem | R-RH17 |
| RHP-176 | **Contrôler** la période (liste d'anomalies) | SYS, GP | §11.3 |
| RHP-177 | Contrôle IA des écarts | IA | N0 |
| RHP-178 | **Arrêter** la période ★ | DIR · `paie.arreter` | jamais ses propres variables (R-RH22) |
| RHP-179 | Rouvrir une période arrêtée non exportée (motif) ★ | DIR | |
| RHP-180 | Reporter les faits tardifs en régularisation | SYS | R-RH15 |

### 20.6 Échange avec le logiciel de paie et retour (RH5)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-181 | **Exporter** les variables (adaptateur) ★ | GP · `paie.exporter` | objet, empreinte |
| RHP-182 | Re-télécharger un export | GP | identique |
| RHP-183 | Annuler un export (motif) ★ | GP + DIR | |
| RHP-184 | Importer les **bulletins** | GP · `paie.retour.importer` | rapprochement par matricule |
| RHP-185 | Importer le **journal de paie** (→ 48) | GP · idem | R-CG22 |
| RHP-186 | Importer les compteurs et les coûts | GP · idem | |
| RHP-187 | Attester le **dépôt de la DSN** et ses anomalies ★ | GP · idem | |
| RHP-188 | Déposer les bulletins aux coffres ★ | SYS après validation GP | sauf opposition |
| RHP-189 | Mettre en quarantaine un bulletin non rapproché | SYS | CRH-16 |
| RHP-190 | Contrôler le retour (bulletin manquant, totaux) | SYS, GP | |
| RHP-191 | Clore la période | GP · `paie.exporter` | |
| RHP-192 | Consulter ses **bulletins** | SAL | mobile ; accès après départ (Q-RH9) |
| RHP-193 | Consulter un bulletin individuel (motif, journalisé) | DIR · `paie.bulletins.lire` | |
| RHP-194 | Importer le fichier des nets pour paiement via le 48 (option) | GP · `paie.payer` | Q-RH10 |
| RHP-195 | Remettre l'archive des bulletins au départ | SYS → SAL | |
| RHP-196 | Changer de logiciel de paie (nouvel adaptateur, mois doublé) | GP · `rh.parametrer` | CRH-15 |
| RHP-197 | Ouvrir un accès temporaire au cabinet de paie ★ | DIR | R-RH30 |
| RHP-198 | Journal de paie : écart avec les variables | SYS → GP | |

### 20.7 Échéances, santé et sécurité au travail (RH6, RH7)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-199 | Tableau des **échéances** (essai, CDD, visites, entretiens, titres, formations, DSN) | DIR, SEC · `rh.echeances.lire` | |
| RHP-200 | Liste des **visites** à demander au service de santé au travail | SEC · `rh.sst.gerer` | sans donnée médicale |
| RHP-201 | Enregistrer un **avis** (apte, aménagements, inapte) | SEC · idem | R-RH6 |
| RHP-202 | Décrire un aménagement de poste (fonctionnel) | DIR · idem | visible du RESP |
| RHP-203 | Préparer une **déclaration d'accident du travail** | SEC · idem | délai [NV] |
| RHP-204 | Tâche « signalement DSN arrêt avant J+5 » | SYS → GP | R-RH20 |
| RHP-205 | Tâche « signalement AT » | SYS → GP | |
| RHP-206 | Gérer le **DUERP** (versions, plan d'actions) | DIR · `rh.sst.gerer` | 40 ans |
| RHP-207 | Registre des accidents bénins [NV] | SEC · idem | |
| RHP-208 | Suivre les **formations obligatoires** et recyclages | SEC · `rh.formation.gerer` | lien 6 |
| RHP-209 | Tracer la remise d'équipements de protection | RESP · idem | lien 44 |
| RHP-210 | Liste de contrôle des affichages et documents obligatoires | DIR | |

### 20.8 Entretiens et formation (RH8)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-211 | Calculer les échéances d'**entretien de parcours** et de **bilan à 8 ans** | SYS | RR-14 |
| RHP-212 | Planifier un entretien (rendez-vous, socle 19) | DIR, RESP · `rh.entretiens.gerer` | |
| RHP-213 | Préparer l'entretien (trame, historique de formation) | DIR ; IA brouillon | |
| RHP-214 | Rédiger et signer le compte rendu (deux parties) ★ | DIR, SAL | copie au salarié |
| RHP-215 | Entretien de retour d'absence longue | SYS → DIR | |
| RHP-216 | Entretien annuel d'évaluation (facultatif) | DIR, RESP | |
| RHP-217 | Demander une formation | SAL, RESP | |
| RHP-218 | Instruire et décider une demande ★ | DIR · `rh.formation.gerer` | |
| RHP-219 | Construire le **plan de développement des compétences** | DIR · idem | budget 48 |
| RHP-220 | Inscrire, suivre la présence, déposer l'attestation | SEC · idem | |
| RHP-221 | Demander une prise en charge à l'OPCO (pièces) | SEC · idem | |
| RHP-222 | Consulter son historique de formation | SAL | |

### 20.9 Sorties (RH2)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-223 | Enregistrer une **démission** (date, préavis) | DIR · `rh.contrat.signer` | |
| RHP-224 | Conduire une **rupture conventionnelle** (étapes, délais, modèles) | DIR · idem | avis juridique |
| RHP-225 | Conduire une procédure de **licenciement** (étapes, modèles) ★ | DIR, PRES · idem | avis juridique obligatoire |
| RHP-226 | Fin de CDD, départ ou mise à la retraite | DIR · idem | |
| RHP-227 | Générer les **tâches de sortie** (solde, attestations, accès, matériel, archivage) | SYS | §8.4 |
| RHP-228 | Déposer les documents de fin de contrat reçus de la paie | GP | coffre |
| RHP-229 | Clore le contrat et commander la fin de l'engagement (6) | SYS | CRH-05 |
| RHP-230 | Gérer un décès (procédure, ayants droit) | DIR | CRH-26 |

### 20.10 Collectif, rapports, public (RH9, RH10)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-231 | Calculer les **effectifs** et les seuils | SYS ; DIR | §16 |
| RHP-232 | Obligations qui s'allument (CSE, index, règlement intérieur) | SYS → DIR | |
| RHP-233 | Catégories d'emploi pour la transparence salariale | GP · `rh.parametrer` | RR-15 |
| RHP-234 | Rapport RH agrégé (effectifs, mouvements, absentéisme agrégé, heures, formation) | DIR · `rh.rapports` | R-RH25 |
| RHP-235 | **Masse salariale** prévue pour le budget (→ 48) | GP · `paie.lire_agrege` | |
| RHP-236 | Coût réel agrégé par section (→ 48) | SYS | |
| RHP-237 | Indicateurs au pilotage (47) | SYS | agrégés |
| RHP-238 | Documents pour le CSE | DIR | |
| RHP-239 | Public : état mensuel AED/AESH au mutualisateur ★ | GEST (public) · `rh.etats.exporter` | Q-RH11 |
| RHP-240 | Public : suivi des contrats AED/AESH employés par l'EPLE | GEST · `rh.dossier.gerer` | |
| RHP-241 | Public : durée annuelle de référence AED (1 607 h) et compteur | SYS | [V-sec] |
| RHP-242 | État des heures d'un vacataire pour l'agent comptable (public) | GEST | |
| RHP-243 | Comparer les rémunérations (accès restreint) | DIR · `paie.lire_remunerations` | journalisé |
| RHP-244 | Simuler l'effet d'un avenant de salaire de la branche sur le budget (agrégé) | GP | 48 |

### 20.11 Système, sécurité, reprise (RH0, RH5)
| N° | Action | Acteur · droit | Notes |
|---|---|---|---|
| RHP-245 | Export RGPD du dossier d'un salarié | DIR, SAL | |
| RHP-246 | Purger selon les durées (§22.4) | SYS | après avis du DPO |
| RHP-247 | Journaliser toute consultation de rémunération, de bulletin, de NIR, d'IBAN, d'avis d'aptitude | SYS | E7 |
| RHP-248 | Revue annuelle des accès RH et paie | DIR | |
| RHP-249 | Neutraliser à la restauration les envois en file (liens d'embauche, dépôts de bulletins) | SYS | E9 |
| RHP-250 | Importer les salariés et contrats en cours (reprise) | GP · `rh.parametrer` | IBAN et NIR **en attente** de confirmation |
| RHP-251 | Importer les compteurs de congés à date | GP · idem | |
| RHP-252 | Importer les HSE de l'année (Next.js M08) | GP · idem | §26 |
| RHP-253 | Mode parallèle (doublon) : export « à blanc » | GP · `paie.exporter` | R-RH35 |
| RHP-254 | Passer en réel ★ | DIR | décision de Florian |
| RHP-255 | Garde : aucune colonne de rémunération hors brique (test de schéma) | SYS (CI) | RH0 |
| RHP-256 | Garde : nature d'absence codée exigée à l'import | SYS | RH0, HD-04 |
| RHP-257 | Garde : variable exige un engagement de l'employeur à la date | SYS | R-RH3 |
| RHP-258 | Notifications (échéance, variable à contester, bulletin disponible, contrat à signer) | SYS | §23 |
| RHP-259 | File de travail du gestionnaire de paie et du secrétariat RH | GP, SEC | socle 29 |
| RHP-260 | Aide intégrée (glossaire, pas de conseil juridique) | tous | |
| RHP-261 | Export de portabilité du domaine (D-046) | administrateur | double validation |
| RHP-262 | Rapport d'anomalies de reprise | SYS | |
| RHP-263 à RHP-279 | (réservés aux lots RH11–RH12 si Q-RH1 = moteur de paie, et aux ajouts personas, §31) | | |

---

## 21. Règles transverses (R-RH1 à R-RH40) [R]

| N° | Règle |
|---|---|
| R-RH1 | ScolIA ne calcule ni ne déclare la paie (option 2), sauf décision contraire de Florian (Q-RH1). |
| R-RH2 | Les agents de l'État (maîtres du privé sous contrat, enseignants du public) n'ont jamais de variable de paie ScolIA. |
| R-RH3 | Une variable exige un engagement actif de l'employeur à la date du fait. |
| R-RH4 | Un contrat appartient à un employeur et est porté par un établissement gestionnaire ; les autres établissements ne voient que l'engagement. |
| R-RH5 | Le dossier du salarié est un domaine restreint (D-027 A), distinct de la fiche du 6. |
| R-RH6 | Aucune donnée de santé : nature d'absence codée ; avis d'aptitude sans motif ; aménagement décrit fonctionnellement. |
| R-RH7 | Aucune colonne de rémunération, de NIR, d'IBAN de salarié en dehors de la brique `paie_rh`. |
| R-RH8 | NIR et IBAN chiffrés (E11), masqués à l'affichage, consultation journalisée ; transmis seulement au logiciel de paie. |
| R-RH9 | Changement d'IBAN d'un salarié : demande par le salarié (passkey) ou pièce, validation par le gestionnaire de paie, avis au salarié par un canal déjà connu (anti-fraude). |
| R-RH10 | Pièces exigées selon la nature du contrat, jamais au-delà. |
| R-RH11 | Contrat et avenant signés : immuables ; une correction = un nouvel avenant. |
| R-RH12 | Bulletin électronique sauf opposition ; opposition réversible. |
| R-RH13 | Bulletins accessibles au salarié pendant la durée légale, y compris après son départ (Q-RH9). |
| R-RH14 | Une variable issue d'un fait est idempotente par clé de fait et se recalcule tant que la période n'est pas arrêtée. |
| R-RH15 | Après arrêté : régularisation sur la période suivante, jamais de réécriture. |
| R-RH16 | Pas de prime ni d'indemnité sans décision tracée (qui, quand, motif). |
| R-RH17 | Une variable ne se supprime pas, elle s'annule (motif). |
| R-RH18 | L'export n'a lieu qu'après arrêté ; un export ne se refait qu'après annulation tracée ; le fichier est conservé. |
| R-RH19 | Le retour de paie se rapproche par matricule, jamais par nom ; un bulletin non rapproché reste en quarantaine. |
| R-RH20 | Arrêt de travail validé → tâche « signalement DSN » avec échéance J+5 de la connaissance, au gestionnaire de paie. |
| R-RH21 | Les échéances légales sont des tâches attribuées, visibles avant leur date. |
| R-RH22 | Personne n'arrête ses propres variables ni ne valide sa propre prime ; le directeur salarié relève du président ou de son délégué. |
| R-RH23 | Un responsable d'équipe voit la présence et les compteurs de son équipe, jamais les rémunérations ni les natures d'absence. |
| R-RH24 | Un collègue ne voit rien du dossier RH d'un autre. |
| R-RH25 | Rapports d'absentéisme agrégés, sans nature médicale pour un groupe de moins de 5 personnes. |
| R-RH26 | Aucune liste de grévistes diffusée au-delà du gestionnaire de paie. |
| R-RH27 | Pas de note interne cachée au salarié dans son dossier. |
| R-RH28 | Les modèles de rupture et de sanction exigent la mention de l'avis juridique obtenu (case, date) avant génération. |
| R-RH29 | Les sanctions du droit du travail ne sont jamais portées au socle 13 (élèves) ni à la fiche du 6. |
| R-RH30 | Accès d'un tiers (cabinet de paie, expert-comptable) : temporaire, à l'espace d'échange seulement, journalisé. |
| R-RH31 | Montants en centimes ; quantités en décimal à 3 chiffres ; dates en Europe/Paris. |
| R-RH32 | Seuils réglementaires (effectifs, durées, plafonds) = paramètres de plateforme datés et sourcés. |
| R-RH33 | Une casquette parent ne voit jamais le dossier RH de la même personne, et inversement. |
| R-RH34 | Aucune donnée RH nominative dans les journaux applicatifs (E14). |
| R-RH35 | Mode parallèle pendant le doublon : exports « à blanc » marqués, jamais remis comme définitifs sans décision. |
| R-RH36 | L'IA ne lit aucun avis d'aptitude, aucune pièce médicale, aucune rémunération hors droit de l'utilisateur. |
| R-RH37 | Une pièce RH exigée par le droit du travail peut être demandée même si elle serait interdite aux familles (D-027 H). |
| R-RH38 | Candidatures non retenues purgées après la durée réglée. |
| R-RH39 | Le registre unique du personnel est dérivé des contrats, jamais saisi. |
| R-RH40 | Fichiers (exports, bulletins, contrats) en objets immuables (E5). |

---

## 22. Droits et minimisation

### 22.1 Catalogue (D-017)
| Droit | Contenu | Rôles par défaut |
|---|---|---|
| `rh.parametrer` | employeur, CCN, rubriques, adaptateur, modèles | Direction, Gestionnaire de paie |
| `rh.recruter` | postes, candidats | Direction, Secrétariat RH |
| `rh.dossier.lire` / `rh.dossier.gerer` | dossiers / pièces, échéances, attestations | Direction / Secrétariat RH |
| `rh.contrat.preparer` / `rh.contrat.signer` ★ | modèles / signature, ruptures | Secrétariat RH / Direction (délégation du président) |
| `rh.disciplinaire` ★ | sanctions du droit du travail | Direction |
| `rh.temps.gerer` / `rh.temps.valider` | temps, compteurs / heures | Gestionnaire de paie / Direction |
| `rh.echeances.lire` | tableau des échéances | Direction, Secrétariat RH |
| `rh.sst.gerer` | visites, avis, AT, DUERP | Direction, Secrétariat RH |
| `rh.entretiens.gerer`, `rh.formation.gerer` | entretiens, formation | Direction ; responsables (leurs équipes) |
| `rh.registre.lire` | registre unique | Direction |
| `rh.rapports` | rapports agrégés | Direction |
| `paie.remunerations.gerer` / `paie.lire_remunerations` | rémunération de référence / comparaison | Gestionnaire de paie / Direction |
| `paie.variables.gerer` / `paie.variables.decider` | variables / primes | Gestionnaire de paie / Direction |
| `paie.arreter` ★ | arrêté | Direction |
| `paie.exporter` ★, `paie.retour.importer` | exports / retours | Gestionnaire de paie |
| `paie.bulletins.lire` | bulletin individuel (motif) | Direction |
| `paie.iban.valider` ★ | IBAN salarié | Gestionnaire de paie |
| `paie.payer` | fichier des nets via le 48 | Gestionnaire de paie |
| `paie.lire_agrege` | masse salariale, coûts agrégés | Compta (48), Direction |
| `rh.etats.exporter` | états au mutualisateur ou au rectorat | Gestionnaire (public) |

**Aucun droit RH ni paie** par défaut pour les rôles Administratif, Vie scolaire, Enseignant ; le CPE responsable des surveillants reçoit, comme responsable d'équipe, la vue présence et compteurs de son équipe (R-RH23).

### 22.2 Pièges à tester
Bulletin d'un autre salarié par identifiant ; NIR par l'API ; variables d'un salarié d'un autre établissement ; arrêté de ses propres variables ; nature d'absence vue par un responsable d'équipe ; dossier RH vu depuis la casquette parent ; accès du cabinet après sa date de fin ; IA demandant un avis d'aptitude ; export contenant un agent de l'État.

### 22.3 Ce que voit chacun
Le **salarié** : tout ce qui le concerne. Le **responsable d'équipe** : présence, compteurs, planning, entretiens de son équipe ; pas de rémunération ni de nature. La **direction** : dossiers, échéances, variables, arrêté ; bulletins individuels sur motif journalisé. Le **gestionnaire de paie** : variables, rémunérations, exports, retours ; pas les entretiens ni les pièces disciplinaires. Le **cabinet** : l'espace d'échange.

### 22.4 Conservation [R, à aligner sur le référentiel CNIL RH du 2-4-2026 et à valider par le DPO]
| Donnée | Durée proposée | Fondement |
|---|---|---|
| Bulletins (double employeur) | 5 ans | L3243-4 [V-sec] |
| Bulletins électroniques disponibles au salarié | 50 ans ou 6 ans après la retraite | D3243-8 [V-sec] |
| Éléments de cotisations, variables, exports, retours | 6 ans | repère CNIL et 15 R-15 [V-sec] |
| Registre unique du personnel | 5 ans après le départ | R1221-26 [V-sec] |
| Contrats, avenants, dossier | durée du contrat + 5 ans (prescription) | repère CNIL [V-sec] |
| Décompte du temps de travail | 1 an (3 ans pour l'action en paiement) | 15 R-15 [V-sec] |
| DUERP | 40 ans | RR-13 [NV] |
| Avis d'aptitude (côté employeur) | durée du contrat + 5 ans [NV] | |
| Candidatures non retenues | 2 ans au plus | repère CNIL [V-sec] |
| Journaux de consultation | 1 an | CNIL journalisation |

---

## 23. Événements et notifications

**Émis** : `rh.contrat_signe|avenant_signe|contrat_clos`, `rh.echeance_proche`, `rh.visite_avis_enregistre` (sans contenu), `paie.periode_arretee|exportee|close`, `paie.bulletin_disponible`, `paie.journal_arrete` (→ 48), `rh.signalement_dsn_a_faire`.
**Consommés** : `absence.validee|modifiee|requalifiee|reouverte` (15), `heure.constatee|validee` (15), `personnel.service_fait_arrete` (6), `mission.remuneration_modifiee` (6), `personnel.engagement_clos` (6), `fonction.modifiee` (6).
**Notifications** : contrat à signer (salarié), pièces manquantes (futur salarié), variables du mois à vérifier (salarié, avant l'arrêté), bulletin disponible (salarié), échéance (direction, secrétariat), signalement DSN (gestionnaire), entretien à planifier (direction) ; **aucun montant** dans une notification ni sur un écran verrouillé.

---

## 24. Écrans

- **Salarié** (web et **mobile**) : « Mon espace » — bulletins, compteurs, heures du mois, demandes (congés au 15, formation, attestation), contrat et avenants, entretiens, formations, pièces ; contester une variable.
- **Gestionnaire de paie** : file du mois (anomalies, contestations, signalements DSN), tableau dense des variables par salarié et rubrique, export, retour.
- **Direction** : tuiles (échéances de la semaine, arrêté du mois, effectifs, signalements) ; arrêté sur mobile avec le résumé des anomalies ; dossiers.
- **Secrétariat RH** : embauches en cours (pièces, contrat, DPAE), échéances.
- **Responsable d'équipe** : présence, congés à valider, compteurs, entretiens de son équipe.

---

## 25. Modèle de données cible (brique `paie_rh`) [R]

Mêmes règles que le 48 (E1–E14, aucune colonne JSON ni tableau, montants en centimes, FK composites, RLS stricte).

```
ent_rh_employeur (id, etablissement_id, organisme_id NULL (4), raison_sociale, siret, idcc TEXT, logiciel_paie CHECK IN ('silae','sage','ebp','charlemagne','cabinet','autre'), adaptateur_id NULL, gestionnaire_etablissement BOOLEAN)
ent_rh_salarie (id, etablissement_id, agent_id (6), employeur_id, matricule TEXT, nir_chiffre, nir_cle_id, iban_chiffre NULL, iban_cle_id NULL, iban_masque, opposition_bulletin_electronique BOOLEAN, opposition_le NULL)
               UNIQUE (etablissement_id, employeur_id, matricule)
ent_rh_salarie_iban_demande (id, etablissement_id, salarie_id, iban_chiffre, iban_cle_id, piece_objet_id NULL, demande_par, etat CHECK IN ('en_attente','valide','refuse'), valide_par NULL, valide_le NULL)
ent_rh_ccn_categorie (id, etablissement_id NULL -- NULL = référentiel plateforme (globale_reference), idcc, code, libelle, niveau, du, au NULL)
ent_rh_contrat (id, etablissement_id, salarie_id, engagement_id (6), type CHECK IN ('cdi','cdd','apprentissage','professionnalisation','aide','vacation','stage_gratifie'),
                motif_cdd CHECK IN (...) NULL, remplace_salarie_id NULL, du, au NULL, terme_imprecis BOOLEAN, quotite_pct NUMERIC(5,2), duree_hebdo NUMERIC(5,2) NULL, annualise BOOLEAN,
                categorie_ccn_id, anciennete_du DATE, essai_fin DATE NULL, essai_renouvele_fin DATE NULL, document_objet_id, signe_salarie_le, signe_employeur_le, signe_par,
                etat CHECK IN ('prepare','signe','en_cours','rompu','termine','annule'), fin_motif CHECK IN (...) NULL, fin_le NULL)
ent_rh_avenant (id, etablissement_id, contrat_id, effet_le, objet CHECK IN ('quotite','poste','classification','remuneration','duree','autre'), document_objet_id, signe_le)
ent_rh_remuneration (id, etablissement_id, contrat_id, du, au NULL, salaire_base_mensuel_centimes, taux_horaire_centimes NULL, decide_par)
ent_rh_dpae (id, etablissement_id, contrat_id, numero_accuse, faite_le, attestee_par)
ent_rh_piece_attendue (id, etablissement_id, type_contrat, nature_piece, obligatoire BOOLEAN)
ent_rh_dossier_piece (id, etablissement_id, salarie_id, nature_piece, objet_id, valide_du NULL, expire_le NULL, deposee_par, deposee_le, retiree_le NULL)
ent_rh_rubrique (id, etablissement_id NULL (référentiel ScolIA), code, libelle, unite CHECK IN ('heure','jour','demi_journee','montant','nombre'), sens CHECK IN ('gain','retenue','information'), source CHECK IN ('absence','heure','mission','prime','retenue','entree_sortie','avantage','autre'))
ent_rh_adaptateur (id, etablissement_id, logiciel, version_format, separateur, encodage, en_tete BOOLEAN)
ent_rh_rubrique_editeur (etablissement_id, adaptateur_id, rubrique_id, code_editeur TEXT, type_editeur CHECK IN ('constante','rubrique','evenement','variable'))
ent_rh_periode (id, etablissement_id, employeur_id, mois DATE, etat CHECK IN ('ouverte','arretee','exportee','retour_recu','close'), arretee_le, arretee_par, parallele BOOLEAN)
ent_rh_variable (id, etablissement_id, periode_id, salarie_id, rubrique_id, quantite NUMERIC(10,3) NULL, montant_centimes BIGINT NULL, du DATE NULL, au DATE NULL,
                 source_type TEXT, source_id UUID NULL, source_version INT NULL, decision_id NULL, regularisation_de_id NULL,
                 etat CHECK IN ('brouillon','contestee','validee','annulee'), annule_motif NULL)
                 UNIQUE (etablissement_id, source_type, source_id, source_version, rubrique_id) WHERE source_id IS NOT NULL
ent_rh_decision (id, etablissement_id, salarie_id, nature CHECK IN ('prime','indemnite','heures','autre'), montant_centimes NULL, quantite NULL, motif, decide_par, decide_le)
ent_rh_contestation (id, etablissement_id, variable_id NULL, periode_id, salarie_id, texte, etat, traitee_par NULL)
ent_rh_export (id, etablissement_id, periode_id, adaptateur_id, numero, fichier_objet_id, empreinte, nb_lignes, parallele BOOLEAN, annule_le NULL, annule_motif NULL)
ent_rh_retour (id, etablissement_id, periode_id, nature CHECK IN ('bulletins','journal','compteurs','couts','dsn_attestation','nets'), fichier_objet_id, empreinte, importe_par, anomalies INT)
ent_rh_bulletin (id, etablissement_id, retour_id, salarie_id NULL, matricule_lu TEXT, mois, objet_id, etat CHECK IN ('rapproche','quarantaine','depose','papier'), depose_le NULL)
ent_rh_journal_ligne (id, etablissement_id, retour_id, compte TEXT, section_id NULL (48), debit_centimes, credit_centimes) -- agrégé, jamais nominatif
ent_rh_cout_agrege (etablissement_id, periode_id, section_id, categorie_ccn_id NULL, cout_centimes, effectif NUMERIC(8,2))
ent_rh_dsn_depot (id, etablissement_id, periode_id, depose_le, statut CHECK IN ('ok','anomalies','rejet'), nb_anomalies, atteste_par)
ent_rh_compteur_type (id, etablissement_id, code CHECK IN ('cp','repos','heures_annualisees','cet'), libelle, unite)
ent_rh_compteur_mvt (id, etablissement_id, salarie_id, type_id, date, quantite NUMERIC(8,3), origine CHECK IN ('acquisition','prise','correction','recalage_paie','reprise'), source_id NULL, motif NULL, par)
ent_rh_echeance (id, etablissement_id, salarie_id NULL, nature CHECK IN ('fin_essai','fin_cdd','visite','entretien_parcours','bilan_8_ans','titre_sejour','formation_obligatoire','dpae','signalement_dsn','dsn_mensuelle','autre'),
                 echeance DATE, responsable_agent_id NULL, source_type, source_id, faite_le NULL, faite_par NULL)
ent_rh_visite (id, etablissement_id, salarie_id, type CHECK IN ('information_prevention','periodique','reprise','pre_reprise','embauche_renforcee','demande'), date, avis CHECK IN ('apte','apte_amenagements','inapte','sans_avis') NULL, amenagement_fonctionnel TEXT NULL, objet_id NULL)
ent_rh_accident (id, etablissement_id, salarie_id, survenu_le, lieu TEXT, declaration_faite_le NULL, signalement_dsn_le NULL) -- aucune description médicale
ent_rh_duerp_version (id, etablissement_id, version, objet_id, adopte_le) ; ent_rh_prevention_action (id, etablissement_id, duerp_version_id, libelle, responsable, echeance, faite_le)
ent_rh_entretien (id, etablissement_id, salarie_id, nature CHECK IN ('parcours','bilan_8_ans','annuel','retour_absence'), prevu_le, realise_le NULL, compte_rendu_objet_id NULL, signe_salarie_le, signe_employeur_le)
ent_rh_formation (id, etablissement_id, intitule, organisme, obligatoire BOOLEAN, qualification_code NULL (6), duree_heures, financeur CHECK IN ('opco','employeur','cpf','autre'))
ent_rh_formation_inscription (id, etablissement_id, formation_id, salarie_id, du, au, etat, attestation_objet_id NULL, validite_jusqu_au NULL)
ent_rh_sanction (id, etablissement_id, salarie_id, nature CHECK IN ('avertissement','mise_a_pied','autre'), notifiee_le, document_objet_id, avis_juridique_le) -- accès rh.disciplinaire
ent_rh_candidat (id, etablissement_id, poste_id, nom, email, cv_objet_id NULL, decision NULL, purger_le DATE)
ent_rh_poste (id, etablissement_id, intitule, categorie_ccn_id, type_contrat, ouvert_le, pourvu_le NULL)
ent_rh_acces_tiers (id, etablissement_id, user_id, organisme, du, au, accorde_par)
```

**Registre (E12)** : portée `etablissement` ; export **particulier** (données RH sensibles, export complet sous double validation D-041, NIR et IBAN exclus de la vue métier CSV) ; `ent_rh_ccn_categorie` et `ent_rh_rubrique` à `etablissement_id NULL` en `globale_reference` ; envois en file (liens d'embauche, dépôts de bulletins, tâches de signalement) **neutralisés à la restauration** (E9). **Clés** (E11) : `nir_chiffre`, `iban_chiffre` avec identifiant de clé, à l'inventaire. **Exceptions D-036 : aucune.**

---

## 26. Reprise des données

- **Next.js** : `paie_periode`, `paie_element` : **ébauche, perdable** (T0) ; **rien n'est repris**, sauf si Florian signale des éléments réels (Q-RH4) ; identifiants `paie-demo-*` ignorés (HD-02) ; libellés jamais repris (HD-01). HSE (M08, collections `demandes-hse__*`) de l'année en cours : reprises en **heures validées** avec leur décision (RHP-252). RH M06 : rien, sauf pièces désignées par Florian (T0).
- **Logiciel de paie actuel** : salariés, matricules, contrats en cours, dates d'ancienneté, compteurs de congés à date (RHP-250, RHP-251), **NIR et IBAN importés en attente** de confirmation par le salarié ou le gestionnaire.
- Tests sur données **fictives** uniquement.

---

## 27. Lots et migrations

### 27.1 Lots

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **RH0** garde-fous | test de schéma « aucune colonne de rémunération, NIR, IBAN de salarié hors `ent_rh_*` » ; préalable : nature d'absence codée (15 A0, HD-04) ; droits RH et paie vides par défaut ; signalement HD-01 à HD-03 à Florian | 15 A0 | test CI vert |
| **RH1** employeur, salariés, dossier | employeur, salarié (NIR, IBAN chiffrés), pièces attendues, dossier, espace salarié (lecture), IBAN avec validation | 6 P1 ; 7 | pièges §22.2 |
| **RH2** contrats et sorties | modèles, contrats, avenants, DPAE attestée, essai, CDD, titres de séjour, registre unique, ruptures (modèles), tâches de sortie, recrutement minimal, dossier d'embauche par lien | RH1 ; 7 (signature) | contrat signé immuable ; registre dérivé |
| **RH3** temps et congés | durée, annualisation, compteurs, congés via le 15, recalage | RH2 ; 15 | compteurs justes sur jeu fictif |
| **RH4** variables et arrêté | rubriques, collecte depuis 15 et 6, décisions, contrôles, contestation, arrêté, régularisations | RH3 ; 15 ; 6 (service fait) | idempotence ; régularisation ; R-RH22 |
| **RH5** échange avec le logiciel de paie | format pivot, adaptateur (un éditeur, Q-RH2), export, retour (bulletins, journal → 48, compteurs, coûts, DSN attestée), coffres, mode parallèle | RH4 ; 48 CG1 | un mois fictif importé sans retouche chez l'éditeur |
| **RH6** échéances | tableau, tâches, signalement DSN J+5 | RH2, RH4 | |
| **RH7** santé et sécurité au travail | visites, avis, AT, DUERP, formations obligatoires, EPI | RH1 ; 6 ; 44 | aucune donnée médicale (test) |
| **RH8** entretiens et formation | parcours 4 ans, bilan 8 ans, plan, OPCO | RH2 ; 19 | échéances calculées |
| **RH9** collectif et rapports | effectifs, seuils, CSE, catégories, rapports agrégés, masse salariale (→ 48) | RH5 | agrégats sans fuite (R-RH25) |
| **RH10** public | AED/AESH : contrats, compteur 1 607 h, états au mutualisateur | RH4 | |
| **RH11** contrôle brut → net (option) | vérification simplifiée des bulletins, sans valeur légale | **Q-RH1** | non engagé |
| **RH12** moteur de paie et DSN (option) | §3.4 | **Q-RH1** | non engagé |

**Parcours minimal pour la rentrée 2027** (la RH est une fonction de l'add-on, D-035) : RH0, RH1, RH2, RH3, RH4, RH5 (en mode parallèle ou réel selon Q-RH3), RH6.

### 27.2 Migrations (plage imposée : **2980–2999**)

| N° | Contenu | Lot |
|---|---|---|
| 2980 | RH0 : aucune table ; réservée (garde de schéma en test, pas en migration) | RH0 |
| 2981 | employeurs, salariés, demandes d'IBAN, référentiel CCN | RH1 |
| 2982 | pièces attendues, pièces du dossier | RH1 |
| 2983 | postes, candidats | RH2 |
| 2984 | contrats, avenants, rémunérations, DPAE | RH2 |
| 2985 | compteurs (types, mouvements) | RH3 |
| 2986 | rubriques, adaptateurs, correspondances | RH4, RH5 |
| 2987 | périodes, variables, décisions, contestations | RH4 |
| 2988 | exports, retours, bulletins, journal agrégé, coûts, dépôts DSN | RH5 |
| 2989 | échéances | RH6 |
| 2990 | visites, accidents, DUERP, actions de prévention | RH7 |
| 2991 | entretiens, formations, inscriptions | RH8 |
| 2992 | sanctions, accès tiers | RH2, RH5 |
| 2993 | public : compteurs AED/AESH, états | RH10 |
| 2994 | RLS stricte des tables `ent_rh_*`, registre E12 | RH1 → RH10 |
| 2995–2998 | réserve (RH11, RH12, ajouts personas) | |
| 2999 | retrait éventuel (sur décision, E8) | — |

---

## 28. Tests et recette

- **Propriétés** : une variable par fait et par rubrique ; une période arrêtée ne change pas ; une régularisation porte sur la période suivante ; aucune variable pour un agent de l'État ; aucune nature d'absence en texte libre.
- **Confidentialité** : matrice des pièges §22.2 ; isolation entre établissements ; casquettes.
- **Restauration** : exports et retours retrouvés par empreinte ; liens et dépôts neutralisés.
- **Recette** avec le gestionnaire de paie de l'OGEC pilote : trois mois fictifs importés dans son logiciel ; bulletins fictifs rapprochés ; journal fictif équilibré au 48.
- **Jeu IA** : contrôle des variables sur un mois fictif avec 20 anomalies plantées.

---

## 29. Réglages signalés — S-4900 à S-4949

| N° | Réglage | Défaut |
|---|---|---|
| S-4900 | Logiciel de paie | à choisir (Q-RH2) |
| S-4901 | Gestion de la paie | externe (option 2) |
| S-4902 | Date d'arrêté mensuel | le 20 |
| S-4903 | Visibilité des heures du mois par le salarié | oui, dès la constatation |
| S-4904 | Seuil d'écart signalé au contrôle | 20 % |
| S-4905 | Échéance DSN mensuelle | 5 ou 15 selon l'effectif [V-sec] |
| S-4910 | Accès à l'ENT retenu tant que la DPAE n'est pas attestée | oui |
| S-4911 | Alerte fin d'essai / fin de CDD | J-15 / J-30 et J-15 |
| S-4912 | Périodicité de l'entretien de parcours | 4 ans (accord : moins) |
| S-4913 | Alerte titre de séjour | J-90 et J-30 |
| S-4914 | Saisies sur salaire dans ScolIA | non |
| S-4915 | Conservation des candidatures non retenues | 2 ans |
| S-4916 | Employeurs paramétrés | l'OGEC seulement |
| S-4920 | Renouvellement du contrôle d'honorabilité | désactivé [NV] |

---

## 30. Décisions appliquées par défaut (D-031) et questions à Florian

### 30.1 Appliquées par défaut
| # | Question | Décision |
|---|---|---|
| Q-RH12 | Le 6 porte-t-il des données de paie ? | Non (R-RH7), confirmé. |
| Q-RH13 | Nature d'absence | Codée, jamais libre (R-RH6). |
| Q-RH14 | Arrêté par qui ? | La direction ; jamais ses propres variables (R-RH22). |
| Q-RH15 | Après arrêté | Régularisation (R-RH15). |
| Q-RH16 | Bulletins | Coffre du salarié, sauf opposition. |
| Q-RH17 | Formation des enseignants agents de l'État | Hors 49. |
| Q-RH18 | Public | États au payeur, aucune paie locale. |
| Q-RH19 | IA | Contrôle et brouillons, jamais de santé ni de décision. |

### 30.2 Soumis à Florian
| # | Question | Recommandation |
|---|---|---|
| **Q-RH1** (**close par D-047**, §34) | **Moteur de paie et DSN dans ScolIA** (option 1, D-025 littéral) ou **préparation et interface** (option 2) ? **Contredit D-025.** | **Option 2** (§3.3) ; RH11–RH12 en option |
| Q-RH2 | Logiciel de paie de l'OGEC pilote, ou cabinet ? Format d'import, coût | à demander ; un adaptateur pour commencer |
| Q-RH3 | Pendant le doublon, RH et préparation de paie en **réel** ? | oui pour le dossier, les contrats, les échéances ; variables en parallèle trois mois puis réel |
| Q-RH4 | Des éléments de paie **réels** sont-ils dans Next.js ? Purge du libellé (HD-01) côté Next.js ? | décision et geste de Florian |
| Q-RH5 | **Juriste en droit social** pour les modèles (contrats, ruptures, sanctions) et la CCN | indispensable avant RH2 |
| Q-RH6 | OPCO et service de santé au travail de l'OGEC | à confirmer |
| Q-RH7 | Signature électronique des contrats (niveau, prestataire) | socle 7 |
| Q-RH8 | Maîtres agréés du premier degré sous contrat simple payés par l'OGEC ? | à vérifier si le premier degré entre (socle 50) |
| Q-RH9 | Accès aux bulletins après le départ (lien nominatif ou archive remise) | les deux |
| Q-RH10 | Paiement des nets par le 48 ou par le logiciel de paie | logiciel de paie |
| Q-RH11 | Public : format des états pour le mutualisateur | après D-029 |
| Q-RH20 | Durées de conservation (§22.4) | DPO |

---

## 31. Ajout personas (2026-11-11, `PERSONAS_49.md`) — l'emporte en cas de contradiction

### 31.1 Régimes pendant l'année de doublon (bloquant D)

| Fonction | Régime 2027-2028 |
|---|---|
| Dossier du salarié, contrats, avenants, registre, échéances, santé et sécurité au travail, entretiens, formation | **réel** dès la rentrée (remplace l'ébauche RH de Next.js, D-035) |
| Temps et congés | **réel** (compteurs recalés chaque mois sur le retour de paie) |
| Variables de paie | **parallèle** de septembre à novembre 2027 avec comparaison outillée (RHP-264), puis **réel** si les écarts sont expliqués |
| Calcul, bulletins, DSN | logiciel de paie (option 2) |

### 31.2 Règles ajoutées (R-RH41 à R-RH50)

| N° | Règle |
|---|---|
| R-RH41 | Rémunération de référence et variables de la gestionnaire de paie et du chef d'établissement : gérées par le cabinet, ou saisies et validées par le président ; jamais par l'intéressé. |
| R-RH42 | Bulletin papier par défaut tant que le salarié n'a pas d'accès actif ; réversible. |
| R-RH43 | Aucun modèle de rupture ni de sanction généré pendant une période protégée (maternité et suites, AT/MP, mandat de représentant et période postérieure) sans mention d'un avis juridique et sans confirmation renforcée ; la période est calculée depuis les faits. |
| R-RH44 | Un engagement OGEC d'un agent de l'État exige la pièce d'autorisation de cumul et suit son échéance. |
| R-RH45 | Une période ne s'arrête pas tant qu'une entrée du mois n'a pas sa DPAE attestée. |
| R-RH46 | Le dossier RH d'une personne ne montre jamais sa scolarité passée, et inversement. |
| R-RH47 | Saisie « pour le compte de » un salarié par le secrétariat : permise, tracée, visible du salarié. |
| R-RH48 | Une absence d'un enseignant agent de l'État ne produit, côté OGEC, que des heures non dues (séances non assurées), jamais une nature d'absence, sauf arrêt de travail transmis à l'OGEC. |
| R-RH49 | Une signature de contrat ou de rupture par le chef d'établissement exige une délégation de pouvoir en cours. |
| R-RH50 | Les heures de nuit, de veille et d'astreinte de l'internat viennent du socle 41, jamais d'une saisie libre. |

### 31.3 Actions ajoutées (RHP-263 à RHP-279)

| N° | Action | Acteur · droit | Lot |
|---|---|---|---|
| RHP-263 | Revue trimestrielle des consultations de rémunérations et de bulletins ★ | PRES | RH1 |
| RHP-264 | **Comparer** l'export ScolIA avec l'état de variables du logiciel de paie (parallèle) | GP | RH5 |
| RHP-265 | Découper un PDF de bulletins multipages par matricule | SYS, GP | RH5 |
| RHP-266 | Attestation trimestrielle santé et sécurité au travail pour le président | DIR → PRES | RH7 |
| RHP-267 | Générer un CDD de remplacement depuis le contrat du remplacé | SEC | RH2 |
| RHP-268 | Contrôler le planning d'un apprenti mineur (horaires, tâches interdites) | SYS → RESP | RH3 |
| RHP-269 | Communiquer à un salarié le journal des consultations de son dossier | DIR | RH1 |
| RHP-270 | Enregistrer une **clé de sécurité** remise à un salarié (procédure du socle 1) | SEC | RH1 |
| RHP-271 | Saisir une demande **pour le compte de** un salarié | SEC | RH3 |
| RHP-272 | Calculer les **périodes de protection** | SYS | RH6 |
| RHP-273 | Exiger et suivre l'**autorisation de cumul** d'un agent public | SEC | RH2 |
| RHP-274 | Enregistrer une **délégation de pouvoir** (pièce, dates) | PRES | RH1 |
| RHP-275 | Déléguer une échéance à une autre personne | DIR | RH6 |
| RHP-276 | Bulletin expliqué en phrases simples (lecture possible à voix haute) | SAL | RH5 |
| RHP-277 | Variable « temps partiel thérapeutique » et tâche de signalement de reprise | SYS, GP | RH4 |
| RHP-278 | Heures de nuit et d'astreinte depuis l'internat (41) | SYS | RH4 |
| RHP-279 | Export à **usage unique** avec expiration ; NIR et IBAN seulement si l'adaptateur les exige | GP | RH5 |

### 31.4 Cas limites ajoutés (CRH-46 à CRH-60)

| # | Cas | Traitement |
|---|---|---|
| CRH-46 | La gestionnaire de paie demande des heures supplémentaires pour elle | validées et arrêtées par la direction ; ses variables relèvent de R-RH41 |
| CRH-47 | Salarié sans smartphone ni clé | bulletin papier, borne, saisie pour son compte |
| CRH-48 | Licenciement préparé pendant la grossesse annoncée | modèle bloqué (R-RH43) |
| CRH-49 | Élue du CSE absente pour son mandat | « absente » pour l'équipe ; rubrique « heures de délégation » au gestionnaire |
| CRH-50 | Professeur (État) sans autorisation de cumul au 1er septembre | engagement OGEC créé, variables bloquées jusqu'à la pièce ; tâche à la direction |
| CRH-51 | Bulletins du cabinet en un seul PDF dont une page est illisible | page en quarantaine ; demande au cabinet |
| CRH-52 | Ancienne élève devenue surveillante | dossiers séparés (R-RH46) |
| CRH-53 | Apprenti mineur planifié au service du soir jusqu'à 22 h 30 | alerte au responsable, planning à corriger [NV sur l'horaire limite] |
| CRH-54 | Mi-temps thérapeutique | variable dédiée, visite de reprise, tâche DSN |
| CRH-55 | AESH partagé entre trois établissements d'un PIAL | chaque établissement exporte ses seules heures |
| CRH-56 | Parallèle : écart de 3 heures entre ScolIA et le logiciel de paie | ligne d'écart, explication attendue (fait manquant, saisie directe dans le logiciel) |
| CRH-57 | Le président refuse de valider les variables du directeur | elles restent en attente ; la période peut être arrêtée **sans** elles (report) |
| CRH-58 | Ex-salarié demande un bulletin de 2027 en 2040 | compte allégé « mes bulletins » ou archive remise au départ |
| CRH-59 | Délégation de pouvoir expirée au moment de signer un CDD | signature bloquée ; le président signe |
| CRH-60 | Fichier d'export retéléchargé une seconde fois | refusé ; nouvel export après annulation tracée |

### 31.5 Réglages ajoutés (S-4921 à S-4928)

| N° | Réglage | Défaut |
|---|---|---|
| S-4921 | Droit de la gestionnaire sur les rémunérations quand un cabinet fait la paie | non |
| S-4922 | Borne ou poste partagé de consultation | désactivé ; activable |
| S-4923 | Affichage d'une absence pour mandat à l'équipe | « absente » |
| S-4924 | Carence après changement d'IBAN d'un salarié | jusqu'à la paie suivante si le changement a lieu après l'arrêté |
| S-4925 | NIR et IBAN dans l'export | seulement si l'adaptateur les exige |
| S-4926 | Durée de validité du lien de téléchargement d'un export | 24 h, usage unique |
| S-4927 | Durée du parallèle des variables | 3 mois |
| S-4928 | Revue des consultations par le président | trimestrielle |

### 31.6 Lots et migrations révisés
- **RH1** intègre R-RH41, les clés de sécurité, la délégation de pouvoir, la revue des consultations et l'empreinte HMAC des IBAN des salariés (48 R-CG47).
- **RH2** intègre l'autorisation de cumul, le CDD de remplacement, le blocage en période protégée (avec **RH6** pour le calcul des périodes ; dépend du port des mandats du socle 46, sinon saisie manuelle de la période).
- **RH4** intègre les rubriques d'internat et de temps partiel thérapeutique (dépend du 41 et du 15).
- **RH5** intègre la comparaison du parallèle, la découpe des bulletins, l'export à usage unique.
- Migrations : **2995** reçoit `ent_rh_periode_protection`, `ent_rh_autorisation_cumul`, `ent_rh_delegation_pouvoir`, `ent_rh_cle_securite_remise`, `ent_rh_iban_empreinte` ; **aucune colonne JSON**.

---

## 32. Faits, hypothèses, contradictions

**Faits [F]** : `352_agents.sql` (natures, employeurs), `355_missions.sql`, `080_absence_rh.sql` ; absence de toute table de paie dans les 231 migrations ; Next.js `paie-rh-spec.ts`, `paie-etablissement-db.ts` (l. 27-36, 60-83, 123-130, 180-205, 228-247), `app/api/paie/route.ts` lus le 2026-11-11.
**Hypothèses [H]** : effectifs d'un OGEC (10 à 60), externalisation fréquente de la paie, date d'arrêté, délégation de l'employeur au chef d'établissement.
**Contradictions** : **D-025** (paie construite dans l'ENT) contre la recommandation §3 (Q-RH1) ; le socle 15 (§437) renvoie le « décompte des congés au socle 49 » alors que la demande de congé reste au 15 : cohérent (le 15 possède la demande, le 49 le compteur) ; `ent-metier-checklist.md` classe « Paie » en « DROP » (déjà signalé).

---

## 33. Sources (consultées le 2026-11-11)

Recherche par moteur ; Légifrance renvoie 403 aux accès automatiques.
- CCN EPNL IDCC 3218 : https://www.legisocial.fr/conventions-collectives-nationales/3218-enseignement-prive-non-lucratif/ ; https://www.editions-tissot.fr/acheter-convention-collective/ccn-enseignement-prive-epnl ; https://presse.tripalio.fr/wp-content/uploads/2022/08/IDCC-3218-Avenant-nouvelle-CCN.pdf ; OPCO : https://opcodirect.fr/convention/3218
- Maîtres du privé : https://www.assemblee-nationale.fr/dyn/17/questions/QANR5L17QE14679.pdf ; https://blog.landot-avocats.net/2022/10/18/les-maitres-agrees-du-prive-sous-contrat-simple-avec-letat-se-voient-bien-appliquer-le-regime-de-rupture-conventionnelle-du-droit-prive-et-non-son-equivalent-public/amp/
- DSN, NEODeS, éditeurs : https://www.net-entreprises.fr/declaration/norme-et-documentation-dsn ; https://www.net-entreprises.fr/dsn-les-editeurs-sengagent-sur-la-qualite-et-la-completude-de-leurs-logiciels/ ; https://www.net-entreprises.fr/media/documentation/charte-editeurs-revue-forme.pdf ; https://www.fiche-paie.net/page/dsn-arret-de-travail-et-maladie
- Bulletin de paie : https://www.service-public.gouv.fr/particuliers/vosdroits/F559 ; https://www.swim.legal/blog/feuille-de-paie-mentions-obligatoires
- Entretien de parcours professionnel : https://formation.lefebvre-dalloz.fr/actualite/loi-seniors-ce-qui-change-en-matiere-de-formation-professionnelle ; https://www.akto.fr/lentretien-professionnel-evolue-pour-devenir-lentretien-de-parcours-professionnel/ ; https://www.editions-tissot.fr/actualite/representants-du-personnel-ce/entretiens-de-parcours-professionnel-les-accords-non-conformes-ne-sappliquent-plus-depuis-le-1er-octobre-2026
- Référentiel CNIL RH : https://cnil.fr/fr/referentiel-durees-conservation-donnees-rh ; https://www.cnil.fr/sites/default/files/2026-04/referentiel_durees_de_conservation_gestion_des_ressources_humaines.pdf (non lu en entier)
- Transparence salariale : https://www.info.gouv.fr/actualite/transparence-des-salaires-regles-et-evolutions-prevues ; https://haut-conseil-egalite.gouv.fr/sites/hce/files/2026-09/AVIS%20HCE%20transposition%20Directive%20UE%20transparence%20des%20salaires%20VF.pdf
- Taxe sur les salaires : https://www.impots.gouv.fr/sites/default/files/formulaires/2502-sd/2026/2502-sd_5581.pdf
- AED et AESH, mutualisateurs : https://www.education.gouv.fr/bo/19/Hebdo23/MENH1915158C.htm ; https://lyc-weil-conflans.ac-versailles.fr/IMG/pdf/mode_operatoirev2-2.pdf ; https://demarche.numerique.gouv.fr/commencer/dombasle-aed-contrats-renouveles
- Imports de variables : https://www.absyscyborg.com/fr/services/centre-de-formation-absys-cyborg/formation-sage-paie-importer-les-elements-variables ; https://absyscyborg.com/fr/logiciels/silae/my-silae/saisie-des-evp ; https://factorial.fr/apps/integration/silae
- Charlemagne Paie (Aplim) : https://aplim.fr/ressources/communication/infomag/infomagaplim_202106.pdf
- Échecs : Légifrance (403) ; PDF du référentiel CNIL non lu en entier ; CCN non lue sur Légifrance.

---

## 34. Révision D-047 (2026-11-11) — calcul de la paie et DSN dans ScolIA — **l'emporte sur tout ce qui précède, §31 compris**

**Décision.** D-047 (Florian, 2026-11-11) **confirme D-025** : la paie, **calcul des bulletins et DSN compris**, est construite dans ScolIA. **Q-RH1 est close** : l'option 1 du §3.2 est retenue ; la recommandation du §3.3 (option 2, calcul et DSN au logiciel de paie de l'OGEC ou de son cabinet) est **abandonnée**. Les lots RH11 et RH12, « non engagés », deviennent **obligatoires** et sont redécoupés (§34.10). Rien n'est supprimé au-dessus : tout ce qui entoure le calcul (RH0 à RH10, dossier, contrats, temps, variables, échéances, santé au travail, formation, collectif, public, personas) reste valable et devient l'**amont** du moteur.
**Numérotation de la révision** : actions **RHP-280 à RHP-339** (étend la plage du socle, qui s'arrêtait à RHP-279) ; règles **R-RH51 à R-RH80** ; cas limites **CRH-61 à CRH-89** ; questions **Q-RH26 à Q-RH40** ; réglages **S-4929 à S-4949** ; lots **RH11 à RH22** ; migrations **dans 2980–2999, plage inchangée** (§34.13).

### 34.1 Ce qui change, ce qui reste

| Passage | Avant | Après D-047 |
|---|---|---|
| §0 point 3, §1 point 3, §3.3 | ScolIA prépare, le logiciel de paie calcule et déclare | **ScolIA calcule les bulletins, produit et dépose la DSN** (mensuelle et signalements), reçoit les retours (CRM, taux de prélèvement à la source), paie les nets (lot de virement du 48) et produit le journal de paie |
| §12 (échange avec le logiciel de paie) | format pivot, adaptateur, retour des bulletins | l'**adaptateur** ne sert plus qu'à la **recette en parallèle** (comparer avec le logiciel actuel, RHP-264) et à la **reprise** ; le circuit officiel devient : variables arrêtées → **calcul** → contrôle → **validation** → bulletins, DSN, virements, écritures (§34.4) |
| §13 | bulletins reçus, DSN jamais produite | bulletins **générés** (modèle clarifié), DSN **générée, contrôlée, déposée** (§34.5, §34.6) |
| RR-06, RR-07, RR-10 | « argument pour ne pas faire » | obligations **du logiciel** (§34.2) |
| Q-RH10 | nets payés par le logiciel de paie | **nets payés par le 48** (lot de virement, double validation R-CG27), fichier des nets **non nominatif** pour le 48 au-delà du droit `paie.payer` |
| R-CG22 (48) | journal de paie agrégé reçu | inchangé dans le 48 (agrégé par rubrique et section) ; le **détail par salarié** reste dans le 49 |
| RR-17 taxe sur les salaires | calculée par le logiciel de paie | **calculée par ScolIA** (base, taux par tranches, abattement des associations, versements et déclaration annuelle 2502 préparée) [NV au détail des échéances de versement, Q-RH27] |
| §32 contradiction D-025 | signalée | **levée** |
| Public (§17, Q-RH18) | états au payeur, aucune paie locale | **inchangé** : AED, AESH et agents publics sont payés par l'État, le rectorat ou l'EPLE mutualisateur ; D-047 vise les **salariés de droit privé de l'OGEC** |
| Enseignants agents de l'État (§4, RR-01) | jamais payés par ScolIA | **inchangé** |

### 34.2 Réglementaire du moteur (vérifié le 2026-11-11 ; [V] = page officielle lue, [V-ext] = extrait officiel de moteur de recherche, [NV] = non vérifié)

| # | Règle | Référence | Statut | Conséquence |
|---|---|---|---|---|
| RR-30 | **Norme DSN 2026 (P26V01)** en production, cahier technique **CT 2026.1.2** du 24-12-2025 ; **norme 2027 (P27V01)** pour janvier 2027, **CT 2027.1.3** du 6-11-2026, avec journal de maintenance, note différentielle et tableaux des types de données ; révisions successives du cahier de l'année suivante publiées dès la fin de l'année N-1 puis jusqu'à l'automne | net-entreprises.fr, « Norme et documentation DSN » (page modifiée le 6-11-2026) | [V] | **une version de norme par année civile** : le moteur DSN est versionné par norme ; la norme N+1 est développée dès la publication de son premier cahier et **livrée avant le 1er janvier** (R-RH54) ; chaque révision du cahier (JMN) est relue |
| RR-31 | Changements 2027 notables : NAF 2025, suppression des codes APEN/APET, suppression de la « sortie de la DSN » (DSN sans individu à déposer) | note différentielle CT2026.1.2 → CT2027.1.3 | [V-ext] | exemple du travail annuel ; cas « mois sans salarié » à traiter (CRH-61) |
| RR-32 | **DSN-Val** : outil officiel gratuit de contrôle local (Windows, Linux 64 bits, Java 8 ou 11), version 2026.1.0.18 du 27-10-2026, version 2027.1.0.5 ; la **brique de contrôle** peut être intégrée à un logiciel de paie après signature d'un **protocole** avec le GIP-MDS ; elle **ne délivre aucun certificat de conformité** | net-entreprises.fr, « Outils de contrôle DSN-Val » ; protocole d'utilisation de la brique (oct. 2025) | [V] | contrôle **systématique** de chaque DSN par la brique avant dépôt (R-RH55) ; **protocole à signer** (Q-RH28) ; licence de la brique à analyser avant intégration (CLAUDE.md) |
| RR-33 | **Trois niveaux de test** : DSN-Val en local ; **DSN de test** déposée sur la production (contrôle du SIRET) ; **plateforme éditeurs test.net-entreprises.fr** (inscription à la déclaration « DSNEDITEUR », tests de bout en bout sans transmission aux organismes, SIRET et NIR **réels** exigés, rétroactivité 12 mois, conservation 13 mois) | guide de la plateforme éditeurs ; guide « émettre une DSN test » | [V-ext] | recette (§34.9) ; les **NIR réels** sur la plateforme de test = traitement de données réelles : **accord écrit de l'établissement et avis du DPO** avant (D-034, Q-RH29) |
| RR-34 | **Échéances** de la DSN mensuelle : le **5 de M+1** (50 salariés et plus, paie du mois) ou le **15 de M+1** (moins de 50, ou paie décalée), à midi ; report au jour ouvrable suivant | urssaf.fr, « Déclaration sociale nominative » ; CSS L133-5-3 (article réglementaire des dates [NV]) | [V-ext] | calendrier calculé par employeur selon l'effectif (S-4929) ; alerte J-3 ; **pénalités** de retard [NV au montant] |
| RR-35 | **Signalements d'événements** : arrêt de travail dans les **5 jours** de la connaissance (la DSN mensuelle peut suffire en cas de subrogation), reprise anticipée dans les 5 jours, **fin de contrat de travail unique (FCTU)** dans les 5 jours | net-entreprises, « gestion des DJT » ; ameli.fr formalités employeur | [V-ext] ; délais **à reconfirmer dans le CT 2027** | signalements **générés** depuis les faits du 15 (absence de nature arrêt) et du 49 (sortie) ; tâche R-RH20 devient génération + dépôt + suivi |
| RR-36 | **Bulletin** : mentions de **R3243-1** et suivants ; **montant net social** (9° bis, décret 2023-1378, en vigueur au 1-1-2024 ; ligne obligatoire depuis le 1-7-2023) ; présentation **clarifiée** (arrêté du 25-2-2016 modifié par les arrêtés du 31-1-2023 et du 25-6-2024) | Légifrance ; boss.gouv.fr ; travail-emploi.gouv.fr | [V] | **modèle de bulletin** conforme à l'arrêté en vigueur, versionné par date d'effet (R-RH56) ; test de présence de chaque mention |
| RR-37 | **Bulletin électronique** sauf opposition (L3243-2) ; disponibilité **50 ans** ou jusqu'à l'âge de L1237-5 **augmenté de 6 ans** ; consultable **via le compte personnel d'activité** ; **information des salariés 3 mois avant** une fermeture du service (D3243-8, décret 2016-1762) ; double conservé **5 ans** par l'employeur (L3243-4) | Légifrance ; service-public.fr | [V] (D3243-8, L3243-2) ; [V-ext] (L3243-4) | coffre du salarié (§13) ; **raccordement au CPA** : modalités techniques à étudier [NV, Q-RH30] ; engagement de disponibilité **au-delà de la vie du contrat ScolIA** (remise d'archive et préavis de 3 mois, R-RH57) |
| RR-38 | **Prélèvement à la source** : taux du foyer (CGI 204 H) transmis au collecteur par le **CRM nominatif** de la DSN ; **grille de taux neutre** (204 H III) à défaut ; obligations du collecteur (BOI-IR-PAS-30-10-10) ; **sanctions** du collecteur (CGI 1759-0 A, BOI-IR-PAS-30-10-60) : 5 % (erreur, omission), 10 % (retard), 40 % (manquement délibéré), 80 % (retenue non reversée sciemment), minimum 250 € | impots.gouv.fr ; bofip.impots.gouv.fr | [V-ext] ; barème 2026 de la grille neutre [NV] ; secret du taux (CGI 1753 bis C [NV]) | import automatique des CRM ; grille neutre paramètre daté ; **taux visible du seul salarié et de la gestionnaire de paie** (R-RH58) |
| RR-39 | **Plafond de la sécurité sociale 2026** : 4 005 € par mois, 48 060 € par an (arrêté du 22-12-2025) ; **SMIC au 1-1-2026** : 12,02 € de l'heure (décret 2025-1228), revalorisation au 1-6-2026 annoncée à 12,31 € [NV] | JO ; info.gouv.fr | [V-sec] (PSS) ; [V] (SMIC 1-1-2026) | **paramètres de plateforme datés** (R-RH53) ; une revalorisation en cours d'année est un **événement** qui relance le calcul des paies concernées |
| RR-40 | **Réduction générale dégressive unique (RGDU)** au 1-1-2026 : CSS L241-13, D241-7 à D241-11 ; rémunérations < 3 SMIC ; C = Tmin + Tdelta × [½ × (3 × SMIC annuel / rémunération annuelle brute − 1)]^P, P = 1,75, Tmin = 0,0200, Tdelta 0,3781 (< 50 salariés) ou 0,3821 (≥ 50) ; **décret 2026-509 du 12-6-2026** fige le SMIC de référence à sa valeur du 1-1-2026 pour 2026 et crée D241-7-1 (paramètre changeant en cours d'année) | Légifrance ; urssaf.fr | [V] (décret 2026-509) ; paramètres [V-ext] ; **divergence** : un texte issu du décret 2025-887 cite Tdelta 0,3813 [à lever, Q-RH31] | calcul annuel avec **régularisation progressive** mensuelle et régularisation annuelle [NV au détail BOSS] ; jeu de tests de référence par paramètre |
| RR-41 | **BOSS** (boss.gouv.fr) : doctrine sociale opposable de l'administration | boss.gouv.fr | [V principe] | chaque règle de cotisation du moteur cite sa **référence BOSS** (fiche, paragraphe, date) dans le référentiel (R-RH52) |
| RR-42 | **CCN de l'enseignement privé non lucratif (EPNL), IDCC 3218, du 13-10-2020** (issue de l'avenant 2020-03, **révisée par l'avenant 2022-2 du 11-4-2022**, texte unique appliqué depuis le 1-9-2022) ; signataires CEPNL, SPELC, FEP-CFDT (puis adhésion CGT) ; **remplace** l'ancienne convention PSAEE (IDCC 2408) et la convention SEP (IDCC 3211) ; chapitre 4 classifications et **salaires minima hiérarchiques** (accord 2022-7 du 12-12-2022 ; accords de salaires 2021-1, 2021-2, 2022-1, 2022-3, 2022-5, avenant 2023-1), CQP classés (EVS, CVS, attaché de gestion) ; chapitre 7 **maintien de salaire** et **protection sociale complémentaire** (prévoyance et frais de santé) ; **extension non établie** (Légifrance n'affiche pas d'arrêté d'extension ; une source secondaire la dit « non étendue ») | Légifrance KALICONT000043874879 ; code.travail.gouv.fr | [V] (identité, chapitres) ; extension, strates, annualisation, internat, surveillance [NV] | **corrige RR-03** (date de la convention : 13-10-2020, révisée le 11-4-2022) ; référentiel CCN **versionné par avenant** (grilles, ancienneté, primes, maintien, prévoyance) ; **lecture intégrale par un juriste en droit social** avant RH12 (Q-RH5) ; application à l'OGEC par adhésion de l'employeur au syndicat signataire à confirmer (Q-RH32) |
| RR-43 | **Aucune labellisation** officielle des logiciels de paie ; la conformité se constate par les contrôles DSN (brique, dépôt, CRM) ; la **responsabilité** des déclarations et cotisations reste celle de l'**employeur** | net-entreprises (brique sans certificat) | [V] (absence de certificat) ; responsabilité [raisonnement, à confirmer par le juriste] | ScolIA ne peut **rien promettre** de plus que la conformité aux contrôles ; responsabilité contractuelle d'éditeur à cadrer (Q-RH33) |
| RR-44 | Données des salariés : **référentiel CNIL « gestion du personnel »** (délibération 2019-160, JO du 15-4-2020) ; **référentiel des durées de conservation RH** du 2-4-2026 | cnil.fr | [V] | §22.4 et §34.8 ; décret-cadre NIR [NV] |

### 34.3 Le moteur de paie

**Principes**
1. **Le calcul est une fonction pure et rejouable** : (variables arrêtées du mois, contrat et rémunération à date, référentiel réglementaire à date, référentiel CCN à date, cumuls de l'année, taux de prélèvement) → bulletin. Mêmes entrées, même bulletin, au centime, aujourd'hui comme dans dix ans (R-RH51). Chaque bulletin conserve l'**identifiant de version** de chaque référentiel utilisé.
2. **Tout paramètre réglementaire est daté et sourcé** : taux, plafonds, SMIC, tranches, grilles, formules ; jamais codé en dur ; chaque valeur porte sa date d'effet, sa source (texte, BOSS, fiche URSSAF, accord de branche, contrat de prévoyance) et la personne qui l'a saisie et validée (R-RH52, R-RH53). Les paramètres **nationaux** sont saisis une fois par ScolIA (référentiel de plateforme, `globale_reference`) ; les paramètres **de l'employeur** (taux AT/MP notifié, contrats de prévoyance et de mutuelle, taux de versement mobilité selon la commune, effectif) le sont par établissement.
3. **Rubriques** : catalogue de rubriques de calcul (gains, cotisations salariales et patronales, réductions, retenues, non soumis, informations), chacune avec sa **formule déclarative** (base, assiette, taux, plafond, arrondi, ordre), ses **codes DSN** (types de base assujettie, codes de cotisation individuelle, CTP URSSAF), son **compte comptable** et sa **ligne du bulletin clarifié**. Formules exprimées dans un petit langage déclaratif **interne, versionné, testé** — pas de script libre ni de JSON (D-036 : une rubrique = des lignes de table typées).
4. **Profils** de cotisation par population (cadre, non-cadre, apprenti, CDD d'usage, temps partiel, salarié de plus de 62 ans [NV], stagiaire gratifié) : le salarié hérite de son profil par son contrat ; aucune case à cocher libre.
5. **Ordre de calcul** : brut (base contractuelle, ancienneté CCN, heures complémentaires et supplémentaires, primes, avantages en nature, absences et maintien) → assiettes (plafonnée, déplafonnée, CSG-CRDS, tranches Agirc-Arrco, chômage) → cotisations → réductions (RGDU, heures supplémentaires) → **net social** → **net imposable** → **prélèvement à la source** → retenues et acomptes → **net à payer** → coût employeur.
6. **Absences** (nature codée du 15) : retenue selon la méthode retenue (heures réelles, jours ouvrés ou calendaires selon la CCN et la jurisprudence [NV, juriste]) ; **maintien de salaire** CCN chapitre 7 et légal ; **indemnités journalières** : subrogation (l'employeur avance et reçoit les IJSS : rapprochement du relevé de l'assurance maladie [NV au format]) ou non ; IJ de prévoyance ; **jamais** de motif médical (R-RH6 inchangé).
7. **Congés payés** : acquisition et prise (déjà RH3), **indemnité** par la comparaison maintien / dixième (R-RH59), provision de congés payés et charges pour la clôture comptable (48 §35.3) ; règles issues de la jurisprudence et de la loi de 2024 sur l'acquisition pendant la maladie [NV, juriste].
8. **Temps de travail annualisé** (CCN EPNL, personnels d'éducation [NV]) : lissage de la rémunération, régularisation en fin de période d'annualisation ou à la sortie.
9. **Régularisations** : un fait tardif ou une erreur se corrige sur la **paie suivante** par des rubriques de rappel (période d'origine conservée), jamais en réécrivant un bulletin émis (R-RH15 inchangé, R-RH60) ; la DSN porte les régularisations dans les blocs prévus [à vérifier par norme].
10. **Sorties** : indemnités de rupture (légale, conventionnelle, rupture conventionnelle, fin de CDD, compensatrice de congés), **solde de tout compte**, **certificat de travail**, **attestation employeur** transmise par la DSN (FCTU) ; modèles relus par le juriste (Q-RH5).
11. **Taxe sur les salaires** : calculée à partir des rémunérations brutes par tranche, abattement des associations, déclarée et versée selon le calendrier en vigueur (Q-RH27).

**Contrôles avant validation** (RHP-286) : bulletin à net négatif ; net inférieur au SMIC horaire proratisé (hors absences) ; salaire de base inférieur au minimum conventionnel de la classification ; heures au-delà des plafonds légaux ; variation du net de plus de S-4930 % sans variable explicative ; rubrique sans paramètre valide à la date ; cotisation de prévoyance absente pour un salarié couvert ; taux PAS absent depuis plus de 2 mois (grille neutre appliquée) ; cumul annuel incohérent ; salarié sorti payé ; double paie.

### 34.4 Circuit mensuel

| Étape | Qui | Geste |
|---|---|---|
| 1. Variables arrêtées | direction (inchangé, §12) | ★ |
| 2. **Calcul** (pré-paie) | SYS, lancé par la gestionnaire | tous les bulletins en brouillon, rejouables |
| 3. Contrôles et revue | gestionnaire de paie | anomalies RHP-286 levées ou motivées ; comparaison au mois précédent |
| 4. **Validation de la paie** | direction ou président (jamais pour sa propre paie, R-RH41) | ★ réauthentification ; bulletins figés et numérotés |
| 5. Bulletins | SYS | PDF/A au coffre de chaque salarié ou papier (opposition, R-RH42), le jour du paiement |
| 6. **Virement des nets** | 48 (lot, double validation) | fichier SEPA ; date de paiement S-4931 |
| 7. **Journal de paie** | SYS → 48 | écritures agrégées (R-CG22) ; contrôle d'égalité avec les bulletins |
| 8. **DSN mensuelle** | SYS génère et contrôle (brique) ; gestionnaire dépose | ★ avant l'échéance (RR-34) |
| 9. **Retours** | SYS | accusés, bilans d'anomalies, CRM nominatifs (taux PAS), CRM de cotisations ; anomalies en tâches |
| 10. **Paiement des organismes** | 48 | prélèvements ou virements URSSAF, retraite complémentaire, prévoyance, DGFiP (PAS) selon les échéances |
| 11. Clôture de la période | gestionnaire | cumuls figés |

**Dépôt de la DSN** : par défaut le fichier est **déposé par la gestionnaire** sur net-entreprises avec ses propres identifiants (ScolIA ne les connaît jamais) ; le dépôt **machine à machine** (API de net-entreprises) est un lot ultérieur, après étude de l'authentification et du stockage des secrets (jamais de mot de passe d'un déclarant stocké en clair ; Q-RH34) ; les retours sont importés de la même façon.

### 34.5 Bulletins dématérialisés

- Générés par ScolIA en **PDF/A**, modèle clarifié en vigueur à la date de paie (RR-36), **un numéro par bulletin**, empreinte, objet immuable (E5) ; aucune régénération d'un bulletin émis (une correction = bulletin de régularisation du mois suivant, ou bulletin rectificatif **distinct** quand le juriste l'exige [NV], CRH-62).
- **Explications** en langage simple (RHP-276 conservé) : chaque ligne cliquable (« pourquoi cette retenue ? »), sans jamais révéler le taux PAS à un tiers.
- **Disponibilité** 50 ans ou jusqu'à l'âge de L1237-5 + 6 ans (RR-37) : coffre du salarié accessible après le départ (compte allégé « mes bulletins », Q-RH9), **archive remise** au départ, et **engagement de continuité** : si ScolIA cesse le service, préavis de 3 mois et remise des archives à chaque salarié (R-RH57) ; raccordement au CPA à étudier (Q-RH30).
- **Double employeur** conservé 5 ans (L3243-4) puis selon le référentiel CNIL (§22.4).

### 34.6 DSN

- **Génération** : un fichier par SIRET et par mois (mensuelle) et par événement (signalement), conforme à la **norme de l'année** ; blocs individus, contrats, rémunérations, versements, bases, cotisations individuelles, cotisations agrégées par organisme (URSSAF, retraite complémentaire, prévoyance, mutuelle), PAS ; **fichier texte** conforme au cahier technique, stocké en objet chiffré (contient des NIR et des salaires) ; les **données** restent en tables typées (aucun JSON, D-036).
- **Contrôle** : brique de contrôle intégrée (RR-32) ; aucune DSN déposable avec une anomalie **bloquante** ; anomalies non bloquantes affichées, décision motivée de la gestionnaire (R-RH55).
- **Signalements** : arrêt de travail, reprise anticipée, FCTU, générés depuis les faits ; délai suivi (RR-35) ; signalement « annule et remplace » quand la règle le prévoit.
- **Retours** : accusé de réception, bilan d'identification ou d'anomalies, CRM nominatif (taux PAS), CRM de cotisations ; chaque retour rattaché à sa DSN ; anomalie = tâche datée à la gestionnaire.
- **Annule et remplace** de la DSN mensuelle avant l'échéance ; après l'échéance, correction le mois suivant (R-RH61).
- **Paramétrage des organismes** : fiches de paramétrage des organismes complémentaires (prévoyance, mutuelle : références de contrat, populations, options) importées ou saisies ; CTP URSSAF ; taux AT/MP **notifié** par le compte AT/MP de l'employeur (saisi, pièce jointe, date d'effet) [sources non relues, §34.2].

### 34.7 Mise à jour réglementaire (au moins annuelle)

| Quand | Quoi | Qui |
|---|---|---|
| dès la publication du premier cahier technique N+1 (fin N-1) puis à chaque révision (JMN) | lecture de la note différentielle, développement de la norme N+1, tests DSN-Val N+1 | équipe paie |
| septembre–décembre | **campagne de fin d'année** : LFSS et décrets (taux, assiettes, réductions), PSS, SMIC, Agirc-Arrco, chômage, versement mobilité, grille de taux neutre ; jeu de référence N+1 calculé et comparé | équipe paie + relecture par un expert paie |
| 1er janvier | **bascule de norme et de paramètres** ; aucune paie de janvier calculée avec la norme N-1 (R-RH54) | SYS |
| en cours d'année | revalorisations (SMIC, salaires minima de branche après accord), décrets (ex. décret 2026-509 en juin 2026), avenants CCN, nouveaux taux AT/MP, contrats de prévoyance | équipe paie ; **événement** qui relance les paies concernées |
| à chaque changement | **notes de version réglementaire** publiées aux établissements, avec la source et les salariés touchés (RHP-300) | SYS |
| chaque année | **revue externe** du référentiel par un expert paie ou un cabinet (Q-RH35) ; rejeu complet du jeu de référence | externe |

### 34.8 Sécurité des données des salariés et droits très restreints

- **Droits nouveaux** (D-017), tous vides par défaut, attribués nominativement, jamais par rôle générique : `paie.calculer` (lancer le calcul, voir les brouillons), `paie.valider` ★ (valider la paie), `paie.bulletins.lire_tous` ★ (accès individuel journalisé avec motif, jamais en masse hors export légal), `paie.dsn.generer`, `paie.dsn.deposer` ★, `paie.parametrer_employeur` (AT/MP, contrats d'organismes), `paie.referentiel.national` (**plateforme seulement**, jamais un établissement), `paie.payer` (déjà prévu).
- **Personne ne valide sa propre paie** ni celle d'un proche déclaré (R-RH41 étendu, R-RH62) ; la gestionnaire ne voit sa propre paie que comme salariée.
- **Chiffrement** (E11) : NIR, IBAN, taux PAS, salaires des bulletins et fichiers DSN en objets chiffrés avec identifiant de clé ; empreinte HMAC des IBAN (R-CG47) ; **aucune** donnée de paie dans les journaux applicatifs (E14) ni dans les notifications (« votre bulletin est disponible », sans montant).
- **Journalisation** de toute consultation individuelle (bulletin, rémunération, taux PAS), revue trimestrielle par le président (RHP-263) et communication au salarié sur demande (RHP-269).
- **Fichiers sortants** (DSN, virements, exports) : usage unique, expiration (RHP-279, S-4926), jamais par courriel.
- **Environnements** : aucun bulletin ni NIR réel hors production ; recette sur données fictives, sauf la plateforme éditeurs (RR-33) avec accord écrit et avis du DPO (Q-RH29).
- **IA** : peut expliquer un bulletin **au salarié concerné**, repérer des anomalies pour la gestionnaire ; **ne calcule rien d'officiel**, ne valide rien, ne lit aucune nature d'absence au-delà du code, ne voit jamais le taux PAS d'un autre que l'utilisateur (R-RH63).
- **Analyse d'impact (AIPD)** du traitement de paie à faire avec le DPO avant la recette sur données réelles (Q-RH36).

### 34.9 Recette en parallèle (doublon 2027-2028) et critères chiffrés de bascule

**Principe** : pendant le doublon, la paie **officielle** reste dans le logiciel actuel (ou chez le cabinet). ScolIA calcule **chaque mois toutes les paies** à partir des **mêmes variables** (les siennes, arrêtées en réel selon §31.1, ou celles du logiciel importées par l'adaptateur) et **compare bulletin par bulletin, ligne par ligne** (RHP-264 étendue, RHP-310) ; il **génère** la DSN du mois, la contrôle par la brique et la dépose sur la **plateforme éditeurs** de test (jamais en production : une seule DSN mensuelle officielle par SIRET). Les salariés ne voient **pas** les bulletins parallèles.

**Calendrier recommandé** : le parallèle commence dès que le moteur passe le **jeu de référence** (au plus tard **janvier 2028**, idéalement septembre 2027) ; il doit couvrir **au moins 12 mois consécutifs comprenant un changement d'année civile** (paramètres de janvier, régularisation annuelle de la RGDU, nouvelle norme DSN, CRM de janvier) ; **bascule au 1er janvier** (cumuls annuels civils, réduction générale, PAS) : **1-1-2029** recommandé ; une bascule au 1-9-2028 n'est possible qu'avec reprise certifiée des cumuls de janvier à août et 12 mois de critères atteints (R-RH64, Q-RH26).

**Critères chiffrés (tous requis, sur les 12 derniers mois)**
| # | Critère | Seuil |
|---|---|---|
| K-RH1 | **Net à payer** ScolIA = officiel | **100 %** des bulletins au centime, 12 mois consécutifs |
| K-RH2 | Chaque ligne de cotisation (base, taux, montant salarial et patronal) | écart ≤ **0,01 €** sur 100 % des lignes ; tout écart expliqué par une erreur **du logiciel officiel** documentée et acceptée par le cabinet |
| K-RH3 | Brut, net social, net imposable, PAS, coût employeur | **100 %** identiques au centime |
| K-RH4 | DSN mensuelles | **0** anomalie bloquante à la brique et **0** anomalie au dépôt sur la plateforme éditeurs, 12 mois ; montants agrégés par organisme = DSN officielle au centime |
| K-RH5 | Signalements | 100 % des arrêts, reprises et fins de contrat du parallèle générés dans le délai et acceptés par la plateforme de test |
| K-RH6 | Changement d'année | paramètres et norme N+1 en place **avant le 1er janvier** ; paie de janvier conforme (K-RH1 à K-RH4) ; régularisation annuelle de la RGDU identique |
| K-RH7 | Sorties | 100 % des soldes de tout compte, certificats et attestations du parallèle identiques ou meilleurs (écart expliqué) |
| K-RH8 | Comptabilité | journal de paie équilibré et égal aux bulletins ; provision de congés payés de la clôture à blanc = officielle à ± 1 € par salarié |
| K-RH9 | Rejeu | recalcul d'un mois ancien à l'identique après mise à jour des référentiels (fonction pure, R-RH51) |
| K-RH10 | Validation externe | avis écrit favorable d'un **expert paie** (cabinet ou gestionnaire certifié) sur le référentiel et les bulletins, et d'un **juriste en droit social** sur la CCN, les absences, les sorties (§34.12) |
| K-RH11 | Sécurité | revue de sécurité sans écart critique ni important ; test des pièges §22.2 étendus ; AIPD faite |
| K-RH12 | Exploitation | dépôt DSN réalisé par la gestionnaire en moins de 30 min par mois sur les 3 derniers mois ; aucune ressaisie |

Un critère manqué **reporte** la bascule à l'année civile suivante, sauf décision écrite de Florian et de l'établissement (R-RH65).

### 34.10 Lots redécoupés

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| RH0 à RH10 | inchangés (§27.1, §31.6) ; RH5 devient « adaptateur de **comparaison** et de reprise » | | |
| **RH11** référentiels | référentiel national daté et sourcé (PSS, SMIC, taux, tranches, formules), référentiel CCN EPNL par avenant, paramètres employeur (AT/MP, organismes), circuit de saisie et double validation | RH1 | chaque valeur a sa source ; double validation |
| **RH12** moteur de calcul | rubriques, profils, langage de formules, ordre de calcul, brut → net à payer, cumuls, contrôles, rejeu | RH11, RH4 | **jeu de référence** (au moins 60 cas types : temps plein, partiel, cadre, apprenti, entrée, sortie, absences de chaque nature, heures, primes, plafond, 3 SMIC) au centime, validé par un expert paie |
| **RH13** absences et maintien | retenues, maintien CCN et légal, IJSS, subrogation, prévoyance, temps partiel thérapeutique | RH12 ; 15 | cas types validés par le juriste |
| **RH14** congés et annualisation | indemnité (maintien / dixième), provision CP, annualisation et sa régularisation | RH12 ; RH3 | |
| **RH15** prélèvement à la source | taux, grille neutre, CRM, régularisations | RH12 | |
| **RH16** bulletins | modèle clarifié versionné, PDF/A, coffre, papier, explication, archive au départ | RH12 ; 7 | 100 % des mentions R3243-1 (test) |
| **RH17** DSN mensuelle | génération par norme, brique de contrôle, dépôt manuel, annule et remplace | RH12 ; protocole GIP-MDS (Q-RH28) | 0 bloquant sur 12 mois fictifs |
| **RH18** DSN événementielle et retours | arrêts, reprises, FCTU, import des retours et CRM, tâches | RH17 ; 15 | |
| **RH19** paiements et comptabilité | nets par le 48, organismes, journal de paie, taxe sur les salaires | RH12 ; 48 CG7 | journal = bulletins |
| **RH20** sorties | indemnités, STC, certificat, attestation par la DSN | RH12, RH18 ; RH2 | modèles relus par le juriste |
| **RH21** veille et versions | calendrier annuel, norme N+1, jeu de référence N+1, notes de version, revue externe | RH12, RH17 | campagne de fin d'année tenue |
| **RH22** recette parallèle et bascule | comparaison bulletin par bulletin, plateforme éditeurs, tableau K-RH, reprise des cumuls | RH12 → RH20 | K-RH1 à K-RH12 |
| (option) | dépôt machine à machine, DPAE générée, raccordement CPA | RH17 | après Q-RH34, Q-RH30 |

**Parcours** : RH0 → RH4 et RH6 en réel à la rentrée 2027 (inchangé) ; RH11 → RH17 **avant le parallèle** ; RH18 → RH22 pendant ; bascule 1-1-2029 recommandée.

### 34.11 Risques

| # | Risque | Gravité | Parade |
|---|---|---|---|
| RK-RH1 | **Salaire faux** versé à une personne | élevée | parallèle 12 mois, K-RH1 à K-RH3, contrôles RHP-286, validation humaine |
| RK-RH2 | DSN rejetée ou fausse : cotisations mal déclarées, droits des salariés (chômage, retraite, IJ) atteints, pénalités | élevée | brique de contrôle, K-RH4, suivi des retours |
| RK-RH3 | **Retard de la norme ou des paramètres au 1er janvier** | élevée | RH21, R-RH54 ; plan de secours : DSN par l'outil de saisie de net-entreprises ou par le cabinet pour un mois (Q-RH37) |
| RK-RH4 | Mauvaise lecture de la CCN EPNL (non étendue, avenants, maintien) | élevée | juriste (Q-RH5), référentiel versionné, revue externe |
| RK-RH5 | Fuite de données de paie (salaires, NIR, taux PAS) | élevée | §34.8 ; chiffrement ; droits nominatifs ; revue trimestrielle |
| RK-RH6 | Dépendance à une ou deux personnes qui connaissent la paie | élevée | documentation du référentiel, revue croisée, jeu de référence |
| RK-RH7 | Coût récurrent sous-estimé | moyenne | §34.12 assumé |
| RK-RH8 | Responsabilité d'éditeur, recours d'un salarié ou de l'employeur | moyenne | contrat, assurance (Q-RH33) |
| RK-RH9 | Calendrier D-034 (V1 en février 2027) **incompatible** avec un moteur de paie fiable | moyenne | la V1 de février 2027 couvre RH0–RH10 ; le moteur (RH11–RH17) vise l'été 2027 au mieux et janvier 2028 au plus tard pour le parallèle (contradiction signalée, Q-RH38) |

### 34.12 Coûts de maintenance réglementaire assumés et validations externes

**Coûts [H, à chiffrer par Florian]** : construction RH11 → RH22 de l'ordre de **3 à 5 années-personne** (dont un profil **expert paie** à temps plein ou quasi) ; ensuite **au moins 1 personne à temps plein** pour la veille, la norme annuelle, les paramètres, la campagne de fin d'année et le support aux gestionnaires, avec un **pic de charge d'octobre à janvier** ; plus une **revue externe annuelle** (expert paie) et des consultations juridiques (CCN, sorties, absences). Ce coût est **le même pour 1 ou 100 établissements** : il ne s'amortit qu'avec le nombre d'employeurs servis.

**À valider par un juriste en droit social** (Q-RH5, renforcé) : lecture intégrale de la CCN EPNL et de ses avenants (classifications, minima, ancienneté, primes, annualisation, éducateurs d'internat, surveillants, congés, maintien de salaire, prévoyance) et de son application à l'OGEC (adhésion, extension) ; méthodes de retenue pour absence ; congés payés et maladie ; temps partiel thérapeutique ; ruptures et indemnités ; modèles de bulletin rectificatif ; périodes protégées (R-RH43) ; cumul d'activités (R-RH44) ; responsabilité de l'éditeur.
**À valider par un expert paie ou un expert-comptable** : référentiel des rubriques et des taux, profils, jeu de référence, cas types des absences et des sorties, provision de congés payés, journal de paie et schéma comptable (48 §9.2), taxe sur les salaires, critères K-RH et avis de bascule (K-RH10).
**À valider par le DPO** : AIPD (Q-RH36), données réelles sur la plateforme éditeurs (Q-RH29), durées de conservation (Q-RH20).

### 34.13 Migrations (plage **2980–2999 inchangée**, contenus réaffectés)

Aucun fichier écrit. 2980 (réservée à RH0, qui n'a pas de table) et 2996–2998 (réserve) reçoivent le moteur :
| N° | Contenu | Lot |
|---|---|---|
| 2980 | référentiel réglementaire daté : `ent_paie_parametre (id, etablissement_id NULL, code, valeur_num, unite, du, au NULL, source_texte, source_url, saisi_par, valide_par)` ; `ent_paie_ccn_grille (id, idcc, avenant_ref, categorie_id, du, au, minimum_centimes)` ; paramètres employeur (AT/MP, versement mobilité, effectif) | RH11 |
| 2986 (étendue) | rubriques de calcul : `ent_paie_rubrique_calcul (id, etablissement_id NULL, code, version, du, au, nature, ordre, arrondi)`, `ent_paie_formule_terme (id, rubrique_id, ordre, operande_type, operande_code, operateur)`, codes DSN et comptes, profils et populations | RH12 |
| 2996 | calcul : `ent_paie_calcul (id, etablissement_id, periode_id, lance_par, versions_referentiels_ref, etat)`, `ent_paie_bulletin (id, etablissement_id, calcul_id, salarie_id, numero, etat CHECK IN ('brouillon','valide','emis','regularise'), net_a_payer_centimes, net_social_centimes, net_imposable_centimes, pas_centimes, cout_employeur_centimes, objet_id NULL, empreinte)`, `ent_paie_bulletin_ligne (id, etablissement_id, bulletin_id, rubrique_id, rubrique_version, base_centimes, taux_pm NULL, montant_sal_centimes, montant_pat_centimes, periode_origine DATE NULL)`, `ent_paie_cumul (etablissement_id, salarie_id, annee, code, montant_centimes)`, organismes et contrats (prévoyance, mutuelle), IJSS et subrogation | RH12 → RH15 |
| 2997 | DSN : `ent_dsn_declaration (id, etablissement_id, employeur_id, mois, nature CHECK IN ('mensuelle','arret','reprise','fin_contrat'), norme, rang, annule_remplace_id NULL, objet_chiffre_id, empreinte, controle_bloquants INT, controle_non_bloquants INT, etat CHECK IN ('generee','controlee','deposee','acceptee','rejetee'), deposee_le, deposee_par, environnement CHECK IN ('production','test_editeur'))`, `ent_dsn_anomalie (id, declaration_id, code, gravite, libelle, traitee_par NULL)`, `ent_dsn_retour (id, declaration_id, nature CHECK IN ('accuse','bilan','crm_nominatif','crm_cotisations'), objet_id, importe_le)`, `ent_paie_taux_pas (id, etablissement_id, salarie_id, taux_pm, du, source CHECK IN ('crm','neutre'), retour_id NULL)` (chiffré) | RH15, RH17, RH18 |
| 2998 | taxe sur les salaires, sorties (indemnités, STC, certificats), comparaison du parallèle (`ent_paie_comparaison`, `ent_paie_comparaison_ecart`), notes de version | RH19 → RH22 |

Toutes les colonnes sensibles chiffrées (E11) ; **aucune colonne JSON ni tableau** ; RLS stricte complétée en 2994.

### 34.14 Actions, règles, cas limites ajoutés

**Actions (RHP-280 à RHP-312 ; RHP-313 à RHP-339 réservés)**
| N° | Action | Acteur · droit | Lot |
|---|---|---|---|
| RHP-280 | Saisir ou importer un paramètre national daté avec sa source (double validation) | ÉQUIPE PAIE (plateforme) · `paie.referentiel.national` | RH11 |
| RHP-281 | Publier une version de la CCN EPNL (grilles, primes, maintien) après accord de branche | idem | RH11 |
| RHP-282 | Saisir le taux AT/MP notifié, le versement mobilité, les contrats de prévoyance et de mutuelle de l'employeur | GP · `paie.parametrer_employeur` | RH11 |
| RHP-283 | Affecter un profil de cotisation à un contrat | GP | RH12 |
| RHP-284 | **Lancer le calcul** de la paie du mois (pré-paie) | GP · `paie.calculer` | RH12 |
| RHP-285 | Simuler un bulletin (embauche, augmentation, temps partiel) sans effet | GP, DIR | RH12 |
| RHP-286 | **Contrôles** automatiques avant validation | SYS | RH12 |
| RHP-287 | Lever ou motiver une anomalie | GP | RH12 |
| RHP-288 | **Valider la paie** ★ | DIR ou PRES · `paie.valider` | RH12 |
| RHP-289 | Émettre les bulletins (coffre, papier) | SYS | RH16 |
| RHP-290 | Préparer le **lot de virement des nets** (48) | GP → 48 | RH19 |
| RHP-291 | Produire le **journal de paie** et ses écritures | SYS | RH19 |
| RHP-292 | **Générer la DSN mensuelle** et la contrôler | GP · `paie.dsn.generer` | RH17 |
| RHP-293 | **Déposer** la DSN (téléchargement du fichier, dépôt sur net-entreprises, attestation) ★ | GP · `paie.dsn.deposer` | RH17 |
| RHP-294 | DSN « annule et remplace » avant l'échéance | GP | RH17 |
| RHP-295 | Générer et déposer un **signalement** (arrêt, reprise, fin de contrat) | SYS → GP | RH18 |
| RHP-296 | Importer les **retours** (accusés, bilans, CRM) | GP | RH18 |
| RHP-297 | Appliquer les **taux PAS** reçus | SYS | RH15 |
| RHP-298 | Calculer la **subrogation** et rapprocher les IJSS reçues | GP | RH13 |
| RHP-299 | Calculer la **provision de congés payés** pour la clôture du 48 | SYS | RH14 |
| RHP-300 | Publier une **note de version réglementaire** | SYS | RH21 |
| RHP-301 | Rejouer le **jeu de référence** sur une nouvelle version | SYS | RH21 |
| RHP-302 | Calculer la **taxe sur les salaires** et préparer sa déclaration | GP | RH19 |
| RHP-303 | Calculer un **solde de tout compte** et éditer certificat et attestation | GP | RH20 |
| RHP-304 | Payer les **organismes** (URSSAF, retraite, prévoyance, PAS) | GP → 48 | RH19 |
| RHP-305 | Régularisation (rappel de salaire, trop-perçu avec échéancier légal de retenue [NV]) | GP | RH12 |
| RHP-306 | Expliquer son bulletin (salarié), ligne par ligne | SAL | RH16 |
| RHP-307 | Contester une ligne de bulletin | SAL | RH16 |
| RHP-308 | Reprendre les **cumuls** de l'année et les compteurs à la bascule | GP | RH22 |
| RHP-309 | Annualisation : régulariser en fin de période ou à la sortie | SYS → GP | RH14 |
| RHP-310 | **Comparer** bulletin par bulletin ScolIA / officiel (parallèle) | GP | RH22 |
| RHP-311 | Déposer une DSN sur la **plateforme éditeurs** (test) | ÉQUIPE PAIE | RH22 |
| RHP-312 | Tableau des critères **K-RH** et avis de bascule | DIR, PRES | RH22 |

**Règles (R-RH51 à R-RH65)**
| N° | Règle |
|---|---|
| R-RH51 | Le calcul est une fonction pure et rejouable ; chaque bulletin garde les versions des référentiels utilisés. |
| R-RH52 | Chaque rubrique et chaque paramètre cite sa source (texte, BOSS, accord, contrat) et sa date d'effet. |
| R-RH53 | Aucun taux, plafond ni montant réglementaire codé en dur ; paramètres datés, double validation. |
| R-RH54 | Aucune paie d'une année civile calculée ni déclarée avec la norme ou les paramètres de l'année précédente ; la norme N+1 est livrée avant le 1er janvier. |
| R-RH55 | Aucune DSN déposée avec une anomalie bloquante ; les non bloquantes sont motivées. |
| R-RH56 | Le modèle de bulletin suit l'arrêté en vigueur à la date de paie ; toutes les mentions R3243-1 sont testées. |
| R-RH57 | Bulletins disponibles 50 ans ou jusqu'à l'âge légal + 6 ans ; préavis de 3 mois et remise d'archive avant toute fermeture du service. |
| R-RH58 | Le taux PAS n'est visible que du salarié et de la gestionnaire de paie ; jamais de la direction, du 48, de l'IA d'un tiers. |
| R-RH59 | L'indemnité de congés payés retient le plus favorable du maintien et du dixième. |
| R-RH60 | Un bulletin émis n'est jamais modifié ni régénéré. |
| R-RH61 | Après l'échéance, une DSN se corrige le mois suivant, jamais en silence. |
| R-RH62 | Personne ne calcule, ne valide ni ne paie sa propre paie ou celle d'un proche déclaré. |
| R-RH63 | L'IA n'est jamais source d'un montant officiel. |
| R-RH64 | La bascule de la paie se fait au 1er janvier, sauf reprise certifiée des cumuls. |
| R-RH65 | Un critère K-RH manqué reporte la bascule à l'année civile suivante, sauf décision écrite de Florian et de l'établissement. |

**Cas limites (CRH-61 à CRH-75)**
| # | Cas | Traitement |
|---|---|---|
| CRH-61 | Mois sans salarié (après la suppression de la « sortie de la DSN » en 2027) | DSN sans individu générée selon la norme |
| CRH-62 | Erreur découverte sur un bulletin émis | régularisation le mois suivant ; bulletin rectificatif distinct seulement sur avis du juriste |
| CRH-63 | Revalorisation du SMIC en cours de mois | paramètre daté ; proratisation selon la règle applicable [NV] ; RGDU selon D241-7-1 |
| CRH-64 | Accord de salaires de branche signé en mars, rétroactif à janvier | rappels sur la paie suivante, période d'origine conservée, DSN selon la norme |
| CRH-65 | Taux PAS non reçu pour un nouveau salarié | grille neutre ; alerte au 3e mois |
| CRH-66 | Salarié dont le net serait négatif (absence longue + retenue) | blocage ; report de la retenue selon la règle légale [NV] |
| CRH-67 | Saisie sur salaire reçue | quotité saisissable selon le barème en vigueur (paramètre daté) [NV] |
| CRH-68 | Arrêt de travail connu le 4e jour, la gestionnaire absente | tâche au suppléant ; signalement avant J+5 |
| CRH-69 | DSN rejetée la veille de l'échéance | correction et redépôt ; si impossible, plan de secours (Q-RH37) |
| CRH-70 | Taux AT/MP notifié en mars, effet au 1er janvier | régularisation des cotisations des mois passés selon la règle URSSAF [NV] |
| CRH-71 | Salarié à cheval sur deux établissements d'un même OGEC (même SIRET ou non) | un employeur par SIRET ; une DSN par SIRET ; paie unique si même employeur [NV] |
| CRH-72 | Apprenti mineur | profil apprenti, exonérations applicables à la date [NV] |
| CRH-73 | Départ en cours de mois avec congés non pris | indemnité compensatrice, STC, FCTU dans le délai |
| CRH-74 | Bascule en janvier avec un salarié en arrêt et subrogation en cours | reprise des IJSS attendues et du maintien dans la reprise des cumuls |
| CRH-75 | Nouveau cahier technique publié en novembre avec une rupture | campagne de fin d'année ; si non tenable, plan de secours pour janvier (Q-RH37) |

**Réglages (S-4929 à S-4935)**
| N° | Réglage | Défaut |
|---|---|---|
| S-4929 | Échéance DSN | calculée par l'effectif (5 ou 15 de M+1) |
| S-4930 | Variation de net déclenchant une anomalie | 10 % |
| S-4931 | Jour de paiement des nets | dernier jour ouvré du mois |
| S-4932 | Mode de dépôt DSN | manuel par la gestionnaire |
| S-4933 | Bulletins parallèles visibles des salariés | non |
| S-4934 | Jeu de référence minimal | 60 cas types |
| S-4935 | Revue externe du référentiel | annuelle |

### 34.15 Décisions

**Appliquées par défaut (D-031)**
| # | Question | Décision |
|---|---|---|
| Q-RH1 | Moteur ou interface | **Close par D-047** : moteur et DSN dans ScolIA. |
| Q-RH10 | Paiement des nets | Par le 48 (lot de virement, double validation). |
| Q-RH39 | Dépôt de la DSN | Manuel par la gestionnaire d'abord ; machine à machine plus tard. |
| Q-RH40 | Bulletins du parallèle | Invisibles des salariés (S-4933). |

**Soumis à Florian**
| # | Question | Recommandation |
|---|---|---|
| Q-RH26 | Date de bascule de la paie | **1-1-2029** (parallèle ≥ 12 mois avec un changement d'année) ; 1-9-2028 seulement avec reprise certifiée des cumuls |
| Q-RH27 | Taxe sur les salaires : échéances de versement et déclaration | à confirmer avec le cabinet |
| Q-RH28 | Signature du **protocole GIP-MDS** pour la brique de contrôle ; inscription éditeur sur la plateforme de test | oui, avant RH17 |
| Q-RH29 | NIR et SIRET **réels** sur la plateforme éditeurs | accord écrit de l'établissement et avis du DPO |
| Q-RH30 | Raccordement des bulletins au CPA | à étudier |
| Q-RH31 | Divergence Tdelta (0,3781 / 0,3821 / 0,3813) | lever avec le BOSS ou un expert paie avant RH12 |
| Q-RH32 | Application de la CCN EPNL à l'OGEC pilote (adhésion, extension) | juriste |
| Q-RH33 | Responsabilité d'éditeur de paie, assurance | juriste et assureur |
| Q-RH34 | Dépôt machine à machine : authentification et stockage des secrets | étude de sécurité |
| Q-RH35 | Budget d'une revue externe annuelle (expert paie) | oui |
| Q-RH36 | AIPD du traitement de paie | DPO, avant toute donnée réelle |
| Q-RH37 | **Plan de secours** si la DSN de janvier n'est pas prête (cabinet, outil de net-entreprises) | contrat de secours avec un cabinet pour la première année |
| Q-RH38 | Contradiction avec D-034 (V1 complète en février 2027) | le moteur suit son propre calendrier (parallèle au plus tard janvier 2028) |
| Q-RH5 (rappel) | Juriste en droit social | **indispensable avant RH12** |

### 34.16 Sources de la révision (consultées le 2026-11-11)

- Norme et documentation DSN (P26V01, CT 2026.1.2 ; P27V01, CT 2027.1.3, note différentielle, JMN) : https://www.net-entreprises.fr/declaration/norme-et-documentation-dsn ; documents : https://www.net-entreprises.fr/media/documentation/
- DSN-Val et brique de contrôle : https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/ ; protocole : https://www.net-entreprises.fr/media/documentation/protocole-utilisation-brique-de-controle.pdf
- Plateforme éditeurs et DSN de test : https://www.net-entreprises.fr/media/documentation/guide-plateforme-editeurs.pdf ; https://www.net-entreprises.fr/media/documentation/guide-utilisateur/emettre-une-dsn-test.pdf
- Échéances DSN : https://www.urssaf.fr/accueil/employeur/gerer-entreprise/declaration-sociale-nominative.html
- Signalements : https://www.net-entreprises.fr/media/documentation/gestion-djt-employeurs.pdf ; https://www.ameli.fr/entreprise/vos-salaries/arret-de-travail/formalites-employeur
- Bulletin : https://www.legifrance.gouv.fr/codes/id/LEGISCTA000018533808 ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000048841718 ; arrêtés : https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000047096915 , https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000049831771 ; net social : https://boss.gouv.fr/portail/accueil/actualites-boss/2023/fevrier/montant-net-social.html , https://travail-emploi.gouv.fr/montant-net-social-et-bulletins-de-paie-documents-utiles
- Bulletin électronique : https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072050/LEGISCTA000006178028/ ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000033627046/ ; double employeur : https://entreprendre.service-public.gouv.fr/vosdroits/F10029
- Prélèvement à la source : https://www.impots.gouv.fr/particulier/je-consulte-le-taux-transmis-mes-collecteurs ; https://bofip.impots.gouv.fr/bofip/11166-PGP.html/identifiant=BOI-IR-PAS-30-10-20-20181203 ; https://bofip.impots.gouv.fr/bofip/11164-PGP.html/identifiant=BOI-IR-PAS-30-10-10-20181025 ; https://bofip.impots.gouv.fr/bofip/11471-PGP.html/identifiant=BOI-IR-PAS-30-10-60-20220608
- PSS 2026 (source secondaire) : https://www.centre-inffo.fr/site-droit-formation/plafond-de-la-securite-sociale-fixation-des-montants-pour-2026 ; SMIC : https://www.info.gouv.fr/actualite/le-smic-revalorise-au-1er-janvier-2026
- RGDU : https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006185779/ ; décret 2026-509 : https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054248488 ; https://www.urssaf.fr/accueil/employeur/beneficier-exonerations/reduction-generale-cotisation.html (indisponible, erreur 503, lors de la consultation)
- CCN EPNL IDCC 3218 : https://www.legifrance.gouv.fr/conv_coll/id/KALICONT000043874879 ; https://code.travail.gouv.fr/convention-collective/3218-enseignement-prive-non-lucratif-epnl ; représentativité : https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051800628
- CNIL : https://www.cnil.fr/fr/gestion-du-personnel ; https://www.cnil.fr/sites/cnil/files/atoms/files/referentiel_grh_novembre_2019_0.pdf ; https://cnil.fr/fr/referentiel-durees-conservation-donnees-rh
- **Non vérifié** : article réglementaire des dates d'échéance DSN ; CGI 1753 bis C ; grille de taux neutre 2026 ; arrêté du PSS au JO ; SMIC au 1-6-2026 ; fiche BOSS de la RGDU et de la régularisation progressive ; CRM, CTP, fiches d'organismes, notification AT/MP ; procédure éditeur détaillée de dsn-info.fr ; extension, strates, annualisation et régimes de prévoyance de la CCN EPNL ; âge exact de D3243-8 ; décret-cadre NIR.

---

## 35. Révision D-048 (2026-11-11) — paie « light » d'établissement scolaire — **l'emporte sur le §34**

**Décision** (D-048, Florian, `DECISIONS.md` de `origin/ent-mc0hoe`) : la paie reste dans ScolIA (D-047), mais **limitée à un établissement scolaire** : **une seule convention collective, ou presque**. ScolIA n'est **pas un logiciel de paie généraliste**. On retire le multi-conventions et le paramétrage générique des branches ; on garde les obligations légales.

### 35.1 Hypothèse à confirmer (Q-RH41, **soumise à Florian**)
La paie ScolIA couvre **les salariés de droit privé de l'organisme de gestion**, relevant de la **CCN de l'enseignement privé non lucratif (EPNL), IDCC 3218** (RR-42) ; **les enseignants sous contrat sont payés par l'État** et restent hors paie (§4, inchangé). Cas à vérifier avant de figer le périmètre :
- un OGEC emploie-t-il des salariés relevant d'**une autre convention** (cuisine en régie sous une autre convention, association immobilière ou diocèse employeur distinct, animateurs de pastorale) ? Si oui : **hors ScolIA** ou employeur distinct, à trancher ;
- **maîtres agréés du premier degré sous contrat simple** (Q-RH8) : rémunération par l'État ou par l'OGEC à vérifier ;
- établissement en **Alsace-Moselle** (régime local d'assurance maladie) : à exclure de la première version ou à prévoir comme seule variante.

### 35.2 Ce qui reste dû (inchangé, §34)
**Bulletin conforme** (R3243-1, modèle clarifié, net social, dématérialisation et disponibilité) ; **DSN conforme** au cahier technique de l'année, contrôlée par la brique DSN-Val, mensuelle et signalements, retours et taux de prélèvement à la source ; cotisations réelles (URSSAF, Agirc-Arrco, chômage, prévoyance et frais de santé de la branche, AT/MP), réduction générale, prélèvement à la source, taxe sur les salaires des associations ; absences, maintien de salaire CCN, subrogation ; congés payés ; sorties ; mise à jour réglementaire **annuelle** ; sécurité et droits très restreints (§34.8) ; recette en parallèle et critères K-RH (§34.9), bascule au 1-1-2029 recommandée.

### 35.3 Ce qui est retiré ou simplifié

| Élément | Décision D-048 |
|---|---|
| Référentiel CCN générique, multi-conventions (`ent_rh_ccn_categorie` avec `idcc` libre, `ent_paie_ccn_grille` multi-IDCC) | **une seule convention, EPNL 3218**, livrée et maintenue par ScolIA (classifications, minima, ancienneté, primes, maintien, prévoyance de branche) ; colonne `idcc` gardée pour la traçabilité, **valeur unique contrôlée** |
| Langage de formules ouvert, rubriques créées par l'établissement | **catalogue fermé** de rubriques livré par ScolIA (l'établissement choisit, active, saisit des montants) ; les formules restent des tables typées internes, éditées par l'équipe paie de ScolIA seulement |
| Profils de cotisation génériques | **profils limités** aux populations d'un établissement : non-cadre, cadre, apprenti, CDD (dont remplacement), temps partiel, contrat aidé si utilisé [H], stagiaire gratifié ; tout autre statut (VRP, intermittent, expatrié, journaliste, mandataire, régimes spéciaux) **hors périmètre** |
| Plusieurs employeurs et SIRET par établissement (CRH-71) | **un employeur** (l'organisme de gestion) et **un SIRET** par établissement ; un salarié partagé entre deux établissements d'un même OGEC sous deux SIRET : deux engagements, deux paies [à valider, cas rare] |
| Dépôt DSN machine à machine (Q-RH34) | **retiré** : dépôt manuel par la gestionnaire, durablement |
| DPAE générée, raccordement CPA | DPAE **attestée** (inchangé, RR-10) ; raccordement CPA **question ouverte** (Q-RH30), pas de lot |
| Simulations générales (RHP-285) | limitées à l'embauche et à l'augmentation d'un salarié de la CCN |
| Indemnités de rupture complexes (transaction, licenciement économique collectif) | indemnités légales et conventionnelles, rupture conventionnelle, fin de CDD, compensatrice de congés ; **le reste traité avec le juriste**, montant saisi |
| Saisies sur salaire, pensions (CRH-67) | conservées (obligation), barème daté |
| Comparateur parallèle multi-éditeurs (adaptateurs RH5) | **un seul adaptateur** : celui du logiciel actuel de l'OGEC pilote (Q-RH2), pour la comparaison et la reprise |

### 35.4 Lots réduits

| Lot | Contenu après D-048 |
|---|---|
| RH0 à RH10 | inchangés ; RH5 = un adaptateur |
| **RH11** | référentiel national daté **et CCN EPNL seule** |
| **RH12** | moteur, catalogue fermé, profils limités, **prélèvement à la source** (ex-RH15 fusionné) |
| **RH13** | absences, maintien, IJSS, **congés payés et annualisation** (ex-RH14 fusionné) |
| **RH16** | bulletins (inchangé) |
| **RH17** | DSN mensuelle **et signalements, retours** (ex-RH18 fusionné) |
| **RH19** | paiements, journal de paie, taxe sur les salaires **et sorties** (ex-RH20 fusionné) |
| **RH21** | veille annuelle réduite : paramètres nationaux, norme DSN, avenants EPNL |
| **RH22** | recette parallèle et bascule (inchangé) |
| ~~RH14, RH15, RH18, RH20~~ | fusionnés |

**Total** : 23 lots → **19** (RH0 à RH13, RH16, RH17, RH19, RH21, RH22).

### 35.5 Coûts revus [H, à chiffrer par Florian]
Construction RH11 → RH22 : **2 à 3 années-personne** (contre 3 à 5), toujours avec un profil **expert paie** ; maintenance : **environ 0,5 personne à temps plein** (norme DSN annuelle, campagne de fin d'année, avenants EPNL, support), avec le pic d'octobre à janvier ; revue externe annuelle conservée (Q-RH35). Le coût reste fixe quel que soit le nombre d'établissements : la réduction vient du périmètre, pas du volume.

### 35.6 Migrations
Plage **2980–2999 inchangée** ; §34.13 conservé, allégé : pas de grille multi-IDCC (contrainte `idcc = '3218'`), pas de table de règles de formule ouverte à l'établissement, pas de tables de dépôt machine à machine.

### 35.7 Questions
- **Nouvelle, soumise à Florian** : **Q-RH41** — confirmer que la paie ne couvre **que** les salariés de l'organisme de gestion sous **CCN EPNL (IDCC 3218)**, et dire si l'OGEC pilote a des salariés sous une autre convention, des maîtres sous contrat simple payés par lui, ou un site en Alsace-Moselle (§35.1).
- **Retirée** : Q-RH34 (dépôt machine à machine).
- **Restent** : Q-RH5 (juriste, indispensable avant RH12), Q-RH26 à Q-RH33, Q-RH35 à Q-RH38.
