# Socle 9 — Interopérabilité nationale et identité fédérée

- **Date :** 2026-11-08 — **Auteur :** session de conception (Opus), branche `ent-socle-interop-conception` (partie de `ent-mc0hoe`, documentation ; code lu sur `ent-integration-lot8b`, état le plus récent).
- **Statut :** **conception, non décidée.** Relecture indépendante intégrée (§30 : 6 objections hautes, 10 moyennes, 2 basses, toutes traitées). **[ajout personas]** Passe personas du 2026-11-08 (`PERSONAS_09.md`, 16 personas, 92 manques M-405 à M-496, 7 bloquants regroupés en 4 points) intégrée sans suppression : ajouts marqués « [ajout personas] » aux §0, §6.12, §9.12, §10.6, §12, §13, §18.3, §25.4, §26, §27, §28. Aucun code modifié. Aucune base, aucun secret, aucun service de production consulté ; l'application Next.js n'a pas été ouverte (seuls les noms de fichiers déjà cités par `02_ELEVE.md` et `04_ADMINISTRATIF.md` sont repris, **aucune copie de code**).
- **À lire avec :** `CLAUDE.md` ; `docs/VISION.md` ; `docs/DECISIONS.md` (D-009, D-012, D-013, D-017, D-019 à D-028) ; `docs/SOCLES.md` (socle 9, et 2 à 8, 10, 22, 23, 25, 37, 38, 48, 50) ; `docs/MATRICE_CHARLEMAGNE.md` (ELE-01, ELE-22, INS-16, EDT-04, NOT-09/10/16, ORI-02, PER-18, CPT-07, PIL-09, PIL-20, T-26, T-28) ; `docs/INTEROPERABILITE_NATIONALE.md` (étude du 2026-11-05, **reprise et prolongée ici, pas remplacée** : ses sources et son §16 restent la référence documentaire) ; `docs/socles/02_ELEVE.md` (§7.3 identifiants, §7.8 propositions, §8, lot E5) ; `03_INSCRIPTIONS.md` (§5, élèves affectés) ; `04_ADMINISTRATIF.md` (§5.3 ports, §8.10–8.11) ; `05_STRUCTURE.md` (§9.4 nomenclatures, §17 échanges) ; `06_PERSONNELS.md` (§8.11, PRS-110/111) ; `07_DOCUMENTS.md` (stockage, analyse, conservation) ; `08_NOTIFICATIONS.md` (le socle 9 ne prévient personne lui-même) ; `IA_COEUR.md` ; `ARCHITECTURE_BRIQUES.md` (branche `ent-architecture-briques`).
- **Plan :** même plan que `02` à `08` : §0 En bref · §1 Thèse · §2 Évaluation franche de l'existant · §3 Acteurs · §4 L'année des échanges · §5 Frontières et briques · §6 Le moteur d'échanges · §7 Concepts · §8 Cycles de vie · §9 Catalogue des flux · §10 Identité fédérée · §11 GAR · §12 Cas limites · §13 Actions métier · §14 Logiciel ou paramétrage · §15 Modèle de données · §16 Sécurité · §17 RGPD et conservation · §18 Droits · §19 Événements · §20 Contrat IA · §21 Écrans · §22 Mobile · §23 Tests · §24 Plan de migration · §25 Lots de code · §26 Démarches hors code · §27 Questions · §28 Faits, hypothèses, contradictions · §29 Sources · §30 Relecture indépendante.
- **Plage de migrations : 3900–3999** (règle « 3000 + 100 × numéro de socle », Q-B2 d'`ARCHITECTURE_BRIQUES.md`) ; **repli 420–439** si Q-B2 n'est pas adoptée. Inscrite dans `docs/socles/MIGRATIONS.md`.
- **Conventions.** **[F]** fait vérifié dans le dépôt (chemin:ligne, état `ent-integration-lot8b` au 2026-11-08) ; **[S]** fait sourcé à l'extérieur, lu **en extrait de moteur de recherche seulement** (les sites officiels — Légifrance, education.gouv.fr, eduscol, gar.education.fr, educonnect, data.gouv.fr, insee.fr — **refusent la connexion depuis la session**, vérifié le 2026-11-08 : `connect_rejected` sur 7 hôtes) ; **[S-tiers]** documentation d'un éditeur ou d'un site non officiel ; **[H]** hypothèse (mémoire ou déduction) ; **[NV]** format, règle ou procédure **non vérifiée sur un texte officiel : interdit de coder sur cette base** ; **[R]** recommandation (non décidée) ; **[Q-Xn]** renvoi à une question du §27. **[V]** (passe du 2026-11-09, `VERIF_09_SOURCES.md`) : texte officiel **ouvert et lu**, avec l'URL et le numéro de ligne du rapport (`V#n`) ; seule cette marque autorise à coder. Les marques [S] et [NV] qui subsistent ont été recherchées le 2026-11-09 sans source lisible. Identifiants nouveaux et stables : actions **`ITP-xx`**, flux **`FX-xx`**, règles **`R-Xn`**, cas limites **`CX-nn`**, lots **`X0`–`X19`**, questions **`Q-Xn`** (préfixes vérifiés libres : `grep -rE 'ITP-[0-9]|R-X[0-9]|CX-[0-9]|Q-X[0-9]|\bX[0-9]{1,2}\b' docs ent-rust` → 0 occurrence). Les lots `IOP-L0`–`IOP-L12` de l'étude sont **remplacés** par les lots `X` (table de correspondance au §25.3).
- **Méthode.** Le code d'import, d'export et de fédération (migrations 060, 090, 110, 144, 281, 283 ; `ent-db/src/{imports,oidc_repo}.rs` ; `domain-eleve/src/{ine,lsun_export,lsl_export,notanet_export,onde_export,parcoursup_export,evaluation}.rs` ; `ent-api/src/{ical,routes}.rs`) a été lu directement. Six sous-agents Haiku ont cherché, en parallèle, les formats et procédures (SIECLE/BEE/ONDE/OMOGEN, STS-web/BCN/COG/EDT, ÉduConnect/GAR/AAF, LSU/LSL/Parcoursup/Affelnet/DNB, privé/comptabilité/annuaires, cadre réglementaire 2026) ; **aucune page officielle n'a pu être ouverte**, d'où l'abondance de marques [S] (extrait) et [NV]. Les exigences adressées au socle 9 par les conceptions 02 à 08 ont été relevées une à une (§5.4). La réflexion et les choix sont ceux de ce document.

---

## 0. En bref

1. **Le constat [F].** Le Rust a un embryon solide mais isolé : un **moteur d'import d'élèves** (migration 144, `imports.rs`) qui prévisualise, rejette ligne à ligne et n'écrase jamais un élève existant — mais qui ne lit qu'un CSV maison (`scolia_eleves_v1`), **écrit directement** dans `ent_eleve` et `ent_eleve_lien` (contraire à D-021 et à D-026, `ARCHITECTURE_BRIQUES.md` §1.5), reclasse les lignes **hors** de la transaction d'application (`03_INSCRIPTIONS.md` §2.2 point 19) et ne connaît qu'un seul type d'entité ; un **journal d'import d'EDT** à part (060) ; des **identifiants externes de l'élève** bien conçus (281 : statut, provenance, unicité par établissement seulement) ; un **SSO OIDC** réutilisable mais qui rattache par e-mail vérifié (inutilisable pour ÉduConnect) ; cinq **exports « officiels » qui n'en sont pas** (CSV maison pour LSU, LSL, Notanet, ONDE, Parcoursup) et un **XML LSU inventé**, avec un espace de noms d'allure officielle (`urn:fr:edu:scolarite:lsun:bilans:import`, `domain-eleve/tests/fixtures/lsu_reference_scolia.xml:3`) qu'un établissement pourrait croire valide ; un **export iCal** qui exige une session (`routes.rs:618-620`), donc impossible à abonner depuis un agenda de téléphone. Aucune UAI vérifiée, aucune nomenclature nationale chargée, aucune fédération nationale, aucun retour d'une application nationale traité.
2. **La thèse.** **Interopérer, ce n'est pas copier des fichiers : c'est traduire et réconcilier.** Les applications nationales et ScolIA ont **chacune** raison sur une partie des données, se trompent parfois, et changent à des rythmes différents. Le socle 9 est le **traducteur** (formats, codes, identifiants) et l'**arbitre loyal** (qui fait foi, pour quel champ, à quelle date) — jamais un tuyau qui écrase.
3. **Rien de l'extérieur n'écrit en silence (principe du socle 2, généralisé).** Tout ce qui entre devient des **propositions** déposées dans le domaine qui possède la donnée (élève, personnes, structure, personnels, EDT, finances…), avec la **preuve** (lot, fichier, ligne, empreinte) et la **source**. Une proposition s'applique seule **uniquement** par une règle explicite et tracée (« SIECLE fait foi pour la date de naissance et rien n'a été corrigé ici »). Le socle 9 n'a **aucune** écriture dans les tables d'un autre domaine (garde statique de D-026).
4. **La mémoire de la source et la différence à trois voies — le cœur du « mieux que l'existant ».** Pour chaque champ reçu, le moteur garde l'**empreinte** de la dernière valeur envoyée par cette source. Il compare alors trois valeurs : *ce que la source disait*, *ce que ScolIA contient*, *ce que la source dit maintenant*. Il sait ainsi distinguer **la source a changé** (proposer, ou appliquer si la règle le permet), **l'établissement a corrigé et la source n'a pas bougé** (ne rien proposer, mais signaler l'écart à corriger **à la source**), et **les deux ont changé** (vrai conflit, arbitrage humain). C'est ce qui permet de ne **jamais écraser une correction locale**, sans pour autant **redemander chaque nuit** la même décision (§6.6).
5. **Un moteur, des connecteurs déclarés.** Un seul moteur (réception, garde-fous, lecture, normalisation, rapprochement, différences, propositions, application, annulation, rapport ; et pour les sorties : pré-contrôle, instantané figé, génération, dépôt, accusé, retour). Chaque flux est un **connecteur** : une déclaration (sens, format et versions acceptées, entités produites, champs, règles de rapprochement, garde-fous, droits, conservation) et un lecteur ou un générateur pur, testé sur des **jeux d'or** anonymisés. Un format inconnu ou une version non déclarée est **refusé** avec un message clair : on ne devine jamais un format (D-019).
6. **Garde-fous d'identité et de volume.** Avant toute analyse : le fichier porte-t-il **notre** UAI ? la **bonne année** ? est-il **plus ancien** que le précédent ? Après l'analyse : si plus de X % des élèves changent d'état civil ou disparaissent, le lot est **bloqué** et demande une confirmation renforcée (★). C'est la parade contre l'incident le plus grave et le plus banal : charger le fichier d'un autre établissement ou de l'an dernier.
7. **Sorties : on prépare, on contrôle, on fige, on prouve.** Chaque export officiel commence par un **pré-contrôle** lisible (« 12 élèves sans INE confirmé, 3 matières sans code national — corriger ») avec un lien vers l'écran qui corrige (trou T-28 de la matrice) ; il est produit **depuis les seules données confirmées**, **figé** (instantané, empreinte, régénération identique), **tracé** (qui, quand, quelles personnes, vers quel destinataire — base de la réponse RGPD « à qui mes données ont-elles été transmises ? ») et **bouclé** : le compte rendu de l'application nationale (rejets LSU, BEE, GAR) est réimporté et chaque rejet devient une tâche de correction rattachée au bon dossier.
8. **Identité fédérée.** ÉduConnect (élèves, responsables) et guichets Agents (personnels) sont **exigés pour les collèges et lycées publics** par l'arrêté du 7 septembre 2026 (art. 3, services socles ; annexe 4 **critères n° 4** ÉduConnect, **n° 5** Agents, **n° 7** GAR, **n° 15** AAF, **n° 18** protocoles ouverts ; pour un « service outillant les missions de vie scolaire » : délais de **10, 22, 22, 34 et 10 mois** après le 23/09/2026 [V, `VERIF_09_SOURCES.md` V#1–4] ; la mention antérieure « exigence n° 6 / n° 15 (GAR) » était **fausse**). Le privé sous contrat n'est **pas** visé par l'arrêté [V#1] ; ÉduConnect y est sur volontariat [S]. Règles : un **guichet central** `connexion.scolia.fr` (un seul fournisseur de service déclaré, compatible avec un sous-domaine par établissement, D-012) qui remet un ticket à usage unique au sous-domaine choisi ; **rattachement par clé de jointure** issue d'un import officiel, ou par **code de rattachement** remis par l'établissement — **jamais par e-mail ni par nom** ; **aucune création de personne** à la connexion ; la fédération **authentifie**, elle n'**autorise** rien (droits D-017 inchangés). ScolIA devient aussi **fournisseur d'identité** pour le GAR (OIDC de préférence si le contrat 8.2 l'admet [S, titre]), avec des identifiants **opaques par service**, jamais l'INE.
9. **Faisable dès maintenant, sans convention ni agrément.** Le **parcours minimal vendable** (lots X0–X7 ; SIECLE et STS-web conditionnés à des fichiers réels anonymisés fournis par un pilote, D-14) permet à un établissement public ou privé de **déposer lui-même** ses exports SIECLE et STS-web, ses tableurs et ses anciens fichiers, de les réconcilier sans risque, de charger les nomenclatures et le COG, et de **pré-contrôler** ses exports. Tout ce qui exige une convention, un agrément ou une spécification en diffusion restreinte (AAF, ÉduConnect, Agents, GAR, remontée BEE, LSU/LSL/Parcoursup officiels, ONDE) est un **lot bloqué** qui ne démarre qu'à réception du document (§26) : **le chemin critique de ce socle n'est pas le code, ce sont les démarches**, à lancer maintenant.
10. **Le plan.** 22 lots principaux (X0 correctifs immédiats → X19 conformité, X2 scindé en X2a–c), 8 lots annexes, 57 fiches de flux, 118 actions `ITP-xx`, 15 droits, 21 événements émis, 16 démarches hors code, 32 questions. Migrations **3900–3999** (3900–3914 attribuées). Cinq décisions à prendre en premier : Q-X1 (passkey et fédération nationale), Q-X2 (où vit la fédération), Q-X3 (guichet central), Q-X8 (mémoire de la source), Q-X14 (remontée BEE pour le privé).
11. **[ajout personas] Ce que la passe personas change.** (A) La **première année** d'un lycée privé qui quitte Charlemagne n'a pas de chemin : LSU, LSL et Parcoursup passent aussi par un logiciel validé et Parcoursup exige les bulletins de **N-1** → stratégie de bascule par module (Q-X34), profil de cohabitation et **journal « à reporter dans le logiciel agréé »** (FX-107, R-X24), reprise structurée de N-1 (FX-106). (B) Le **rattachement de rentrée** ne peut pas exiger 1 300 validations à la main : confirmation automatique sur chaîne officielle complète, humaine sur discordance, et **aucune nouvelle jointure** après une révocation pour protection (Q-X35, R-X19). (C) Dans le public, ScolIA sera souvent un **service raccordé à l'ENT de la collectivité**, pas l'ENT (Q-X33, FX-99, lot X8 bis). (D) Les **mandats SEPA** et les soldes doivent être repris avant la première remise (FX-76). Puis : garde-fou de volume conscient de la bascule (R-X20), dépôt manuel déclaré et export périmé (R-X21), attestation et journal scellé (R-X22), révocation générale et coupe-circuit (R-X25), aperçu « ce qui part pour la première fois » (R-X17), élèves protégés exclus des flux tiers (R-X18). Cinq décisions s'ajoutent en tête : **Q-X34, Q-X35, Q-X33**, puis Q-X37 (ensemble scolaire = un tenant) et Q-X38 (export de structure par un professeur, D-022).

---

## 1. Thèse : ce que veut dire « interopérer »

**Ce que vivent les établissements aujourd'hui [H, témoignages courants, expérience de Florian au secrétariat d'un lycée, `PERSONAS_02-04.md`, `PERSONAS_05-06.md`].** La rentrée, c'est **ressaisir** : les fiches papier dans le logiciel de gestion, puis dans SIECLE (privé) ; les services dans un tableur, puis dans le logiciel d'emploi du temps, puis dans STS-web. Un import « met à jour » et **efface en silence** l'adresse corrigée la veille au guichet ; ou au contraire refuse tout dès qu'une ligne est fausse. On découvre en mars, la veille de la remontée Parcoursup, que trois élèves n'ont pas d'INE et qu'une matière n'a pas de code. On reçoit un compte rendu de rejet LSU incompréhensible (« élément inattendu ligne 4 512 »). Un parent ne peut pas se connecter « parce que son adresse SIECLE n'est pas la bonne ». Personne ne sait dire si le fichier chargé ce matin était celui de cette année.

**Ce que vivent les familles et les personnels [H].** Le parent qui a deux enfants dans deux établissements a deux comptes, puis trois avec ÉduConnect ; le professeur a un mot de passe par outil. L'élève qui a changé de nom d'usage voit l'ancien réapparaître après chaque import. Le parent séparé découvre que le second foyer a été « fusionné » dans le sien par un import qui rapprochait par adresse.

**Ce que vit l'administration [H].** Des remontées tardives, incomplètes ou fausses (effectifs du constat de rentrée dans le privé, notes Parcoursup), des éditeurs qui interprètent les formats différemment.

**Ce qui en découle, en sept principes [R] :**

1. **Chaque donnée a un propriétaire dans ScolIA et une ou plusieurs sources.** Le propriétaire (socle 2 pour l'élève, 4 pour les personnes, 5 pour la structure, 6 pour les personnels…) décide ; les sources (SIECLE, STS, famille, ancien logiciel…) **proposent**. Le socle 9 ne possède que ses lots, ses connecteurs, ses traces et ses correspondances.
2. **Qui fait foi est explicite, champ par champ, et dépend du type d'établissement.** Dans un collège public, SIECLE fait foi pour l'état civil et la structure (c'est la base de l'administration) ; dans un lycée privé sous contrat qui remonte vers BEE, **ScolIA** fait foi et SIECLE n'est qu'un retour ; dans un hors contrat, il n'y a pas de source nationale. Le paramétrage existe déjà au socle 2 (`ent_param_eleve_champ.source_qui_fait_foi`, `02_ELEVE.md` §7.8) et au socle 4 (§10) : le socle 9 le **lit**, il ne le duplique pas.
3. **Une correction locale n'est jamais écrasée, et jamais redemandée pour rien** (mémoire de la source, §6.6).
4. **Un format se vérifie, il ne se devine pas.** Chaque connecteur déclare les versions qu'il comprend ; tout le reste est refusé. Les lots bloqués attendent la spécification.
5. **Ce qui sort est préparé, contrôlé, figé, tracé, bouclé** (§6.10–6.11).
6. **L'identité fédérée authentifie, ScolIA autorise.** Aucune donnée d'un fournisseur d'identité ne crée une personne, un rôle ou un lien familial.
7. **Le cloisonnement passe avant la commodité.** Un fichier appartient à un établissement ; une jointure ÉduConnect ne vaut que dans les établissements où la personne a été importée ; un identifiant transmis à un service tiers est propre à ce service (D-013).

---

## 2. Évaluation franche de l'existant

### 2.1 Ce qui tient (à garder)

| Élément | Pourquoi c'est bien | Où |
|---|---|---|
| Moteur d'import : prévisualisation sans écriture, lignes numérotées, statuts `a_creer` / `existant` / `rejet`, motifs par ligne, plafond 5 000 lignes et 2 Mo, verrou `FOR UPDATE` sur le lot à l'application | La bonne forme ; base de X1–X2 | `144_import_lot.sql` ; `imports.rs:56-57, 636, 742` |
| « Un élève déjà présent n'est jamais modifié » | Principe juste (A18) ; à généraliser par les propositions | `imports.rs:10-14` |
| Rapprochement INE puis identité (nom, prénom, date de naissance) | Ordre juste ; à compléter par les identifiants SIECLE et la mémoire de correspondance | `imports.rs` |
| Identifiants externes de l'élève : référentiel des systèmes (`INE`, `SIECLE_ELEVE`, `ONDE`, `AAF_JOINTURE`, `BADGE`, `ANCIEN_LOGICIEL`), statut (`declare` → `confirme` → `conteste` → `revoque`), provenance (`import_aaf`, `import_bee`, `retour_bee`, `import_onde`, `educonnect`…), unicité **dans l'établissement seulement** | Exactement le modèle attendu par l'étude ; D-013 respectée | `281_eleve_identifiants.sql` |
| Origine et destination d'une scolarisation par UAI | Utile aux transferts et aux exports | `283_eleve_scolarisation.sql:67-98` |
| Site porteur d'une UAI | Juste pour les ensembles scolaires (plusieurs UAI sous un tenant) | `110_referentiel_structure.sql:14-15` |
| MEF à 11 caractères, MEFSTAT11, import MEF avec rejets | Compatible avec les nomenclatures [S] | `05_STRUCTURE.md` §2.1, `110:8-66` |
| SSO OIDC : PKCE, RS256, JWKS par `kid`, points d'accès stockés (la page de connexion ne dépend pas de l'IdP), secret jamais exposé au portail, rattachement `(provider, subject)` stable | Base saine pour la fédération | `090_oidc.sql` ; `ent-oidc` ; `oidc_repo.rs` |
| Fichier LSU de référence **marqué [NV]** dans son en-tête | Honnête sur son statut | `lsu_reference_scolia.xml:2` |
| Export iCal RFC 5545, plage limitée à 62 jours | Format juste, borne raisonnable | `ical.rs`, `routes.rs:8324` |

### 2.2 Ce qui est à revoir

