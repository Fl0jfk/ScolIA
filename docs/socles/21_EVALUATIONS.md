# Socle 21 — Évaluations et notes (avec ce que les socles 22 et 23 en attendent)

**Date :** 2026-11-09 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code (lot 7, S2-7) pour le rattraper (`docs/PILOTAGE.md` §4, « rattrapage des domaines codés avant la méthode »). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à l'application Next.js.
**Branche :** `ent-socle-evaluations-conception` (partie de `ent-mc0hoe`, `253e725`). Code évalué sur cette base (lots 9 et 10 intégrés) et, pour le socle 5, sur `ent-integration-lot11` (`1dd39be` : périodisations 335, offre et matières 338, enseignements suivis 340) et `ent-socle-structure-s4` (cours 341).
**À lire avec :** `CLAUDE.md` ; `docs/PILOTAGE.md` ; `docs/DECISIONS.md` (D-017, D-020, D-021, **D-022**, **D-023**, D-024, D-025, D-026, D-027, D-028, D-029) ; `docs/SOCLES.md` (socles 12, 13, 20, **21, 22, 23**, 34, 35, 37, 38) ; `docs/MATRICE_CHARLEMAGNE.md` §3.7 et §5 ; `docs/SOCLE2_DOMAINES.md` §3 (EVA-01 à EVA-14) et §6.4 (bilan de S2-7) ; `docs/socles/05_STRUCTURE.md` (périodes S7, offre S5, cours S4), `06_PERSONNELS.md` (services, PP, remplaçants), `04_ADMINISTRATIF.md` §8.6 (droits du lien), `07_DOCUMENTS.md` (moteur de documents, coffre), `08_NOTIFICATIONS.md` (natures `evaluations.*`), `09_INTEROP.md` et `VERIF_09_SOURCES.md` (LSU, LSL, Parcoursup, DNB), `IA_COEUR.md` ; modèle de forme : `10_EMPLOI_DU_TEMPS.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée) ; **[NV]** règle non vérifiée sur une source officielle ; **[H]** hypothèse (pratique du terrain, marché) ; **[R]** recommandation (non décidée). Numérotation propre à ce socle : actions **EVA-100** et suivantes (EVA-01 à EVA-14 sont pris par `SOCLE2_DOMAINES.md`), règles **R-V1**…, cas limites **CE-01**…, questions **Q-EV1**…, réglages signalés **S-100**… (après S-99 du socle 10).
**Plage de migrations :** **5100–5199** si la règle « 3000 + 100 × numéro de socle » (Q-B2) est adoptée ; **repli 460–479** sinon (vérifié libre sur toutes les branches distantes le 2026-11-09 ; registre `MIGRATIONS.md`). Les socles 22 et 23 recevront 5200–5299 et 5300–5399 (ou un repli) à leur propre conception ; les **correctifs urgents** qui touchent leurs tables actuelles (conseil, bulletin, compétences) sont pris dans la plage du 21 (lot V0).
**Méthode.** Lecture des décisions, de la carte, des conceptions 04 à 10 ; lecture directe des migrations 008, 022, 056, 071, 088, 180, 181, 182 (et 335, 338, 341 sur leurs branches) ; audit indépendant du code (sous-agent Sonnet, lecture seule, ~60 fichiers), dont les **trois défauts les plus graves ont été relus ligne à ligne** (`service.rs:9881-9900`, `service.rs:6457-6475`, `routes.rs:8305-8330`) ; recherche réglementaire (sous-agent) sur Légifrance et eduscol, dont **le texte décisif** (mémento eduscol de décembre 2025 sur l'évaluation en 3ᵉ) **a été relu directement** dans le PDF officiel.

Sommaire : §0 En bref · §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs · §4 Frontières (21, 22, 23 et les autres) · §5 Réglementaire · §6 Ce que les socles 22 et 23 attendent du 21 · §7 Concepts · §8 Règles de calcul · §9 Vie d'une période · §10 Publication · §11 Cas limites · §12 Appréciations · §13 IA · §14 Droits et minimisation · §15 Notifications · §16 Échanges nationaux · §17 Actions · §18 Règles transverses · §19 Événements · §20 Écrans · §21 Modèle de données · §22 Plan de migration · §23 Lots de code · §24 Tests et recette · §25 Réglages signalés · §26 Questions à Florian · §27 Faits, hypothèses, contradictions · §28 Les cinq décisions à prendre en priorité.

---

## 0. En bref

1. **Le moteur est bon, la maison autour fuit.** Le calcul des moyennes du lot 7 (`domain-eleve/src/evaluation.rs`) est exact (fractions entières, aucun arrondi intermédiaire), testé et réutilisable ; la grille S2-7 contrôle service, effectif, portée D-022 et verrou ; le bulletin est versionné et **immuable en base**. Mais une **génération historique** de routes reste branchée sur les mêmes tables (UI JavaScript, applis mobiles) et contourne tout : **quinze défauts** (§2.2, D1 à D15), dont trois relus ligne à ligne : (D3) un **parent sans droit aux notes télécharge le bulletin** par le module documents, et **les appréciations sont lisibles par la famille et l'élève avant le conseil** ; (D2) **l'administratif peut rouvrir une période fermée** et modifier des notes sans trace ; (D1) une note peut être saisie dans n'importe quelle matière, hors période, publique aussitôt ; (D6–D7) les notes sont **écrasées et supprimées sans historique**, même publiées. **Aucune RLS** sur ces tables. Un **lot V0 de correctifs** passe avant tout le reste.
2. **Le réglementaire tranche trois points [V]** : (a) **un absent n'est jamais un zéro** — « la note zéro ne peut pas être utilisée pour sanctionner l'absence non justifiée à une évaluation disciplinaire ordinaire » ; on rattrape, puis, si la moyenne n'est pas représentative, le chef d'établissement organise une **évaluation de remplacement** ; seul l'absent non justifié à celle-ci reçoit 0 (mémento eduscol 3ᵉ, décembre 2025, p. 13 et FAQ) : le réglage existant `absent_compte_zero` doit disparaître ; (b) en **3ᵉ**, une **note sur 20 par enseignement obligatoire et par bilan périodique**, « en attente » possible en cours d'année, **moyenne annuelle chiffrée obligatoire** (DNB 2026 : 40 % de contrôle continu sur les moyennes annuelles de 3ᵉ) ; « dispensé » seulement pour EPS et LV2 ; (c) l'**avertissement est une sanction disciplinaire** du chef d'établissement (R. 511-13) : un conseil de classe ne le prononce pas ; les mentions « avertissement travail / conduite » du code actuel deviennent des **mises en garde** sans valeur disciplinaire (socle 23).
3. **Quatre gestes distincts** que l'existant confond : **saisir** (privé), **publier** (famille, élève), **arrêter** (plus de saisie, instantané figé que le conseil examine), **figer** (bulletin). À l'arrêt, tout ce qui n'est pas publié l'est : relevé famille et bulletin comptent enfin la même chose (§9, §10).
4. **L'évaluation vise un cours** (socle 5 : classe entière ou groupe, D-022 bis), dans une **période de la périodisation de la classe** (trimestres et semestres cohabitent, migration 335) ; le public est celui du cours **à la date** de l'évaluation : un élève arrivé après n'est pas « absent », il n'était **pas concerné** ; un changement de groupe garde les notes de l'ancien cours dans la moyenne de matière (§8, §11).
5. **Peu de règles de calcul, toutes expliquées** (D-023) : barème libre (sur 10, 20, 5, 40…), coefficients d'évaluation et de matière, sous-matières, deux variantes seulement (« compte si elle améliore », « ne compte pas »), moyenne annuelle selon une méthode figée pour l'année, arrondi à l'affichage seulement, statistiques masquées sous 5 élèves, pas de rang par défaut. Chaque moyenne s'**explique** ligne à ligne à la famille (§8).
6. **Ce que chacun voit est minimal** (D-022) : le professeur, ses cours et leurs élèves ; au conseil, les élèves de ses groupes, pour toutes les matières mais **seulement pendant la fenêtre du conseil** (Q-EV6) ; la famille, ce qui est publié et autorisé par **son** lien ; l'AESH, le calendrier des évaluations et les aménagements de l'élève accompagné, jamais ses notes ; les exports LSU et LSL, les seuls administratifs (§14).
7. **L'IA aide sans décider** : premières versions d'appréciations **à partir des seules données du professeur**, relues et validées une à une ; contrôle de cohérence (appréciation « excellent » avec 6/20, copier-coller) ; **alertes déterministes et expliquées** d'élèves en difficulté pour le PP et la CPE (jamais un « score de risque », jamais visibles des familles) ; préparation du conseil. Un système d'IA qui **évalue les acquis** est classé **à haut risque** par le règlement européen sur l'IA (annexe III, point 3 [NV sur le texte consolidé et le calendrier]) : ScolIA ne produit **jamais une note ni une décision** par l'IA (§13, Q-EV8).
8. **133 actions** (EVA-100 à EVA-232), **64 cas limites**, **28 règles**, **14 lots** de code (V0 à V13) avec critères de fin ; **parcours minimal vendable** : V0 à V8 ; les formats officiels LSU et LSL restent **bloqués par la démarche éditeur** (D-2 du socle 9), pas par le code (§17, §11, §18, §23).
9. **Cinq décisions à prendre en priorité** : §28.
10. **[ajout personas] Ce que la passe personas change** (`PERSONAS_21.md`, 16 personas, 90 manques M-579 à M-668). Trois **bloquants**, tranchés selon leur recommandation (D-031) : **(A)** la période et le verrou se résolvent **par élève** et non par évaluation — un groupe de spécialité réunit six classes aux dates d'arrêt différentes, une option multi-niveaux mêle trimestres et semestres (R-V29, §9.4, à poser dans le schéma de V1) ; **(B)** l'an 1 du lycée pilote (D-032 : notes et bulletins dans Charlemagne) est **outillé** : export de l'assiduité pour le bulletin Charlemagne, dépôt des bulletins Charlemagne dans ScolIA et **distribution par ScolIA seule** selon les droits des liens (lot V14, juste après V0) ; **(C)** une matière **suivie hors de l'établissement** (CNED, autre lycée, conservatoire) reçoit une **moyenne externe** qui vaut moyenne de matière (R-V38 ; sinon la 3ᵉ ne peut pas être close). Puis : appréciations ouvertes par `recevoir_bulletins` (R-V34), parent sans exercice destinataire par défaut des bulletins et décisions (R-V43), aucune activité d'un parent visible de l'autre (R-V46), guichet qui relit les droits (R-V39), assiduité bornée, aucune sanction ni rétention pour impayé au bulletin (R-V31 à R-V33), prénom d'usage (R-V36), rubrique « accompagnements » jamais vers un tiers (R-V37), exclusions écrites de l'IA (R-V40) ; 48 actions (EVA-233 à EVA-280), 29 cas limites (CE-65 à CE-93), lots V14 à V16.

---

## 1. Thèse : une note est un fait pédagogique daté, la moyenne un calcul, le bulletin un acte

Pour l'élève et sa famille, les notes répondent à trois questions : **où en suis-je, est-ce que je progresse, que dois-je faire ?** Pour le professeur, elles sont la trace de son travail d'évaluation et un outil de dialogue. Pour l'établissement, elles fondent des **actes** : le bulletin, l'avis du conseil, la proposition d'orientation, la moyenne de contrôle continu du DNB et du baccalauréat, le dossier Parcoursup. Une erreur ne fait pas un écran faux : elle change une orientation, une mention, un examen.

Douze principes [R] :

1. **La note appartient à une évaluation, l'évaluation à un cours et à une période.** Le cours (socle 5) dit à qui ; la période (socle 5) dit quand ; le service (socle 6) dit qui peut noter. Plus jamais de `classe TEXT` ni de note « hors devoir ».
2. **Le public se calcule à la date.** Qui devait composer le 12 octobre en allemand LV2 ? La fonction unique du socle 5 (`ent_public_cours_jour`) répond ; le statut `non_concerne` en découle, il n'est jamais saisi.
3. **Un seul moteur de calcul, exact et explicable**, partagé par la grille, le relevé, le conseil, le bulletin, les exports et l'IA. Deux calculs qui divergent (aujourd'hui : f64 historique contre fractions exactes) sont une faute.
4. **Absent n'est pas zéro** [V]. Le logiciel ne transforme jamais une absence en note ; il propose le rattrapage, puis l'évaluation de remplacement.
5. **Saisir, publier, arrêter, figer** sont quatre gestes, avec quatre publics.
6. **Rien ne s'efface.** Toute modification de résultat ou d'appréciation est historisée ; après l'arrêt, c'est une correction motivée, par la direction, qui reversionne l'instantané et le bulletin.
7. **La famille comprend.** Chaque moyenne s'explique ; les moyennes provisoires sont marquées ; une note retirée l'est avec un mot.
8. **Le professeur saisit comme il pense** : au clavier, en colonne, en collant un tableur, depuis la séance ou le cahier de textes, sur le web ou le téléphone ; les absents de l'appel et les dispenses sont déjà là.
9. **La fin de période se pilote, elle ne se subit pas** : état de saisie en temps réel, relances en un geste, arrêt automatique, dossier de conseil prêt.
10. **Ce que chacun voit est minimal** (D-022, socle 4) : portée au groupe, droits du lien, choix de l'élève majeur, fenêtre du conseil.
11. **L'IA propose, une personne valide** (D-025, `IA_COEUR.md`), sans note ni décision produite par un modèle.
12. **Peu de réglages, de bons défauts** (D-023) ; les données (matières, coefficients, mentions, modalités) sont libres, la présentation (bulletins) suit 3 ou 4 modèles prêts.

**Ce qui fera la différence au quotidien** [H — comparaison de mémoire avec Pronote, École Directe, Charlemagne, non vérifiée au détail] : la **grille au clavier** avec collage depuis un tableur et absents pré-remplis ; la **publication programmée** (« le jour où je rends les copies ») ; le **tableau de fin de période** (« il manque 3 évaluations en physique, 41 appréciations, M. X est en arrêt : son remplaçant est prévenu ») ; l'**instantané d'arrêt** qui rend le conseil reproductible ; la **moyenne expliquée** aux familles ; les **corrections après conseil** en un geste, tracées, sans tout réimprimer ; les **cas de parcours** (arrivée, changement de groupe, option quittée, dispense) justes sans manipulation.

---

## 2. Évaluation franche de l'existant

Lu sur `ent-mc0hoe` (`253e725`). Audit indépendant du code (sous-agent, lecture seule), constats décisifs relus directement. Les numéros de ligne sont ceux de cette branche.

### 2.1 Ce qu'il y a [F]

| Brique | Où | Ce qu'elle fait | Verdict |
|---|---|---|---|
| Moteur de calcul | `domain-eleve/src/evaluation.rs` (`moyenne_matiere` :282, générale :316, `arrondie` :252, `centiemes` :132) | moyennes en fractions entières exactes, arrondi d'affichage demi vers le haut, absent / dispensé / non noté exclus, aucune moyenne sans note | **garder**, étendre (§8) |
| Grille S2-7 | `ent-db/src/evaluations.rs` (1 654 lignes) ; `ent-api/src/evaluations.rs` (31 routes) | création d'évaluation sur un service, saisie groupée avec rapport de refus, contrôle service × effectif × portée, verrou de période relu en transaction, publication explicite, correction après fermeture motivée et journalisée | **garder**, corriger D6–D8 |
| Règles et coefficients | `180:41-63` (`ent_eva_regle`, `ent_eva_coef_matiere`) | arrondi, `absent_compte_zero`, coefficient matière × classe | **corriger** : retirer `absent_compte_zero` [V §5], cibles MEF/niveau |
| Journal de correction | `180:66-77` | ancienne/nouvelle valeur, motif, auteur | **garder**, généraliser à l'historique complet |
| Périodes de notes | `022:3-20` (`ent_note_periode`), lien à `ent_periode` par déclencheur (112) | trois statuts sans machine d'états ; code unique **à vie** par établissement | **remplacer** par un état par classe sur la période du socle 5 (§9) |
| Appréciations | `056` | une par (période, élève, matière) + générale | **corriger** : types, versions, publication |
| Conseil | `071`, `181`, `evaluations_conseil.rs` (1 144 lignes) | préparation (moyennes, assiduité, sanctions, appréciations manquantes), décisions, mentions, avis sur les vœux, réponse de la famille, bilan de cycle | **corriger** (socle 23) : verrou « tenu », historique des décisions, mentions |
| Bulletins | `182`, `evaluations_bulletin.rs` (994 lignes) | instantané JSON, empreinte, versions, **déclencheur d'immuabilité** (182:36-60), publication transactionnelle, dépôt au dossier, outbox | **garder** le modèle ; **remplacer** le PDF (texte ASCII, accents perdus, `bulletin_pdf.rs:99-116`) par le moteur du socle 7 |
| Exports LSU / LSL | `evaluations_bulletin.rs:762-980`, `evaluation.rs:489` ; CSV historiques `lsl_export.rs`, `lsun_export.rs` | XML à espace de noms **inventé** (`urn:fr:edu:scolarite:lsun:bilans:import`), LSL interne, trace `ent_eva_export` | **corriger** au moteur du socle 9 (X0 puis X7) ; jamais présenté comme officiel |
| Compétences | `088`, `competence.rs` ; `ent_bilan_cycle` (181:41-70) | deux modèles concurrents (items D1–D5 par période ; 8 composantes par cycle) | **remplacer** par un seul modèle (socle 22) |
| Écrans Leptos | `ent-web-app/src/modules/evaluations/` (accueil, grille, classe, conseil, bulletins, famille) | une première interface sérieuse | **garder**, compléter (règles, coefficients, correction, périodes, bilan de cycle absents) |
| Notifications | `reactions.rs:352-380`, consommateur `d6.evaluations` | une notification par destinataire, droits relus au traitement, sans donnée | **garder** ; brancher sur le moteur du socle 8 |
| Tests | `tests_evaluations.rs` (4 scénarios base), 33 tests de domaine, 2 scénarios web | période complète, refus, isolation, arrondis | **garder** ; ajouter routes historiques, changement de classe, fuites |

### 2.2 Défauts trouvés (à corriger avant toute extension)

| # | Défaut | Conséquence | Où [F] | Correction |
|---|---|---|---|---|
| D1 | **Saisie historique** `POST /eleves/{id}/notes` : matière quelconque, sans contrôle de service ; `periode_id` nul = hors verrou ; note sans devoir **publique immédiatement** ; la **valeur de la note est dans l'événement** | un professeur note un élève dans une matière qu'il n'enseigne pas ; fuite de donnée dans l'outbox | `routes.rs:609`, `service.rs:7129-7210`, `note_repo.rs:294-299` | V0 : route fermée (refus 410 avec message), clients migrés |
| D2 | **Deux routes de statut de période** ; l'historique (`/note-periodes/{id}/statut`) n'exige que `matieres.gerer` (administratif) ; aucune machine d'états | l'administratif **rouvre** une période fermée, les notes changent **sans journal de correction** | `routes.rs:8305-8330` (relu : `require_manage_matieres`) | V0 : route fermée ; machine d'états en base (V1) |
| D3 | **Fuites avant publication** : (a) `GET /eleves/{id}/appreciations` sans filtre de publication ni de période ; (b) compétences visibles dès la saisie ; (c) le **PDF du bulletin**, déposé au tiroir famille « standard », est lisible par **tout parent lié**, sans contrôle de `peut_voir_notes` ni de `recevoir_bulletins` ; (d) conseils de l'élève lisibles par tout parent lié | un parent séparé privé des notes par la direction télécharge le bulletin ; famille et élève lisent les appréciations avant le conseil (l'appli iOS élève les appelle) | (a) `service.rs:6457-6475` (relu), `note_repo.rs:384-425` ; (b) `service.rs:6755-6777` ; (c) `service.rs:9881-9900` (relu : seul le lien est vérifié) ; (d) `service.rs:13975-14010` | V0 : filtres de publication ; droit de lecture du document « bulletin » = `recevoir_bulletins` du lien |
| D4 | **Décision de conseil historique** : tout porteur de `conseils.gerer` qui suit l'élève (un professeur simple) décide ; notification aux parents **sans** contrôle des droits ; « tenu » ne verrouille rien ; la nouvelle voie écrase sans historique | une décision du conseil prise ou changée par n'importe quel professeur, envoyée à un parent privé d'accès | `service.rs:13827-13915`, `evaluations_conseil.rs:632-680` | V0 : route fermée ; socle 23 : décisions historisées, verrou |
| D5 | **Bulletin historique** `POST /eleves/{id}/bulletins` : un professeur qui suit l'élève publie un bulletin, sans coefficient de matière, sans version, un nouveau document et un envoi à chaque appel | deux bulletins différents pour la même période dans le coffre | `service.rs:6478-6600` | V0 : route fermée |
| D6 | **Aucun historique avant fermeture** : notes et appréciations écrasées par `UPDATE` | impossible de répondre à « cette note a-t-elle changé après que je l'ai vue ? » | `evaluations.rs:1117` | V1 : historique en ajout seul |
| D7 | **Suppression physique** d'une note quand on vide une cellule, même publiée | perte de preuve | `evaluations.rs:1102` | V1 : un résultat se remplace ou s'annule, jamais ne se supprime |
| D8 | **Pas d'unicité** `(devoir, élève)` ; le lecteur prend le plus récent, la voie historique crée des doublons comptés **deux fois** dans la moyenne historique | moyennes divergentes selon l'écran | `note_repo.rs:429-460` | V0 : dédoublonnage + contrainte unique |
| D9 | **Aucune RLS** ni clé étrangère composée sur les tables d'évaluation ; `eleve_id` sans clé étrangère sur `ent_note` et `ent_note_appreciation` | l'isolation entre établissements ne repose que sur les filtres applicatifs | absence dans `100_rls_pilote.sql`, `206_fin_rls.sql` | V0 : RLS permissive puis stricte (V13) |
| D10 | **`absent_compte_zero`** : réglage qui transforme une absence en 0 | contraire à la règle nationale [V §5] | `180:45-46` | V0 : réglage neutralisé (lu comme faux), retiré de l'écran |
| D11 | **Mentions** « avertissement travail / conduite » listées par le conseil | confusion avec la sanction disciplinaire du chef d'établissement [V §5] | `071:34-39`, `evaluations_conseil.rs:30-37` | V0 : libellés « mise en garde » ; socle 23 : mentions = données de l'établissement |
| D12 | **Deux calculs** : f64 arrondi au centième en cours de calcul, sans coefficient de matière, sur les routes historiques | deux moyennes différentes pour la même période | `notes.rs:123`, `service.rs:7250-7290` | V0 : les lectures historiques appellent le moteur exact |
| D13 | **Code de période unique à vie** (`UNIQUE (etablissement_id, code)`) ; pas de contrôle de chevauchement | « T1 » ne peut pas être réutilisé l'année suivante | `022:13` | V1 (le socle 5 porte désormais les dates) |
| D14 | **Exports** : deux droits pour la même chose (`livrets.exporter` direction, `exports.officiels` administratif) ; XML d'allure officielle ; CSV sans protection contre l'injection de formule | contraire à D-022 ; faux sentiment de conformité ; risque tableur | `actions.rs:1060, 1078`, `evaluation.rs:489`, `lsun_export.rs` | V0 : un seul droit (administratifs et direction), mention « format de travail non officiel », échappement (socle 9 X0) |
| D15 | **Effectif d'un élève qui change de classe** : présent dans les deux classes ; relevé limité à la dernière, bulletins possibles des deux côtés sous la même clé [H non testé] | notes de l'ancienne classe disparues du relevé ; deux bulletins concurrents | `evaluations.rs:362-386, 526` | V5 : public au cours et à la date, bulletin de la classe de rattachement à l'arrêt |

### 2.3 Ce que Charlemagne et Pronote font, et qu'il ne faut pas perdre [H, `MATRICE_CHARLEMAGNE.md` §3.7]
Évaluations, barèmes, coefficients, saisie en grille (NOT-01) ; absent / dispensé / non noté (NOT-02) ; moyennes paramétrables, sous-matières, rangs (NOT-04) ; rattrapages, correction après clôture (NOT-05) ; statistiques (NOT-06) ; saisie mobile (NOT-07) ; appréciations (NOT-11) ; conseil et bulletins (NOT-12 à 15, socle 23) ; exports (NOT-09, 16). **Trous** du marché à combler : publication réclamée (G28), saisie peu ergonomique (G35), exports bloqués au dernier moment (G9), perte de notes à la bascule (G12).

## 3. Acteurs, besoins réels, irritants

Ce qui suit vient de l'expérience de Florian au secrétariat d'un lycée sous Charlemagne, des passes personas des socles 2 à 10, de `MATRICE_CHARLEMAGNE.md` §3.7 et §5 (G9, G28, G35) et d'une comparaison de mémoire avec Pronote, École Directe et les ENT usuels **[H, non vérifiée au détail]**.

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps aujourd'hui | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Professeur** | Crée ses évaluations, corrige, saisit, publie, rédige ses appréciations, prépare le conseil de ses groupes | Saisie en double (carnet papier ou tableur, puis logiciel) ; grille lente à la souris ; barème « sur 10 » saisi comme « sur 20 » ; recopier les absents de l'appel ; rédiger 140 appréciations en trois soirs ; ne pas savoir quels élèves sont réellement dans son groupe cette semaine ; une évaluation saisie dans la mauvaise période | Qu'une note parte aux familles **avant** qu'il ait rendu les copies ; se tromper de barème devant toute la classe ; qu'on lui reproche un zéro ; perdre une saisie | Saisir au clavier comme dans un tableur (flèches, Entrée, codes « abs », « disp », « nr »), coller une colonne depuis un tableur, voir les absents de l'appel pré-remplis, publier quand il veut, une aide qui propose une première version d'appréciation **à partir de ses propres notes**, qu'il corrige | plusieurs fois par semaine ; pic à l'arrêt des notes |
| **Professeur principal (PP)** | Suit la classe, rédige l'appréciation générale, prépare et anime le conseil, rencontre les familles | Relancer à la main les collègues qui n'ont pas saisi ; reconstituer un tableau Excel pour le conseil ; relire 30 bulletins à la recherche d'appréciations vides ou contradictoires | Arriver au conseil avec des trous ; oublier un élève en grande difficulté ; une famille qui conteste la mention | Un tableau de bord « ce qui manque » avec relance en un geste ; un dossier de conseil prêt, projetable ; une synthèse par élève qui l'aide à écrire l'appréciation générale | hebdomadaire ; pic avant chaque conseil |
| **Direction** (chef d'établissement, adjoint) | Fixe les règles d'évaluation (projet d'évaluation au lycée), arrête les notes, préside les conseils, valide les bulletins, décide du passage et de l'orientation, traite les recours | Courir après les saisies ; vérifier les bulletins un par un ; corriger après conseil = tout régénérer et renvoyer ; trancher des contestations sans historique | Un bulletin faux envoyé à 900 familles ; un recours perdu faute de trace ; une note modifiée après le conseil sans que personne ne le sache | État de saisie en temps réel par classe et par professeur ; un contrôle automatique avant publication ; une correction tracée qui produit une version 2 et prévient qui il faut | quotidienne en fin de période |
| **Secrétariat de scolarité** | Paramètre périodes, matières, coefficients ; génère, imprime, publie et envoie les bulletins ; exporte LSU, LSL, Parcoursup ; archive | Mise sous pli de centaines de bulletins ; double envoi aux parents séparés oublié ; exports refusés pour un INE ou un code matière manquant découvert la veille ; bulletins sous Excel figés depuis vingt ans (D-023) | Envoyer le bulletin au parent qui ne devait pas l'avoir ; rater l'échéance Parcoursup ; perdre les notes à la bascule d'année (G12) | Un modèle de bulletin propre choisi une fois ; l'envoi à chaque destinataire calculé ; un **état de préparation** des exports des semaines avant ; zéro ressaisie | pics à chaque fin de période, en mars (Parcoursup), en juin |
| **CPE / vie scolaire** | Rédige l'appréciation de vie scolaire ; apporte assiduité et comportement au conseil ; signale les élèves à suivre | Recopier absences et retards dans le dossier de conseil ; saisir son appréciation sans voir les résultats | Qu'un décrochage scolaire ne soit vu qu'au conseil | Assiduité et ponctualité de la période calculées et posées dans le dossier ; alerte précoce « résultats en chute + absences » | par période |
| **Élève (mineur)** | Consulte ses notes, ses moyennes, ses appréciations, son bulletin | Notification à chaque note, y compris en cours ; moyenne qu'il ne comprend pas | Que ses parents voient une note avant qu'il ait pu en parler ; une erreur de saisie non corrigée | Comprendre sa moyenne (« pourquoi 11,6 ? »), voir sa progression, savoir ce qu'il doit rattraper | quotidienne |
| **Élève majeur** | Idem, et devient destinataire principal ; choisit ce que ses parents continuent de recevoir (socle 4, Q-A7) | Les parents reçoivent tout sans qu'on lui ait demandé | Être traité en mineur ; ou couper ses parents sans le vouloir | Le choix clair, au moment de la majorité, révocable | ponctuelle |
| **Famille** (titulaire de l'autorité parentale) | Consulte notes, moyennes, bulletins ; signe la fiche de dialogue ; répond à la proposition d'orientation | Trop de notifications (G26) ; publication réclamée puis jamais faite (G28) ; moyenne et moyenne de classe incompréhensibles ; bulletin introuvable un an après | Découvrir une chute au bulletin, trop tard ; ne pas pouvoir contester | Un résumé sobre, des moyennes expliquées, le bulletin dans son coffre, une alerte quand un travail est à rattraper | hebdomadaire |
| **Parent séparé** | Reçoit son propre exemplaire du bulletin, voit les notes si son lien le permet | Devoir réclamer le bulletin ; ne recevoir que ce que l'autre parent veut bien transmettre | Être oublié ; que l'autre parent voie son adresse sur le bulletin | Son exemplaire nominatif, à son adresse, sans démarche | par période |
| **Parent sans autorité parentale** | Peut être destinataire des bulletins selon la décision de la direction (socle 4, C6) | Ne rien recevoir ou tout recevoir sans règle | Être exclu à tort ; ou recevoir alors qu'un jugement l'interdit | Une règle lisible, appliquée sans geste | rare |
| **AESH** | Accompagne un élève en évaluation (lecture des consignes, temps majoré, reformulation) | Ne pas savoir qu'une évaluation a lieu demain ni quels aménagements s'appliquent | Arriver sans être prévenue ; voir des données qui ne la regardent pas | Le calendrier des évaluations de l'élève accompagné et ses aménagements, rien de plus | hebdomadaire |
| **Remplaçant, suppléant, contractuel** | Reprend un cours en cours de période | Ne pas voir les évaluations déjà faites ; ne pas pouvoir saisir les appréciations ; perdre l'accès le dernier jour avant le conseil | Rédiger des appréciations sans connaître les élèves | Historique du cours dès sa prise de fonction, droits datés, accès en lecture prolongé jusqu'au conseil | par remplacement |
| **Délégués élèves et parents au conseil** | Assistent au conseil de classe | Rien d'outillé : ils écoutent | Être exposés à des informations sensibles sur des camarades | Une vue « séance » qui n'affiche que ce qui est dit, sans santé ni détail disciplinaire | par période |
| **Infirmière, médecin scolaire** | Établit les inaptitudes en EPS (avec le certificat) | Transmettre l'inaptitude au professeur d'EPS par papier | Que le motif médical circule | Une dispense datée transmise sans motif (socle 34) | ponctuelle |
| **Psychologue de l'Éducation nationale (PsyEN)** | Participe aux réflexions d'orientation | Pas d'accès structuré aux avis | Être tenu à l'écart ou trop exposé | Lecture des propositions d'orientation des élèves qu'il suit (socle 37) | par période |

**Ce qui fait perdre le plus de temps aujourd'hui, en résumé [H]** :
1. **La double saisie** (copie → carnet → logiciel ; tableur → logiciel) et la grille à la souris.
2. **La course aux saisies avant le conseil** (relances par courriel, tableaux Excel, « qui n'a pas mis ses appréciations ? »).
3. **Les bulletins papier** (impression, mise sous pli, double envoi aux parents séparés, réclamations) et des **modèles figés** depuis la livraison du logiciel (D-023).
4. **Les corrections après conseil** (régénérer, réimprimer, renvoyer, sans trace de ce qui a changé).
5. **Les exports officiels** bloqués au dernier moment (G9) et la **remontée Parcoursup** sous tension (mars).
6. **Les cas de parcours** gérés à la main : élève arrivé en cours de trimestre, changement de groupe, option abandonnée, dispense d'EPS, absences aux évaluations, rattrapages.
7. **Les moyennes incomprises** par les familles, source d'appels au secrétariat et au PP.

## 4. Frontières

### 4.1 Tableau

| Sujet | 21 | 22 | 23 | Autres |
|---|---|---|---|---|
| Évaluations, barèmes, coefficients, résultats, statuts, publication | **possède** | lit (une évaluation peut porter des compétences) | lit | 20 relie un travail à une évaluation ; 10 fournit la séance |
| Règles de calcul, moyennes de cours, de matière, générale, annuelle, statistiques | **possède** (moteur unique) | — | lit (instantané) | 38 (DNB, bac) lit les moyennes annuelles |
| Périodes : dates, périodisation, date d'arrêt, date de conseil | lit | lit | lit | **5 possède** (335) |
| État d'évaluation d'une période par classe (ouverte, arrêtée, close) | **possède** | lit | fait passer à `conseil_tenu` et `close` | 5 refuse la clôture d'année tant que les périodes ne sont pas closes |
| Instantané d'arrêt | **possède** | lit (positionnements) | lit, complète | — |
| Appréciations de cours, générale, de vie scolaire | **possède** (saisie, versions, publication avec le bulletin) | — | lit, imprime ; possède l'**appréciation du conseil** | 13 fournit les faits de discipline ; 12 l'assiduité |
| Référentiels de compétences, positionnements, bilans de fin de cycle, LSU | — | **possède** | lit | 9 transporte |
| Conseil de classe, décisions, mentions, avis, passage, redoublement | — | — | **possède** | 37 orientation, vœux ; 35 aménagements ; 6 participants |
| Bulletins (contenu, version, publication), LSL | fournit l'instantané | fournit les bilans | **possède** | 7 rend le PDF et le coffre ; 8 avise ; 9 transporte |
| Dispenses et inaptitudes (EPS…) | **lit** (statut `dispense` pré-rempli, **sans motif**) | — | imprime « dispensé » | 34 possède l'inaptitude médicale ; 35 la dispense d'enseignement |
| Absences à une évaluation | pré-remplit `absent` depuis l'appel, lit la justification | — | — | **12 possède** l'absence et sa justification |
| Aménagements d'évaluation (temps majoré, sujet adapté, lecteur) | marque le résultat « aménagé » (interne) ; montre à l'AESH | — | rubrique « accompagnements » au bilan collège [V §5] | **35 possède** PAP, PPS, PPRE, PAI |
| Évaluations communes, examens blancs | **possède** comme évaluations (multi-cours) | — | — | **38 possède** l'organisation (salles, surveillances, convocations, sujets) |
| Notes venues d'ailleurs (établissement d'origine, CNED, échange à l'étranger) | **possède** (résultats externes sourcés) | — | — | 9 (lecture de fichiers), 3 (dossier d'admission) |
| Exports LSU, LSL, Parcoursup, Affelnet, DNB/Cyclades | fournit les données | **produit** LSU | **produit** LSL | 9 moteur et traces ; 37 Parcoursup, Affelnet ; 38 DNB |

### 4.2 Ports exposés par le socle 21 [R]
- `evaluations::moyennes(tx, eleve, periode | annee, mode) -> Moyennes` (avec explication) — 22, 23, 37, 38, 47, IA.
- `evaluations::instantane(tx, classe, periode) -> Instantane` (version, empreinte) — 23.
- `evaluations::etat_periode(tx, classe, periode) -> EtatPeriode` — 5 (clôture d'année), 23, 20 (le cahier signale « période arrêtée »).
- `evaluations::calendrier(tx, eleve | cours, du, au) -> [EvaluationPrevue]` (sans note) — 20 (charge de travail), 35 (AESH), 10, 11 (mobile).
- `evaluations::etat_saisie(tx, classe | prof, periode) -> EtatSaisie` — 23 (préparation du conseil), 47 (pilotage).
- `evaluations::moyennes_annuelles(tx, classe, annee, enseignements) -> [MoyenneAnnuelle]` — 22 (LSU, Affelnet), 23 (LSL), 38 (DNB).

### 4.3 Ports consommés [R]
- **5** : `structure::periode(date, classe)`, `periodes(annee, classe)` ; `ent_public_cours_jour(cours, date)` ; offre et coefficients de maquette ; matières et sous-matières (338).
- **6** : `services::intervenants(cours, date)` (titulaire, suppléant, co-enseignant), `missions::pp(classe, date)`.
- **12** : `absences::a_la_seance(eleve, seance) -> {absent, justification}`.
- **34 / 35** : `dispenses::actives(eleve, matiere, date) -> {dispense, partielle}` **sans motif** ; `amenagements::evaluation(eleve, date) -> [Amenagement]` (libellés fonctionnels : « temps majoré d'un tiers », jamais le diagnostic).
- **4** : `liens::droits(eleve, personne) -> {voir_notes, recevoir_bulletins}` ; capacité juridique (majeur, émancipé).
- **8** : avis (`evaluations.*`), heures calmes, résumé ; **7** : rendu et coffre ; **9** : moteur d'exports.

### 4.4 Points à accorder avec les socles voisins [R]
1. **Socle 5** : la date d'arrêt et la date de conseil sont **par période** dans 335 ; le terrain les veut souvent **par classe** (conseils étalés sur deux semaines). Proposition : l'état du socle 21 porte une date d'arrêt **effective par classe**, par défaut celle de la période (CE-41). À confirmer avec le socle 5 (Q-EV12).
2. **Socle 12** : l'absence à la séance de l'évaluation est connue **après** l'appel ; une évaluation hors séance (DM, oral sur rendez-vous) n'a pas d'appel : `absent` y est saisi à la main.
3. **Socle 20** : « ce travail sera noté » crée l'évaluation **en brouillon** ; supprimer le travail ne supprime jamais une évaluation qui a des résultats.
4. **Socle 38** : un examen blanc crée **une évaluation par cours concerné** (même sujet, même date, barème commun) reliée par une **évaluation commune** ; l'organisation (salles, tiers-temps, surveillances) est au 38.

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-09)

Recherche par sous-agent sur Légifrance et eduscol ; les pages HTML d'education.gouv.fr, d'eduscol et de parcoursup.gouv.fr étaient refusées aux outils (HTTP 403), d'où des [NV]. **Le mémento eduscol de décembre 2025 a été lu directement** (PDF extrait en texte).

| # | Règle | Source | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| R-01 | **« La note zéro ne peut pas être utilisée pour sanctionner l'absence non justifiée à une évaluation disciplinaire ordinaire »** ; en cas de moyenne non représentative, **évaluation de remplacement** unique de fin d'année organisée par le chef d'établissement ; « seule l'absence non justifiée à cette évaluation de remplacement permet l'attribution de la note zéro » ; absence justifiée → nouvelle convocation | Mémento eduscol « Évaluer les élèves de 3ᵉ… DNB », décembre 2025, p. 13 et FAQ « rattrapage et remplacement » — https://eduscol.education.gouv.fr/sites/default/files/document/memento-evaluer-les-eleves-de-3e-dans-le-cadre-des-nouvelles-modalites-d-attribution-du-diplome-national-du-brevet-123226.pdf | **[V]** (lu directement) ; la même règle est posée pour le lycée par le guide eduscol de l'évaluation au lycée GT (2023) [V par sous-agent] | `absent_compte_zero` retiré (D10) ; statuts `absent`, `a_rattraper` ; **évaluation de remplacement** comme nature d'évaluation (EVA-172) ; 0 possible seulement sur une évaluation de remplacement et après absence non justifiée |
| R-02 | « Le zéro est une note d'évaluation. Il ne peut pas être utilisé avec une visée de sanction d'ordre disciplinaire. Les éditeurs de note proposent une ou des modalités permettant d'indiquer que l'élève n'a pas été évalué, sans avoir à recourir à la note 0 » ; absence injustifiée récurrente : sanction possible (discipline), et « la moyenne périodique est en attente » (LSU : EA) | idem, FAQ | **[V]** | statut `en_attente` de **moyenne de période** (distinct de la case vide) ; lien vers le socle 13 pour la suite disciplinaire, jamais par la note |
| R-03 | Principes « **à respecter dans les logiciels de suivi des évaluations** » (3ᵉ) : 2 ou 3 bilans périodiques par an ; **note chiffrée sur 20 requise** pour chaque enseignement obligatoire suivi sur chaque bilan ; « en attente » possible sur un ou plusieurs bilans ; **moyenne annuelle chiffrée obligatoire** en fin d'année ; dispense réglementaire → aucune moyenne attendue | idem, « La construction et le parcours des notes » | **[V]** | contrôles bloquants à l'arrêt de la dernière période de 3ᵉ (R-V22) ; échelle sur 20 imposée en 3ᵉ quel que soit le réglage (S-101 ne s'applique pas en 3ᵉ) |
| R-04 | LSU 3ᵉ : « dispensé » seulement pour EPS et LV2, jamais LV1 ; moyennes annuelles transmises à Cyclades et Affelnet (jusqu'à 9 décimales) ; deux décimales dans LSU | idem, et `VERIF_09_SOURCES.md` n° 20, 25 | [V] (mémento, via sous-agent) | précision d'export distincte de l'affichage (§8.6) ; contrôle « dispensé interdit en LV1 » au pré-contrôle LSU (socle 22) |
| R-05 | DNB à partir de 2026 : contrôle continu = moyenne des **moyennes annuelles** des enseignements obligatoires de 3ᵉ ; 40 % / 60 % ; points au-dessus de 10 d'un enseignement facultatif ajoutés ; commission académique d'harmonisation | Décret 2025-328, arrêté du 10/04/2025 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051451798 ; https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051451951 ; mémento p. 3 | [V] | moyenne annuelle (§8.5) méthode **figée** et exportable ; les « points au-dessus de 10 » sont un calcul du **socle 38**, pas une variante de moyenne du 21 (S-104) |
| R-06 | Bilan périodique du cycle 4 : par enseignement, éléments travaillés, acquis, progrès et difficultés ; **en 5ᵉ et 4ᵉ : la note ou un autre positionnement ; en 3ᵉ : la note** ; EPI, parcours, **accompagnements spécifiques** (PAP, PAI, PPRE, PPS, ULIS, UPE2A, SEGPA) ; vie scolaire (assiduité, ponctualité, demi-journées justifiées et non justifiées). Bilan de fin de cycle : 8 composantes, visé par PP, chef d'établissement et parents | Arrêté du 31/12/2015 modifié (annexes II et III modifiées par arrêté du 10/04/2025, rentrée 2025) — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000031742317 | [V] (sous-agent) | modèle de bulletin « collège » calé sur le bilan périodique (socle 23, D-023) ; collège en 6ᵉ–4ᵉ peut fonctionner **sans note** (positionnement, socle 22) : le 21 ne l'impose pas ; rubrique « accompagnements » sur le bulletin **remis à la famille** uniquement (jamais sur un document à un tiers, D-027 B) |
| R-07 | Conseil de classe (public) : présidé par le chef d'établissement ; enseignants, **2 délégués des parents, 2 des élèves**, CPE, PsyEN ; médecin, assistant social, infirmier quand ils ont connu le cas ; se réunit **au moins trois fois par an** (deux en lycée professionnel possible) ; examine les résultats ; en terminale, se prononce sur les vœux | C. éduc. R. 421-50 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000029625871 ; R. 421-51 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000036627857 | [V] (sous-agent) ; privé sous contrat : [NV] | socle 23 : participants, délégués (vue « séance » minimisée, §14.4) ; nombre de périodes ≥ 2 |
| R-08 | Information des parents : « tenus régulièrement informés » des acquis par le livret scolaire (collège) et bulletin et livret (lycée), à la périodicité fixée par le conseil d'administration ; l'établissement prend les mesures pour que les parents en prennent connaissance | C. éduc. D. 111-3 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006525717 | [V] (sous-agent) | accusé de première ouverture du bulletin (socle 7) ; liste « à prévenir autrement » (socle 8) |
| R-09 | Parents séparés : mêmes documents et bulletins aux deux parents, aux deux adresses | Circulaire 2006-137 du 25/08/2006 — https://www.education.gouv.fr/bo/2006/31/MENE0602215C.htm | **[NV]** (page refusée ; extrait) | exemplaire **nominatif** par lien `recevoir_bulletins` (socle 7, déjà conçu) |
| R-10 | Élève majeur : accès des parents aux résultats | aucun texte trouvé | **[NV]** | reste au socle 4 (Q-A7) : choix de l'élève, défaut « maintien jusqu'à sa réponse » |
| R-11 | **Avertissement** = sanction disciplinaire (R. 511-13), prononcée par le chef d'établissement (ou le conseil de discipline), effacée à la fin de l'année ; version en vigueur du 19/07/2026 (décret 2026-637) | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000054446411 | [V] (sous-agent) | le conseil de classe **ne prononce aucune sanction** ; mentions du conseil = encouragements, compliments, félicitations, **mises en garde** (libellés de l'établissement) ; un avertissement vient du socle 13 |
| R-12 | Félicitations, compliments, encouragements : aucune base réglementaire | — | [NV] (rien trouvé) | **données de l'établissement** (D-022 : liberté des données), préréglage de 3 + 2 mises en garde |
| R-13 | Orientation : demande de la famille (ou de l'élève majeur), proposition du conseil, décision du chef d'établissement, entretien en cas de désaccord, **3 jours ouvrables** pour faire appel, commission d'appel ; maintien possible un an ; procédure miroir dans le privé sous contrat avec commission propre | C. éduc. D. 331-23 à D. 331-61 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166846/ | [V] (sous-agent) | socle 23 et 37 : circuit tracé, délai calculé en jours ouvrables de l'établissement (socle 5) |
| R-14 | **Redoublement** : décidé par le chef d'établissement en fin d'année, après dialogue avec la famille et **avis du conseil de classe**, si l'accompagnement a échoué ; notifié avec voies d'appel ; un seul au collège (un second après interruption, accord du DASEN) ; accompagné d'un dispositif (PPRE possible) | C. éduc. D. 331-62, version du 18/03/2024 (décret 2024-228) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000036626978 | [V] (sous-agent) | socle 23 : décision « redoublement » de la direction, jamais du conseil ; compteur de redoublements au collège (socle 2) |
| R-15 | Lycée : contrôle continu = 40 % du baccalauréat, fondé sur les **moyennes annuelles** du livret ; **projet d'évaluation** de l'établissement (équipe, conseil pédagogique, conseil d'administration, communiqué aux élèves et familles) ; moyenne non représentative → évaluation de remplacement | Guide eduscol de l'évaluation au lycée GT (nov. 2023) — https://eduscol.education.gouv.fr/sites/default/files/document/guide-l-evaluation-des-apprentissages-et-des-acquis-au-lycee-gt-69588.pdf ; note de service du 25/08/2025 (MENE2523744N) [NV, BO refusé] | [V] guide (sous-agent) ; note de service [NV] | réglage « projet d'évaluation » = document publié aux familles (socle 7) + règles du §8 ; coefficients consultables par élèves et familles |
| R-16 | LSL : moyennes périodiques et annuelles, appréciation annuelle par enseignement, compétences, engagements, **avis du conseil pour l'examen** ; importé « depuis les éditeurs de notes » ; transmis à Cyclades ; public et privé sous contrat | FAQ familles LSL — https://eduscol.education.gouv.fr/sites/default/files/document/lsl-faq-famillepdf-104442.pdf ; `VERIF_09_SOURCES.md` n° 23 | [V] (sous-agent) ; format XML [NV] | socle 23 ; appréciation **annuelle** distincte des appréciations de période (§12) |
| R-17 | Parcoursup : bulletins de 1ʳᵉ et de terminale remontés par l'éditeur de notes ; à défaut, l'élève saisit ; fiche Avenir | `VERIF_09_SOURCES.md` n° 24 | [V] partiel | socle 37 ; reprise de N-1 obligatoire pour un lycée qui arrive (FX-106) |
| R-18 | Dispense d'EPS : possible pour inaptitude totale reconnue par un médecin ; dans le DNB, un enseignement dispensé n'entre pas dans le contrôle continu | mémento (« EPS : dispense possible… ») [V] ; décret 88-977 (version lue abrogée en 2006, base codifiée actuelle [NV]) | [V] / [NV] | statut `dispense` pré-rempli depuis le socle 34 **sans motif** ; période entière dispensée → « dispensé » au bulletin, pas de moyenne |
| R-19 | Élèves en PPS, PAP, PPRE : aménagements pris en compte dans le contrôle continu ; dispense d'un enseignement par le recteur (PPS) → évaluation neutralisée | mémento [V] | [V] | marque « aménagé » sur le résultat (interne) ; dispense d'enseignement du socle 35 → `dispense` |
| R-20 | Conservation : dossier scolaire DUA 50 ans (ou 10 ans) ; LSUN : base active le cycle + 1 an ; LSL : effacement au plus tard 8 mois après les résultats | Instruction archives du 16/06/2005 [NV] ; avis CNIL 2017-159 sur LSUN [V pour le cycle + 1 an] ; LSL [NV] | partiel | durées tenues au socle 7 ; les **évaluations détaillées** (notes d'une copie) ont une durée plus courte que le **bulletin** (Q-EV14) |
| R-21 | LSUN (avis CNIL) : accès restreint aux besoins des fonctions, journalisation, prise de connaissance parentale | Délibération CNIL 2017-159 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000035944649 | [V] (sous-agent) | journal des consultations de bulletins et de dossiers de conseil (§14.5) |
| R-22 | Règlement européen sur l'IA : systèmes destinés à **évaluer les acquis d'apprentissage**, y compris pour orienter l'apprentissage, = **haut risque** (annexe III, point 3 b) ; obligations reportées (accord du 7 mai 2026 : 2 décembre 2027 ; règlement 2026/1744 du 24/07/2026) | https://artificialintelligenceact.eu/annex/3/ (non officiel) ; EUR-Lex non lisible | **[NV]** | §13 : l'IA ne note pas, ne décide pas ; aide à la rédaction classée « tâche préparatoire » à documenter (art. 6.3) ; étude juridique avant mise en service (Q-EV8) |
| R-23 | RGPD art. 22 (décision fondée exclusivement sur un traitement automatisé) | non lu | [NV] | aucune décision automatique (R-V20) |
| R-24 | Contestation d'une note : aucune règle trouvée (mesure d'ordre intérieur [NV]) ; les décisions d'orientation et de redoublement ont, elles, des voies d'appel [V] | — | [NV] | demande de révision d'une note = demande tracée au professeur (EVA-166), pas un recours juridique |
| R-25 | **[ajout personas]** « Le parent qui n'a pas l'exercice de l'autorité parentale conserve le droit et le devoir de surveiller l'entretien et l'éducation de l'enfant. Il doit être informé des choix importants relatifs à la vie de ce dernier. » | C. civ. 373-2-1, al. 5 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000022469781 | **[V]** (sous-agent, 2026-11-09) | R-V43 : bulletins et décisions d'orientation **par défaut** au parent sans exercice ; notes au fil de l'eau non par défaut |
| R-26 | **[ajout personas]** Retrait total de l'autorité parentale par la juridiction pénale (crime ou agression sexuelle incestueuse sur l'enfant, crime sur l'autre parent) ; l'enfant peut être confié à un tiers ou à l'ASE | C. civ. 378 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049294106 ; 380 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049294114 | [V] | aucun document ni avis (socle 4) ; le guichet le voit (R-V39) ; le tiers ou le service gardien reçoit ce qu'aurait reçu le parent |
| R-27 | **[ajout personas]** « Lorsque deux adresses sont indiquées, les informations communiquées par courrier le sont aux deux adresses » ; le tiers à qui l'autorité est confiée a les mêmes droits | Circulaire 2006-137, § I.1 — BO n° 31 du 31/08/2006 (page officielle refusée, texte du BO lu sur une copie : https://33.snuipp.fr/IMG/pdf/Role_parents_circulaire_2006-137-1.pdf) | **[V*]** (texte officiel lu par un relais) ; complète R-09 | exemplaire nominatif à chaque adresse (inchangé) ; le lot d'impression regroupe par adresse, jamais deux parents séparés dans une enveloppe (EVA-248) |
| R-28 | **[ajout personas]** Majorité à 18 ans accomplis (C. civ. 414) ; orientation : « les parents ou l'élève majeur » formulent les demandes (D. 331-32) ; **aucun texte trouvé** sur l'information des parents d'un élève majeur | C. civ. 414 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006427400/ ; D. 331-32 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000045047667 | [V] pour 414 et D. 331-32 ; [NV] pour l'information des parents (confirme R-10) | inchangé (socle 4, Q-A7) ; destinataires calculés à la date de publication (R-V47) |
| R-29 | **[ajout personas]** Baccalauréat général : « un coefficient 40 est affecté à l'évaluation chiffrée annuelle » ; un candidat qui « ne dispose pas d'une évaluation chiffrée annuelle » se voit proposer une **évaluation de remplacement** ; l'absence injustifiée à celle-ci est « sanctionnée par la note zéro » | Arrêté du 16/07/2018 modifié, art. 2 — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000037202834 ; arrêté sur les modalités du contrôle continu, art. 12 — https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000043871640 | [V] (sous-agent) | R-V45 : contrôle des moyennes annuelles des enseignements du baccalauréat avant l'arrêt de la dernière période de 1ʳᵉ et de terminale ; EVA-172 vaut aussi au lycée |
| R-30 | **[ajout personas]** « Les évaluations non rendues par un élève doivent être signalées comme telles dans le bulletin » (lycée GT) ; le mémento 3ᵉ recommande de ne pas noter les devoirs à la maison quand l'usage de l'IA par l'élève est incontrôlable | Guide eduscol de l'évaluation au lycée GT — URL de R-15 ; mémento 3ᵉ — URL de R-01 | [V] (sous-agent) | Q-EV1 confirmée (non rendu non compté) **et** nombre de non-rendus de la période **imprimé** au bulletin par matière (instantané, socle 23) ; la nature « DM » porte un rappel au professeur (§8.9) |
| R-31 | **[ajout personas]** Fiche Avenir : appréciation du professeur principal (méthode, autonomie, engagement), avis du chef d'établissement sur chaque vœu, appréciations disciplinaires « récupérées automatiquement par Parcoursup via le lycée » | https://www.parcoursup.gouv.fr/sites/default/files/database/documents/2026-03/la-fiche-avenir-dans-la-proc-dure-parcoursup-1146.pdf | [V] ; page « remontée des notes » [NV] | socle 37 : fiche Avenir depuis les appréciations du 21 ; **jamais** la rubrique « accompagnements » (R-V37) |
| R-32 | **[ajout personas]** Livret scolaire du lycée : avis de l'équipe pédagogique pour l'examen « très favorable », « favorable » ou « doit faire ses preuves » ; engagements | Arrêté du 4 mars 2020 modifié — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000041814643 ; libellés lus sur les modèles STAV et bac pro d'eduscol | [V] pour la référence, [V*] pour les libellés ; annexes GT et version consolidée **[NV]** | socle 23 : trois libellés fixes (non paramétrables) ; explication à l'élève (M-637) |
| R-33 | **[ajout personas]** Règlement IA, annexe III point 3 : (a) admission, (b) **évaluer les acquis d'apprentissage**, (c) niveau d'éducation approprié, (d) **surveiller et détecter les comportements interdits des élèves pendant les tests** ; art. 6 § 3 : pas à haut risque pour une « tâche préparatoire à une évaluation », **mais toujours à haut risque en cas de profilage** ; obligations « haut risque » à partir du 2 décembre 2027 (page de la Commission) | https://ai-act-service-desk.ec.europa.eu/en/ai-act/annex-3 ; https://ai-act-service-desk.ec.europa.eu/en/ai-act/article-6 ; https://digital-strategy.ec.europa.eu/en/policies/regulatory-framework-ai | [V*] (version consolidée publiée par la Commission ; JOUE et art. 113 modifié [NV]) | R-V40 : ni correction de copies, ni détection de fraude, ni formation de groupes d'élèves par l'IA ; les alertes du §13.3 restent des **règles fixées par des personnes** (pas un système d'IA), mais elles évaluent des personnes : AIPD et contrôle RGPD art. 22 (§13.8) |
| R-34 | **[ajout personas]** Le conseil de classe « est chargé du suivi des élèves », se réunit au moins trois fois par an, donne en terminale son avis sur les vœux ; le conseil examine les demandes et **émet des propositions** d'orientation, « l'avis de l'élève mineur est recueilli » | R. 421-51 (URL de R-07) ; D. 331-32 (URL de R-28) | [V] ; présidence par un **représentant** du chef d'établissement, **discrétion** des délégués (D. 111-11 cité ailleurs) et privé sous contrat **[NV]** | délégation de présidence (EVA-256) avec mention [NV] à lever ; rappel de discrétion à l'ouverture de la vue séance (§14.6) |
| R-35 | **[ajout personas]** Archives des EPLE : « dossiers scolaires » (dont les bulletins trimestriels ; le livret y est classé) : **durée d'utilité administrative 10 ou 50 ans**, puis tri ; registres d'appel : 10 ans | Instruction DPACI/RES/2005/003, BO n° 24 du 16/06/2005, annexe — https://www.education.gouv.fr/bo/BoAnnexes/2005/24/24.pdf | [V] ; procès-verbaux de conseils de classe et registres de notes **[NV]** | registre annuel des bulletins (EVA-254) gardé **50 ans par défaut** (10 ans si l'établissement tient un registre matricule complet), au socle 7 ; complète R-20 et Q-EV14 |
| R-36 | **[ajout personas]** Prénom d'usage d'un élève (identité de genre) dans les documents internes, avec l'accord des représentants légaux pour un mineur ; état civil dans les documents officiels | Circulaire du 29/09/2021 « Pour une meilleure prise en compte des questions relatives à l'identité de genre en milieu scolaire » [NV : non relue ici] | **[NV]** | R-V36 : prénom d'usage sur bulletin, relevé, appréciations, IA ; état civil sur LSU, LSL, Parcoursup, documents à un tiers ; **à relire avant V15** |
| R-37 | **[ajout personas]** Droit d'accès (RGPD art. 15) ; CNIL : « l'accès aux notes, qui sont des données personnelles, est en effet strictement personnel » ; l'ENT garantit que chacun n'accède qu'aux informations le concernant | https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre3 ; guide CNIL de l'enseignement — https://linc.cnil.fr/sites/default/files/typo/document/CNIL_Guide_enseignement.pdf | [V*] (copie CNIL du RGPD ; guide ancien) | EVA-273 (export de toutes les données d'évaluation d'un élève pour le DPO) ; R-V41 (la note privée est communicable) |
| R-38 | **[ajout personas]** Bilan périodique du cycle 4 (annexe III de l'arrêté du 31/12/2015, à partir de la rentrée 2025) : en 5ᵉ et 4ᵉ « la note de l'élève ou tout autre positionnement », en 3ᵉ « la note de l'élève » ; vie scolaire : assiduité, ponctualité, **demi-journées justifiées ou non** ; accompagnements PAP, PAI, PPRE, PPS, ULIS, UPE2A, SEGPA. **L'annexe III ne fixe pas d'échelle à quatre niveaux** | https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000032615791 | [V] (sous-agent) | confirme R-06 ; l'échelle à quatre niveaux relève du **bilan de fin de cycle** (socle 22), pas du bilan périodique : C13 (§27) |
| R-39 | **[ajout personas]** CNED : enseignement pour les élèves « ne pouvant être scolarisés totalement ou partiellement » en établissement ; inscription décidée par le directeur général du CNED (avis du DASEN pour l'instruction obligatoire) | R. 426-2 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000020320175 ; R. 426-2-1 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000025164621/ | [V] partiel ; **qui évalue et qui fait le bulletin en scolarité partagée : [NV]** | R-V38 : moyenne externe transcrite, décision de la direction de la compter, source imprimée ; **ne rien coder qui suppose une règle nationale** sur ce point |

**Non vérifié et interdit de coder dessus** : XSD LSU 2ᵈ degré et LSL (espace éditeurs Pléiade, D-2 du socle 9) ; composition du conseil de classe dans le privé sous contrat ; durée de conservation des notes détaillées ; règle de l'élève majeur ; texte consolidé et calendrier du règlement IA.

---

## 6. Ce que les socles 22 et 23 attendent du 21

Le socle 21 est conçu pour que 22 et 23 n'aient **rien à recalculer** ni à redemander.

### 6.1 Socle 22 (compétences et livrets) [R]
- **Une évaluation peut porter des compétences** (items d'un référentiel du socle 22) en plus ou à la place d'une note ; le résultat porte alors un **positionnement** par item (échelle à 4 niveaux [V arrêté 2015, via sous-agent ; D. 122-3 NV]). Le 21 stocke le lien évaluation ↔ items ; le 22 possède les items et les positionnements.
- **6ᵉ, 5ᵉ, 4ᵉ sans note** : l'arrêté admet « la note ou un autre positionnement » [V]. Une classe peut donc avoir des évaluations **sans barème** (positionnement seul) : le 21 l'accepte (évaluation de nature « positionnement », aucune moyenne chiffrée) ; le bilan périodique lit alors les positionnements (22).
- **Fusion des deux modèles** existants (`ent_competence_*` et `ent_bilan_cycle`) : au socle 22 ; le lot V0 se borne à fermer la fuite (compétences visibles avant publication, D3 b).
- LSU : produit par le 22 sur le moteur du socle 9, avec les moyennes du 21 (port `moyennes_annuelles`) ; « en attente » (EA) et « dispensé » (EPS, LV2) viennent des statuts du 21.

### 6.2 Socle 23 (bulletins, conseils, passage) [R]
- **L'instantané d'arrêt** (§9.1) est l'entrée unique du conseil et du bulletin : il porte, par élève, moyennes de cours et de matière, générale, statistiques, statuts (« dispensé », « en attente »), nombre d'évaluations, appréciations (avec leur version), assiduité de la période (socle 12) et faits de discipline autorisés (socle 13).
- **Dossier de conseil préparé en un clic** (A7) à partir de l'instantané et de l'état de saisie ; **fenêtre du conseil** pour les droits des professeurs (§14.3).
- **Décisions historisées** (chaque changement est une version, avec auteur et motif), verrou à la tenue, réouverture motivée.
- **Mentions** = liste de l'établissement (préréglage : félicitations, compliments, encouragements, mise en garde travail, mise en garde comportement) — **jamais** « avertissement » [V R-11].
- **Bulletin** : 3 ou 4 modèles prêts (D-023) : *collège (bilan périodique)*, *lycée général et technologique*, *lycée professionnel*, *sans note (positionnements)* ; rendu par le moteur de documents du socle 7 (Typst, PDF/A, accents justes), exemplaires nominatifs, coffre, courrier pour qui n'a pas de compte.
- **Passage, redoublement, orientation** : décisions de la **direction** après avis du conseil (R-13, R-14), délais d'appel calculés, avec le socle 37.
- **Appréciation annuelle** (LSL) et **avis pour l'examen** (LSL) : au socle 23 ; le 21 fournit les moyennes annuelles.
- **Fiches de dialogue** (T-11, en production sous Next.js chez Florian : `rapport-fiches-dialogue-personas-v2.md`) : au socle 23 avec le 19 ; à ne pas perdre.

### 6.3 [ajout personas] Attentes nouvelles posées aux socles 23, 37 et 19 (`PERSONAS_21.md`)
- **Conseil** : présidence **déléguée** pour un conseil ou une série (EVA-256, R-07) ; **procès-verbal** et liste des présents archivés (EVA-258) ; **synthèse anonyme** de la classe validée par le président pour le compte rendu des délégués des parents (EVA-257) ; geste « délégués sortis » tracé (CE-87) ; **plusieurs mentions** pour un élève ; **note au président** d'un professeur absent pour ses élèves (M-592) ; dossier « ce qui a changé depuis le dernier conseil » (§13.4).
- **Bulletins** : **publication programmée** par classe ou niveau, « remis en main propre le …, en ligne le … » (EVA-247) ; **contrôle avant publication** (EVA-246) ; **lot d'impression** par adresse (EVA-248) ; renvoi papier de la seule v2 (EVA-249) ; **duplicata, archives, registre annuel** (EVA-251 à 254) ; **moyenne du groupe** pour un enseignement multi-classes (R-V30) ; **assiduité bornée** (R-V31) ; **aucune sanction** (R-V32) ; **aucune rétention** pour impayé (R-V33) ; **prénom d'usage** (R-V36) ; statut scolaire ou apprenti ; **diplômes de mentions** (trames prêtes, D-023) ; appréciation du **coordonnateur du dispositif** (ULIS, §12.3) ; glossaire des mentions et statuts accessible depuis le bulletin (M-640) ; destinataires calculés **à la date de publication** (R-V47).
- **Livret et examen** : explication de l'« avis pour l'examen » du LSL à l'élève, demande d'explication tracée (M-637).
- **Socle 37 (Parcoursup)** : fiche Avenir alimentée par les appréciations annuelles ; assemblage des appréciations de sous-matières ; **jamais** la rubrique « accompagnements » (R-V37).
- **Socle 19 (rendez-vous)** : proposer des créneaux à une famille depuis la fiche du conseil (M-600).

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Sens dans ScolIA | Ce que ce n'est pas |
|---|---|---|
| **Évaluation** | Un moment où un travail d'élèves est apprécié par une note (ou un statut), pour un **cours** (socle 5), à une **date**, dans une **période** d'évaluation, avec un **barème** et un **coefficient**. Remplace le « devoir » (`ent_devoir`). | Un travail à faire (socle 20) — un travail peut *donner lieu* à une évaluation, ils restent deux objets reliés |
| **Barème** | Le maximum de l'évaluation : 20, 10, 5, 40, 100, 7,5… (décimal > 0). La note est saisie **sur le barème**. | Une échelle d'affichage |
| **Échelle d'affichage** | L'échelle dans laquelle on montre moyennes et notes ramenées : **sur 20 par défaut** ; un établissement peut choisir « sur 10 » ou « sur 100 » pour ses bulletins (réglage d'établissement, S-101) | Le barème d'une évaluation |
| **Résultat** | Ce qu'un élève obtient à une évaluation : une **note** ou un **statut** (§8.2) | Une moyenne |
| **Coefficient d'évaluation** | Poids d'une évaluation dans la moyenne de son cours (décimal > 0, défaut 1) | Le coefficient de matière |
| **Coefficient de matière** | Poids d'une matière dans la moyenne générale, fixé par cible (classe > MEF > niveau > défaut), par année | Un poids propre à un élève |
| **Moyenne de cours** | Moyenne d'un élève dans un cours pour une période | La moyenne de matière (un élève peut avoir suivi deux cours de la même matière dans la période) |
| **Moyenne de matière** | Moyenne d'un élève dans une matière (et ses sous-matières) pour une période, tous cours confondus | — |
| **Moyenne générale** | Moyenne pondérée des moyennes de matière ; **indicative**, jamais une décision | Un résultat d'examen |
| **Moyenne annuelle** | Moyenne d'une matière sur l'année (méthode fixée par l'établissement, §8.5) ; c'est elle que lisent LSL, DNB et Affelnet (socle 9 FX-34, FX-36 ; DNB : VERIF_09 n° 25) | — |
| **Période d'évaluation** | Une période de la périodisation d'évaluation de la classe (socle 5, migration 335 : trimestres, semestres, périodes libres) | Une table propre aux notes : `ent_note_periode` devient un **état** de la période (§9) |
| **Arrêt des notes** | Date à partir de laquelle on ne saisit plus de note pour la période (sauf rattrapage autorisé) ; porté par `ent_periode.date_arret_notes` (socle 5) ; **l'état** « notes arrêtées » est au socle 21 | La clôture de la période |
| **Publication** | Le geste (ou la date programmée) à partir duquel famille et élève voient les résultats d'une évaluation | La saisie |
| **Appréciation** | Texte rédigé par une personne sur un élève pour une période : de **cours** (professeur), **générale** (PP ou direction), de **vie scolaire** (CPE), **du conseil** (président) | Une observation de vie scolaire au fil de l'eau (socle 13) |
| **Instantané d'arrêt** | Copie figée, à l'arrêt des notes, des résultats, moyennes et appréciations d'une classe pour une période : ce que le conseil examine et ce que le bulletin imprime | Une moyenne stockée « pour aller vite » |
| **Bulletin** | Document officiel figé, versionné, d'un élève pour une période (socle 23) | Le relevé de notes (lecture vivante) |
| **Relevé de notes** | Vue vivante des résultats publiés et des moyennes provisoires | Un document officiel |

### 7.2 Quatre distinctions qui manquent aujourd'hui [F → R]

1. **Saisir ≠ publier ≠ arrêter ≠ figer.** Aujourd'hui une note historique est publique dès la saisie, la grille publie par devoir, le bulletin compte même ce qui n'est pas publié, et rien n'est figé avant le bulletin. Cible : saisie (privée au professeur et à la direction), publication (famille, élève), arrêt (plus de saisie, instantané), bulletin (document).
2. **Cours ≠ matière ≠ classe.** Une note appartient à un cours (le groupe d'allemand de 4B-4C), la moyenne de matière agrège les cours suivis, le bulletin est celui de la **classe de l'élève** (même si l'élève est noté dans un groupe à cheval sur deux classes).
3. **Absent ≠ zéro ≠ dispensé ≠ non concerné.** Un élève arrivé le 4 novembre n'est pas « absent » aux évaluations d'octobre : il n'était **pas concerné**. Un absent n'est pas un zéro (§5, R-V5). Un dispensé n'a pas à rattraper.
4. **Modifier avant l'arrêt ≠ corriger après.** Avant l'arrêt, le professeur modifie librement mais **tout est historisé** (aujourd'hui : écrasement et suppression physique, D6–D7). Après l'arrêt, c'est une **correction** motivée, par la direction, qui produit une nouvelle version d'instantané et, si besoin, de bulletin.

---

## 8. Les règles de calcul (moteur unique, explicable)

Principe : **un seul moteur de calcul**, celui de `domain-eleve/src/evaluation.rs` [F : fractions exactes en entiers, aucun arrondi intermédiaire, testé], **étendu** et appelé par toutes les lectures (grille, relevé famille, vue de classe, conseil, bulletin, exports, IA). La voie historique en f64 (`notes.rs:123`, `service.rs:7250-7290`) disparaît (lot V0/V1). Chaque moyenne produite peut être **expliquée** : le moteur rend, à côté du nombre, la liste des termes (évaluation, note ramenée, coefficient, compté ou non et pourquoi).

### 8.1 Valeurs [R]
- **Stockage décimal exact** : `NUMERIC(7,3)` pour les notes et barèmes, `NUMERIC(6,3)` pour les coefficients ; contraintes en base `0 ≤ note ≤ barème` (sauf évaluation marquée « points au-delà du barème autorisés », S-104), `barème > 0`, `coef > 0`. Le `DOUBLE PRECISION` actuel est migré sans perte (le moteur lit déjà des centièmes, [F] audit §2).
- **Pas de saisie** : par défaut 0,01 (ce que fait le moteur). L'établissement peut imposer le demi-point ou le quart de point (S-102) : la grille refuse alors 13,3.
- **Saisie hors barème** : refusée avec le message « 23 sur 20 : vérifiez le barème » ; un barème se corrige en un geste et **reconvertit** les notes saisies si le professeur le demande (« mes notes étaient sur 10 »).

### 8.2 Statuts de résultat [R]

| Statut | Sens | Compte dans la moyenne ? | Montré à la famille | Source |
|---|---|---|---|---|
| `note` | une valeur sur le barème | oui | la note | professeur |
| `absent` | absent à l'évaluation ; **justification lue au socle 12** (justifiée, non justifiée, en attente) | **non** (R-V5) | « Absent » | professeur ; **pré-rempli** depuis l'appel de la séance (socle 12) |
| `a_rattraper` | absent ou non rendu, rattrapage prévu (date, modalité) | non tant qu'il n'est pas noté | « À rattraper le … » | professeur ; rattrapage noté = le résultat devient `note` (historique gardé) |
| `dispense` | dispense ou inaptitude couvrant l'évaluation (EPS : socle 34 ; dispense d'enseignement : socle 35) | non | « Dispensé » | pré-rempli depuis la dispense datée, **sans motif** |
| `non_rendu` | travail non remis | **non par défaut** ; voir Q-EV1 | « Non rendu » | professeur |
| `non_note` | l'élève a composé mais le professeur ne note pas (copie illisible, travail collectif non individualisable, adaptation PAP sans note) | non | « Non noté » | professeur |
| `non_concerne` | l'élève n'était pas dans le public du cours à la date (arrivé après, parti avant, changement de groupe, exclu temporairement du cours, stage) | non | rien (ligne absente) | **calculé** (socle 5 `ent_public_cours_jour`), jamais saisi |
| `annule` | résultat annulé (incident, fraude présumée en attente de décision, erreur de sujet pour cet élève) | non | « Annulé » | professeur ou direction, motif obligatoire |
| `en_attente` | copie pas encore corrigée (saisie partielle) | non | rien (la case reste vide) | défaut d'une case vide |

Le moteur existant connaît `note`, `absent`, `dispense`, `non_note` [F `180:6-23`] ; les autres statuts s'ajoutent sans casser (V1).

### 8.3 Moyenne de cours et de matière [R]
1. **Moyenne de cours** = Σ (note ramenée × coef d'évaluation) / Σ coef d'évaluation, sur les résultats `note` de la période ; ramener = note / barème × échelle (20 par défaut).
2. **Évaluation « compte si elle améliore »** (option par évaluation, S-103) : comptée seulement si elle fait monter la moyenne de cours de l'élève. **Évaluation « ne compte pas »** (diagnostique, entraînement) : publiée, jamais comptée. Ce sont les deux seules variantes ; pas de « note bonus » à points au-dessus de 10 (S-104).
3. **Sous-matières** (`ent_matiere.parent_id`, socle 5 migration 338) : la moyenne de la matière parente est la moyenne pondérée des moyennes de sous-matières (coefficient de sous-matière, défaut 1) ; une sous-matière sans moyenne est ignorée. **Une seule méthode** (S-105 signale l'autre méthode du marché, « toutes les notes à plat »).
4. **Moyenne de matière** = moyenne pondérée par coefficients d'évaluation de toutes les évaluations des cours de la matière suivis par l'élève dans la période (un élève passé du groupe 1 au groupe 2 de maths : ses notes des deux groupes comptent, chacune sur sa partie de période) [R, CE-12].
5. **Aucune note comptée → aucune moyenne** (jamais 0) [F : règle déjà tenue].

### 8.4 Moyenne générale [R]
- Σ (moyenne de matière × coefficient de matière) / Σ coefficients des matières **ayant une moyenne** ; les matières marquées « hors moyenne générale » (vie de classe, option facultative selon le choix de l'établissement, dispositif) sont exclues.
- La moyenne générale est **indicative** : aucune décision automatique n'en découle (passage, mention, orientation sont des décisions humaines, socle 23 ; R-V20).

### 8.5 Moyenne annuelle [R, Q-EV3]
- **Méthode par défaut : moyenne des moyennes de période** de la matière, **à poids égaux** (et non pondérées par la durée des périodes), ce qui est l'usage le plus courant et le plus lisible pour les familles [H] ; une période sans moyenne est ignorée.
- **Seconde méthode** (réglage d'établissement, S-106) : moyenne de **toutes les évaluations de l'année** (pondérées par leurs coefficients). Elle évite qu'un trimestre à deux notes pèse autant qu'un trimestre à huit.
- La méthode est **figée pour l'année** une fois la première période arrêtée (changer en cours d'année changerait des moyennes déjà communiquées).
- Au lycée, la méthode et les modalités relèvent du **projet d'évaluation** de l'établissement (§5) : l'écran de réglage l'affiche.

### 8.6 Arrondis [R]
- **Le calcul est exact**, l'arrondi n'existe qu'**à l'affichage et à l'export**, toujours « demi vers le haut » [F : règle tenue, `evaluation.rs:252`].
- **Précision d'affichage** : centième par défaut ; dixième possible (S-107). Le « demi-point » d'affichage existant (`ent_eva_regle.arrondi = 'demi_point'`) est **signalé** (S-107) : il change la moyenne affichée de jusqu'à 0,25 point et crée des contestations ; recommandation : le retirer de l'écran, le garder seulement pour les établissements qui l'ont déjà.
- **Une seule règle d'arrondi** pour tout l'établissement ; les exports officiels appliquent la précision exigée par leur format [NV : précision LSL/LSU, réservée éditeurs].

### 8.7 Statistiques de classe [R]
- Moyenne de classe = moyenne des moyennes d'élèves (pas de toutes les notes) ; minimum, maximum ; **médiane** en plus pour le professeur et la direction.
- Calculées sur le **public du cours ou de la classe à la date d'arrêt** ; un élève parti avant l'arrêt n'entre pas dans les statistiques de la classe (mais garde son bulletin, CE-08).
- **Seuil de confidentialité** : aucune statistique montrée à une famille sous **5 élèves notés** (un groupe d'option de 3 élèves révèle sinon les notes des autres) [R, R-V12].
- Le professeur d'un groupe ne voit pas les statistiques de la classe entière [F : règle déjà tenue, `evaluations.rs:1583-1587`].
- **Rang** : non calculé par défaut [F : non calculé] ; réglage signalé S-108 (Q-EV5).

### 8.8 Exemple de référence (sert de test d'or) [R]
Élève Léa, 4B, trimestre 1, mathématiques (coef de matière 3) : DS 1 = 14/20 coef 2 ; interro = 7/10 coef 1 ; DM = non rendu ; DS 2 = absent justifié, rattrapé 11/20 coef 2 ; évaluation diagnostique « ne compte pas » 4/20.
Moyenne de cours = (14×2 + 14×1 + 11×2) / (2+1+2) = 64/5 = **12,80**. L'explication montrée à la famille liste les cinq lignes et dit pour chacune « comptée », « non rendue, non comptée », « ne compte pas (diagnostique) ».

### 8.9 [ajout personas] Barème détaillé et moyennes particulières (`PERSONAS_21.md` M-579, M-583, M-589, M-652)
- **Barème détaillé** (facultatif, par évaluation) : liste d'exercices ou de critères, chacun avec son maximum ; la note est la **somme** des points, le barème la somme des maximums ; le moteur ne change pas (il lit la note). Le professeur voit la **réussite par exercice** (moyenne et répartition, ses cours seulement) ; l'élève voit son détail si le professeur le choisit (EVA-262 à 264).
- **Exercice annulé** (énoncé faux) : le barème est réduit de son maximum ; ScolIA **propose** de ramener les notes sur le barème réduit, le professeur valide, l'historique garde l'ancienne valeur (CE-83).
- **Moyenne du groupe** : pour un enseignement en groupe dont le public vient de plusieurs classes, la statistique de comparaison au bulletin est celle **du groupe** (élèves notés du cours, seuil de 5, §8.7), libellée « moyenne du groupe » ; pour un enseignement en classe entière, celle de la classe (R-V30).
- **Moyenne externe** : pour un enseignement suivi hors de l'établissement (CNED, autre établissement, conservatoire), la moyenne de période transcrite par le secrétariat **est** la moyenne de matière de l'élève quand la direction l'a déclaré ; elle entre dans la moyenne générale selon le coefficient de la matière, dans la moyenne annuelle selon la méthode de l'établissement (§8.5), et porte sa source (R-V38).

## 9. La vie d'une période d'évaluation

### 9.1 États [R]

La période (dates, périodisation, date d'arrêt, date de conseil) appartient au **socle 5** (migration 335 [F, `ent-integration-lot11`]). Son **état d'évaluation** appartient au socle 21, **par classe** (deux classes d'un même niveau peuvent avoir leur conseil à huit jours d'écart) :

```
ouverte ──(date d'arrêt atteinte, ou geste direction)──▶ notes_arretees
   ▲                                                       │  instantané d'arrêt (V8)
   │ réouverture motivée ★ (direction)                     │  appréciations encore ouvertes
   └───────────────────────────────────────────────────────┤
                                                           ▼
                                        appreciations_closes (veille du conseil, auto ou geste)
                                                           │
                                          conseil tenu (socle 23) ▼
                                                     conseil_tenu
                                                           │ bulletins publiés (socle 23)
                                                           ▼
                                                        close
```

- **Transitions contrôlées par une machine d'états en base** (aujourd'hui : n'importe quelle transition, y compris « fermée → ouverte » par une route historique qui exige seulement `matieres.gerer`, défaut D2 [F `routes.rs:8300-8330`]).
- **Arrêt automatique à la date d'arrêt** (heure locale de l'établissement, fin de journée) si la direction l'a choisi (défaut : oui, S-109) ; rappel aux professeurs à J-7 et J-2 de ce qui leur manque (§15).
- **À l'arrêt** : (a) les évaluations non encore publiées de la période sont **publiées automatiquement** (R-V9, Q-EV2) — le relevé famille et le bulletin comptent alors exactement les mêmes évaluations, ce qui supprime l'écart signalé dans `SOCLE2_DOMAINES.md` §6.4 ; (b) les cases vides deviennent visibles comme « manquantes » au tableau de la direction ; (c) l'**instantané d'arrêt** est pris (résultats, moyennes, statistiques, appréciations à cet instant) et sera complété par les appréciations jusqu'au conseil.
- **Rattrapage après l'arrêt** : la direction peut autoriser, pour un élève et une évaluation, un rattrapage noté jusqu'à la veille du conseil (R-V10) ; l'instantané est **reversionné**, l'écart est tracé.
- **Réouverture** (★, direction) : motif obligatoire, liste des classes concernées, durée ; les changements de la réouverture sont des **corrections** journalisées ; si un bulletin est publié, la réouverture prépare une version 2 (§9.3).

### 9.2 Ce qui se passe à chaque date de l'année (vue du terrain) [H]

| Moment | Ce que fait ScolIA |
|---|---|
| Juin N-1 / rentrée | Le socle 5 pose la périodisation par niveau (trimestres en collège, semestres possibles au lycée) ; le socle 21 propose les coefficients de matière de l'an passé ; la direction confirme le **projet d'évaluation** (lycée) et les règles (§8). |
| Septembre | Les professeurs créent leurs premières évaluations depuis leur liste unique d'enseignements (D-022 bis) ou depuis le cahier de textes (socle 20 : « ce travail sera noté ») ou la séance de l'EDT (socle 10). |
| Fil de la période | Saisie, publication, absents pré-remplis depuis l'appel, rattrapages programmés, dispenses appliquées ; alertes précoces au PP et à la CPE (§13.3). |
| J-15 avant l'arrêt | Tableau « état de saisie » de la direction et du PP : évaluations par cours, élèves sans note, cours sans évaluation, professeurs absents (remplaçant à prévenir). |
| Date d'arrêt | Arrêt (auto), publication automatique, instantané. Ouverture du « temps des appréciations ». |
| Arrêt → veille du conseil | Appréciations de cours, de vie scolaire, générale ; aide IA facultative (§13.1) ; relances ; fiches de dialogue (socle 23, T-11). |
| Conseil | Socle 23 : séance, décisions, mentions, avis. |
| Après le conseil | Bulletins générés, contrôlés, publiés, remis à chaque destinataire (socles 23, 7, 8) ; période close. |
| Fin d'année | Moyennes annuelles, LSU (collège, socle 22), LSL (lycée, socle 23), DNB (socle 38) via le moteur du socle 9 ; décisions de passage et d'orientation (23, 37). |

### 9.3 Corriger après l'arrêt, après le conseil, après la publication [R]

| Moment | Qui | Comment | Effets |
|---|---|---|---|
| Avant l'arrêt | le professeur du cours | modification libre | historique conservé (ajout seul), aucune justification ; si l'évaluation est publiée, la famille voit la nouvelle valeur et l'avis « note modifiée » est **regroupé** au résumé |
| Après l'arrêt, avant le conseil | le professeur **demande**, la direction **valide** (ou corrige elle-même) ★ | motif obligatoire (liste + texte) | instantané reversionné ; le PP est prévenu |
| Après le conseil, avant la publication du bulletin | direction ★ | motif obligatoire ; la direction voit l'effet sur les moyennes et sur les décisions déjà prises (« la moyenne passe de 9,96 à 10,04 ») | instantané reversionné ; **les décisions du conseil ne changent pas seules** : si l'effet est significatif, la direction rouvre la décision (socle 23) |
| Après la publication | direction ★ | motif obligatoire | bulletin **version 2** (l'existant le sait faire [F `182`]), publié à tous les destinataires de la v1 avec la mention « bulletin rectifié le … » ; l'ancienne version reste au registre, hors du coffre ; si un export officiel est déjà parti (LSL, Parcoursup), le socle 9 montre l'écart et propose un nouvel export (CX-45) |
| Après la clôture de l'année | direction ★, rare | idem, avec une confirmation renforcée | idem ; réouverture d'une année close refusée par le socle 5 : la correction ne rouvre pas l'année, elle produit seulement les nouvelles versions |

Le journal de correction existant (`ent_note_correction` [F `180:66-77`]) est **gardé** et généralisé à l'historique complet (V1).

### 9.4 [ajout personas] La période se résout par élève (bloquant A de `PERSONAS_21.md`, appliqué par défaut, D-031)
Un cours peut réunir des élèves de plusieurs classes qui n'ont **ni la même date d'arrêt** (CE-41) **ni la même périodisation** (option multi-niveaux : trimestres et semestres). La règle « une évaluation, une période » (R-V1, `eva_evaluation.periode_id`) ne tient pas. Règle de remplacement (R-V29) :
1. L'évaluation porte une **date**. Pour chaque élève de son public, la **période** est celle de la périodisation de **la classe de rattachement de l'élève à cette date** (socle 5), et l'**état** est celui de cette classe pour cette période (`eva_periode_etat`).
2. La grille verrouille **cellule par cellule** : dans le même devoir du 1ᵉʳ décembre, les élèves de la 1ʳᵉ A (arrêtée le 28 novembre) sont en lecture seule, ceux de la 1ʳᵉ D (arrêt le 5 décembre) restent saisissables ; l'écran dit pourquoi (« notes arrêtées pour la 1ʳᵉ A le 28/11 ») et propose la demande de rattrapage ou de correction (EVA-183, EVA-189).
3. **Publication à l'arrêt** (R-V9), **instantané**, **rappels** (J-7, J-2) et **clôture des appréciations** se font **par classe** ; un professeur de groupe reçoit un rappel par classe (EVA-239).
4. La colonne `periode_id` de l'évaluation devient une **période indicative** (celle de la classe majoritaire, pour trier l'écran du professeur) ; **aucun contrôle** ne la lit. Un port `evaluations::periode_eleve(tx, evaluation, eleve)` (et sa version ensembliste) est la seule source.
5. Une évaluation dont **aucun** élève n'est dans une période ouverte est refusée à la création ; une évaluation dont une partie seulement l'est est acceptée, les autres élèves étant marqués « hors période ouverte » (et non `non_concerne`).
6. Moyenne de cours et de matière : calculées **par élève sur sa période** ; les statistiques d'une évaluation multi-classes restent celles de l'évaluation (tous élèves notés) ; la moyenne de comparaison au bulletin est celle du groupe (R-V30).

---

## 10. Publication aux familles et aux élèves

### 10.1 Règles [R]
- **R-V7 — Rien n'est visible d'une famille ou d'un élève sans publication** : notes, statuts, appréciations, compétences, décisions. Les trois fuites constatées (appréciations historiques, compétences, bulletin lisible sans droit aux notes, §2) sont fermées au lot V0.
- **Modes de publication d'une évaluation** : « publier maintenant », « publier le … à … » (après la remise des copies), « publier à l'arrêt des notes ». Défaut : réglage du professeur (dans les limites de l'établissement), recommandation **publication explicite** avec rappel au professeur après 10 jours (S-110, Q-EV2).
- **Délai maximal de publication** (réglage d'établissement, facultatif, S-111) : « toute évaluation est publiée au plus tard 15 jours après sa date » ; au-delà, publication automatique **sauf** évaluation marquée « en cours de correction », avec rappel au professeur. Répond à G28 (« publication des notes réclamée »).
- **Publication = la famille voit aussi la moyenne provisoire mise à jour** ; la moyenne provisoire est marquée comme telle (« provisoire, susceptible d'évoluer jusqu'à l'arrêt des notes du 28 novembre »).
- **Retrait de publication** : possible par le professeur pendant 24 h (erreur de barème), au-delà par la direction ; la famille voit « résultats retirés pour vérification », jamais une note qui disparaît sans mot.
- **Appréciations** : visibles à la publication du bulletin, pas avant (R-V8). Exception choisie par l'établissement : appréciations « intermédiaires » publiées (S-112, signalé).
- **Statistiques de classe** : publiées avec l'évaluation, sous le seuil de 5 (R-V12) ; l'établissement peut les masquer (S-113).

### 10.2 Qui voit quoi, côté famille [R, socle 4 pour les droits du lien]
- Chaque **lien** ayant `voir_notes` voit notes et moyennes publiées ; chaque lien ayant `recevoir_bulletins` reçoit **son exemplaire** du bulletin (socle 7, nominatif, à son adresse) — **les deux droits sont distincts** (un parent sans autorité peut recevoir le bulletin sans voir les notes au fil de l'eau, ou l'inverse, selon la décision de la direction) [R, `04_ADMINISTRATIF.md` §8.6].
- **Élève majeur** : il voit tout ce qui le concerne ; ce que ses parents continuent de voir dépend de **son choix** (socle 4, Q-A7), appliqué aux deux droits ; par défaut, maintien jusqu'à sa réponse (recommandation du socle 4), et l'élève est prévenu du maintien.
- **Personne de confiance, contact d'urgence, payeur seul** : jamais de notes ni de bulletins (R-N60 du socle 8).
- **Interdiction de communication** (socle 4, jugement) : aucune note, aucun bulletin, aucune notification ; le bulletin ne porte **jamais** l'adresse d'un autre destinataire.

## 11. Cas limites (catalogue)

### 11.1 Parcours de l'élève
| Id | Situation | Ce que fait ScolIA [R] |
|---|---|---|
| CE-01 | Élève **arrivé le 4 novembre** dans une période commencée le 1ᵉʳ septembre | Évaluations antérieures : `non_concerne` (ligne absente pour la famille) ; moyenne sur ses seules évaluations ; au bulletin, mention automatique « arrivé le 4 novembre » ; si trop peu d'évaluations, la direction peut marquer la moyenne « non significative » (CE-04) |
| CE-02 | Arrivée avec des **notes de l'établissement d'origine** (relevé papier ou PDF) | Le secrétariat ou le professeur saisit des **résultats externes** (source, établissement, date, pièce du dossier, socle 7) rattachés à la matière ; **ne comptent pas par défaut** ; le professeur peut décider de les compter dans sa moyenne de cours (geste explicite, tracé) |
| CE-03 | Élève **parti** en cours de période | Ses résultats restent ; il sort des statistiques à l'arrêt ; un bulletin de la période est produit si la direction le demande (« bulletin de départ »), sinon le dernier bulletin publié est au dossier de transfert (FX-100) |
| CE-04 | Moyenne fondée sur **une seule note** | Le moteur la calcule ; l'écran du conseil la signale (« 1 évaluation ») ; la direction peut la marquer « non significative » → « en attente » au bulletin (R-02) |
| CE-05 | Élève **absent toute la période** (maladie longue, hospitalisation) | Aucune moyenne ; « en attente » au bulletin ; si fin d'année, évaluation de remplacement proposée (R-01) ; aucune mention du motif |
| CE-06 | Élève en **scolarité à domicile, à l'hôpital** ou au CNED pour une matière | Résultats externes (CE-02) comptés si la direction le décide ; mention neutre « évalué par un autre organisme » |
| CE-07 | **Échange à l'étranger** trois mois (personas Chloé, M-201) | Évaluations de la période : `non_concerne` pendant l'exclusion temporaire du public (socle 5 `ent_cours_membre`) ; relevé de l'établissement d'accueil en résultat externe |
| CE-08 | Élève accueilli (correspondant étranger) quelques semaines | Peut être inclus dans le public d'un cours sans être inscrit en classe (socle 5) ; aucune moyenne au bulletin, aucune statistique |
| CE-09 | **Changement de classe** le 15 janvier (semestre 2 commencé le 10) | Bulletin du semestre dans la **classe de rattachement à l'arrêt** ; les notes de l'ancienne classe comptent (moyenne de matière, §8.3) et l'origine de chaque note est visible ; l'ancienne classe ne fait pas de bulletin pour lui (D15) |
| CE-10 | Changement de classe **la veille de l'arrêt** | Idem ; le PP de la nouvelle classe est alerté ; les appréciations peuvent être écrites par les professeurs de l'ancienne classe (droit maintenu jusqu'au conseil, R-V15) |
| CE-11 | **Changement de groupe** de même matière (groupes de besoin, groupes de niveau) | Les deux cours alimentent la moyenne de matière ; chaque professeur voit ses seules évaluations et l'élève pour ses dates (D-022, Q-S6) |
| CE-12 | Groupes de besoin **recomposés chaque période** | Pas de cas particulier : cours inchangés, public daté |
| CE-13 | **Option abandonnée** en cours d'année (latin à la Toussaint) | Moyenne de l'option sur la période où elle était suivie, affichée « abandonné le … » au bulletin de cette période ; absente des périodes suivantes ; comptée dans la moyenne annuelle selon la méthode (§8.5) et signalée |
| CE-14 | **Spécialité changée** en décembre de 1ʳᵉ | Idem ; les exports officiels reçoivent les moyennes des enseignements réellement suivis par période [NV : règles nationales de changement de spécialité] |
| CE-15 | Spécialité **abandonnée en fin de 1ʳᵉ** | Moyenne annuelle de 1ʳᵉ conservée et exportée (LSL) ; aucune période en terminale |
| CE-16 | **Double cursus** : élève d'ULIS inclus en 5B pour deux matières | Ses évaluations viennent des cours de la 5B où il est inclus (socle 5) et des cours du dispositif ; le bulletin est celui de **sa classe de référence** (socle 2), avec une rubrique pour le dispositif ; l'AESH voit le calendrier |
| CE-17 | Élève de **SEGPA** inclus dans une classe ordinaire pour l'EPS | Idem CE-16 |
| CE-18 | **Section sportive**, internat d'excellence, CHAM : enseignement hors classe | Cours de groupe à public multi-classes ; moyenne au bulletin si l'établissement la compte (matière « hors moyenne générale » possible) |
| CE-19 | Élève en **PFMP** (lycée professionnel) pendant six semaines | Exclusion temporaire datée des cours généraux (socle 39) ; évaluations de la PFMP (tuteur) = résultats externes ou compétences (22), [NV : modalités CCF] |
| CE-20 | Élève **apprenti** dans une classe mixte | Calendrier d'alternance (socle 5) ; mêmes règles ; bulletin du CFA éventuel hors périmètre [H] |
| CE-21 | Élève **exclu temporairement** (sanction) le jour d'un devoir | `absent` avec justification « exclusion » lue au socle 13 (pas de motif affiché) ; rattrapage de droit [NV] ; jamais un zéro |
| CE-22 | **Fusion de deux fiches** d'élève (doublon) | Les résultats suivent la fiche absorbante (la fusion couvre déjà ces tables [F `fusion.rs:72-77`]) ; conflit sur la même évaluation → les deux résultats gardés, l'un marqué « doublon » à trancher par la direction |
| CE-23 | **Élève décédé** | Gel immédiat de tous les avis (socle 8 CN-13) ; aucune publication ; bulletins non émis sauf demande de la direction |
| CE-24 | **Élève majeur** qui retire l'accès de ses parents | À sa demande (socle 4) : plus de notes ni de bulletins aux parents à partir de la date ; ce qui a déjà été remis reste remis ; les parents payeurs gardent la facturation |

### 11.2 Absences, dispenses, aménagements
| Id | Situation | Ce que fait ScolIA [R] |
|---|---|---|
| CE-25 | Absent à un DS, **absence justifiée** | `absent` pré-rempli depuis l'appel avec sa justification ; proposition « programmer un rattrapage » ; jamais un zéro (R-01) |
| CE-26 | Absent, **non justifié** | Idem ; la CPE voit l'absence à une évaluation dans son suivi (socle 12) ; répétition → alerte (CE-27) ; **jamais de zéro** |
| CE-27 | **Absences répétées aux évaluations** (stratégie d'évitement) | Alerte au professeur, au PP, à la CPE (seuil : 3 évaluations dans la période, S-114) ; le professeur peut proposer une évaluation dédiée ; en fin d'année, la direction convoque à une **évaluation de remplacement** (EVA-172) ; la moyenne de période peut être mise « en attente » (R-02) ; la suite disciplinaire éventuelle est au socle 13 |
| CE-28 | Absent **justifié à l'évaluation de remplacement** | Nouvelle convocation (R-01) ; ScolIA trace les convocations |
| CE-29 | Absent **non justifié à l'évaluation de remplacement** | Seul cas où la note 0 est proposée, avec confirmation de la direction et la référence réglementaire affichée (R-01) |
| CE-30 | L'appel n'a pas encore été fait quand le professeur saisit | Cases vides ; quand l'appel arrive, ScolIA **propose** (sans écraser une note déjà saisie) « 2 élèves absents à la séance : marquer absents ? » |
| CE-31 | L'appel dit absent, le professeur a une copie (retard après l'appel) | La saisie du professeur prévaut ; la vie scolaire est prévenue de l'incohérence (socle 12 corrige l'absence en retard) |
| CE-32 | **Dispense d'EPS** pour trois semaines | `dispense` pré-rempli sur les évaluations de la plage ; moyenne sur les autres évaluations ; aucun motif visible |
| CE-33 | **Inaptitude totale** à l'année en EPS | Pas de moyenne d'EPS ; « dispensé » au bulletin et à LSU [V R-04] ; matière exclue de la moyenne générale |
| CE-34 | **Inaptitude partielle** (pas de course) | Le professeur adapte et note (sa décision, [NV] au bac) ; ScolIA affiche l'inaptitude partielle au professeur d'EPS, sans motif |
| CE-35 | Élève avec **PAP** (temps majoré d'un tiers) | Le professeur et l'AESH voient l'aménagement fonctionnel au moment de l'évaluation ; le résultat est marqué « aménagé » (interne, jamais imprimé) |
| CE-36 | Élève avec **PPS** dispensé d'un enseignement par le recteur | `dispense` sur la matière ; évaluation neutralisée (R-19) |
| CE-37 | **AESH absente** le jour d'un devoir | L'information vient du socle 6 ; ScolIA alerte le professeur « l'aménagement « lecteur » ne pourra pas être assuré » et propose de reporter pour l'élève |
| CE-38 | Élève **allophone** nouvellement arrivé (UPE2A) | Évaluations adaptées ou `non_note` ; moyenne « en attente » permise ; l'évaluation de remplacement de fin d'année est prévue par le mémento [V] |

### 11.3 Évaluations et résultats
| Id | Situation | Ce que fait ScolIA [R] |
|---|---|---|
| CE-39 | Professeur se **trompe de barème** (notes sur 10 saisies sur 20) | Changer le barème propose « convertir les notes saisies » ou « garder les valeurs » ; si publiée, la famille voit « note modifiée » au résumé |
| CE-40 | **Évaluation annulée** (sujet diffusé, incident) | État `annulee` avec motif ; résultats conservés mais exclus ; la famille voit « évaluation annulée » (sans le motif s'il est sensible) |
| CE-41 | Conseils **étalés** sur deux semaines | Date d'arrêt effective par classe (§4.4) |
| CE-42 | Professeur **en arrêt** à l'arrêt des notes | Le remplaçant (socle 6) saisit ; s'il n'y en a pas, le PP et la direction voient « appréciations impossibles » et la direction peut écrire « Pas d'appréciation (professeur absent) » |
| CE-43 | **Co-enseignement** (deux professeurs sur un cours) | Les deux créent et saisissent ; chaque évaluation garde son auteur ; une seule appréciation de cours, cosignée (S-40 du socle 6 : les deux noms au bulletin) |
| CE-44 | **Remplaçant** arrivé le 3 novembre | Voit tout l'historique du cours (lecture) ; crée ses évaluations ; modifie les évaluations du titulaire seulement si la direction l'autorise (S-115) |
| CE-45 | Titulaire de retour après la suppléance | Retrouve les évaluations du suppléant, en lecture et en écriture (même cours) |
| CE-46 | **Évaluation commune** à quatre classes (examen blanc) | Une évaluation commune, une évaluation par cours, même barème ; statistiques communes visibles de la direction et des professeurs concernés ; organisation au socle 38 |
| CE-47 | Évaluation **sur deux périodes** (DM donné le 20 novembre, rendu le 5 décembre, arrêt le 1ᵉʳ) | La période est celle de la **date de l'évaluation** choisie par le professeur (date de rendu par défaut) ; refus si la période est arrêtée (sauf autorisation) |
| CE-48 | Note **supérieure au barème** (point bonus) | Refusée sauf évaluation « points au-delà du barème autorisés » (S-104) ; plafonnement de la moyenne de cours à l'échelle (20) |
| CE-49 | **Fraude présumée** | `annule` en attente ; la décision relève du socle 13 (discipline) ; jamais un zéro de sanction (R-02) ; le professeur peut faire refaire l'évaluation |
| CE-50 | Copie **perdue** par le professeur | `non_note` avec motif interne ; rattrapage proposé ; jamais à la charge de l'élève |
| CE-51 | Évaluation créée dans **le mauvais cours** | Déplacement vers un autre cours du même professeur tant qu'elle n'est pas publiée ; après, annulation et recréation tracées |
| CE-52 | **Matière renommée** ou sous-matière créée en cours d'année | Les évaluations restent attachées ; le bulletin prend le libellé à la date d'arrêt (socle 5) |
| CE-53 | **Coefficient de matière changé** après le premier trimestre | Changement **daté** : s'applique aux périodes non arrêtées ; les instantanés et bulletins déjà faits ne bougent pas ; la moyenne annuelle utilise le coefficient de chaque période (affiché) |
| CE-54 | Élève **noté deux fois** pour la même évaluation (deux professeurs, saisies simultanées) | Une seule valeur courante (contrainte unique, D8), la seconde saisie voit un conflit à résoudre (version attendue) |
| CE-55 | Professeur **colle 32 lignes** depuis un tableur pour une classe de 30 | Rapprochement par nom et prénom (normalisés) puis par ordre ; 2 lignes non rapprochées montrées, rien n'est écrit sans confirmation |
| CE-56 | **Saisie hors ligne** sur téléphone, envoyée trois heures plus tard (socle 11) | Envoi refusé si la période a été arrêtée entre-temps, avec la liste à envoyer par la direction ; conflit de version montré |

### 11.4 Fin de période, conseil, correction
| Id | Situation | Ce que fait ScolIA [R] |
|---|---|---|
| CE-57 | Note **corrigée après le conseil** (erreur de report) | §9.3 : correction ★, instantané reversionné, décision du conseil non modifiée seule, bulletin v2 si publié, export officiel signalé (CX-45) |
| CE-58 | Correction qui fait **franchir un seuil** (9,96 → 10,04) | La direction voit l'effet ; la mention ou la décision n'est jamais recalculée seule |
| CE-59 | Famille **conteste une note** | Demande de révision adressée au professeur (EVA-166), tracée ; le professeur répond ; rien ne change sans son geste (avant l'arrêt) ou celui de la direction (après) |
| CE-60 | Bulletin publié, puis **appréciation inadaptée** signalée | Correction d'appréciation ★ par la direction ou son auteur avec la direction ; v2 |
| CE-61 | Période **avancée d'une semaine** (socle 5 STR-33) | Le 21 répond « 6 évaluations seraient hors période » ; la direction choisit : les rattacher à la période suivante ou refuser le changement |
| CE-62 | **Bascule d'année** avant la clôture des périodes | Refus par le socle 5 (déjà tenu [F `referentiel_annee.rs:251`]) sauf forçage tracé ; aucune note perdue (G12) |
| CE-63 | 3ᵉ : une matière obligatoire **sans moyenne annuelle** à la clôture | Blocage de la clôture de la dernière période (R-03) : liste des élèves, proposition d'évaluation de remplacement |
| CE-64 | **Harmonisation académique** du DNB qui modifie des moyennes | Hors du 21 : l'harmonisation est au socle 38 ; les moyennes de l'établissement ne sont pas réécrites [V mémento] |

### 11.5 [ajout personas] Cas limites ajoutés par la passe (`PERSONAS_21.md`)
| Id | Situation | Ce que fait ScolIA [R] |
|---|---|---|
| CE-65 | Groupe de spécialité de **six classes**, arrêts du 28 novembre au 9 décembre ; devoir du 1ᵉʳ décembre | Période et verrou **par élève** (§9.4, R-V29) : cellules de la 1ʳᵉ A verrouillées, autres saisissables ; un rappel par classe |
| CE-66 | Option **multi-niveaux** (DGEMC 1ʳᵉ–terminale, latin 2ᵈᵉ–1ʳᵉ) : trimestres pour les uns, semestres pour les autres | Chaque élève a sa période ; l'évaluation du 10 janvier est au T2 pour un 2ᵈᵉ, au S1 pour un 1ʳᵉ ; la grille affiche les deux |
| CE-67 | **LV2 au CNED** en 3ᵉ ; résultats envoyés au fil de l'eau, hors trimestres | Enseignement suivi à l'extérieur déclaré (EVA-240) ; chaque relevé rattaché à la période de l'établissement **par sa date** ; moyenne de période externe saisie (EVA-241), qui vaut moyenne de matière (R-V38) ; relance avant l'arrêt (EVA-242) ; moyenne annuelle chiffrée exigée comme pour toute matière obligatoire (R-V22) |
| CE-68 | **Spécialité suivie dans un autre lycée**, lui aussi sous ScolIA (convention) | **Rien ne passe automatiquement** entre les deux établissements (D-013) ; le lycée d'accueil remet un relevé (document ou export) ; le lycée de l'élève saisit la moyenne externe ; aucun professeur d'un établissement ne voit l'autre |
| CE-69 | **Horaires aménagés musique** : formation musicale évaluée par le conservatoire | Appréciation externe transcrite avec sa source ; moyenne seulement si le conservatoire en donne une et que la direction la compte |
| CE-70 | Élève **sportif de haut niveau** ou en soins réguliers : nombreuses absences justifiées aux évaluations | L'alerte « absences répétées aux évaluations » (CE-27) ne se déclenche pas quand la justification relève d'un aménagement de scolarité (socle 35), sans en afficher la nature |
| CE-71 | Professeure **sur deux établissements** : sa banque de phrases du collège A proposée au collège B | Refus : banque, style et exemples sont rangés par (personne, établissement) (R-V35) ; copie vers l'autre établissement par **son** geste, sans noms (EVA-243) |
| CE-72 | Parent **sans exercice** de l'autorité parentale demande bulletin et décision d'orientation | Reçus par défaut (R-V43, C. civ. 373-2-1 [V R-25]) ; notes au fil de l'eau non, sauf décision de la direction ; jamais en cas de retrait ou de décision de justice contraire |
| CE-73 | Père sous **ordonnance de protection**, l'école de l'enfant lui est cachée | Aucun document ni avis qui révèle l'établissement : ni bulletin papier, ni duplicata, ni réponse à une demande, ni courriel ; liste « à prévenir autrement » de la direction (socle 8, Q-N9) |
| CE-74 | Parent **privé d'accès** qui téléphone au secrétariat pour « un double du bulletin » | Au moment de produire le document, ScolIA relit les droits du lien et affiche un bandeau « ne rien remettre à cette personne » avec la conduite à tenir, sans le motif détaillé (R-V39, EVA-244) |
| CE-75 | Élève avec **prénom d'usage** accepté par l'établissement | Bulletin, relevé, appréciations et propositions IA au prénom d'usage (et pronom décidé) ; LSU, LSL, Parcoursup, certificats à un tiers à l'état civil (R-V36) ; le contrôle d'appréciation signale l'ancien prénom |
| CE-76 | Élève **majeur le 10 décembre**, bulletin du 1ᵉʳ semestre publié le 25 janvier ; il n'a pas encore fait son choix | Destinataires calculés **à la publication** (R-V47) : lui en premier ; ses parents selon le défaut du socle 4 (maintien jusqu'à sa réponse), message à l'élève ; courrier à **son** adresse s'il a un foyer propre |
| CE-77 | La comptabilité demande de **retenir le bulletin** d'une famille en impayé | Refus : aucun lien entre facturation et publication (R-V33) ; la demande est tracée et la direction informée |
| CE-78 | Bulletins de 2ᵈᵉ **remis en main propre** le 15 décembre, en ligne le soir ; une famille absente | Publication en ligne à l'heure prévue pour tous (EVA-247) ; l'exemplaire non remis reste disponible au coffre, aucun courrier supplémentaire sauf famille sans compte |
| CE-79 | **Ancienne élève** (partie il y a huit ans) demande ses bulletins pour un concours | Recherche dans les archives (nom, naissance, INE, année) ; duplicata daté, mention « duplicata délivré le … » ; identité vérifiée selon le socle 7 ; envoi à l'intéressée seulement (EVA-251, EVA-252) |
| CE-80 | Élève **redoublant** ou **nouveau** | Le PP, la direction et la CPE voient le dernier bulletin de l'an passé (EVA-158) ; les professeurs non ; aucune note de l'an passé ne compte ; aucune mention de comportement reprise |
| CE-81 | **Rattrapage collectif** : un élève absent aussi au rattrapage | Le résultat reste `absent` avec la nouvelle date manquée ; deuxième convocation proposée ; en fin d'année, évaluation de remplacement (R-01, R-29) |
| CE-82 | **QCM corrigé automatiquement** dont le corrigé était faux | Le professeur corrige le corrigé (socle 20) ; ScolIA **propose** les nouveaux scores ; rien ne change sans sa validation ; historique gardé |
| CE-83 | **Exercice annulé** dans un barème détaillé (énoncé faux) | Barème réduit, notes ramenées sur le nouveau barème proposées, validées par le professeur (§8.9) |
| CE-84 | **AESH mutualisée** : deux élèves accompagnés évalués jeudi à 10 h dans deux salles | Alerte à l'AESH, aux deux professeurs et au référent la veille au plus tard ; proposition de décaler pour l'un (EVA-268) |
| CE-85 | **Sujet adapté oublié** pour un élève à PAP | La veille, rappel au professeur : « 1 élève a un sujet agrandi prévu, aucun sujet adapté joint » ; jamais visible des autres élèves |
| CE-86 | Conseil **présidé par délégation** ; la présidente tombe malade le jour même | La direction change la délégation en un geste (EVA-256) ; le PV porte le nom du président effectif |
| CE-87 | Le conseil discute du cas d'un **délégué** | Geste « délégués sortis » : vue complète le temps du cas, retour à la vue séance, tracé au PV |
| CE-88 | Élève qui part vers un **autre établissement ScolIA** | Dossier de transfert (EVA-255) : bulletins de l'année et relevé de la période en cours, tout publié et figé au jour du départ (R-V42) ; remis par le secrétariat ; l'établissement d'arrivée saisit des résultats externes (CE-02) ; aucun accès direct (D-013) |
| CE-89 | **Cohabitation** : bulletin Charlemagne non rapproché (INE absent, homonyme) | Reste dans la file « à rapprocher » ; **jamais distribué** ; rapprochement par la secrétaire avec aperçu (EVA-234) |
| CE-90 | **Cohabitation** : Charlemagne ou son portail diffuse aussi les bulletins | Double envoi, et envoi selon des droits périmés ; recommandation : diffusion **par ScolIA seule**, celle du portail désactivée pour les bulletins [H : à confirmer avec le paramétrage de Charlemagne] |
| CE-91 | Terminale : **moyenne annuelle manquante** d'un enseignement du baccalauréat (élève malade au 2ᵈ semestre) | Contrôle R-V45 avant l'arrêt : élève et enseignement listés ; évaluation de remplacement proposée (R-29) |
| CE-92 | L'établissement demande une **note de vie scolaire** comptée dans la moyenne | Non proposée (S-127) ; l'appréciation de vie scolaire et l'assiduité restent |
| CE-93 | Une appréciation cite une **sanction effacée** (« après son exclusion de l'an passé… ») | Avertissement à l'auteur avant validation (comme R-V18) ; aucune sanction au bulletin (R-V32) |

## 12. Appréciations

### 12.1 Types [R]
| Type | Auteur | Quand | Visible famille | Notes |
|---|---|---|---|---|
| **De cours** | professeur du cours (ou co-enseignants, remplaçant) | de l'ouverture à la clôture des appréciations | avec le bulletin | une par (élève, cours, période) ; plusieurs cours de la même matière → plusieurs appréciations, toutes imprimées |
| **Générale** | PP (ou direction) | arrêt → conseil (souvent rédigée **pendant** le conseil) | avec le bulletin | peut reprendre la synthèse du conseil |
| **De vie scolaire** | CPE | arrêt → conseil | avec le bulletin (si le modèle l'imprime) | assiduité et ponctualité calculées à côté (socle 12) |
| **Du conseil** | président du conseil | pendant le conseil | avec le bulletin | **socle 23** |
| **Annuelle** (LSL) | professeur | fin d'année, 1ʳᵉ et terminale | via le livret | **socle 23** |
| **Intermédiaire** (option S-112) | professeur | en cours de période | si l'établissement l'a choisi | « point d'étape » à mi-trimestre |

### 12.2 Règles [R]
- **Longueur** : limite par modèle de bulletin (défaut 400 caractères pour une appréciation de cours, 600 pour la générale), compteur visible ; jamais de troncature silencieuse à l'impression.
- **Versions** : chaque enregistrement est une version ; l'auteur de chaque version est gardé (aujourd'hui seul le dernier auteur reste, D6).
- **Brouillon / validée** : une appréciation proposée par l'IA est un **brouillon** tant que son auteur ne l'a pas validée ; un brouillon n'est **jamais** imprimé, ni montré à quiconque d'autre que son auteur.
- **Banque de phrases du professeur** (ses propres formulations, réutilisables) : oui ; banque commune imposée : non (D-023, S-116).
- **Contrôles** (non bloquants) : appréciation vide, identique pour plus de 5 élèves, contradiction avec la moyenne (« excellent travail » sous 8), prénom d'un autre élève dans le texte, mots interdits par l'établissement (liste courte, S-117), diagnostic médical apparent (« dyslexique », « TDAH ») → avertissement « information de santé, à retirer » (R-V18).
- **Après publication** : correction ★ (§9.3).

### 12.3 [ajout personas] Compléments (`PERSONAS_21.md` M-585, M-586, M-598, M-640, M-651)
- **Appréciation du dispositif** (type `dispositif`) : rédigée par le coordonnateur ULIS, SEGPA ou UPE2A pour les élèves du dispositif ; imprimée au bulletin de la **classe de référence** (CE-16) ; jamais sur un document à un tiers (R-V37).
- **Appréciation générale et du conseil** : un seul texte par défaut (S-121).
- **Saisie** : filtre par groupe (et non seulement par classe), **dictée vocale** sur mobile (V9), éléments des séances du professeur (oublis, travaux non faits, socles 12 et 20) affichés à côté de l'appréciation, pour ses cours seulement.
- **Contrôles** ajoutés à ceux du §12.2 : ancien prénom d'un élève qui a un prénom d'usage (R-V36) ; mention d'une sanction ou d'un fait disciplinaire ancien (CE-93) ; prénom d'un autre élève (déjà prévu).
- **Glossaire** : les libellés imprimés (mentions, « en attente », « dispensé », « moyenne du groupe ») ont une définition courte accessible depuis le bulletin en ligne et le relevé.

---

## 13. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5)

Principe (D-025, CLAUDE.md) : l'IA n'a **aucun droit propre** ; elle appelle les capacités du socle **avec les droits de la personne** ; chaque proposition est un **brouillon** validé par une personne ; tout est journalisé ; **tout fonctionne sans IA**.

### 13.1 Aide à la rédaction des appréciations [R]
- **Pour qui** : le professeur pour ses cours ; le PP pour l'appréciation générale ; la CPE pour la vie scolaire.
- **Ce que l'IA lit** (capacité `evaluations.contexte_appreciation`, minimisée) : pour **un élève et un cours** : les résultats de la période et de la précédente (valeurs, natures, tendances), les statuts (sans motif), les commentaires **que le professeur a lui-même** écrits sur les copies, ses appréciations précédentes, sa banque de phrases, les consignes de l'établissement (longueur, ton). Pour le PP : les appréciations **validées** des collègues, les moyennes, l'assiduité calculée. **Jamais** : santé, PAP/PPS (sauf le libellé fonctionnel si le professeur le choisit), discipline détaillée, situation familiale, notes des autres élèves (seules la moyenne de classe et la médiane sont fournies), nom de famille (pseudonymisé : prénom ou initiale).
- **Ce qu'elle produit** : 1 à 3 propositions courtes, dans le **style** du professeur (appris de ses appréciations passées, à sa demande), en français ; chaque proposition indique les faits sur lesquels elle s'appuie (« moyenne en hausse de 2,1 points ; 2 évaluations non rendues »).
- **Validation** : une par une (jamais « tout valider » sur une classe entière sans relecture : la validation en lot exige d'avoir ouvert chaque appréciation, R-V19) ; la version validée porte la marque interne « rédigée avec aide » (journal), invisible de la famille [R, Q-EV8 : transparence vis-à-vis des familles à décider].
- **Interdits** : proposer une note, une mention, une décision ; comparer nommément à d'autres élèves ; inventer un fait absent des données ; écrire au nom d'un autre professeur.

### 13.2 Cohérence et relecture [R]
Contrôles du §12.2 (déterministes) + relecture IA facultative de l'ensemble d'une classe par le PP : appréciations contradictoires entre matières, ton blessant, formulations stigmatisantes, fautes. La sortie est une **liste de remarques** adressées à l'auteur de chaque appréciation, jamais une modification.

### 13.3 Élèves en difficulté : alertes précoces [R]
- **D'abord déterministe** (aucun modèle) : règles lisibles, réglage à trois positions (S-118) : baisse de la moyenne générale ≥ 2 points d'une période à l'autre ; ≥ 3 matières sous 8 ; ≥ 3 absences ou non-rendus aux évaluations dans la période ; cumul avec l'absentéisme (socle 12) ou des faits de vie scolaire (socle 13) **si la personne a le droit de les voir**.
- **Pour qui** : PP de la classe, CPE de référence, direction ; le professeur pour **ses** cours seulement (« 4 élèves sous 8 dans ce groupe »).
- **Expliquée** : chaque alerte dit pourquoi (les faits) ; **jamais un « score de risque »** ni une probabilité de décrochage ; **jamais visible** de la famille ni de l'élève ; aucune conséquence automatique.
- **Suite humaine** : entretien, PPRE (socle 35), cellule de veille, information de la famille — décidées et tracées par une personne.
- **IA facultative** : résumé en langage clair de la situation d'un élève pour le PP (« depuis la Toussaint, chute en sciences, absences les lundis matin ») à partir des seules capacités autorisées.

### 13.4 Préparation du conseil [R]
- Dossier par élève (déterministe) : moyennes et évolution, statuts, appréciations, alertes, propositions de mentions selon **les règles de l'établissement** (« moyenne ≥ 16 et aucune appréciation négative → félicitations proposées ») — proposition, jamais décision.
- IA facultative : **synthèse des appréciations** d'un élève pour le PP (aide à l'appréciation générale) ; liste des élèves « à discuter » ordonnée et motivée.
- Pendant le conseil (socle 23) : prise de notes du secrétaire de séance, proposition de compte rendu à valider.

### 13.5 Côté famille et élève [R]
- **« Explique-moi la moyenne »** : l'explication est **déterministe** (§8) ; l'IA peut la reformuler en langage simple ou la **traduire** dans la langue du responsable (socle 8), avec la mention « traduction automatique, seul le bulletin en français fait foi ».
- L'IA d'un parent ne voit que ce que son lien lui ouvre ; celle d'un élève, ses seules données publiées.

### 13.6 Capacités déclarées (registre) [R]
| Capacité | Genre | Droit | Sensibilité | Confirmation |
|---|---|---|---|---|
| `evaluations.contexte_appreciation(eleve, cours, periode)` | lecture | `appreciations.saisir` sur le cours | C2 | — |
| `evaluations.proposer_appreciation(...)` | brouillon | idem | C2 | validation par l'auteur |
| `evaluations.alertes(classe, periode)` | lecture | `evaluations.suivre_classe` | C2 | — |
| `evaluations.expliquer_moyenne(eleve, matiere, periode)` | lecture | `evaluations.lire` (portée) ou lien `voir_notes` | C1 | — |
| `evaluations.etat_saisie(classe, periode)` | lecture | `evaluations.piloter` | C1 | — |
| `evaluations.relancer(professeurs, message)` | commande | `evaluations.piloter` | C1 | aperçu du message, envoi confirmé (socle 8) |
| `conseils.synthese_eleve(eleve, periode)` (socle 23) | lecture | `conseils.preparer` | C2 | — |
**Aucune capacité IA n'écrit une note, un statut, une mention, une décision ni une publication.**

### 13.7 Cadre juridique de l'IA [NV, Q-EV8]
Le règlement européen sur l'IA classe à haut risque les systèmes **destinés à évaluer les acquis d'apprentissage** (annexe III, 3 b) [NV sur le texte consolidé, report annoncé au 2 décembre 2027]. Lecture prudente [R] : (1) ScolIA n'utilise **aucun modèle pour évaluer** ; (2) l'aide à la rédaction d'appréciations relève d'une **tâche préparatoire** à une évaluation faite par le professeur (art. 6.3, à faire confirmer par un juriste) ; (3) les alertes du §13.3 sont des **règles** fixées par l'établissement, pas un système d'IA ; (4) une évaluation d'impact (AIPD) est faite pour les capacités C2 ; (5) fournisseur souverain (Mistral prioritaire, CLAUDE.md), aucune donnée réutilisée pour l'entraînement, pseudonymisation. **Aucune capacité IA de ce socle n'est mise en service sans l'étude juridique** (lot V11 conditionnel).

### 13.8 [ajout personas] Ce que l'IA ne fait pas, et ce qu'elle ajoute (`PERSONAS_21.md` M-584, M-610, M-659 à M-662)
**Exclusions écrites (R-V40)** — à afficher au professeur qui le demande, avec la raison :
- **pas de correction de copies** ni de proposition de note (annexe III point 3 b [V* R-33] ; D-032) ;
- **pas de détection de fraude ni de plagiat** (point 3 d : surveiller et détecter les comportements interdits pendant les tests) ;
- **pas de formation de groupes d'élèves** (remédiation, niveau, besoin) ni d'« orientation du processus d'apprentissage » individuelle (point 3 b) ;
- **pas de proposition de mention, d'avis ou de décision** (R-V20).
**Style et exemples (R-V35)** : les appréciations passées ne servent d'exemples de style que si le professeur l'a choisi, **pseudonymisées** (prénoms et noms retirés), et seulement dans l'établissement où elles ont été écrites.
**Profilage** : les alertes du §13.3 sont des règles écrites par des personnes, pas un système d'IA ; elles évaluent néanmoins des personnes (profilage au sens du RGPD [V* R-33]) : AIPD, explication de chaque alerte, aucune conséquence automatique (RGPD art. 22, R-23), jamais un modèle qui les remplace sans l'étude juridique.
**Ajouts différenciants** (lot V11, après l'étude juridique pour ce qui utilise un modèle de langage) :
- **Relecture des bulletins** avant publication pour le secrétariat et la direction (EVA-275) : prénom ou pronom incohérent avec le prénom d'usage, appréciation qui cite un autre élève, texte tronqué, ton ; **liste de remarques**, jamais une modification.
- **« Ce qui a changé depuis le dernier conseil »** (déterministe, sans modèle) dans le dossier de chaque élève : évolution des moyennes, nouvelles alertes, appréciations du conseil précédent (§13.4).
- **Synthèse agrégée d'une évaluation** pour son professeur (exercices les moins réussis, à partir du barème détaillé §8.9), **sans classement ni liste d'élèves** : utile, mais elle touche l'évaluation des acquis ; **sous l'étude juridique** (Q-EV8 c).
- **Résumé hebdomadaire des familles** (EVA-278) : faits déterministes ; reformulation et traduction par l'IA facultatives (§13.5).

---

## 14. Droits et minimisation

### 14.1 Catalogue d'actions proposé (D-017) [R]
| Action | Portée | Modèles par défaut | Remplace [F] |
|---|---|---|---|
| `evaluations.lire` | cours (professeur), classe (PP), établissement (direction, vie scolaire pour moyennes seulement, secrétariat) | professeur, PP, direction, secrétariat | `notes.lire` |
| `evaluations.saisir` | cours (service actif à la date) | professeur, remplaçant | `notes.saisir` |
| `evaluations.saisir_tout` | établissement | direction | `notes.saisir` (établissement) |
| `evaluations.publier` | cours | professeur | — |
| `evaluations.corriger` ★ | établissement | direction | `notes.corriger` |
| `evaluations.periodes_gerer` ★ (arrêter, rouvrir) | établissement | direction | `periodes.verrouiller` ; **plus** `matieres.gerer` (D2) |
| `evaluations.parametrer` (règles, coefficients, projet d'évaluation) | établissement | direction ; secrétariat pour les coefficients si la direction le délègue | `periodes.verrouiller` (règles), `matieres.gerer` (coefficients) |
| `evaluations.piloter` (état de saisie, relances) | établissement / classe | direction, PP (sa classe) | — |
| `evaluations.suivre_classe` (alertes) | classe | PP, CPE de référence, direction | — |
| `appreciations.saisir` | cours | professeur | `notes.saisir` |
| `appreciations.generale` | classe | PP, direction | — |
| `appreciations.vie_scolaire` | classe / élèves suivis | CPE | — |
| `evaluations.resultats_externes` | établissement | secrétariat, direction | — |
| `evaluations.calendrier_accompagne` | élèves accompagnés | AESH | — |
| `livrets.exporter` ★ | établissement | **secrétariat, direction** (D-022), jamais professeur | `livrets.exporter` + `exports.officiels` (D14) |
Les actions `conseils.*` et `bulletins.*` sont au socle 23 ; `competences.*` au 22.

### 14.2 Ce que voit chacun [R]
| Qui | Voit | Ne voit pas |
|---|---|---|
| Professeur | ses cours : évaluations, résultats, statuts **sans motif**, aménagements fonctionnels ; moyennes et statistiques de **ses** cours ; pendant la **fenêtre du conseil** : pour les élèves de ses groupes, les moyennes et appréciations validées des autres matières (§14.3) | la classe entière s'il n'a qu'un groupe (D-022) ; les notes détaillées des autres matières ; les motifs d'absence et de dispense |
| PP | la classe entière : moyennes, appréciations, état de saisie, alertes | les notes détaillées des autres professeurs avant l'arrêt (S-119) |
| CPE | moyennes et appréciations des élèves qu'elle suit, alertes | notes détaillées |
| Direction | tout l'établissement | — |
| Secrétariat | bulletins, état de préparation des exports, coefficients | notes détaillées (sauf délégation) |
| Accueil, AED | rien | tout |
| AESH | calendrier des évaluations et aménagements des élèves accompagnés | notes, moyennes, appréciations (Q-EV9) |
| Infirmière, médecin | rien (elles saisissent l'inaptitude au socle 34) | notes |
| Famille | ce qui est publié et que **son lien** ouvre (`voir_notes`, `recevoir_bulletins`) | ce qui n'est pas publié ; les autres élèves (statistiques sous 5 masquées) |
| Élève | ses résultats publiés, moyennes, bulletins | idem |
| Délégués au conseil | la vue « séance » (§14.4) projetée, rien sur leur compte | tout dossier |

### 14.3 La fenêtre du conseil (D-022 précisé) [R, Q-EV6]
D-022 dit qu'un professeur ne voit, au conseil, que les élèves de son groupe. Pour ces élèves, le conseil exige pourtant une vue **collégiale** (toutes les matières). Proposition : **de l'arrêt des notes au conseil + 7 jours**, un professeur voit, **pour les seuls élèves de ses cours**, les moyennes et les appréciations **validées** des autres matières, l'appréciation générale et la vie scolaire ; hors de cette fenêtre, il ne voit que ses cours. Toute consultation est journalisée.

### 14.4 La vue « séance » du conseil [R]
Projetée en présence des délégués : nom, moyennes, appréciations, proposition de mention et de décision ; **sans** santé, aménagements, faits de discipline détaillés, situation familiale, alertes. La direction bascule en vue complète hors présence des délégués si un cas l'exige (socle 23).

### 14.5 Garde-fous [R]
- Établissement **toujours** pris dans la session ; RLS sur toutes les tables (D9) ; clés étrangères composées.
- Droits **datés** : service actif à la date (socle 6), lien actif à la date (socle 4), délai de grâce en lecture pour un professeur parti jusqu'au conseil de la période (P37 du socle 6).
- Journal des **consultations** de bulletins et de dossiers de conseil (R-21) ; journal des corrections et réouvertures.
- Événements **sans note ni texte** (déjà tenu pour la grille [F], à tenir aussi pour l'historique, D1).
- Matrice de séparation (tests) : professeur d'un autre groupe de la même classe, PP d'une autre classe, remplaçant avant sa date, professeur parti, parent sans `voir_notes`, parent sans `recevoir_bulletins`, parent interdit de communication, élève majeur ayant retiré l'accès, AESH, établissement B, appli mobile, capacité IA.

### 14.6 [ajout personas] Pièges de droits et de minimisation (`PERSONAS_21.md`)
1. **Les appréciations et les décisions suivent `recevoir_bulletins`**, pas `voir_notes` (R-V34) : un parent qui voit les notes au fil de l'eau sans recevoir les bulletins ne lit pas les appréciations.
2. **Parent sans exercice** de l'autorité parentale : bulletins et décisions d'orientation par défaut, notes non (R-V43) ; **retrait** ou décision de justice : rien (R-26).
3. **Aucune activité d'un parent visible de l'autre** (lecture, accusé, demande de révision, réponse) (R-V46) ; le personnel voit, par parent, ce qui est utile au suivi (accusé de lecture).
4. **Le guichet relit les droits** : toute production manuelle (relevé, duplicata, dossier de transfert, réponse à une demande) relit les droits du lien à l'instant et affiche les restrictions (R-V39).
5. **Établissement caché** (ordonnance de protection) : aucun document qui porte l'en-tête de l'établissement n'est envoyé à la personne visée (CE-73).
6. **Banque de phrases, style, exemples IA** : rangés par (personne, établissement) (R-V35) — corrige `eva_banque_phrase user_id` (§21, C9).
7. **Vue « séance » du conseil** : jamais d'aménagement, de PAP, de PPS ni de dispositif, même sous forme de pictogramme (Pronote 2026 affiche une mini-fiche d'aménagements en mode conseil [V]) ; rappel de discrétion à l'ouverture (R-34 [NV sur la discrétion]) ; geste « délégués sortis » (CE-87).
8. **Mode conseil de groupe** (professeur de spécialité) : il navigue d'un élève à l'autre de **son** groupe, chaque élève n'étant visible que pendant la fenêtre du conseil **de sa classe** (§14.3) ; hors de ces fenêtres, ses seuls cours.
9. **Rubrique « accompagnements »** : jamais vers un tiers, Parcoursup compris (R-V37).
10. **Statistiques par professeur** et comparaisons d'évaluations communes : direction seulement, jamais de classement, consultations journalisées ; un professeur voit ses groupes et la moyenne anonyme des autres (R-V44).
11. **AESH** : accès au calendrier et aux aménagements de l'élève **pour la durée de l'accompagnement** ; sujet visible seulement si le professeur le partage (EVA-267) ; fin d'accompagnement = fin d'accès le jour même.
12. **Ancien élève** : accès en lecture à son coffre de bulletins pendant un an après son départ (Q-EV22), puis sur demande au secrétariat (duplicata).
13. **Redoublant, nouvel élève** : le bulletin de l'an passé est visible du PP, de la direction et de la CPE, pas des professeurs (CE-80).
14. **Note privée** du professeur : communicable au titre du droit d'accès (R-37), l'écran le dit (R-V41).
15. **Finances** : aucun lien entre une dette et la publication d'un document d'évaluation (R-V33).

---

## 15. Notifications (socle 8)

| Nature | Urgence | Destinataires | Canal | Regroupement | Notes |
|---|---|---|---|---|---|
| `evaluations.note_publiee` [F, socle 8 §4] | U0 | élève ; liens `voir_notes` | in-app, résumé quotidien | par jour et par foyer | jamais la note dans le message externe (P3 du socle 8) ; collégien : différé pendant ses heures de cours (CN-75) ; nature désactivable par l'établissement (NTF-113) |
| `evaluations.note_modifiee` | U0 | idem | résumé | oui | « une note a été modifiée » |
| `evaluations.evaluation_annulee` | U0 | idem | résumé | oui | — |
| `evaluations.rattrapage_programme` | U1 | élève, liens `voir_notes` | in-app, push | non | date et lieu |
| `evaluations.remplacement_convocation` | U1 | élève, liens titulaires | in-app, e-mail **enveloppe**, courrier si aucun compte | non | convocation de la direction (R-01) |
| `evaluations.rappel_saisie` | U0 | professeur | in-app, résumé | par professeur | J-7 et J-2 avant l'arrêt : « il vous manque… » |
| `evaluations.relance` | U1 | professeurs choisis | in-app, e-mail | non | message de la direction ou du PP, aperçu confirmé |
| `evaluations.periode_arretee` | U0 | professeurs, PP | in-app | oui | ouverture du temps des appréciations |
| `evaluations.correction_demandee` / `_validee` | U1 | direction / professeur | in-app | non | — |
| `evaluations.alerte_eleve` | U1 | PP, CPE | in-app | résumé hebdomadaire | **jamais** à la famille |
| `evaluations.bulletin_disponible`, `decision_conseil` [F socle 8] | U1 | chaque lien `recevoir_bulletins`, élève | in-app, e-mail enveloppe, push ; courrier si aucun | non | **socle 23** |
Pas d'avis « moyenne en baisse » aux familles (S-120, signalé).

---

## 16. Échanges nationaux

| Flux | Sens | Socle qui produit | Rôle du 21 | État |
|---|---|---|---|---|
| **LSU** (collège : bilans périodiques, fin de cycle, moyennes annuelles de 3ᵉ) | sortie | 22 | moyennes, statuts EA et dispensé, appréciations | **bloqué** par la validation éditeur (D-2 du socle 9) ; d'ici là, **aide à la saisie** (export de travail lisible) et pré-contrôle |
| **Affelnet** (moyennes annuelles de 3ᵉ) | via LSU | 22 / 37 | moyennes annuelles | via LSU [S] |
| **DNB / Cyclades** (contrôle continu de 3ᵉ) | via LSU | 38 | moyennes annuelles | via LSU [V mémento] |
| **LSL** (1ʳᵉ, terminale ; lycée pro) | sortie | 23 | moyennes périodiques et annuelles | bloqué (D-2) |
| **Parcoursup** (bulletins de 1ʳᵉ et terminale) | sortie | 37 | moyennes, appréciations | bloqué (certification) ; reprise de N-1 obligatoire (FX-106) |
| **Reprise de N-1** depuis Charlemagne ou Pronote | entrée | 9 (X3) | résultats et moyennes de période en **résultats externes** structurés | parcours minimal (tableur, mappage) |
| **Import de notes par un professeur** (tableur) | entrée | 21 | dans **ses** évaluations, à sa portée (M-447) | V3 |
| **Notes d'un autre établissement** (transfert) | entrée | 21 | résultats externes avec pièce | V5 |

Règles : tout export passe par le moteur du socle 9 (pré-contrôle, instantané, trace, retour) ; « en attente » non résolu en fin de 3ᵉ = **erreur bloquante** du pré-contrôle (R-03) ; un export n'est jamais produit depuis des données non arrêtées ; **un professeur n'exporte jamais** (D-022).

**Cohabitation de la première année (Q-X34 du socle 9, Q-EV11)** : si l'établissement pilote garde son logiciel agréé pour LSL et Parcoursup l'an 1, ScolIA ne peut pas lui transmettre les notes automatiquement [H : pas d'import connu dans Charlemagne] : double saisie. C'est la décision la plus lourde du socle (§28).

**[ajout personas] Cohabitation outillée de l'an 1 (bloquant B de `PERSONAS_21.md`, appliqué par défaut, D-031 ; D-032 a gardé notes et bulletins dans Charlemagne).** Ce n'est plus seulement « double saisie » : ScolIA tient l'appel, les justifications et les **droits fins des familles** ; Charlemagne tient notes et bulletins. Les échanges se font dans les deux sens, par le secrétariat, sous la forme du **journal à reporter** du socle 9 (R-X24, FX-107) :

| Flux | Sens | Contenu | Format | Lot |
|---|---|---|---|---|
| Assiduité de la période | ScolIA → Charlemagne | par élève : demi-journées d'absence justifiées et non justifiées, retards, bornes de la période (R-V31) | tableur lisible pour la saisie ; format d'import de Charlemagne s'il existe **[NV : à lire sur l'exemple F-07]** | V14 (EVA-233) |
| Bulletins PDF | Charlemagne → ScolIA | un PDF par élève (ou par classe, découpé) | dépôt en lot, rapprochement INE → nom + date de naissance, file « à rapprocher » (CE-89) | V14 (EVA-234) |
| Distribution des bulletins | ScolIA → familles | exemplaire de chaque lien `recevoir_bulletins`, coffre ou courrier, élève majeur, exclusions | moteur du socle 7 (le PDF de Charlemagne est rangé tel quel, un exemplaire par destinataire) | V14 (EVA-235) |
| Moyennes et appréciations de période | Charlemagne → ScolIA | moyennes de matière par élève, appréciations | export tableur de Charlemagne [H], en **résultats externes de période**, lecture seule | V14 (EVA-236) |
Règles : les bulletins de Charlemagne **ne sont jamais distribués par ScolIA sans rapprochement certain** ; la diffusion par le portail de l'éditeur est désactivée pour les bulletins (CE-90) ; les moyennes importées servent à l'historique, aux alertes du §13.3 et à la reprise de l'an 2 (FX-106), jamais à un document officiel ; aucune saisie de note dans ScolIA pour ce lycée l'an 1.

## 17. Actions métier (EVA-100 et suivantes)

Notation des acteurs : PRF professeur, PP professeur principal, DIR direction, ADF secrétariat / administratif, CPE, ELV élève, PAR responsable, AESH, SYS système. ★ = action sensible (récapitulatif avant / après, confirmation ; ré-authentification quand elle sera branchée). Les droits sont ceux du §14.1.


### 17.1 Paramétrer (direction, secrétariat)

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-100 | Choisir les règles de calcul de l'établissement (échelle, précision d'affichage, méthode de moyenne annuelle, pas de saisie) | DIR · `evaluations.parametrer` ★ | méthode annuelle figée après le premier arrêt (§8.5) ; aperçu de l'effet sur l'an passé | `evaluations.regles_modifiees` [F] |
| EVA-101 | Définir les coefficients de matière par classe, MEF, niveau ou défaut | DIR, ADF (délégation) · idem | changement daté, sans effet sur les périodes arrêtées (CE-53) ; reprise des coefficients de l'an passé proposée | `evaluations.coefficients_modifies` |
| EVA-102 | Marquer une matière « hors moyenne générale » | DIR · idem | vie de classe, options facultatives, dispositifs | idem |
| EVA-103 | Définir les coefficients de sous-matières | DIR · idem | sous-matières du socle 5 (338) | idem |
| EVA-104 | Publier le **projet d'évaluation** (lycée) aux élèves et aux familles | DIR · idem | document du socle 7, version datée ; règles du §8 affichées à côté (R-15) | `evaluations.projet_publie` |
| EVA-105 | Définir la liste des **natures d'évaluation** (DS, interrogation, DM, oral, TP, projet, évaluation commune, examen blanc…) | DIR · idem | préréglage de 8 natures ; données de l'établissement | — |
| EVA-106 | Fixer le délai maximal de publication (facultatif) | DIR · idem | S-111 | idem |
| EVA-107 | Choisir ce que voient les familles : statistiques de classe, médiane | DIR · idem | seuil de 5 non désactivable (R-V12) | idem |
| EVA-108 | Choisir l'arrêt automatique à la date d'arrêt | DIR · idem | défaut oui (S-109) | idem |
| EVA-109 | Régler les seuils des alertes précoces (trois positions) | DIR · idem | S-118 | idem |
| EVA-110 | Fixer la limite de longueur des appréciations par modèle | DIR · idem | dans les bornes du modèle de bulletin | idem |
| EVA-111 | Copier les réglages de l'année précédente à la préparation de rentrée | DIR · idem | aperçu des différences ; appelé par le chantier de rentrée du socle 5 | `evaluations.reglages_reconduits` |

### 17.2 Créer et organiser les évaluations (professeur)

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-112 | Créer une évaluation depuis la **liste unique de ses enseignements** (D-022 bis) | PRF · `evaluations.saisir` | cours actif à la date ; période ouverte de la classe ; barème, coefficient, nature, date | `evaluations.creee` |
| EVA-113 | Créer une évaluation **depuis une séance** de l'EDT | PRF · idem | séance du cours (socle 10) ; date et public pris de la séance | idem |
| EVA-114 | Créer une évaluation **depuis un travail** du cahier de textes (« sera noté ») | PRF · idem | brouillon relié au travail (socle 20) | idem |
| EVA-115 | Créer une évaluation **par positionnement** (sans note, compétences) | PRF · idem | items du socle 22 ; aucune moyenne chiffrée | idem |
| EVA-116 | Créer une évaluation pour **plusieurs de ses cours** en un geste (même sujet) | PRF · idem | une évaluation par cours, reliées | idem |
| EVA-117 | Programmer une évaluation à venir (calendrier) | PRF · idem | visible élève, famille, AESH comme « évaluation prévue » si le professeur l'annonce ; charge de travail (socle 20) | `evaluations.annoncee` |
| EVA-118 | Modifier titre, date, nature, coefficient, barème | PRF · idem | barème : conversion proposée (CE-39) ; date : même période ou refus ; historisé | `evaluations.modifiee` |
| EVA-119 | Déplacer une évaluation vers un autre de ses cours (non publiée) | PRF · idem | CE-51 | idem |
| EVA-120 | Marquer une évaluation « compte si elle améliore » ou « ne compte pas » | PRF · idem | S-103 | idem |
| EVA-121 | Autoriser les points au-delà du barème sur une évaluation | PRF · idem | si l'établissement l'a permis (S-104) | idem |
| EVA-122 | Restreindre une évaluation à une partie du public (demi-groupe, élèves volontaires) | PRF · idem | les autres élèves `non_concerne`, avec motif interne | idem |
| EVA-123 | Annuler une évaluation (avec motif) | PRF · idem ; après publication : DIR | CE-40 ; résultats gardés, exclus | `evaluations.annulee` |
| EVA-124 | Supprimer une évaluation **sans aucun résultat** | PRF · idem | jamais si un résultat existe | `evaluations.supprimee` |
| EVA-125 | Dupliquer une évaluation de l'an passé (modèle) | PRF · idem | sans résultats | idem création |
| EVA-126 | Relier des compétences à une évaluation | PRF · `competences.evaluer` | socle 22 | — |
| EVA-127 | Joindre le sujet et le corrigé (documents) | PRF · idem | socle 7 ; corrigé publié avec les notes si choisi | — |

### 17.3 Saisir les résultats

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-128 | Saisir en grille au clavier (flèches, Entrée, Tab ; codes « abs », « disp », « nr », « nn », « ar ») | PRF · `evaluations.saisir` | public à la date ; barème ; pas de saisie ; enregistrement continu | `evaluations.resultats_saisis` [F] |
| EVA-129 | Coller une colonne depuis un tableur | PRF · idem | rapprochement par nom puis ordre (CE-55) ; aperçu avant écriture | idem |
| EVA-130 | Importer un fichier tableur dans ses évaluations | PRF · idem | M-447 ; même chemin que le collage ; à sa portée | idem |
| EVA-131 | Saisir sur le téléphone (une évaluation, élève par élève) | PRF · idem | socle 11 ; hors ligne avec version (CE-56) | idem |
| EVA-132 | Accepter les absents proposés par l'appel | PRF · idem | jamais d'écrasement d'une note (CE-30) | idem |
| EVA-133 | Appliquer les dispenses actives (pré-rempli) | SYS · — | depuis 34/35, sans motif | idem |
| EVA-134 | Programmer un rattrapage pour un ou plusieurs élèves | PRF · idem | date, lieu, modalité ; avis à l'élève et aux liens `voir_notes` | `evaluations.rattrapage_programme` |
| EVA-135 | Noter un rattrapage | PRF · idem | le résultat `a_rattraper` devient `note`, historique gardé | `evaluations.resultats_saisis` |
| EVA-136 | Saisir un commentaire visible par l'élève sur un résultat | PRF · idem | publié avec la note | idem |
| EVA-137 | Saisir une note privée (visible du seul professeur) | PRF · idem | jamais exportée ni montrée à l'IA sans choix du professeur | — |
| EVA-138 | Annuler un résultat individuel (motif) | PRF · idem | CE-49, CE-50 | idem |
| EVA-139 | Voir l'historique d'un résultat | PRF, DIR · `evaluations.lire` | qui, quand, avant, après | — |
| EVA-140 | Résoudre un conflit de version (deux saisies) | PRF · idem | CE-54 | — |
| EVA-141 | Voir la grille de toute la période (toutes ses évaluations, moyennes provisoires) | PRF · `evaluations.lire` | moyennes expliquées au survol | — |

### 17.4 Publier

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-142 | Publier maintenant | PRF · `evaluations.publier` | confirmation si des cases sont vides | `evaluations.publiee` [F] |
| EVA-143 | Programmer la publication (date et heure) | PRF · idem | heure locale de l'établissement | `evaluations.publication_programmee` |
| EVA-144 | Publier à l'arrêt des notes | PRF · idem | défaut possible du professeur | — |
| EVA-145 | Retirer une publication (24 h) puis par la direction | PRF ; DIR | message « retiré pour vérification » | `evaluations.publication_retiree` |
| EVA-146 | Publier plusieurs évaluations ensemble | PRF · idem | un seul avis groupé | idem |
| EVA-147 | Publication automatique au délai maximal | SYS | sauf « en cours de correction » ; rappel au professeur avant | idem |
| EVA-148 | Publication automatique de tout à l'arrêt | SYS | R-V9 | idem |

### 17.5 Consulter (professeur, PP, CPE, direction)

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-149 | Voir les moyennes et statistiques de son cours (médiane, distribution) | PRF · `evaluations.lire` | — | — |
| EVA-150 | Voir l'explication d'une moyenne | PRF, PP, DIR | moteur unique | — |
| EVA-151 | Voir la vue de classe (toutes matières) | PP, DIR · `evaluations.lire` (classe) | — | — |
| EVA-152 | Voir, dans la fenêtre du conseil, les autres matières des élèves de ses groupes | PRF | §14.3 ; journalisé | — |
| EVA-153 | Voir le calendrier des évaluations d'une classe (charge) | PRF, PP, DIR | sans notes | — |
| EVA-154 | Voir les alertes précoces de sa classe | PP, CPE, DIR · `evaluations.suivre_classe` | §13.3 | — |
| EVA-155 | Voir ses propres alertes de cours | PRF | ses cours seulement | — |
| EVA-156 | Comparer deux évaluations communes (examen blanc) | PRF concernés, DIR | CE-46 | — |
| EVA-157 | Voir le suivi d'un élève sur l'année (évolution par matière) | PP, DIR ; PRF pour ses cours | — | — |
| EVA-158 | Voir l'historique de l'élève sur plusieurs années (bulletins) | PP, DIR, CPE (moyennes) | bulletins figés ; jamais les notes détaillées d'années closes au professeur | — |

### 17.6 Consulter (famille, élève)

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-159 | Voir les résultats publiés et les commentaires | ELV · soi ; PAR · `voir_notes` | — | — |
| EVA-160 | Voir les moyennes provisoires et leur explication | ELV, PAR | marquées « provisoires » | — |
| EVA-161 | Voir la moyenne de classe, min, max (si l'établissement l'autorise) | ELV, PAR | seuil de 5 | — |
| EVA-162 | Voir les évaluations annoncées et les rattrapages à faire | ELV, PAR | — | — |
| EVA-163 | Voir les dispenses et statuts appliqués | ELV, PAR | sans motif | — |
| EVA-164 | Voir le projet d'évaluation et les coefficients | ELV, PAR | R-15 | — |
| EVA-165 | Voir ses bulletins (coffre) | ELV ; PAR · `recevoir_bulletins` | socle 23 / 7 | — |
| EVA-166 | Demander la révision d'une note au professeur | ELV, PAR | message tracé, réponse attendue ; pas de recours juridique (R-24) | `evaluations.revision_demandee` |
| EVA-167 | Choisir ses avis (résumé quotidien, hebdomadaire) | ELV, PAR | socle 8 | — |
| EVA-168 | Élève majeur : choisir ce que ses parents continuent de voir | ELV majeur | socle 4 (Q-A7) | `liens.droits_modifies` |

### 17.7 Statuts spéciaux, rattrapages, remplacement

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-169 | Proposer une évaluation dédiée à un élève absent souvent | PRF | CE-27 | idem création |
| EVA-170 | Mettre une moyenne de période « en attente » | PRF (son cours), DIR | R-02 ; motif interne | `evaluations.moyenne_en_attente` |
| EVA-171 | Marquer une moyenne « non significative » | DIR | CE-04 | idem |
| EVA-172 | Convoquer à une **évaluation de remplacement** de fin d'année | DIR · `evaluations.saisir_tout` ★ | R-01 ; convocation (socle 7, 8) | `evaluations.remplacement_convoque` |
| EVA-173 | Saisir le résultat d'une évaluation de remplacement | PRF désigné, DIR | remplace la moyenne annuelle non représentative (R-15) | idem |
| EVA-174 | Attribuer 0 après absence non justifiée à l'évaluation de remplacement | DIR ★ | seul cas (R-01, CE-29) ; référence affichée | idem |
| EVA-175 | Reconvoquer après absence justifiée | DIR | CE-28 | idem |
| EVA-176 | Saisir un résultat externe (autre établissement, CNED, étranger) | ADF, DIR · `evaluations.resultats_externes` | source, pièce, date ; non compté par défaut | `evaluations.resultat_externe_saisi` |
| EVA-177 | Décider de compter un résultat externe dans sa moyenne | PRF du cours | geste explicite, tracé | idem |

### 17.8 Période et arrêt des notes

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-178 | Voir l'état de saisie (par classe, cours, professeur) | DIR, PP · `evaluations.piloter` | — | — |
| EVA-179 | Relancer des professeurs (message prérempli) | DIR, PP · idem | aperçu, confirmation | `evaluations.relance` |
| EVA-180 | Arrêter les notes d'une classe ou d'un niveau | DIR · `evaluations.periodes_gerer` ★ | aperçu : publications automatiques, cases vides | `evaluations.notes_arretees` |
| EVA-181 | Arrêt automatique à la date d'arrêt | SYS | heure locale de l'établissement | idem |
| EVA-182 | Fixer une date d'arrêt propre à une classe | DIR · idem | CE-41 | `evaluations.date_arret_modifiee` |
| EVA-183 | Autoriser un rattrapage après l'arrêt | DIR · idem | jusqu'à la veille du conseil (R-V10) | `evaluations.rattrapage_autorise` |
| EVA-184 | Clore les appréciations | DIR · idem ; SYS la veille du conseil | — | `evaluations.appreciations_closes` |
| EVA-185 | Rouvrir une période ★ | DIR · idem | motif, durée, classes ; corrections journalisées | `evaluations.periode_rouverte` |
| EVA-186 | Voir l'instantané d'arrêt et ses versions | DIR, PP | empreinte, écarts entre versions | — |
| EVA-187 | Clore la période (après bulletins) | SYS sur publication des bulletins (socle 23) ; DIR | — | `evaluations.periode_close` |
| EVA-188 | Contrôle de fin d'année de 3ᵉ (moyennes annuelles chiffrées) | SYS, DIR | R-03, CE-63 | `evaluations.controle_3e` |

### 17.9 Corrections

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-189 | Demander une correction après l'arrêt | PRF | motif | `evaluations.correction_demandee` |
| EVA-190 | Valider ou refuser une demande | DIR · `evaluations.corriger` ★ | effet sur moyennes affiché | `evaluations.corrigee` [F `note.corrigee`] |
| EVA-191 | Corriger directement | DIR ★ | idem | idem |
| EVA-192 | Corriger une appréciation après l'arrêt | auteur + DIR ★ | — | `evaluations.appreciation_corrigee` |
| EVA-193 | Voir le journal des corrections | DIR | export pour un recours | — |

### 17.10 Appréciations

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-194 | Saisir l'appréciation de cours | PRF · `appreciations.saisir` | §12 ; versions | `evaluations.appreciation_saisie` [F `note.appreciation`] |
| EVA-195 | Saisir les appréciations en série (élève suivant au clavier, notes à côté) | PRF · idem | — | idem |
| EVA-196 | Gérer sa banque de phrases | PRF | personnelle | — |
| EVA-197 | Demander des propositions à l'IA pour un élève | PRF · idem | §13.1 ; brouillon | `ia.brouillon_cree` |
| EVA-198 | Demander des propositions pour tout un groupe | PRF · idem | une par élève, à relire une à une (R-V19) | idem |
| EVA-199 | Valider, modifier ou rejeter une proposition | PRF | — | idem saisie |
| EVA-200 | Voir les contrôles de cohérence de ses appréciations | PRF | §12.2 | — |
| EVA-201 | Saisir l'appréciation générale | PP · `appreciations.generale` | souvent pendant le conseil | `evaluations.appreciation_generale` [F] |
| EVA-202 | Demander la synthèse des appréciations d'un élève (IA) | PP | §13.4 | `ia.brouillon_cree` |
| EVA-203 | Saisir l'appréciation de vie scolaire | CPE · `appreciations.vie_scolaire` | assiduité calculée à côté | idem |
| EVA-204 | Relire les appréciations d'une classe (remarques) | PP, DIR | §13.2 ; remarques adressées aux auteurs | `evaluations.remarques_appreciation` |
| EVA-205 | Écrire « pas d'appréciation (professeur absent) » | DIR | CE-42 | idem |
| EVA-206 | Copier une appréciation intermédiaire dans la finale | PRF | S-112 | — |

### 17.11 Préparation du conseil (frontière 21 / 23)

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-207 | Produire le dossier de conseil depuis l'instantané | SYS, PP, DIR | socle 23 | `conseils.dossier_pret` |
| EVA-208 | Proposer les mentions selon les règles de l'établissement | SYS | proposition seulement | — |
| EVA-209 | Lister les élèves « à discuter » (alertes, seuils) | SYS, PP | motivé | — |
| EVA-210 | Marquer un élève pour discussion | PRF (ses élèves), PP | — | — |
| EVA-211 | Préparer une proposition de décision ou d'avis (PP) | PP | socle 23 | — |

### 17.12 Pilotage (direction)

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-212 | Tableau de fin de période de l'établissement | DIR | classes prêtes / en retard | — |
| EVA-213 | Statistiques par matière, niveau, classe, professeur | DIR | jamais publiées ; professeur voit les siennes | — |
| EVA-214 | Comparer les résultats d'une évaluation commune entre classes | DIR, professeurs concernés | — | — |
| EVA-215 | Voir la répartition des notes par nature d'évaluation | DIR | aide au projet d'évaluation | — |
| EVA-216 | Voir les écarts entre moyennes de contrôle continu et examens blancs | DIR | socle 38 | — |
| EVA-217 | Voir la charge d'évaluations par classe et par semaine | DIR, PP | avec le socle 20 | — |
| EVA-218 | Voir les évaluations non publiées depuis longtemps | DIR | délai | — |

### 17.13 Échanges et reprise

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-219 | Préparer la reprise de N-1 (moyennes et appréciations par période) | ADF · `echanges.importer` | socle 9 FX-106 ; résultats externes structurés | `echanges.lot_applique` |
| EVA-220 | Voir l'état de préparation des exports (LSU, LSL, Parcoursup) | ADF, DIR · `echanges.etat_preparation` | socle 9 | — |
| EVA-221 | Produire l'export de travail LSU, LSL ou Parcoursup | ADF, DIR · `livrets.exporter` ★ | « format de travail non officiel » tant que D-2 n'est pas obtenue | `export.realise` [F] |
| EVA-222 | Produire l'export officiel (après validation éditeur) | ADF, DIR ★ | moteur X7 | idem |
| EVA-223 | Traiter un compte rendu de rejet (tâches) | ADF | FX-08 | — |
| EVA-224 | Exporter le relevé d'un élève (PDF) | ADF, DIR ; ELV, PAR (le sien) | socle 7 | — |
| EVA-225 | Exporter une grille vers un tableur | PRF (son cours) | échappement des cellules | — |
| EVA-226 | Exporter les résultats de l'établissement (anonymisé) pour une étude interne | DIR | pseudonymisé | `export.realise` |

### 17.14 Cas particuliers et administration

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-227 | Transférer les résultats lors d'une fusion de fiches | SYS | CE-22 | — |
| EVA-228 | Geler les résultats d'un élève (décès, protection) | DIR | CE-23 | `evaluations.gel` |
| EVA-229 | Prolonger l'accès en lecture d'un professeur parti jusqu'au conseil | DIR | P37 du socle 6 | — |
| EVA-230 | Autoriser un remplaçant à modifier les évaluations du titulaire | DIR | S-115 | — |
| EVA-231 | Voir qui a consulté le dossier de conseil ou un bulletin | DIR | R-21 | — |
| EVA-232 | Recalculer et comparer (contrôle d'intégrité) les moyennes d'un instantané | DIR, SYS | empreinte | — |

### 17.15 [ajout personas] Actions ajoutées par la passe (`PERSONAS_21.md`, M-579 à M-668)

**Cohabitation de l'an 1 (bloquant B, lot V14)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-233 | Exporter l'assiduité d'une période par élève (demi-journées justifiées, non justifiées, retards, bornes) pour le logiciel de notes tiers | ADF · `echanges.exporter` | R-V31 ; journal à reporter (R-X24) ; échappement des cellules | `export.realise` |
| EVA-234 | Déposer en lot les bulletins PDF du logiciel tiers et les rapprocher des élèves | ADF · `echanges.importer` | INE, puis nom et date de naissance ; non rapprochés en file (CE-89) ; aperçu | `echanges.lot_applique` |
| EVA-235 | Distribuer les bulletins déposés selon les droits des liens | ADF, DIR · `bulletins.publier` ★ | `recevoir_bulletins` à la date de publication (R-V47) ; contrôle avant publication (EVA-246) ; lot d'impression (EVA-248) | `bulletins.publies` |
| EVA-236 | Importer en lecture les moyennes et appréciations de période du logiciel tiers | ADF · `echanges.importer` | résultats externes de période ; jamais sur un document officiel | `echanges.lot_applique` |
| EVA-237 | Afficher aux familles et aux élèves où consulter les notes pendant la cohabitation | DIR · `evaluations.parametrer` | lien vers le portail de l'éditeur, texte encadré | — |

**Période par élève (bloquant A, lot V1)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-238 | Voir, dans la grille d'un cours multi-classes, la période et l'état de chaque élève (cellule verrouillée et raison) | PRF · `evaluations.saisir` | R-V29, §9.4 | — |
| EVA-239 | Recevoir rappels d'arrêt et de clôture des appréciations par classe pour un cours multi-classes | SYS → PRF | un rappel par classe ; regroupé au résumé | `evaluations.rappel_saisie` |

**Enseignements suivis à l'extérieur (bloquant C, lots V1 et V16)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-240 | Déclarer qu'un élève suit une matière hors de l'établissement (organisme, dates, référent, compté ou non dans la moyenne générale) | ADF, DIR · `evaluations.resultats_externes` | port du socle 5 (offre) ; R-V38 ; [NV R-39] | `evaluations.enseignement_externe_declare` |
| EVA-241 | Saisir la moyenne de période externe et l'appréciation transcrite, avec la pièce | ADF · idem | rattachée à la période par sa date (CE-67) ; vaut moyenne de matière (R-V38) | `evaluations.resultat_externe_saisi` |
| EVA-242 | Relancer l'organisme externe avant l'arrêt (tâche et message préparé) | ADF | J-15 avant l'arrêt de la classe | — |

**Droits, guichet, identité**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-243 | Copier sa banque de phrases vers un autre de ses établissements | PRF (soi) | geste de la personne ; noms d'élèves retirés ; R-V35 | — |
| EVA-244 | Voir, au moment de produire un document à la main, les restrictions du lien du demandeur | SYS → ADF | R-V39 ; bandeau sans motif détaillé ; conduite à tenir | `documents.restriction_affichee` |
| EVA-245 | Appliquer le prénom d'usage aux documents internes et l'état civil aux exports | SYS | décision du socle 2 ; R-V36 ; [NV R-36] | — |

**Bulletins et secrétariat (lot V15, avec le socle 23)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-246 | Lancer le contrôle avant publication d'un lot de bulletins (bloquants, avertissements) | ADF, DIR · `bulletins.publier` | bloquants : destinataire interdit, bulletin sans moyenne ni « en attente », destinataire sans canal ; avertissements : appréciation vide, texte tronqué, prénom d'un autre élève | `bulletins.controle_realise` |
| EVA-247 | Programmer la publication des bulletins par classe ou par niveau ; « remis en main propre le …, en ligne le … » | DIR · idem ★ | heure locale ; destinataires recalculés à l'heure (R-V47) ; CE-78 | `bulletins.publication_programmee` |
| EVA-248 | Produire le lot d'impression : tri par adresse, fratries d'un même destinataire dans une enveloppe, exclusions appliquées, nombre d'envois et de pages | ADF · idem | jamais deux parents séparés dans une enveloppe (R-27) ; S-131 | `documents.lot_imprime` |
| EVA-249 | Réimprimer et renvoyer la seule version 2 aux destinataires papier de la v1 | ADF · idem | mention « bulletin rectifié » | idem |
| EVA-250 | Produire les relevés de notes en lot (classe, date) pour une rencontre parents-professeurs | ADF, PP · `evaluations.lire` (classe) | publiés seulement ; mention « relevé provisoire au … » ; droits du lien | `documents.lot_produit` |
| EVA-251 | Produire un duplicata de bulletin (toute année) | ADF · `bulletins.lire` | mention « duplicata délivré le … » ; R-V39 ; identité du demandeur (socle 7) | `bulletins.duplicata` |
| EVA-252 | Rechercher dans les archives des bulletins (nom, naissance, INE, année) | ADF · idem | journalisé (R-V26) | — |
| EVA-253 | Ancien élève : consulter son coffre de bulletins pendant un an après son départ | ELV ancien | Q-EV22 ; lecture seule | — |
| EVA-254 | Constituer le registre annuel (PDF/A de tous les bulletins par classe, exemplaire établissement) et verser les PV de conseils | ADF, SYS | DUA 50 ans par défaut (R-35) ; socle 7 | `bulletins.registre_constitue` |
| EVA-255 | Constituer le dossier de transfert d'un élève qui part | ADF · `evaluations.lire` | tout publié et figé au jour du départ (R-V42) ; R-V37 ; R-V39 | `evaluations.dossier_transfert` |

**Conseil (attentes au socle 23)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-256 | Déléguer la présidence d'un conseil ou d'une série de conseils | DIR · `conseils.deleguer` ★ | à un adjoint ou un PP ; changeable le jour même (CE-86) ; [NV R-34 sur le « représentant »] | `conseils.presidence_deleguee` |
| EVA-257 | Produire la synthèse anonyme de la classe pour le compte rendu des délégués des parents | PP, président | aucune donnée individuelle ; validée par le président | `conseils.synthese_validee` |
| EVA-258 | Émarger les présents et produire le procès-verbal du conseil | président, secrétaire de séance | archivé (R-35 [NV sur la durée]) ; « délégués sortis » tracé | `conseils.pv_produit` |

**Rattrapages collectifs (lot V4)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-259 | Organiser une séance de rattrapage collective (date, salle, surveillant, liste des élèves à rattraper toutes matières) | CPE, ADF · `evaluations.rattrapages_organiser` | élèves `absent`/`non_rendu` avec rattrapage prévu ; aménagements (tiers-temps) pris en compte ; socle 38 pour les salles | `evaluations.seance_rattrapage_creee` |
| EVA-260 | Déposer le sujet de rattrapage et recevoir les copies | PRF · `evaluations.saisir` | sujet visible du surveillant le jour même seulement | — |
| EVA-261 | Signaler au professeur qu'un élève a composé (ou manqué) au rattrapage | SYS, surveillant | statut « composé, à corriger » ; absent : CE-81 | `evaluations.rattrapage_effectue` |

**Évaluations détaillées (lot V3)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-262 | Définir un barème détaillé par exercice ou critère | PRF · `evaluations.saisir` | §8.9 ; note = somme | `evaluations.modifiee` |
| EVA-263 | Voir la réussite par exercice de l'évaluation | PRF | ses cours ; sans classement | — |
| EVA-264 | Montrer à l'élève le détail par exercice | PRF | choix du professeur, publié avec la note | — |
| EVA-265 | Verser les scores d'un QCM corrigé automatiquement comme résultats proposés | PRF | socle 20 ; validation du professeur ; CE-82 | `evaluations.resultats_saisis` |

**Aménagements et AESH (lot V4)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-266 | Voir, à la création d'une évaluation, le résumé des aménagements de son public et joindre un sujet adapté | PRF | libellés fonctionnels, sans diagnostic (port 35) ; rappel la veille (CE-85) | — |
| EVA-267 | Partager le sujet (adapté) avec l'AESH de l'élève avant l'évaluation | PRF | choix du professeur ; durée de l'accompagnement | — |
| EVA-268 | Alerter l'AESH et les professeurs de deux évaluations simultanées d'élèves accompagnés | SYS | CE-84 | `evaluations.conflit_accompagnement` |

**Élèves et familles (lot V7)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-269 | Voir la date prévue du conseil et de la publication du bulletin | ELV, PAR | socle 23 ; « date prévue » | — |
| EVA-270 | Simuler sa moyenne (« si j'ai 12 au prochain devoir ») | ELV seulement | calcul local par le moteur, rien n'est enregistré ; S-130 | — |
| EVA-271 | Voir la copie corrigée reliée au résultat | ELV ; PAR `voir_notes` | socle 20 ; publiée avec la note | — |
| EVA-272 | Lire le glossaire des mentions, statuts et statistiques | ELV, PAR | §12.3 | — |
| EVA-273 | Exporter toutes les données d'évaluation d'un élève pour une demande d'accès | DIR, DPO | RGPD art. 15 (R-37) ; notes privées comprises (R-V41) ; journalisé | `evaluations.acces_exporte` |
| EVA-274 | Voir côte à côte, dans l'appli, ses établissements (professeur sur deux établissements, famille à deux enfants) | PRF, PAR, ELV | **agrégation dans l'appli seulement**, deux sessions, aucun croisement serveur (D-013) ; établissement toujours affiché | — |

**Vie scolaire et IA (lots V8, V11)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-275 | Relire par l'IA un lot de bulletins avant publication (remarques) | ADF, DIR | §13.8 ; jamais de modification ; après l'étude juridique | `ia.remarques_bulletins` |
| EVA-276 | Voir dans le dossier du conseil « ce qui a changé depuis le dernier conseil » | PP, DIR, professeurs (fenêtre) | déterministe (§13.8) | — |
| EVA-277 | Transmettre un signalement de décrochage à la cellule de veille | CPE, PP, DIR · `evaluations.suivre_classe` | faits expliqués ; décision humaine ; socles 12, 35 | `evaluations.signalement_decrochage` |
| EVA-278 | Recevoir un résumé hebdomadaire (tous ses enfants d'un même établissement) | PAR | socle 8 ; faits déterministes ; reformulation et traduction IA facultatives | — |

**Direction (lots V8, V10)**
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| EVA-279 | Voir l'harmonisation entre classes d'un niveau (écarts par matière), sans classement des professeurs | DIR · `evaluations.piloter` | R-V44 ; journalisé | — |
| EVA-280 | Contrôle des moyennes annuelles des enseignements du baccalauréat (1ʳᵉ, terminale) avant l'arrêt de la dernière période | SYS, DIR | R-V45, R-29 ; CE-91 | `evaluations.controle_bac` |

Actions précisées sans nouveau numéro : **EVA-125** dupliquer **plusieurs** évaluations en une opération (M-667) ; **EVA-152** mode conseil de groupe (§14.6 point 8) ; **EVA-158** bulletin de l'an passé visible du PP, de la direction et de la CPE seulement (CE-80) ; **EVA-171** « en attente » appliqué en masse à une classe et une matière (M-608) ; **EVA-195** dictée vocale (V9).

## 18. Règles transverses (R-V1 et suivantes) [R]

| Id | Règle |
|---|---|
| R-V1 | Toute évaluation vise **un cours** (socle 5) et **une période** de la périodisation d'évaluation de sa classe ; aucune note hors évaluation. |
| R-V2 | Le public d'une évaluation est celui du cours **à sa date** (`ent_public_cours_jour`) ; `non_concerne` est calculé, jamais saisi. |
| R-V3 | **Un seul moteur de calcul**, exact (fractions entières), sans arrondi intermédiaire, appelé par toutes les lectures ; tout écart entre deux écrans est un défaut. |
| R-V4 | Valeurs stockées en décimal exact ; `0 ≤ note ≤ barème` sauf dérogation explicite de l'évaluation ; barème et coefficients > 0. |
| R-V5 | **Une absence n'est jamais convertie en note** ; le 0 pour absence n'existe que sur une évaluation de remplacement, après absence non justifiée, par la direction [V R-01]. |
| R-V6 | Un résultat ne se supprime pas : il se remplace (historique) ou s'annule (motif). |
| R-V7 | Rien n'est visible d'une famille ou d'un élève sans publication. |
| R-V8 | Les appréciations deviennent visibles avec le bulletin publié (sauf appréciations intermédiaires choisies, S-112). |
| R-V9 | À l'arrêt des notes, toutes les évaluations de la période sont publiées ; relevé et bulletin comptent les mêmes évaluations. |
| R-V10 | Après l'arrêt, une saisie n'est possible que comme rattrapage autorisé ou correction motivée par la direction. |
| R-V11 | L'instantané d'arrêt est versionné et porte une empreinte ; le conseil et le bulletin citent la version utilisée. |
| R-V12 | Aucune statistique de groupe n'est montrée à une famille ou à un élève sous 5 élèves notés. |
| R-V13 | Le professeur voit les élèves de ses cours pour les dates de son service (D-022) ; au-delà, seulement dans la fenêtre du conseil, pour ces élèves (§14.3). |
| R-V14 | Famille : droits du **lien** à la date (`voir_notes`, `recevoir_bulletins`) ; élève majeur : son choix (socle 4). |
| R-V15 | Un professeur garde l'écriture des appréciations des élèves qui ont quitté son cours dans la période, jusqu'à la clôture des appréciations. |
| R-V16 | Une dispense ou un aménagement est affiché sans motif ; un aménagement n'est jamais imprimé sur un document destiné à un tiers (D-027 B). |
| R-V17 | Les événements ne portent ni note, ni appréciation, ni motif (P3 du socle 8). |
| R-V18 | Une information de santé apparente dans une appréciation déclenche un avertissement à l'auteur avant validation. |
| R-V19 | Une proposition de l'IA n'est validée qu'ouverte et relue, une par une. |
| R-V20 | Aucune décision (mention, passage, orientation, sanction) n'est produite automatiquement à partir d'une moyenne. |
| R-V21 | La méthode de moyenne annuelle et l'échelle sont figées pour l'année dès le premier arrêt. |
| R-V22 | En 3ᵉ : note sur 20 pour chaque enseignement obligatoire et chaque bilan ; moyenne annuelle chiffrée obligatoire en fin d'année, sauf dispense réglementaire [V R-03]. |
| R-V23 | Un changement de coefficient ou de règle est daté et ne touche jamais une période arrêtée. |
| R-V24 | Un professeur n'exporte jamais de livret (D-022). |
| R-V25 | Les corrections après l'arrêt et les réouvertures sont journalisées en ajout seul et consultables par la direction. |
| R-V26 | Les consultations de dossiers de conseil et de bulletins par le personnel sont journalisées. |
| R-V27 | La saisie hors ligne est refusée si la période a été arrêtée entre-temps ; jamais d'écrasement silencieux. |
| R-V28 | Le bulletin est celui de la classe de rattachement de l'élève à la date d'arrêt ; les notes d'une ancienne classe y figurent avec leur origine. |
| R-V29 | **[ajout personas]** La **période et l'état** d'un résultat se résolvent **par élève** : classe de rattachement à la date de l'évaluation → périodisation → période → état de cette classe ; une même évaluation peut être ouverte pour certains élèves et arrêtée pour d'autres (§9.4). Précise R-V1. |
| R-V30 | **[ajout personas]** Pour un enseignement en groupe multi-classes, la statistique de comparaison au bulletin est la **moyenne du groupe** (élèves notés du cours, seuil de 5), libellée comme telle. |
| R-V31 | **[ajout personas]** L'**assiduité** au bulletin compte du premier jour de la période à la **date d'arrêt effective** de la classe ; les bornes sont imprimées ; demi-journées justifiées et non justifiées distinguées (R-38). |
| R-V32 | **[ajout personas]** **Aucune sanction disciplinaire** n'est imprimée sur un bulletin ni portée dans l'instantané destiné aux familles ; la vie scolaire au bulletin = assiduité, ponctualité, appréciation de la CPE. |
| R-V33 | **[ajout personas]** **Aucune rétention** d'un bulletin, d'un relevé ou d'un duplicata pour un motif financier ; les socles 24 et 25 n'ont aucun accès à la publication des documents d'évaluation. |
| R-V34 | **[ajout personas]** Les **appréciations** et les **décisions** du conseil suivent le droit `recevoir_bulletins` du lien, pas `voir_notes`. |
| R-V35 | **[ajout personas]** Banque de phrases, préférences de style et exemples de l'IA sont rangés par **(personne, établissement)** ; aucun texte d'un établissement n'est lu dans un autre sans geste de la personne ; les exemples sont pseudonymisés (D-013). |
| R-V36 | **[ajout personas]** Les documents internes (bulletin, relevé, appréciations, propositions IA) portent le **prénom d'usage** décidé par l'établissement (socle 2) ; les exports officiels et documents à un tiers portent l'état civil [NV R-36]. |
| R-V37 | **[ajout personas]** La rubrique « accompagnements » (PAP, PPS, PPRE, ULIS, UPE2A, SEGPA) n'est **jamais** transmise à un tiers (Parcoursup, dossier d'admission, relevé de transfert) ; seulement à la famille et au LSU officiel. |
| R-V38 | **[ajout personas]** Une **moyenne externe** (enseignement suivi hors de l'établissement, déclaré par la direction) vaut moyenne de matière ; sa source est imprimée ; elle est soumise aux mêmes contrôles (R-V22, R-V45). |
| R-V39 | **[ajout personas]** Toute **production manuelle** d'un document d'évaluation (relevé, duplicata, dossier de transfert, réponse à une demande) relit les droits du lien à l'instant et affiche les restrictions à l'agent. |
| R-V40 | **[ajout personas]** L'IA ne corrige aucune copie, ne propose aucune note, ne détecte pas la fraude, ne forme aucun groupe d'élèves et ne propose ni mention, ni avis, ni décision (annexe III point 3 b et d [V* R-33]). |
| R-V41 | **[ajout personas]** Une **note privée** du professeur est une donnée de l'élève, communicable au titre du droit d'accès ; l'écran de saisie le dit. |
| R-V42 | **[ajout personas]** Au **départ** d'un élève, toutes ses évaluations de la période sont publiées et figées pour lui ; le dossier de transfert est produit depuis cet état. |
| R-V43 | **[ajout personas]** Un parent **sans exercice** de l'autorité parentale reçoit par défaut les **bulletins** et les **décisions d'orientation** (C. civ. 373-2-1 [V R-25]) mais pas les notes au fil de l'eau ; rien en cas de **retrait** (R-26) ou de décision de justice contraire ; la direction peut élargir ou restreindre, motif tracé. |
| R-V44 | **[ajout personas]** Les statistiques **par professeur** et les comparaisons entre groupes d'une évaluation commune sont réservées à la direction ; un professeur voit ses groupes et la moyenne **anonyme** des autres ; jamais de classement ; consultations journalisées. |
| R-V45 | **[ajout personas]** Au lycée, les **moyennes annuelles** des enseignements du baccalauréat sont contrôlées avant l'arrêt de la dernière période de 1ʳᵉ et de terminale, comme en 3ᵉ (R-V22) ; une moyenne manquante appelle l'évaluation de remplacement (R-29). |
| R-V46 | **[ajout personas]** Un parent ne voit **jamais l'activité de l'autre parent** (lecture, accusé, demande de révision, réponse reçue). |
| R-V47 | **[ajout personas]** Les **destinataires** d'un bulletin (et d'un avis de décision) sont calculés **à la publication**, pas à la génération : majorité, nouveau lien, droit retiré entre-temps sont pris en compte. |

---

## 19. Événements (outbox, dans la transaction du fait)

**Émis** : `evaluations.creee`, `.modifiee`, `.annulee`, `.supprimee`, `.annoncee`, `.resultats_saisis` [F], `.publiee` [F], `.publication_programmee`, `.publication_retiree`, `.rattrapage_programme`, `.moyenne_en_attente`, `.remplacement_convoque`, `.resultat_externe_saisi`, `.notes_arretees`, `.date_arret_modifiee`, `.rattrapage_autorise`, `.appreciations_closes`, `.periode_rouverte`, `.periode_close`, `.controle_3e`, `.correction_demandee`, `.corrigee` (succède à `note.corrigee` [F]), `.appreciation_saisie` (succède à `note.appreciation` [F]), `.appreciation_generale` [F], `.appreciation_corrigee`, `.remarques_appreciation`, `.revision_demandee`, `.regles_modifiees` [F], `.coefficients_modifies`, `.projet_publie`, `.reglages_reconduits`, `.relance`, `.gel`. Charges : identifiants et dates seulement (R-V17).
**Consommés** : `referentiel.periode_modifiee`, `periodisation_assignee` (5) → contrôle « évaluations hors période » ; `referentiel.cours_membre_ajoute/_annule`, `affectation_corrigee` (5) → recalcul du public ; `services.intervention_*` (6) → droits ; `absences.*` (12) → propositions d'absents ; `sante.inaptitude_*` (34), `accompagnement.*` (35) → dispenses, aménagements ; `eleve.majeur`, `liens.*` (2, 4) → droits ; `eleve.decede` (2) → gel ; `eleve.fusionne` (2) → transfert ; `conseils.tenu`, `bulletins.publies` (23) → états de période ; `cahier.travail_note` (20) → brouillon d'évaluation.

---

## 20. Écrans clés

| Écran | Où | Pour qui | Ce qu'il montre |
|---|---|---|---|
| **Mes évaluations** | `/web/evaluations` | professeur | liste unique de ses enseignements, évaluations de la période, état (brouillon, à corriger, à publier, publiée), rappels d'arrêt |
| **Grille** | `/web/evaluations/grille` [F, à refondre] | professeur | grille au clavier, collage, codes, absents proposés, moyennes provisoires expliquées, publication |
| **Appréciations** | `/web/evaluations/appreciations` | professeur, PP, CPE | élève par élève : notes à gauche, appréciation au centre, propositions IA à droite, compteur, contrôles |
| **Fin de période** | `/web/evaluations/pilotage` | direction, PP | état de saisie par classe et par professeur, relance, arrêt, rattrapages autorisés, réouvertures |
| **Vue de classe** | `/web/evaluations/classes/{id}` [F] | PP, direction | tableau élèves × matières, alertes, évolution |
| **Corrections** | `/web/evaluations/corrections` | direction | demandes, effet sur moyennes, journal |
| **Réglages** | `/web/evaluations/reglages` | direction | règles, coefficients (tableau classes × matières), natures, projet d'évaluation |
| **Relevé** | `/web/eleves/{id}/notes` [F `famille.rs`] | élève, famille | résultats publiés, moyennes provisoires et explication, rattrapages, évaluations annoncées |
| **Mobile** | appli (socle 11) | professeur ; élève, famille | saisie d'une évaluation ; relevé, avis |
Conseil et bulletins : socle 23 (écrans existants à garder, `conseil.rs`, `bulletins.rs` [F]). Exigences : clavier complet, 375 px, lecteur d'écran, grille mesurée à 40 élèves × 20 évaluations sans latence perceptible (`FRONTEND_LEPTOS.md`).

---

## 21. Modèle de données cible (brique `evaluations`)

```
eva_reglage                     -- un par (établissement, année) ; remplace ent_eva_regle (180)
  etablissement_id, annee_id, echelle NUMERIC(5,2) DEFAULT 20, precision ('centieme','dixieme'),
  pas_saisie NUMERIC(4,3) DEFAULT 0.01, methode_annuelle ('moyenne_periodes','toutes_evaluations'),
  delai_publication_max_jours NULL, stats_familles BOOLEAN, mediane_familles BOOLEAN,
  arret_auto BOOLEAN DEFAULT true, seuils_alertes ('souple','normal','strict'), fige_le NULL, version
eva_coefficient_matiere         -- succède à ent_eva_coef_matiere (180) ; cible la plus précise gagne
  id, etablissement_id, annee_id, matiere_id, classe_id NULL | mef_id NULL | niveau_id NULL | defaut,
  coef NUMERIC(6,3), hors_moyenne_generale BOOLEAN, valable_du DATE, valable_au DATE NULL
eva_coefficient_sous_matiere    matiere_id (sous-matière), annee_id, coef
eva_nature                      id, etablissement_id, code, libelle, ordre, archivee_le

eva_evaluation                  -- succède à ent_devoir (022, 180)
  id, etablissement_id, annee_id, cours_id (socle 5), periode_id (ent_periode, socle 5), date DATE,
  titre, nature_id, mode ('note','positionnement'), bareme NUMERIC(7,3) NULL, coef NUMERIC(6,3),
  comptage ('normal','si_ameliore','ne_compte_pas'), hors_bareme_autorise BOOLEAN,
  remplacement BOOLEAN,           -- évaluation de remplacement (R-01)
  commune_id NULL, seance_id NULL (10), travail_id NULL (20), matiere_id NULL (sous-matière),
  etat ('brouillon','ouverte','annulee'), motif_annulation NULL,
  publication ('explicite','programmee','a_l_arret'), publication_prevue_le NULL, publiee_le NULL, publiee_par,
  annoncee BOOLEAN, cree_par, cree_le, version
  FK composées (etab, cours), (etab, periode)
eva_evaluation_commune          id, etablissement_id, annee_id, libelle, date, bareme ; (lien socle 38)
eva_evaluation_restriction      evaluation_id, eleve_id, motif_code   -- demi-groupe, volontaires
eva_resultat                    -- succède à ent_note (008, 022, 180)
  id, etablissement_id, evaluation_id, eleve_id,
  statut ('note','absent','a_rattraper','dispense','non_rendu','non_note','annule'),
  valeur NUMERIC(7,3) NULL CHECK (statut='note') = (valeur IS NOT NULL), amenage BOOLEAN,
  commentaire_eleve NULL, note_privee NULL, rattrapage_le NULL, source ('saisie','appel','dispense','import','hors_ligne'),
  version INT, saisi_par, saisi_le
  UNIQUE (evaluation_id, eleve_id)   -- D8
eva_resultat_historique         -- ajout seul : chaque version (avant/après, auteur, moment, motif si correction)
eva_resultat_externe            id, etablissement_id, eleve_id, matiere_id, periode_id NULL, annee_libelle,
                                origine (établissement, CNED, étranger, reprise), valeur, bareme, coef, compte_dans_cours_id NULL,
                                document_id NULL (7), saisi_par
eva_moyenne_statut              eleve_id, cours_id | matiere_id, periode_id, statut ('en_attente','non_significative'), motif_interne, par
eva_periode_etat                -- succède à ent_note_periode (022)
  etablissement_id, periode_id, classe_id, etat ('ouverte','notes_arretees','appreciations_closes','conseil_tenu','close'),
  date_arret_effective DATE, arretee_le, par ; machine d'états par déclencheur
eva_periode_transition          -- ajout seul : réouvertures, motifs, durées
eva_rattrapage_autorise         evaluation_id, eleve_id, jusqu_au, par
eva_instantane                  etablissement_id, classe_id, periode_id, version, contenu JSONB, empreinte, motif NULL, pris_le, par
                                (immuable par déclencheur, sur le modèle de ent_bulletin 182)
eva_appreciation                -- succède à ent_note_appreciation (056)
  id, etablissement_id, periode_id, eleve_id, type ('cours','generale','vie_scolaire','intermediaire'),
  cours_id NULL, texte, etat ('brouillon','validee'), aide_ia BOOLEAN, version, auteur, maj_le
eva_appreciation_historique     -- ajout seul
eva_banque_phrase               user_id, texte, etiquettes
eva_correction_demande          resultat_id | appreciation_id, demandeur, motif, etat, decide_par, decide_le
eva_revision_demande            resultat_id, demandeur (personne), message, reponse, etat
eva_consultation                -- journal des consultations (R-V26), ajout seul
```
Toutes les tables : `etablissement_id` non nul, clés étrangères composées, **RLS forcée** dès la création. Les tables du conseil, des bulletins et des exports (071, 181, 182) restent celles du socle 23, corrigées par V0 puis par la conception du 23. Les compétences (088, `ent_bilan_cycle`) sont au socle 22.

### 21.1 [ajout personas] Compléments de modèle (`PERSONAS_21.md`)
```
eva_evaluation.periode_id        -- devient « période indicative » (tri de l'écran) ; aucun contrôle ne la lit (§9.4)
evaluations::periode_eleve()     -- port : (évaluation, élève) → (classe à la date, période, état) ; version ensembliste pour la grille
eva_bareme_item                  evaluation_id, ordre, libelle, maximum NUMERIC(7,3), annule BOOLEAN   -- §8.9
eva_resultat_item                resultat_id, item_id, valeur NUMERIC(7,3)                             -- somme = eva_resultat.valeur
eva_enseignement_externe         id, etablissement_id, eleve_id, matiere_id, organisme ('cned','etablissement','conservatoire','etranger','autre'),
                                 organisme_libelle, du, au NULL, referent NULL, compte_moyenne_generale BOOLEAN, declare_par
eva_moyenne_externe              id, etablissement_id, enseignement_externe_id, periode_id, valeur NULL, bareme, appreciation_transcrite NULL,
                                 statut ('moyenne','en_attente','dispense'), document_id NULL (7), saisi_par   -- R-V38
eva_banque_phrase                -- clé (user_id, etablissement_id) au lieu de user_id seul (R-V35, C9)
eva_style_exemple                user_id, etablissement_id, texte_pseudonymise, consenti_le                   -- §13.8
eva_import_tiers                 id, etablissement_id, source ('charlemagne', …), nature ('bulletins_pdf','moyennes'), periode_id,
                                 depose_par, depose_le, etat ; eva_import_tiers_ligne (rapprochement eleve_id NULL, motif)   -- V14
eva_rattrapage_seance            id, etablissement_id, date, debut, fin, salle_id NULL, surveillant_id NULL
eva_rattrapage_inscription       seance_id, resultat_id, statut ('convoque','compose','absent'), sujet_document_id NULL       -- V4
eva_sujet_partage                evaluation_id, eleve_id (accompagné), aesh_user_id, du, au                    -- EVA-267
eva_appreciation.type            + 'dispositif' (§12.3)
```
Côté socle 23 (attentes, tables à concevoir là-bas) : publication programmée et remise en main propre par classe ou niveau ; délégation de présidence ; PV et présents ; registre annuel ; duplicatas. Côté socle 7 : lot d'impression par adresse. Les compléments entrent dans la réserve **5108–5189** (repli **468–477**).

---

## 22. Plan de migration depuis l'existant (sans casser l'appel, le mobile ni le labo)

| Étape | Contenu | Compatibilité | Contrôle |
|---|---|---|---|
| M1 (V0) | Fermeture des routes historiques d'écriture (D1, D2, D4, D5) ; filtres de publication sur les lectures historiques (D3) ; RLS permissive ; dédoublonnage et unicité `ent_note (devoir_id, eleve_id)` ; `absent_compte_zero` neutralisé ; libellés « mise en garde » | les lectures historiques (mobiles, UI JS) restent ouvertes **filtrées** jusqu'à la migration des clients | tests de fuite (parent sans droit, appréciation avant publication) |
| M2 (V1) | Création des tables `eva_*` ; reprise : `ent_devoir` → `eva_evaluation` (cours trouvé par `service_id` → `cours_id` du socle 5, ou par classe et matière ; à défaut, cours « reprise » signalé) ; `ent_note` → `eva_resultat` (DOUBLE → NUMERIC, contrôle exact par centièmes) ; `ent_note_appreciation` → `eva_appreciation` ; `ent_note_periode` → `eva_periode_etat` (par classe) | **vues de compatibilité** `ent_devoir`, `ent_note`, `ent_note_periode` en lecture, alimentées par les nouvelles tables | moyennes identiques avant / après sur données de labo (test d'or) ; écarts listés |
| M3 (V2–V8) | Les services écrivent dans `eva_*` ; les lectures historiques appellent le moteur | vues | — |
| M4 (V12) | Clients mobiles et UI JS migrés vers les nouvelles routes | — | aucune lecture des vues dans les journaux pendant 30 jours |
| M5 (V13) | RLS stricte ; retrait des vues et anciennes tables après la durée d'archive | — | — |

---

## 23. Lots de code (équipe Sonnet)

### 23.1 Tableau des lots

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **V0** correctifs urgents | D1–D5, D8–D12, D14 : fermeture des routes historiques d'écriture (réponse 410 explicite), filtres de publication sur `/eleves/{id}/appreciations`, `/competences`, `/conseils` ; lecture du document « bulletin » soumise à `recevoir_bulletins` du lien ; dédoublonnage + unicité ; RLS permissive sur toutes les tables d'évaluation, conseil, bulletin, compétences ; `absent_compte_zero` lu comme faux et retiré de l'écran ; mentions « avertissement » → « mise en garde » (libellés, données reprises) ; un seul droit d'export (`livrets.exporter` : secrétariat, direction) ; lectures historiques sur le moteur exact | rien (peut partir tout de suite) | tests : parent sans `voir_notes` ne télécharge pas le bulletin ; élève ne lit pas une appréciation non publiée ; administratif ne rouvre pas une période ; professeur ne publie pas de bulletin ni ne décide au conseil ; isolation B ; aucune régression des 4 scénarios existants ; CI verte |
| **V1** modèle et reprise | tables §21 (hors IA), reprise M2, vues de compatibilité, machine d'états de période, historique en ajout seul, plus de suppression physique (D6, D7), RLS | V0 ; socle 5 S4 (cours) et S7 (périodes) intégrés | test d'or de reprise (moyennes identiques) ; déclencheurs d'immuabilité testés ; `MIGRATION_TAG` calculé |
| **V2** moteur étendu | statuts §8.2, « compte si améliore », « ne compte pas », sous-matières, moyenne de matière multi-cours, annuelle (deux méthodes), statistiques et médiane, seuil de 5, explication ligne à ligne ; tests de propriétés | V1 | exemple de référence §8.8 ; propriétés (ordre des saisies, arrondi, aucune moyenne sans note) |
| **V3** grille et saisie | écran grille au clavier, collage tableur, import fichier professeur, codes rapides, conflits de version, publication programmée, retrait, délai maximal | V2 | mesure 40 × 20 ; tests clavier et 375 px ; axe-core |
| **V4** absences, dispenses, aménagements, rattrapages | ports 12, 34, 35 ; propositions d'absents ; rattrapages ; évaluation de remplacement et seul 0 autorisé ; aménagements visibles de l'AESH | V2 ; socles 12 (appel) ; ports 34/35 en simulation tant qu'ils n'existent pas | CE-25 à CE-38 en tests |
| **V5** parcours | public à la date, `non_concerne`, changement de classe et de groupe, option abandonnée, résultats externes, double cursus, bulletin de la classe de rattachement (D15) | V2 ; socle 5 | CE-01 à CE-24 en tests |
| **V6** appréciations | types, versions, brouillons, banque de phrases, contrôles, saisie en série, fenêtre et clôture | V1 | — |
| **V7** famille et élève | relevé avec explication, statistiques, rattrapages, projet d'évaluation, demande de révision ; avis du socle 8 | V2, socle 8 | matrice de séparation famille complète |
| **V8** fin de période | pilotage, relances, arrêt (auto et manuel), publication à l'arrêt, instantané versionné, rattrapages autorisés, réouverture, corrections, contrôle de 3ᵉ | V2, V6 | CE-57 à CE-63 ; empreinte stable |
| **V9** mobile | API de saisie et de relevé pour l'appli unique (socle 11) ; hors ligne avec version | V3, V7 | parité des droits avec le web |
| **V10** pilotage et statistiques | tableaux de la direction, évaluations communes (lien socle 38), charge | V8 | — |
| **V11** IA (conditionnel à Q-EV8) | capacités §13.6, propositions d'appréciations, relecture, alertes déterministes (sans IA d'abord), synthèse du PP ; journal ; fournisseur nul | V6, V8 ; cœur IA | les alertes déterministes peuvent sortir **avant** l'étude juridique ; les capacités à modèle de langage **après** |
| **V12** migration des clients | UI JS (F-001) et applis : routes nouvelles ; fermeture des lectures historiques | V7, V9 | aucun appel aux anciennes routes 30 jours |
| **V13** RLS stricte et retraits | RLS stricte, retrait des vues et anciennes tables | V12 | inventaire RLS ; tests d'isolation |

**Parcours minimal vendable** : V0 à V8 (avec, côté 23, le conseil corrigé et les bulletins rendus par le socle 7). **Socle 23** et **socle 22** : lots à concevoir à leur conception propre ; ce document leur fournit les ports (§4.2) et l'instantané (§9.1).

### 23.2 Migrations (registre `MIGRATIONS.md`)
Si Q-B2 est adoptée : **5100** V0 (RLS permissive, unicité, neutralisation du réglage, mentions) ; **5101** réglages et coefficients ; **5102** natures, évaluations, évaluations communes, restrictions ; **5103** résultats, historique, résultats externes, statuts de moyenne ; **5104** états de période, transitions, rattrapages autorisés, instantanés ; **5105** appréciations, historique, banque de phrases ; **5106** demandes de correction et de révision, consultations ; **5107** reprise de l'existant et vues de compatibilité ; 5108–5189 réserve (V2 à V11) ; **5190–5199** RLS stricte et retrait des anciennes tables (`ent_devoir`, `ent_note`, `ent_note_periode`, `ent_note_appreciation`, `ent_eva_regle`, `ent_eva_coef_matiere`) après la durée d'archive.
**Repli** si Q-B2 n'est pas adoptée : **460–479** (460–467 dans le même ordre, 468–477 réserve, 478–479 RLS stricte et retraits). Le lot V0 peut partir avant toute décision sur Q-B2 : il prend alors **460**.

### 23.3 [ajout personas] Lots ajoutés et compléments (`PERSONAS_21.md`)
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **V1** (complément) | période **par élève** (§9.4, R-V29) : port `periode_eleve`, période indicative ; tables `eva_enseignement_externe`, `eva_moyenne_externe` ; clé `(user_id, etablissement_id)` de la banque de phrases | — | tests : groupe de six classes aux arrêts différents (CE-65) ; option trimestres + semestres (CE-66) ; aucune requête de contrôle ne lit `eva_evaluation.periode_id` |
| **V3** (complément) | barème détaillé, réussite par exercice, QCM proposés (EVA-262 à 265), duplication multiple | V2 | exemple de référence avec exercice annulé (CE-83) |
| **V4** (complément) | rattrapages collectifs (EVA-259 à 261), aménagements à la création, sujet adapté, partage AESH, conflits d'accompagnement (EVA-266 à 268) | V2 ; port 35 en simulation | CE-81, CE-84, CE-85 |
| **V11** (complément) | exclusions écrites (R-V40), exemples de style pseudonymisés (R-V35), relecture des bulletins (EVA-275), « ce qui a changé » (EVA-276, déterministe, peut sortir avant l'étude juridique) | V6, V8 | refus testés : demande de note, de mention, de groupes ; aucun exemple d'un autre établissement |
| **V14** cohabitation de l'an 1 | export de l'assiduité, dépôt et rapprochement des bulletins du logiciel tiers, distribution par les droits des liens, moyennes importées en lecture, affichage « où sont les notes » (EVA-233 à 237) | **V0** ; socle 12 (assiduité) ; socle 7 (coffre, lot d'impression) ; socle 9 X7 bis (journal à reporter) | **à livrer avant le premier bulletin du pilote** ; tests : bulletin non rapproché jamais distribué ; parent interdit exclu ; élève majeur destinataire ; bornes d'assiduité |
| **V15** secrétariat | contrôle avant publication, publication programmée, lot d'impression, v2 papier, relevés en lot, duplicata, archives, registre annuel, dossier de transfert, bandeau du guichet, prénom d'usage (EVA-244 à 255) | V8 ; socle 23 (bulletins) ; socle 7 | R-V31 à R-V34, R-V36, R-V37, R-V39, R-V42, R-V47 en tests ; matrice du guichet (parent sans exercice, retrait, ordonnance de protection) |
| **V16** enseignements externes et transferts | écrans de déclaration et de saisie des moyennes externes, relance, contrôle 3ᵉ et baccalauréat (EVA-240 à 242, EVA-280) | V1, V8 | CE-67 à CE-69, CE-88, CE-91 |
**Ordre recommandé (appliqué par défaut, D-031)** : V0 → **V14** (le pilote en a besoin dès le premier trimestre) → V1 → V2 → … ; V15 avec le premier bulletin produit **par ScolIA** (collège ou an 2 du lycée) ; V16 avec V8.

---

## 24. Tests et recette

- **Tests d'or** : exemple §8.8 ; reprise M2 (moyennes identiques) ; 3ᵉ (R-V22) ; arrondis (12,495 → 12,50 ; 13,125 → 13,13 [F]).
- **Propriétés** : la moyenne ne dépend pas de l'ordre des saisies ; ajouter une évaluation « ne compte pas » ne change rien ; une évaluation « si améliore » ne fait jamais baisser.
- **Matrice de séparation** (§14.5) générée depuis la déclaration des routes (D-026).
- **Recette par personas** (D-021) : professeur de LV2 qui tente de voir l'autre groupe ; remplaçant avant sa date ; parent séparé sans `voir_notes` qui tente le bulletin par trois chemins (relevé, coffre, document) ; élève qui lit ses appréciations la veille du conseil ; administratif qui rouvre une période ; professeur qui exporte un LSU ; établissement B ; appli mobile hors ligne après arrêt ; IA d'un parent qui demande les notes d'un camarade.

---

## 25. Réglages signalés « usine à gaz » (D-023) — signalés, non supprimés

| Id | Réglage | Pourquoi il existe | Recommandation |
|---|---|---|---|
| S-100 | `absent_compte_zero` [F] | habitude de certains établissements | **retirer** : contraire à la règle nationale [V R-01] |
| S-101 | Échelle d'affichage autre que 20 | quelques établissements notent sur 10 | garder, sauf en 3ᵉ (R-V22) |
| S-102 | Pas de saisie (demi-point, quart de point) | usage de certaines disciplines | garder, simple |
| S-103 | « Compte si elle améliore », « ne compte pas » | évaluations diagnostiques, bonus d'encouragement | garder ces deux seulement |
| S-104 | Points au-delà du barème, « note bonus » au-dessus de 10 | marché (Pronote) ; DNB pour un facultatif [V R-05] | **ne pas proposer** au 21 ; le DNB le calcule au 38 |
| S-105 | Sous-matières « à plat » (toutes les notes ensemble) | autre méthode du marché | **une seule** méthode (pondérée par sous-matière) |
| S-106 | Deux méthodes de moyenne annuelle | vrai choix pédagogique | garder les deux, figées pour l'année |
| S-107 | Arrondi au demi-point d'affichage [F] | ancien usage | retirer de l'écran (contestations) |
| S-108 | Rang de l'élève | demandé par certaines familles | **désactivé**, non proposé à l'écran sans décision de Florian (Q-EV5) |
| S-109 | Arrêt automatique ou manuel | — | garder, défaut automatique |
| S-110 | Mode de publication par défaut du professeur | — | garder (choix du professeur) |
| S-111 | Délai maximal de publication | répond à G28 | garder, facultatif |
| S-112 | Appréciations intermédiaires publiées | demande de certains collèges | garder, désactivé par défaut |
| S-113 | Masquer les statistiques aux familles | — | garder |
| S-114 | Seuil d'absences aux évaluations | — | fixe (3), réglable seulement par S-118 |
| S-115 | Remplaçant qui modifie les évaluations du titulaire | — | garder, décision de la direction au cas par cas |
| S-116 | Banque commune de phrases imposée | marché | **ne pas faire** |
| S-117 | Mots interdits dans les appréciations | — | liste courte fournie, extensible |
| S-118 | Seuils des alertes | — | trois positions, pas de curseurs |
| S-119 | Le PP voit les notes détaillées des collègues | — | non par défaut ; à décider |
| S-120 | Avis « moyenne en baisse » aux familles | marché | **ne pas faire** (anxiété, lien humain) |
| S-121 | **[ajout personas]** Six types d'appréciations ; « générale » et « du conseil » séparées | un seul texte dans la pratique [H] | une **appréciation générale (du conseil)** par défaut ; la séparée activable |
| S-122 | **[ajout personas]** `a_rattraper` statut distinct d'`absent` et `non_rendu` | deux codes pour une situation | `a_rattraper` devient un **attribut** (date de rattrapage) ; codes clavier inchangés |
| S-123 | **[ajout personas]** `non_significative` et `en_attente` | deux notions, un seul affichage (et EA au LSU) | une seule, « en attente », motif interne |
| S-124 | **[ajout personas]** Publication : trois modes, délai maximal, rappel, retrait | cinq notions pour « publier » | un écran « publier » guidé, défaut du professeur |
| S-125 | **[ajout personas]** Décalage de publication élève / parents | tentant, sans enjeu réglementaire | **ne pas faire** |
| S-126 | **[ajout personas]** Rang (parité Pronote et Charlemagne [H]) | demandé par certaines familles | désactivé (Q-EV5) ; **écart de parité assumé**, signalé à Florian |
| S-127 | **[ajout personas]** Note de vie scolaire ou de conduite | usage ancien [H] | **ne pas faire** |
| S-128 | **[ajout personas]** Sanctions imprimées au bulletin | usage [H] | **ne pas faire** (R-V32) |
| S-129 | **[ajout personas]** Bibliothèque commune d'appréciations (Pronote [V titre]) | marché | inchangé : S-116 |
| S-130 | **[ajout personas]** Simulateur de moyenne | demande des élèves [H] | élève seulement, calcul local |
| S-131 | **[ajout personas]** Mise en forme du lot d'impression | tentation d'un éditeur de lot | fixe : tri par adresse, fratries regroupées, recto-verso |

---

## 26. Questions à Florian (avec recommandations)

| Id | Question | Recommandation |
|---|---|---|
| **Q-EV1** | `non_rendu` (travail non remis) compte-t-il 0 ? La règle nationale interdit le 0 comme sanction de l'absence [V] ; un travail non remis est-il une « absence » ? | **Non compté par défaut** (même logique que l'absence), le professeur pouvant évaluer un travail rendu vide ; à faire confirmer par l'inspection [NV] |
| **Q-EV2** | Publication : explicite par le professeur, avec publication automatique à l'arrêt (R-V9) et délai maximal facultatif ? | **Oui** : explicite, rappel à 10 jours, tout publié à l'arrêt ; délai maximal désactivé par défaut |
| **Q-EV3** | Moyenne annuelle : moyenne des périodes (défaut) ou de toutes les évaluations ? | Défaut **moyenne des périodes** ; au lycée, choix inscrit au projet d'évaluation |
| **Q-EV4** | Pas de saisie et précision : centième par défaut ? | **Oui** |
| **Q-EV5** | Rang de l'élève : jamais, ou sur demande de l'établissement ? | **Jamais** dans la première version |
| **Q-EV6** | Fenêtre du conseil (§14.3) : un professeur voit-il, pour les élèves de ses groupes et pendant cette fenêtre seulement, les moyennes et appréciations des autres matières ? | **Oui**, journalisé ; c'est l'application de D-022 qui garde un conseil collégial |
| **Q-EV7** | Statistiques de classe montrées aux familles (moyenne, min, max) ? | **Oui**, seuil de 5, désactivable |
| **Q-EV8** | IA : (a) propositions d'appréciations autorisées ? (b) les familles doivent-elles savoir qu'une appréciation a été rédigée avec aide ? (c) étude juridique AI Act avant mise en service ? | (a) **oui**, relues une à une ; (b) **mention dans la charte d'usage** de l'établissement, pas sur chaque appréciation ; (c) **oui**, alertes déterministes d'abord |
| **Q-EV9** | AESH : calendrier et aménagements seulement, ou aussi les notes de l'élève accompagné ? | **Calendrier et aménagements seulement** |
| **Q-EV10** | Mentions : préréglage félicitations / compliments / encouragements / mise en garde travail / mise en garde comportement, modifiable par l'établissement ? | **Oui** ; jamais « avertissement » (sanction, R-11) |
| **Q-EV11** | Établissement pilote (lycée privé), an 1 : les notes sont-elles dans ScolIA, sachant que LSL et Parcoursup exigent un éditeur validé (D-2) et que Parcoursup demande les bulletins de N-1 ? | **Décision la plus lourde** : soit (A) notes dans ScolIA dès l'an 1 **si** la validation éditeur LSL et la certification Parcoursup sont obtenues avant mars de l'an 1 ; soit (B) notes **et bulletins** restent dans Charlemagne l'an 1, ScolIA prend tout le reste et reprend N-1 en fin d'année. Recommandation : **lancer D-2 maintenant** et viser (B) pour un lycée, (A) possible pour un collège (LSU) |
| **Q-EV12** | Date d'arrêt par classe (conseils étalés) : dans le socle 5 ou dans le 21 ? | **Dans le 21** (date effective par classe), le 5 garde la date de période par défaut |
| **Q-EV13** | Collège : autoriser les classes « sans note » en 6ᵉ–4ᵉ (positionnements seulement) ? | **Oui**, permis par l'arrêté [V] ; sans réglage supplémentaire : c'est le mode de l'évaluation |
| **Q-EV14** | Durée de conservation des notes détaillées (copies, résultats) distincte de celle du bulletin ? | Proposition : notes détaillées **fin de cycle + 1 an** (comme LSUN [V]), bulletins selon le socle 7 ; à valider avec le DPO |
| **Q-EV15** | Le PP voit-il les notes détaillées des collègues avant l'arrêt (S-119) ? | **Non** : moyennes seulement |
| **Q-EV16** | **[ajout personas, bloquant A]** Période et verrou par évaluation ou par élève ? | **Par élève** (R-V29, §9.4), dans le schéma de V1 — *appliqué par défaut (D-031)* |
| **Q-EV17** | **[ajout personas, bloquant B]** An 1 du lycée pilote (D-032) : double saisie ou cohabitation outillée ? | **Cohabitation outillée** (§16, lot V14 juste après V0) ; **ScolIA seul distribue les bulletins** produits par Charlemagne — *appliqué par défaut (D-031)* ; reste à lire le format d'import de Charlemagne sur l'exemple F-07 |
| **Q-EV18** | **[ajout personas, bloquant C]** Une matière suivie hors de l'établissement (CNED, autre établissement, conservatoire) peut-elle avoir une moyenne externe qui vaut moyenne de matière ? | **Oui**, sur déclaration de la direction, source imprimée (R-V38) — *appliqué par défaut (D-031)* |
| **Q-EV19** | **[ajout personas]** Parent sans exercice de l'autorité parentale : défaut ? | Bulletins et décisions d'orientation **oui**, notes au fil de l'eau **non** (R-V43, C. civ. 373-2-1 [V]) ; à confirmer avec le socle 4 (contradiction C10) |
| **Q-EV20** | **[ajout personas]** Sanctions au bulletin ; rétention d'un bulletin pour impayé ? | **Jamais** dans les deux cas (R-V32, R-V33) |
| **Q-EV21** | **[ajout personas]** Prénom d'usage sur les documents internes ? | **Oui**, selon la décision du socle 2, état civil sur les exports et documents à un tiers (R-V36) ; texte à relire [NV R-36] |
| **Q-EV22** | **[ajout personas]** Accès d'un ancien élève à son coffre de bulletins après son départ ? | **Un an** en lecture, puis duplicata sur demande |
| **Q-EV23** | **[ajout personas]** Rattrapages collectifs : au socle 21 ou au 38 ? | **Au 21** (liste, convocations, sujets, copies, V4) ; salles et surveillances par le 38 quand il existera |
| **Q-EV24** | **[ajout personas]** Envoi postal des bulletins par un prestataire (comme Pronote avec Maileva [V]) ? | **Pas par défaut** (dépendance externe, CLAUDE.md) : lot d'impression local d'abord (EVA-248) ; prestataire à étudier (identité, contrat, données, coût) et à soumettre à Florian |
| **Q-EV25** | **[ajout personas]** Nombre d'évaluations non rendues imprimé au bulletin (R-30 [V] pour le lycée) ? | **Oui**, par matière, au lycée ; au collège aussi, par cohérence |

---

## 27. Faits, hypothèses, contradictions

**Faits vérifiés [F]** : moteur exact (`evaluation.rs:132, 252, 282, 316`) ; grille et contrôles (`evaluations.rs:1019-1040, 1583-1587`) ; écrasement et suppression (`evaluations.rs:1102, 1117`) ; fuites (`service.rs:6457-6475, 9881-9900, 6755-6777, 13975-14010`) ; route de statut historique (`routes.rs:8305-8330`) ; décision et bulletin historiques (`service.rs:13827-13915, 6478-6600`) ; réglage `absent_compte_zero` (`180:45-46`) ; mentions (`071:34-39`) ; immuabilité du bulletin (`182:36-60`) ; périodisations du socle 5 (`335`, sur `ent-integration-lot11`) ; cours (`341`, sur `ent-socle-structure-s4`). Les numéros de ligne non relus directement viennent de l'audit du sous-agent.
**Vérifié [V] le 2026-11-09** : mémento eduscol de décembre 2025 (absence et zéro, principes pour les logiciels, 3ᵉ) lu directement ; R. 421-50, R. 421-51, D. 111-3, D. 331-23 à 61, D. 331-62, R. 511-13, arrêté du 31/12/2015 modifié, décret 2025-328 et arrêté du 10/04/2025, FAQ LSL, guide eduscol lycée, avis CNIL 2017-159 (lus par sous-agent, résumés).
**Non vérifié [NV]** : circulaire 2006-137 (parents séparés) ; règle de l'élève majeur ; privé sous contrat pour le conseil ; base actuelle des inaptitudes d'EPS ; durées de conservation ; XSD LSU/LSL ; texte consolidé du règlement IA et son calendrier ; art. 22 RGPD.
**Hypothèses [H]** : irritants du terrain (§3) ; absence d'import des notes dans Charlemagne (Q-EV11) ; usages du marché (rang, notes bonus).
**Contradictions relevées** :
- **C1** `SOCLE2_DOMAINES.md` §6.4 propose que le bulletin compte les évaluations non publiées ; ce document propose la publication de tout à l'arrêt (R-V9), qui supprime l'écart.
- **C2** `SOCLES.md` (socle 21, « moyennes selon des règles paramétrables ») et D-023 : ce document **réduit** les règles (S-104, S-105, S-107).
- **C3** Droits d'export : le code donne `livrets.exporter` à la direction seule et `exports.officiels` aux administratifs ; D-022 dit « administratifs (direction, secrétariat) » : un seul droit pour les deux.
- **C4** Socle 5 porte `date_arret_notes` par période ; le terrain la veut par classe (Q-EV12).
- **C5** Le bulletin du collège imprime la rubrique « accompagnements » (PAP, PPS…) selon l'arrêté [V], alors que D-027 B interdit de révéler un dispositif : pas de contradiction réelle (le bulletin va à la famille, pas à un tiers), mais l'exemplaire remis à un **tiers** (dossier d'admission ailleurs) doit l'omettre (socle 23).
- **C6 [ajout personas]** R-V1 et `eva_evaluation.periode_id` (une période par évaluation) contre CE-41 (date d'arrêt par classe) et les cours multi-classes : **tranché par R-V29** (période par élève, §9.4).
- **C7 [ajout personas]** §16 « double saisie » l'an 1 contre la cohabitation outillée (lot V14) : **tranché par V14** ; D-032 inchangée (notes et bulletins dans Charlemagne).
- **C8 [ajout personas]** Résultats externes comptés seulement dans un cours (`compte_dans_cours_id`) contre les matières sans professeur dans l'établissement : **tranché par R-V38** (moyenne externe) ; le mécanisme d'origine reste pour les notes d'un établissement d'origine (CE-02).
- **C9 [ajout personas]** `eva_banque_phrase user_id` et le « style appris des appréciations passées » contre D-013 (aucune visibilité entre établissements) : **tranché par R-V35**.
- **C10 [ajout personas]** « Parent sans autorité : selon la décision de la direction » (§3, socle 4 C6) contre C. civ. 373-2-1 (droit d'être informé des choix importants [V]) : **défaut R-V43**, la direction gardant la main ; à reporter dans le socle 4.
- **C11 [ajout personas]** C5 vise le dossier d'admission ; Parcoursup et les relevés de transfert sont aussi des tiers : **R-V37** étend l'exclusion.
- **C12 [ajout personas]** Six types d'appréciations (§12.1) contre D-023 : **S-121** (un seul texte « générale (du conseil) » par défaut).
- **C13 [ajout personas]** §6.1 cite une « échelle à 4 niveaux » [V arrêté 2015, via sous-agent] pour les positionnements ; l'annexe III (bilan périodique du cycle 4) relue le 2026-11-09 dit seulement « la note ou tout autre positionnement » [V R-38] : l'échelle à quatre niveaux relève du **bilan de fin de cycle** (socle 22) ; le bilan périodique n'impose aucune échelle.

---

## 28. Les cinq décisions à prendre en priorité

1. **Lancer le lot V0 maintenant** (correctifs urgents, plage 5100 ou 460) : un parent privé d'accès aux notes peut aujourd'hui télécharger le bulletin, les appréciations sont lisibles avant le conseil, l'administratif peut rouvrir une période sans trace, l'absence peut valoir zéro. **Recommandation : oui, avant tout autre lot du domaine.**
2. **Première année du lycée pilote (Q-EV11)** : notes et bulletins dans ScolIA dès l'an 1, ou dans Charlemagne l'an 1 en attendant la validation éditeur LSL et la certification Parcoursup ? **Recommandation : lancer la démarche éditeur (D-2) cette semaine ; pour un lycée, garder notes et bulletins dans Charlemagne l'an 1 sauf validation obtenue avant mars.**
3. **Publication (Q-EV2)** : explicite par le professeur, tout publié automatiquement à l'arrêt des notes. **Recommandation : oui** (le relevé et le bulletin comptent enfin la même chose).
4. **Fenêtre du conseil (Q-EV6)** : pendant la préparation et la tenue du conseil, un professeur voit, pour les seuls élèves de ses groupes, les moyennes et appréciations des autres matières. **Recommandation : oui**, journalisé — c'est la lecture de D-022 qui garde un conseil collégial.
5. **IA (Q-EV8)** : propositions d'appréciations relues une à une, alertes déterministes, aucune note ni décision par l'IA, étude juridique (règlement européen sur l'IA) avant la mise en service des capacités à modèle de langage. **Recommandation : oui** ; les alertes déterministes peuvent sortir sans attendre.

**[ajout personas] Trois décisions s'ajoutent en tête** (`PERSONAS_21.md` §0), tranchées selon leur recommandation (D-031) : **(A)** période et verrou **par élève** (Q-EV16, dans le schéma de V1) ; **(B)** **cohabitation outillée** de l'an 1 du lycée pilote, ScolIA seul distribuant les bulletins produits par Charlemagne (Q-EV17, lot V14 juste après V0) ; **(C)** **moyenne externe** pour une matière suivie hors de l'établissement (Q-EV18). Puis, avant V7 et V15 : parent sans exercice (Q-EV19), aucune sanction ni rétention au bulletin (Q-EV20), prénom d'usage (Q-EV21).