| # | Constat | Conséquence | Où | Traitement |
|---|---|---|---|---|
| 1 | L'import **écrit** `INSERT INTO ent_eleve`, `ent_eleve_lien` | Contourne les commandes du socle 2 (identité versionnée, provenance) et du socle 4 ; contraire à D-021/D-026 | `imports.rs:508, 593` ; `ARCHITECTURE_BRIQUES.md` §1.5 | X0 puis X2 (commandes et propositions) |
| 2 | Reclassement des lignes **hors** de la transaction d'application | Deux lignes identiques corrigées peuvent créer deux élèves | `03_INSCRIPTIONS.md` §2.2 point 19 | X0 |
| 3 | Aucun événement pour les personnes et liens créés par l'import ; le second responsable devient payeur si le premier est absent | Les autres domaines ignorent l'arrivée ; règle de payeur implicite | idem | X2 (les commandes publient) |
| 4 | Un seul format (`scolia_eleves_v1`), un seul type d'entité ; `format` contraint par `CHECK` | Chaque nouveau flux exigerait une migration de contrainte et un nouveau moteur | `144:15` | X1 (connecteurs déclarés) |
| 5 | Import d'EDT dans une table à part, sans lien avec le moteur | Deux moteurs, deux comportements | `060_edt_import.sql` | X16 |
| 6 | Cinq exports « nationaux » en **CSV maison** (LSU, LSL, Notanet, ONDE, Parcoursup) | Inutilisables par les applications nationales ; risque de faire croire à une conformité | `domain-eleve/src/*_export.rs` (en-têtes « P1 partiel », « P2 stub ») | X0 : renommés « export de travail (non officiel) » à l'écran ; formats officiels dans les socles 22, 23, 37, 38, 50 sur le moteur X7 |
| 7 | **XML LSU inventé** avec un espace de noms d'allure officielle `urn:fr:edu:scolarite:lsun:bilans:import` et `schemaVersion="NV"` | Un fichier qui *ressemble* à un format ministériel ; si un établissement le dépose, rejet au mieux, données mal interprétées au pire | `lsu_reference_scolia.xml:3` ; `evaluation.rs:952-962` | X0 : espace de noms `urn:scolia:…` explicitement non officiel, ou retrait ; jamais proposé au téléchargement comme « LSU » |
| 8 | INE accepté de 9 à 16 caractères | Trop large ; mais **ne pas coder** de clé de contrôle de mémoire (aucune source officielle trouvée, §28) | `ine.rs:7-20` | X5 : format resserré **seulement** sur texte officiel ; d'ici là, alerte « format inhabituel » sans blocage. **Passe 2026-11-09** : 11 caractères (10 chiffres + 1 lettre, ou 9 chiffres + 2 lettres depuis 2018 [S-tiers]) ; **aucun algorithme de clé publié** trouvé (V#26) → alerte si ≠ 11 alphanumériques, jamais de clé |
| 9 | UAI sans contrôle de format (`'^[0-9A-Za-z]{1,20}$'`) | Valeurs fantaisistes en origine et destination | `283:97-98` | X4 : format « 7 chiffres + 1 lettre » [S, extrait OSM et académies] signalé, contrôle de la lettre **NV** (recherché le 2026-11-09, aucun algorithme officiel trouvé, V#27) → contrôle par **présence dans l'annuaire de l'éducation** [V#29], pas par calcul |
| 10 | SSO : première connexion rattachée par **e-mail vérifié** | Inapplicable à ÉduConnect (élèves sans e-mail, e-mail non garanti) ; dangereux pour les responsables | `service.rs:3010` (cité par l'étude §4.3) | X8 : rattachement par jointure ou code |
| 11 | Secret client OIDC stocké en base dans `client_secret`, **chiffré par le service** (« rechiffrement d'un secret hérité », `oidc_repo.rs:168-169`), jamais exposé (`ent-api/src/lib.rs:2426`) | Acceptable pour l'OIDC d'établissement ; pour les secrets des canaux du socle 9 (SFTP, certificats NetSynchro, clés de l'IdP), un **coffre** est préférable (rotation, accès journalisé) | `090:19` | X8 : décider avec le socle 1 d'un même mécanisme (coffre ou chiffrement applicatif) pour tous les secrets de fédération et de canaux |
| 12 | Export iCal derrière la session | Aucun abonnement possible depuis un agenda (Google, Apple, Outlook) | `routes.rs:618-620` | X15 : jetons d'abonnement révocables |
| 13 | Tests d'intégration silencieux sans base (`ENT_DATABASE_URL` absent → retour sans rien vérifier) | Fausse assurance (déjà relevé aux socles 3 et 5) | `03_INSCRIPTIONS.md` §2.2 point 13 | règle du kit (§23) |

### 2.3 Ce qui manque entièrement

Lecture de **tout** format national (SIECLE BEE, STS-web, AAF, ONDE, GAR, retours LSU) ; nomenclatures BCN, COG, annuaire des UAI, calendrier scolaire officiel ; mémoire de la source et différences à trois voies ; garde-fous d'identité du fichier (UAI, année, ancienneté) et de volume ; annulation par compensation ; flux automatiques planifiés et surveillance (« l'AAF n'est pas arrivé cette nuit ») ; **réception centrale** multi-établissements ; pré-contrôle des exports, instantanés figés, traces d'export, retours bouclés ; ÉduConnect, guichet Agents, guichet central de fédération, codes de rattachement ; ScolIA fournisseur d'identité ; alimentation GAR et médiacentre ; annuaires sortants (Google, Microsoft, Apple) ; abonnements de calendrier ; imports des logiciels d'emploi du temps du marché ; échanges bancaires et comptables ; flux de l'enseignement catholique, du hors contrat et de l'agricole ; paquet de réversibilité ; tableau de bord des échanges et calendrier de l'année.

### 2.4 Verdict

L'existant a **la bonne intuition** (prévisualiser, ne pas écraser, identifiants avec provenance) et **la mauvaise place** (un import qui écrit chez les autres, des exports qui se donnent un air officiel). Le socle 9 garde l'intuition, la généralise en un moteur à connecteurs, et la remet à sa place : **un traducteur qui propose, au service des domaines qui décident**. Pour la fédération, `ent-oidc` est une bonne fondation à compléter par un rattachement sûr, un guichet central et, si les fournisseurs nationaux l'imposent, une brique SAML isolée.

---

## 3. Acteurs et ce qu'ils vivent

| Acteur | Ce qu'il fait des échanges | Ce qui lui fait perdre du temps / ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|
| **Secrétariat de scolarité** | Exporte de SIECLE, importe ; prépare les remontées (BEE dans le privé, LSU, LSL, Parcoursup) ; traite les rejets | Ressaisie ; import qui écrase ; rejets incompréhensibles ; découvrir un manque la veille d'une échéance | « Déposer, relire en dix minutes, valider » ; **état de préparation** des exports des semaines avant ; rejets traduits en tâches | quotidienne à la rentrée, puis hebdomadaire ; pics juin, septembre, mars |
| **Chef d'établissement, adjoint** | Responsable de traitement ; signe les remontées ; prépare la rentrée (structure, services) ; responsable de la conformité dans le public | Être en faute (données fausses transmises, fichier d'un autre établissement chargé) ; ne pas savoir ce qui est parti où | Un tableau de bord des flux et des échéances ; preuve de ce qui a été transmis ; confirmation renforcée pour l'irréversible | hebdomadaire |
| **Responsable de l'emploi du temps** | Échange structure, cours, services avec le logiciel d'EDT ; services vers STS-web | Noms de groupes qui ne correspondent pas ; barrettes perdues ; STS qui renomme les groupes [S, académie de Lyon] | Correspondance mémorisée une fois pour toutes ; aperçu des écarts | juin à septembre |
| **Comptable, gestionnaire (privé : OGEC ; public : adjoint gestionnaire)** | Relevés bancaires, prélèvements, export vers le logiciel comptable ou l'expert-comptable ; dans le public, droits constatés vers l'agent comptable | Pointage à la main ; doubles saisies ; formats propriétaires | Relevés qui se rapprochent seuls, avec proposition ; FEC propre | mensuelle |
| **Documentaliste, référent numérique** | Responsable d'affectation des ressources dans la console du GAR [S] ; abonnements | Élèves absents du médiacentre en septembre | Médiacentre juste dès le premier jour | rentrée |
| **Administrateur de l'établissement (ou du groupe scolaire)** | Active les connecteurs, les fournisseurs d'identité, les annuaires (Google, Microsoft) | Configuration obscure ; secrets dans des courriels | Un assistant par connecteur ; secrets jamais affichés ; essai sans risque | annuelle |
| **Professeur** | Se connecte (guichet Agents dans le public) ; abonne son agenda à son emploi du temps ; ouvre des ressources du médiacentre | Mots de passe multiples ; agenda faux | Une connexion ; un agenda qui se met à jour seul | quotidienne |
| **Responsable légal** | Se connecte (ÉduConnect dans le public, compte ScolIA ailleurs) ; reçoit l'emploi du temps et les devoirs dans son agenda | « Mon compte ne marche pas » ; deux comptes ; données de l'autre parent visibles | Une identité pour tous ses enfants, sans fuite entre établissements (D-013) | quotidienne |
| **Élève (mineur, majeur)** | Se connecte (ÉduConnect), médiacentre, agenda | Compte bloqué en septembre ; ancien nom réapparu | Accès dès le jour de la rentrée ; prénom d'usage respecté | quotidienne |
| **DSI académique, DSDEN, rectorat (public)** | Conventions AAF, raccordements ; reçoit remontées | Éditeurs mal conformes | Un éditeur prévisible, qui respecte les spécifications | annuelle |
| **Direction diocésaine, UDOGEC, SGEC (privé catholique)** | Statistiques, annuaire (« Gabriel » [S, documents 2013–2019]), cotisations calculées sur les effectifs [H] | Chiffres faux ou tardifs | Exports d'effectifs justes et datés | annuelle |
| **Organisme de gestion (OGEC)** | Comptabilité, budget (socle 48) | Écart entre logiciel de gestion et comptabilité | Écritures générées, traçables | mensuelle |
| **Partenaires** (restauration, transport, CDI, plateformes de ressources) | Reçoivent des listes, renvoient des passages | Fichiers Excel envoyés par courriel | API restreinte, consentie, journalisée | quotidienne |
| **Opérateur ScolIA (support)** | Charge les nomenclatures globales ; surveille les flux | Être tenté de regarder des données d'élèves | Outils qui ne montrent **aucune** donnée personnelle (D-013) | continue |

---

## 4. L'année des échanges

Calendrier **indicatif**, reconstitué d'extraits académiques et d'éditeurs [S] ; les dates varient par académie et par année [NV]. Il sert à deux choses : planifier les rappels d'**état de préparation** (§6.10) et vérifier qu'aucun lot n'oublie un moment de l'année.

| Période | Public (collège, lycée) | Privé sous contrat | Hors contrat | Flux |
|---|---|---|---|---|
| **Janvier – mars** | Parcoursup : remontée des bulletins par webservice (dates limites citées par des académies : début à fin mars [S, Lille, Paris, extraits]) | idem | candidats sans remontée [H] | FX-35 |
| **Février – mai** | Préparation de rentrée : dotation, structure prévisionnelle ; Affelnet 6ᵉ (depuis le premier degré) | idem ; inscriptions et réinscriptions (décembre–mars, `03_INSCRIPTIONS.md` §1) | inscriptions | FX-05, socle 3 |
| **Mai – juin** | Affelnet lycée : vœux des familles (téléservices, compte ÉduConnect [S]), évaluations transmises **depuis LSU** [S, extrait] ; LSU : bilans de fin d'année et de fin de cycle (remontée citée au 18 juin dans une présentation syndicale [S, NV]) ; LSL : livrets de première et terminale ; DNB : notes de contrôle continu (réforme 2026 contradictoire selon les sources, §28) ; STS-web : préparation des services | idem (éditeur validé) ; remontée BEE des changements | DNB en candidat libre [S, justice.fr] | FX-33, 34, 36, 37 |
| **Juillet** | Élèves affectés (Affelnet) à inscrire ; bascule d'année dans SIECLE et STS (à l'ouverture des campagnes [S, Bordeaux]) | idem | — | FX-05, FX-02 |
| **Fin août** | Import structure et élèves (SIECLE, AAF) ; EDT → STS-web (export fin août, ajustements jusqu'à fin septembre [S, DANE Lyon]) ; GAR : alimentation de rentrée | import BEE ; logiciel → BEE | tableurs, reprise | FX-01, 02, 03, 32, 39 |
| **Septembre** | AAF quotidien ; ÉduConnect : activation des comptes des familles ; GAR quotidien ; validation des services dans STS-web | **constat de rentrée** (effectifs, base du forfait d'externat [S, Nancy-Metz ; L. 442-9]) par la remontée BEE ; ÉduConnect si l'établissement l'a choisi | déclaration des effectifs [NV] | FX-01, 30, 39 ; ÉduConnect (§10.3) |
| **Octobre – décembre** | Mises à jour quotidiennes (arrivées, départs) ; fin du premier trimestre : bulletins | idem ; enquêtes diocésaines [H] | — | FX-01, 02, 62 |
| **Toute l'année** | Relevés bancaires, prélèvements (privé), droits constatés (public, vers l'agent comptable), calendriers, provisionnement des comptes Google/Microsoft, transferts d'élèves (exeat), comptes rendus des applications nationales | idem | idem | FX-70, 71, 74, 85, 90–93, 100, 08 |

---

## 5. Frontières et place dans l'architecture en briques

### 5.1 Qui possède quoi

| Sujet | Socle 9 | Autres socles |
|---|---|---|
| Moteur d'échanges (lots, connecteurs, garde-fous, rapprochement, mémoire de la source, différences, annulation, exports figés, traces, retours) | **possède** | utilisent son contrat |
| Données reçues une fois appliquées (identité de l'élève, personnes, structure, personnels, créneaux, écritures) | ne possède **rien** | 2, 4, 5, 6, 10, 24/25, 48 possèdent et décident |
| Propositions (file, décision, application) | **dépose** et suit | 2 (`ent_eleve_proposition`), 4 (`ent_dossier_proposition`), 5, 6, 10 possèdent la file et la décision ; leurs commandes appliquent |
| Identifiants externes (INE, SIECLE, ONDE, STS, AAF_JOINTURE) | **alimente** (provenance `import_*`) | 2 (élève, 281), 4 (personne), 6 (agent) possèdent |
| Identifiants **de service** opaques (GAR, annuaires sortants, partenaires) | **possède** (ils n'ont de sens que pour le flux) | — |
| Nomenclatures globales (BCN, COG, annuaire des UAI, calendrier officiel, jours fériés) | **possède** les tables globales et leur chargement (lot X4, demandé par `05_STRUCTURE.md` §9.4) | 5 adopte et référence ; 2, 4 utilisent (COG pour la naissance et les adresses) |
| Contenu métier des exports officiels (LSU, LSL, Parcoursup, Affelnet, DNB, Cyclades, ONDE, FEC, SEPA) | **fiche du flux** (§9), générateur commun, pré-contrôle, instantané, dépôt, retour | 22, 23, 37, 38, 50, 25, 48 **produisent** le contenu par le contrat d'export (§6.10) — conforme à `SOCLES.md` §9 « Ne contient pas » |
| Fédération entrante (ÉduConnect, Agents, OIDC d'établissement) | **conçoit** (règles, rattachement, guichet central) | **1** (sécurité) l'exécute dans le noyau : seule la sécurité crée une session [R, Q-X2] |
| ScolIA fournisseur d'identité (GAR, puis Google/Microsoft, partenaires) | **possède** (brique `federation-idp`) | 1 fournit l'`Acteur` et l'assurance |
| Notifications (« votre import a 3 conflits », « l'AAF n'est pas arrivé ») | **déclare** ses natures d'avis | **8** décide qui, comment, quand (`08_NOTIFICATIONS.md` §6) |
| Fichiers reçus et générés (stockage, analyse antivirus, conservation) | demande | **7** stocke, analyse, applique la conservation (`07_DOCUMENTS.md`) ; noyau plateforme signe les liens |
| Reprise d'un ancien logiciel à la mise en service | **fournit** les connecteurs et le moteur | **28** pilote la mise en service |
| Réversibilité, export complet d'un dossier (T-26), portabilité | **fournit** le format et le générateur | **27** décide quand et pour qui |
| Abonnements de calendrier | **possède** les jetons et le rendu iCal | 10 (séances), 19 (événements), 20 (travail) fournissent les lectures |

### 5.2 Règles de frontière [R]

1. **Le socle 9 n'écrit dans aucune table d'un autre domaine.** Il appelle des **ports de proposition** (`proposer(tx, acteur, proposition)`) et, pour une application automatique autorisée par une règle, la **commande** du domaine avec la proposition en `source_ref`. Garde statique de D-026 (`ARCHITECTURE_BRIQUES.md` §4.6).
2. **Un domaine qui reçoit des données de l'extérieur expose un port de proposition** et un **port de correspondance** (`trouver_par_identifiant(systeme, valeur)`, `candidats(identite)`). C'est la seule surface que le socle 9 lui demande (§5.4, exigences au noyau).
3. **Le socle 9 ne décide pas de qui fait foi** : il lit `source_qui_fait_foi` et `modification_locale` du domaine (02 §7.8, 04 §10) ; il ne possède que les règles **propres au flux** (garde-fous, disparitions, volume, application automatique d'une catégorie de différences).
4. **Un export n'est jamais produit hors du moteur** : un socle qui veut produire LSU appelle `ServiceExports::preparer(…)` avec sa déclaration ; le moteur pose le pré-contrôle, fige, trace. Un export de **travail** (une liste dans un tableur) n'est pas un échange : il reste dans son socle, mais utilise l'échappement commun des cellules (injection de formule, §16).
5. **Aucune notification aux familles par un import.** Un lot appliqué publie des événements métier (par les commandes des domaines) ; c'est le socle 8 qui décide, et les natures d'avis déclenchées par un import sont **désactivées par défaut** pendant l'application d'un lot (drapeau `origine = import` dans l'événement) [R, Q-X9] : une rentrée importée ne doit pas envoyer 900 courriels « nouvelle adresse enregistrée ».
6. **Une fédération ne crée rien.** Elle rattache un compte à une personne **déjà connue** de l'établissement, ou refuse.

### 5.3 Les briques du socle 9

Le socle est découpé en **quatre briques** et une **extension du noyau** (sécurité) [R] :

| Brique | Crate | Contenu | Plage |
|---|---|---|---|
| `echanges` | `briques/echanges` | moteur : réception, lots, connecteurs (trait), garde-fous, rapprochement, mémoire de la source, différences, propositions, annulation, exports figés, traces, retours, planification, tableau de bord | 3900–3908 |
| `connecteurs-*` | `briques/echanges/connecteurs/<famille>` (modules du crate `echanges` au début ; crates séparés si la compilation le justifie) | lecteurs et générateurs purs par famille : `tableur`, `siecle`, `sts`, `aaf`, `onde`, `gar`, `edt` (aSc, FET, EDT…), `banque`, `ical`, `oneroster`, `scolia` (format de transfert et de réversibilité) | — (sans table propre) |
| `referentiels` | `briques/referentiels` | tables **globales** (sans `etablissement_id`) : nomenclatures, COG, annuaire des UAI, calendrier officiel, jours fériés ; chargement central versionné | 3909 |
| `federation-idp` | `briques/federation-idp` | ScolIA fournisseur d'identité (OIDC, SAML si exigé) ; identifiants de service opaques ; GAR (SSO, alimentation, médiacentre) ; annuaires sortants | 3911–3913 |
| `calendriers` | `briques/calendriers` | jetons d'abonnement, rendu iCal, imports ICS externes | 3914 |
| **extension noyau sécurité** (socle 1) | `noyau-securite` | fournisseurs fédérés (abstraction OIDC/SAML), guichet central, tickets de transfert vers le sous-domaine, rattachements, codes de rattachement | **3910** (dans la plage du socle 9 parce que conçue ici, mais **propriété du noyau** ; à renuméroter dans 1000–1199 si Q-B2 attribue ces tables au noyau, Q-X2) |

### 5.4 Exigences adressées au socle 9 par les autres conceptions, et réponse

| Source | Exigence | Réponse |
|---|---|---|
| `02_ELEVE.md` §4, §7.8, E5 | Rien de l'extérieur sans proposition ; « source qui fait foi » par champ ; règle d'application automatique explicite et tracée | §6.7 ; R-X1–R-X4 ; dépend d'**E5** (file `ent_eleve_proposition`, non encore codée [F : aucune table `*_proposition` dans les migrations]) |
| `02_ELEVE.md` §7.3, §8.1 | INE par cinq voies, statut selon la provenance ; collision → `conteste` ; exports nationaux avec INE confirmé seulement | connecteurs SIECLE/AAF/ONDE déposent `import_*` ; pré-contrôle « INE confirmé » (§6.10) ; retour BEE (`retour_bee`) |
| `02_ELEVE.md` §8.2 | AAF, BEE, remontée BEE, ONDE, ÉduConnect, GAR (pas d'INE), ancien établissement, famille, ancien logiciel | fiches FX-01, 02, 30, 04/31, 39, 101, 102 ; ÉduConnect §10.3 |
| `03_INSCRIPTIONS.md` §5, INS-51 | Import d'élèves affectés → dossiers `affectes`, **jamais des élèves** ; téléservice d'inscription sans doublon | FX-05, FX-06 : entité normalisée `dossier_affecte` vers le port du socle 3 |
| `03_INSCRIPTIONS.md` §2.2 (19), §5 (634) | `ent_import_lot` appartient au socle 9 ; reclassement dans la transaction | X0, X1 ; reprise de 144 (§24) |
| `04_ADMINISTRATIF.md` §5.2 point 1, §5.3 | Personne n'écrit dans le socle 4 ; port `correspondances(declaration)` ; rapprochement de foyer **par identifiant d'adresse, jamais par libellé** (défaut Next.js relevé) | port de correspondance utilisé ; R-X7 (jamais de rapprochement de foyer par adresse seule) |
| `04_ADMINISTRATIF.md` §8.11 | `ent_personne_identifiant` (SIECLE_PERSONNE, ONDE_RESPONSABLE, AAF_JOINTURE) | alimentés par FX-01, 02, 04 |
| `04_ADMINISTRATIF.md` §8.6 (adresse) | COG pays/communes ; adresse protégée | FX-51 (COG), FX-55 (BAN locale) ; une adresse protégée n'est **jamais** exportée en clair (R-X15) |
| `05_STRUCTURE.md` §9.4, §17 | `ref_nomenclature` global tenu par le socle 9 ; calendrier officiel par zone ; imports de structure SIECLE et STS en propositions ; logiciels d'EDT ; « attendre la confirmation du format STS » | X4 ; FX-02, 03, 18 ; X6 marqué « format à confirmer sur fichiers réels » |
| `05_STRUCTURE.md` §4 (public) | Codes nationaux obligatoires à l'export, tableau de bord des manques | pré-contrôle (§6.10) |
| `06_PERSONNELS.md` PRS-110/111, §8.11 | Import STS-web et AAF des personnels, jointure AAF, NUMEN **non** stocké (Q-P15) | FX-01, 03 ; NUMEN jamais lu ni stocké (R-X16) ; `gar` à retirer des systèmes d'identifiants d'agent (§28, contradiction C3) |
| `07_DOCUMENTS.md` | Stockage, analyse, conservation des fichiers | fichiers reçus et générés déposés par le port du noyau, nature « pièce d'échange » à déclarer au socle 7 (conservation courte, §17) |
| `08_NOTIFICATIONS.md` §6 | Les briques déclarent leurs natures d'avis | §19.3 : 7 natures (personnels seulement) |
| `IA_COEUR.md` §5 | Contrat IA de chaque lot | §20 |
| `ARCHITECTURE_BRIQUES.md` §1.5 | `imports.rs` écrit dans le noyau | X0, X2 |
| `MATRICE_CHARLEMAGNE.md` T-26, T-28, PIL-09, PIL-20 | Export complet, pré-contrôle des exports, reprise, identité nationale | FX-104, §6.10, FX-102, §10 |

**Exigences du socle 9 envers le noyau et les autres socles (nouvelles) :** (a) ports `proposer` et `correspondance` des modules élève, personnes, structure, personnels (forme commune §6.7) ; (b) champ `origine` (`saisie` | `import` | `famille` | `systeme`) dans l'enveloppe des événements publiés par une commande, pour que le socle 8 puisse taire les avis d'un import ; (c) au socle 1 : fournisseurs fédérés, tickets, rattachements (§10) ; (d) au socle 7 : nature « pièce d'échange » à conservation courte ; (e) aux socles 22, 23, 37, 38, 48, 50 : produire leurs exports par `ServiceExports`.

---

## 6. Le moteur d'échanges

### 6.1 La chaîne, en entrée

```
 dépôt manuel ─┐                                                    ┌─▶ proposition (domaine) ─▶ décision humaine ─▶ commande du domaine
 canal auto ───┼─▶ 1 RÉCEPTION ─▶ 2 IDENTIFICATION ─▶ 3 LECTURE ─▶ 4 NORMALISATION ─▶ 5 RAPPROCHEMENT ─▶ 6 DIFFÉRENCES ─┤
 réception     │    empreinte,      format, version,    flux, en-     fiches             mémoire de        3 voies,      └─▶ règle explicite ─▶ commande du domaine
 centrale ─────┘    analyse,        UAI, année,         codage,       canoniques         correspondance,   classes            (application automatique, tracée)
                    quarantaine     ancienneté          limites       (§6.4)             identifiants
                                                                                                             ▼
                                         7 GARDE-FOU DE VOLUME ─▶ 8 APPLICATION (transaction par paquet) ─▶ 9 RAPPORT ─▶ 10 ANNULATION possible (compensation)
```

Chaque étape est **pure ou transactionnelle**, rejouable, et laisse une trace dans `ent_ech_lot_etape`. Les étapes 1–7 n'écrivent **rien** hors des tables du moteur : la prévisualisation est la vraie analyse, pas une simulation approximative.

### 6.2 Le contrat de connecteur (ce que chaque flux déclare)

Dans le crate de la brique `echanges` (module `connecteurs/<famille>`) :

```rust
pub struct ConnecteurDecl {
    pub code: &'static str,               // "siecle.bee.eleves", "sts.emp", "tableur.eleves", "gar.alimentation"
    pub famille: Famille,                 // Siecle | Sts | Aaf | Onde | Gar | Tableur | Edt | Banque | Ical | OneRoster | Scolia | ...
    pub sens: Sens,                       // Entrant | Sortant | Retour (compte rendu d'une sortie)
    pub versions: &'static [VersionFormat],   // versions comprises ; toute autre est refusée (R-X5)
    pub profils: &'static [ProfilEtab],   // public_2d, prive_contrat_2d, hors_contrat, agricole, premier_degre…
    pub entites: &'static [TypeEntite],   // Eleve, Responsable, LienResponsabilite, Adresse, Agent, Service, Division, Groupe, AppartenanceGroupe, Matiere, Mef, Creneau, Ecriture…
    pub canaux: &'static [Canal],         // DepotManuel | Sftp | WebService | ReceptionCentrale
    pub garde_fous: GardeFous,            // UAI obligatoire ? année ? seuils de volume
    pub disparitions: PolitiqueDisparition, // Ignorer | ProposerSortie | Signaler (jamais Appliquer, R-X10)
    pub conservation: Conservation,       // brut, normalisé, rapport (§17)
    pub action_import: Action,            // droit D-017 exigé pour déposer
    pub action_appliquer: Action,         // droit exigé pour appliquer (★)
    pub statut: StatutConnecteur,         // Disponible | Experimental | Bloque { raison: &'static str }   (ex. "spécification AAF non reçue")
}

pub trait Lecteur {                       // pur : pas de base, pas de réseau, pas d'horloge
    fn identifier(&self, entete: &[u8], nom: &str) -> Option<Identification>;   // format, version, UAI, année, date d'extraction
    fn lire(&self, flux: &mut dyn Read, limites: &Limites) -> Result<FluxEnregistrements, ErreurLecture>; // itérateur, jamais tout en mémoire
}
pub trait Generateur {                    // pur : à partir d'un instantané, produit des octets déterministes
    fn generer(&self, instantane: &Instantane, version: VersionFormat) -> Result<Vec<Fichier>, ErreurGeneration>;
}
```

- **Statut `Bloque`** : le connecteur existe dans le catalogue (l'écran l'affiche avec la raison et la démarche en cours) mais ne peut pas être activé. C'est ce qui évite de coder un format deviné tout en montrant au client ce qui viendra.
- **Jeu d'or** : chaque connecteur `Disponible` a des fichiers d'exemple **anonymisés** (§26, démarche D-14) et leurs sorties attendues versionnées ; un changement de lecteur qui modifie une sortie casse le test.
- **Fuzzing** : chaque lecteur a une cible `cargo fuzz` (XML, CSV, zip) ; aucune entrée ne doit faire paniquer ni dépasser les limites (§16).

### 6.3 Réception et garde-fous d'identité du fichier

1. **Dépôt** : glisser-déposer, ou canal automatique (SFTP de l'académie, webservice, réception centrale). Taille plafonnée par connecteur (défaut 50 Mo compressé, 500 Mo décompressé, 20 000 enregistrements par lot de tableur ; les flux AAF de grands lycées sont plus gros [H] : plafond par connecteur).
2. **Empreinte** SHA-256 du fichier brut : un **même fichier** déjà importé est signalé (« déjà importé le 3 septembre par Mme B. ») ; on peut le réanalyser, pas l'appliquer deux fois par erreur.
3. **Analyse** par le socle 7 (antivirus, type réel, archive saine : nombre d'entrées, taux de compression, chemins sans `..` ni absolu, pas de lien symbolique) ; échec → **quarantaine**, rien n'est lu.
4. **Identification** (`Lecteur::identifier`) : format et version ; **UAI** présente dans le fichier ; **année scolaire** ; **date d'extraction**.
5. **Garde-fous bloquants** [R-X6] : UAI absente des sites de l'établissement → **refus** (« ce fichier concerne l'établissement 0xxxxxxY, qui n'est pas le vôtre ») — le contenu n'est ni lu ni conservé au-delà de la quarantaine ; une UAI ne peut être déclarée que par **un seul** établissement de la plateforme, **validée par l'opérateur** contre `ref_uai` avant activation de tout connecteur (sinon le garde-fou protège de l'erreur, pas de l'abus) [relecture, obj. 5] ; année différente de l'année visée → refus sauf choix explicite (« préparer la rentrée N+1 ») ; date d'extraction **antérieure** à celle du dernier lot appliqué du même connecteur → avertissement bloquant (« ce fichier est plus ancien que celui du 12 septembre ; l'appliquer remettrait d'anciennes valeurs ») levé seulement par ★.
6. **Encodage** : détection (UTF-8, avec ou sans BOM, ISO-8859-1/15, Windows-1252) par le connecteur selon sa déclaration ; l'encodage exact des exports SIECLE est **NV** ; jamais de remplacement silencieux de caractères : un caractère non décodable est un rejet de ligne lisible (« caractère illisible dans le nom, ligne 214 »).

### 6.4 Normalisation : les fiches canoniques

Chaque connecteur produit des **enregistrements canoniques** typés, indépendants du format :

| Type | Contenu canonique (extrait) | Port du domaine cible |
|---|---|---|
| `Eleve` | identifiants externes (système, valeur), état civil (nom de famille, nom d'usage, prénoms, sexe, date de naissance, commune ou pays de naissance COG), scolarité (MEF, division, régime, statut), entrée et sortie | socle 2 : `eleve::proposer`, `eleve::correspondance` |
| `Responsable` | identifiant source, civilité, nom, prénom, coordonnées, adresse (structurée), PCS si fournie et activée | socle 4 : `personnes::proposer` |
| `LienResponsabilite` | élève, responsable, nature codée (code de parenté source → vocabulaire du socle 4), niveau de responsabilité, autorité, payeur source, ordre de contact | socle 4 |
| `Adresse` | identifiant source d'adresse, lignes, code postal, commune COG, pays COG | socle 4 (foyer **par identifiant d'adresse**, R-X7) |
| `Agent` | identifiants (STS, jointure), nom, prénom, fonction codée, discipline de poste, engagement | socle 6 : `personnels::proposer` |
| `Service` | agent, matière (code national), division ou groupe, heures, modalité | socle 6 / 5 |
| `Division`, `Groupe`, `AppartenanceGroupe`, `OptionEleve` | codes source, libellés, MEF, rattachements | socle 5 : `structure::proposer` |
| `Matiere`, `Mef`, `Nomenclature` | code national, libellés, validité | socle 5 (référence) / `referentiels` |
| `Creneau` | jour, heures, semaine, cours, salle, intervenants | socle 10 |
| `DossierAffecte` | élève affecté (identité, INE, formation d'affectation, origine) | socle 3 : `inscriptions::proposer_affecte` |
| `MouvementBancaire`, `RejetPrelevement` | montant, date, référence de bout en bout, mandat | socle 25 |
| `EvenementCalendrier` | début, fin, titre, catégorie, cible | socle 19 / 5 |
| `RetourRejet` | référence de l'export, enregistrement visé, code et libellé du rejet | moteur (§6.10) |

Les valeurs sensibles (adresse d'une personne protégée, PCS, nationalité) sont **chiffrées** dans les enregistrements du moteur (`champs_chiffres`, socle 1) et purgées à la fin de la fenêtre d'annulation (§17).

### 6.5 Rapprochement et mémoire de correspondance

Ordre de rapprochement d'un enregistrement entrant avec une entité locale [R-X7] :

1. **Correspondance mémorisée** : `ent_ech_correspondance (connecteur_famille, type_entite, cle_source) → entite_id`, créée au premier rapprochement confirmé. Stable d'une année sur l'autre ; c'est elle qui évite de refaire le travail à chaque import.
2. **Identifiant externe confirmé** dans le domaine (INE, identifiant élève SIECLE, identifiant de personne SIECLE, identifiant STS) par le port `correspondance` — **dans l'établissement seulement** (D-013).
3. **Candidats par identité** (port `candidats` du domaine : nom, prénom, date de naissance, sexe pour l'élève ; nom, prénom, coordonnées **vérifiées** pour une personne), avec score et raisons. **Jamais d'application automatique sur un rapprochement par identité** : il est proposé (« même élève ? » avec les différences), confirmé une fois, puis mémorisé (étape 1).
4. Sinon : **nouvel enregistrement** → proposition `nouvel_eleve` (socle 2), `nouvelle_personne` (socle 4)…

Règles : un foyer n'est **jamais** rapproché par libellé ou adresse seule (défaut Next.js relevé, `04_ADMINISTRATIF.md` §2.3) ; deux enregistrements du même lot qui visent la même entité sont **fusionnés avant** le calcul des différences (dédoublonnage dans le lot, défaut STR-65 relevé au socle 5) ; une collision d'identifiant (même INE sur deux élèves) n'est pas résolue par le moteur : elle devient une alerte de qualité au socle 2 (`conteste`).

Bascule d'année et transferts [relecture, obj. 17] : les correspondances portent l'**année** pour les entités annuelles (divisions, groupes, services) — un code de division réattribué l'année suivante ne pointe pas vers l'ancienne classe ; les correspondances d'élèves et de personnes sont pérennes. Un élève qui passe d'un établissement ScolIA à un autre n'emporte **aucune** correspondance (D-013) : le second établissement repart de ses propres imports.

### 6.6 La mémoire de la source et la différence à trois voies

Pour chaque `(établissement, connecteur_famille, entité, champ)`, le moteur garde `ent_ech_valeur_source` : **l'empreinte** (HMAC-SHA-256 avec une clé par établissement, jamais la valeur) de la dernière valeur **reçue** de cette source, et la date du lot. Avec la valeur locale (lue par le port du domaine) et la valeur entrante, on obtient la table de vérité [R-X8, Q-X8] :

| Source avant (B) | Local (L) | Source maintenant (E) | Classe | Ce que fait le moteur |
|---|---|---|---|---|
| — (jamais reçu) | vide | valeur | `nouveau` | proposition ; application automatique si la règle du champ le permet |
| — | valeur = E | valeur | `deja_aligne` | rien (mémorise B := E) |
| — | valeur ≠ E | valeur | `ecart_initial` | proposition (premier import : on ne sait pas qui a raison) |
| B | L = B | E = B | `inchange` | rien |
| B | L = B | E ≠ B | `source_modifiee` | la source a changé et l'établissement n'avait pas touché : proposition, **application automatique si la source fait foi** pour ce champ |
| B | L ≠ B | E = B | `correction_locale_maintenue` | l'établissement a corrigé, la source n'a pas bougé : **rien n'est proposé** ; un **écart persistant** est noté (« SIECLE dit encore *Dupond* : corrigez dans SIECLE ») — dans le privé qui remonte vers BEE, la correction **partira** au prochain envoi |
| B | L ≠ B | E = L | `convergence` | la source a rejoint la correction locale : rien ; écart persistant clos |
| B | L ≠ B | E ≠ B, E ≠ L | `conflit` | les deux ont changé : **arbitrage humain** obligatoire, jamais automatique |
| B | L | absente du fichier | `disparu` | jamais d'effacement ; selon la politique du connecteur : signaler, ou proposer « sortie probable » (élève), « fin d'engagement probable » (agent) — R-X10 |

- **Quand B s'écrit** [relecture, obj. 8] : B n'est **jamais** mis à jour à la réception. Il prend la valeur entrante quand la différence est **décidée** : acceptée (B := E = L), **refusée** (B := E aussi : la source a bien dit E, l'établissement a dit non ; tant que la source répète E, rien n'est reproposé — `correction_locale_maintenue` — et si elle change, on repropose), ou appliquée par règle. Une proposition encore **en attente** quand arrive le lot suivant est remplacée (`obsolete`) par la nouvelle si E a changé, conservée sinon. L'**annulation** d'un lot restaure les B qu'il avait écrits (journal des B par lot).
- **Lignes complémentaires** : la source **vide** un champ (E vide, L et B non vides) → classe `source_videe`, toujours en revue humaine, jamais appliquée seule ; **deux sources** sur le même champ (AAF et exports BEE) → une mémoire B **par famille de source**, et le profil désigne la source de référence (l'autre ne produit que des signalements) ; entité absente localement et présente dans B → l'entité a été fusionnée ou retirée localement : signalement, pas de recréation.
- **Normalisation versionnée** : la comparaison porte sur la valeur **normalisée** (NFC, espaces, casse des noms selon la règle du domaine, formats de date et de téléphone) ; la version de normalisation et la version de clé font partie de l'empreinte ; un changement de l'une ou de l'autre déclenche un **recalcul** des B à partir du dernier lot conservé, pas une vague d'`ecart_initial`. Rotation de la clé HMAC : procédure dédiée (double empreinte pendant la transition).
- **Valeur affichée d'un écart** : pour les seuls écarts **ouverts**, la valeur source est conservée **chiffrée** dans `ent_ech_ecart` (on doit pouvoir dire « SIECLE dit encore *Dupond* »), et effacée à la clôture de l'écart. La clé HMAC est au coffre (les champs à faible entropie — sexe, date de naissance — seraient sinon attaquables par dictionnaire).
- Pourquoi une empreinte et pas la valeur : la comparaison n'a besoin que de l'égalité ; on évite de **dupliquer** l'état civil et les adresses dans une troisième table (minimisation). La valeur entrante, elle, est dans l'enregistrement du lot (chiffrée si sensible) pendant la fenêtre d'annulation.
- La **première** fois qu'un établissement branche un connecteur, tout est `nouveau`, `deja_aligne` ou `ecart_initial` : le premier import est **le plus long à relire**, les suivants ne montrent que ce qui a vraiment bougé. L'écran le dit (« premier import : 214 écarts à arbitrer une fois ; les prochains ne montreront que les changements »).
- **Écarts persistants** : tableau dédié (« 12 corrections faites ici et pas dans SIECLE ») ; dans le public, c'est une **liste de travail pour corriger SIECLE** ; dans le privé remontant vers BEE, c'est la liste de ce que la prochaine remontée corrigera.

### 6.7 Propositions et application

Forme commune du port de proposition, implémentée par chaque domaine [R, exigence (a) du §5.4] :

```rust
pub struct PropositionEntrante {
    pub cible: CibleProposition,          // entité existante ou nouvelle
    pub champs: Vec<ChampPropose>,        // { champ, actuel_empreinte, propose (valeur, chiffrée si sensible), classe: ClasseDifference }
    pub source: SourceProposition::Import { connecteur, lot_id, enregistrement_id },
    pub preuve: Preuve,                   // fichier (document_id), empreinte, numéro de ligne ou chemin XML
    pub application: Application,         // Humaine | AutoSiRegle { regle: CodeRegle }
}
pub trait PortPropositions {
    async fn proposer(&self, tx: &mut TenantTx<'_>, a: &Acteur, p: PropositionEntrante) -> Result<PropositionId, Refus>;
    async fn etat(&self, tx: &mut TenantTx<'_>, ids: &[PropositionId]) -> Result<Vec<EtatProposition>, Erreur>; // en_attente, acceptee, refusee…
    async fn retirer(&self, tx: &mut TenantTx<'_>, a: &Acteur, id: PropositionId, motif: MotifRetrait) -> Result<(), Refus>; // annulation d'un lot
}
```

- **Application automatique** : seulement si (1) la classe est `nouveau` (champ vide) ou `source_modifiee`, (2) le domaine dit que la source fait foi pour ce champ dans ce profil, (3) `modification_locale ≠ interdite`, (4) le lot a passé le garde-fou de volume, (5) la règle n'est pas suspendue par l'établissement. Le domaine applique **sa** commande, avec la proposition en `source_ref` ; il refuse si sa propre règle l'interdit (ex. identité d'un élève à confidentialité renforcée : jamais automatique, `02_ELEVE.md`).
- **Décisions groupées** : l'écran de revue permet d'accepter une **classe** entière d'un connecteur (« accepter les 31 changements de téléphone venant de SIECLE ») ; chaque proposition reste tracée individuellement. La décision exige **le droit du domaine** (ex. `eleves.identite.modifier`) en plus du droit d'échange : un secrétaire sans droit sur la santé n'appliquera jamais une donnée de santé importée.
- **Transaction par paquets** de 200 enregistrements [R] : un lot de 2 000 élèves ne verrouille pas la base dix minutes ; chaque paquet revérifie que les valeurs locales n'ont pas changé depuis la prévisualisation (version ou empreinte) et **recalcule** la ligne sinon (au lieu d'écraser la saisie faite entre-temps au guichet). Le lot est `applique_partiellement` tant que tous les paquets ne sont pas passés ; une reprise après panne repart du paquet suivant (idempotence par `(lot, enregistrement)`).

### 6.8 Annulation d'un lot

- **Par compensation, jamais par `DELETE`** : pour chaque proposition appliquée, le moteur demande au domaine sa commande inverse (revenir à la version précédente de l'identité, clore la scolarisation ouverte par erreur, retirer la personne créée si rien ne la référence) ; pour chaque proposition en attente, `retirer`. Le domaine refuse ce qui ne peut plus être défait (une personne créée par le lot a depuis signé un document ; un élève a été noté) et le rapport d'annulation le **liste** (« 3 élèves ne peuvent pas être retirés : ils ont des absences saisies depuis »).
- **Fenêtre** : paramétrée par le logiciel, 30 jours [R, même délai que la fusion de personnes `02_ELEVE.md` §7.10] ; au-delà, les enregistrements normalisés sont purgés (§17) et l'annulation n'est plus proposée.
- Droit ★ `echanges.annuler` ; journalisé.

### 6.9 Flux automatiques, planification, surveillance

- Un connecteur à canal automatique (AAF quotidien, GAR, relevés bancaires, calendrier officiel) est une **tâche déclarée** (`TacheDecl`, `ARCHITECTURE_BRIQUES.md` §4.4) avec son motif ; elle reçoit, analyse et **n'applique que ce que les règles automatiques autorisent** ; le reste attend une personne.
- **Surveillance** : chaque connecteur automatique déclare sa périodicité attendue ; une absence (« aucun fichier AAF depuis 2 nuits »), un taux de rejet anormal ou une erreur de canal crée une **alerte** (nature d'avis `echanges.flux_en_retard`, socle 8, destinataires : administrateur et direction) et une ligne au tableau de bord.
- **Réception centrale** (flux qui arrive au niveau de la plateforme pour plusieurs établissements, cas possible de l'AAF ou du GAR [H, NV]) : un **routeur** de la plateforme lit **seulement** l'UAI et l'en-tête (une UAI ambiguë, déclarée par zéro ou plusieurs tenants, fait échouer le routage vers la quarantaine), découpe par établissement, et dépose chaque part dans le tenant correspondant ; une UAI inconnue va en **quarantaine plateforme** (aucun tenant ne la voit, purge à 7 jours, alerte à l'opérateur **sans contenu**). C'est la seule étape qui manipule des données de plusieurs établissements : processus séparé, journal dédié, test d'isolation spécifique (Q-X13).

### 6.10 Sorties : pré-contrôle, instantané, génération, dépôt, retour

```
 socle propriétaire (22, 23, 37, 38, 48, 50, 9) ── ServiceExports::preparer(declaration, perimetre)
     │
     ▼
 1 PÉRIMÈTRE (qui est inclus : élèves, personnes, agents — via eleves_visibles ; minimisation par la déclaration de champs)
 2 PRÉ-CONTRÔLE (règles du format + règles métier du socle) ─▶ anomalies bloquantes / avertissements, chacune avec un LIEN vers l'écran qui corrige
 3 INSTANTANÉ figé (données confirmées seulement, versions citées) ─▶ empreinte
 4 GÉNÉRATION (Generateur pur, déterministe) ─▶ validation XSD si le schéma est disponible ─▶ fichier(s) + empreinte
 5 CONFIRMATION humaine (★ pour une remontée nationale) ─▶ DÉPÔT (téléchargement, ou canal automatique) ─▶ trace d'export
 6 RETOUR (compte rendu de l'application nationale, importé par un connecteur « Retour ») ─▶ rejets rattachés aux enregistrements ─▶ tâches de correction
```

- **Pré-contrôle partout et en avance** : le même pré-contrôle tourne **à la demande** et **chaque semaine** dans les huit semaines qui précèdent l'échéance déclarée du flux (§4) ; le tableau de bord affiche un **état de préparation** (« LSL dans 3 semaines : 12 élèves sans INE confirmé, 3 matières sans code national, 2 appréciations manquantes — voir ») [R, trou T-28 de la matrice].
- **Données confirmées seulement** : un INE `declare` n'est pas exporté vers une application nationale ; une note non publiée n'est pas exportée ; l'écart est une anomalie, pas une omission silencieuse.
- **Déterminisme** : à instantané égal, octets égaux (ordre de tri fixé, horodatage d'en-tête = date de l'instantané). On peut **régénérer** un export passé à l'identique et **comparer** deux exports (« ce qui a changé depuis la remontée du 15 septembre »).
- **Validation XSD** : quand le schéma officiel est disponible (fourni à l'éditeur validé), il est versionné dans le connecteur et la génération échoue si le fichier n'est pas valide. Sans schéma, le connecteur reste `Bloque` (pas de « XML approximatif » : défaut 7 du §2.2).
- **Retours bouclés** : synthèse d'import LSU (succès, alertes, rejets [S]), rejets BEE, compte rendu d'alimentation GAR [S] : chaque rejet est relié à l'enregistrement exporté (par l'instantané) et devient une tâche (« Léa M. : INE refusé par BEE — vérifier ») dans l'à-faire du socle 29.

### 6.11 Traces d'export

`ent_ech_export_inclusion (export_id, type_entite, entite_id, champs TEXT[])` : **qui** a été inclus, **avec quels champs**, **vers quel destinataire**, **quand**, **par qui**. Usages : répondre à une demande d'accès RGPD (« à qui les données de mon enfant ont-elles été transmises ? » : liste des exports qui l'incluent, sans les valeurs) ; prouver une remontée ; limiter un export aux personnes autorisées (une adresse protégée n'est pas incluse en clair, R-X15). Conservation : celle du flux (§17).

### 6.12 Règles du moteur (récapitulatif)

| N° | Règle |
|---|---|
| R-X1 | Rien de ce qui vient de l'extérieur n'est écrit sans **proposition** déposée dans le domaine propriétaire, avec source et preuve (fichier, empreinte, localisation). |
| R-X2 | Le socle 9 **n'écrit dans aucune table** d'un autre domaine ; il appelle les ports `proposer` et, pour une application automatique, la commande du domaine. |
| R-X3 | **Qui fait foi** est décidé par le domaine, champ par champ, selon le profil d'établissement ; le socle 9 le lit. |
| R-X4 | Une application automatique est **explicite, tracée et suspendable** (règle nommée sur chaque proposition appliquée). |
| R-X5 | Un format ou une **version non déclarée** est refusé ; on ne devine jamais un format. |
| R-X6 | Garde-fous d'**identité du fichier** (UAI, année, ancienneté) avant toute lecture du contenu ; non désactivables. |
| R-X7 | Rapprochement : correspondance mémorisée → identifiant confirmé → candidats par identité (jamais appliqués seuls) ; jamais de foyer rapproché par adresse ou libellé. |
| R-X8 | Différence à **trois voies** (source avant, local, source maintenant) ; un conflit n'est jamais appliqué automatiquement. |
| R-X9 | Un import ne **prévient aucune famille** par lui-même (`origine = import`). |
| R-X10 | Une **disparition** n'est jamais appliquée : signalée ou proposée. |
| R-X11 | Rattachement fédéré par preuve forte seulement (jointure officielle, code, annuaire provisionné, e-mail professionnel confirmé si autorisé). |
| R-X12 | Code de rattachement : un par personne, usage unique, 30 jours, empreinte seule, essais limités. |
| R-X13 | Un rattachement suit le lien qui le fonde (suspendu avec lui, sessions révoquées), relu à chaque connexion. |
| R-X14 | Lier un nouveau moyen de connexion exige une assurance forte ou un code, jamais une concordance d'e-mail. |
| R-X15 | Une **adresse protégée** n'est jamais exportée en clair ; elle n'entre dans une proposition que scellée. |
| R-X16 | Le **NUMEN** n'est ni lu ni stocké ; les champs non déclarés par un connecteur ne sont pas chargés. |
| R-X17 | **[ajout personas]** Un flux sortant qui **s'élargit** (nouvelle version de connecteur, nouveau champ déclaré, nouveau périmètre d'un partenaire) n'envoie rien avant une reconfirmation ★ de l'administrateur ; l'aperçu montre « ce qui part pour la première fois ». |
| R-X18 | **[ajout personas]** Un élève à confidentialité renforcée ou à identité protégée (socle 2) est **exclu par défaut** des annuaires sortants (FX-90–93, 97) et des partenaires (FX-95) ; l'inclusion est une décision de la direction, personne par personne, journalisée ; le GAR reçoit l'identifiant de service et le nom d'usage seulement si la déclaration du flux l'exige. |
| R-X19 | **[ajout personas]** Un rattachement **révoqué pour un motif de protection** (interdiction, autorité retirée, mesure judiciaire) bloque toute nouvelle jointure de la même personne dans l'établissement jusqu'à levée ★ par la direction ; les listes de confirmation, unitaires ou en masse, **excluent** les personnes dont un lien est suspendu ou sous mesure. |
| R-X20 | **[ajout personas]** Le garde-fou de volume est **conscient de la bascule** : les départs attendus (élèves du dernier niveau, départs déclarés, non-réinscrits du socle 3, élèves sortis) ne comptent pas dans le seuil ; seules les disparitions inattendues le déclenchent. |
| R-X21 | **[ajout personas]** Un export déposé **à la main** se clôt par une **déclaration de dépôt** (date, compte rendu joint) ; un export généré non déclaré déposé est relancé (S-79) ; avant la déclaration, si les données ont changé depuis l'instantané, l'export est marqué **périmé** avec l'écart. |
| R-X22 | **[ajout personas]** Chaque remontée nationale produit une **attestation** (PDF signé par le socle 7 : empreinte, nombre de personnes, destinataire, dates, accusé) et une entrée dans un **journal scellé** (chaîne d'empreintes horodatées), conservée selon le flux (§17). |
| R-X23 | **[ajout personas]** Une **reprise** depuis un ancien logiciel suit un **catalogue validé ★ par la direction** (repris / archivé en PDF / abandonné) et les règles de conservation : rien n'est repris au-delà de sa durée (sanctions effacées selon **R. 511-13 IV** : avertissement à l'issue de l'année scolaire, blâme et mesure de responsabilisation à l'issue de l'année suivante, autres sanctions sauf exclusion définitive à l'issue de la deuxième année suivante [V copie, décret 2019-906, V#36 ; application au privé **NV**], santé hors PAI en cours, anciens responsables) ; tout passe en propositions. |
| R-X24 | **[ajout personas]** **Cohabitation** : tant qu'un logiciel agréé reste en place pour une remontée, le profil `prive_contrat_cohabitation` fait de ScolIA la source de saisie, du logiciel agréé la source de remontée et de SIECLE la source de retour ; chaque mouvement concerné crée une ligne du **journal à reporter**, rappelée jusqu'à son pointage. |
| R-X25 | **[ajout personas]** **Révocation générale** d'une personne en un geste ★ : sessions web et mobiles, rattachements, abonnements iCal, comptes tiers suspendus, nouvelles assertions de l'IdP refusées ; effet en moins d'une minute ; journal et avis de sécurité. |
| R-X26 | **[ajout personas]** Toute **échéance d'expiration** (certificat, clé de l'IdP, secret de canal, convention) est déclarée et surveillée : alertes J-30 et J-7 à l'administrateur et à l'opérateur (sans contenu). |
| R-X27 | **[ajout personas]** Un professeur n'importe jamais par `echanges.importer` ; l'import de ses propres notes passe par le socle propriétaire (21), à sa portée (D-022), sur le même lecteur de tableur. |
| R-X28 | **[ajout personas]** Un partenaire **révoqué** reçoit une demande de purge signée ; l'attestation de purge est attendue et relancée ; aucun envoi ne repart. |

---

## 7. Concepts

| Concept | Définition | Table |
|---|---|---|
| **Connecteur** | Déclaration d'un flux (sens, format, versions, entités, canaux, garde-fous, droits, conservation, statut) et son lecteur ou générateur pur | code (`ConnecteurDecl`) ; activation par établissement : `ent_ech_connecteur_etab` |
| **Canal** | Moyen par lequel un fichier arrive ou part : dépôt manuel, SFTP, webservice, réception centrale | `ent_ech_canal` (paramètres ; secrets au coffre) |
| **Réception** | Fichier reçu : empreinte, taille, type réel, analyse, quarantaine | `ent_ech_reception` |
| **Lot** | Une analyse d'une réception par un connecteur, avec son cycle de vie (§8.1) | `ent_ech_lot`, `ent_ech_lot_etape` |
| **Enregistrement** | Fiche canonique issue d'un lot, avec sa localisation dans le fichier | `ent_ech_enregistrement` |
| **Rejet** | Enregistrement ou partie de fichier illisible ou invalide, avec motif codé et libellé lisible | `ent_ech_rejet` |
| **Correspondance** | Lien durable « clé dans la source → entité ScolIA » | `ent_ech_correspondance` |
| **Valeur source** | Empreinte de la dernière valeur reçue d'une source pour un champ | `ent_ech_valeur_source` |
| **Différence** | Comparaison à trois voies d'un champ, avec sa classe (§6.6) | `ent_ech_difference` |
| **Écart persistant** | Correction locale que la source ne reflète pas | `ent_ech_ecart` |
| **Règle d'application** | Paramètre de l'établissement qui suspend ou autorise l'application automatique d'une classe de différences pour un connecteur (ce qui fait foi reste au domaine) | `ent_ech_regle` |
| **Export** | Préparation, pré-contrôle, instantané, fichier(s), dépôt et retour d'une sortie | `ent_ech_export`, `ent_ech_export_anomalie` |
| **Instantané** | Copie figée et datée des données confirmées d'un périmètre, citant les versions | `ent_ech_instantane` (JSON chiffré, objet stocké) |
| **Inclusion** | Trace des personnes et champs inclus dans un export | `ent_ech_export_inclusion` |
| **Retour** | Compte rendu d'une application nationale relié à un export | lot de sens `Retour` + `ent_ech_retour_rejet` |
| **Référentiel global** | Nomenclature commune à tous les établissements (BCN, COG, UAI, calendrier) | `ref_*` (§15.3) |
| **Fournisseur fédéré** | Fournisseur d'identité externe (ÉduConnect, guichet Agents, OIDC d'établissement) | `ent_fed_fournisseur` (noyau sécurité) |
| **Rattachement** | Lien entre une identité fédérée (`issuer`, `sujet`) et une personne d'un établissement, avec sa preuve (jointure ou code) | `ent_fed_rattachement` |
| **Code de rattachement** | Code à usage unique remis par l'établissement à une personne pour lier son identité fédérée quand aucune jointure n'existe | `ent_fed_code` |
| **Ticket de transfert** | Jeton à usage unique, lié à l'hôte, qui fait passer une authentification du guichet central au sous-domaine | `ent_fed_ticket` |
| **Identifiant de service** | Identifiant opaque et stable d'une personne pour un service tiers (GAR, Google…) | `ent_idp_identifiant_service` |
| **Abonnement de calendrier** | URL personnelle à jeton, révocable, qui sert un flux iCal minimisé | `ent_cal_abonnement` |

---

## 8. Cycles de vie

### 8.1 Lot entrant

```
recu ──analyse OK──▶ identifie ──garde-fous OK──▶ lu ──▶ analyse (rapprochement + différences) ──▶ previsualise
  │                    │                                                                            │
  └─quarantaine        └─refuse (UAI, année, version inconnue)                  volume anormal ─▶ bloque_volume ──★──▶ previsualise
                                                                                                    │
                            previsualise ──appliquer (★)──▶ en_application ──paquets──▶ applique | applique_partiellement
                                 │                                                         │
                                 └─abandonner──▶ abandonne                                  └─annuler (★, ≤ 30 j)──▶ annule | annule_partiellement
                            applique ──30 j──▶ clos (enregistrements normalisés purgés, rapport conservé)
```
Un lot `previsualise` non appliqué expire à 14 jours [R] (les valeurs locales auront bougé).

### 8.2 Export
`brouillon → controle (anomalies) → pret → genere → confirme (★) → depose → accuse | rejete_partiellement | rejete → (corrige → nouvel export)` ; un export ne se modifie jamais : on en produit un nouveau, qui cite le précédent.

### 8.3 Rattachement fédéré
`propose (jointure trouvée ou code saisi) → confirme (première connexion réussie ; pour un code, consommation) → actif → suspendu (lien familial suspendu, départ, révocation) → revoque` ; un rattachement révoqué ne se réactive pas : on en crée un nouveau (preuve neuve).

### 8.4 Abonnement de calendrier
`cree → actif → (rotation du jeton) → revoque | expire (fin de scolarité, départ, 12 mois sans lecture [R])`.

---

## 9. Catalogue des flux

### 9.1 Comment lire une fiche

Chaque ligne répond aux questions de la mission : **qui** l'utilise et **quand**, **format** (avec sa marque de vérification), **accès** nécessaires (convention, habilitation, agrément), **fréquence**, ce qui se passe **en cas d'erreur**, et ce qui est **faisable sans accès officiel**. « Sens » : E entrant, S sortant, R retour, E/S les deux. « Prop. » : socle qui produit ou reçoit le contenu métier ; « Lot » : lot de ce document.

### 9.2 Applications nationales — entrants

| N° | Flux | Sens | Qui, quand | Format | Accès | Fréquence | En cas d'erreur | Sans accès officiel | Prop. / Lot |
|---|---|---|---|---|---|---|---|---|---|
| **FX-01** | **AAF** (annuaire académique fédérateur, alimenté par SIECLE et STS-web) : élèves, responsables, personnels, établissement, matières, MEF | E | Public 2ᵈ degré ; automatique chaque nuit [S, guides académiques] ; voie normale de création des comptes ENT | XML ; fichiers `ENT_<UAI>_…_<Type>_0000.xml` (exemple `ENT_xxx_PersEducNat_0000.xml` [S, Martinique]) ; **complets et différentiels (« Delta »)** décrits par l'annexe SDET « Alimentation » v6.5 de juin 2022 [S, sommaire seulement] ; schéma, attributs (`ENTPersonJointure`…) **NV** (annexe non publique, recherchée le 2026-11-09, V#15) ; le critère **n° 15** de l'arrêté de 2026 (« Utilisation des exports de l'Annuaire fédérateur ») s'applique aussi aux services de vie scolaire, délai **34 mois** [V#2–3] | **Convention** éditeur–DSI académique, par académie [S] | quotidienne | lot rejeté → l'ancien état reste ; alerte « AAF non appliqué » ; rejets ligne à ligne ; disparition → « sortie probable » proposée | **Non** (pas de fichier sans convention) ; en attendant : FX-02 et FX-03 déposés par l'établissement | 2, 4, 6, 5 / **X13** (bloqué) |
| **FX-02** | **SIECLE BEE — exports XML** : élèves (avec ou sans adresses), responsables avec adresses, structures, nomenclatures | E | Secrétariat ou direction, public **et** privé sous contrat ; rentrée (après la bascule), puis à chaque vague d'arrivées [H] | XML zippé : `ExportXML_ElevesSansAdresses.zip` → `ElevesSansAdresses.xml`, `ElevesAvecAdresses.xml`, `ResponsablesAvecAdresses.xml`, `Structures.xml`, `Nomenclature.xml` [S, IREM Lille, Index Éducation, doc Fregata] ; ordre : l'export Structures exige l'export préalable des nomenclatures [S, guides académiques, V#16] (« structures avant élèves » : non retrouvé) ; balises citées par l'import Next.js (`ID_NATIONAL`, `ELEVE_ID`, `CODE_MEF`, `CODE_DIVISION`…) [F Next, `02_ELEVE.md` §2.4] ; encodage et schéma **NV** (aucun XSD public trouvé le 2026-11-09 ; spécifications sur Pléiade, espace éditeurs [S], V#16) | **aucun** : l'établissement exporte ses propres données (il est responsable de traitement, ScolIA sous-traitant) | à la demande | rejets lisibles ; garde-fous UAI/année ; un fichier élèves sans structures → message « importez d'abord Structures.xml » | **Oui — voie principale du pilote** ; le lecteur se construit sur fichiers **réels anonymisés** (D-14) et toute documentation publique ; marqué `Experimental` jusqu'à validation sur trois établissements | 2, 4, 5 / **X5** |
| **FX-03** | **STS-web — export** `sts_emp_<UAI>_<année>.xml` : individus, fonctions, structures, services, matières | E | Chef d'établissement, adjoint, secrétariat ; préparation de rentrée (juin–septembre) ; menu Exports > Emplois du temps [S, Index Éducation] | XML ; contenu détaillé et encodage **NV** ; bascule vers OMOGEN : **[H] non confirmée** (source commerciale seulement, Skolengo [S-tiers] ; aucune source officielle trouvée le 2026-11-09, V#18) | aucun (export de l'établissement) | quelques fois par an | idem ; groupes non conformes renommés par STS [S, DANE Lyon] → correspondance de groupes mémorisée | **Oui** (même méthode que FX-02) | 6, 5 / **X6** |
| **FX-04** | **ONDE** (premier degré) : élèves, responsables, classes | E | Directeur d'école ; rentrée | **NV** ; spécifications « accessibles après inscription sur Pléiade (espace éditeurs) » [S, eduscol 4920] | inscription éditeur | à la demande | idem | non (sans spécification) ; tableur en attendant | 50 / X14 bis (socle 50) |
| **FX-05** | **Affelnet** : élèves affectés (6ᵉ, 2ᵈᵉ, voie professionnelle) | E | Secrétariat ; juillet | **NV** (liste fournie par l'administration, ou arrivée des affectés dans SIECLE [H]) ; aucune interface éditeur Affelnet trouvée le 2026-11-09 (V#20) | selon académie | annuelle | crée des **dossiers `affectes`**, jamais des élèves (`03_INSCRIPTIONS.md` INS-51) | **Oui** par tableur (liste transmise par la DSDEN) | 3 / X3 puis X5 |
| **FX-06** | **Téléservice d'inscription** (Scolarité services) | E | Secrétariat ; juillet | service **national** « Inscription en ligne » ; import **dans SIECLE** (« Importations > Dossiers d'inscription », trois imports ordonnés) et export CSV des vœux [S, guides eduscol 2025, V#21] ; aucune interface éditeur documentée | selon académie | annuelle | pas de doublon de saisie (`03_INSCRIPTIONS.md` §3.2) | tableur si un export existe | 3 / X3 |
| **FX-07** | **Fregata** (enseignement agricole, a remplacé Libellule [S, notes de service du ministère de l'agriculture]) | E/S | Établissements agricoles publics et privés (CNEAP) | **NV** ; doc citant les mêmes noms de fichiers que BEE [S, extrait à confirmer] | selon ministère de l'agriculture | annuelle | — | tableur | 2, 4, 5 / X14 ter (si marché agricole, Q-X25) |
| **FX-08** | **Comptes rendus des applications nationales** (synthèse LSU, rejets BEE, compte rendu GAR, Parcoursup) | R | Secrétariat ; après chaque dépôt | variés ; LSU : synthèse succès/alertes/rejets [S] | celui du flux | après dépôt | chaque rejet → tâche reliée à l'enregistrement exporté | **Oui** : dépôt manuel du compte rendu, même sans automatisation | moteur / **X7** |

### 9.3 Applications nationales — sortants

| N° | Flux | Sens | Qui, quand | Format | Accès | Fréquence | En cas d'erreur | Sans accès officiel | Prop. / Lot |
|---|---|---|---|---|---|---|---|---|---|
| **FX-30** | **Remontée vers SIECLE BEE** (privé sous contrat) : dossiers élèves, liens élèves-groupes ; base du **constat de rentrée** et des effectifs | S + R | Secrétariat de l'établissement privé ; rentrée (constat de septembre) puis au fil des mouvements | XML zippé [S, guide BEE EPHC 2022] par **NetSynchro**, en convergence avec SiecleSynchro vers **« OMOGEN Fichier »** avec un « import éditeurs privé » (note de version SIECLE 24.4.0.0, octobre 2024 [S, ac-Nice]) ; certificat installé dans la solution éditeur [S] ; interface « **Import du privé** » (2ᵈ degré, privés sous contrat et CNED) avec **validation par éditeur** (éditeurs BEE validés 2020–2022 : Alias, APLIM, ASREC, Axess, CNED, Docaposte, EasyScol [S, eduscol 4920 en extrait]) ; procédure et état d'OMOGEN en 2026 **NV** (V#17) | **Validation éditeur** de l'interface par le ministère (D-2/D-3) [S] | à chaque mouvement, au moins à la rentrée | rejets corrigés **dans le logiciel éditeur** [S] → retour importé (FX-08) → tâches ; l'INE attribué revient (`retour_bee`) [H, canal NV] | **Non** ; en attendant, le client garde son logiciel agréé (cohabitation, Q-X14) | 2, 4, 5 / **X14** (bloqué) |
| **FX-31** | **ONDE « interface école »** (écoles privées) | S | Directeur d'école privée | **NV** ; éditeurs validés V1 : APLIM, ASREC, Docaposte, EasyScol, École futée, Kosmos, **SGEC (Ange)**, TIQTEC ; V2 : APLIM, ASREC, École Futée, EasyScol [S, eduscol 4920 en extrait, 2026-11-09, V#19] | validation éditeur | rentrée et mouvements | — | non | 50 |
| **FX-32** | **Services vers STS-web** (si ScolIA construit l'emploi du temps, D-025) | S | Adjoint ; fin août, ajustements jusqu'à fin septembre, validation des services [S, DANE Lyon] | XML ; voie actuelle : le **logiciel d'EDT** exporte vers STS-web ou passe par NetSynchro [S, Index Éducation, UnDeuxTEMPS] | agrément / interface éditeur [H] | annuelle + ajustements | matières, modalités ou divisions sans code reconnu **bloquent** l'export ; heures de vie de classe à retirer [S, étude §6] → pré-contrôle | **Non** ; voie de contournement : l'établissement garde son logiciel d'EDT pour STS | 10 / X16 bis (après spécification) |
| **FX-33** | **LSU** (collège ; école) : bilans périodiques et de fin de cycle | S + R | Secrétariat, direction (D-022 : jamais un professeur) ; fin de période et fin d'année (remontée de juin [S, NV]) | XML conforme XSD [S, ac-Paris] ; premier degré : `import-lsun-{timestamp}.xml`, schéma `import-bilan-1d.xsd` [S, spec. LSUN 1ᵉʳ degré v1.22] ; XSD 2ᵈ degré réservé aux éditeurs validés ; un import éditeur est « supposé complet » [S] ; **FAQ LSU 1D avril 2026** : import soumis aux mêmes conditions que la saisie, bilans **verrouillés** après import, données non importées avant la fin d'année perdues [V copie, V#22] ; nom de fichier et `import-bilan-1d.xsd` non retrouvés le 2026-11-09 : **NV** | **Validation éditeur** (validations de mars–avril 2026 : APLIM, CGI, Docaposte/Index, EasyScol, École Futée, Gestibase, Kosmos, Sésamath [S, eduscol 4920 en extrait, V#22] ; périmètre LSU 2ᵈ / 1ᵉʳ degré / LSL à relire) | par période | synthèse d'import → retour (FX-08) | **Partiel** : pré-contrôle et **aide à la saisie** dans LSU (export de travail lisible) ; jamais de faux XML (X0) | 22 / X7 + lot du socle 22 |
| **FX-34** | **LSL** (lycée) et **Cyclades** (examens, à partir du LSL [S]) | S + R | Secrétariat, direction ; fin de première, fin de terminale | LSL (arrêté du 4 mars 2020) : moyennes, appréciations, compétences, avis d'examen « importées depuis les éditeurs de notes » ; scolarité « conforme à celle présente dans BEE » ; moyennes annuelles et livret transmis à **Cyclades** pour le bac [V, mémento eduscol 2025, V#23] ; format XML et nomenclature **NV** ; livret de la voie professionnelle intégré au LSL depuis 2024-2025 [S-tiers] | éditeur validé (même liste que le 2ᵈ degré [S, extrait]) | annuelle | idem | partiel (pré-contrôle) | 23 / X7 + socle 23 |
| **FX-35** | **Parcoursup** : bulletins de première et terminale (initialise la fiche Avenir) | S | Secrétariat, direction ; janvier–mars (dates limites citées fin mars [S, académies]) remontée automatique des moyennes et appréciations par l'éditeur de notes ; à défaut, l'élève saisit ses bulletins (saisie ouverte fin mars) [V, fiche Avenir 2026 et mémento, V#24] ; « WebService depuis 2025 » **non retrouvé** (rétrogradé [S]) ; documentation éditeur **NV** (saio.parcoursup.fr, accès réservé) | **logiciel certifié** (liste citée : Ypareo, Gepi, Gestibase, CNED, Pronote, Kosmos/Skolengo [S, extrait académique]) | annuelle | une remontée tardive remplace les saisies des élèves [S] | partiel (pré-contrôle ; saisie par les candidats) | 37 / X7 + socle 37 |
| **FX-36** | **Affelnet lycée** : évaluations de 3ᵉ | S | Secrétariat ; mai–juin | **transmis depuis LSU** par un sous-menu dédié [S, extrait] ; contenu 2026 : **seules les moyennes annuelles** des disciplines de 3ᵉ, plus de bilan de fin de cycle ; transfert LSU → Affelnet fin mai–début juin, qui **écrase** les saisies manuelles [S, guides Affelnet 2026 ac-Lyon, ac-Normandie, V#20] (C2 tranchée) | via LSU | annuelle | — | **par LSU** : si ScolIA alimente LSU, rien de plus à coder [H] | 37 / — |
| **FX-37** | **DNB / Notanet** : notes de contrôle continu | S | Secrétariat ; juin **NV** (aucune interface éditeur Notanet trouvée le 2026-11-09) ; réforme : depuis la session 2026, contrôle continu = **moyennes annuelles de 3ᵉ** par enseignement (plus les huit composantes du socle), pondération **40 % / 60 %** [V, réponse ministérielle QE AN 3477, V#25] | **NV** | annuelle | — | partiel | 38 |
| **FX-38** | **Cyclades** : inscriptions aux examens | S | Secrétariat ; automne [H] | **NV** | **NV** | annuelle | — | saisie dans Cyclades | 38 |
| **FX-39** | **GAR** : alimentation (annuaire minimal) + SSO (§11) | S + R | Automatique ; quotidien [H] ; rentrée décisive | archive de fichiers XML conformes à `GAR-ENT-2D.xsd` / `GAR-ENT-1D.xsd`, contrôle nommage, **MD5**, XSD ; au moins cinq types dont « Eleve » [S, RTFS 7.2/8.2] ; dépôt **SFTP par clé SSH** [V, dossier d'inscription GAR, V#9] ; liste, fréquence **NV** ; documents marqués **diffusion restreinte** [V] (à citer, jamais recopier dans le dépôt) | **raccordement GAR** demandé par un **porteur de projet ENT public** (collectivité ou académie) via la DANé ; l'éditeur est l'exploitant [V, V#7] | quotidienne [H] | compte rendu → retour ; ressource inaccessible à l'élève → diagnostic au médiacentre | non | 9 / **X11, X12** (bloqués) |
| **FX-40** | **Statistiques et enquêtes nationales** (hors constat BEE) | S | Direction | **NV** | — | — | — | — | 47 |

### 9.4 Référentiels ouverts (globaux, chargés une fois pour tous les établissements)

| N° | Flux | Contenu | Format | Licence | Fréquence | Faisable | Lot |
|---|---|---|---|---|---|---|---|
| **FX-50** | **BCN** (base centrale des nomenclatures, DEPP) | `N_MEF`, `N_MEF_MATIERE` (matières et horaires par MEF [S, doc BCN scolarité]), `N_MATIERE_ENSEIGNEE` (champ `GESTION_DIFFUSION` [S]), modalités d'élection, formations, dispositifs | consultation « en libre accès », API via OMOGEN [V, fiche DEPP, V#28] ; `GESTION_DIFFUSION` = filtre de diffusion par application [S] ; export et format **NV** | **aucune licence énoncée** par la DEPP [V#28] ; jeux dérivés sur data.gouv (miroirs de l'Agriculture) « notspecified » [V#28] | à chaque version | **Non tant qu'aucune licence n'est accordée par écrit** (D-12) : saisie manuelle des codes utiles ou codes reçus dans les exports de l'établissement (FX-02, FX-03) | X4 |
| **FX-51** | **COG INSEE** : communes, communes historiques, pays et territoires étrangers | CSV annuel, séparateur virgule, **UTF-8 depuis le millésime 2019** [S, insee.fr] ; pays : deux listes (au 1ᵉʳ janvier, depuis 1943) [S] ; **`99100` = « République française »** [V, insee.fr, V#32] ; depuis 2024 : fichier des pays millésimé et historique depuis 1943, un code 99xxx par territoire [V#32] ; encodage UTF-8 non relu | **NV** (Licence Ouverte probable [H] ; non énoncée sur les pages lues le 2026-11-09) | annuelle (1ᵉʳ janvier) | **Oui** | X4 |
| **FX-52** | **Annuaire de l'éducation** (UAI, nom, adresse, type, secteur) | jeu `fr-en-annuaire-education` [V, API data.education.gouv.fr, V#29] : 75 champs dont `identifiant_de_l_etablissement`, `statut_public_prive`, `type_contrat_prive`, `multi_uai`, `etat`, `date_maj_ligne` ; format UAI « 7 chiffres + 1 lettre » [S] ; règle de la lettre **NV** | **Licence Ouverte v2.0** (DNE) [V#29] : citer la source et la date de mise à jour | **quotidienne** [V#29] | **Oui** (affichage du nom de l'établissement d'origine, contrôle d'UAI saisie) | X4 |
| **FX-53** | **Calendrier scolaire officiel** par zone | jeu `fr-en-calendrier-scolaire` [V, API data.education.gouv.fr, V#30] : champs `description`, `population`, `start_date`, `end_date`, `location`, `zones`, `annee_scolaire` ; iCal par zone ; texte : arrêté annuel (2027-2028 : arrêté du 21/07/2026 [S-tiers]) **hors Corse et outre-mer**, fixés par arrêtés locaux → couche locale saisie par l'opérateur ; anomalies signalées [S] | **Licence Ouverte v2.0** (DNE) [V#30] | annuelle | **Oui** | X4 (adoption : socle 5) |
| **FX-54** | **Jours fériés** | CSV, JSON, ICS (DINUM) [S] | **Licence Ouverte 2.0** [S] | annuelle | **Oui** | X4 |
| **FX-55** | **Base adresse nationale** (normalisation et validation des adresses) | l'API a été transférée à l'IGN (Géoplateforme) ; l'ancienne `api-adresse.data.gouv.fr` annoncée arrêtée fin janvier 2026 [S] ; exports BAN téléchargeables [S] ; ne pas confondre avec la **BANO** (OpenStreetMap, ODbL) | licence ouverte Etalab depuis 2020 [S-tiers, V#33] ; version à relire | quotidienne (exports) [S] | **Oui**, **jeu téléchargé et interrogé chez nous** (aucune adresse de famille envoyée à un service externe, Q-X18) | X4 bis |

### 9.5 Enseignement privé : catholique, hors contrat, agricole

| N° | Flux | Ce qu'on sait | Ce qu'il faut faire | Lot |
|---|---|---|---|---|
| **FX-60** | **SGEC – « Gabriel »** (système d'information de l'enseignement catholique : répertoire des établissements, annuaire des acteurs, outil d'échange entre applications, entrepôt décisionnel anonymisé, données issues de « SOLFEGE ») | [S, présentations 2013–2019 ; éditeurs cités : Agate, Aplon, Charlemagne, Darwin] ; état 2026 et interfaces **NV** | Démarche D-9 (contact SGEC) ; en attendant : **export d'effectifs** documenté (CSV ScolIA) | X17 bis (après réponse) |
| **FX-61** | **UDOGEC / FNOGEC** : cotisations calculées sur les effectifs [H], modèle d'annexe OGEC 2021 (ANC 2018-06 et nomenclature de l'enseignement catholique) [S] | **NV** sur les flux | export d'effectifs daté ; plan comptable (socle 48) | 48 |
| **FX-62** | **Direction diocésaine** : statistiques, enquêtes [H] | **NV** | export d'effectifs par niveau et formation, daté, figé | X7 (générateur `scolia.effectifs`) |
| **FX-63** | **Hors contrat** : déclaration d'ouverture (guichet unique, maire, procureur, préfet), contrôle annuel (L. 442-2 [S]) ; **registre des élèves internes** (identité, adresse, responsables, dates d'inscription et de sortie de l'internat), « en permanence à la disposition des autorités judiciaires et administratives » : **art. R. 442-1** (décret 2018-407), et non L. 442-1 [V copie, V#35] ; transmission **annuelle, première quinzaine de novembre**, des noms, titres et dates d'entrée des **personnels enseignants** : art. D. 442-22-1 [V copie, V#35] ; RNIE renforcé [S, titre Localtis] | **aucune transmission annuelle de la liste des élèves** trouvée ; la liste est vérifiée lors des contrôles [S-tiers] | **registre des internes** (R. 442-1) et **liste des élèves** exportables, datés et figés ; **état annuel des enseignants** (D. 442-22-1) avec rappel en octobre ; ne rien envoyer d'office (Q-X26) | X7 |
| **FX-64** | **Hors contrat : examens** — DNB en candidat libre (inscription par la famille sur le site de l'académie) [S, justice.fr] | — | attestation de scolarité et relevé pour la famille (socle 7) | 38 |
| **FX-65** | **Agricole privé (CNEAP)** : Fregata (FX-07) | [S] | si marché visé (Q-X25) | — |

### 9.6 Banque et comptabilité

| N° | Flux | Sens | Format | Qui, quand | Faisable | Prop. / Lot |
|---|---|---|---|---|---|---|
| **FX-70** | **Prélèvements SEPA** : remise d'ordres | S | ISO 20022 **pain.008** ; le CFONB recommande **pain.008.001.08** (version 2019) « avant novembre 2026 » [S, CFONB 17/12/2025, V#39 ; report évoqué S-tiers] ; version acceptée à confirmer auprès de la banque (D-20) | comptable (privé) ; mensuel | **Oui** (standard public) | 25 (existant `ent-bank` à vérifier) / X7 |
| **FX-71** | **Relevés et avis** : camt.053 (relevé), camt.054 (avis d'écritures, dont prélèvements), rejets (pain.002) [S, guides bancaires] | E | ISO 20022 | comptable ; quotidien ou mensuel | **Oui** | 25 / **X18** : rapprochement → propositions de lettrage, jamais lettrage silencieux |
| **FX-72** | **FEC** (fichier des écritures comptables, art. A47 A-1 LPF) | S | texte à séparateur tabulation ou barre verticale, ou XML ; première ligne = noms des champs [S, BOFiP] ; **18 champs** (1° et 2° du VII de l'article) [S, BOFiP BOI-CF-IOR-60-40-20, V#37] ; « 23 » non retrouvé (C4 tranchée en faveur de 18, tableaux à relire avant de coder) ; obligation pour un OGEC association **NV** | OGEC, expert-comptable ; à la clôture et sur demande | après vérification du texte | 48 |
| **FX-73** | **Écritures vers un logiciel comptable tiers** (Sage, EBP, Cegid, Quadra…) pendant la transition (D-025 : la comptabilité est dans ScolIA, socle 48) | S | formats propriétaires **NV** | expert-comptable | **CSV générique paramétrable** (plan de correspondance des comptes) ; formats propriétaires seulement à la demande d'un client (Q-X27) | 48 |
| **FX-74** | **EPLE public : Op@le** (comptabilité de l'établissement, agent comptable) ; interface **CRDC** (droits constatés de restauration) ouverte à deux éditeurs de restauration depuis le 1ᵉʳ janvier 2026 [S, étude §8] | S | **NV** | adjoint gestionnaire, agent comptable | **état des droits constatés** (restauration, internat) **à saisir** dans Op@le par l'établissement, au format le plus utile ; interface automatique seulement si le ministère l'ouvre (Q-X28) | 40, 41, 48 |

### 9.7 Emploi du temps et calendriers

| N° | Flux | Sens | Format | Faisable | Prop. / Lot |
|---|---|---|---|---|---|
| **FX-80** | **Logiciel EDT (Index Éducation)** : structure en entrée, créneaux en sortie | E/S | import des données STS-web en XML par EDT [S] ; export vers un ENT **NV** | fichiers réels d'un pilote | 10 / X16 |
| **FX-81** | **aSc Timetables** | E/S | « aSc Timetables 2012 XML », identifiants internes aléatoires, export configurable [S, aide aSc] | **Oui** [S-tiers], à confirmer sur fichier réel (`Experimental`) | 10 / X16 |
| **FX-82** | **FET** (logiciel libre) | E/S | données de projet en XML ; exports HTML/XML/CSV [S] ; **licence contradictoire GPL-2.0 / AGPL-3.0** [S] : on lit son **format**, on ne reprend **aucun code** | **Oui** | 10 / X16 |
| **FX-83** | **UnDeuxTEMPS** (AED) | E/S | liaison STS-web et NetSynchro documentée par l'éditeur [S] ; format ENT **NV** | fichiers réels | 10 / X16 |
| **FX-84** | **Tableur de créneaux** (format ScolIA documenté) | E | CSV/XLSX + mappage | **Oui** | 10 / X3 |
| **FX-85** | **Abonnement iCal** personnel (emploi du temps, travail à faire, événements, réunions) | S | iCalendar RFC 5545 [S] ; `webcal://` ou `https://` à jeton | **Oui** | 10, 19, 20 / **X15** |
| **FX-86** | **Calendriers externes ICS** (diocèse, mairie, association sportive, réseau d'établissements) | E | iCalendar | **Oui** : événements **proposés** au socle 19 ou 5, jamais publiés seuls | 19 / X15 |
| **FX-87** | **CalDAV** (RFC 4791 [S]) : écriture depuis un agenda externe | E/S | — | **non retenu en première version** (Q-X20) | — |

### 9.8 Annuaires sortants et partenaires

| N° | Flux | Format | Qui | Faisable | Lot |
|---|---|---|---|---|---|
| **FX-90** | **Google Workspace for Education** : comptes élèves et personnels, groupes de classe | OneRoster (via outils de synchronisation) ou API ; SCIM pour certaines liaisons [S] | administrateur de l'établissement | **Oui** (OneRoster CSV d'abord) | X17 |
| **FX-91** | **Microsoft 365 Education — School Data Sync** | CSV **v2.1**, OneRoster API ou CSV v1 ; SDS Classic arrêté le 31 décembre 2024 [S, Microsoft] | idem | **Oui** après vérification sur l'environnement de test (`Experimental`) | X17 |
| **FX-92** | **Apple School Manager** | SFTP de CSV (modèles Apple ou OneRoster) ou OneRoster API (`/v1p1` ou `/v1p2`) ; synchronisation quotidienne ; domaine vérifié requis [S, Apple] ; version OneRoster acceptée contradictoire [S] | idem | **Oui** | X17 |
| **FX-93** | **OneRoster 1.2** (1EdTech) générique | CSV ou REST ; norme actuelle, pas de 1.3 prévue [S-tiers, extrait 1EdTech] | — | **Oui** (norme publique) | X17 |
| **FX-94** | **SCIM 2.0** (RFC 7643/7644 [S]) | REST | — | plus tard | X17 bis |
| **FX-95** | **Partenaires** (restauration municipale ou prestataire, transport régional, logiciel du CDI, plateformes de ressources hors GAR) | **API ScolIA** restreinte : client OAuth2 par partenaire et par établissement, périmètre et champs déclarés, consentement du chef d'établissement, journal ; webhooks signés | direction | contrat conçu ici, construit après le pilote (Q-X22) | X19 bis |
| **FX-96** | **SSO sortant** : ScolIA fournisseur d'identité pour Google, Microsoft, partenaires (« une passkey pour tout ») | OIDC ; SAML si un service l'exige | administrateur | après X11 (Q-X17) | X11 bis |

### 9.9 Autres établissements, reprise, réversibilité

| N° | Flux | Sens | Format | Règle | Lot |
|---|---|---|---|---|---|
| **FX-100** | **Dossier de transfert** vers un autre établissement (exeat, dernier bulletin, livret) | S | PDF signés (socle 7) + format de transfert ScolIA documenté | à la demande de l'établissement d'accueil vérifiée (`03_INSCRIPTIONS.md` INS-75) ou de la famille ; **entre deux établissements ScolIA, aucun transfert automatique** (D-013) : la famille porte la demande | X7 |
| **FX-101** | **Dossier reçu** d'un autre établissement | E | PDF, livret, format ScolIA | pièces au socle 7 ; INE `ancien_etablissement` (`a_verifier`) ; propositions | X3 |
| **FX-102** | **Reprise d'un ancien logiciel** (Charlemagne, École Directe, Pronote, GEPI…) à la mise en service | E (une fois) | exports CSV/Excel de ces logiciels [F Next : import Excel générique `eleves-import.ts`] ; formats exacts **NV** | moteur + connecteurs `reprise.*` (mappage mémorisé, propositions en masse, rapport) ; pilotée par le socle 28 | X3, socle 28 |
| **FX-103** | **Tableur générique** (élèves, responsables, personnels, salles, matières, groupes, options) | E | CSV (séparateur détecté), XLSX ; **mappage de colonnes** proposé, mémorisé par établissement et par source | toujours en propositions | **X3** |
| **FX-104** | **Paquet de réversibilité** (SDET : plan et liste des données éligibles à la reprise [S]) et **export complet d'un dossier** (portabilité, droit d'accès, T-26) | S | archive : JSON documenté (schéma publié), CSV par entité, documents (PDF/A), manifeste et empreintes | déclenché par la direction (★) ou par le socle 27 (droit d'accès) ; format **public et stable** : un argument commercial | X7, socle 27 |
| **FX-105** | **Format de transfert ScolIA** (`scolia.transfert.v1`) | E/S | même schéma que FX-104, restreint à un élève | sert aussi de format documenté pour les éditeurs tiers | X7 |

### 9.10 Matrice public / privé / hors contrat

| Brique | Public (collège, lycée) | Privé sous contrat | Hors contrat | Agricole |
|---|---|---|---|---|
| Source de l'état civil et de la structure | SIECLE (fait foi) via AAF ou exports | **ScolIA** (fait foi), remonté vers BEE | ScolIA | Fregata [S] |
| Entrée des élèves | AAF (FX-01) ou exports BEE (FX-02) | saisie et inscriptions ScolIA ; exports BEE en contrôle | saisie, tableur | Fregata [NV] |
| Personnels | AAF, STS-web (FX-03) | STS-web pour les maîtres [H], saisie pour les salariés OGEC | saisie, tableur | [NV] |
| Remontée nationale | — (SIECLE est la base) | **BEE** (FX-30, agrément) | aucune | Fregata |
| ÉduConnect / guichets Agents | **obligatoires** (arrêté 2026, art. 3 ; annexe 4 critères 4 et 5 ; vie scolaire : 10 et 22 mois [V#1–3]) | volontariat [S] ; hors champ de l'arrêté [V#1] | non [H] | [NV] |
| GAR | **obligatoire** (critère 7 ; vie scolaire : 22 mois [V#2–3]) ; raccordement par un **porteur public** [V#7] | ouvert (arrêté du 18/12/2017) [V#6] ; voie de raccordement d'un privé isolé **NV** (Q-X45) | non [H] | ouvert (agricole privé sous contrat) [V#6] |
| LSU, LSL, Parcoursup | éditeur validé | éditeur validé | non (candidats libres) | [NV] |
| Facturation | Op@le, agent comptable (hors ScolIA pour l'encaissement) | ScolIA (OGEC) | ScolIA | — |
| Arrêté de 2026 | s'impose aux collèges et lycées publics [V#1] | **non visé** [V#1] (exigible seulement par contrat) | non [V#1] | privé agricole non visé [V#1] ; lycées agricoles **publics** (« lycées publics » ?) **NV** |
| Interlocuteur | DSI académique, DNE | DNE (agrément), SGEC, UDOGEC | aucun national | ministère de l'agriculture, CNEAP |

### 9.11 Ce qui est faisable sans aucun accès officiel

**Tout le parcours minimal vendable** : tableurs (FX-103), exports SIECLE et STS-web déposés par l'établissement (FX-02, FX-03), élèves affectés en tableur (FX-05), référentiels ouverts (FX-51, 53, 54, 55 ; FX-50 et 52 après licence), comptes rendus déposés à la main (FX-08), pré-contrôles et **aides à la saisie** dans LSU, LSL, Parcoursup (exports de travail lisibles, jamais de faux format), SEPA et relevés (FX-70, 71), abonnements iCal (FX-85), annuaires sortants OneRoster/SDS/Apple (FX-90–93), reprise d'anciens logiciels (FX-102), réversibilité (FX-104). **Ce qui ne l'est pas** : AAF, ÉduConnect, Agents, GAR, remontée BEE, ONDE, LSU/LSL/Parcoursup officiels, STS-web en écriture, Notanet, Cyclades.

### 9.12 Flux ajoutés par la passe personas **[ajout personas]**

Même grille que les fiches précédentes, condensée ; même règle : un format [NV] ne se code pas.

| N° | Flux | Sens | Qui, quand | Format | Faisable | Prop. / Lot |
|---|---|---|---|---|---|---|
| **FX-09** | **Résultats aux examens** (DNB, baccalauréat, CAP) rapatriés dans le dossier | E | secrétariat ; juillet | extraction Cyclades ou publication des résultats **NV** | tableur en attendant | 38 / X3 |
| **FX-41** | **États de service et heures supplémentaires des maîtres** (privé sous contrat), missions | S | direction ; mensuel | application et format **NV** | état de travail signé (socle 7) | 6, 49 / X7 |
| **FX-42** | **Enquête annuelle sur les apprentis** (SIFA [H]) | S | unité de formation d'apprentis ; annuelle | **NV** | export de travail | 44 / X7 |
| **FX-43** | **Plateforme des stages de seconde** [H] | E/S | chef de travaux, professeurs principaux ; juin | **NV** | — | 44 |
| **FX-56** | **Référentiel des entreprises** (base Sirene téléchargée [S : Licence Ouverte 2.0, V#34] ; les unités en **diffusion partielle** (« P ») ne sont pas affichées) | E (global) | conventions de stage | fichiers de stock ; **jamais d'interrogation en ligne portant une donnée d'élève** ; entrepreneurs individuels = personnes (minimisation) ; la taxe d'apprentissage (plateforme nationale [H]) n'appelle qu'une UAI et des coordonnées d'établissement à jour | oui après licence | 44 / X4 |
| **FX-66** | **Licences sportives** UGSEL (enseignement catholique) et UNSS ; outil national Pix pour un établissement sans GAR [H] | S | secrétariat, professeur d'EPS (lecture de la liste seulement), documentaliste ; rentrée | CSV des plateformes **NV** ; export de travail tracé, champs minimaux | oui (export de travail) | 9 / X7 |
| **FX-67** | **Assurance scolaire collective** (effectifs ou liste) | S | secrétariat ; rentrée | **NV** | oui (export de travail, agrégé si possible) | 9 / X7 |
| **FX-75** | **Relevés CFONB 120** (en plus de camt) ; **canal EBICS** plus tard | E | comptable ; quotidien ou mensuel | format interbancaire français [H, très répandu] | oui | 25 / X18 |
| **FX-76** | **Reprise des mandats SEPA** (RUM, date de signature, séquence, identifiant créancier) et des échéanciers | E (une fois) | comptable ; avant la première remise | depuis l'ancien logiciel (FX-102) ; règles de reprise **NV** (D-20) | oui, **bloquant pour le privé** | 25 / X18, socle 28 |
| **FX-77** | **Déclaration sociale nominative** et comptes rendus | S + R | paie (socle 49) | **NV** ; même cycle que §6.10 (pré-contrôle, dépôt, retour) | au socle 49 | 49 / X7 |
| **FX-78** | **Facturation électronique** (réforme 2026–2027) et Chorus Pro (factures à une collectivité) | S | comptable | une association **non assujettie à la TVA** « n'est pas concernée par la réforme » ni par la réception au 1/09/2026 ; une association **assujettie, même exonérée**, doit pouvoir **recevoir** dès le 1/09/2026 [V, fiche DGFiP association, juillet 2026, V#38] ; statut TVA de chaque OGEC (cantine, internat) à établir ; Chorus Pro pour un non-assujetti **NV** (Q-X42) | après vérification | 48 |
| **FX-88** | **Restauration et badges** : export des ayants droit, import des passages (logiciel de cantine) | E/S | gestion, vie scolaire ; quotidien | CSV des logiciels du marché **NV** ; tableur documenté d'abord | oui (tableur) ; module « Passage » de Charlemagne | 40 / X3 |
| **FX-97** | **Annuaire local** de l'établissement (annuaire Windows, serveur pédagogique) | S | informaticien | CSV, LDIF ; jamais de mot de passe | oui | 9 / X17 |
| **FX-98** | **Reprise d'un annuaire existant** (export Google ou Microsoft) pour rapprocher les comptes avant provisionnement | E (une fois) | informaticien | CSV des consoles [H] | oui | 9 / X17 |
| **FX-99** | **SSO entrant depuis l'ENT de la collectivité** (ScolIA service de vie scolaire raccordé) | E (identité) | public ; quotidien | CAS, SAML ou OIDC selon l'ENT **NV** ; mêmes règles que §10 (aucune création, rattachement par jointure ou code, tests négatifs) | après D-17 | 1, 9 / **X8 bis** |
| **FX-106** | **Reprise structurée de l'historique scolaire de N-1** (moyennes et appréciations par période et par matière ; soldes d'ouverture) | E (une fois) | secrétariat, comptable ; avant la rentrée | depuis l'ancien logiciel ; **obligatoire pour un lycée** (LSL et Parcoursup de terminale) | oui (tableur, mappage) | 21, 23, 25, 37 / X3, socle 28 |
| **FX-107** | **Journal « à reporter dans le logiciel agréé »** (cohabitation) | S (liste de travail) | secrétariat ; au fil des mouvements | liste ScolIA, pointage ; export lisible pour la saisie | oui | 9 / **X7 bis** |
| **FX-108** | **Photos du photographe scolaire** (lot de fichiers + table de correspondance) | E | secrétariat ; rentrée | ZIP + CSV ; noms de fichiers jamais exécutés ni utilisés comme chemins | oui | 2 / X3 |

Notes sur les fiches existantes : **FX-34** couvre aussi le livret de la voie professionnelle [NV] ; **FX-38** pré-contrôle les durées de PFMP et positionnements [NV] ; **FX-04** reçoit une aide à la saisie dans ONDE tant qu'il est bloqué ; **FX-95** exige un bac à sable à données fictives, une documentation versionnée, une politique de dépréciation (6 mois) et le journal des lectures (ITP-147), et couvre le logiciel du CDI ; **FX-102** suit le catalogue de reprise (R-X23) et la démarche D-19.

---

## 10. Identité fédérée

### 10.1 Principes [R]

1. **Authentifier n'est pas autoriser.** Un fournisseur fédéré prouve « c'est bien cette personne » ; les droits viennent de ScolIA (rôles D-017 pour les personnels, liens du socle 4 pour les familles, compte élève du socle 2). Un attribut « profil enseignant » reçu d'un fournisseur n'accorde rien ; il peut **suggérer** un modèle de rôle à l'import, que l'établissement confirme.
2. **Aucune création à la connexion.** Une identité fédérée inconnue reçoit « votre établissement ne vous a pas encore enregistré ; contactez son secrétariat » — même message que l'identité existe ou non ailleurs (anti-énumération, D-013).
3. **Rattachement par preuve forte** : clé de jointure issue d'un import officiel (AAF, BEE), ou code de rattachement remis par l'établissement. **Jamais** par e-mail, nom ou date de naissance seuls (R-X11).
4. **Par établissement.** Un rattachement vaut dans un établissement ; une même identité ÉduConnect (un parent de deux enfants dans deux établissements ScolIA) a deux rattachements, et aucun établissement ne voit l'autre.
5. **Une identité de personne, plusieurs moyens de connexion** (D-013, D-020) : passkey, ÉduConnect, guichet Agents, SSO d'établissement s'attachent au même `ent_user` ; on lie un nouveau moyen **en étant déjà connecté** par un moyen existant, ou par code.
6. **Sous-domaine par établissement conservé** (D-012) grâce au guichet central (§10.2).
7. **Le protocole est une brique remplaçable** : OIDC par `ent-oidc` existant ; SAML 2.0 dans une brique isolée **seulement si** un fournisseur l'impose (Q-X5).

### 10.2 Le guichet central de fédération

Les fournisseurs nationaux enregistrent un fournisseur de service avec des adresses de retour fixes [H] ; ScolIA a un sous-domaine par établissement (D-012) et un cookie de session par hôte (`__Host-ent_session`). Proposition [R, Q-X3] :

```
famille sur lpnb.scolia.fr ─clic « ÉduConnect »─▶ connexion.scolia.fr/educonnect?etab=lpnb (état signé, PKCE)
   ─▶ ÉduConnect ─▶ connexion.scolia.fr/retour (assertion vérifiée : signature, audience, échéances, rejeu)
   ─▶ recherche des rattachements de cette identité (issuer, sujet) DANS l'établissement demandé
        ├─ trouvé et actif ─▶ ticket à usage unique (60 s, lié à l'hôte lpnb.scolia.fr, à l'état, au navigateur) ─▶ lpnb.scolia.fr/fed/ticket ─▶ session
        ├─ jointure possible (identifiant présent dans un import de cet établissement) ─▶ rattachement `propose` ─▶ confirmation (« êtes-vous bien … responsable de … ? ») ─▶ ticket
        └─ rien ─▶ page neutre « contactez l'établissement » + saisie d'un code de rattachement s'il en a reçu un
```

- **Ce que le guichet peut lire** [relecture, obj. 1] : c'est un lecteur inter-établissements, donc une **exception déclarée** au cloisonnement (§16.2) : rôle SQL dédié qui ne voit que `ent_fed_rattachement`, `ent_fed_ticket`, `ent_fed_tentative` et la liste des établissements (hôtes) ; l'empreinte du sujet utilise une **clé de pseudonymisation globale** au coffre (D-16), seule façon de retrouver les établissements d'une même identité ; le paramètre `etab` n'est accepté que s'il désigne un hôte de la plateforme (liste blanche, pas de redirection ouverte) ; test d'isolation dédié.
- **Rien n'est montré avant preuve** [relecture, obj. 2] : dans la branche « jointure possible », **aucun nom** (ni de l'enfant, ni de la personne) n'est affiché ; le rattachement est confirmé par un second élément (code de rattachement, ou validation par le secrétariat sur sa liste « rattachements à confirmer ») ; les trois branches renvoient la **même page** à la personne (« si votre établissement vous a enregistré, votre accès sera ouvert ; vous pouvez aussi saisir un code ») tant qu'aucune preuve n'est complète ; débit limité par sujet.
- **Liaison au navigateur** [relecture, obj. 3] : avant de rediriger vers le guichet, l'hôte de l'établissement pose son **propre** cookie de liaison (`__Host-`, aléatoire, 10 min) et transmet son empreinte dans l'état signé ; `/fed/ticket` exige ce cookie et l'empreinte correspondante, puis émet une **nouvelle** session (pas de fixation).
- Le guichet central **ne garde aucune session** : il ne fait que vérifier l'assertion et remettre un ticket. Pas de cookie de session sur `connexion.scolia.fr` (sauf l'état de la transaction, 10 min).
- **Point d'entrée sans établissement** (application mobile, ou parent qui arrive de Scolarité services) : après authentification, le guichet affiche **à la personne seule** la liste des établissements où elle est rattachée (D-013 : la personne connaît ses établissements ; les établissements ne se connaissent pas) ; s'il n'y en a qu'un, redirection directe.
- Déconnexion : la session ScolIA est locale (D-011) ; la déconnexion unique (SLO) du fournisseur est honorée **si** le protocole l'exige [NV] ; un changement côté fournisseur (compte désactivé) est reflété à la prochaine connexion, pas en direct.

### 10.3 ÉduConnect (élèves et responsables)

| Point | Ce qu'on sait | Conséquence |
|---|---|---|
| Rôle | Guichet national d'identification, d'authentification et de gestion des comptes des élèves et de leurs représentants légaux [S, SDET] ; déployé dans tout le public depuis décembre 2020 ; **volontariat** dans le privé sous contrat [S] | activable par établissement ; **obligatoire** pour un collège ou lycée public : « Interfaçage avec ÉduConnect en cas de nécessité d'identification et d'authentification des élèves et de leurs responsables » (annexe 4, **critère n° 4**, **10 mois** pour un service de vie scolaire, soit vers **juillet 2027** [V#2–3]) ; le SDET 2024 : l'ENT « délègue l'identification de l'élève à ÉduConnect » [V, V#12] |
| Activation par le parent | être référencé comme responsable dans la base élèves avec un téléphone ou un e-mail à jour ; code SMS ; **FranceConnect** possible **dans** ÉduConnect ; un compte responsable pour tous les enfants [S, guide ÉduConnect] | FranceConnect n'est **pas** un fournisseur à raccorder séparément [S, étude §13] ; la qualité des données SIECLE conditionne l'accès : le pré-contrôle signale les responsables sans téléphone ni e-mail |
| Protocole | point d'entrée SAML 2.0 observé en production [S, étude §4.1] ; OIDC = cible du référentiel d'interopérabilité [S ; annexe 2 de l'arrêté **non lue** le 2026-11-09, V#5] ; critère 18 : « protocoles ouverts et documentés (mentionnés dans le référentiel d'interopérabilité) » pour l'authentification, 10 mois [V#2] ; **contrats d'interface sur demande lors de l'accrochage** [S] ; aucune documentation publique de raccordement trouvée (V#12) | lot **X9 bloqué** jusqu'au contrat ; abstraction de protocole dès X8 |
| Attributs | `FrEduVecteur`, `ENTPersonJointure`, lien responsable → enfants, présence de l'INE : **NV** (aucune source trouvée par les recherches du 2026-11-08) | **ne rien coder** sur une hypothèse d'attribut ; le rattachement utilisera ce que le contrat donnera ; à défaut, code de rattachement |
| Éligibilité | un éditeur (et non une collectivité) peut-il se raccorder ? un ENT d'un seul établissement privé ? **NV** | démarche D-4 |

### 10.4 Guichet Agents (personnels)

Service d'authentification des personnels (rectorats, établissements, enseignants), donnant accès aux applications académiques [S, extrait] ; sélection « Authentification Éducation nationale » puis guichet de l'académie dans apps.education.fr [S, guide académique] → **probablement un guichet par académie** [H]. L'arrêté de 2026 parle des « guichets Agents » **au pluriel** [V#14]. Protocole, attributs, accès des **maîtres du privé sous contrat** : **NV** (recherchés le 2026-11-09, rien de public). Exigé dans le public (annexe 4, **critère n° 5**, **22 mois** pour un service de vie scolaire, soit vers juillet 2028 [V#2–3]). Lot **X10 bloqué**. Les personnels non enseignants de la collectivité (agents d'entretien, de restauration) n'y sont pas [H] : passkey locale. **ProConnect** (ex-AgentConnect, ouvert à l'interministériel et au privé [S, titre]) : pertinence pour l'Éducation nationale en 2026 **NV**, veille seulement.

### 10.5 SSO d'établissement (Google, Microsoft, Keycloak…)

Existant `ent-oidc` [F]. À changer (X8) : rattachement **par identifiant d'annuaire** quand ScolIA a lui-même provisionné le compte Google ou Microsoft (FX-90/91 : l'identifiant renvoyé à la création est une preuve forte), sinon par code ; le rattachement par e-mail vérifié est **limité aux personnels** dont l'e-mail professionnel est déjà confirmé dans ScolIA, et désactivable ; secret client au coffre (§2.2 n° 11).

### 10.6 Rattachement : règles

| R-X | Règle |
|---|---|
| R-X11 | Preuves admises : (a) clé de jointure d'un import officiel confirmé de **cet** établissement, égale à l'attribut reçu ; (b) code de rattachement valide ; (c) identifiant d'annuaire provisionné par ScolIA ; (d) personnel : e-mail professionnel confirmé, si l'établissement l'autorise. Rien d'autre. |
| R-X12 | **Code de rattachement** : 10 caractères sans ambiguïté (pas de 0/O, 1/l), usage unique, 30 jours, lié à une personne et un établissement, empreinte seule en base ; remis par courrier, en main propre, par lien nominatif (socle 7) ou affiché au guichet ; 5 essais par identité et par heure, puis attente ; journalisé. Lot de codes imprimables pour la rentrée (une feuille par famille, cachetée par foyer, D-014 : **un code par parent**, jamais un par famille). |
| R-X13 | Un rattachement est **suspendu** quand le lien qui le fonde est suspendu (interdiction, autorité retirée, départ de l'agent, élève sorti) — relu à chaque connexion **et** les sessions ouvertes de la personne dans cet établissement sont révoquées au moment de la suspension (D-011). |
| R-X14 | Lier un deuxième moyen (ÉduConnect à un compte qui a déjà une passkey) exige d'être connecté avec assurance forte, ou un code ; jamais par simple concordance d'e-mail. |

**[ajout personas] Rentrée et chaîne officielle (Q-X35, contradiction C12).** Le second élément exigé au §10.2 (obj. 2) donne, à la rentrée d'un collège public, 1 300 validations à la main ; elles seront faites en masse sans lecture. Proposition : quand la jointure vient de la **chaîne officielle complète** (import AAF ou BEE confirmé de cet établissement **et** attribut signé du fournisseur, qui a lui-même vérifié la personne) et que le lien du socle 4 est actif sans mesure, le rattachement est **confirmé automatiquement** ; la validation humaine est réservée aux discordances (attribut absent, lien suspendu, homonymie, personne récemment révoquée, R-X19). Rien n'est affiché avant preuve (obj. 2 inchangée).

### 10.7 Passkey (D-009) et connexion fédérée — **dérogation à D-009 proposée, à faire valider** (Q-X1)

D-009 veut une passkey pour tous ; l'arrêté de 2026 impose aux collèges et lycées publics l'interfaçage avec ÉduConnect et les guichets Agents « en cas de nécessité d'identification et d'authentification » (art. 3, annexe 4 critères 4 et 5 [V#1–2]). Exiger une passkey **en plus** d'ÉduConnect doublerait la connexion et contredirait l'esprit de l'exigence [H]. **Recommandation** : (1) dans un établissement où un fournisseur national est activé, la connexion fédérée **suffit** pour l'usage courant ; (2) les actions sensibles ★ (signature, changement d'IBAN, export, validation d'une diffusion) exigent une **assurance forte** : passkey récente **ou** connexion fédérée récente dont le niveau d'authentification déclaré est multifacteur [NV : niveau exposé par ÉduConnect] ; (3) la passkey reste **proposée** (et obligatoire pour les comptes locaux, conformément à D-009) ; (4) une connexion locale de secours (passkey) reste possible pour un personnel quand le guichet académique est en panne, si le cadre l'admet [NV, question D-4/D-5].

### 10.8 ScolIA fournisseur d'identité

- **Pour qui** : le GAR d'abord (§11) ; puis Google, Microsoft et des partenaires (FX-96).
- **Protocole** : OIDC de préférence (le contrat SSO ENT du GAR 8.2, avril 2025, admet **OIDC depuis sa version 3.0 du 12/10/2022** à côté de SAML 2.0 [V, V#8]) ; SAML 2.0 si exigé (même brique isolée qu'en entrée, Q-X5).
- **Clés de signature** : hors base, dans le coffre (KMS du fournisseur d'hébergement [H : service Scaleway à vérifier]) ; rotation planifiée avec publication anticipée de la nouvelle clé (JWKS ou métadonnées) ; journal de chaque signature par lot (pas par assertion) ; révocation documentée.
- **Identifiants transmis** : **identifiant de service** opaque, aléatoire, stable, **différent par service** et **jamais réutilisé**, jamais l'INE, l'UUID interne ni l'e-mail (`02_ELEVE.md` §7.3) ; table `ent_idp_identifiant_service`.
- **Consentement et minimisation** : chaque service déclare ses attributs ; l'établissement active le service ; les familles sont informées (mentions, socle 27).

### 10.9 Cas limites de la fédération (extraits du §12)

Parent avec deux enfants dans deux établissements ScolIA (CX-20) ; parent séparé dont le lien est suspendu (CX-21) ; élève devenu majeur (CX-22) ; homonymes (CX-23) ; responsable sans téléphone ni e-mail dans SIECLE, donc sans ÉduConnect (CX-24) ; professeur qui enseigne dans deux établissements de deux académies (CX-25) ; compte ÉduConnect d'un élève réutilisé par un parent (CX-26) ; panne d'ÉduConnect un matin de rentrée (CX-27).

---

## 11. GAR (gestionnaire d'accès aux ressources)

| Point | Ce qu'on sait | Conception |
|---|---|---|
| Rôle | Gère les autorisations d'accès aux ressources numériques présentées dans les **médiacentres** des ENT (ou ÉduGAR) ; « 97 % des EPLE du second degré (rentrée 2023) », 51 % des écoles [V, page d'accueil gar.education.fr, V#10 ; le « 99 % » antérieur n'est pas retrouvé] ; ouvert aux « établissements d'enseignement privés sous contrat » et à l'agricole privé sous contrat (arrêté du 18 décembre 2017) [V, V#6] | brique `federation-idp`, module `gar`, activable |
| Raccordement | « Le Porteur de projet ENT (maître d'ouvrage) transmet au ministère sa demande d'inscription via la **DANé** » ; le porteur est « **la puissance publique** qui assure la maîtrise d'ouvrage » ; l'exploitant (éditeur) réalise l'accrochage ; dossier d'inscription (code projet conforme SDET, métadonnées IdP SAML **ou OIDC** de test, clé SSH SFTP, CSR du webservice ListeRessources) [V, V#7] | **ScolIA ne peut pas se raccorder seul** : il lui faut un porteur public. Dans le public : le projet ENT de la collectivité, ou un projet porté par l'académie. Dans le privé isolé : **aucun cas prévu** dans les documents (Q-X45, D-6) |
| Affectations | Faites **par l'établissement** dans la **console d'affectation** du GAR (nouvelle version du 22/04/2025) [V, V#11] — **pas** par l'ENT | correction de `SOCLES.md` §9 (« transmission des affectations ») : l'ENT transmet **l'annuaire** et **l'identité**, pas les affectations |
| SSO | Le projet ENT génère l'assertion (SAML 2.0 ou OIDC) ; attributs `idEnt` et `GARPersonIdentifiant` présents dans l'exemple d'assertion du contrat SSO ENT 8.2 [V, V#8] ; `UAI` non relu ; document marqué « diffusion restreinte » : citer, ne pas recopier | ScolIA IdP (§10.8) ; `GARPersonIdentifiant` = identifiant de service ; profils déduits des rôles par une table de correspondance (jamais l'inverse) |
| Alimentation | Archive de fichiers XML `GAR-ENT-2D.xsd` / `GAR-ENT-1D.xsd`, nommage, MD5, XSD [S] ; canal **SFTP par clé SSH** [V, V#9] ; liste des fichiers, différentiel, fréquence **NV** (paquetage XSD remis à l'inscription) | générateur `gar.alimentation` sur le moteur d'export (instantané, inclusions, retour) ; **bloqué** jusqu'aux XSD (X12) |
| Responsables d'affectation | désignés par l'établissement [S, FAQ console] | action ITP-75 ; transmis dans l'alimentation [H] |
| Projet ENT (`idEnt`) | un éditeur multi-académies est-il un ou plusieurs projets ? **NV** | Q-X7 ; ne pas figer le modèle : `idEnt` est un paramètre du connecteur, par établissement |
| Médiacentre | ressources affectées, ouvertes via le GAR [S] ; API exacte **NV** | écran Leptos (élève, professeur) ; diagnostic « pourquoi je ne vois pas cette ressource » (affectation absente, élève hors groupe, alimentation en retard) |
| Minimisation | l'alimentation ne contient pas l'INE [H, `02_ELEVE.md` §8.2] | pré-contrôle : aucun champ hors déclaration |

---

## 12. Cas limites

| N° | Situation | Ce que fait ScolIA |
|---|---|---|
| CX-01 | Fichier SIECLE d'un **autre établissement** déposé par erreur | refus à l'identification (UAI), contenu non lu ni conservé au-delà de la quarantaine ; journal ; message clair |
| CX-02 | Fichier de **l'an dernier** | refus (année) ; si « préparer la rentrée », lot rattaché à l'année N+1 |
| CX-03 | Fichier **plus ancien** que le dernier appliqué | avertissement bloquant, levé par ★ |
| CX-04 | Même fichier déposé deux fois | « déjà importé le … » ; réanalyse possible, pas de double application |
| CX-05 | 40 % des élèves « disparaissent » (export partiel : une seule division cochée) | garde-fou de volume → `bloque_volume` ; disparitions jamais appliquées |
| CX-06 | Une correction d'orthographe du nom faite au guichet, SIECLE inchangé | `correction_locale_maintenue` : rien n'est proposé ; écart persistant (« à corriger dans SIECLE ») |
| CX-07 | SIECLE corrige aussi, autrement (« Dupont-Martin » contre « Dupont ») | `conflit` : arbitrage humain, preuve des deux côtés |
| CX-08 | Prénom d'usage choisi par l'élève (`02_ELEVE.md` §7.2), SIECLE ne porte que l'état civil | le prénom d'usage n'est pas un champ de la source : jamais touché ; les exports nationaux utilisent l'état civil |
| CX-09 | Élève à confidentialité renforcée | aucune application automatique sur son identité ; ses enregistrements de lot ne sont visibles qu'avec le droit dédié |
| CX-10 | Adresse protégée d'un responsable (socle 4) et SIECLE envoie une adresse | proposition **scellée**, visible des seuls détenteurs du droit ; jamais exportée en clair (R-X15) |
| CX-11 | Deux parents séparés avec la même adresse dans SIECLE (ancienne) | aucun rapprochement de foyer par adresse ; deux personnes, deux foyers si la source les distingue par identifiant ; sinon proposition à arbitrer |
| CX-12 | Le même INE arrive sur deux élèves | `conteste` (socle 2) + alerte doublon ; aucune fusion par le moteur |
| CX-13 | Élève importé sans INE (arrivée de l'étranger, maternelle) | normal ; pré-contrôle avant export national |
| CX-14 | Élève parti en cours d'année absent du fichier suivant | « sortie probable » proposée, avec la date du fichier ; jamais de sortie automatique (la date réelle est souvent antérieure) |
| CX-15 | Division renommée dans SIECLE (« 3A » → « 3ᵉ A ») | correspondance mémorisée par code source : renommage proposé, pas de nouvelle classe |
| CX-16 | Groupe défini dans ScolIA sans équivalent SIECLE (atelier théâtre) | ignoré par l'import ; jamais supprimé ; non exporté vers BEE sauf déclaration |
| CX-17 | Import pendant qu'un secrétaire modifie la même fiche | revérification par paquet : la ligne est recalculée, la saisie n'est pas écrasée |
| CX-18 | Panne au milieu de l'application | `applique_partiellement`, reprise au paquet suivant, idempotence |
| CX-19 | Annulation demandée après que des absences ont été saisies sur un élève créé par le lot | refus pour cet élève, listé dans le rapport ; annulation du reste |
| CX-20 | Parent avec deux enfants dans deux établissements ScolIA, se connecte par ÉduConnect | deux rattachements ; au guichet central, il voit ses deux établissements ; aucun des deux ne voit l'autre |
| CX-21 | Parent dont le lien est suspendu (interdiction) | rattachement suspendu, connexion refusée avec message neutre ; journal |
| CX-22 | Élève devenu majeur | son compte reste le sien ; les droits des parents suivent le socle 4 ; aucune incidence sur le rattachement |
| CX-23 | Homonymes parfaits (même nom, prénom, date de naissance) dans une division | rapprochement par identifiant seulement ; par identité, proposition « même élève ? » jamais automatique |
| CX-24 | Responsable sans téléphone ni e-mail dans SIECLE (pas d'ÉduConnect possible) | pré-contrôle « responsables non activables » ; code de rattachement pour un compte ScolIA local |
| CX-25 | Professeur dans deux établissements de deux académies | deux rattachements Agents (un par guichet [H]), une identité ScolIA |
| CX-26 | Parent qui se connecte avec le compte ÉduConnect de son enfant | le rattachement est celui de l'élève : il voit l'espace élève ; profil incohérent (âge) signalé à la personne, pas d'élévation |
| CX-27 | ÉduConnect en panne le matin de la rentrée | message clair ; les sessions ouvertes continuent ; passkey proposée à ceux qui en ont une ; aucune connexion « de secours » sans preuve |
| CX-28 | Remontée BEE rejetée pour 3 élèves | retour importé ; 3 tâches ; les autres élèves sont acceptés ; nouvel export partiel après correction |
| CX-29 | XSD officiel change en cours d'année | nouvelle version déclarée du connecteur ; l'ancienne reste pour régénérer les exports passés ; pré-contrôle des deux pendant la transition |
| CX-30 | Nomenclature BCN : une matière est retirée | aucun effet destructeur : contrôle « code retiré » sur les matières qui la référencent (`05_STRUCTURE.md` §9.4) |
| CX-31 | Commune de naissance fusionnée (commune nouvelle) | COG historique : la commune reste valide à la date de naissance ; l'export utilise le code en vigueur à la date demandée par le format [NV] |
| CX-32 | Élève né à l'étranger | pays COG ; commune en texte libre |
| CX-33 | Abonnement iCal d'un parent après la sortie de l'enfant | abonnement expiré à la date de sortie ; le flux renvoie un calendrier vide signé « abonnement terminé », pas d'erreur révélatrice |
| CX-34 | URL d'abonnement iCal publiée par erreur sur un réseau social | rotation du jeton en un geste ; contenu minimisé (aucun nom d'autre élève, aucune salle si l'établissement le choisit) ; journal des lectures par adresse réseau tronquée |
| CX-35 | Établissement à deux UAI (collège + lycée) sous un tenant | les deux UAI sont acceptées à l'identification ; les exports nationaux sont produits **par UAI** |
| CX-36 | L'académie envoie l'AAF de tous ses établissements en un seul dépôt | réception centrale : découpage par UAI, quarantaine plateforme pour les inconnues (§6.9) |
| CX-37 | Agent de la collectivité (cuisine) absent de STS et de l'AAF | saisie ou tableur ; passkey locale |
| CX-38 | Un import propose un changement d'identité pour un élève dont une procédure disciplinaire est en cours | aucune incidence : l'identité versionnée garde la valeur à la date des actes (`02_ELEVE.md` §6) |
| CX-39 | Un parent demande « à qui avez-vous transmis les données de mon fils ? » | traces d'export (§6.11) : liste des destinataires et dates, sans valeurs |
| CX-40 | Un professeur tente d'exporter un fichier LSU | refus (D-022) ; l'action `echanges.exporter` n'existe pas dans le modèle « professeur » |
| CX-41 | Le GAR renvoie des rejets pour des élèves sans groupe | retour → tâches au socle 5 (affectations) |
| CX-42 | Le relevé bancaire contient un virement d'un montant proche mais différent | proposition de lettrage avec écart, jamais lettrage silencieux (socle 25) |
| CX-43 | Fichier tableur avec des formules (`=HYPERLINK(...)`) dans une cellule | lu comme texte ; en sortie, toute cellule commençant par `=`, `+`, `-`, `@`, tabulation ou retour chariot est échappée |
| CX-44 | Fichier ZIP piégé (bombe, chemin `../`) | rejet à l'analyse, quarantaine, alerte sécurité sans contenu |
| CX-45 | Export Parcoursup fait, puis une note corrigée après le conseil | export **nouveau** (l'ancien est figé) ; écart montré entre les deux ; nouvelle remontée si la fenêtre est ouverte |
| CX-46 | Établissement qui quitte ScolIA | paquet de réversibilité (FX-104) ; correspondances et identifiants de service exportés pour la reprise par le nouveau titulaire, selon la liste des données éligibles [S, SDET] |
| CX-47 | **[ajout personas]** Élève arrivant refusé par BEE : « encore inscrit dans un autre établissement » | tâche avec l'UAI, le nom et les coordonnées publiques (annuaire des UAI) de l'établissement d'origine, modèle de message ; aucune information sur l'élève n'est cherchée ailleurs (D-013) |
| CX-48 | **[ajout personas]** Même responsable saisi deux fois dans l'ancien logiciel (une fiche par enfant, orthographes différentes) | proposition groupée « même personne probable » au socle 4 (fusion), jamais automatique |
| CX-49 | **[ajout personas]** Élève sorti l'an dernier qui revient | candidats incluant les sortis ; réouverture d'une scolarisation proposée, pas de nouvel élève |
| CX-50 | **[ajout personas]** Jumeaux (même date de naissance, mêmes responsables) | un prénom différent exclut la proposition « même élève » |
| CX-51 | **[ajout personas]** Plateforme nationale indisponible la veille de l'échéance | tentatives horodatées enregistrées (ITP-128), modèle de message à la DSDEN, alerte direction ; l'export reste prêt |
| CX-52 | **[ajout personas]** Bascule de juillet : 30 % de départs, élèves de N+1 sans division | départs attendus hors seuil (R-X20) ; propositions sans division, jamais de rejet |
| CX-53 | **[ajout personas]** Élève affecté présent dans SIECLE et dossier `affecte` au socle 3 | rapprochement avec le dossier par le port de correspondance du socle 3 ; pas de nouvel élève |
| CX-54 | **[ajout personas]** Nouveau conjoint déclaré responsable dans SIECLE avec un code de parenté vague | aucun lien d'autorité déduit ; proposition au socle 4 avec le code source affiché |
| CX-55 | **[ajout personas]** Parent sous interdiction, encore responsable dans SIECLE, qui se reconnecte par ÉduConnect | R-X19 : aucune nouvelle jointure ; message neutre ; journal ; alerte à la direction |
| CX-56 | **[ajout personas]** Élève qui change de prénom d'usage | GAR et annuaires reçoivent l'usage ; compte tiers renommé, identifiant de service inchangé ; exports nationaux à l'état civil (Q-X29) |
| CX-57 | **[ajout personas]** Élève à identité protégée et annuaire Google partagé du domaine | exclu par défaut (R-X18) ; inclusion décidée par la direction |
| CX-58 | **[ajout personas]** Remontée par webservice coupée après 300 élèves sur 600 | état `depose_partiellement` ; renvoi idempotent du reste (ITP-141) |
| CX-59 | **[ajout personas]** Fichier généré lundi, note corrigée mardi, ancien fichier déposé à la main mercredi | export **périmé** signalé à la déclaration de dépôt (R-X21), écart montré, nouvel export proposé |
| CX-60 | **[ajout personas]** Reprise : sanction de 2023 qui aurait dû être effacée (R. 511-13 IV [V#36]) | non reprise (R-X23), comptée dans le rapport « abandonné » |
| CX-61 | **[ajout personas]** Mandat SEPA repris sans date de signature | rejet de reprise lisible ; mandat à faire signer avant la première remise ; aucun prélèvement sur ce mandat |
| CX-62 | **[ajout personas]** Certificat du canal ou clé de l'IdP qui expire pendant les vacances | alertes J-30 et J-7 (R-X26) ; rotation planifiée |
| CX-63 | **[ajout personas]** Partenaire révoqué qui continue d'appeler l'API | refus, journal, alerte ; demande de purge relancée (R-X28) |
| CX-64 | **[ajout personas]** Compte d'un personnel compromis | révocation générale (R-X25), puis nouveau moyen de connexion par code ou passkey au guichet |
| CX-65 | **[ajout personas]** Compte rendu national reçu en PDF sans identifiant d'enregistrement | rattachement assisté rejet → enregistrement exporté (ITP-140), par l'instantané |

---

## 13. Actions métier

Abréviations : **SEC** secrétariat (scolarité, direction), **DIR** chef d'établissement ou adjoint, **ADM** administrateur de l'établissement, **CPT** comptable ou gestionnaire, **EDT** responsable de l'emploi du temps, **DOC** documentaliste ou référent numérique, **PER** tout personnel, **FAM** responsable légal, **ELV** élève, **SYS** tâche du système, **OPS** opérateur de la plateforme ScolIA (aucune donnée personnelle). ★ = assurance forte (passkey récente, D-009). Les droits sont ceux du §18.

### 13.1 Déposer et analyser

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-01 | Déposer un fichier (glisser-déposer) ; **détection automatique** du connecteur, de la version, de l'UAI, de l'année | SEC, DIR, EDT, CPT · `echanges.importer` | analyse socle 7 ; garde-fous §6.3 | `echanges.reception_recue` |
| ITP-02 | Choisir le connecteur quand la détection hésite (deux formats possibles) | idem | liste limitée aux connecteurs activés | — |
| ITP-03 | Déposer un lot de plusieurs fichiers liés (les cinq exports SIECLE) et laisser le moteur les ordonner (nomenclatures, structures, puis élèves, puis responsables) | idem | ordre déclaré par le connecteur | — |
| ITP-04 | Voir l'analyse : synthèse par classe de différences (nouveaux, modifiés par la source, corrections maintenues, conflits, disparus, rejets) | idem · `echanges.lire` | — | `echanges.lot_previsualise` |
| ITP-05 | Voir un rejet expliqué en clair, avec la ligne d'origine (valeurs sensibles masquées) et la correction attendue | idem | — | — |
| ITP-06 | Corriger un rejet **dans le lot** (valeur fautive du fichier) sans modifier le fichier source | SEC · `echanges.importer` | la correction est une proposition de l'utilisateur, tracée comme telle | — |
| ITP-07 | Ignorer un rejet avec motif | idem | motif codé | — |
| ITP-08 | Réanalyser un lot (après correction dans ScolIA ou changement de règle) | idem | même réception, nouvelle analyse, l'ancienne est conservée dans le journal du lot | — |
| ITP-09 | Abandonner un lot | idem | rien n'est appliqué | `echanges.lot_abandonne` |
| ITP-10 | Lever un garde-fou (fichier plus ancien, volume anormal, année N+1) | DIR · `echanges.appliquer` ★ | motif obligatoire ; journal | — |
| ITP-11 | Comparer deux lots d'un même connecteur (« ce qui a changé dans SIECLE depuis le 3 septembre ») | SEC · `echanges.lire` | — | — |
| ITP-12 | Prévisualiser l'effet d'un lot sur **un** élève, une personne, une classe | idem | — | — |

### 13.2 Rapprocher

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-13 | Confirmer un rapprochement proposé par identité (« même élève ? ») | SEC · `echanges.appliquer` + droit du domaine | mémorisé (correspondance) | `echanges.correspondance_confirmee` |
| ITP-14 | Refuser un rapprochement (homonyme) | idem | mémorisé comme « distinct » pour ne plus le reproposer | idem |
| ITP-15 | Rapprocher à la main un enregistrement d'une entité existante | idem | recherche bornée à l'établissement | idem |
| ITP-16 | Défaire une correspondance erronée | DIR ★ | les propositions futures repartent de zéro ; les applications passées ne sont pas défaites (annulation séparée) | idem |
| ITP-17 | Voir les correspondances d'un élève ou d'une personne avec chaque source | SEC · `echanges.lire` | — | — |
| ITP-18 | Rapprocher les **divisions et groupes** source avec ceux de ScolIA (une fois, mémorisé) | SEC, EDT | — | idem |
| ITP-19 | Rapprocher les **matières** locales des codes nationaux | SEC, EDT | proposition au socle 5 | — |

### 13.3 Décider et appliquer

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-20 | Accepter une proposition (champ par champ) | SEC · droit du domaine | la commande du domaine applique | événement du domaine (`origine = import`) |
| ITP-21 | Accepter une classe entière d'un connecteur (ex. tous les téléphones « modifiés par la source ») | SEC · `echanges.appliquer` ★ + droit du domaine | chaque proposition reste tracée | idem |
| ITP-22 | Refuser une proposition avec motif ; la mémoire de la source retient la valeur refusée (elle ne sera plus reproposée tant que la source ne change pas) | SEC | — | — |
| ITP-23 | Arbitrer un conflit (garder local, prendre la source, saisir une troisième valeur) | SEC · droit du domaine | preuve des deux côtés affichée | — |
| ITP-24 | Appliquer le lot (règles automatiques + décisions prises) | SEC, DIR · `echanges.appliquer` ★ | paquets, revérification (§6.7) | `echanges.lot_applique` |
| ITP-25 | Suivre l'application en direct (paquets passés, recalculés, refusés) | idem | — | — |
| ITP-26 | Reprendre une application interrompue | idem / SYS | idempotence | — |
| ITP-27 | Voir le rapport d'un lot (ce qui a été appliqué, par quelle règle ou par qui) | `echanges.lire` | — | — |
| ITP-28 | Annuler un lot (compensation) | DIR · `echanges.annuler` ★ | fenêtre 30 j ; ce qui ne peut être défait est listé | `echanges.lot_annule` |
| ITP-29 | Traiter les **disparus** : confirmer une sortie probable (date, motif), ou marquer « absent du fichier, toujours scolarisé » | SEC · droit du socle 2 | jamais automatique | — |
| ITP-30 | Transformer des élèves affectés (FX-05) en **dossiers d'inscription** `affectes` | SEC · `inscriptions.gerer` | socle 3 crée les dossiers | événement du socle 3 |

### 13.4 Écarts et qualité

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-31 | Voir les **écarts persistants** (corrections locales non reflétées par la source) | SEC · `echanges.lire` | — | — |
| ITP-32 | Exporter la liste des écarts pour corriger SIECLE (public) — export de travail, nominatif | SEC · `echanges.exporter` | trace d'export | `echanges.export_genere` |
| ITP-33 | Marquer un écart « assumé » (on sait, on ne corrigera pas la source) | SEC | motif | — |
| ITP-34 | Voir, par élève, la provenance de chaque champ (« INE confirmé par SIECLE, lot du 3 sept. ») — affichée dans la fiche (socle 2) | PER selon droits de la fiche | lecture publiée du moteur | — |
| ITP-35 | Voir les responsables **non activables** pour ÉduConnect (ni téléphone ni e-mail) | SEC | pré-contrôle | — |

### 13.5 Paramétrer les connecteurs

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-36 | Voir le **catalogue des connecteurs** : disponibles, expérimentaux, bloqués (avec la raison et la démarche en cours) | ADM, DIR · `echanges.lire` | — | — |
| ITP-37 | Activer, désactiver un connecteur | ADM · `echanges.connecteurs.gerer` ★ | profil d'établissement compatible | `echanges.connecteur_modifie` |
| ITP-38 | Configurer un canal automatique (SFTP, webservice) : hôte, identifiant ; **secret saisi une fois, jamais réaffiché** | ADM ★ | coffre ; test de connexion | idem |
| ITP-39 | Tester un canal (connexion, droits, sans transfert de données) | ADM | — | — |
| ITP-40 | Régler les règles d'application automatique d'un connecteur (suspendre une classe, exiger une revue humaine pour tel champ) | DIR · `echanges.regles.gerer` ★ | ce qui fait foi reste au domaine | idem |
| ITP-41 | Régler les seuils de garde-fou de volume (dans des bornes fixées par le logiciel) | DIR ★ | bornes : 5–30 % [R] | idem |
| ITP-42 | Choisir la politique de disparition (signaler / proposer sortie) | DIR | jamais « appliquer » | idem |
| ITP-43 | Mémoriser et modifier un **mappage de colonnes** de tableur | SEC | par source déclarée (« export Pronote ») | — |
| ITP-44 | Déclarer le **profil** de l'établissement pour les échanges (public, privé sous contrat, hors contrat, agricole ; degrés) — préremplit qui fait foi (socles 2, 4) | ADM ★ | changement en cours d'année = ★ et revue | — |
| ITP-45 | Planifier un import automatique et sa fenêtre horaire | ADM | hors heures de cours par défaut | — |
| ITP-46 | Voir l'état des **démarches externes** (convention AAF signée ? contrat ÉduConnect reçu ?) | DIR, ADM | tenu par ScolIA, lecture seule | — |

### 13.6 Exporter

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-47 | Lancer le **pré-contrôle** d'un export (sans générer) | SEC, DIR · `echanges.exporter` | lien vers l'écran de correction de chaque anomalie | — |
| ITP-48 | Voir l'**état de préparation** des exports à venir (échéances du §4) | SEC, DIR · `echanges.lire` | hebdomadaire 8 semaines avant | — |
| ITP-49 | Générer un export (instantané + fichier) | SEC · `echanges.exporter` | données confirmées seulement | `echanges.export_genere` |
| ITP-50 | Confirmer et déposer une remontée nationale | DIR, SEC · `echanges.exporter` ★ | aperçu : nombre de personnes, champs, destinataire | `echanges.export_depose` |
| ITP-51 | Télécharger un export généré (lien pré-signé court) | idem | journal | — |
| ITP-52 | Régénérer un export passé à l'identique (preuve) | DIR | même empreinte attendue | — |
| ITP-53 | Comparer deux exports | SEC | — | — |
| ITP-54 | Déposer le **compte rendu** d'une application nationale (retour) | SEC · `echanges.importer` | relié à l'export par son identifiant ou par choix | `echanges.retour_recu` |
| ITP-55 | Traiter les rejets d'un retour (tâches rattachées) | SEC | une tâche par enregistrement rejeté | — |
| ITP-56 | Voir les **traces d'export** d'un élève ou d'une personne (à qui, quand, quels champs) | DIR, SEC ; DPO | sans valeurs | — |
| ITP-57 | Produire le **paquet de réversibilité** de l'établissement | DIR ★ · `echanges.reversibilite` | chiffré, lien court, journal | `echanges.export_genere` |
| ITP-58 | Produire l'**export complet d'un dossier** (droit d'accès, portabilité, départ) | DIR, SEC (socle 27 décide) | périmètre selon le demandeur (le parent ne reçoit pas ce qui ne lui est pas communicable, socle 4) | idem |
| ITP-59 | Produire un **dossier de transfert** pour l'établissement d'accueil | SEC | demande vérifiée (INS-75) | idem |
| ITP-60 | Exporter les **effectifs** datés (diocèse, UDOGEC, collectivité, enquêtes) | DIR, SEC | agrégés ; nominatif seulement si la déclaration le prévoit | idem |
| ITP-61 | Exporter le **registre des élèves** (hors contrat, internat) | DIR | [NV] contenu réglementaire | idem |
| ITP-62 | Exporter vers un **logiciel d'EDT** (structure, cours, services) | EDT · `echanges.exporter` | format du logiciel choisi | idem |

### 13.7 Référentiels globaux (plateforme)

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-63 | Charger une nouvelle version d'une nomenclature (BCN, COG, UAI, calendrier, jours fériés) | OPS (tâche de plateforme) | licence vérifiée ; versionnée ; jamais d'écrasement d'une valeur d'établissement | `referentiels.version_publiee` |
| ITP-64 | Voir les effets d'une nouvelle version sur un établissement (codes retirés utilisés) | SEC, DIR | contrôle, pas de modification | — |
| ITP-65 | **Adopter** une version du calendrier officiel (zone) | DIR (socle 5) | propositions des jours modifiés | événement du socle 5 |
| ITP-66 | Chercher un établissement par UAI, nom, commune (origine, destination) | PER | référentiel global, aucune donnée personnelle | — |
| ITP-67 | Normaliser une adresse (suggestion BAN locale) | SEC, FAM (à la saisie) | suggestion, jamais imposée ; aucune requête externe | — |
| ITP-68 | Choisir une commune ou un pays de naissance dans le COG (avec communes historiques à la date) | SEC, FAM | — | — |

### 13.8 Identité fédérée

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-69 | Activer un fournisseur national (ÉduConnect, guichet Agents de l'académie) pour l'établissement | ADM · `federation.gerer` ★ | contrat de raccordement en place | `federation.fournisseur_modifie` |
| ITP-70 | Configurer un SSO d'établissement (OIDC) | ADM ★ | secret au coffre | idem |
| ITP-71 | Se connecter par un fournisseur fédéré | FAM, ELV, PER | guichet central, ticket | (journal de sécurité) |
| ITP-72 | Choisir son établissement après authentification (si plusieurs) | FAM, PER | liste visible de la seule personne | — |
| ITP-73 | Saisir un **code de rattachement** | FAM, ELV, PER | R-X12 | `federation.rattachement_cree` |
| ITP-74 | Générer des codes de rattachement (un, ou en lot pour une classe, avec feuilles à imprimer par parent) | SEC · `federation.rattacher` | un code par personne ; empreinte seule | — |
| ITP-75 | Désigner les **responsables d'affectation** GAR | DIR · `gar.gerer` | — | `gar.responsables_modifies` |
| ITP-76 | Voir l'état des rattachements (qui peut se connecter, qui n'a jamais activé) | SEC · `federation.rattacher` | — | — |
| ITP-77 | Suspendre ou révoquer un rattachement | DIR · `federation.gerer` ★ | motif | `federation.rattachement_revoque` |
| ITP-78 | Lier un moyen de connexion supplémentaire à son compte (ÉduConnect + passkey) | FAM, PER (soi) | R-X14 | `federation.rattachement_cree` |
| ITP-79 | Voir ses moyens de connexion et ses établissements | FAM, ELV, PER (soi) | — | — |
| ITP-80 | Retirer un moyen de connexion | soi ★ | il en reste au moins un | idem |
| ITP-81 | Voir le journal des connexions refusées (inconnu, suspendu, code faux) | ADM, DIR | sans donnée du fournisseur au-delà du nécessaire | — |
| ITP-82 | Débloquer une personne qui a épuisé ses essais de code | SEC | nouveau code | — |
| ITP-83 | Consulter la page « je n'arrive pas à me connecter » (diagnostic guidé : établissement, fournisseur, code) | public | aucune information sur l'existence d'un compte | — |

### 13.9 ScolIA fournisseur d'identité, GAR, annuaires

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-84 | Activer le GAR pour l'établissement (paramètres `idEnt`, UAI) | ADM · `gar.gerer` ★ | raccordement en place | `gar.active` |
| ITP-85 | Voir l'état de l'alimentation GAR (dernier dépôt, rejets) | DOC, DIR · `gar.gerer` | — | — |
| ITP-86 | Ouvrir le **médiacentre** et une ressource | ELV, PER | SSO GAR | — |
| ITP-87 | Diagnostiquer « je ne vois pas cette ressource » | DOC | affectation, groupe, alimentation | — |
| ITP-88 | Activer un service relié (Google, Microsoft, Apple, partenaire) avec la liste exacte des attributs transmis | ADM · `annuaires.gerer` ★ | déclaration DPO, information des familles | `annuaires.service_active` |
| ITP-89 | Prévisualiser un provisionnement (comptes à créer, modifier, suspendre) | ADM | — | — |
| ITP-90 | Appliquer un provisionnement | ADM ★ | jamais de mot de passe généré envoyé par courriel | `annuaires.provisionnement_applique` |
| ITP-91 | Suspendre automatiquement les comptes tiers au départ (avec délai paramétré) | SYS | consomme `eleve.scolarisation_close`, `personnel.engagement_clos` | idem |
| ITP-92 | Exclure une personne d'un service tiers (opposition, situation particulière) | ADM | motif | — |
| ITP-93 | Faire tourner les clés de signature de l'IdP | OPS / ADM plateforme | publication anticipée | `federation.cle_rotation` |
| ITP-94 | Voir les identifiants de service d'une personne (lecture restreinte, support) | ADM ★ | journal | — |
| ITP-95 | Créer un client partenaire (API) avec périmètre et champs | ADM ★ · `annuaires.gerer` | consentement du chef d'établissement | `annuaires.partenaire_cree` |
| ITP-96 | Révoquer un client partenaire | ADM ★ | immédiat | idem |

### 13.10 Calendriers

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-97 | Créer son abonnement de calendrier (emploi du temps, travail, événements, réunions — au choix) | PER, FAM, ELV · `calendriers.abonner` (portée moi-même) | jeton aléatoire, empreinte en base | — |
| ITP-98 | Abonner un agenda familial à l'emploi du temps **d'un enfant** | FAM | droit du lien (socle 4) relu à chaque lecture | — |
| ITP-99 | Faire tourner ou révoquer son jeton | soi | immédiat | — |
| ITP-100 | Voir ses abonnements et leur dernière lecture | soi | — | — |
| ITP-101 | Régler ce que montrent les abonnements de l'établissement (salles, noms des professeurs) | ADM | minimisation par défaut | — |
| ITP-102 | Révoquer tous les abonnements d'une personne (incident, départ) | ADM | — | — |
| ITP-103 | Importer un calendrier ICS externe (diocèse, mairie) en événements proposés | SEC | propositions au socle 19 | — |
| ITP-104 | S'abonner à un flux ICS externe récurrent (mise à jour hebdomadaire) | SEC | nouveaux événements en propositions | — |

### 13.11 Banque et comptabilité (contenu au socle 25 et 48)

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-105 | Importer un relevé (camt.053/054) | CPT · `echanges.importer` | rapprochement → propositions de lettrage (socle 25) | `echanges.lot_applique` |
| ITP-106 | Importer les rejets de prélèvement | CPT | propositions au socle 25 | idem |
| ITP-107 | Générer une remise de prélèvements (pain.008) | CPT ★ · `echanges.exporter` | contenu du socle 25 | `echanges.export_genere` |
| ITP-108 | Exporter le FEC ou des écritures pour l'expert-comptable | CPT · `echanges.exporter` | contenu du socle 48 | idem |
| ITP-109 | Exporter l'état des droits constatés (EPLE public, pour Op@le) | CPT | contenu des socles 40, 41 | idem |

### 13.12 Surveillance, support, conformité

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-110 | Voir le **tableau de bord des échanges** (flux, dernier passage, retards, rejets, échéances) | DIR, SEC, ADM · `echanges.lire` | — | — |
| ITP-111 | Recevoir une alerte de flux en retard ou en échec | DIR, ADM | socle 8 | `echanges.flux_en_retard` |
| ITP-112 | Voir le journal d'un lot ou d'un export (étapes, durées, acteurs) | DIR, ADM | — | — |
| ITP-113 | Exporter un lot anonymisé pour le support (structure sans valeurs personnelles) | ADM | anonymisation dans l'établissement ; aucune consultation par l'opérateur (D-013) | — |
| ITP-114 | Voir la **matrice de conformité** au cadre de référence (exigences, état, preuves) | DIR | lecture ; tenue par ScolIA | — |
| ITP-115 | Voir la surveillance plateforme des flux (sans contenu) | OPS | aucune donnée personnelle | — |
| ITP-116 | Traiter la quarantaine plateforme (UAI inconnue : identifier l'établissement par l'UAI seule, relancer le routage ou purger) | OPS | jamais d'ouverture du contenu | — |
| ITP-117 | Exécuter l'**anonymiseur local** sur des exports réels (pilote → jeu d'or) | ADM (dans l'établissement) | outil livré ; sortie vérifiée par un humain de l'établissement avant envoi | — |
| ITP-118 | Demander à l'assistant d'expliquer un rejet ou de proposer un mappage de colonnes | SEC | IA sans écriture (§20) | — |

### 13.13 Actions ajoutées par la passe personas **[ajout personas]**

| N° | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ITP-119 | Tenir le **journal à reporter dans le logiciel agréé** (cohabitation) : voir, pointer, exporter la liste de saisie | SEC · `echanges.lire` | R-X24 ; rappel jusqu'au pointage, sorties en tête | — |
| ITP-120 | **Concordance ScolIA ↔ BEE à une date** avant le constat (à partir d'un export BEE déposé) | SEC, DIR · `echanges.lire` | lecture ; écarts listés, rien n'est appliqué | — |
| ITP-121 | Traiter « **élève encore inscrit ailleurs** » | SEC | CX-47 | — |
| ITP-122 | **Assistant de reprise** : catalogue repris / archivé / abandonné, validation | DIR ★ · `echanges.reprise` | R-X23 ; rapport signé | `echanges.reprise_validee` |
| ITP-123 | Voir un **rejet national traduit** (catalogue de codes tenu par ScolIA : cause, écran de correction, nature données / format / plateforme) | SEC · `echanges.lire` | catalogue logiciel, enrichi par l'opérateur sans donnée personnelle | — |
| ITP-124 | **Déclarer le dépôt** manuel d'un export (date, compte rendu joint) | SEC · `echanges.exporter` | R-X21 ; export périmé signalé | `echanges.export_depose` |
| ITP-125 | Produire des **effectifs à une date passée**, figés | DIR, SEC · `echanges.exporter` | identité et scolarité versionnées (socle 2) | `echanges.export_genere` |
| ITP-126 | Produire l'**attestation de remontée** | DIR | R-X22 | — |
| ITP-127 | **Contrôle de sincérité** avant constat (inscrits sans présence relevée, départs annoncés non saisis, doubles UAI) | DIR · `echanges.lire` | lecture du socle 12 | — |
| ITP-128 | Enregistrer une **tentative de dépôt échouée** (horodatage, capture), message à la DSDEN | SEC | CX-51 | — |
| ITP-129 | Imprimer la **notice ÉduConnect** (et le code s'il y a lieu) par famille non activée ; suivre les relances | SEC, CPE · `federation.rattacher` | un document par parent (D-014) | — |
| ITP-130 | Suivre une **rectification** jusqu'à la source (état visible par la famille) | SEC ; FAM (lecture) | écart persistant relié à la demande | — |
| ITP-131 | Voir « **à qui mes données ont été transmises** » (titulaire pour son enfant, élève majeur pour lui-même), sans valeurs | FAM, ELV majeur · `echanges.mes_traces` | droits du lien (socle 4) relus | — |
| ITP-132 | Demander son **paquet de portabilité** ou **s'opposer** à un service tiers (titulaire, élève majeur) | FAM, ELV | crée une tâche (socle 27, ITP-58, ITP-92) | — |
| ITP-133 | **Vérifier avant remontée** ce qui partira vers Parcoursup ou le LSL (ses données seulement) | ELV (terminale) | lecture de l'instantané en préparation | — |
| ITP-134 | **Essayer un fournisseur SSO** sur une session d'essai avant activation | ADM · `federation.gerer` | aucune copie de données (Q-X32) ; un administrateur garde une passkey locale | — |
| ITP-135 | **Révocation générale** d'une personne | DIR, ADM ★ · `federation.gerer` | R-X25 | `federation.revocation_generale` |
| ITP-136 | **Coupe-circuit** d'un fournisseur fédéré ou d'un service tiers | ADM ★ ; OPS pour un fournisseur national | message clair aux personnes ; passkey proposée | `federation.fournisseur_modifie` |
| ITP-137 | Exporter la **configuration des connecteurs** (sans secrets) et le journal des flux pour un audit | ADM | — | — |
| ITP-138 | Préparer une **notification d'incident** (périmètre tiré des traces et du journal) | DIR, DPO | délais **NV** | — |
| ITP-139 | Générer les **fiches du registre des traitements** depuis les flux actifs | DPO, DIR | lecture | — |
| ITP-140 | **Rattacher à la main un rejet non structuré** à un enregistrement exporté | SEC | CX-65 | — |
| ITP-141 | **Reprendre un dépôt interrompu** (webservice) | SEC / SYS | idempotent, CX-58 | `echanges.export_depose` |
| ITP-142 | Reprendre la **correspondance des groupes de N-1** pour N | EDT | propositions | `echanges.correspondance_confirmee` |
| ITP-143 | **Rapprocher un annuaire existant** avant provisionnement | ADM · `annuaires.gerer` | FX-98 ; aucun compte créé avant | — |
| ITP-144 | Exporter l'**annuaire local** | ADM · `annuaires.gerer` | FX-97 | `echanges.export_genere` |
| ITP-145 | Voir et traiter les **expirations** (certificats, clés, conventions) | ADM | R-X26 | — |
| ITP-146 | Demander la **purge** à un partenaire révoqué ; suivre l'attestation | ADM | R-X28 | — |
| ITP-147 | Voir le **journal des lectures** d'un partenaire | ADM, DPO | — | — |
| ITP-148 | Importer les **photos** du photographe scolaire | SEC · `echanges.importer` | FX-108 ; propositions au socle 2 | — |
| ITP-149 | Importer les **passages de restauration** ; exporter les ayants droit | gestion · `echanges.importer` | FX-88 | — |
| ITP-150 | Exporter les **licenciables** (UGSEL, UNSS) et la liste pour l'**assurance scolaire** | SEC · `echanges.exporter` | champs minimaux, trace d'export | `echanges.export_genere` |
| ITP-151 | Marquer un écart « **corrigé à la source, en attente du prochain import** » | SEC | se clôt seul à la convergence | — |
| ITP-152 | Tenir le **calendrier national** à couche académique (opérateur) ; signaler un écart de date (établissement) | OPS ; SEC | référentiel global sans donnée personnelle | `referentiels.version_publiee` |
| ITP-153 | Reprendre les **mandats SEPA**, les échéanciers et les **soldes d'ouverture** | CPT ★ · `echanges.reprise` | FX-76, FX-106 ; propositions au socle 25 | — |
| ITP-154 | Exporter les **états de service et heures** des maîtres pour le rectorat [NV] | DIR · `echanges.exporter` | FX-41 | `echanges.export_genere` |

**Couverture Charlemagne (D-024).** ELE-01, ELE-22, INS-16, EDT-04, NOT-09/10/16, ORI-02, PER-18, CPT-07, PIL-09, PIL-20, T-26, T-28 sont couverts par ITP-01–30, 47–62, 69–96 ; ce que Charlemagne fait par son agrément (remontée BEE, LSU, LSL) reste **bloqué par les démarches**, pas par la conception. Réglages candidats à l'allègement (D-023), **signalés, non supprimés** : choix manuel de l'encodage (la détection suffit), seuils de volume au-delà des bornes, plus de deux politiques de disparition, configuration fine des attributs transmis au GAR (imposés par le contrat).

**[ajout personas] Réglages signalés S-68 à S-79** (`PERSONAS_09.md` §9, rien supprimé) : règles d'application au niveau du champ (S-68 : par famille de champs par défaut) ; `version_preferee` (S-69) ; seuils de volume réglables (S-70 : fixes et conscients de la bascule) ; politique de disparition (S-71) ; fenêtre des imports automatiques (S-72) ; contenu des abonnements iCal (S-73) ; échéances saisies par l'établissement (S-74 : couche académique tenue par l'opérateur) ; encodage et choix du connecteur (S-75) ; attributs GAR (S-76) ; profil modifiable par l'administrateur (S-77) ; rattachement par e-mail professionnel (S-78) ; délai de relance des dépôts (S-79). **Couverture Charlemagne après la passe** : voir `PERSONAS_09.md` §4 ; l'équivalence de la **première année** dépend de Q-X34.

---

## 14. Ce qui relève du logiciel, ce qui relève du paramétrage (D-023)

| Logiciel (fixe, testé) | Paramétrage de l'établissement |
|---|---|
| Catalogue des connecteurs, formats et versions acceptés, lecteurs et générateurs | Connecteurs activés, canaux, planification |
| Garde-fous d'identité (UAI, année, ancienneté) : **non désactivables** | Seuils de volume dans des bornes (5–30 %) |
| Table de vérité à trois voies ; « jamais d'application automatique d'un conflit, d'une disparition, d'un rapprochement par identité » | Suspension de l'application automatique par classe et par champ |
| Ce qui fait foi : **au domaine** (socles 2, 4), préréglé par le profil d'établissement | Profil d'établissement (public, privé sous contrat, hors contrat, agricole) |
| Pré-contrôles de chaque format ; données confirmées seulement | Échéances locales (une académie qui avance une date) |
| Règles de rattachement fédéré (R-X11–R-X14) ; format des codes | Fournisseurs activés ; autorisation du rattachement par e-mail professionnel |
| Minimisation des flux sortants (attributs déclarés par le connecteur) | Services tiers activés ; exclusions individuelles |
| Contenu minimal des abonnements iCal | Affichage des salles et des noms des professeurs |
| Conservation des fichiers bruts et normalisés (§17) | — (décision DPO, pas un réglage) |

---

## 15. Modèle de données cible

Toutes les tables d'établissement portent `etablissement_id UUID NOT NULL`, sont sous RLS (`SELECT ent_rls_activer('…')`) et suivent les règles de migration d'`ARCHITECTURE_BRIQUES.md` §5 (rejouables, contraintes nommées `DROP … IF EXISTS` + `ADD`, aucune modification d'une table d'une autre brique). Les **valeurs** personnelles ne vivent que dans les enregistrements de lot (chiffrées si sensibles, purgées à la fin de la fenêtre) et dans les instantanés (objets chiffrés) ; le reste du modèle ne porte que des identifiants, des empreintes et des métadonnées.

### 15.1 Moteur d'échanges (brique `echanges`, 3900–3908)

```
-- 3900_echanges_parametres.sql
ent_ech_profil              etablissement_id PK, profil ('public','prive_contrat','hors_contrat','agricole_public','agricole_prive'), degres TEXT[], modifie_le, modifie_par
ent_ech_connecteur_etab     id, etablissement_id, connecteur TEXT (code du catalogue), actif BOOL, version_preferee TEXT NULL,
                            planification JSONB NULL, active_le, active_par ; UNIQUE (etablissement_id, connecteur)
ent_ech_canal               id, etablissement_id, connecteur, nature ('sftp','webservice','reception_centrale'), hote, identifiant,
                            secret_ref TEXT NULL  -- référence au coffre, JAMAIS la valeur
                            , dernier_test_le, dernier_test_ok
ent_ech_regle               id, etablissement_id, connecteur, cible TEXT (type d'entité), champ TEXT NULL, classe TEXT,
                            mode ('auto_si_source_fait_foi','revue_humaine'), seuil_volume_pct SMALLINT NULL CHECK (5..30),
                            politique_disparition ('signaler','proposer_sortie') NULL, motif, modifie_le, modifie_par

-- 3901_echanges_reception_lot.sql
ent_ech_reception           id, etablissement_id, document_id UUID (socle 7, objet chiffré), empreinte_sha256, taille, type_reel,
                            nom_fichier (borné), canal ('depot','sftp','webservice','reception_centrale'), recu_le, recu_par NULL,
                            analyse ('en_cours','saine','quarantaine'), motif_quarantaine NULL
                            ; INDEX (etablissement_id, empreinte_sha256)
ent_ech_lot                 id, etablissement_id, reception_id, connecteur, version_format, sens ('entrant','retour'),
                            uai_fichier, annee_id NULL, date_extraction NULL,
                            statut ('recu','identifie','refuse','lu','analyse','previsualise','bloque_volume','en_application',
                                    'applique','applique_partiellement','abandonne','annule','annule_partiellement','clos','expire'),
                            synthese JSONB  -- compteurs par classe, jamais de valeur
                            , garde_fou_leve_par NULL, garde_fou_motif NULL, applique_par NULL, applique_le NULL,
                            annule_par NULL, annule_le NULL, parent_export_id NULL (pour un retour), cree_le, expire_le
ent_ech_lot_etape           id, etablissement_id, lot_id, etape, debut, fin, resultat, details JSONB (sans valeur personnelle)

-- 3902_echanges_enregistrements.sql
ent_ech_enregistrement      id, etablissement_id, lot_id, ordre INT, type_entite, cle_source TEXT,
                            localisation TEXT  -- « ligne 214 » ou chemin XML, sans valeur
                            , donnees BYTEA  -- fiche canonique chiffrée (champs_chiffres), purgée à la clôture
                            , statut ('a_rapprocher','rapproche','nouveau','rejete','ignore','applique','refuse','annule'),
                            entite_id UUID NULL, paquet INT NULL ; UNIQUE (lot_id, ordre)
ent_ech_rejet               id, etablissement_id, lot_id, enregistrement_id NULL, localisation, code TEXT (catalogue), libelle, gravite,
                            traite ('non','corrige','ignore'), traite_par, traite_le, motif

-- 3903_echanges_correspondance.sql
ent_ech_correspondance      id, etablissement_id, famille TEXT (siecle, sts, aaf, onde, edt_asc…), type_entite, cle_source TEXT,
                            entite_id UUID NULL, nature ('meme','distinct'), confirme_par, confirme_le, origine_lot_id
                            ; UNIQUE (etablissement_id, famille, type_entite, cle_source) WHERE nature = 'meme'
ent_ech_valeur_source       etablissement_id, famille, type_entite, entite_id, champ, empreinte BYTEA (HMAC, clé par établissement),
                            lot_id, recu_le ; PRIMARY KEY (etablissement_id, famille, type_entite, entite_id, champ)

-- 3904_echanges_differences.sql
ent_ech_difference          id, etablissement_id, lot_id, enregistrement_id, champ, classe ('nouveau','deja_aligne','ecart_initial',
                            'inchange','source_modifiee','correction_locale_maintenue','convergence','conflit','disparu'),
                            proposition_ref JSONB NULL  -- {domaine, id} : la proposition vit dans le domaine
                            , application ('aucune','auto','humaine'), regle TEXT NULL, decision TEXT NULL
ent_ech_ecart               id, etablissement_id, famille, type_entite, entite_id, champ, depuis_lot_id, etat ('ouvert','assume','clos'),
                            motif NULL, clos_le NULL

-- 3905_echanges_exports.sql
ent_ech_export              id, etablissement_id, connecteur, version_format, declaration TEXT (socle propriétaire + nature),
                            perimetre JSONB (sans valeur), uai, annee_id,
                            statut ('brouillon','controle','pret','genere','confirme','depose','accuse','rejete_partiellement','rejete','abandonne'),
                            instantane_id NULL, document_ids UUID[], empreinte_fichiers BYTEA NULL, destinataire TEXT,
                            precedent_id NULL, genere_par, genere_le, confirme_par, confirme_le, depose_le
ent_ech_export_anomalie     id, etablissement_id, export_id, gravite ('bloquante','avertissement'), code, libelle,
                            entite_ref JSONB, lien_correction TEXT  -- route de l'écran qui corrige
ent_ech_instantane          id, etablissement_id, document_id (objet JSON chiffré), empreinte, versions_citees JSONB, cree_le

-- 3906_echanges_traces.sql
ent_ech_export_inclusion    etablissement_id, export_id, type_entite, entite_id, champs TEXT[] ; PRIMARY KEY (export_id, type_entite, entite_id)
                            ; INDEX (etablissement_id, type_entite, entite_id)   -- « à qui a-t-on transmis X ? »
ent_ech_retour_rejet        id, etablissement_id, retour_lot_id, export_id, type_entite, entite_id NULL, code, libelle, tache_ref NULL

-- 3907_echanges_surveillance.sql
ent_ech_flux_attendu        etablissement_id, connecteur, periodicite INTERVAL, tolerance INTERVAL, dernier_passage, PRIMARY KEY (etablissement_id, connecteur)
ent_ech_echeance            id, etablissement_id, connecteur, libelle, date_limite, source ('logiciel','etablissement'), annee_id

-- 3908_echanges_reception_centrale.sql  (PLATEFORME : sans etablissement_id, hors RLS d'établissement, rôle dédié)
ent_ech_routage             id, recu_le, connecteur, empreinte, uai_detectees TEXT[], statut ('route','quarantaine','purge'),
                            route_vers JSONB  -- {uai: reception_id}, purge_le
```

### 15.2 Extension du noyau sécurité (3910, ou plage du noyau si Q-X2)

```
ent_fed_fournisseur         id, etablissement_id NULL (NULL = fournisseur national partagé), nature ('educonnect','agents','oidc_etab','saml_etab'),
                            protocole ('oidc','saml2'), parametres JSONB (issuer, métadonnées, points d'accès), secret_ref NULL,
                            academie TEXT NULL (guichet Agents), actif
ent_fed_activation          etablissement_id, fournisseur_id, actif, active_le, active_par ; PRIMARY KEY (etablissement_id, fournisseur_id)
ent_fed_rattachement        id, etablissement_id, fournisseur_id, sujet_empreinte BYTEA (HMAC du sujet), user_id, personne_ref JSONB
                            ({domaine: eleve|personne|agent, id}), preuve ('jointure','code','annuaire','email_pro'), preuve_ref,
                            etat ('propose','confirme','suspendu','revoque'), cree_le, confirme_le, revoque_le, motif
                            ; UNIQUE (etablissement_id, fournisseur_id, sujet_empreinte) WHERE etat IN ('propose','confirme','suspendu')
ent_fed_code                id, etablissement_id, personne_ref JSONB, empreinte BYTEA, expire_le, consomme_le NULL, essais SMALLINT, cree_par
ent_fed_ticket              empreinte BYTEA PK, hote TEXT, etat_transaction BYTEA, user_id, etablissement_id, expire_le (60 s), consomme_le NULL
ent_fed_tentative           id, etablissement_id NULL, fournisseur_id, resultat ('inconnu','suspendu','code_faux','assertion_invalide','ok'),
                            le, adresse_tronquee   -- journal de sécurité, rétention courte
```
Le `sujet` n'est stocké qu'en empreinte : une fuite de la table ne donne pas les identifiants ÉduConnect.

### 15.3 Référentiels globaux (brique `referentiels`, 3909)

Tables **sans `etablissement_id`**, en lecture seule pour les rôles d'établissement, écrites par une tâche de plateforme uniquement (exception RLS déclarée et testée, Q-X24) :
```
ref_version            id, referentiel ('bcn','cog','uai','calendrier','jours_feries','ban'), version, source_url, licence, empreinte, charge_le
ref_nomenclature       type, code, libelle_court, libelle_long, valide_du, valide_au, attributs JSONB, version_id ; PK (type, code, valide_du)
ref_cog_commune        code, libelle, departement, valide_du, valide_au, successeur_code NULL, version_id
ref_cog_pays           code, libelle, valide_du, valide_au, version_id
ref_uai                uai PK, nom, nature, secteur ('public','prive'), contrat NULL, commune_cog, ouvert BOOL, version_id
ref_calendrier_officiel zone, annee, debut, fin, libelle, population NULL, version_id
ref_jour_ferie         date, libelle, territoire ('metropole','alsace_moselle','971'…), version_id
ref_ban_*              tables d'index d'adresses (format du jeu BAN téléchargé, NV), version_id
```

### 15.4 Fournisseur d'identité, GAR, annuaires (brique `federation-idp`, 3911–3913)

```
-- 3911_idp.sql
ent_idp_service             id, etablissement_id, nature ('gar','google','microsoft','apple','partenaire'), protocole, client_id,
                            attributs_declares TEXT[], actif, active_par, active_le
ent_idp_identifiant_service id, etablissement_id, service_id, personne_ref JSONB, valeur TEXT (aléatoire 128 bits), cree_le, retire_le NULL
                            ; UNIQUE (service_id, valeur) -- jamais réutilisé, même après retrait
ent_idp_cle                 id, usage ('signature_oidc','signature_saml'), kid, coffre_ref, publiee_le, active_du, retiree_le   (plateforme)
-- 3912_gar.sql
ent_gar_parametres          etablissement_id PK, id_ent TEXT, uai TEXT[], actif
ent_gar_responsable_affectation etablissement_id, agent_id, du, au NULL
-- (les alimentations sont des ent_ech_export de connecteur 'gar.alimentation')
-- 3913_annuaires.sql
ent_ann_compte              id, etablissement_id, service_id, personne_ref JSONB, identifiant_externe TEXT, etat ('a_creer','actif','suspendu','supprime'),
                            derniere_synchro, erreur NULL
ent_ann_exclusion           etablissement_id, service_id, personne_ref JSONB, motif, cree_par, cree_le
ent_api_partenaire          id, etablissement_id, nom, client_id, secret_ref, perimetre JSONB, champs TEXT[], webhook_url NULL,
                            consentement_par, consentement_le, revoque_le NULL
```

### 15.5 Calendriers (brique `calendriers`, 3914)

```
ent_cal_abonnement          id, etablissement_id, user_id, cible JSONB ({type: soi | eleve, id}), contenus TEXT[] ('edt','travail','evenements','reunions'),
                            jeton_empreinte BYTEA UNIQUE, cree_le, derniere_lecture, etat ('actif','revoque','expire'), expire_le NULL
ent_cal_source_externe      id, etablissement_id, url, categorie, cible, derniere_lecture, actif
```
Règles [relecture, obj. 6] : l'abonnement est servi **sur le sous-domaine de l'établissement** (qui donne le tenant avant toute lecture) ; jeton de **256 bits**, empreinte seule en base ; le chemin du jeton est **filtré des journaux** et la réponse porte `Referrer-Policy: no-referrer` ; les **droits sont revérifiés à chaque lecture** (lien parental, engagement de l'agent, majorité de l'élève, sortie) ; expiration par défaut à la fin de l'année scolaire, renouvelable en un geste.

### 15.6 Registre des migrations

| N° | Fichier | Contenu |
|---|---|---|
| 3900 | `echanges_parametres` | profil, connecteurs activés, canaux, règles |
| 3901 | `echanges_reception_lot` | réceptions, lots, étapes |
| 3902 | `echanges_enregistrements` | enregistrements canoniques, rejets |
| 3903 | `echanges_correspondance` | correspondances, mémoire de la source |
| 3904 | `echanges_differences` | différences, écarts persistants |
| 3905 | `echanges_exports` | exports, anomalies, instantanés |
| 3906 | `echanges_traces` | inclusions, rejets de retour |
| 3907 | `echanges_surveillance` | flux attendus, échéances |
| 3908 | `echanges_reception_centrale` | routage plateforme, quarantaine |
| 3909 | `referentiels` | tables globales `ref_*` |
| 3910 | `federation` | fournisseurs, activations, rattachements, codes, tickets, tentatives (**noyau sécurité**, Q-X2 : **à trancher avant X1**, comme la propriété de 3908, tables de plateforme sans RLS d'établissement) |
| 3911 | `idp` | services, identifiants de service, clés |
| 3912 | `gar` | paramètres, responsables d'affectation |
| 3913 | `annuaires` | comptes tiers, exclusions, partenaires API |
| 3914 | `calendriers` | abonnements, sources externes |
| 3915–3989 | réserve | — |
| 3990–3999 | retrait | `ent_import_lot`, `ent_import_ligne` (144), `ent_edt_import` (060) après reprise et durée d'archive (§24) |

Repli si Q-B2 n'est pas adoptée : **420–439** (420–434 attribuées dans le même ordre, 439 retrait).

---

## 16. Sécurité

### 16.1 Menaces et parades

| Menace | Parade |
|---|---|
| Fichier d'un autre établissement chargé (fuite et pollution) | garde-fou UAI **avant lecture** ; quarantaine ; test d'isolation dédié |
| XML : entités externes (XXE), expansion d'entités (« billion laughs »), DTD | lecteur XML sans résolution de DTD ni d'entités externes (configuration explicite du parseur, test négatif par charge piégée) ; profondeur et taille d'attribut bornées |
| Archives : bombe de compression, chemins `../` ou absolus, liens symboliques, milliers d'entrées | analyse socle 7 + bornes du connecteur (taux de compression, entrées, taille décompressée) ; extraction en mémoire bornée, jamais sur disque à un chemin dérivé du nom |
| CSV/XLSX : injection de formule à l'export ; macros à l'import | échappement commun des cellules (CX-43) ; XLSX lu comme données, macros ignorées ; `.xlsm` refusé |
| Encodage : caractères de contrôle, homoglyphes dans les noms | normalisation Unicode (NFC), refus des caractères de contrôle ; alerte sur mélange d'écritures dans un nom [R] |
| Parseurs : panique, boucle, consommation mémoire | lecture en flux, limites, `cargo fuzz` par lecteur dans la CI (court) et en nocturne (long) |
| SAML : enveloppement de signature, assertion non signée, rejeu, mauvaise audience, échéances, algorithmes faibles, commentaires dans l'identifiant | brique isolée ; seule l'assertion signée par la clé épinglée du fournisseur est acceptée ; vérification de la référence signée **sur l'élément utilisé** ; `InResponseTo`, audience, `NotOnOrAfter`, rejeu (cache des identifiants d'assertion) ; liste blanche d'algorithmes ; tests négatifs issus des failles publiques connues ; revue sécurité externe avant production |
| OIDC : confusion d'émetteur, jeton d'un autre client, `nonce` absent | vérifications existantes d'`ent-oidc` [F] + `iss` par fournisseur + `aud` + `nonce` + PKCE ; guide ANSSI de sécurisation d'OpenID Connect (2020) [S] comme liste de contrôle |
| Ticket de transfert intercepté ou rejoué | 60 s, usage unique, lié à l'hôte, à l'état et au navigateur (cookie de transaction), empreinte seule en base |
| Code de rattachement deviné | 10 caractères (≈ 50 bits), 5 essais/heure, empreinte seule, expiration |
| Rattachement par concordance faible (prise de compte) | R-X11 : aucune preuve faible acceptée |
| Clés de l'IdP volées | coffre, rotation, journal, révocation, alerte |
| Secrets de canaux (SFTP, certificats NetSynchro) dans la base ou les journaux | références de coffre seulement ; filtres de journalisation ; jamais réaffichés |
| Abonnement iCal fuité | jeton révocable, contenu minimisé, expiration, journal de lecture |
| API partenaire trop large | périmètre et champs déclarés, consentement, jetons courts, journal, révocation immédiate |
| **Requêtes sortantes vers une adresse saisie par un établissement** (hôte SFTP, URL ICS, webhook partenaire, points d'accès OIDC) : rebond vers le réseau interne (SSRF) | refus des adresses privées, locales et de métadonnées ; résolution DNS épinglée et revérifiée ; pas de redirection suivie ; délais et tailles bornés ; sortie par un mandataire dédié [relecture, obj. 4] |
| Opérateur ScolIA qui lit des données d'élèves | aucun outil plateforme n'affiche de contenu ; quarantaine plateforme sans lecture ; support par lot anonymisé (ITP-113) |
| Réception centrale : erreur de routage entre établissements | routage par UAI seule, double vérification à la réception dans le tenant (garde-fou UAI rejoué), test d'isolation |
| Escalade par un import (rôle déduit d'un profil national) | un import ne donne aucun droit ; il **suggère** un modèle de rôle, confirmé par une personne ayant ce droit (pas d'escalade, D-017) |

### 16.2 Cloisonnement
Tout est par établissement (RLS) sauf : référentiels globaux (lecture seule, aucune donnée personnelle), réception centrale (processus plateforme isolé), **guichet central de fédération** (rôle dédié, tables de §15.2 seulement, §10.2), fournisseurs nationaux partagés (configuration sans donnée personnelle), clés de l'IdP. Les identifiants externes restent uniques **dans** l'établissement (281 [F]) ; les identifiants de service sont propres à chaque service.

---

## 17. RGPD et conservation

- **Rôles** [H, à valider par le DPO] : pour les exports déposés par l'établissement (SIECLE, STS), le chef d'établissement est responsable de traitement et ScolIA sous-traitant ; pour l'AAF, la convention académique fixe les rôles ; le GAR est un traitement du ministère (arrêté du 18 décembre 2017 [S]) ; l'ENT relève de l'acte réglementaire RU-003 de la CNIL (délibération 2006-104 [S, archive]) — durées **NV** ; le provisionnement vers Google ou Microsoft est une décision de l'établissement avec son propre contrat de sous-traitance.
- **Minimisation** : chaque connecteur déclare ses champs ; les champs non déclarés d'un fichier **ne sont pas lus** dans la fiche canonique (ex. un export SIECLE « avec adresses » utilisé pour un connecteur « sans adresses » ne charge pas les adresses) ; NUMEN jamais lu ni stocké (R-X16) ; PCS et nationalité seulement si l'établissement les a activées (socles 2, 4).
- **Conservation proposée** [R, Q-X12, décision DPO] : fichier brut **30 jours** (au-delà, seule l'empreinte reste) ; enregistrements normalisés et valeurs chiffrées : jusqu'à la **clôture** du lot (30 jours après application) ; synthèse, rejets sans valeur, différences sans valeur et journal : **durée de l'année scolaire + 1 an** ; exports générés et instantanés : **durée propre au flux** (preuve d'une remontée : 3 ans [H] ; paquet de réversibilité : 30 jours après remise) ; traces d'inclusion : celle de l'export ; tentatives de connexion : 6 mois [H] ; codes de rattachement : jusqu'à consommation + 30 jours ; **correspondances** et **mémoire de la source** : tant que l'entité est scolarisée ou engagée, puis purge à la sortie + 1 an ; **écarts** : valeur chiffrée effacée à la clôture, ligne à la clôture + 1 an ; **rattachements fédérés** : révoqués à la sortie, purgés à + 1 an ; **identifiants de service** : retirés à la sortie, la valeur reste réservée (jamais réutilisée) mais n'est plus reliée à la personne après + 1 an ; **inclusions** : durée de l'export. Le droit à l'effacement, quand il s'applique, purge ces tables pour la personne (liste tenue par le registre des tables de la brique) [relecture, obj. 7].
- **Droit d'accès** : traces d'export (§6.11) ; **rectification** : une correction locale est maintenue et signalée comme écart à corriger à la source (public) ; **opposition** au provisionnement tiers : exclusion individuelle (ITP-92) si le cadre de l'établissement le permet.
- **AIPD** : la fédération, le GAR et le provisionnement tiers entrent dans l'AIPD de l'ENT (obligation spécifique aux ENT **NV**).
- **Information des familles** : liste publique des flux actifs de l'établissement (« vos données sont transmises à : SIECLE, GAR… ») générée depuis les connecteurs activés, dans la notice (socle 27).

---

## 18. Droits (catalogue `ent_auth::Action`, D-017)

### 18.1 Existant [F]
`imports.appliquer` (lecture et application des lots d'import, socle 3, `03_INSCRIPTIONS.md` §2.2 point 14) ; gestion des fournisseurs OIDC sous les droits d'administration du socle 1.

### 18.2 Actions proposées [R]

| Action | Portées | Modèles par défaut | Couvre | Remplace |
|---|---|---|---|---|
| `echanges.lire` | établissement | Direction, Secrétariat, Administrateur, Comptabilité (ses connecteurs) | ITP-04, 11, 17, 27, 31, 36, 48, 110, 112 | partie de `imports.appliquer` |
| `echanges.importer` | établissement | Direction, Secrétariat, Comptabilité, Responsable EDT | ITP-01–09, 43, 54, 105, 106 | idem |
| `echanges.appliquer` ★ | établissement | Direction, Secrétariat | ITP-10, 13–16, 21, 24–26 ; **plus** le droit du domaine pour chaque proposition | idem |
| `echanges.annuler` ★ | établissement | Direction | ITP-28 | — |
| `echanges.exporter` [export] | établissement | Direction, Secrétariat, Comptabilité (ses flux) | ITP-32, 47, 49–53, 59–62, 107–109 ; **jamais un professeur** (D-022, marqueur `export`) | exports actuels |
| `echanges.reversibilite` ★ [export] | établissement | Direction | ITP-57, 58 | — |
| `echanges.connecteurs.gerer` ★ | établissement | Administrateur | ITP-37–39, 44, 45 | — |
| `echanges.regles.gerer` ★ | établissement | Direction | ITP-40–42 | — |
| `federation.gerer` ★ | établissement | Administrateur | ITP-69, 70, 77, 81 | droits OIDC du socle 1 |
| `federation.rattacher` | établissement | Secrétariat, Direction | ITP-74, 76, 82 | — |
| `gar.gerer` | établissement | Direction, Documentaliste | ITP-75, 84, 85, 87 | — |
| `annuaires.gerer` ★ | établissement | Administrateur | ITP-88–92, 94–96 | — |
| `calendriers.abonner` | moi-même | tous (personnels, familles, élèves) | ITP-97–100 ; pour un enfant : droit du lien (socle 4) | routes `.ics` actuelles |
| `calendriers.gerer` | établissement | Administrateur, Secrétariat | ITP-101–104 | — |
| `referentiels.gerer` | plateforme (hors établissement) | opérateur ScolIA | ITP-63, 115, 116 | — |

**18.3 [ajout personas] Actions complémentaires [R]**

| Action | Portées | Modèles par défaut | Couvre |
|---|---|---|---|
| `echanges.exporter_structure` | établissement | Responsable EDT (même professeur), Direction | ITP-62 limité à la structure, aux cours et aux services, **sans donnée nominative d'élève** ; ne porte pas le marqueur `export` (D-022 reste entière pour les exports officiels, Q-X38) |
| `echanges.reprise` ★ | établissement | Direction ; Comptabilité (ses flux) | ITP-122, ITP-153 |
| `echanges.mes_traces` | moi-même (et enfant par le lien du socle 4) | familles, élèves majeurs | ITP-131 |
| révocation générale | — | — | ITP-135 sous `federation.gerer` ★ (pas d'action nouvelle) |

La portée « moi-même » manque encore au code (`ARCHITECTURE_BRIQUES.md` Q-B16) : `calendriers.abonner` en dépend.

---

## 19. Événements

### 19.1 Émis [R]
Charge utile minimisée : identifiants, compteurs, codes ; **jamais** de valeur d'identité ni de nom de fichier.

| Type | Quand | Consommateurs |
|---|---|---|
| `echanges.reception_recue` | fichier reçu et analysé | journal |
| `echanges.lot_previsualise` | analyse terminée | à-faire (socle 29) : « lot à relire » |
| `echanges.lot_applique` | application terminée (ou partielle) | pilotage (47), à-faire |
| `echanges.lot_abandonne`, `echanges.lot_annule` | idem | journal, à-faire |
| `echanges.correspondance_confirmee` | rapprochement confirmé ou défait | journal |
| `echanges.connecteur_modifie` | activation, canal, règle | journal de sécurité |
| `echanges.export_genere`, `echanges.export_depose` | export | journal, pilotage |
| `echanges.retour_recu` | compte rendu national importé | à-faire (tâches de rejet) |
| `echanges.flux_en_retard` | flux automatique manquant ou en échec | socle 8 (alerte) |
| `referentiels.version_publiee` | nouvelle version d'un référentiel global | socle 5 (contrôles, adoption du calendrier) |
| `federation.fournisseur_modifie`, `federation.rattachement_cree`, `federation.rattachement_revoque`, `federation.cle_rotation` | identité | journal de sécurité ; socle 8 (« un nouveau moyen de connexion a été lié à votre compte » : avis de sécurité non désactivable, `08_NOTIFICATIONS.md`) |
| `gar.active`, `gar.responsables_modifies` | GAR | journal |
| `echanges.reprise_validee`, `federation.revocation_generale` | **[ajout personas]** catalogue de reprise validé ; révocation générale d'une personne | journal ; journal de sécurité, socle 8 (avis de sécurité à la personne) |
| `annuaires.service_active`, `annuaires.provisionnement_applique`, `annuaires.partenaire_cree` | services tiers | journal, notice publique des flux (socle 27) |

### 19.2 Consommés
`eleve.scolarisation_ouverte` / `_close`, `eleve.identite_modifiee` (GAR différentiel, annuaires, abonnements), `personne.lien_suspendu` (rattachements, abonnements), `personnel.engagement_clos` (rattachements Agents, annuaires), `structure.*` (GAR groupes), `eleve.majeur` (droits des abonnements des parents). Les événements produits **par un import** portent `origine = import` (exigence (b) du §5.4).

### 19.3 Natures d'avis déclarées au socle 8
`echanges.lot_a_relire` (U1, personnels) ; `echanges.conflits_en_attente` (U1, résumé quotidien) ; `echanges.flux_en_retard` (U2, administrateur, direction) ; `echanges.echeance_proche` (U1, 8 semaines puis 2 semaines avant) ; `echanges.retour_rejets` (U1) ; `federation.moyen_lie` (sécurité, **non désactivable**, à la personne) ; `federation.code_rattachement` (remise du code par lien nominatif ou courrier, socle 7). **Aucune nature à destination des familles n'est déclenchée par l'application d'un lot** (§5.2 règle 5).

---

## 20. Contrat IA (`IA_COEUR.md` §5)

| Capacité | Type | Droit | Sensibilité | Confirmation | Remarque |
|---|---|---|---|---|---|
| `echanges.expliquer_rejet` | lecture | `echanges.lire` | C1 (aucune valeur transmise au modèle : code, localisation, règle) | — | traduit un rejet technique en consigne |
| `echanges.proposer_mappage` | lecture | `echanges.importer` | C1 (en-têtes et **10 valeurs anonymisées** au plus) | — | propose un mappage de colonnes, que la personne valide |
| `echanges.resumer_lot` | lecture | `echanges.lire` | C1 (compteurs) | — | « 31 téléphones changés, 2 conflits d'état civil » |
| `echanges.etat_preparation` | lecture | `echanges.exporter` | C1 | — | « ce qui manque pour le LSL » |
| `echanges.preparer_decisions` | commande (brouillon) | `echanges.appliquer` + droits des domaines | C2 | **N3** (passkey) ; application par l'humain | l'IA peut **regrouper** des décisions ; elle n'applique rien |
| `federation.diagnostiquer_connexion` | lecture | `federation.rattacher` | C1 | — | « Mme X n'a pas de rattachement : générer un code ? » |

Interdits : appliquer un lot, lever un garde-fou, annuler, déposer une remontée, rattacher une identité, activer un connecteur ou un service tiers. Tests de garde : équivalence écran / IA, isolation, simulation sans écriture ; jeu d'évaluation : 30 rejets réels anonymisés avec explication attendue.

---

## 21. Écrans clés (Leptos, `docs/FRONTEND_LEPTOS.md`)

1. **Centre des échanges** (`/web/echanges`) — tuile par flux actif : dernier passage, état (vert, orange « 2 conflits », rouge « AAF absent depuis 2 nuits »), prochaine échéance ; colonne « à faire » (lots à relire, retours à traiter, écarts à corriger à la source) ; **calendrier de l'année** (§4) avec l'état de préparation de chaque échéance ; bouton unique « Déposer un fichier ».
2. **Dépôt** — zone de dépôt ; détection affichée en clair (« Export SIECLE — élèves avec adresses — UAI 0xxxxxxY — année 2026-2027 — extrait le 2 septembre ») ; garde-fous rouges en tête s'ils bloquent ; plusieurs fichiers ordonnés automatiquement.
3. **Revue d'un lot** — en-tête : synthèse par classe (cartes cliquables) ; liste dense filtrable (classe, entité, champ, division) ; chaque ligne : avant / après, source, preuve (ligne du fichier), règle qui s'appliquerait ; actions groupées (« accepter tous les téléphones modifiés par la source ») ; panneau latéral « cet élève » (toutes ses différences) ; raccourcis clavier (accepter, refuser, suivant) ; mention permanente « rien n'est appliqué tant que vous n'avez pas validé ».
4. **Premier import** — mode guidé : correspondances de divisions et de matières d'abord, puis élèves, puis responsables ; compteur « décisions restantes » ; explication de ce qui changera aux imports suivants.
5. **Écarts persistants** — liste « corrigé ici, pas dans SIECLE », export de travail pour la correction à la source, marquage « assumé ».
6. **Exports** — par flux : pré-contrôle (anomalies groupées par écran de correction, avec lien), génération, aperçu de ce qui part (nombre de personnes, champs, destinataire), confirmation ★, dépôt, retour, historique des exports et comparaison.
7. **Connecteurs** (administrateur) — catalogue : disponibles, expérimentaux, **bloqués avec la raison et l'état de la démarche** ; assistant d'activation ; canal (secret saisi une fois) ; règles ; planification.
8. **Identité** (administrateur) — fournisseurs ; état des rattachements par population (« 412 responsables sur 640 peuvent se connecter ; 38 sans téléphone ni e-mail ») ; génération et impression des codes ; journal des refus.
9. **Connexion** (public) — boutons des fournisseurs activés pour l'établissement ; page « je n'arrive pas à me connecter » (diagnostic sans révéler l'existence d'un compte) ; saisie de code.
10. **Mes moyens de connexion** (chacun) — passkeys, ÉduConnect, SSO ; établissements (pour la seule personne) ; lier, retirer.
11. **Médiacentre** (élève, professeur) — ressources affectées ; diagnostic pour la documentaliste.
12. **Services reliés** (administrateur) — Google, Microsoft, Apple, partenaires : attributs exacts transmis, aperçu du provisionnement, exclusions.
13. **Mes calendriers** (chacun) — créer un abonnement (choix du contenu), QR code et lien à copier, instructions par agenda (Apple, Google, Outlook), rotation, révocation.
14. **Fiche élève** (socle 2) — dans chaque champ, l'**origine** (« SIECLE, lot du 3 septembre ») et, s'il existe, l'**écart** avec la source.

Accessibilité : listes de revue utilisables au clavier, états annoncés (RGAA ; SDET : ≥ 75 % des critères applicables pour les modules éditeur [S]).

---

## 22. Applications mobiles (D-003)

Pas d'import ni d'export sur mobile (tâches complexes réservées au web). Sur mobile : **connexion fédérée** (ÉduConnect, Agents) par session d'authentification du système (`ASWebAuthenticationSession`, Custom Tabs) avec PKCE vers le guichet central, puis jeton Bearer de l'application (D-011) ; **choix de l'établissement** ; **saisie d'un code de rattachement** (ou scan du QR code imprimé sur la feuille de rentrée) ; **abonnement au calendrier** en un geste (ouverture du lien `webcal`) ; alertes de flux pour la direction (socle 8). Le médiacentre s'ouvre dans le navigateur du système.

---

## 23. Tests

| Famille | Contenu |
|---|---|
| **Jeux d'or** | par connecteur, fichiers anonymisés et sorties canoniques attendues ; tout changement de sortie casse le test |
| **Propriétés** | importer deux fois le même fichier = aucune nouvelle proposition ; annuler puis réimporter = même état ; ordre des paquets indifférent ; la table de vérité à trois voies (§6.6) testée exhaustivement (9 classes × champ vide/non vide × sensible/non sensible) |
| **Garde-fous** | UAI étrangère refusée **sans lecture** (vérifié : aucune ligne `ent_ech_enregistrement`) ; année ; ancienneté ; volume (export partiel d'une division) |
| **Frontières** | garde SQL : aucune écriture hors des tables du socle 9 ; une application passe par les commandes (vérifié par les versions créées au socle 2 avec `source_ref`) |
| **Isolation** | lot de A invisible de B ; réception centrale : routage par UAI, quarantaine invisible de tous ; correspondances et mémoire de source par établissement ; rattachement fédéré de A inutilisable sur l'hôte de B ; ticket lié à l'hôte |
| **Sécurité des lecteurs** | XXE, expansion d'entités, bombe zip, chemins, encodages piégés, cellules à formule ; `cargo fuzz` par lecteur |
| **Fédération** | assertion non signée, signature déplacée, rejeu, audience, échéance, `nonce` ; rattachement refusé par e-mail ou nom seul ; code : usage unique, essais, expiration ; lien suspendu → connexion refusée ; message identique pour inconnu et suspendu |
| **IdP** | identifiant de service stable, différent par service, jamais réutilisé ; aucun INE ni e-mail dans une assertion GAR ; rotation de clé sans coupure |
| **Exports** | déterminisme (deux générations, mêmes octets) ; données confirmées seulement ; pré-contrôle : chaque anomalie porte un lien valide ; professeur refusé (D-022) ; inclusions exactes |
| **Calendriers** | jeton révoqué → 404 indistinct ; enfant sorti → flux vide ; aucun nom d'autre élève dans le flux |
| **IA** | équivalence écran / IA ; aucune valeur personnelle dans le contexte de `expliquer_rejet` ; simulation sans écriture |
| **Base obligatoire** | les tests d'intégration **échouent** (et ne reviennent pas silencieusement) sans base en CI (§2.2 n° 13) |
| **Recette par personas** (D-021) | secrétaire qui charge le mauvais fichier ; parent séparé qui tente de rattacher l'ÉduConnect de l'autre parent ; élève qui saisit le code de son frère ; professeur qui tente un export LSU ; établissement B qui tente de lire un lot de A |

---

## 24. Plan de migration depuis l'existant

1. **X0 (immédiat, sans attendre le moteur)** : corriger `imports.rs` (reclassement dans la transaction ; écritures par les commandes du socle 2 et les ports du socle 4 quand ils existent, `dossier_personnes_ports.rs` [F, lot 7]) ; libellés « export de travail (non officiel) » sur les cinq CSV ; espace de noms `urn:scolia:…` et mention « format interne non officiel » sur le XML LSU de référence, retrait de tout bouton qui le présenterait comme un fichier LSU ; échappement des cellules dans tous les exports CSV. (Le mécanisme commun des secrets de fédération et de canaux se décide avec le socle 1, au lot X8.)
2. **X1–X3** : le moteur remplace le chemin de 144 ; le format `scolia_eleves_v1` devient un connecteur `tableur.eleves` (mappage fixe, documenté) ; les lots 144 existants restent lisibles (vue de compatibilité) et ne sont plus créés.
3. **X16** : l'import d'EDT (060) passe par le moteur ; `ent_edt_import` devient lecture seule.
4. **X15** : les routes `.ics` derrière session restent pour l'usage connecté ; les abonnements à jeton s'y ajoutent.
5. **3990–3999** : retrait de `ent_import_lot`, `ent_import_ligne`, `ent_edt_import` après la durée d'archive (même règle que le socle 8 pour ses anciennes tables).
6. Les exports CSV de `domain-eleve` migrent vers le `modele.rs` des briques propriétaires (22, 23, 37, 38, 50) au moment de leur socle (`ARCHITECTURE_BRIQUES.md` §6).

---

## 25. Découpage en lots de code (équipe Sonnet)

### 25.1 Lots

| Lot | Contenu | Dépend de | Migrations | Taille | Critères de fin |
|---|---|---|---|---|---|
| **X0 Correctifs immédiats** | §24 point 1 | — | — | S | reclassement dans la transaction testé ; aucune écriture directe nouvelle ; libellés ; XML de référence non trompeur ; échappement testé |
| **X1 Socle du moteur** | brique `echanges` : réception, analyse (socle 7), lots, étapes, garde-fous d'identité, trait `Connecteur`, catalogue (statuts), droits, journal | kit K (D-026), socle 7 (dépôt), E5 (socle 2) | 3900–3902 | M | dépôt → identification → refus UAI sans lecture ; lot factice rejouable ; isolation ; fuzzing branché |
| **X2a Rapprochement** | correspondances, ports `correspondance` des domaines, candidats par identité, dédoublonnage dans le lot | X1 ; ports des socles 2, 4 (puis 5, 6) | 3903 | M | CI : rapprochement par identifiant, par identité jamais automatique, foyer jamais par adresse |
| **X2b Mémoire et trois voies** | mémoire de la source (HMAC, normalisation versionnée), table de vérité complète (§6.6), écarts, ports `proposer` | X2a ; E5 (socle 2) et propositions du socle 4 | 3904 | M | CI : chaque ligne de la table testée en propriété ; refus non reproposé ; annulation restaure B |
| **X2c Application et annulation** | règles, garde-fou de volume, application par paquets avec revérification, reprise, annulation par compensation | X2b | — | M | CI : reprise après panne, saisie concurrente non écrasée, annulation avec liste des refus |
| **X3 Tableurs et formats ScolIA** (dépend aussi du port `proposer_affecte` du socle 3) | CSV/XLSX, détection de séparateur et d'encodage, mappage mémorisé, connecteurs `tableur.*` (élèves, responsables, personnels, salles, matières, groupes, options, affectés, créneaux), format `scolia.transfert.v1` en lecture | X2 | — | M | jeux d'or ; mappage mémorisé ; élèves affectés → dossiers socle 3 |
| **X4 Référentiels globaux** | `ref_*` : COG, calendrier officiel, jours fériés (licences [S]) ; BCN et annuaire des UAI **après vérification de licence** (D-12) ; exception RLS testée ; tâche de chargement versionné | kit | 3909 | M | versions chargées ; aucune écriture d'établissement ; contrôle « code retiré » |
| **X4 bis BAN locale** | index d'adresses téléchargé, suggestion de normalisation sans appel externe | X4 | 3909 | M | aucune requête sortante portant une adresse |
| **X5 SIECLE BEE (exports)** | lecteurs des cinq exports, ordre, rapprochement par `ELEVE_ID`/INE/identifiant de personne, liens de responsabilité, structures, options ; statut `Experimental` | X2c, X4, **jeux d'or réels anonymisés (D-14)** | — | L | **CI** : jeux d'or, second import idempotent ; **recette externe** : trois établissements pilotes importés sans perte, écarts arbitrés ; passage de `Experimental` à `Disponible` après la recette |
| **X6 STS-web (export emp)** | lecteur `sts_emp` : agents, fonctions, services, divisions, groupes ; propositions au socle 6 et 5 | X2, X4, jeux d'or | — | M | idem ; NUMEN ignoré (test) |
| **X7 Exports** | `ServiceExports` : périmètre, pré-contrôle, instantané, générateur, inclusions, dépôt manuel, retours, état de préparation et échéances ; générateurs ScolIA : effectifs, registre, transfert, réversibilité, dossier complet ; export de travail des écarts | X1 | 3905–3907 | L | déterminisme ; données confirmées ; liens de correction ; traces ; professeur refusé |
| **X8 Fédération (socle)** | noyau sécurité : fournisseurs, abstraction de protocole, guichet central, tickets, rattachements (jointure, code, annuaire), codes en lot, liaison de moyens, page de diagnostic ; OIDC d'établissement durci | socle 1, X2 (identifiants) | 3910 | L | parcours complet avec un IdP OIDC de test ; tests négatifs ; aucun rattachement faible ; isolation hôte |
| **X9 ÉduConnect** | raccordement selon le contrat (OIDC par `ent-oidc` ou SAML par X9-s) | X8 ; **contrat d'interface (D-4)** | — | M–L | connexion de bout en bout sur l'environnement de test ; refus d'un inconnu |
| **X9-s Brique SAML** (si exigée) | fournisseur de service SAML 2.0 à profil restreint ; dépendance choisie après analyse de licence (pas d'AGPL), de maintenance et de dépendances C ; revue externe | X8 ; Q-X5 | — | L | suite de tests négatifs publique passée ; revue de sécurité |
| **X10 Guichet Agents** | idem par académie | X8 ; **D-5** | — | M | idem |
| **X11 ScolIA fournisseur d'identité + SSO GAR** | IdP OIDC (puis SAML si exigé), clés au coffre, rotation, identifiants de service, profils GAR | X8 ; **contrat SSO GAR (D-6)** | 3911 | L | assertion acceptée par l'intégration GAR |
| **X12 Alimentation GAR + médiacentre** | générateur XSD, archive, MD5, dépôt, compte rendu, médiacentre, responsables d'affectation | X7, X11 ; **XSD et canal (D-6)** | 3912 | M–L | archive acceptée sans rejet en intégration |
| **X13 AAF** | lecteur AAF complet et différentiel, réception automatique (canal), réception centrale si l'académie l'exige, application automatique selon le profil « public » | X2, X5 ; **convention (D-7)** | 3908 | M–L | fichiers d'exemple officiels importés sans perte ; nuit manquée → alerte |
| **X14 Remontée BEE (privé)** | générateur et canal NetSynchro / OMOGEN Fichier, certificat, retours (INE `retour_bee`), écarts | X7 ; **agrément (D-3)** | — | L | fichier accepté en qualification |
| **X15 Calendriers** | abonnements à jeton, rendu minimisé, révocation, expiration ; imports ICS externes en propositions ; adoption du calendrier officiel (avec socle 5) | X4 ; socles 10, 19, 20 (lectures) | 3914 | M | client Apple/Google/Outlook abonné ; révocation ; enfant sorti |
| **X16 Logiciels d'EDT** | connecteurs aSc (XML), FET (XML, lecture du format seulement), EDT/Index et UnDeuxTEMPS sur fichiers réels ; export de structure ; reprise de 060 | X2, socle 10 | — | M | jeux d'or ; correspondance de groupes mémorisée |
| **X17 Annuaires sortants** | OneRoster 1.2 CSV, SDS CSV v2.1, Apple School Manager (SFTP) ; aperçu, provisionnement, suspension au départ, exclusions | X7, X11 (identifiants de service) | 3913 | M | import accepté par chaque plateforme de test ; aucun parent exporté par défaut |
| **X18 Banque** | lecteurs camt.053/054 et pain.002 ; générateur pain.008 (avec socle 25) | X2, X7, socle 25 | — | M | relevé de test rapproché en propositions ; remise validée par le validateur de la banque |
| **X19 Conformité** | matrice du cadre de référence (annexe 4 de l'arrêté de 2026, SDET), notice publique des flux, déclaration d'accessibilité, plan de réversibilité ; balisage DNMA **si** obligatoire (Q-X21) | texte intégral (D-1) | — | M puis continu | matrice remplie, preuves liées, revue |

**Lots annexes** cités au §9 (non détaillés, à concevoir au moment voulu) : **X4 bis** BAN locale ; **X9-s** brique SAML ; **X11 bis** SSO sortant vers Google/Microsoft (après X11, Q-X17) ; **X14 bis** ONDE (au socle 50) ; **X14 ter** Fregata (si Q-X25) ; **X16 bis** services vers STS-web (après spécification) ; **X17 bis** SCIM et export Gabriel (après réponse du SGEC) ; **X19 bis** API partenaires (après le pilote, Q-X22). Pour les lots bloqués (X9–X14, X17), chaque critère est séparé en **test de CI** (jeux d'exemple, tests négatifs) et **recette externe** (environnement de qualification du partenaire).

Lots **hors de ce socle** qui utilisent X7 : LSU (22), LSL et Cyclades (23), Parcoursup et Affelnet (37), DNB/Notanet et Cyclades (38), FEC et écritures (48), droits constatés (40, 41), ONDE (50). Ils ne démarrent leur partie « format officiel » qu'avec la validation éditeur (D-2).

### 25.2 Ordre et parcours minimal vendable

```
X0 ─▶ X1 ─▶ X2a ─▶ X2b ─▶ X2c ─┬─▶ X3 ─▶ X5 ─▶ X6   (le pilote charge ses propres exports, sans convention ; jeux d'or D-14 nécessaires)
                ├─▶ X4 (─▶ X4 bis)
                ├─▶ X7 ─▶ (22, 23, 37, 38, 48 : pré-contrôles et aides à la saisie)
                ├─▶ X8 ─▶ X9 / X10 / X11 ─▶ X12      (dès réception des contrats)
                ├─▶ X15, X16, X17, X18 (en parallèle selon les socles 10, 19, 25)
                └─▶ X13, X14 (dès convention / agrément)
X19 en continu, dès que le texte intégral de l'arrêté est lu.
```
**Parcours minimal vendable**, en deux paliers [relecture, obj. 11] : (1) **sur tableurs** — X0, X1, X2a–c, X3, X4 (COG, calendrier, jours fériés), X7 : vendable sans aucune démarche externe (privé, hors contrat, reprise) ; (2) **SIECLE et STS-web** — X5, X6 : sans convention ni agrément, mais conditionné aux jeux d'or anonymisés (D-14) et à la recette sur trois établissements avant de quitter le statut `Experimental`. Un collège public ou un lycée privé y charge alors SIECLE et STS-web lui-même, réconcilie sans risque, et voit ce qui manque avant chaque échéance. **Pour un établissement public, la vente exige en plus X8 + X9 + X10 + X11 + X12** (exigences n° 6 et 15) : c'est le chemin critique, gouverné par les démarches D-1 à D-7.

### 25.3 Correspondance avec les lots de l'étude

IOP-L0 → X4 (+ identifiants : socles 2, 4, 6 déjà conçus) ; IOP-L1 → X1, X2 ; IOP-L2 → X13 ; IOP-L3 → X5, X6 ; IOP-L4 → X14 ; IOP-L5 → X8, X9, X10 ; IOP-L6 → X11 ; IOP-L7 → X12 ; IOP-L8 → X12 ; IOP-L9 → socle 22 sur X7 ; IOP-L10 → socles 23, 37 sur X7 ; IOP-L11 → X19 ; IOP-L12 → veille (§26).

### 25.4 [ajout personas] Répartition des ajouts dans les lots

- **X1** : surveillance des expirations (R-X26) ; essais de connexion (ITP-134) ; catalogue des rejets connus (ITP-123).
- **X2a** : candidats incluant les sortis, jumeaux, affectés du socle 3 (CX-49, 50, 53). **X2c** : garde-fou conscient de la bascule (R-X20).
- **X3** : photos (FX-108), restauration en tableur (FX-88), résultats (FX-09), **reprise structurée de N-1** (FX-106) avec le socle 28.
- **X7** : dépôt déclaré, export périmé, attestation, journal scellé, effectifs datés, contrôle de sincérité, concordance BEE, rejets non structurés, reprise d'un dépôt, calendrier national (R-X21, R-X22 ; ITP-120, 124–128, 140, 141, 152) ; exports de travail FX-41, 42, 66, 67.
- **X7 bis (nouveau) Cohabitation** : profil `prive_contrat_cohabitation`, journal à reporter, sorties à remonter (R-X24, FX-107, ITP-119, 121) ; **dépend de X7 ; à livrer avant le premier pilote privé** si Q-X34 retient la cohabitation.
- **X8** : règle de rentrée (Q-X35), R-X19, révocation générale, coupe-circuit (R-X25 ; ITP-135, 136). **X8 bis (nouveau) SSO entrant depuis l'ENT de la collectivité** (FX-99) : après D-17 ; mêmes tests négatifs que X8.
- **X17** : annuaires existants et local (FX-97, 98), exclusions par défaut (R-X18), reconfirmation sur élargissement (R-X17). **X18** : CFONB 120, mandats et échéanciers (FX-75, 76) avec le socle 25. **X19 bis** : bac à sable, dépréciation, journal des lectures, purge (R-X28, ITP-146, 147).
- **Qualification** : un environnement ScolIA à données fictives ouvert aux DSI et aux partenaires (M-478, M-479). **Recette personas** (§23) : ajouter la bascule de juillet, la rentrée ÉduConnect avec un parent révoqué, la révocation générale, l'export périmé, la reprise qui contient une sanction effacée.

---

## 26. Démarches hors code (chemin critique)

| N° | Démarche | Auprès de | Ce qu'on demande | Débloque | Délai [H] | Porteur |
|---|---|---|---|---|---|---|
| **D-1** | Lire le **texte intégral** de l'arrêté du cadre de référence (JO du 23 septembre 2026) et de ses annexes ; poser les questions d'application (privé sous contrat ? ENT ou service de vie scolaire ? délais de l'annexe 4 ?) | Légifrance ; DNE | texte ; classement de ScolIA | X19, priorités | jours (texte) ; mois (réponse) | Florian / structure juridique |
| **D-2** | **Inscription éditeur** et accès à l'espace éditeurs de **Pléiade** (spécifications des interfaces nationales [S]) ; demandes de **validation LSU**, **LSL**, **certification Parcoursup** | DNE, `communication-editeurs@education.gouv.fr` [S] | procédure, spécifications, calendrier de qualification | socles 22, 23, 37 ; FX-04 | plusieurs mois | Florian |
| **D-3** | **Agrément « logiciel privé »** pour la remontée BEE (NetSynchro, OMOGEN Fichier) ; état d'OMOGEN en 2026 | DNE / service SIECLE | agrément, certificat, environnement de qualification | X14 | plusieurs mois | Florian |
| **D-4** | **Raccordement ÉduConnect** : éligibilité d'un éditeur ; public et privé sous contrat ; protocole (OIDC ?) ; attributs ; environnement de test ; position sur une passkey en plus et une connexion de secours | équipe ÉduConnect (DNE) | contrat d'interface (« accrochage ») | X9, Q-X1 | mois | Florian |
| **D-5** | **Guichets Agents** : national ou par académie ; maîtres du privé sous contrat | DNE, DSI académiques | contrat, attributs | X10 | mois | Florian |
| **D-6** | **GAR** : projet ENT d'un éditeur multi-académies (`idEnt`), ENT d'un établissement privé isolé, documents en diffusion restreinte (RTFS, XSD), canal et fréquence, environnement d'intégration | équipe GAR | raccordement, documents | X11, X12, Q-X7 | mois | Florian |
| **D-7** | **Convention AAF** avec la DSI de l'académie du premier établissement public pilote | rectorat | convention, fichiers d'exemple | X13 | mois | établissement pilote + Florian |
| **D-8** | **ONDE V2 interface école** (socle 50) | DNE | validation | socle 50 | mois | plus tard |
| **D-9** | **Enseignement catholique** : interfaces Gabriel / SOLFEGE, attentes des directions diocésaines et des UDOGEC | SGEC, FNOGEC | formats, besoins | FX-60–62 | semaines | Florian |
| **D-10** | **Agricole** (si visé) : Fregata, CNEAP | ministère de l'agriculture | — | FX-07 | — | Q-X25 |
| **D-11** | **DNMA** : obligation de balisage pour un outil de vie scolaire (citée par une foire aux questions de 2025 [S]) | DNE | référentiel en vigueur | X19, Q-X21 | semaines | — |
| **D-12** | **Licences** des données ouvertes (BCN, annuaire de l'éducation, COG, BAN) | sites officiels (accès réseau de la session) | texte de licence | X4 | jours | **2026-11-09** : annuaire et calendrier en Licence Ouverte 2.0 [V] ; **BCN sans licence** → demande écrite à la DEPP (Q-X46) ; COG et BAN à relire |
| **D-13** | **Réseau de la session** : autoriser `legifrance.gouv.fr`, `education.gouv.fr`, `eduscol.education.gouv.fr`, `gar.education.fr`, `educonnect.education.gouv.fr`, `data.gouv.fr`, `data.education.gouv.fr`, `insee.fr`, `bcn.depp.education.fr`, `cyber.gouv.fr`, `cnil.fr` | réglages de l'environnement | — | passe de vérification de tous les [NV] | immédiat | **fait le 2026-11-09** (`VERIF_09_SOURCES.md`) ; reste bloqué : `education.gouv.fr` et eduscol (403 Cloudflare), à relire dans un navigateur par Florian (eduscol 4920, annexe 2 de l'arrêté en PDF) |
| **D-14** | **Jeux d'or** : obtenir d'un établissement pilote des exports SIECLE et STS-web **anonymisés chez lui** par un outil livré par ScolIA (ITP-117), vérifiés par une personne de l'établissement avant envoi ; aucune donnée réelle ne quitte l'établissement | établissement pilote | fichiers anonymisés, convention | X5, X6, X16 | semaines | Florian |
| **D-15** | **Contrats et RGPD** : modèle de contrat de sous-traitance, AIPD de l'ENT (fédération, GAR, provisionnement), notice publique des flux, statut de RU-003 | DPO | validation | §17 | semaines | Florian + DPO |
| **D-16** | **Hébergement des clés** (IdP, HMAC, coffre de secrets) chez l'hébergeur | Scaleway | service de gestion de clés [H] | X8, X11 | semaines | technique |
| **D-17** | **[ajout personas] Positionnement dans le public** : raccordement d'un service de vie scolaire à l'ENT d'une collectivité (protocole, fiche de raccordement) ; accès d'un tel service à l'AAF | collectivités (départements, régions), DSI académiques, DNE | position, contrat de raccordement | Q-X33, X8 bis, X13 | mois | Florian |
| **D-18** | **[ajout personas] Dossier technique de raccordement** (architecture, hébergement, AIPD, adresses réseau sortantes publiées, contacts, procédure d'incident) | — (à rédiger) | dossier type réutilisable | D-4 à D-7, canaux bancaires | semaines | technique |
| **D-19** | **[ajout personas] Export complet chez l'ancien éditeur** : le client demande lui-même à son éditeur (portabilité, réversibilité contractuelle) l'export de ses données dans un format structuré | établissement pilote → son éditeur | export complet et sa description | FX-102, FX-106, X3, socle 28 | semaines | établissement pilote |
| **D-20** | **[ajout personas] Banque** : formats de relevés (CFONB, camt), reprise des mandats sans changement de RUM, canal | banque de l'OGEC pilote | règles écrites | FX-75, FX-76, X18 | semaines | comptable pilote |

---

## 27. Questions à trancher

| N° | Question | Recommandation |
|---|---|---|
| **Q-X1** | Passkey (D-009) et connexion fédérée nationale : la connexion ÉduConnect / Agents suffit-elle ? | **Oui pour l'usage courant** dans un établissement où le fournisseur est activé ; actions ★ : passkey récente **ou** connexion fédérée récente de niveau multifacteur ; passkey toujours proposée, obligatoire pour les comptes locaux (§10.7). Exception explicite à D-009, à acter. |
| **Q-X2** | Où vit la fédération entrante ? | **Dans le noyau sécurité** (seule la sécurité crée une session) ; tables 3910 réservées ici, à renuméroter dans 1000–1199 si Q-B2 les y range. |
| **Q-X3** | Guichet central `connexion.scolia.fr` avec ticket à usage unique vers le sous-domaine ? | **Oui** : un seul fournisseur de service déclaré auprès des fournisseurs nationaux, D-012 préservée. |
| **Q-X4** | Rattachement sans jointure : code de rattachement remis par l'établissement ? | **Oui**, un code par personne (D-014) ; jamais de rattachement par e-mail ou nom (R-X11). |
| **Q-X5** | Si un fournisseur impose SAML : crate Rust dans une brique isolée, ou passerelle externe ? | **Attendre la réponse sur le protocole** ; OIDC d'abord ; si SAML est exigé, brique Rust isolée à profil restreint, dépendance choisie après analyse de licence et de sécurité, revue externe ; une passerelle (logiciel libre auto-hébergé) seulement en solution temporaire et après accord. |
| **Q-X6** | ScolIA fournisseur d'identité pour le GAR : OIDC ou SAML ? | **OIDC** : le contrat SSO ENT 8.2 l'admet depuis octobre 2022 [V#8] ; SAML seulement si le porteur l'impose. **Mis à jour 2026-11-09.** |
| **Q-X7** | `idEnt` : un projet ENT ScolIA, un par client, un par académie ? **[2026-11-09]** Le projet ENT est celui d'un **porteur public** (collectivité ou académie) [V#7] : un `idEnt` par projet porté, jamais « un par éditeur ». Et un professeur dans deux établissements, ou un élève qui change d'établissement, garde-t-il le même identifiant GAR ? | À demander au GAR (D-6) **avant de figer** `ent_idp_identifiant_service` (aujourd'hui par service et par établissement) ; modèle neutre en attendant. |
| **Q-X8** | Mémoire de la source par empreinte HMAC et différence à trois voies ? | **Oui** (§6.6) : c'est la garantie « jamais d'écrasement, jamais de question répétée ». |
| **Q-X9** | Les avis aux familles déclenchés par les commandes pendant un import sont-ils tus ? | **Oui par défaut** (`origine = import`) ; une étape explicite « prévenir les familles concernées » reste possible après application. |
| **Q-X10** | Disparition d'un enregistrement : que faire ? | **Jamais d'application** ; signaler, ou proposer « sortie probable » selon le connecteur. |
| **Q-X11** | Seuils du garde-fou de volume ? | **15 %** des entités avec un changement d'identité, ou **5 %** de disparitions → blocage et ★ ; bornes de réglage 5–30 %. |
| **Q-X12** | Durées de conservation (fichiers bruts, normalisés, exports, traces) ? | §17 (brut 30 j ; normalisé jusqu'à la clôture ; exports selon le flux) ; **décision DPO**. |
| **Q-X13** | Réception centrale multi-établissements (si une académie livre un seul flux) ? | **Oui**, processus plateforme isolé, routage par UAI seule, quarantaine sans lecture (§6.9). |
| **Q-X14** | Privé sous contrat : ScolIA remplace-t-il le logiciel agréé (remontée BEE) dès la première version ? | D-024 (équivalent Charlemagne) l'exige à terme : **lancer la validation éditeur de l'interface « Import du privé » maintenant (D-2/D-3)** — c'est une validation par application sur spécifications Pléiade, pas un agrément général [S, V#17] ; au pilote, **cohabitation** (le client garde son logiciel agréé pour BEE), puis bascule à l'agrément. |
| **Q-X15** | Les exports officiels sont-ils produits par les socles métier sur le moteur commun ? | **Oui** (confirme `SOCLES.md` §9) ; le socle 9 tient leurs fiches de flux et le générateur commun. |
| **Q-X16** | Annuaires sortants (Google, Microsoft, Apple) dans la première version ? | **Export OneRoster / SDS / Apple en fichier d'abord** (aucun secret d'API) ; synchronisation automatique ensuite. |
| **Q-X17** | ScolIA fournisseur d'identité pour Google et Microsoft (« une passkey pour tout ») ? | **Oui, après X11** ; fort argument, coût faible une fois l'IdP construit. |
| **Q-X18** | Normalisation des adresses : jeu BAN téléchargé ou API de la Géoplateforme ? | **Jeu téléchargé, interrogé chez nous** : aucune adresse de famille ne sort. |
| **Q-X19** | Contenu des abonnements iCal des familles : travail à faire inclus ? salles ? | Emploi du temps et travail inclus, **sans nom d'autre personne** ; salles et noms des professeurs selon l'établissement (défaut : salle oui, nom du professeur non). |
| **Q-X20** | CalDAV (écriture depuis un agenda externe) ? | **Non en première version.** |
| **Q-X21** | Balisage DNMA ? | Le DNMA est un **service socle** de l'arrêté de 2026 pour le public [V#40] ; obligation pour un service de vie scolaire à lire ligne à ligne dans l'annexe 4 et le RFTI 3.0 (D-11) ; si exigé : balisage **côté serveur**, minimal, sans script tiers dans les pages. |
| **Q-X22** | API partenaires (restauration, transport, CDI) ? | Contrat conçu ici ; **construit après le pilote**, partenaire par partenaire, avec consentement du chef d'établissement. |
| **Q-X23** | Anonymiseur local livré aux établissements pilotes pour constituer les jeux d'or ? | **Oui** (outil libre, exécuté chez l'établissement, sortie relue par lui) ; aucune donnée réelle chez les développeurs. |
| **Q-X24** | Tables globales `ref_*` sans `etablissement_id` : exception au RLS ? | **Oui**, déclarée et testée : lecture seule pour les rôles d'établissement, écriture par la seule tâche de plateforme, aucune donnée personnelle. |
| **Q-X25** | Enseignement agricole dans la cible ? | **Pas dans la première version** ; modèle compatible (profil `agricole_*`). |
| **Q-X26** | Hors contrat : quelles transmissions légales (liste des élèves, registre) ? | **Vérifié 2026-11-09** [V#35] : registre des **internes** (R. 442-1) à tenir à disposition ; état annuel des **enseignants** en novembre (D. 442-22-1) ; aucune transmission annuelle de liste d'élèves trouvée. ScolIA : registre des internes, liste des élèves et état des enseignants exportables, datés, figés ; rien d'envoyé d'office. |
| **Q-X27** | Comptabilité : formats propriétaires (Sage, EBP, Cegid, Quadra) ? | **FEC + CSV paramétrable** ; formats propriétaires seulement à la demande d'un client signé. |
| **Q-X28** | EPLE public : interface Op@le (CRDC) ? | **Non tant que le ministère ne l'ouvre pas** ; état des droits constatés à saisir par l'établissement. |
| **Q-X29** | Prénom d'usage et nom d'usage : quels exports les portent ? | Exports nationaux : **état civil seulement** ; GAR, annuaires, calendriers : **usage** (ce que l'élève vit au quotidien), sauf contrainte du format. |
| **Q-X30** | Plage de repli si Q-B2 n'est pas adoptée ? | **420–439**. |
| **Q-X31** | Une division ou un groupe sans code national bloque-t-il l'import ? | **Non** à l'import (proposition), **oui** au pré-contrôle des exports qui l'exigent. |
| **Q-X32** | Faut-il un « mode essai » (appliquer sur une copie) ? | **Non** : la prévisualisation est l'analyse réelle et l'annulation existe ; une copie de base multiplierait les données personnelles. |
| **Q-X33** | **[ajout personas]** Dans le public, ScolIA est-il l'ENT, ou un service de vie scolaire raccordé à l'ENT de la collectivité ? | **Tranché par le texte (2026-11-09)** : l'arrêté définit l'ENT comme mis à disposition « **par le biais d'un projet territorial** » et, à part, le « **service numérique outillant les missions de vie scolaire** » (absences, notes, évaluations, bulletins) [V#4]. Dans le public, ScolIA est donc un **service de vie scolaire** (délais : ÉduConnect et protocoles 10 mois, Agents et GAR 22, AAF 34 [V#3]) ; il n'est ENT que s'il est retenu par un projet territorial. Dans le privé (hors champ de l'arrêté), ScolIA peut être l'ENT de l'établissement. Conséquence : **X9 (ÉduConnect) devient le premier lot fédéré du public** (échéance juillet 2027) ; D-17 avant X11–X13. |
| **Q-X34** | **[ajout personas]** Première année d'un lycée privé qui quitte Charlemagne : quels modules basculent, comment sont produits BEE, LSU, LSL, Parcoursup tant que les validations manquent ? | **Bascule par module** : ScolIA pour l'élève, l'administratif, la vie scolaire, les inscriptions, les finances dès l'an 1 ; cohabitation outillée (X7 bis) pour BEE ; pour LSU, LSL, Parcoursup : soit les notes restent dans le logiciel agréé l'an 1, soit saisie dans les applications nationales aidée par ScolIA — **à trancher avec Florian au cas par cas du pilote** ; reprise de N-1 (FX-106) dans tous les cas pour un lycée. |
| **Q-X35** | **[ajout personas]** Rattachement de rentrée : confirmation automatique sur chaîne officielle complète ? | **Oui** (§10.6, note) ; humain sur discordance ; R-X19 pour les personnes révoquées. |
| **Q-X36** | **[ajout personas]** Reprise de N-1 : jusqu'où ? | Lycée : évaluations de N-1 structurées (obligatoire) ; collège : bulletins de N-1 en PDF, positionnements de cycle si disponibles ; au-delà : archives PDF consultables ; rien au-delà des durées de conservation (R-X23). |
| **Q-X37** | **[ajout personas]** Un ensemble scolaire (école, collège, lycée, plusieurs UAI) = un seul établissement ScolIA ? | **Oui par défaut** (continuité, D-013 respectée) ; plusieurs tenants seulement si les entités juridiques l'exigent. |
| **Q-X38** | **[ajout personas]** Un professeur responsable de l'EDT peut-il exporter la structure (D-022) ? | **Oui** par `echanges.exporter_structure`, sans donnée nominative d'élève ; les exports officiels restent interdits aux professeurs. **À confirmer par Florian (lecture de D-022).** |
| **Q-X39** | **[ajout personas]** Relevés CFONB 120 en plus de camt ? | **Oui** (FX-75). |
| **Q-X40** | **[ajout personas]** Export vers l'annuaire local (annuaire Windows, serveur pédagogique) ? | **Oui**, fichier sans mot de passe (FX-97) ; synchronisation directe plus tard. |
| **Q-X41** | **[ajout personas]** FranceConnect en direct pour les familles d'un établissement privé sans ÉduConnect ? | **Éligibilité NV** (conditions des fournisseurs de service FranceConnect non lisibles le 2026-11-09, V#13 ; contact DINUM à prendre) ; en attendant : passkey et code de rattachement ; ne rien construire. |
| **Q-X42** | **[ajout personas]** Facturation électronique et Chorus Pro pour l'OGEC ? | **Vérifié en partie (2026-11-09)** [V#38] : un OGEC **non assujetti** à la TVA n'est pas concerné (ni émission ni réception) ; s'il est **assujetti, même exonéré** (cantine, internat, activités lucratives au-delà de 81 051 € en 2026), il doit pouvoir **recevoir** dès le 1/09/2026. Établir le statut TVA du pilote avec son expert-comptable avant le socle 48 ; Chorus Pro pour un non-assujetti **NV**. |
| **Q-X43** | **[ajout personas]** Contenu du dossier de transfert (FX-100) au regard de D-027 (B) ? | Bulletins, livret et certificat de radiation ; **aucun dispositif ni information sensible par défaut** : ils passent par la famille ou la notification officielle. |
| **Q-X44** | **[ajout personas]** Qui tient le calendrier des campagnes nationales ? | **L'opérateur**, référentiel global à couche académique ; l'établissement signale un écart, il ne ressaisit pas (S-74). |
| **Q-X45** | **[vérif 2026-11-09]** Un établissement **privé sous contrat isolé** (sans projet territorial) peut-il accéder au GAR, et par quel porteur public ? | Le GAR lui est ouvert (arrêté 2017 [V#6]) mais le raccordement passe par un porteur public via la DANé [V#7] : **demander à `dne-gar@education.gouv.fr` et à la DANé** (D-6) ; ne pas construire X11–X12 avant la réponse. |
| **Q-X46** | **[vérif 2026-11-09]** Importer la **BCN** alors qu'aucune licence n'est publiée ? | **Non** tant que la DEPP n'a pas répondu par écrit (D-12) ; les codes utiles arrivent par les exports SIECLE et STS de l'établissement. |
| **Q-X47** | **[vérif 2026-11-09]** Le calendrier de l'annexe 4 (ÉduConnect et protocoles à 10 mois pour la vie scolaire, soit juillet 2027) change-t-il l'ordre des lots pour le public ? | **Oui si le public est visé avant 2028** : X8 puis X9 avant X11–X13 ; sinon inchangé. **Tranché (D-029, 2026-11-09)** : public visé avant 2028 → X8 puis X9 avant X11–X13. |

---

## 28. Récapitulatif : faits, hypothèses, contradictions

**Faits du dépôt [F]** : §2 (migrations 060, 090, 110, 144, 281, 283 ; `imports.rs`, `ine.rs`, exports CSV, `lsu_reference_scolia.xml`, `ical.rs`, `routes.rs:618-620, 8324`).

**Faits sourcés en extrait [S] (à relire sur le texte)** : noms des exports SIECLE ; export `sts_emp` et calendrier EDT → STS ; convergence NetSynchro/SiecleSynchro vers OMOGEN Fichier (note SIECLE 24.4.0.0) ; éditeurs ONDE V1/V2 ; spécifications ONDE sur Pléiade ; COG CSV UTF-8 ; calendrier scolaire et jours fériés sous Licence Ouverte 2.0 ; transfert de l'API Adresse à l'IGN ; aSc XML ; SDS CSV v2.1, fin de SDS Classic ; Apple School Manager (SFTP, OneRoster) ; OneRoster 1.2 ; RFC 5545, 4791, 7643, 7644 ; pain.008, camt.053/054 ; FEC (art. A47 A-1, BOFiP) ; ANC 2018-06 et annexe OGEC 2021 ; Gabriel (SGEC) ; Fregata ; hors contrat (déclaration, contrôle, registre d'internat) ; guide ANSSI OIDC 2020 et guide MFA 2021 ; DNMA cité pour les outils de vie scolaire ; tout ce qui vient de l'étude du 2026-11-05.

**Hypothèses [H] principales** : AAF quotidien ; guichet Agents par académie ; réception centrale possible ; durées de conservation proposées ; cotisations UDOGEC sur effectifs ; personnels de la collectivité hors STS.

**Vérifié [V] le 2026-11-09** (`VERIF_09_SOURCES.md`) : arrêté du 7 septembre 2026 (art. 1–4, annexe 4 : champ public, services socles, définitions ENT / vie scolaire, critères 4, 5, 7, 15, 18 et délais) ; GAR (ouverture au privé, porteur public via la DANé, OIDC admis depuis 2022, SFTP, console d'affectation) ; licences de l'annuaire de l'éducation et du calendrier scolaire (Licence Ouverte 2.0) ; France = `99100` ; absence de licence BCN ; R. 442-1 et D. 442-22-1 (hors contrat) ; R. 511-13 IV (sanctions) ; DNB 40/60 ; circuit LSL → Cyclades ; Parcoursup sans remontée → saisie par l'élève ; facturation électronique des associations non assujetties.

**Non vérifié [NV] — interdit de coder dessus** (recherché sans succès le 2026-11-09, sauf mention) : schémas et encodages SIECLE, STS, AAF, ONDE, alimentation GAR (réservés : Pléiade, paquetage GAR) ; attributs ÉduConnect et Agents ; annexe 2 de l'arrêté (protocoles : non lue) ; format et clé de l'INE (aucun algorithme publié trouvé) ; lettre de contrôle de l'UAI (idem) ; licence du COG ; XSD LSU 2ᵈ degré, LSL ; documentation éditeur Parcoursup ; Notanet, Cyclades ; Chorus Pro pour un non-assujetti ; obligation de FEC d'un OGEC ; statut de l'arrêté de 2006 et durées RU-003 ; obligation d'AIPD propre aux ENT ; voie GAR d'un privé isolé (Q-X45).

**Contradictions relevées** :
- **C1** `SOCLES.md` §9 range « GAR (transmission des affectations) » dans le socle : **faux sur le sens** — les affectations se font dans la console du GAR ; l'ENT transmet l'annuaire et l'identité [S, étude §13]. Corrigé dans `SOCLES.md` (renvoi à cette conception).
- **C2** Réforme du DNB 2026 (contrôle continu 40 % ? échelle ?) et contenu Affelnet 2026 (socle retiré ? moyennes seules ?) : sources grand public contradictoires ; à lire au BO. **Tranchée le 2026-11-09** : DNB 40/60 sur les moyennes annuelles de 3ᵉ [V#25] ; Affelnet lycée : moyennes annuelles seules [S, V#20].
- **C3** `06_PERSONNELS.md` §8.11 liste `gar` et `guichet_agents` parmi les systèmes d'identifiants **de l'agent** ; ici, les identifiants de service (GAR) vivent dans `ent_idp_identifiant_service` et la connexion Agents dans `ent_fed_rattachement`. Recommandation : retirer `gar` et `guichet_agents` de `ent_agent_identifiant` (garder `aaf_jointure`, `sts`).
- **C4** FEC : 18 champs selon des guides, 23 selon un éditeur ; à trancher sur l'article A47 A-1. **2026-11-09** : 18 selon le BOFiP [S, V#37] ; tableaux à relire avant de coder.
- **C5** Dates de remontée Parcoursup différentes selon les académies (début et fin mars) : normal (académies) mais l'échéance est donc un **paramètre local**, pas une constante.
- **C6** Liste des éditeurs validés LSU : l'étude citait trois éditeurs pour le LSL 2026 ; la recherche du 2026-11-08 en cite huit pour le second degré : à reconfirmer à la source. **2026-11-09** : un extrait d'eduscol 4920 donne APLIM, CGI, Docaposte/Index pour « LSU et LSL » (mars–avril 2026) et un autre les huit noms sans préciser l'application [S, V#22] : les deux listes coexistent, **périmètre à lire dans un navigateur** (eduscol bloque les outils).
- **[vérif] C18** La conception citait « exigence n° 6 » (ÉduConnect/Agents) et « exigence n° 15 » (GAR) de l'arrêté de 2026 : dans l'annexe 4, ce sont les **critères 4, 5 et 7**, le n° 15 portant sur l'AAF [V#2]. Corrigé dans tout le document (§0.8, §9.10, §10.3, §10.4, §10.7).
- **[vérif] C19** `SOCLES.md`, la passe personas et §11 supposaient que l'éditeur se raccorde au GAR : le raccordement est demandé par un **porteur public** [V#7] → Q-X45, Q-X7.
- **[vérif] C20** FX-63 citait « L. 442-1 » pour le registre : c'est **R. 442-1**, et pour les **internes** seulement [V#35].
- **C7** Licence de FET : GPL-2.0 ou AGPL-3.0 selon les sources ; sans effet ici (lecture du format, aucune reprise de code).
- **C8** `03_INSCRIPTIONS.md` donne au socle 3 l'action `imports.appliquer` ; elle est remplacée par les droits `echanges.*` (§18), le socle 3 gardant `inscriptions.gerer` pour transformer des affectés en dossiers.
- **C9** Le CLAUDE.md demande Haiku pour la recherche : fait, mais **aucune** page officielle n'était joignable ; tout le §9 garde ses marques jusqu'à la passe de vérification (D-13).
- **[ajout personas] C10** `SOCLES.md` §9 dit encore « parcours minimal vendable X0–X7 sans aucun accès officiel » ; §0.9 et §25.2 ont été corrigés (jeux d'or réels pour X5–X6). Note ajoutée dans `SOCLES.md`.
- **[ajout personas] C11** §9.10 « privé sous contrat : ScolIA fait foi » contre Q-X14 (le logiciel agréé remonte pendant la cohabitation) → profil `prive_contrat_cohabitation` (R-X24), Q-X34.
- **[ajout personas] C12** §10.2 (second élément obligatoire, obj. 2) contre R-X11 (a) (jointure = preuve forte) → Q-X35.
- **[ajout personas] C13** ITP-62 sous `echanges.exporter` (marqueur `export`) contre un professeur responsable de l'EDT (**D-022**) → `echanges.exporter_structure`, Q-X38.
- **[ajout personas] C14** Hypothèse implicite « ScolIA = projet ENT » (§11, X11, exigences n° 6 et 15) contre la pratique des ENT de collectivité [H] → Q-X33.
- **[ajout personas] C15** FX-100 muet sur son contenu ; **D-027** (B) → Q-X43.
- **[ajout personas] C16** Q-X32 (pas de mode essai) et l'essai d'un SSO : compatibles (ITP-134 n'essaie qu'une connexion).
- **[ajout personas] C17** Q-X11 (5 % de disparitions) contre la bascule de juillet (25 à 35 % de départs normaux) → R-X20.

---

## 29. Sources consultées (2026-11-08, extraits de moteur de recherche ; aucune page officielle ouverte)

**Passe du 2026-11-09** : les sources lues, leurs URL et leur niveau sont dans `docs/socles/VERIF_09_SOURCES.md` (42 éléments). Elles priment sur la liste ci-dessous.

Outre les sources de `INTEROPERABILITE_NATIONALE.md` §16 : Index Éducation (docs EDT : SIECLE, STSWEB import, envoi des services, NetSynchro ; Pronote : export Parcoursup, export LSL) ; IREM de Lille (import élèves depuis SIECLE) ; guides SIECLE BEE (La Réunion 2016, Nice EPHC 2022) ; note de version SIECLE 24.4.0.0 (ac-Nice) ; eduscol 4920 (interfaces éditeurs, Pléiade) ; education.gouv.fr (BE1D devient ONDE ; ÉduConnect et Scolarité services ; fiche questions-réponses de la journée nationale des ENT) ; DANE Lyon (mettre à jour son collège sur l'ENT à la rentrée) ; IH2EF (emploi du temps) ; Skolengo (EDT 2026, interopérabilité) ; doc BCN scolarité (`bcn.depp.education.fr`) ; OpenStreetMap (UAI) ; insee.fr et data.gouv.fr (COG, calendrier scolaire, jours fériés) ; data.gouv.fr (transfert de l'API Adresse) ; CNIG (juin 2026) ; aide aSc Timetables ; FET (openSUSE, Wikipédia) ; AED UnDeuxTEMPS ; académies de Lille, Paris, Orléans-Tours, Bordeaux, Versailles, Lyon, Guadeloupe, Martinique ; SNES Lille ; digischool, diplomeo (DNB, à confirmer) ; justice.fr (candidat libre) ; Landot avocats (hors contrat) ; Fondation pour l'école ; info.agriculture.gouv.fr (Fregata) ; présentations Gabriel / SOLFEGE (enseignement catholique de Bretagne) ; schéma directeur RGPD du SGEC (2018) ; modèle d'annexe OGEC 2021 ; Option Finance et Associathèque (ANC 2018-06) ; BOFiP (FEC) ; guides bancaires ISO 20022 (HVB, UBS) ; Microsoft Learn (School Data Sync, provisionnement Google) ; Apple (Apple School Manager) ; Claris (OneRoster) ; 1EdTech (OneRoster 1.2) ; IETF (RFC 7643) ; icalendar.org, CalConnect ; ANSSI (guide OIDC 2020, guide MFA 2021) ; Lexing (RU-003) ; Banque des territoires (décret 2025-1165, arrêté de 2026, RNIE) ; Café pédagogique (doctrine technique 1.1) ; Acteurs publics (ProConnect).


---

## 30. Relecture indépendante (2026-11-08) et traitement

Relecture par un agent distinct (lecture seule), centrée sur la sécurité, la cohérence, les marques de vérification et la faisabilité. 18 objections, toutes traitées dans le texte (marque « [relecture, obj. n] »).

| # | Gravité | Objection | Traitement |
|---|---|---|---|
| 1 | haute | Le guichet central lit plusieurs établissements sans être déclaré comme exception ; clé du HMAC du sujet non dite ; `etab` non validé | §10.2 (rôle dédié, clé globale de pseudonymisation au coffre, liste blanche) ; §16.2 |
| 2 | haute | Réponses différentes selon les branches ; nom d'enfant affiché avant preuve | §10.2 : aucun nom avant preuve, page unique, débit limité |
| 3 | haute | Liaison du ticket au navigateur impossible avec un cookie `__Host-` d'un autre hôte | §10.2 : cookie de liaison posé par l'hôte de l'établissement, nouvelle session |
| 4 | haute | SSRF sur les adresses saisies (SFTP, ICS, webhooks, OIDC) | §16.1 : nouvelle ligne |
| 5 | moyenne | Le garde-fou UAI ne protège pas d'un établissement qui déclare l'UAI d'un autre | §6.3 : UAI unique sur la plateforme, validée par l'opérateur ; §6.9 : UAI ambiguë → quarantaine |
| 6 | moyenne | iCal : tenant inconnu à la lecture, jeton dans les journaux, droits non relus | §15.5 : sous-domaine, 256 bits, journaux filtrés, droits à chaque lecture, expiration |
| 7 | moyenne | Conservation : tables à données personnelles oubliées | §17 complété |
| 8 | haute | Table à trois voies : écriture de B, annulation, source qui vide, propositions en attente, deux sources | §6.6 : règles ajoutées ; ligne vide retirée |
| 9 | moyenne | Empreinte seule insuffisante (affichage des écarts, normalisation, rotation, dictionnaire) | §6.6 : valeur chiffrée des écarts ouverts, normalisation et clé versionnées, clé au coffre |
| 10 | moyenne | Lots bis cités au §9 absents du §25 | §25.1 : lots annexes listés |
| 11 | moyenne | « Sans aucun accès officiel » contredit par les jeux d'or ; X3 dépend du socle 3 | §0.9, §25.2 (deux paliers), §25.1 (X3) |
| 12 | moyenne | Propriété de 3908 et 3910 provisoire | §15.6 : à trancher avant X1 (Q-X2) |
| 13 | moyenne | Exigences de l'arrêté et formats tiers affirmés trop fermement | §0.8 au conditionnel, FX-81, 91, 93 requalifiés |
| 14 | moyenne | §10.7 rédigé comme une position contre D-009 | titre : dérogation proposée, à valider |
| 15 | moyenne | Identifiants GAR par établissement : risque de casser le pivot | Q-X7 élargie, table non figée |
| 16 | basse | Sessions qui survivent à une suspension | R-X13 : sessions révoquées |
| 17 | basse | Bascule d'année et transferts | §6.5 : correspondances annuelles, aucun transfert de correspondance (D-013) |
| 18 | moyenne | X2 trop gros ; critères non vérifiables en CI | X2a, X2b, X2c ; critères CI et recette externe séparés |

Non traité ici : la relecture n'a pas couvert intégralement §9, §12, §13, §20–§23, §27–§29 ; un contrôle automatique de cohérence des identifiants (FX, ITP, R-X, X) est recommandé au lot X1 (script de documentation).

## Ajout du 2026-11-09 — OMOGEN (information de Florian, vérifiée en extrait)
- **OMOGEN** est le canal du **ministère** (pas de Charlemagne) par lequel le logiciel d'un éditeur échange directement avec SIECLE / BEE, sans export et réimport manuels du XML : l'interface « **Import du privé** » permet aux établissements privés sous contrat utilisant un logiciel d'éditeur de remonter les dossiers des élèves dans la BEE ; les anciens flux SiecleSynchro et NetSynchro convergent vers « OMOGEN Fichier » (élèves, liens élèves-groupes). [S] eduscol 4920 « Interfaces entre les applications nationales et les solutions numériques tierces pour l'éducation » (https://eduscol.education.gouv.fr/4920/interfaces-entre-les-applications-nationales-et-les-solutions-numeriques-tierces-pour-l-education) ; documentation académique de Nice, mises à jour SIECLE 24.2–24.4 (https://diff.ac-nice.fr/telechargement/administratif/Documentations_version/), lues en extrait de recherche.
- **Conséquence** : c'est ce que ScolIA doit faire pour le pilote (Florian utilise aujourd'hui OMOGEN via Charlemagne). L'accès passe par l'inscription éditeur (F-06, reportée vers mai 2027). En attendant : lecture et écriture des XML d'après des exemples anonymisés (F-07), puis branchement OMOGEN après accréditation.

