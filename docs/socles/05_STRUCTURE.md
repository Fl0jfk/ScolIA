# Socle 5 — La structure de l'établissement et l'année scolaire

**Date :** 2026-11-06 — **Statut :** conception, **non décidée** (à valider par Florian). Aucun code écrit, rien compilé ni testé pour ce document. Aucun accès à la base ni aux secrets de production ; l'application Next.js a été lue en lecture seule ciblée, sans copie de code.
**Branche :** `ent-socle-structure-conception`.
**À lire avec :** `CLAUDE.md`, `docs/VISION.md`, `docs/PRODUCT_SPEC.md`, `docs/DECISIONS.md` (D-017, D-019, D-020, D-021, **D-022**), `docs/ROLES_ET_DROITS.md`, `docs/SOCLES.md` (§3, socle 5), `docs/socles/02_ELEVE.md`, `03_INSCRIPTIONS.md`, `04_ADMINISTRATIF.md` (sur lesquels ce document s'appuie), `docs/SOCLE2_DOMAINES.md` (§3 D1, §6.3 S2-2), `docs/INTEROPERABILITE_NATIONALE.md`, `docs/socles/MIGRATIONS.md`.

**[personas] Passe personas du 2026-11-07** (`docs/socles/PERSONAS_05-06.md`, branche `ent-personas-socles-5-6`) : ajouts marqués **[personas]** — cas limites en fin de §4.5, modèle §9.10, actions STR-93 à STR-114 (§12.10), règles R-S1 à R-S24 (§12.11), droits §13.3, événements §14.3, lots §18 (note), questions Q-S20 à Q-S26. **À lire d'abord : `PERSONAS_05-06.md` §0** (contradictions avec le socle 6 à trancher avant les lots S1 et S4). Rien n'a été supprimé.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[F Next]** fait lu dans l'application Next.js (lecture seule) ; **[S]** fait sourcé repris d'un document du dépôt qui cite sa source ; **[H]** hypothèse (mémoire, déduction, pratique du terrain) ; **[NV]** règle réglementaire ou format national cité de mémoire, **à vérifier sur un texte officiel avant de coder** ; **[R]** recommandation (non décidée). Les identifiants d'actions métier `STR-xx` sont nouveaux et stables ; la correspondance avec les anciens `REF-xx` de `SOCLE2_DOMAINES.md` est indiquée. **Plage de migrations : 330–349** (`docs/socles/MIGRATIONS.md`).

**Méthode.** Trois explorations en lecture seule (sous-agents) : schéma SQL du référentiel (migrations 008, 021, 022, 048, 073, 076, 077, 092, 096, 098, 110–112), logique et droits (`ent-db/src/referentiel_*.rs`, `referentiel_repo.rs`, `perimetre.rs`, `droits_repo.rs`, `ent-api/src/referentiel*.rs`, `ent-auth/src/actions.rs`, tests), écrans Leptos et application Next.js. Les constats décisifs ont été relus directement (`perimetre.rs:127-139`, `referentiel_annee.rs:230-250` et `297-335`, `referentiel_repo.rs:508-534`). Ce qui n'a pas été relu ligne à ligne est signalé au §19.

Sommaire : §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs et ce qu'ils vivent · §4 L'année vue du terrain · §5 Public, privé sous contrat, hors contrat, premier degré · §6 Frontières · §7 Concepts · §8 La règle de visibilité du professeur (D-022) · §9 Modèle de données cible · §10 Logiciel ou paramétrage · §11 Échanges avec l'extérieur · §12 Actions métier · §13 Droits · §14 Événements · §15 Écrans clés · §16 Sécurité et données personnelles · §17 Plan de migration · §18 Lots de code · §19 Questions à trancher · §20 Récapitulatif.

---

## 1. Thèse : le « où » et le « quand » de tout l'ENT

Chaque autre socle pose une question à celui-ci. L'appel : *quels élèves attendre dans cette séance ?* Les notes : *quelle période, quelle matière, quels élèves ce professeur peut-il noter ?* La facturation : *l'élève était-il demi-pensionnaire le 12 mars, l'établissement était-il ouvert ce jour-là ?* La messagerie : *qui est « l'équipe de la 4B » ?* L'emploi du temps : *quels cours faut-il placer, pour quels publics, dans quelles salles ?* Si la structure est fausse, tout le reste l'est aussi, en silence.

Le porteur du projet a fixé le principe (D-022) : **les matières, groupes, options, ateliers, services et rattachements sont du paramétrage de l'établissement.** Le logiciel ne code ni la liste des matières, ni les noms des groupes, ni les options d'un niveau. Il code un **cadre sûr** et des **règles génériques**. Ce document en tire dix principes [R] :

1. **Le code connaît des formes, l'établissement les nomme.** Le logiciel sait ce qu'est une classe, un groupe, un enseignement proposé, un cours, une période, une journée sans cours. Il ne sait pas ce qu'est « Espagnol LV2 », « Atelier théâtre » ou « Groupe de besoin maths 6e ». Ces objets sont créés, nommés, colorés et rattachés par l'établissement. Les codes nationaux (matières, MEF, modalités) sont des **étiquettes facultatives** posées sur ces objets, obligatoires seulement pour les échanges qui les exigent.
2. **Une règle de visibilité, une seule, écrite une fois.** Un personnel voit un élève parce qu'un **fait de structure** l'y autorise : il intervient dans un **cours** dont l'élève fait partie du public, il est responsable d'un **groupe** dont l'élève est membre, ou un rôle lui donne une portée plus large. Tout se calcule à une date. Le professeur d'allemand d'une classe mixte allemand/espagnol voit le groupe allemand, pas la classe (§8).
3. **Tout est daté, rien n'est écrasé.** Une affectation, un changement de groupe, une grille horaire modifiée en février, une salle fermée pour travaux, une période raccourcie : chaque fait a une date d'effet. « Qui était dans quel groupe le 12 mars ? » a une réponse exacte, toujours.
4. **Deux années vivent en même temps.** De février à septembre, l'année N tourne pendant que N+1 se construit : options, constitution des classes, cours, export vers le logiciel d'emploi du temps. Les deux sont manipulables sans interférence ; chacun travaille sur l'année qui le concerne (le secrétariat sur N+1 en juillet, les professeurs sur N pour les bulletins).
5. **La rentrée est un chantier piloté, pas un bouton.** Le logiciel montre l'état de préparation de N+1 (élèves sans classe, choix d'options sans groupe, cours sans professeur, calendrier non adopté, périodes non définies…), chaque contrôle étant fourni par le socle qui en est propriétaire. Ouvrir l'année, c'est voir **avant** les effets calculés par le serveur, puis confirmer.
6. **La constitution des classes est un outil, pas un tableur.** Répartir 180 élèves de 5e en six classes équilibrées, en respectant les options, les séparations demandées par les équipes, l'accompagnement des élèves à besoins particuliers : c'est la tâche la plus longue de juin. Le logiciel propose, la direction décide, tout se fait dans l'ENT, et les avis sensibles disparaissent ensuite.
7. **Le calendrier vient de l'extérieur, les exceptions viennent de l'établissement.** Les vacances de la zone et les jours fériés sont **adoptés**, pas saisis ; l'établissement ajoute ses ponts, journées banalisées, fermetures, examens, stages, et peut **cibler** une exception (un site, un niveau, une classe, un groupe).
8. **Une fermeture se déclare une fois et produit tous ses effets.** « Le collège est fermé demain (neige) » est un fait du calendrier ; l'emploi du temps annule les séances, l'appel ne les attend plus, la restauration annule les repas, les familles sont prévenues (socles 10, 12, 40, 8, par événements).
9. **Un modèle pour tous les établissements.** Collège, lycée général, technologique, professionnel, ensemble scolaire multi-sites, public, privé sous contrat, hors contrat, établissement d'outre-mer au calendrier austral, et demain l'école primaire (classes à plusieurs niveaux, demi-journées) : le modèle ne présuppose ni la rentrée de septembre, ni le découpage en trimestres, ni une seule grille horaire.
10. **Rien ne se crée par accident.** Aujourd'hui, taper un code de classe inconnu dans un écran crée une classe, et lire la liste de ses classes peut créer une année [F, §2.2]. Dans la cible, la structure ne change que par une commande explicite, journalisée, qui publie son événement.

Ce qui fera la différence au quotidien [H — comparaison de mémoire avec les logiciels du marché, non vérifiée] : le **tableau de bord de rentrée** qui dit ce qui manque avant qu'il manque ; la **constitution des classes** intégrée (scénarios, équilibre en direct, contraintes d'équipe) ; la **génération des cours** depuis la maquette (« un cours par enseignement commun et par classe, un par groupe de LV2 ») ; la **cohérence options ↔ groupes** contrôlée en continu (« 3 élèves suivent l'espagnol LV2 sans être dans aucun groupe d'espagnol ») ; le **changement de classe en cours d'année** en un geste avec ses conséquences affichées ; le **calendrier adopté** et ciblé ; la **fermeture exceptionnelle** à effets automatiques ; et une visibilité du professeur **exacte au groupe près**.

---

## 2. Évaluation franche de l'existant

Le référentiel a été largement livré en S2-2 (`SOCLE2_DOMAINES.md` §6.3) : ~30 routes, écrans Leptos, tests sur base jetable. C'est une base solide, plus avancée que l'application Next.js sur l'année, les niveaux, les MEF et les périodes [F, F Next]. Mais il a été construit pour **rattacher l'existant** (texte `classe` → `classe_id`) plus que pour **porter la structure réelle d'un établissement**, et plusieurs défauts touchent la sécurité ou l'exactitude.

### 2.1 Ce qui tient (à garder)

| Élément | Pourquoi c'est bon | Où [F] |
|---|---|---|
| Année à cycle de vie (`preparation` → `courante` → `cloturee`), une seule courante | La bonne idée ; l'année clôturée est en lecture seule (`exiger_annee_modifiable`) | `092:6-19`, `110:73-98`, `referentiel_annee.rs:94` |
| Création d'année avec refus de chevauchement et verrou par établissement | Évite deux années sur les mêmes jours | `referentiel_annee.rs:117-148` |
| Préparation de N+1 avec reconduction idempotente (classes, groupes, périodes décalées, services sans PP) | Le geste attendu en mai ; rejouable | `referentiel_annee.rs:358-499, 504` |
| Affectation en lot avec aperçu, conflits jamais écrasés | Le bon contrat (aperçu → confirmation) | `referentiel_annee.rs:539-604` |
| Affectations datées, classe XOR groupe, une classe ouverte par année | Le socle de « où était l'élève à la date D » | `092:56-84` |
| Service de groupe : un professeur de groupe ne voit que le groupe dans la branche référentiel | Déjà conforme à D-022 **dans cette branche** | `referentiel_repo.rs:986-1001`, `perimetre.rs:112-125` |
| Sites avec UAI, niveaux avec cycle, MEF à 11 caractères, MEFSTAT11, import MEF avec rejets ligne à ligne | Compatible avec les nomenclatures nationales [S] | `110:8-66`, `referentiel_structure.rs:448-538` |
| Périodes contrôlées (couverture sans trou ni chevauchement), refus de retirer une période utilisée par les notes | Bonne règle métier | `referentiel_structure.rs:192, 738` |
| Séances datées générées depuis créneaux × grille × A/B × exceptions × calendrier, séances manuelles jamais écrasées | Indispensable à l'appel et au cahier de textes | `referentiel_seances.rs` |
| Événements `referentiel.*` publiés dans la transaction (module S2) | Conforme à la toile (S2-0) | `ref_commun.rs:73-104` |
| Actions `referentiel.lire` / `referentiel.gerer` dans le moteur, modèles équivalents à l'existant | Bascule sans changement de comportement | `actions.rs:225-226, 390-393` |
| Écrans Leptos : années et bascule, structure, classes et groupes, calendrier et périodes, grille et séances | Une première interface sérieuse | `ent-web-app/src/modules/referentiel_admin/` |

### 2.2 Ce qui est à revoir

| # | Constat | Conséquence | Où [F] | Cible [R] |
|---|---|---|---|---|
| 1 | **D-022 non tenue de bout en bout.** La portée « mes classes » unit la branche référentiel (au groupe) et une branche **emploi du temps historique** qui joint `ent_edt_creneau.classe = ent_eleve.classe` : un créneau sur « 4B » ouvre **toute** la 4B. `ent_edt_creneau` n'a pas de groupe | Le professeur d'allemand voit les élèves d'espagnol (notes, absences, dossier) | `perimetre.rs:127-139`, `021:3-18` | Une seule règle (§8) sur les **cours** ; branche EDT retirée quand chaque créneau pointe vers un cours |
| 2 | `classes_du_prof` liste la classe entière dès qu'un service porte sur un **groupe** de cette classe ; `GET /referentiel/mes-classes` n'exige que « être du personnel » | L'écran « mes classes » montre plus que le périmètre | `referentiel_repo.rs:508-534`, `referentiel.rs:400-405` | « Mes cours et mes groupes » calculé par la règle du §8 |
| 3 | **Lire crée.** `classes_du_prof` appelle `referentiel_assurer_annee`, qui crée l'année du jour si elle manque ; le trigger `classe_id` **crée une classe** quand on tape un code inconnu ; `sync_affectation_classe` aussi | Classes fantômes, années inventées, structure qui change sans décision | `referentiel_repo.rs:287-335, 514, 796-798`, `111:57-78, 109` | Aucune création implicite (principe 10) ; code inconnu = erreur explicite |
| 4 | **Ouvrir N+1 clôt N sans contrôle** (notes non fermées, conseils non tenus) alors que la clôture directe les vérifie | Une année se ferme par effet de bord, avec des périodes ouvertes | `referentiel_annee.rs:237-246` vs `320-332` | L'ouverture **n'implique pas** la clôture (§4.4) : N reste ouverte jusqu'à sa clôture propre |
| 5 | **Clôturer l'année courante sans successeur** est possible ; l'ancienne route `POST /referentiel/annees` avec `courante=true` retire la courante sans la clôturer (le trigger la remet en « préparation ») et sans contrôle de chevauchement ; remettre `cloturee_le` à NULL rouvre sans garde | États incohérents (aucune année courante, ou une année « en préparation » qui a déjà vécu) | `referentiel_annee.rs:297-335`, `referentiel_repo.rs:227-268`, `110:80-96` | Machine à états stricte en base (§9.2), réouverture par commande ★ seulement |
| 6 | **Un groupe n'a qu'une classe parente** (`ent_groupe.classe_id`) ; pas de table de liaison. Un groupe d'espagnol réunissant 4A et 4B est « transverse » et disparaît des listes de classe (jointure `g.classe_id = c.id`) | Les barrettes de LV2, les spécialités de lycée, les options inter-classes sont mal représentées | `092:39-52`, `referentiel_repo.rs:964` | Groupe rattaché à **0..n** classes (§9.6) |
| 7 | **Rien ne relie le choix d'option de l'élève au groupe.** Options en texte libre (`ent_eleve_option`, `_option_annee`, année en texte) ; affectation à un groupe sans vérifier l'appartenance à la classe parente ni le choix | Impossible de contrôler « tous les latinistes sont dans un groupe de latin » | `015:24-38`, `018:50-69`, `referentiel_repo.rs:662-715` | Offre d'enseignements + enseignements suivis + règles de composition (§7, §9.5) |
| 8 | **Le service mélange deux choses** : *ce qui est enseigné à qui* (matière × public) et *qui l'enseigne* (professeur). Pas de quotité, pas de co-enseignement, pas d'heures ; `matiere_libelle` texte en double de `matiere_id` | Un cours sans professeur (poste vacant) n'existe pas ; deux professeurs en co-intervention = deux services sans lien ; la maquette horaire est impossible | `092:87-116` | Séparer **cours** (socle 5) et **service** (socle 6) (§7, §9.7) |
| 9 | **Reconduction incomplète** : ni responsables de groupe, ni composition des groupes ; un service de groupe reconduit pointe vers un groupe vide | En septembre, les professeurs de groupe ne voient personne tant que les groupes ne sont pas repeuplés à la main | `referentiel_annee.rs:358-499` | Reconduction choisie élément par élément, avec rapport (§12, STR-12) |
| 10 | **Montée par code de classe** (« 6A » → « 5A ») | La plupart des collèges recomposent les classes chaque année [H] ; la montée doit suivre la **décision de passage** et aboutir à un **niveau**, la classe venant de la constitution | `referentiel_annee.rs:648`, `SOCLE2_DOMAINES.md` §6.3 | Montée → inscription annuelle N+1 (socle 2) → constitution (§4.3) |
| 11 | **Deux tables de jours fermés** (`ent_periode_vacances` : vacances/férié/fermeture ; `ent_jour_calendrier` : fermé/banalisé/férié/vacances) et deux voies d'écriture ; une « fermeture » est lue comme « vacances » ; aucune zone, aucune source officielle, aucun lien à l'année | Doublons, sens perdu, vacances ressaisies chaque année | `073:3-18`, `110:145-161`, `referentiel_structure.rs:893-947, 1052`, `vacances_repo.rs` | Un seul calendrier daté, ciblé, sourcé (§9.3) |
| 12 | **Deux tables de périodes** : `ent_periode` (par année, par niveau) et `ent_note_periode` (unique **par établissement**, sans année) ; notes et devoirs pointent vers la seconde | Le code « T1 » ne peut exister qu'une fois dans toute la vie de l'établissement | `022:3-21`, `110:113-140`, `112:91-107` | Les périodes appartiennent à l'année ; l'état « ouverte à la saisie » reste au socle 21 (§6) |
| 13 | **Salles et sites en texte** : `ent_salle.site`, `ent_edt_creneau.salle`, `ent_edt_seance.salle`, `ent_edt_creneau.site` ; pas de type de salle ni de bâtiment ; aucun écran salles ni matières | Impossible de savoir si une salle de 24 places reçoit un cours de 30 | `076:16-32`, `021:14`, `065:3-4`, `110:207` | Salles rattachées, typées, avec capacité et équipement (§9.4) |
| 14 | **Matières pauvres** : code et libellé seulement (ni abréviation, ni couleur, ni sous-matière, ni code national structuré, ni archivage) | Bulletins, EDT et cahier de textes réinventent chacun leur présentation | `008:4-16` | Matière enrichie (§9.5) |
| 15 | **Établissement sans profil** : ni secteur (public, privé), ni académie, ni zone de vacances, ni fuseau horaire ; paramètres en clé/valeur texte (`ent_param_etab`) ; l'heure locale est codée « Paris » (`heure_paris`) | Impossible de préremplir quoi que ce soit ; outre-mer impossible | `096:10-18`, `076:3-14`, `SOCLE2_DOMAINES.md` §6.4 | Profil d'établissement typé (§9.1) |
| 16 | **Cloisonnement par clés simples** : `classe_id`, `groupe_id` d'une affectation ou d'un service ne vérifient ni l'établissement ni l'année ; la migration 112 **écrit des données** (crée années et classes) à son application ; `ent_edt_seance.classe` texte sans trigger (10ᵉ table) | Une ligne peut pointer vers la classe d'un autre établissement ou d'une autre année | `092`, `112:60`, `110:202` | Clés étrangères composées `(etablissement_id, annee_id, id)` (§9.2) |
| 17 | **Anciennes écritures sans événement** : création de classe, de groupe, de service, affectation unitaire, amorçage (`referentiel_repo.rs`, aucun `publier`) ; affectation **supprimée physiquement** si la date d'effet précède le début | Les consommateurs (appel, messagerie, droits) ne savent pas qu'une composition a changé ; historique perdu | `referentiel_repo.rs:729-794` | Toutes les écritures par les commandes du socle 5, en transaction avec événement (§14) |
| 18 | **Sortie de l'élève** : personne ne ferme ses affectations ; la portée « mes groupes » ne filtre pas les élèves sortis | Un élève parti reste visible du responsable d'atelier | `droits_repo.rs:193-211` | Port `structure::fermer_affectations` appelé par le socle 2 (ELV-11) dans sa transaction |
| 19 | **Tests silencieux sans base** : `tests_referentiel_s2.rs` retourne sans rien vérifier si `ENT_DATABASE_URL` manque ; aucun test « professeur de groupe, classe mixte » (exigé par D-022) | Fausse impression de couverture | `tests_referentiel_s2.rs:14-22` | Échec explicite en CI si la base manque ; matrice D-022 (§18, S1) |

### 2.3 Ce qui manque

- **Offre d'enseignements** (ce que l'établissement propose à chaque niveau ou MEF : enseignements communs, LV1, LV2, options, spécialités, dispositifs) et **règles de choix** (« une LV2 en 5e », « trois spécialités en 1re » [NV]), paramétrées.
- **Enseignements suivis** par chaque élève, datés, reliés aux groupes.
- **Cours** comme unité d'enseignement (ce qui est enseigné à quel public, combien d'heures), support de l'emploi du temps, des notes, du cahier de textes, et **maquette horaire** ; **barrettes** (cours alignés à la même heure).
- **Constitution des classes** (critères, avis des équipes, scénarios, équilibre).
- **Tableau de bord de rentrée** alimenté par tous les socles ; **aperçu serveur** de l'ouverture.
- **Calendrier officiel** adopté par zone ; jours fériés calculés (dont Alsace-Moselle, outre-mer [NV]) ; exceptions ciblées ; **fermeture exceptionnelle** à effets ; **types de semaine** (A/B, cycles) gérés comme fait du calendrier (aujourd'hui `ent_edt_calendrier_ab`, une seule date de référence, sous `edt.gerer` [F `048:13-19`]).
- **Périodisations** multiples (trimestres au collège, semestres en terminale) assignées par niveau, MEF ou classe.
- **Grilles horaires** versionnées et ciblées (collège et lycée sur un même site n'ont pas les mêmes sonneries [H]).
- **Profil d'établissement**, bâtiments, salles typées, indisponibilités de salles.
- **Dates de rentrée par niveau** (rentrée échelonnée), pré-rentrée des personnels.
- **Changement de classe en cours d'année** comme commande unique (groupes, cours, conséquences).
- **Dotation horaire** (public, privé sous contrat) confrontée aux besoins de la structure.
- Écrans : salles, matières, offre, cours, constitution, tableau de bord de rentrée.

### 2.4 Ce que Next.js fait et qu'il ne faut pas perdre (lecture seule)

| Idée [F Next] | Où | Reprise [R] |
|---|---|---|
| **Outil de répartition des classes** : campagne par niveau (classes sources, classes cibles), avis des personnels (séparer de, accepte de prendre l'élève), « score » manuel de niveau et genre par élève, solveur, scénarios, publication, journal ; souhaits des parents (camarades préférés / à éviter, **professeur souhaité / à éviter**) dont la saisie libre est résolue par un modèle d'IA externe | `app/lib/class-allocation-*.ts`, `app/(admin)/toolbox/repartition-classes/`, `class-allocation-wish-match.ts` (clé Mistral lue côté serveur) | **Reprendre l'idée** (§4.3, STR-71 à 78) ; **ne pas reprendre** le choix d'un professeur par les familles (pression sur les personnels, non opposable [R]) ni la résolution des saisies libres par un modèle externe (on fait choisir dans une liste d'élèves du même niveau, sans IA) |
| Coefficient par matière × classe, « compte dans la moyenne générale » | `db/schema-charlemagne-p1.ts:157-179` | Socle 21 (évaluations), pas ici ; le socle 5 fournit le **cours** auquel le coefficient s'attache |
| Famille de matière (regroupement pour le bulletin), matière active | `schema-charlemagne-p1.ts:97-115` | Champ `famille` et archivage (§9.5) |
| Plusieurs grilles horaires nommées | `TimetableGridsSettingsPanel.tsx:103-153` | Grilles ciblées (§9.3) |
| Zone de vacances par site | `SettingsSitePanel.tsx:156-159` | Zone au profil (et par site en exception) |
| Salles avec bâtiment, type, « réservable » | `db/schema-reservation-rooms.ts:19-40` | §9.4 ; les réservations restent au socle 44 |
| Bilan avant bascule (encours de factures conservés) | `app/lib/rentree-scolaire.ts:34-64` | Contrôle fourni par le socle 24 au tableau de bord de rentrée |
| Calendrier de vacances 2025-2026 **figé dans le code** | `app/lib/vs-calendrier-db.ts:269-288` | Anomalie à **ne pas reproduire** : calendrier adopté depuis une source, jamais codé |
| Table `annee_bascule` (préparée / validée) jamais utilisée | `db/schema-socle-faits.ts:406-430` | Journal des opérations d'année (§9.2) |

---

## 3. Acteurs et ce qu'ils vivent

| Acteur | Ce qu'il fait avec la structure | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement, adjoint** | Décide de la structure (nombre de classes, options ouvertes, dédoublements), arbitre la dotation, valide la constitution des classes, ouvre et clôt l'année | Tableurs de structure et de dotation qui divergent de la base élèves ; ressaisies entre logiciel d'EDT, base nationale et ENT | Une rentrée où une classe n'a pas de professeur, des élèves sans classe le jour J, une option ouverte sans effectif | Un écran de rentrée qui dit « prêt » ou « il manque ceci » ; simuler « et si j'ouvre une 7ᵉ classe de 6e ? » avec ses effets en heures |
| **Secrétariat de scolarité** | Crée l'année, adopte le calendrier, saisit les options, affecte les élèves, gère les arrivées et changements en cours d'année, exporte | Saisir deux fois les options (base nationale et ENT), refaire les groupes à la main en septembre, corriger les listes fausses du logiciel d'EDT | Une erreur d'affectation qui envoie les notes d'un élève au mauvais professeur ; une liste d'appel fausse le jour de la rentrée | Coller la liste des options, voir les groupes se composer seuls, n'avoir que les anomalies à traiter |
| **Responsable de l'emploi du temps** (souvent l'adjoint) | Exporte structure, cours et services vers le logiciel d'EDT, importe le résultat | Structures qui ne correspondent pas entre logiciels ; groupes nommés différemment ; barrettes perdues | Un import qui écrase les groupes corrigés à la main | Un aller-retour sans perte, avec un rapport des différences avant application |
| **CPE, vie scolaire** | Utilise classes et groupes pour l'appel, les retenues, les listes de diffusion ; participe à la constitution (séparer des élèves) | Listes d'appel fausses en septembre ; groupes qui ne suivent pas les changements | Un élève absent d'une liste d'appel (donc jamais signalé absent) | Une liste toujours juste, et un changement de groupe qui prévient tout le monde |
| **Professeur** | Consulte ses cours, ses groupes, leurs élèves ; donne son avis pour la constitution | Liste de classe qui contient des élèves qu'il n'a pas ; groupes de besoin recomposés sans être prévenu | Noter un élève qu'il n'a pas, en oublier un qu'il a | « Mes cours » exact au groupe près, et une notification « 2 élèves entrent dans votre groupe lundi » |
| **Professeur principal** | Suit toute sa classe, ses options, ses changements | Ne sait pas qui a changé d'option | Découvrir au conseil qu'un élève a quitté la classe en novembre | Une fiche de classe avec ses mouvements |
| **Responsable d'atelier, de club, d'association sportive** (professeur ou non) | Compose son groupe, fait l'appel de l'activité | Ne peut pas inscrire lui-même ses élèves | Un élève parti qui reste dans sa liste | Composer son groupe seul, dans la limite de ses droits |
| **Comptabilité, gestion** | Lit régimes, présence, jours d'ouverture pour facturer | Fermetures non saisies → repas facturés à tort | Facturer un jour de fermeture | Calendrier unique dont la facturation lit les jours ouvrés |
| **Accueil** | Lit l'emploi du temps et les classes pour orienter, répondre aux familles | — | — | Savoir en un coup d'œil où est une classe à 10 h |
| **Familles, élèves** | Voient leur calendrier, leur classe, leurs groupes et options, la date de rentrée | Calendrier publié en PDF, dates de rentrée par niveau introuvables | Apprendre la classe la veille de la rentrée par une liste affichée au portail | Un calendrier personnel (vacances, journées sans cours, stage de 3e, examens) synchronisable sur téléphone |
| **Organisme gestionnaire, tutelle (privé)** | Lit les effectifs et la structure pour le budget | Chiffres envoyés en tableur | — | Effectifs datés comparables d'une année sur l'autre |
| **Autorité académique, services nationaux** (public, privé sous contrat) | Reçoivent structures et effectifs via les applications nationales | — | — | — (le gain est la fin de la ressaisie côté établissement) |
| **Socles internes** (10, 12, 20, 21, 23, 24, 40, 41, 43…) | Lisent la structure à une date, réagissent à ses événements | — | Une structure qui change sans événement | Des ports stables et une seule règle de visibilité |

---

## 4. L'année vue du terrain

### 4.1 Le calendrier réel d'une année (collège ou lycée, France métropolitaine) [H, dates indicatives à vérifier]

| Période | Année N (en cours) | Année N+1 (en construction) |
|---|---|---|
| Septembre–octobre | Rentrée ; ajustements de groupes et d'options jusqu'à une date limite ; élèves non présentés ; arrivées tardives | — |
| Novembre–décembre | Conseils du 1er trimestre | Premières prévisions d'effectifs (socle 3) |
| Janvier–février | Conseils du 2ᵉ trimestre (ou du 1er semestre) | **Dotation horaire** reçue (public, privé sous contrat) [NV] ; **structure prévisionnelle** (nombre de classes par niveau, options, dédoublements) ; présentation aux instances [NV] ; ouverture des réinscriptions (socle 3) |
| Mars–mai | Orientation (socle 37) | Vœux d'options pour N+1 (socle 3) ; **préparation de N+1** (création, reconduction) ; services prévisionnels des professeurs (socle 6) |
| Juin | Conseils du 3ᵉ trimestre : **décisions de passage** (socle 23) ; examens (lycée surtout) ; fin des cours avancée dans certains lycées [H] | **Constitution des classes** ; affectations nationales des entrants [NV] ; cours générés ; export vers le logiciel d'EDT |
| Juillet | **Clôture** de N quand tout est arrêté | Import de l'EDT ; arrivées de l'été ; classes publiées (ou non) |
| Fin août | — | Pré-rentrée des personnels ; **ouverture** de N+1 ; rentrée échelonnée par niveau |

Conséquence pour le modèle : N et N+1 coexistent pendant **sept mois**. La « courante » sert de défaut d'affichage et de cible des actions quotidiennes ; N+1 est pleinement manipulable en préparation (§9.2).

### 4.2 La préparation de rentrée, étape par étape [R]

Le **tableau de bord de rentrée** (STR-13) déroule ces étapes, chacune avec ses contrôles (§9.12) :

1. **Créer N+1** (dates, dates de rentrée par niveau, pré-rentrée) et **adopter le calendrier** de la zone.
2. **Reconduire** ce qui doit l'être (périodisations, grilles, classes « modèles », groupes et ateliers durables avec leurs responsables, offre d'enseignements, maquette) : chaque élément est coché, l'aperçu dit ce qui sera créé.
3. **Arrêter l'offre** de N+1 (options ouvertes ou fermées, capacités) — c'est elle que le socle 3 présente aux familles pour les vœux.
4. **Construire la structure** : nombre de classes par niveau, groupes d'enseignement, ateliers ; confrontation **besoins en heures / dotation** (public et privé sous contrat).
5. **Faire monter** : à partir des décisions de passage (socle 23) et des réinscriptions (socle 3), chaque élève reçoit une **inscription annuelle N+1** (niveau, MEF ; socle 2) et ses **enseignements suivis** ; les départs de fin d'année sont préparés (socle 3).
6. **Constituer les classes** (§4.3) puis **composer les groupes** depuis les enseignements suivis.
7. **Générer les cours** depuis la maquette ; le socle 6 y attache les services.
8. **Échanger avec le logiciel d'EDT** (export structure + cours + services ; import des créneaux, socle 10).
9. **Vérifier** : le tableau de bord ne montre plus de blocage.
10. **Ouvrir** N+1 (aperçu serveur des effets, confirmation, éventuellement programmée à une date).

### 4.3 La constitution des classes

C'est la tâche la plus redoutée de juin [H] : des heures de tableur et de papillons, des oublis (« on a mis les deux élèves qu'on avait demandé de séparer »), des déséquilibres découverts en septembre. Ce que le logiciel apporte [R] :

- **Une campagne par niveau** : élèves attendus (montée, redoublants, nouveaux admis, entrants affectés), classes cibles (nombre, capacité, enseignements que chaque classe peut accueillir : « bilangue en 6A et 6B seulement »).
- **Des critères d'équilibre choisis** par l'établissement parmi un catalogue générique : effectif, répartition filles/garçons, enseignements suivis, établissement ou classe d'origine, redoublants, nouveaux, **indicateur de niveau facultatif** (1 à 4, saisi pour la campagne seulement), élèves accompagnés (AESH partagé : regrouper), demi-pensionnaires (contraintes d'EDT) [R]. Aucun critère tiré de la santé ni de la vie familiale.
- **Des contraintes** : séparer, regrouper, classe imposée, classe exclue ; saisies par les équipes (professeurs, CPE, direction) avec un **motif codé** (« conflit », « dynamique de groupe », « accompagnement commun »…) et, si besoin, une note courte **scellée** lisible de la seule direction (§16).
- **Des souhaits des familles**, si l'établissement l'active (désactivé par défaut) : un ou deux camarades, **choisis dans la liste** du niveau (pas de saisie libre, pas de professeur) ; non opposables, annoncés comme tels.
- **Des scénarios** : un calcul propose une répartition ; la direction compare plusieurs scénarios, **verrouille** des élèves, déplace à la main par glisser-déposer, voit en direct les indicateurs et les contraintes violées.
- **Une validation** qui produit, en une transaction, les affectations N+1 (date = début de l'année) ; une **publication** à la date choisie (beaucoup d'établissements ne publient qu'à la rentrée [H]).
- **Une purge** des contraintes, indicateurs et souhaits à la clôture de la campagne (paramétrable, défaut : 30 jours après la rentrée), seul l'historique des affectations restant.

Algorithme [R] : recherche locale (affectation initiale gloutonne puis échanges améliorants sous contraintes dures), déterministe à graine fixée, testable, exécutée côté serveur en tâche ; aucune dépendance externe ; quelques secondes pour 300 élèves [H]. Pas d'IA générative : le calcul est explicable (« A et B sont ensemble parce que la contrainte de séparation a été levée »).

### 4.4 Ouvrir et clôturer : deux gestes distincts [R]

- **Ouvrir N+1** fait de N+1 l'année courante (affichage par défaut, cible des actions quotidiennes, séances du jour). **N n'est pas clôturée par ce geste** (correction du défaut n° 4) : elle passe à l'état `terminee` (vie quotidienne finie, encore modifiable pour finir bulletins, livrets, exports) jusqu'à sa clôture.
- **Clôturer N** n'est possible que si N n'est plus courante et que les contrôles des socles passent (périodes de notes fermées — socle 21 ; conseils tenus et décisions de passage saisies — socle 23 ; exports officiels faits ou explicitement écartés ; soldes signalés — socle 24…). Un **forçage** reste possible pour la direction, motivé, journalisé, avec la liste des contrôles ignorés.
- **Une année clôturée est en lecture seule.** Une **réouverture** pour correction (bulletin erroné découvert en octobre) est une commande ★ réservée, limitée dans le temps (défaut : 15 jours) et à un périmètre (« corrections d'évaluations seulement »), journalisée [R].
- **Ouverture programmée** : la direction peut programmer l'ouverture à une date (par défaut le jour de pré-rentrée) ; l'aperçu est recalculé à l'exécution, et si un blocage est apparu entre-temps, l'ouverture n'a pas lieu et la direction est prévenue.

### 4.5 Les changements en cours d'année (catalogue de cas)

| Cas | Ce que fait le logiciel [R] |
|---|---|
| Un élève change de classe le 4 novembre | Commande unique : affectation de classe close au 3, ouverte au 4 ; groupes **recalculés** (il quitte le demi-groupe de 5A, entre dans celui de 5C ; il garde son groupe d'espagnol inter-classes si le cours existe pour 5C) ; aperçu des conséquences : cours quittés et rejoints, professeurs concernés, notes conservées (elles appartiennent à l'élève et au cours d'origine, socle 21), appel des séances suivantes ; professeurs principaux et vie scolaire prévenus (socle 8) |
| Un élève abandonne le latin à la Toussaint | Enseignement suivi clos à la date ; sortie du groupe de latin proposée et appliquée ; le professeur de latin le voit jusqu'à la date seulement ; date limite de changement d'option paramétrable (alerte au-delà, pas de blocage) |
| Les groupes de besoin de maths sont recomposés à chaque période [NV : organisation des groupes de besoin en 6e-5e à vérifier] | Composition en lot, datée ; le groupe reste le même objet, sa composition change ; professeurs prévenus des entrées et sorties |
| Une classe est scindée en janvier (effectif trop lourd) | Nouvelle classe ouverte à la date ; affectations déplacées en lot ; cours dupliqués pour la nouvelle classe ; services à attribuer (alerte socle 6) |
| Deux classes fusionnent | L'une est close à la date ; affectations déplacées ; cours de la classe close clos |
| Un nouvel élève arrive le 15 janvier | Le socle 3 valide l'inscription en cours d'année et propose une classe (seul cas, `03_INSCRIPTIONS.md` §7.5) ; le socle 5 exécute l'affectation, propose les groupes selon ses enseignements suivis |
| Un élève part | Le socle 2 clôt la scolarisation et appelle `fermer_affectations` dans sa transaction (correction du défaut n° 18) |
| Une salle est fermée pour travaux trois semaines | Indisponibilité datée ; le socle 10 signale les séances concernées |
| La grille horaire change au retour des vacances d'hiver | Nouvelle version de grille valable à une date ; les séances futures sont régénérées par le socle 10 |
| La fin du 2ᵉ trimestre est avancée d'une semaine | Commande sur la période ; contrôle : aucune note saisie hors de la nouvelle période (socle 21 répond), conseils déjà planifiés signalés (socle 23) |
| Neige : fermeture du site demain | Fermeture exceptionnelle ciblée (site, niveaux), aperçu des effets, confirmation ; événement consommé par 10, 12, 40, 8 |
| Un professeur part en congé long | Rien dans la structure : les cours restent, le service est modifié (socles 6, 15) ; le tableau de bord signale « cours sans intervenant » |
| Un atelier s'arrête en février | Groupe fermé à la date ; responsables et membres clos |
| **[personas]** Un élève d'ULIS suit les mathématiques et l'EPS en 5B, le reste dans sa classe de référence | **Inclusion individuelle** datée dans les deux cours de la 5B (STR-102, R-S5) : il figure sur leurs listes d'appel et de notes ; les professeurs de la 5B le voient pour ces cours seulement ; l'AESH est prévenue de tout changement (06 R-P20) |
| **[personas]** Décloisonnement à l'école : les CE2 vont en CM1 pour les mathématiques le jeudi | Inclusions individuelles (ou groupe « CE2 maths ») dans le cours de CM1 ; exclusion du cours de mathématiques de CE2 sur le même créneau |
| **[personas]** Une élève part trois mois en échange au Canada ; une correspondante est accueillie au printemps | Exclusion datée de tous ses cours (pas d'absence à chaque heure, socle 12) ; la correspondante, non inscrite, est **accueillie** dans les cours datés de la classe (STR-102 ; statut d'accueil temporaire au socle 2) et figure sur les listes d'appel |
| **[personas]** Neige : transports interdits, 180 internes présents | Fermeture **à effets différenciés** (R-S1) : cours fermés, internat et restauration maintenus, accueil des externes présents ; l'aperçu liste les internes présents |
| **[personas]** Alerte cyclonique (outre-mer) : fermeture décidée par la préfecture selon le niveau, levée parfois dans la nuit [NV] | Fermeture conditionnelle avec **levée programmée** ou prolongation (STR-94) ; un seul message par changement (socle 8) |
| **[personas]** Un élève de 2ᵈᵉ générale est réorienté en 2ᵈᵉ professionnelle du même lycée en novembre | **Réorientation** en une commande (STR-101) : inscription annuelle (formation) par le port du socle 2, classe, enseignements suivis, groupes |
| **[personas]** Une commission d'appel de juillet fait passer en 1ʳᵉ un élève prévu en redoublement, déjà placé en 2ᵈᵉ de N+1 | **Révision de la montée** (STR-105) : affectations N+1 annulées, élève remis « à affecter », alerte à la direction et à la campagne de constitution |
| **[personas]** Neuf désistements et six arrivées après la validation de la constitution | **Campagne rouverte** (STR-106) : élèves déjà placés verrouillés, seuls les mouvements d'été sont calculés |
| **[personas]** Un élève est resté trois semaines dans le mauvais groupe de spécialité (appels faits, deux notes) | **Correction rétroactive** (STR-104, R-S6) : affectation erronée annulée, vraie affectation à la date d'origine ; appels et notes concernés listés aux socles 12 et 21 ; aucune annonce de « changement » |
| **[personas]** Le groupe d'espagnol de 1ʳᵉ compte 36 élèves en septembre | **Dédoublement** (STR-103) : deux groupes, cours dupliqué, service à pourvoir signalé au socle 6 |
| **[personas]** Le gymnase municipal ferme deux semaines (travaux de la mairie) | Indisponibilité datée du **lieu extérieur** (STR-109) ; séances signalées au socle 10 |
| **[personas]** Classes mêlant élèves sous statut scolaire et apprentis (lycée professionnel) | Calendrier d'alternance porté par un groupe « apprentis de la classe » (cible de groupe, §9.3) ; périodes en entreprise par classe |
| **[personas]** L'établissement passe à ScolIA en janvier (reprise en pleine année) | **Reprise datée** (STR-113, R-S13) : affectations et enseignements suivis depuis la rentrée, pas l'état du jour |
| **[personas]** Un élève change de classe pour des raisons de protection | Motif codé confidentiel ; notifications sans motif ; la classe d'origine n'est pas informée de la destination (R-S12) |

---

## 5. Public, privé sous contrat, hors contrat, premier degré

### 5.1 Public (second degré) [NV pour tout ce paragraphe]
- La structure (divisions, groupes) et les options des élèves sont tenues dans la base nationale des élèves (SIECLE) ; les services des enseignants et leur ventilation sur divisions et groupes dans STS-Web, alimenté par le logiciel d'emploi du temps ; ces structures doivent correspondre, avec des **codes nationaux** de matières (BCN `N_MATIERE_ENSEIGNEE`), de MEF et de **modalités d'élection** (obligatoire, facultative… : code qui qualifie la manière dont un élève suit une matière dans son MEF) [S `INTEROPERABILITE_NATIONALE.md:116-122`].
- Une **dotation horaire globale** est notifiée chaque année ; sa répartition par discipline est présentée aux instances de l'établissement.
- Le calendrier des vacances est fixé par arrêté national par zone ; les académies d'outre-mer ont leur calendrier.
- Conséquence [R] : en public, les codes nationaux sont **obligatoires à l'export** ; le tableau de bord signale chaque matière, enseignement ou classe sans code ; l'import de structure SIECLE et STS (socle 9) arrive **avant** qu'un établissement public saisisse quoi que ce soit (D-019).

### 5.2 Privé sous contrat [NV]
- Mêmes applications nationales pour les élèves et les enseignants sous contrat (à vérifier finement : usage de SIECLE et de STS-Web, dotation par le rectorat) ; s'y ajoutent l'**organisme gestionnaire** (budget, personnels de droit privé) et, pour l'enseignement catholique, la **tutelle** et les services diocésains.
- Classes ou options **hors contrat** dans un établissement sous contrat (une option financée par les familles) : un enseignement proposé peut porter « hors contrat » (non exporté, facturable — socle 24).
- Ensembles scolaires : école, collège, lycée sur un ou plusieurs sites, plusieurs UAI sous un même tenant (`02_ELEVE.md` Q-E13) : l'UAI est porté par le **site** [F `110:14-15`] ; le profil de l'établissement liste les degrés.

### 5.3 Hors contrat [NV]
- Aucune obligation d'utiliser les applications nationales de structure ; programmes, matières et volumes horaires libres ; pédagogies propres (bilingue, international, Montessori au primaire) ; parfois années décalées (écoles internationales).
- Conséquence [R] : codes nationaux facultatifs, maquette libre, contrôles de conformité désactivés par le profil ; l'année n'est pas forcément septembre–août.

### 5.4 Premier degré (socle 50) : ne pas le rendre impossible
- Classes à **plusieurs niveaux** (CE1-CE2) : une classe porte **0..n niveaux** (§9.6).
- Enseignant polyvalent : un **cours unique** « classe entière, toutes disciplines » (ou des cours par domaine d'apprentissage) ; la règle de visibilité est la même.
- Appel par **demi-journée** : la grille peut n'avoir que deux créneaux (matin, après-midi).
- Rythmes : semaine de 4 jours ou 4,5 jours [NV] → jours ouvrés paramétrables.
- Base nationale ONDE au lieu de SIECLE [S] → codes portés par les mêmes étiquettes facultatives.

### 5.5 Outre-mer, étranger [NV]
- Calendrier austral (La Réunion, Mayotte : rentrée mi-août [NV]), fuseau horaire propre : **fuseau horaire au profil** (correction de `heure_paris`), année aux dates libres.
- Établissements français à l'étranger (rythme nord ou sud) : même mécanisme, hors cible commerciale initiale.

### 5.6 Préréglages par profil [R]
Au premier paramétrage, l'établissement choisit un **profil** (public collège, public lycée général et technologique, public lycée professionnel, privé sous contrat collège/lycée, ensemble scolaire, hors contrat second degré, et plus tard école). Le profil **préremplit** (copie modifiable, jamais un lien figé) : périodisation par défaut (trimestres ; semestres possibles), grille type M1–M4/S1–S4, jours ouvrés (mercredi après-midi libre), niveaux de la nomenclature, matières courantes du niveau avec leur code national, offre type (LV1, LV2 en 5e…) [NV], critères de constitution, contrôles du tableau de bord actifs (codes nationaux obligatoires ou non). Après coup, rien ne dépend du profil : seul le paramétrage compte.

---

## 6. Frontières

### 6.1 Tableau

| Sujet | Socle 2 (élève) | **Socle 5 (ce document)** | Socle 6 (personnels, services) | Autres |
|---|---|---|---|---|
| Profil, identité de l'établissement, sites, UAI, bâtiments, salles | — | **possède** | — | 1 (tenant, domaine), 44 réservations, prêts |
| Années, états, ouverture, clôture, réouverture | — | **possède** ; chaque socle fournit ses contrôles | — | tous lisent |
| Calendrier (vacances, fériés, fermetures, journées banalisées, examens, stages au niveau d'une classe), types de semaine | — | **possède** | — | 10 neutralise les séances ; 39 possède les périodes de stage **par élève** ; 38 possède les épreuves |
| Annulation de séances (grève d'un professeur, sortie) | — | — | — | **10** (séances), 15, 42 |
| Périodes, périodisations | — | **possède** les dates et l'assignation | — | **21** possède l'état de saisie (ouverte, fermée) ; 23 les conseils |
| Grille horaire, coupure de demi-journée | — | **possède** | — | 10 l'utilise ; 12 dérive les demi-journées |
| Niveaux, MEF, formations, matières, offre d'enseignements, maquette, dotation | — | **possède** | — | 9 (imports BCN), 21 (coefficients) |
| Classes, groupes, ateliers et leur composition | — | **possède** | responsables : désignés ici (fait de structure), droits par le moteur (1) | 43 possède inscriptions et facturation des activités |
| Inscription annuelle (niveau, MEF, état), scolarisation | **possède** | lit ; reçoit niveau N+1 de la montée | — | 3 déclenche |
| **Enseignements suivis** par l'élève | lit (onglet scolarité) | **possède** [R, Q-S2] | — | 3 écrit les vœux, puis appelle le port à la validation |
| **Affectations** aux classes et groupes | lit (« scolarité à la date D ») | **possède** (table et commandes) [R, Q-S1] | — | 3 propose (arrivée en cours d'année) ; 2 appelle la fermeture à la sortie |
| **Cours** (ce qui est enseigné, à quel public, combien d'heures), barrettes | — | **possède** | **services** : qui intervient dans un cours, quotité, rôle | 10 place les cours ; 20, 21, 12 s'y rattachent |
| Professeur principal, équipes, référents | — | — (le **cours** et la **classe** sont les cibles) | **possède** [R, Q-S4] | — |
| Règle « qui voit quels élèves » | — | **fournit** les faits (cours, groupes, affectations) et le calcul de périmètre | fournit les services | **1** applique (moteur `autoriser`) |
| Constitution des classes | — | **possède** | les professeurs y donnent leur avis | 23 fournit les décisions de passage, 3 les admis |
| Décisions de passage | — | consomme | — | **23** |
| Créneaux, séances, exceptions | — | — | — | **10** (la génération des séances, aujourd'hui dans `referentiel_seances.rs`, lui revient : Q-S11) |
| Échanges nationaux (cadre d'import, traces) | fournit le contenu de ses exports | **fournit** les correspondances de structure | idem | **9** possède le cadre |

### 6.2 Règles de frontière [R]
1. **Le socle 5 possède ses tables** ; les autres socles appellent ses ports (§6.3) ou réagissent à ses événements. Un test de garde (même principe que `03_INSCRIPTIONS.md` §5.2 point 5) échoue si un autre module écrit dans `ent_classe`, `ent_groupe`, `ent_affectation`, `ent_cours`, `ent_calendrier_*`, `ent_periode*`.
2. **Le socle 5 n'écrit jamais chez les autres.** L'ouverture d'année d'aujourd'hui met à jour `ent_eleve.classe` et `classe_id` [F `referentiel_annee.rs:255-268`] : dans la cible, ces projections sont maintenues par la base à partir d'`ent_affectation`, puis retirées (plan Q14, `02_ELEVE.md` §15 M11).
3. **La génération des séances** est un fait de l'emploi du temps : elle part au socle 10 (code déplacé, pas réécrit), qui lit calendrier, grille et types de semaine du socle 5.

### 6.3 Ports exposés (fonctions Rust prenant la transaction de l'appelant, `&mut TenantTx`)

| Port | Appelé par | Sens |
|---|---|---|
| `structure::annee_a_la_date(date) -> Annee` / `annee_courante()` / `annees_vivantes()` | tous | lecture ; **ne crée jamais** |
| `structure::scolarite_structurelle(eleve_id, date) -> { classe, groupes, cours, enseignements_suivis }` | 2 (fiche), 10, 12, 20, 21, 24 | lecture |
| `structure::public_du_cours(cours_id, date) -> Vec<eleve_id>` / `membres_du_groupe(groupe_id, date)` | 10, 12, 20, 21, 43 | lecture |
| `structure::perimetre(personne, date) -> Perimetre { eleves, cours, groupes, classes }` | 1 (moteur), 6, 8, 16 | lecture ; **seule implémentation** de la règle du §8 |
| `structure::jour(date, cible) -> Jour { ouvre, type, demi_journees, semaine_type }` / `jours_ouvres(plage, cible)` | 10, 12, 24, 40, 41, 43 | lecture |
| `structure::periode(date, classe) -> Periode` / `periodes(annee, classe)` | 21, 23, 24 | lecture |
| `structure::grille(date, site, niveau) -> Grille` | 10, 12 | lecture |
| `structure::affecter(eleve_id, cible, date, motif)` | 3 (arrivée en cours d'année) | écriture |
| `structure::enregistrer_enseignements_suivis(eleve_id, annee_id, choix, source)` | 3 (validation, réinscription) | écriture |
| `structure::fermer_affectations(eleve_id, date, motif)` | 2 (ELV-11 sortie) | écriture |
| `structure::monter(eleve_id, annee_cible, niveau_id)` → appelle `eleve::enregistrer_inscription_annuelle` (socle 2) | 23 (décision de passage) via la commande de montée | écriture composée |
| `structure::controles_rentree(annee)` / trait `ControleRentree` implémenté par chaque socle | tableau de bord | lecture |
| `structure::controles_cloture(annee)` / trait `ControleCloture` | clôture | lecture |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition | Exemple |
|---|---|---|
| **Établissement** | Le tenant (un sous-domaine, D-012), avec son profil | « Ensemble scolaire Saint-X » |
| **Site** | Un lieu, porteur d'un UAI s'il en a un | « Collège », « Lycée », « Annexe » |
| **Année scolaire** | Intervalle de dates (non forcément septembre–août), avec un état | 2027-2028 |
| **Jour** | Le fait calendaire d'une date pour une cible : ouvert ou non, pourquoi, quelles demi-journées, quel type de semaine | « 12/01, site Lycée : fermeture exceptionnelle » |
| **Type de semaine** | Étiquette d'une semaine (A, B, ou toute autre) | « semaine A » |
| **Périodisation** | Un découpage nommé de l'année, assigné à des niveaux, MEF ou classes | « Trimestres », « Semestres terminale » |
| **Période** | Un intervalle d'une périodisation | « T2 : 1er déc. – 10 mars » |
| **Grille horaire** | Liste de créneaux nommés d'une journée type, versionnée, ciblée | « M1 8 h 00 – 8 h 55 » |
| **Niveau** | Échelon de scolarité (6e… Tle, plus tard CP…) avec cycle | 5e, cycle 4 |
| **MEF / formation** | Module élémentaire de formation national [S] ; une formation suivie | « 1re générale » |
| **Matière** | Discipline, nommée par l'établissement, code national facultatif | « Espagnol » |
| **Enseignement proposé** (offre) | Ce que l'établissement propose pour un niveau ou un MEF : une matière (ou un dispositif) dans une **modalité** (commun, langue de rang 1/2/3, option obligatoire ou facultative, spécialité, dispositif, accompagnement), avec capacité et règles de choix | « Espagnol LV2 en 4e », « Latin (option facultative) 5e », « Spécialité NSI 1re » |
| **Enseignement suivi** | Le choix daté d'un élève pour un enseignement proposé | « Léa suit Espagnol LV2 depuis le 1er sept. » |
| **Classe** (division) | L'unité administrative de rattachement : chaque élève scolarisé a **une** classe à une date | « 4B » |
| **Groupe** | Un ensemble d'élèves à une date, de **nature** générique : *enseignement* (public d'un cours : LV2, option, spécialité, demi-groupe, groupe de besoin), *dispositif* (ULIS, UPE2A [NV]), *activité* (atelier, club, AS, étude), *liste* (délégués, élèves d'un voyage — sans droits) | « 4AB-Espagnol », « Atelier théâtre » |
| **Règle de composition** | La définition d'un groupe à partir de faits (classes sources × enseignement suivi) ; elle **propose**, une personne valide | « élèves de 4A et 4B qui suivent Espagnol LV2 » |
| **Affectation** | L'appartenance datée d'un élève à une classe ou un groupe | « Léa ∈ 4AB-Espagnol du 1/9 au … » |
| **Cours** | Une unité d'enseignement : un enseignement (ou une matière) donné à un **public** (une classe ou un groupe), avec un volume horaire. Support des créneaux, des séances, des notes, du cahier de textes | « Espagnol LV2 — 4AB-Espagnol — 2 h 30 » |
| **Barrette** (alignement) | Des cours qui doivent avoir lieu en même temps (pour que les élèves d'une classe se répartissent) | « LV2 4e : Allemand 4AB, Espagnol 4A, Espagnol 4B » |
| **Maquette** | Les volumes horaires prévus par enseignement et par niveau ou MEF | « Maths 4e : 3 h 30 » [NV] |
| **Service** (socle 6) | La participation datée d'une personne à un cours, avec rôle et quotité | « M. X, titulaire, Espagnol 4AB » |
| **Dotation** | Les heures attribuées à l'établissement pour l'année (public, privé sous contrat) [NV] | — |

### 7.2 Trois distinctions qui manquent aujourd'hui

1. **Choix ≠ groupe.** `02_ELEVE.md` §7.4 recommandait « option = groupe ». Ce document propose de **séparer** [R, Q-S2] : l'élève *suit* un enseignement (son choix, qui sert aux exports nationaux, aux vœux, à la facturation d'une option payante) ; il est *membre* d'un groupe (l'organisation concrète). La relation entre les deux est une **règle contrôlée** : tout élève qui suit un enseignement à public de groupe doit être dans exactement un groupe de cet enseignement. On gagne le contrôle de cohérence le plus utile de la rentrée, et on supprime quand même les deux tables d'options parallèles d'aujourd'hui (elles deviennent des enseignements suivis).
2. **Cours ≠ service.** Un cours existe sans professeur (poste non pourvu, recrutement en cours) ; deux personnes peuvent intervenir dans un cours (co-intervention, assistant de langue) ; le volume horaire est une propriété du cours, la quotité une propriété du service.
3. **Ouvert ≠ clos.** Une année peut être terminée sans être clôturée (§4.4).

---

## 8. La règle de visibilité du professeur (D-022)

### 8.1 La règle [R]
Pour une personne `p` à la date `d`, la portée **« mes classes »** (nom conservé pour la compatibilité ; libellé à l'écran : « mes élèves ») est l'ensemble des élèves `e` tels qu'il existe un **cours** `c` où :
- `p` a un **service** actif à `d` sur `c` (socle 6 ; titulaire, co-intervenant, remplaçant — remplacement daté du socle 15), **et**
- `e` appartient au **public** de `c` à `d` : membre de la classe si le public est une classe, membre du groupe si le public est un groupe.

S'y ajoutent, par des faits de structure explicites :
- **Professeur principal** (et tout autre fonction attachée à une classe par le socle 6) : toute la classe [R, Q-S5].
- **Responsable de groupe** : portée « mes groupes » (inchangée), qui inclut désormais les membres **actifs à la date** (filtre sur `date_fin`, correction du défaut n° 18).

Et rien d'autre : ni l'ancienne jointure par texte de classe, ni « une classe où j'ai un groupe ».

**[personas]** Le « public » du cours inclut les **membres individuels** datés (R-S5 : élève d'ULIS inclus, décloisonnement, accueil temporaire) et en retire les exclus ; un élève exclu de la saisie (échange, scolarité partagée) n'apparaît plus dans l'appel du cours pour ces dates. Cette fonction est **la seule** (Q-S21) : le socle 6 lui fournit les interventions, il ne recalcule pas le périmètre.

### 8.2 Conséquences
- Classe mixte allemand/espagnol : le cours d'allemand a pour public le groupe « 4AB-Allemand » ; son professeur voit ce groupe. Le cours de mathématiques a pour public la classe ; son professeur voit la classe.
- Demi-groupes de sciences : deux cours (ou un cours par demi-groupe) ; le professeur qui n'a que le demi-groupe A voit le demi-groupe A.
- Un cours de **classe entière** dont certains élèves sont dispensés (EPS) : le professeur voit la classe ; la dispense est un fait du socle 12.
- L'élève qui change de groupe au 4 novembre : le professeur de l'ancien groupe le voit **pour les dates antérieures** (ses notes, ses absences du groupe) et plus après [R] ; c'est ce qui permet de finir un bulletin de trimestre honnêtement. Règle : la portée s'évalue à la **date de la donnée** (une absence du 10 octobre) et à la date du jour pour l'annuaire.
- Liste d'appel et liste de saisie des notes : `public_du_cours(c, date de la séance ou de l'évaluation)`.

### 8.3 Mise en œuvre [R]
- Une **seule** fonction SQL/Rust `structure::perimetre` (et son prédicat unitaire `suit(p, e, d)`), appelée par `droits_repo` et `perimetre.rs` ; la branche emploi du temps historique (`perimetre.rs:54-68, 127-139`) est retirée dès que chaque créneau pointe vers un cours (lot S4) ; **avant** son retrait, elle est restreinte : un créneau dont le service vise un groupe n'ouvre que ce groupe.
- **Tests de séparation** (D-022) dans la matrice d'isolation : professeur de groupe dans une classe mixte (ne voit pas l'autre groupe, sur toutes les routes élève : appel, notes, appréciations, cahier, vie scolaire, dossier) ; demi-groupe ; groupe inter-classes ; changement de groupe à une date ; remplaçant daté ; professeur principal ; responsable d'atelier ; élève sorti.
- `GET /referentiel/classes/{id}` : la **désignation** des élèves (nom, classe, photo) reste un annuaire visible du personnel (`02_ELEVE.md` Q-E3) ; tout ce qui va au-delà suit la portée.

---

## 9. Modèle de données cible

Toutes les tables portent `etablissement_id NOT NULL` ; clés étrangères **composées** `(etablissement_id, id)` et, pour les objets d'une année, `(etablissement_id, annee_id, id)`, ajoutées `NOT VALID` puis validées (même méthode que `02_ELEVE.md` §15 M1) ; RLS (permissive puis stricte, comme le socle 1) ; colonnes de création et d'auteur ; aucune suppression physique d'un objet ayant servi (fermeture datée).

### 9.1 Profil, paramètres, sites, bâtiments

```
ent_etablissement_profil                     -- 1:1 avec ent_etablissement (096)
  etablissement_id PK
  nom_officiel, nom_usage, logo_document_id NULL
  secteur          TEXT NOT NULL CHECK (secteur IN ('public','prive_sous_contrat','prive_hors_contrat'))
  contrat_type     TEXT NULL            -- [NV] contrat d'association / simple (privé sous contrat)
  degres           TEXT[] NOT NULL      -- {'premier','second','post_bac'}
  types            TEXT[] NOT NULL      -- {'college','lycee_gt','lycee_pro','ecole',...} : préréglages, pas de logique
  academie_code    TEXT NULL, departement_code TEXT NULL
  zone_calendrier  TEXT NULL            -- code de la source officielle (zone A/B/C, Corse, académie d'outre-mer…) [NV]
  fuseau_horaire   TEXT NOT NULL DEFAULT 'Europe/Paris'   -- IANA
  regime_local     TEXT NULL            -- 'alsace_moselle' : fériés supplémentaires [NV]
  organisme_gestionnaire_id UUID NULL   -- → ent_organisme (socle 4, §8.2) pour le privé
  tutelle_libelle  TEXT NULL
  jours_ouvres     JSONB NOT NULL       -- {lun:[matin,apres_midi], mer:[matin], …}
  profil_preregle  TEXT NULL            -- profil choisi au départ (§5.6), information seulement

ent_parametre                                -- succède à ent_param_etab (076) : clés connues, typées
  etablissement_id, cle TEXT, valeur JSONB, socle_proprietaire SMALLINT, modifie_par, modifie_le
  PK (etablissement_id, cle)
  -- registre des clés dans le code (type, défaut, socle propriétaire, validation) ; clé inconnue refusée

ent_site (existant 110, enrichi)
  + adresse_id NULL (modèle d'adresse du socle 4), + degres TEXT[], + couleur, + fuseau_horaire NULL (exception)
  + ferme_le DATE NULL (au lieu de actif)

ent_batiment
  id, etablissement_id, site_id, code, libelle, ferme_le

ent_salle (existant 076, enrichi)
  + site_id (FK, remplace site TEXT), + batiment_id NULL, + etage TEXT NULL
  + type TEXT NOT NULL DEFAULT 'banalisee'     -- générique : banalisee, laboratoire, informatique, gymnase, atelier,
                                                -- cdi, musique, arts, amphitheatre, exterieur, autre
  + equipements TEXT[]                          -- libellés libres de l'établissement (vidéoprojecteur, paillasses…)
  + accessible_pmr BOOLEAN NULL, + reservable BOOLEAN DEFAULT false (lu par le socle 44)
  + ferme_le DATE NULL
ent_salle_indisponibilite
  id, salle_id, du TIMESTAMPTZ, au TIMESTAMPTZ, motif_code, motif TEXT NULL, par
```

### 9.2 Années et opérations

```
ent_annee_scolaire (existant 092/110, resserré)
  etat TEXT NOT NULL CHECK (etat IN ('preparation','courante','terminee','cloturee'))   -- succède à statut/courante
  date_debut, date_fin                           -- bornes de l'année (facturation, droits)
  date_pre_rentree DATE NULL, date_fin_cours DATE NULL
  ouverte_le, terminee_le, cloturee_le TIMESTAMPTZ NULL
  ouverture_programmee_le TIMESTAMPTZ NULL
  rouverte_jusqu_au TIMESTAMPTZ NULL, rouverture_perimetre TEXT[] NULL
  precedente_id UUID NULL                       -- chaînage N-1 → N
  -- transitions autorisées par trigger : preparation→courante ; courante→terminee (uniquement à l'ouverture de la suivante) ;
  -- terminee→cloturee ; cloturee→(réouverture temporaire, état inchangé, rouverte_jusqu_au posé) ; aucune autre
  -- une seule courante (index existant) ; pas de chevauchement (EXCLUDE gist, btree_gist) ; libellé unique

ent_annee_rentree_niveau
  annee_id, niveau_id, date_rentree DATE, heure TIME NULL     -- rentrée échelonnée

ent_annee_operation                             -- journal métier des gestes d'année (successeur de l'idée annee_bascule [F Next])
  id, etablissement_id, annee_id, type TEXT     -- creation, reconduction, montee, constitution_validee, ouverture,
                                                -- ouverture_programmee, terminaison, cloture, rouverture, import, export
  apercu JSONB, rapport JSONB, controles_ignores JSONB NULL, force BOOLEAN, motif TEXT NULL,
  par, demande_le, execute_le, statut ('apercu','programmee','executee','annulee','echouee')
```

### 9.3 Calendrier, semaines, périodes, grilles

```
ref_calendrier_officiel                         -- GLOBAL (hors tenant), tenu par ScolIA, lecture seule pour les établissements
  id, zone TEXT, annee_libelle TEXT, type ('vacances','ferie','pont_national'), libelle, du DATE, au DATE,
  source TEXT, source_version TEXT, importe_le          -- source : jeu de données ouvert du ministère [NV]
  -- jours fériés : calculés par le code (Pâques, Ascension, Pentecôte) + régimes locaux, versionnés, testés

ent_calendrier_evenement                        -- remplace ent_periode_vacances (073) et ent_jour_calendrier (110)
  id, etablissement_id, annee_id
  type TEXT NOT NULL CHECK (type IN ('vacances','ferie','pont','fermeture','fermeture_exceptionnelle',
       'journee_banalisee','journee_pedagogique','rattrapage','examen','stage','periode_entreprise',
       'rentree','pre_rentree','evenement'))
  libelle, du DATE, au DATE, du_demi ('matin','apres_midi') NULL, au_demi NULL
  effet TEXT NOT NULL CHECK (effet IN ('sans_cours','sans_eleves','cours_modifies','informatif'))
  origine ('officiel','calcule','saisie','import'), ref_officiel_id NULL
  publie_familles BOOLEAN, motif TEXT NULL, annule_le NULL, par
ent_calendrier_cible                            -- vide = tout l'établissement
  evenement_id, site_id NULL | niveau_id NULL | classe_id NULL | groupe_id NULL   (une colonne exactement)

ent_semaine_type_regle
  annee_id, etiquettes TEXT[] ('A','B'), mode ('alternance_continue','reprise_apres_vacances'), premiere_semaine DATE
ent_semaine                                     -- matérialisée et modifiable ; remplace ent_edt_calendrier_ab (048)
  annee_id, lundi DATE, etiquette TEXT NULL, source ('calcule','force')

ent_periodisation
  id, annee_id, code, libelle, usage TEXT NOT NULL CHECK (usage IN ('evaluation','autre'))
ent_periodisation_cible                         -- la plus précise gagne : classe > MEF > niveau > défaut
  periodisation_id, niveau_id NULL | mef_id NULL | classe_id NULL | defaut BOOLEAN
ent_periode (existant 110) + periodisation_id ; niveau_id migré en cible ; contrainte « dans l'année »
  -- ent_note_periode (022) : reliée par periode_id (déjà fait, 112), unicité passée par année ; son état de saisie reste au socle 21

ent_grille                                       -- versions de grille
  id, etablissement_id, code, libelle, valable_du DATE, valable_au DATE NULL, coupure_demi_journee TIME
ent_grille_cible   grille_id, site_id NULL, niveau_id NULL, jours SMALLINT[] NULL
ent_grille_creneau (succède à ent_grille_horaire 110) grille_id, code, libelle, heure_debut, heure_fin,
  nature ('cours','recreation','pause_meridienne','etude'), ordre UNIQUE par grille
```

Lecture : `jour(date, cible)` combine profil (jours ouvrés), calendrier officiel adopté, événements ciblés, semaine ; c'est **la** réponse à « y a-t-il cours ? » pour les socles 10, 12, 24, 40.

### 9.4 Niveaux, MEF, nomenclatures

```
ref_nomenclature                                 -- GLOBAL, tenu par ScolIA (lot IOP-L0 du socle 9)
  type TEXT ('matiere','mef','mefstat','modalite_election','formation_diplome','niveau','dispositif',...)
  code, libelle_court, libelle_long, valide_du, valide_au, attributs JSONB, source, version
ent_niveau (existant)  + ordre UNIQUE, + degre ; code national facultatif
ent_mef    (existant)  + code_national lié à ref_nomenclature (vérifié à l'import) ; ent_filiere.mef TEXT → mef_id
```
Les objets de l'établissement **référencent** un code national (texte + type) sans en dépendre : une nomenclature retirée ne casse rien, elle déclenche un contrôle.

### 9.5 Matières, offre, maquette, dotation

```
ent_matiere (existant 008, enrichi)
  + libelle_court, + couleur, + famille TEXT NULL (regroupement du bulletin), + parent_id NULL (sous-matière : « Français – oral »)
  + code_national NULL (BCN), + archivee_le ; code unique par établissement (existant)

ent_offre_enseignement                           -- l'enseignement proposé, PAR ANNÉE
  id, etablissement_id, annee_id
  matiere_id NULL           -- NULL pour un dispositif sans matière (« Devoirs faits », « Vie de classe »)
  libelle, modalite TEXT NOT NULL CHECK (modalite IN ('commun','langue_1','langue_2','langue_3','option_obligatoire',
       'option_facultative','specialite','dispositif','accompagnement','autre'))
  code_modalite_election NULL [NV]
  hors_contrat BOOLEAN DEFAULT false              -- privé sous contrat : non exporté
  capacite INTEGER NULL, ouvert_aux_voeux BOOLEAN   -- lu par le socle 3
  public_type TEXT CHECK (public_type IN ('classe','groupe'))   -- comment il est organisé
ent_offre_cible   offre_id, niveau_id NULL | mef_id NULL
ent_offre_regle_choix                             -- paramétrage, pas de code
  id, annee_id, niveau_id NULL, mef_id NULL, libelle,
  offres UUID[], min SMALLINT, max SMALLINT, bloquant BOOLEAN   -- « LV2 : exactement 1 », « spécialités 1re : exactement 3 » [NV]
ent_offre_incompatibilite  offre_a, offre_b, motif   -- « latin et grec sur la même barrette »

ent_maquette_horaire
  annee_id, offre_id, niveau_id NULL, mef_id NULL, heures_eleve NUMERIC(4,2), heures_dedoublement NUMERIC NULL, semaines TEXT
ent_dotation                                       -- public, privé sous contrat [NV : nature exacte des heures]
  annee_id, nature ('heures_poste','heures_supplementaires','autre'), heures NUMERIC, source, notifiee_le, document_id NULL
```
Besoins en heures = Σ (cours × heures) ; le tableau de bord compare besoins et dotation ; **aucune règle nationale codée** (les volumes de référence sont proposés par le profil, modifiables).

### 9.6 Classes, groupes, composition

```
ent_classe (existant, enrichi)
  + capacite NULL, + salle_reference_id NULL, + ouverte_le DATE, + fermee_le DATE NULL (remplace archivee_le)
  + reconduite_de_id NULL
  niveau TEXT supprimé (après migration) ; niveau_id reste le niveau principal
ent_classe_niveau  classe_id, niveau_id      -- 0..n niveaux (classes multi-niveaux, premier degré)
ent_classe_mef     classe_id, mef_id         -- 0..n MEF (classe mixte de deux formations)

ent_groupe (existant, enrichi)
  + nature TEXT NOT NULL CHECK (nature IN ('enseignement','dispositif','activite','liste'))
  + offre_id NULL                              -- l'enseignement qu'il porte (nature enseignement)
  + capacite NULL, + ouvert_le, + ferme_le, + reconduit_de_id
  + regle_composition JSONB NULL               -- {classes:[…], offre_id, filtre…} : propose, ne s'applique pas seul
  + ouvre_droits BOOLEAN NOT NULL               -- false pour 'liste' : aucun droit n'en découle
  classe_id (existant) → migré vers ent_groupe_classe, puis retiré
ent_groupe_classe  groupe_id, classe_id      -- 0..n classes d'origine
ent_groupe_responsable (existant 098) + role TEXT (libellé libre), + du DATE, + au DATE NULL

ent_affectation (existant 092, resserré)
  + inscription_id NULL (02 §7.4), + motif_code, + decide_par
  FK composées (etab, annee, classe) / (etab, annee, groupe) ; date_debut/date_fin dans l'année (trigger)
  plus jamais de suppression : une affectation erronée est **annulée** (annulee_le, motif), pas effacée
  -- contrôle (avertissement, pas blocage) : membre d'un groupe dont aucune classe d'origine n'est la sienne

ent_eleve_enseignement                         -- enseignements suivis (succède à ent_eleve_option, _option_annee)
  id, etablissement_id, eleve_id, annee_id, offre_id, rang SMALLINT NULL,
  etat ('voeu','retenu','abandonne'), du DATE, au DATE NULL, source ('reinscription','admission','saisie','import','reprise'),
  source_ref UUID NULL
  UNIQUE partiel (eleve_id, offre_id) WHERE etat='retenu' AND au IS NULL
```

### 9.7 Cours et barrettes
*[S4, code — §24]* `ent_cours` existe (migration 341) ; `ent_service_enseignement.cours_id` et `ent_edt_creneau.cours_id` aussi ; `ent_cours_membre` (S1) tient lieu de `ent_cours_membre_individuel`. Les barrettes (`ent_cours_alignement`) ne sont pas faites.

```
ent_cours
  id, etablissement_id, annee_id
  offre_id NULL, matiere_id NULL, libelle                -- au moins l'un des deux ; libellé affiché
  classe_id NULL, groupe_id NULL                         -- public : exactement un
  heures_hebdo NUMERIC(4,2) NULL, semaines TEXT NULL     -- 'toutes', 'A', 'B'…
  ouvert_le DATE, ferme_le DATE NULL, reconduit_de_id NULL
  codes_export JSONB NULL                                -- correspondances nationales si différentes
ent_cours_alignement  id, annee_id, libelle ; ent_cours_alignement_membre (alignement_id, cours_id)

-- socle 6 (cible) : ent_service(personne_id|user_id, cours_id, role, quotite, du, au)
-- transition : ent_service_enseignement (092) reçoit cours_id ; chaque ligne existante devient un cours (matière × cible)
--             + un service ; vue de compatibilité tant que les lecteurs ne sont pas migrés (§17, M7)
```
Les créneaux d'EDT (`ent_edt_creneau.cours_id`, socle 10), les évaluations (`ent_devoir.cours_id`, socle 21) et le cahier de textes (socle 20) se rattachent au **cours**.

### 9.8 Constitution des classes

```
ent_constitution_campagne
  id, etablissement_id, annee_id (cible), niveau_id, libelle, etat ('preparation','avis','calcul','validee','publiee','purgee'),
  criteres JSONB (poids choisis), souhaits_familles BOOLEAN DEFAULT false, avis_jusqu_au, publication_le, purge_le
ent_constitution_classe       campagne_id, classe_id, capacite, offres_accueillies UUID[]
ent_constitution_eleve        campagne_id, eleve_id, indicateur_niveau SMALLINT NULL (1..4), accompagnement BOOLEAN,
                              classe_verrouillee_id NULL
ent_constitution_contrainte   id, campagne_id, type ('separer','regrouper','classe_imposee','classe_exclue'),
                              eleves UUID[], classe_id NULL, motif_code, note_scellee BYTEA NULL, par, le, retiree_le
ent_constitution_souhait      campagne_id, eleve_id, camarades UUID[] (max paramétré), depose_par_personne_id, le
ent_constitution_scenario     id, campagne_id, graine, repartition JSONB, indicateurs JSONB, violations JSONB, par, le, retenu BOOLEAN
```
La validation écrit les affectations par la commande `affecter` (source `constitution`) ; la purge efface contraintes, notes scellées, indicateurs et souhaits (ne garde que : campagne, scénario retenu sans données sensibles, journal).

### 9.9 Ce qui est repris ou retiré

| Existant [F] | Devenir [R] |
|---|---|
| `ent_annee_scolaire.courante`, `statut` | `etat` (vue de compatibilité pour `courante`) |
| `ent_periode_vacances` (073), `ent_jour_calendrier` (110) | → `ent_calendrier_evenement` ; une `fermeture` reste une fermeture |
| `ent_edt_calendrier_ab` (048) | → `ent_semaine` (calculée depuis `reference_lundi`) |
| `ent_grille_horaire` (110) | → `ent_grille` + `ent_grille_creneau` (une version « depuis toujours » par site) |
| `ent_periode.niveau_id` | → `ent_periodisation_cible` |
| `ent_param_etab` (076), `ent_sortie_param` (065) | → `ent_parametre` (clés enregistrées) |
| `ent_salle.site` texte | → `site_id` (rapprochement par libellé, rejets listés) |
| `ent_classe.niveau` texte, `archivee_le` | → `niveau_id`, `fermee_le` |
| `ent_groupe.classe_id` | → `ent_groupe_classe` |
| `ent_eleve_option`, `ent_eleve_option_annee` | → `ent_eleve_enseignement` + offres créées depuis les codes rencontrés |
| `ent_service_enseignement` | → `ent_cours` + service (socle 6) |
| `ent_filiere.mef` texte | → `mef_id` |
| Fonctions qui créent (`ent_assurer_annee`, `ent_resoudre_classe`, `referentiel_assurer_annee`, `sync_affectation_classe`) | création retirée ; résolution qui **échoue** sur inconnu (le trigger de double écriture garde la lecture tant que Q14 n'est pas fini) |

### 9.10 Compléments de modèle [personas]

Pseudo-DDL, mêmes règles que §9 (établissement, clés composées, RLS, aucune suppression physique). **Numéros de migration à attribuer** dans `MIGRATIONS.md` (la plage 330–349 est déjà entièrement répartie, §18).

```
-- Point A de PERSONAS_05-06 §0 (Q-S20) : si le cours et le service d'enseignement du socle 6 deviennent une seule table,
-- les colonnes ci-dessous s'appliquent à cette table unique.

ent_cours_membre_individuel                  -- R-S5 : inclusions et exclusions datées (ULIS, décloisonnement, échange, accueil)
  id, etablissement_id, annee_id, cours_id
  eleve_id NULL, personne_accueillie_id NULL -- élève inscrit, ou personne accueillie temporairement (socle 2 : statut d'accueil)
  nature TEXT CHECK (nature IN ('inclusion','exclusion'))
  du DATE, au DATE NULL, motif_code TEXT     -- ulis, upe2a, decloisonnement, avance, parcours_amenage, scolarite_partagee,
                                             -- echange, accueil_temporaire, dispense_cours, autre
  annulee_le TIMESTAMPTZ NULL
  -- public_du_cours(c, d) = (classe ou groupe à d) ∪ inclusions actives − exclusions actives

ent_calendrier_evenement (§9.3) + services_touches TEXT[] NULL
  -- R-S1 : {cours, accueil_eleves, restauration, internat, transports, secretariat, intendance, accueil, cdi}
  -- effet 'accueil_seulement' ajouté (établissement ouvert, cours non assurés)
  + levee_prevue_le TIMESTAMPTZ NULL, + conditionnelle BOOLEAN DEFAULT false   -- STR-94 (alerte météo, préfecture)
  + publie_personnels BOOLEAN DEFAULT false                                     -- R-S19
ent_calendrier_cible + service TEXT NULL     -- STR-93 : fermeture ou ouverture d'un service seul (secrétariat fermé en août)
ref_calendrier_officiel.type + 'examen_national'                                -- STR-114 [NV : source]

ent_lieu                                     -- STR-109, R-S14 : lieux extérieurs conventionnés
  id, etablissement_id, libelle, adresse_id NULL, proprietaire TEXT NULL, convention_document_id NULL,
  trajet_minutes SMALLINT NULL, mode_trajet TEXT NULL, ferme_le DATE NULL
ent_salle + lieu_id NULL                     -- une salle « extérieure » appartient à un lieu
ent_salle_disponibilite                      -- fenêtres attribuées (inverse des indisponibilités) : jour, heure_debut, heure_fin, du, au

ent_classe + code_national TEXT NULL         -- R-S10 : code de division de la base nationale, distinct du code affiché
ent_groupe + capacite_bloquante BOOLEAN DEFAULT false                           -- R-S15
ent_offre_enseignement + admission TEXT DEFAULT 'libre'
  CHECK (admission IN ('libre','sur_avis','sur_test','sur_dossier'))            -- section européenne, option sélective
ent_eleve_enseignement.etat + 'quitte' ; + fin_nature TEXT NULL
  CHECK (fin_nature IN ('changement_en_cours_annee','abandon_fin_premiere','fermeture_offre','depart','correction'))  -- R-S16

ent_demande_structure                        -- STR-100, STR-108 : demandes de changement d'enseignement ou de classe
  id, etablissement_id, annee_id, eleve_id, type ('enseignement','classe'), demande JSONB,
  deposee_par_personne_id NULL, deposee_par_user_id NULL, motif_code, avis JSONB (PP, professeurs), etat
  ('deposee','en_avis','acceptee','refusee','retiree'), decision_par, decide_le, effet_le DATE NULL, reponse_document_id NULL
  -- s'appuie sur le moteur de formulaires (18) ou de demandes (N1) quand ils existent ; ici : l'objet et ses effets

ent_structure_scenario                       -- STR-98 : scénarios de structure prévisionnelle
  id, etablissement_id, annee_id, libelle, contenu JSONB (classes par niveau, dédoublements, offres), besoins_heures NUMERIC,
  retenu BOOLEAN, presente_le DATE NULL, document_id NULL, par, le

ent_constat_effectifs                        -- STR-97, R-S4 : photographie figée
  id, etablissement_id, annee_id, date_reference DATE, version SMALLINT, chiffres JSONB (par niveau, MEF, classe,
  enseignement, régime), signe_par, signe_le, remplace_id NULL, document_id NULL
  -- jamais recalculé ; une correction = une nouvelle version

ent_constitution_contributeur                -- STR-107, R-S17 : avis des écoles d'origine
  id, campagne_id, personne_id, ecole_origine TEXT, eleves UUID[], invite_le, jeton_hash, expire_le, revoque_le
ent_constitution_eleve + choix_fratrie TEXT NULL CHECK (… IN ('ensemble','separes','indifferent'))   -- R-S8
ent_constitution_contrainte.type + origine TEXT DEFAULT 'saisie' CHECK (origine IN ('saisie','mesure'))  -- R-S7
  -- origine 'mesure' : posée par le socle 13 ou 36, motif jamais copié, non retirable par la campagne

ent_liste_favorite                           -- STR-112 : liste à la demande enregistrée
  id, etablissement_id, user_id, libelle, filtres JSONB (structure), colonnes TEXT[] (catalogue du socle 2), partagee BOOLEAN
```

---

## 10. Ce qui relève du logiciel, ce qui relève du paramétrage

| Sujet | Codé par ScolIA (non paramétrable) | Paramétré par l'établissement |
|---|---|---|
| Visibilité | La règle du §8 ; l'évaluation à la date ; le cloisonnement | Qui a quel service, qui est responsable de quel groupe, quelles portées dans quels rôles |
| Année | États et transitions ; une seule courante ; pas de chevauchement ; lecture seule après clôture ; contrôles fournis par les socles | Dates, rentrées par niveau, pré-rentrée, ouverture programmée, durée de réouverture |
| Calendrier | Types d'événements et effets génériques ; calcul des fériés ; adoption d'une source officielle | Zone, ponts, fermetures, journées banalisées, examens, cibles, publication |
| Semaines | Mécanisme d'étiquetage | Étiquettes (A/B ou autres), mode, exceptions |
| Périodes | Couverture sans trou ni chevauchement par périodisation ; résolution la plus précise | Nombre, noms, dates, assignations |
| Grilles | Non-chevauchement, versions datées | Créneaux, noms, coupure, cibles |
| Matières | Forme (libellé, abrégé, couleur, parent, code national facultatif) | Liste, noms, couleurs, regroupements |
| Offre | Modalités génériques ; règles de choix min/max ; incompatibilités | Enseignements proposés, capacités, règles, ouverture aux vœux |
| Classes et groupes | Natures génériques ; une classe par élève à une date ; cohérence choix ↔ groupe contrôlée | Noms, capacités, groupes, ateliers, règles de composition |
| Cours | Un public par cours ; rattachement des créneaux, notes, cahier | Cours, volumes, barrettes |
| Constitution | Catalogue de critères, algorithme, purge | Critères retenus, poids, souhaits familles oui/non, dates |
| Contrôles de rentrée | Catalogue de contrôles et leur gravité par défaut | Activation et gravité (bloquant / avertissement) de chacun |
| Échanges | Formats et correspondances | Quels échanges sont actifs |

---

## 11. Échanges avec l'extérieur

| Flux | Sens | Contenu | Règle [R] | Socle porteur |
|---|---|---|---|---|
| Nomenclatures BCN (matières, MEF, MEFSTAT, modalités d'élection, formations) [S, licence NV] | entrant | `ref_nomenclature` global | mise à jour centrale par ScolIA ; jamais d'écrasement d'un libellé de l'établissement | 9 (IOP-L0) |
| Calendrier scolaire officiel (jeu de données ouvert du ministère, par zone) [NV : jeu exact, licence, format] | entrant | `ref_calendrier_officiel` | import central versionné ; l'établissement **adopte** une version ; une nouvelle version propose les différences | 5 |
| Jours fériés | calculés | — | algorithme testé ; régimes locaux | 5 |
| Structures de la base élèves SIECLE (divisions, groupes, options des élèves) [S, format NV] | entrant (public, privé sous contrat) | classes, groupes, offres, enseignements suivis | via le cadre d'import (prévisualisation, différences, rejets, annulation d'un lot) ; une correction locale n'est jamais écrasée sans proposition | 9 |
| STS-Web (services, ventilation) [S, format NV, transition annoncée vers une autre application [S `INTEROPERABILITE_NATIONALE.md:118`]] | entrant / sortant | cours, services, codes de modalité | **attendre la confirmation du format** avant de coder (`INTEROPERABILITE_NATIONALE.md` §12) | 9, 6 |
| Logiciels d'emploi du temps (export de structure, import des créneaux) [H : formats propres à chaque éditeur] | sortant / entrant | classes, groupes, cours, barrettes, services, salles → créneaux | l'import **ne crée pas** de structure : il propose les écarts (groupe inconnu, salle inconnue) à valider ; correspondance par identifiants stables | 10 (import), 5 (export de structure) |
| GAR (affectations, divisions, groupes, enseignements) [S, format NV] | sortant (public) | structure et appartenances | consentement et minimisation selon le cadre GAR | 9 |
| Enquêtes et remontées d'effectifs [NV] | sortant | effectifs par niveau, MEF, enseignement, à une date | extraites de la structure datée ; export réservé aux administratifs (D-022) | 47, 9 |
| Calendrier des familles et des personnels (iCal) | sortant | événements publiés, rentrées, journées sans cours | lien personnel révocable ; rien de nominatif dans un calendrier public | 5 |
| Réversibilité | sortant | toute la structure, CSV/JSON documentés | fin de contrat | 27 |

---

## 12. Actions métier

Colonnes : identifiant ; action ; qui (profil par défaut · action de droit) ; règles et cas limites ; effet ; événement. ★ = confirmation renforcée (récapitulatif, motif, journal). Correspondance REF-xx entre parenthèses.

### 12.1 Établissement et lieux

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-01 | Renseigner l'identité (nom officiel, d'usage, logo, coordonnées) | Admin · `etablissement.parametrer` | logo via socle 7 | `referentiel.profil_modifie` |
| STR-02 | Choisir un profil et appliquer ses préréglages | Admin · idem ★ | copie modifiable ; aperçu de ce qui sera créé ; jamais réappliqué sur des données existantes | `referentiel.prereglages_appliques` |
| STR-03 | Renseigner secteur, académie, zone, fuseau, régime local, organisme gestionnaire, tutelle | Admin · idem | zone et fuseau déduits de l'UAI si la nomenclature le permet [NV] ; changer de fuseau en cours d'année = ★ | `referentiel.profil_modifie` |
| STR-04 | Définir les jours ouvrés et demi-journées de la semaine type | Admin, Dir · idem | effet à une date ; séances futures régénérées (10) | `referentiel.semaine_type_modifiee` |
| STR-05 | Créer, modifier, fermer un site (UAI, adresse, degrés, couleur) (REF-14) | Admin · idem | UAI au format national [NV] ; fermeture refusée s'il reste des classes ouvertes | `referentiel.site_cree` [F] / `_modifie` [F] / `_ferme` |
| STR-06 | Créer, fermer un bâtiment | Admin, Adm · idem | — | `referentiel.batiment_*` |
| STR-07 | Créer, modifier, fermer une salle (type, capacité, équipements, accessibilité, réservable) (REF-14) | Admin, Adm · idem | code unique ; fermeture refusée si des créneaux futurs l'utilisent (proposer de les déplacer, socle 10) | `referentiel.salle_*` |
| STR-08 | Importer des salles (collage, fichier du logiciel d'EDT) | Admin, Adm · idem | aperçu, rejets ligne à ligne, idempotent | `referentiel.salles_importees` |
| STR-09 | Déclarer une indisponibilité de salle (travaux) | Admin, Adm · idem | datée ; séances concernées signalées (10) | `referentiel.salle_indisponible` |
| STR-10 | Modifier un paramètre enregistré | selon la clé (socle propriétaire) | clé connue, valeur validée, historique | `referentiel.parametre_modifie` |

### 12.2 Années

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-11 | Créer une année (REF-01) | Dir, Adm · `annee.preparer` | dates libres, sans chevauchement ; libellé unique ; état préparation | `referentiel.annee_creee` [F] |
| STR-12 | Préparer l'année suivante : reconduire, élément par élément, avec aperçu (classes modèles, groupes durables **et leurs responsables**, ateliers, périodisations, grilles, offre, maquette, cours, barrettes, semaines A/B) | Dir, Adm · `annee.preparer` | idempotent ; une seule transaction pour création + reconduction (correction de la non-atomicité [F `referentiel_annee.rs:504`]) ; rapport | `referentiel.structures_reconduites` [F] |
| STR-13 | Consulter le tableau de bord de rentrée | Dir, Adm · `annee.preparer` (lecture : `referentiel.lire`) | contrôles fournis par les socles, avec gravité, lien vers l'écran qui corrige | — |
| STR-14 | Définir les dates de rentrée par niveau, la pré-rentrée, la fin des cours | Dir, Adm · `annee.preparer` | dans l'année | `referentiel.rentrees_definies` |
| STR-15 | Voir l'aperçu serveur de l'ouverture | Dir · `annee.basculer` | effets : classes, élèves affectés, élèves sans classe, cours sans intervenant, séances générées (10), contrôles | — |
| STR-16 | Ouvrir l'année (immédiatement ou à une date programmée) ★ (REF-02) | Dir · `annee.basculer` | contrôles bloquants passés ; N devient `terminee`, **pas clôturée** ; programmée : recontrôle à l'exécution | `referentiel.annee_ouverte` [F], `referentiel.annee_terminee` |
| STR-17 | Annuler une ouverture programmée | Dir · `annee.basculer` | avant exécution | `referentiel.ouverture_annulee` |
| STR-18 | Voir les contrôles de clôture | Dir, Adm · `referentiel.lire` | fournis par 21, 23, 24, 9… | — |
| STR-19 | Clôturer l'année ★ (REF-03) | Dir · `annee.basculer` | année `terminee` seulement ; contrôles ; forçage motivé avec liste des contrôles ignorés | `referentiel.annee_cloturee` [F] |
| STR-20 | Rouvrir temporairement une année clôturée ★ | Admin, Dir · `annee.rouvrir` | durée bornée, périmètre (socles concernés), journal ; refermeture automatique | `referentiel.annee_rouverte` / `_refermee` |
| STR-21 | Choisir son année de travail (préférence d'écran) | tout personnel | défaut selon le rôle et la saison (paramétrable) ; affichage permanent de l'année consultée | — |
| STR-22 | Corriger les dates d'une année en préparation | Dir, Adm · `annee.preparer` | refusé si des faits datés tomberaient hors de l'année | `referentiel.annee_modifiee` |

### 12.3 Calendrier et semaines

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-23 | Adopter le calendrier officiel de la zone pour une année (REF-05) | Dir, Adm · `calendrier.gerer` | crée les vacances et fériés (origine `officiel`) ; adoption d'une nouvelle version = différences à accepter | `referentiel.calendrier_adopte` |
| STR-24 | Ajouter un pont, une journée banalisée, pédagogique, d'examen, de stage, un événement, avec cible et effet | Dir, Adm · `calendrier.gerer` | cible : établissement, site, niveau, classe, groupe ; demi-journée possible ; chevauchements expliqués | `referentiel.calendrier_modifie` (dates, cibles, effet) |
| STR-25 | Déclarer une fermeture exceptionnelle ★ | Dir · `calendrier.gerer` (portée établissement) | aperçu des effets (séances, appels, repas, internat, familles à prévenir) ; motif ; possible le jour même | `referentiel.fermeture_exceptionnelle` |
| STR-26 | Lever une fermeture (annulation) | Dir · idem | effets inverses signalés aux socles | `referentiel.fermeture_levee` |
| STR-27 | Déclarer une journée de rattrapage (« le mercredi 3 est travaillé comme un lundi ») | Dir · `calendrier.gerer` | la journée prend l'emploi du temps d'un autre jour (10) [R] | `referentiel.calendrier_modifie` |
| STR-28 | Définir les types de semaine et leur calendrier ; forcer l'étiquette d'une semaine | Dir, Adm · `calendrier.gerer` (aujourd'hui `edt.gerer` [F]) | modes ; semaines de vacances sans étiquette ; séances futures régénérées | `referentiel.semaines_definies` |
| STR-29 | Publier le calendrier aux familles ; abonnement iCal | Dir, Adm · `calendrier.gerer` | seuls les événements marqués publiés ; lien personnel révocable | `referentiel.calendrier_publie` |
| STR-30 | Comparer le calendrier avec N-1 (jours ouvrés, semaines A/B) | personnel · `referentiel.lire` | — | — |

### 12.4 Périodes et grilles

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-31 | Créer une périodisation et ses périodes (REF-04) | Dir, Adm · `periodes.gerer` | couverture sans trou ni chevauchement ; modèles (3 trimestres, 2 semestres) | `referentiel.periodes_definies` [F] |
| STR-32 | Assigner une périodisation à des niveaux, MEF, classes | Dir, Adm · idem | chaque classe résout exactement une périodisation d'évaluation (contrôle) | `referentiel.periodisation_assignee` |
| STR-33 | Modifier les dates d'une période en cours | Dir · idem ★ | le socle 21 répond « notes hors période » ; le socle 23 signale les conseils planifiés | `referentiel.periode_modifiee` |
| STR-34 | Créer ou versionner une grille horaire (valable à une date), la cibler (REF-06) | Dir, Adm · `etablissement.parametrer` | non-chevauchement ; ordre unique ; séances futures régénérées (10) | `referentiel.grille_horaire_definie` [F] |
| STR-35 | Définir la coupure des demi-journées | Dir · idem | lue par le socle 12 (décompte) ; changement daté | `referentiel.grille_horaire_definie` |

### 12.5 Catalogue pédagogique

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-36 | Créer, modifier une matière (abrégé, couleur, famille, parent, code national) (REF-14) | Dir, Adm · `enseignements.gerer` (succède à `matieres.gerer` [F]) | code unique ; code national validé contre la nomenclature | `matiere.created` [F] / `referentiel.matiere_modifiee` |
| STR-37 | Importer des matières nationales (sélection dans la nomenclature) | idem | aperçu ; pas de doublon | `referentiel.matieres_importees` |
| STR-38 | Fusionner deux matières en doublon ★ | Dir · idem | les cours et évaluations suivent ; journal ; défusion possible 30 jours [R] | `referentiel.matieres_fusionnees` |
| STR-39 | Archiver une matière | Dir, Adm · idem | refusé si des cours ouverts l'utilisent | `referentiel.matiere_archivee` |
| STR-40 | Gérer niveaux et MEF ; importer les MEF (REF-07) | Dir, Adm · `enseignements.gerer` | existant [F] | `referentiel.niveau_cree` [F], `mef_importes` [F] |
| STR-41 | Définir l'offre d'enseignements d'une année par niveau ou MEF | Dir, Adm · idem | modalités génériques ; capacité ; ouverture aux vœux (lue par le socle 3) ; hors contrat | `referentiel.offre_modifiee` |
| STR-42 | Définir les règles de choix et incompatibilités | Dir, Adm · idem | min/max par groupe d'offres ; bloquant ou avertissement | `referentiel.offre_modifiee` |
| STR-43 | Fermer un enseignement proposé (option non ouverte faute d'effectif) | Dir · idem ★ | élèves qui l'avaient choisi listés ; proposition de choix de remplacement envoyée aux familles (socle 3 ou 8) | `referentiel.offre_fermee` |
| STR-44 | Définir la maquette horaire | Dir, Adm · idem | volumes de référence du profil modifiables | `referentiel.maquette_modifiee` |
| STR-45 | Saisir la dotation et comparer besoins / dotation | Dir · idem | public et privé sous contrat ; simulation « et si » (une classe de plus, un dédoublement) sans écriture | `referentiel.dotation_enregistree` |

### 12.6 Structure de l'année

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-46 | Créer une classe (niveaux, MEF, site, capacité, salle de référence) (REF-08) | Dir, Adm · `structure.gerer` | code unique dans l'année ; année non clôturée | `referentiel.classe_creee` |
| STR-47 | Modifier, renommer une classe | idem | renommer ne casse rien (identifiant stable) ; ancien code conservé pour l'historique et les exports | `referentiel.classe_modifiee` [F] |
| STR-48 | Fermer une classe (en cours d'année ou à l'archivage) | Dir · idem ★ | refusé s'il reste des élèves affectés à la date (aujourd'hui refus si élèves [F]) ; cours clos | `referentiel.classe_archivee` [F] |
| STR-49 | Scinder une classe / fusionner deux classes ★ | Dir · idem | assistant : répartition des élèves, duplication ou fermeture des cours, services à revoir (6) | `referentiel.classe_scindee` / `_fusionnee` |
| STR-50 | Créer un groupe (nature, classes d'origine, offre, capacité, règle) (REF-09) | Dir, Adm · `structure.gerer` | nature `liste` sans droits | `referentiel.groupe_cree` |
| STR-51 | Générer les groupes d'un enseignement (« un groupe d'espagnol par classe », « un groupe pour 4A+4B ») | idem | aperçu ; idempotent | `referentiel.groupes_generes` |
| STR-52 | Créer des demi-groupes d'une classe (alphabétique, alterné, équilibré, manuel) | idem | les deux demi-groupes couvrent la classe (contrôle) | `referentiel.groupe_cree` |
| STR-53 | Créer un atelier, club, groupe d'activité | Dir, Adm, CPE · `structure.gerer` ; ou `groupes.creer_activite` [R] | nature `activite` ; responsables | `referentiel.groupe_cree` |
| STR-54 | Désigner, retirer un responsable de groupe (rôle libre, daté) | Dir, Adm · `structure.gerer` (aujourd'hui `roles.gerer` [F]) | donne la portée « mes groupes » | `referentiel.responsable_groupe_change` |
| STR-55 | Modifier, fermer un groupe | idem | fermer = toutes les affectations closes à la date | `referentiel.groupe_modifie` / `_ferme` |
| STR-56 | Générer les cours depuis la maquette et la structure | Dir, Adm · `structure.gerer` | un cours par offre `commun` et par classe, un par groupe pour les offres à public de groupe ; aperçu ; idempotent | `referentiel.cours_generes` |
| STR-57 | Créer, modifier, fermer un cours (volume, semaines, public) | idem | public unique ; changer de public en cours d'année = fermer et ouvrir (historique des notes conservé) | `referentiel.cours_cree` / `_modifie` / `_ferme` |
| STR-58 | Définir une barrette (cours alignés) | idem | lue par le socle 10 comme contrainte | `referentiel.alignement_defini` |
| STR-59 | Voir le plan de structure de l'année et ses contrôles | personnel · `referentiel.lire` | §15 | — |

### 12.7 Élèves dans la structure

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-60 | Affecter un élève à une classe, avec date d'effet (REF-10) | Adm · `affectations.gerer` | une classe ouverte par élève et par année [F] ; élève scolarisé à la date (socle 2) ; aperçu des groupes et cours touchés | `scolarite.classe_changed` [F], `referentiel.affectation_changee` |
| STR-61 | Changer un élève de classe en cours d'année (commande unique, §4.5) | Adm · idem ★ | groupes recalculés et proposés ; professeurs, PP, vie scolaire prévenus | idem + `referentiel.composition_modifiee` |
| STR-62 | Enregistrer ou modifier les enseignements suivis d'un élève | Adm · idem | règles de choix ; date limite de changement (avertissement) ; groupe proposé | `referentiel.enseignements_suivis_modifies` |
| STR-63 | Affecter des élèves à un groupe (unitaire, collage d'une liste, depuis la règle) | Adm · idem ; responsable · `groupes.composer` (mes groupes, natures `activite`, `liste`) | capacité (avertissement) ; cohérence avec les choix | `referentiel.composition_modifiee` |
| STR-64 | Appliquer les règles de composition (proposition → validation) | Adm · `affectations.gerer` | ne s'applique jamais seule ; écarts listés | idem |
| STR-65 | Affecter en lot (rentrée, import) (REF-11) | Adm · idem ★ | aperçu exact (doublons du lot dédoublonnés avant calcul, correction de l'inexactitude [F]) ; conflits jamais écrasés | `referentiel.affectations_lot` [F] |
| STR-66 | Retirer d'un groupe, à une date | Adm · idem ; responsable · `groupes.composer` | jamais de suppression ; annulation d'une erreur = `annulee_le` | `referentiel.composition_modifiee` |
| STR-67 | Fermer toutes les affectations à la sortie (port, appelé par le socle 2) | SYS | dans la transaction de la sortie | `referentiel.composition_modifiee` |
| STR-68 | Lister les élèves à affecter (sans classe, choix sans groupe, groupe incohérent) | Adm · `referentiel.lire` | file de travail | — |
| STR-69 | Voir la scolarité structurelle d'un élève à une date (frise) | selon portée · `eleves.lire` (02) | lecture du port | — |

### 12.8 Préparation de rentrée et constitution

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-70 | Faire monter : proposer le niveau N+1 depuis les décisions de passage (23) et les réinscriptions (3) | Dir, Adm · `annee.preparer` | élèves sans décision listés ; redoublement, orientation, départ ; écrit l'inscription annuelle N+1 par le port du socle 2 | `referentiel.montee_proposee` / `_appliquee` |
| STR-71 | Ouvrir une campagne de constitution pour un niveau | Dir · `constitution.preparer` | élèves attendus calculés ; classes cibles | `constitution.ouverte` |
| STR-72 | Choisir les critères et leurs poids | Dir · idem | catalogue générique ; aucun critère de santé ni familial | — |
| STR-73 | Saisir indicateurs de niveau et besoins d'accompagnement pour la campagne | Dir, PP · `constitution.contribuer` (portée mes classes, sur les élèves de N) | facultatifs ; purgés | — |
| STR-74 | Déposer une contrainte (séparer, regrouper…) avec motif codé | Prof, CPE · `constitution.contribuer` ; Dir | dans la période d'avis ; l'auteur voit ses contraintes, la direction toutes | `constitution.contrainte_deposee` |
| STR-75 | Recueillir les souhaits des familles (si activé) | Famille (lien actif avec droit « décider ») | choix dans la liste du niveau, plafond paramétré ; non opposable | — |
| STR-76 | Calculer, comparer, ajuster des scénarios (verrouiller, glisser-déposer) | Dir · `constitution.preparer` | indicateurs en direct ; contraintes violées visibles | `constitution.scenario_calcule` |
| STR-77 | Valider un scénario ★ | Dir · idem | crée les affectations N+1 en lot (STR-65) ; idempotent | `constitution.validee` |
| STR-78 | Publier les classes aux familles et aux élèves (à une date) | Dir · idem | avant la publication, la classe N+1 n'est visible que du personnel | `constitution.publiee` |
| STR-79 | Purger les données de campagne | SYS à la date ; Dir à la demande | §9.8 | `constitution.purgee` |
| STR-80 | Préparer les départs de fin d'année en lot (avec le socle 3) | Adm · `annee.preparer` | élèves sans inscription N+1 ; appelle le socle 3 | — |

### 12.9 Échanges, consultation, qualité

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-81 | Importer une structure (SIECLE, fichier) → propositions | Adm · `structure.importer` | cadre du socle 9 ; aucune écriture sans validation | `referentiel.import_*` |
| STR-82 | Exporter la structure pour le logiciel d'EDT | Adm · `structure.exporter` | identifiants stables ; trace d'export | `export.realise` (9) |
| STR-83 | Accepter les écarts d'un import d'EDT (groupe, salle, cours inconnus) | Adm · `structure.gerer` | jamais de création silencieuse | `referentiel.*` |
| STR-84 | Exporter effectifs et listes (enquêtes, pilotage) | Adm, Dir · `structure.exporter` | D-022 : jamais un professeur | `export.realise` |
| STR-85 | Consulter mes cours et mes groupes, leurs élèves | personnel · portée | règle du §8 | — |
| STR-86 | Consulter une fiche classe (équipe, effectifs, mouvements, cours, calendrier, EDT) | personnel · `referentiel.lire` + portée pour les données élèves | — | — |
| STR-87 | Consulter un trombinoscope de classe ou de groupe | personnel | confidentialité renforcée respectée (02 §7.6) | — |
| STR-88 | Consulter les effectifs à une date, comparés à N-1 | Dir, Adm · `referentiel.lire` | — | — |
| STR-89 | Consulter son calendrier personnel (famille, élève) | famille, élève | événements publiés qui concernent ses classes et groupes | — |
| STR-90 | Rechercher une classe, un groupe, une salle, un cours | personnel | palette de commandes (29) | — |
| STR-91 | Voir le journal des changements de structure | Dir, Adm · `audit.lire` ou `referentiel.lire` (sans données élève) | — | — |
| STR-92 | Vérifier la cohérence (tâche nocturne et à la demande) | SYS | mêmes contrôles que le tableau de bord ; alerte aux responsables | `referentiel.controle_alerte` |

### 12.10 Actions ajoutées par la passe personas [personas]

Source de chaque action : manque `M-xxx` de `PERSONAS_05-06.md`. Même format que ci-dessus.

| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| STR-93 | Déclarer les **jours d'ouverture et de fermeture des services** (secrétariat, intendance, restauration, internat, accueil, CDI), indépendamment des cours (M-167) | Dir, Adm · `calendrier.gerer` | cible `service` (§9.10) ; « fermeture estivale du secrétariat » ; lu par les délais des socles 3, 8, 24 (R-S2) | `referentiel.calendrier_modifie` |
| STR-94 | Déclarer une **fermeture conditionnelle** et programmer sa **levée** ou sa prolongation (alerte météo, décision préfectorale) ★ (M-216) | Dir · `calendrier.gerer` | prolonge STR-25 ; levée programmée exécutée seule, avec un seul message par changement ; outre-mer : niveaux d'alerte [NV] | `referentiel.fermeture_exceptionnelle`, `referentiel.fermeture_levee` |
| STR-95 | **Annuler une ouverture d'année** (retour arrière complet) ★ (M-146, trou T-31) | Dir · `annee.basculer` | R-S3 ; aperçu : notifications déjà parties, faits déjà saisis dans N+1 ; N redevient courante, N+1 repasse en préparation | `referentiel.ouverture_annulee` |
| STR-96 | **Publier les informations de rentrée** de N+1 aux familles et aux élèves avant l'ouverture : date et heure de rentrée du niveau, classe (si publiée, STR-78), PP (si nommé, 06), calendrier N+1 (M-145) | Dir, Adm · `calendrier.gerer` | rien d'autre de N+1 n'est visible ; le portail montre N et « l'année prochaine » côte à côte (constat G13) | `referentiel.rentree_publiee` |
| STR-97 | **Arrêter un constat d'effectifs** à une date : chiffres figés, signés, comparés à N-1, exportés ★ (M-147) | Dir · `annee.preparer` ; export : `structure.exporter` | R-S4 ; nouvelle version si correction ; trame « constat d'effectifs » ; transmis par le socle 9, consolidé au 47 | `referentiel.constat_arrete` |
| STR-98 | Enregistrer, comparer, retenir un **scénario de structure prévisionnelle** (classes par niveau, dédoublements, offres) et le présenter (M-139) | Dir · `enseignements.gerer` | besoins en heures calculés comme STR-45 ; un seul retenu ; trame de présentation aux instances (socle 46) ; sans effet sur la structure tant qu'il n'est pas appliqué | `referentiel.scenario_structure_retenu` |
| STR-99 | Voir le **tableau des combinaisons de choix** (doublettes, triplettes, LV × options) et les **incompatibilités avec les barrettes** ; simuler une autre barrette (M-152) | Dir, Adm · `enseignements.gerer` | lit les vœux (socle 3) et les enseignements suivis ; simulation sans écriture ; élèves bloqués nommés | — |
| STR-100 | Instruire une **demande de changement d'enseignement suivi** (option, spécialité, langue) déposée par la famille, l'élève majeur ou le PP : avis, décision, effet daté, réponse (M-169) | dépôt : famille (lien avec droit « décider »), élève majeur, PP ; avis : professeurs concernés, PP · `constitution.contribuer` ; décision : Dir, Adm · `affectations.gerer` | date limite de l'offre = avertissement ; capacité ; effet par STR-62 ; réponse par trame ; enseignement quitté conservé (R-S16) | `referentiel.demande_structure_decidee` |
| STR-101 | **Réorientation en cours d'année** (changement de formation) : inscription annuelle par le port du socle 2, classe, enseignements suivis, groupes, en une commande ★ (M-199) | Dir, Adm · `affectations.gerer` | aperçu des cours quittés et rejoints ; notes conservées au cours d'origine (21) ; passerelle vers un autre établissement : socle 3 | `scolarite.classe_changed`, `referentiel.enseignements_suivis_modifies` |
| STR-102 | **Inclure** un élève dans un cours d'une autre classe ou d'un autre groupe, l'en **exclure**, ou **accueillir** une personne non inscrite dans des cours, à des dates (M-224, M-211, M-200) | Adm · `affectations.gerer` ; enseignant référent ULIS · `affectations.gerer` portée `mon_perimetre` [R] | R-S5 ; motif codé ; jamais de motif de santé affiché ; listes d'appel et de notes mises à jour ; AESH prévenue (06 R-P20) | `referentiel.composition_modifiee` (`cours_id`, entrants, sortants) |
| STR-103 | **Scinder un groupe** (dédoublement) ou fusionner deux groupes ★ (M-153) | Dir, Adm · `structure.gerer` | assistant : répartition des membres (alphabétique, équilibrée, manuelle), cours dupliqué ou fermé, service à pourvoir signalé au socle 6 | `referentiel.groupe_scinde` / `_fusionne` |
| STR-104 | **Corriger rétroactivement** une affectation erronée ★ (M-154) | Adm · `affectations.gerer` | R-S6 ; aperçu des appels et notes saisis sur la mauvaise cible ; aucune notification de « changement » | `referentiel.affectation_corrigee` |
| STR-105 | **Réviser la montée** après une décision d'appel ou une commission de juillet (M-166) | Dir, Adm · `annee.preparer` | affectations N+1 annulées, inscription annuelle N+1 corrigée par le port du socle 2, élève remis « à affecter », campagne prévenue | `referentiel.montee_revisee` |
| STR-106 | **Rouvrir une campagne de constitution validée** pour les arrivées et départs d'été (M-144) | Dir · `constitution.preparer` | R-S18 ; élèves déjà placés verrouillés par défaut ; nouvelle validation = nouvelles affectations seulement | `constitution.rouverte` |
| STR-107 | Recueillir les **avis des enseignants des écoles d'origine** (liaison école–collège) : accès invité limité à la campagne, ou import d'une fiche de liaison (M-141) | Dir · `constitution.preparer` | R-S17 ; même grille que STR-73/74 ; purgé avec la campagne | `constitution.contributeur_invite` |
| STR-108 | Instruire les **demandes de changement de classe** des familles après publication (M-148) | Dir · `constitution.preparer` | non opposables ; motif codé ; réponse par trame ; date limite fixe (S-42) ; effet par STR-61 | `referentiel.demande_structure_decidee` |
| STR-109 | Déclarer un **lieu extérieur** (gymnase municipal, piscine, stade) : adresse, temps de trajet, convention, **fenêtres de disponibilité** (M-205) | Admin, Adm · `etablissement.parametrer` | R-S14 ; indisponibilités du lieu (travaux de la mairie) comme STR-09 | `referentiel.lieu_*` |
| STR-110 | Rendre **plusieurs salles indisponibles** pour un événement du calendrier (examens, réquisition) en un geste (M-227) | Dir, Adm · `etablissement.parametrer` | liées à l'événement : déplacées ou annulées avec lui ; séances touchées signalées (10) | `referentiel.salle_indisponible` (lot) |
| STR-111 | **Imprimer ou exporter des listes** par classe, groupe, cours, option, langue, régime selon quatre trames : liste simple, émargement, trombinoscope, étiquettes ; et la **liste d'affichage de rentrée** (M-162) | personnel selon portée ; export fichier : Adm · `structure.exporter` (D-022) | R-S9 pour l'affichage ; filigrane et journal pour le trombinoscope (02 §7.6) ; disponible hors ligne en PDF préparé la veille de la rentrée | — |
| STR-112 | **Liste à la demande** : filtres de structure, colonnes choisies dans le catalogue autorisé par les groupes de lecture du socle 2, enregistrée en favori (M-163) | Adm, Dir · `structure.exporter` | jamais un professeur pour l'export (D-022) ; colonnes = groupes de lecture du lecteur ; réglage signalé S-43 | `export.realise` (9) |
| STR-113 | **Reprendre en cours d'année** la structure, les affectations et les enseignements suivis **datés depuis la rentrée** depuis l'ancien logiciel (M-161) | Adm · `structure.importer` | R-S13 ; cadre d'import du socle 9, socle 28 ; rejets listés ; aucune affectation inventée | `referentiel.affectations_lot` |
| STR-114 | **Adopter les dates des examens nationaux** (écrits du brevet, du baccalauréat) comme les vacances (M-168) | Dir, Adm · `calendrier.gerer` | source centrale [NV] ; effets ciblés par niveau (fin des cours avancée) | `referentiel.calendrier_adopte` |

### 12.11 Règles ajoutées par la passe personas [personas]

| Id | Règle | Manque |
|---|---|---|
| R-S1 | **Une fermeture a des effets différenciés.** Le déclarant coche les services touchés (cours, accueil des élèves présents, restauration, internat, transports) ; **par défaut, si des internes sont présents, internat et restauration restent ouverts** ; l'effet `accueil_seulement` (« établissement ouvert, cours non assurés ») existe ; l'aperçu nomme les internes présents et les repas maintenus. Les socles 40 et 41 n'annulent que ce qui est coché | M-202, M-203 |
| R-S2 | Les **jours ouvrés administratifs** (services ouverts) sont distincts des jours de cours ; les délais des socles 3, 8 et 24 les comptent | M-167 |
| R-S3 | **Annuler une ouverture** est possible jusqu'au **premier jour de cours** de N+1 si aucun appel ni aucune évaluation n'y a été saisi ; au-delà, refus (corrections ordinaires) ; les notifications déjà envoyées sont listées et un message correctif est proposé (socle 8, envoi humain) | M-146 |
| R-S4 | Un **constat d'effectifs** est figé : il n'est jamais recalculé ; une correction crée une nouvelle version qui cite la précédente | M-147 |
| R-S5 | **Public d'un cours = classe ou groupe, plus les inclusions, moins les exclusions**, datées. `public_du_cours` et la règle de visibilité du §8 les intègrent : le professeur voit l'élève inclus pour les dates d'inclusion ; l'élève exclu n'est ni sur la liste d'appel ni sur la saisie de notes du cours pour les dates d'exclusion (il reste dans l'annuaire de sa classe) | M-224, M-211, M-200 |
| R-S6 | **Corriger ≠ changer.** Une affectation erronée est annulée (`annulee_le`) et la bonne posée à la date d'origine ; aucune notification de « changement » aux familles ; les appels et notes saisis sur la mauvaise cible sont signalés aux socles 12 et 21 pour revue, jamais déplacés seuls | M-154 |
| R-S7 | Une **mesure d'éloignement** publiée par le socle 13 ou 36 (identifiants seulement) devient une contrainte « séparer » d'origine `mesure` dans toute campagne de constitution, et une alerte de composition si les deux élèves se retrouvent dans un même groupe ou cours ; la direction voit « contrainte réservée », jamais le motif | M-143 |
| R-S8 | **Fratrie du même niveau** (jumeaux) : le choix de la famille (ensemble, séparés, indifférent) est recueilli par défaut, même si les souhaits des familles sont désactivés ; c'est une contrainte souple | M-142 |
| R-S9 | **Liste d'affichage de rentrée** : prénom et initiale du nom par défaut, classe, salle, heure ; jamais de date de naissance, de dispositif (ULIS, UPE2A), d'option révélant une situation | M-164 |
| R-S10 | La classe porte un **code national de division** distinct de son libellé ; les exports utilisent le code ; contrôle de rentrée « division sans code » en public et privé sous contrat | M-165 |
| R-S11 | Le **fuseau du profil** (STR-03) est la **seule** source du fuseau pour tous les socles, y compris le socle 6 (`06_PERSONNELS.md` R-P2) | M-215 |
| R-S12 | **Changement de classe pour protection** : motif codé confidentiel (lecture direction) ; notifications aux équipes sans motif ; l'équipe d'origine apprend le départ, pas la destination | M-170 |
| R-S13 | **Reprise en cours d'année** : affectations, enseignements suivis et groupes repris **avec leurs dates** depuis la rentrée ; un historique absent est signalé, jamais inventé | M-161 |
| R-S14 | Un **lieu extérieur** n'est utilisable que dans ses fenêtres de disponibilité ; le socle 10 lit fenêtres et temps de trajet comme contraintes ; le socle 42 lit le trajet | M-205 |
| R-S15 | Une **capacité de sécurité** (bassin, atelier, laboratoire) peut être bloquante ; les autres capacités restent des avertissements | M-206 |
| R-S16 | Un **enseignement quitté** garde sa nature (changement en cours d'année, abandon de fin de 1ʳᵉ, fermeture de l'offre) et ses dates ; l'abandon d'une spécialité en fin de 1ʳᵉ est une transition normale, pas un changement [NV] ; les exports d'examen et d'orientation (9, 38) le lisent | M-198 |
| R-S17 | Un **contributeur invité** d'une école d'origine ne voit que les élèves entrants qui viennent de son école, pendant la période d'avis, avec la même grille que les professeurs ; jeton à usage limité ; purgé avec la campagne | M-141 |
| R-S18 | Une **campagne rouverte** garde verrouillés les élèves déjà placés ; seuls arrivées, départs et déverrouillages explicites sont recalculés ; les familles déjà informées ne reçoivent un message que si la classe de leur enfant change | M-144 |
| R-S19 | Un événement du calendrier peut être **publié aux personnels seulement** (conseil pédagogique, formation, pré-rentrée) | M-149 |
| R-S20 | Contrôle de rentrée : **effectif d'un cours supérieur à la capacité** de sa salle de référence ou prévue | M-160 |
| R-S21 | Un cours peut avoir pour public un **groupe d'activité** (chorale) ; il compte alors dans le service de l'intervenant (06) | M-183 |
| R-S22 | Le **calendrier personnel** d'une famille réunit tous ses enfants de l'établissement (un abonnement iCal, chaque événement étiqueté par enfant) | M-229 |
| R-S23 | Les **noms d'événements** du socle 5 (`referentiel.*`) font foi ; les consommateurs (socle 6) s'y alignent | M-234 |
| R-S24 | **Contrat IA** (D-025, `IA_COEUR.md` §5) des actions ajoutées : lectures déclarées (STR-99, 111, 112 en sensibilité « liste nominative ») ; commandes ★ (STR-94, 95, 97, 101, 103, 104) en confirmation renforcée avec aperçu serveur ; aucune lecture des notes scellées, des contraintes d'origine `mesure` ni des motifs de protection par l'IA | M-235 |

---

## 13. Droits (catalogue `ent_auth::Action`)

### 13.1 Existant [F]
`referentiel.lire` (tout le personnel), `referentiel.gerer` (Admin, Direction, Administratif), `matieres.gerer`, `edt.gerer` (calendrier A/B), `roles.gerer` (responsables de groupe), toutes à portée « établissement » [F `actions.rs:197, 214, 222, 225-226, 303-329, 390-393`]. Portée `mes_groupes` = groupes dont la personne est responsable ; portée `mes_classes` = calcul de `perimetre.rs` (§8).

### 13.2 Actions proposées [R]

| Action | Libellé | Portées admises | Modèle par défaut |
|---|---|---|---|
| `referentiel.lire` [F] | Voir la structure (sans données élève au-delà de l'annuaire) | établissement | inchangé |
| `referentiel.gerer` [F] | **Conservée comme raccourci** : détenue ⇒ toutes les actions ci-dessous sauf `annee.rouvrir` (bascule à l'identique) | établissement | inchangé |
| `etablissement.parametrer` | Profil, sites, bâtiments, salles, grilles, paramètres | établissement | Admin, Direction (+ Administratif via `referentiel.gerer`) |
| `annee.preparer` | Créer, reconduire, monter, tableau de bord | établissement | Admin, Direction, Administratif |
| `annee.basculer` | Ouvrir, programmer, clôturer | établissement | Admin, Direction ; Administratif **via `referentiel.gerer`** (comportement actuel) — Q-S8 |
| `annee.rouvrir` | Réouverture temporaire | établissement | Admin |
| `calendrier.gerer` | Calendrier, fermetures, semaines | établissement, **sites désignés** | Admin, Direction, Administratif, CPE (CPE détient `edt.gerer` aujourd'hui, qui couvrait le calendrier A/B [F]) |
| `periodes.gerer` | Périodisations et périodes | établissement | Admin, Direction, Administratif |
| `enseignements.gerer` | Matières, niveaux, MEF, offre, maquette, dotation (succède à `matieres.gerer`, gardée en alias) | établissement | Admin, Direction, Administratif |
| `structure.gerer` | Classes, groupes, responsables de groupe, cours, barrettes | établissement | Admin, Direction, Administratif |
| `affectations.gerer` | Affecter, changer de classe, enseignements suivis, compositions | établissement, **désignés** (un adjoint « niveau 6e ») | Admin, Direction, Administratif |
| `groupes.composer` | Composer **son** groupe d'activité ou de liste | **mes groupes**, désignés | aucun par défaut ; présent dans le modèle « Responsable d'atelier » fourni |
| `groupes.creer_activite` | Créer un groupe d'activité dont on devient responsable | établissement | aucun par défaut ; modèle « Responsable d'atelier » |
| `constitution.preparer` | Campagnes, scénarios, validation, publication | établissement | Admin, Direction |
| `constitution.contribuer` | Indicateurs et contraintes sur ses élèves de l'année | mes classes, mes groupes, établissement | Prof et PP : mes classes ; CPE : établissement |
| `structure.importer` | Importer structures | établissement | Admin, Direction, Administratif |
| `structure.exporter` | Exports de structure, effectifs, listes officielles (D-022) | établissement | Admin, Direction, Administratif ; **jamais le modèle Professeur** |

Règles :
- **Oracle de compatibilité** (même mécanisme que `r2_modele_identique_aux_capacites_historiques` [F]) : pour chaque profil, l'union des nouvelles actions par défaut ouvre exactement les routes ouvertes aujourd'hui ; les seules différences volontaires sont les nouveautés (constitution, composition d'atelier) et la **restriction D-022** (le professeur perd la vue de la classe entière là où il n'a qu'un groupe), annoncée comme correction.
- `annee.basculer`, `annee.rouvrir`, `constitution.preparer` : portée établissement seulement ; anti-escalade inchangée.
- Familles et élèves : hors rôles libres (D-017) ; ils lisent leur calendrier, leur classe et leurs groupes **publiés** par le lien actif ; souhaits de constitution par le lien avec droit de décision (socle 4).

### 13.3 Droits des actions ajoutées [personas]

Aucune nouvelle action de droit n'est nécessaire : les actions STR-93 à STR-114 utilisent celles du §13.2. Deux précisions [R] :
- `affectations.gerer` admet la portée **`mon_perimetre`** (socle 6) pour l'enseignant référent d'un dispositif (ULIS, UPE2A), limité aux inclusions et exclusions des élèves du dispositif (STR-102).
- Le **contributeur invité** d'une école d'origine (STR-107) n'a aucun rôle : il agit par un jeton de campagne limité à `constitution.contribuer` sur les élèves entrants de son école (R-S17) ; il n'est pas un personnel de l'établissement.
- Familles et élèves majeurs déposent les demandes de STR-100 et STR-108 par leur lien (droit « décider », socle 4), sans action du catalogue.

---

## 14. Événements

**Charge utile : identifiants, codes, dates ; jamais de nom d'élève, jamais de motif de contrainte, jamais de note scellée.** Noms existants conservés [F] (`referentiel.*`, `scolarite.classe_changed`, `edt.*`) ; types déclarés dans `ent-types::evenements` (aucun aujourd'hui [F]).

### 14.1 Émis

| Événement | Charge utile | Consommateurs [R] |
|---|---|---|
| `referentiel.annee_creee` [F], `_ouverte` [F], `_terminee`, `_cloturee` [F], `_rouverte`, `_refermee`, `ouverture_programmee`, `ouverture_annulee` | `annee_id`, dates, forçage | tous (année de travail), 24 (campagne), 47, 8 |
| `referentiel.structures_reconduites` [F], `montee_appliquee` | `annee_id`, compteurs | 3, 6 |
| `referentiel.calendrier_adopte`, `calendrier_modifie`, `semaines_definies`, `rentrees_definies` | `annee_id`, dates, cibles, effet | **10** (séances), 12, 24, 40, 41, 43, 8 (si publié), 29 |
| `referentiel.fermeture_exceptionnelle` / `fermeture_levee` | dates, demi-journées, cibles | **10** (annulation), **12** (appels non attendus), **40** (repas), 41, 43, **8** (familles, personnels), 24 |
| `referentiel.periodes_definies` [F], `periode_modifiee`, `periodisation_assignee` | `annee_id`, `periodisation_id`, dates | 21, 23, 24 |
| `referentiel.grille_horaire_definie` [F], `semaine_type_modifiee` | grille, date d'effet | 10, 12 |
| `referentiel.site_*` [F], `salle_*`, `salle_indisponible`, `batiment_*` | identifiants, dates | 10, 44, 14 |
| `referentiel.profil_modifie`, `parametre_modifie` | clés | tous (relecture) |
| `matiere.created` [F], `referentiel.matiere_modifiee`, `matieres_fusionnees`, `matiere_archivee` | identifiants | 21, 20, 10 |
| `referentiel.offre_modifiee`, `offre_fermee`, `maquette_modifiee` | `offre_id`, `annee_id` | **3** (vœux, capacités), 24 (option payante), 6 |
| `referentiel.classe_creee`, `_modifiee` [F], `_archivee` [F], `_scindee`, `_fusionnee` | `classe_id`, `annee_id` | 10, 16 (conversations de classe), 6, 23 |
| `referentiel.groupe_cree`, `_modifie`, `_ferme`, `groupes_generes`, `responsable_groupe_change` | `groupe_id`, nature | 1 (droits relus), 10, 16, 43 |
| `referentiel.cours_cree` / `_modifie` / `_ferme`, `cours_generes`, `alignement_defini` | `cours_id`, public, volume | **6** (services à pourvoir), **10**, 20, 21 |
| `scolarite.classe_changed` [F], `referentiel.affectation_changee` | `eleve_id`, classe avant/après, date | 2 (chronologie), 10, 12, 16, 20, 21, 24 (si tarif par classe), 8 (PP, VS) |
| `referentiel.composition_modifiee` | `groupe_id` ou `classe_id`, `entrants[]`, `sortants[]` (identifiants), date | 10, 12, 16, 20, 21, 43, 8 (professeurs du cours) |
| `referentiel.enseignements_suivis_modifies` | `eleve_id`, offres ajoutées/retirées, date | 24 (options payantes), 9 (exports), 21 |
| `referentiel.affectations_lot` [F] | `annee_id`, compteurs | idem en masse (traitement par lot) |
| `constitution.ouverte`, `contrainte_deposee` (sans contenu), `scenario_calcule`, `validee`, `publiee`, `purgee` | identifiants, compteurs | 8 (avis attendus des professeurs, publication aux familles) |
| `referentiel.controle_alerte` | code du contrôle, compteur | 8 (responsables), 29 (à-faire) |

### 14.2 Consommés

| Événement | Producteur | Réaction du socle 5 |
|---|---|---|
| `inscription.validee` [F] | 3 | ajoute l'élève à la file « à affecter » de l'année visée ; **aucune affectation automatique** (`03_INSCRIPTIONS.md` Q-I3) ; enseignements suivis reçus par le port dans la transaction |
| `eleve.reinscription_confirmee` [F] | 3 | prépare la montée (STR-70) |
| décision de passage (socle 23, nom à fixer) | 23 | alimente la montée |
| `eleve.sortie` [F] / `scolarite.closed` | 2 | rien : la fermeture des affectations est faite **dans** la transaction par le port (STR-67) ; contrôle nocturne de rattrapage |
| `eleve.fusionne` | 2 | affectations et enseignements suivent le registre de fusion |
| `eleve.confidentialite_changee` | 2 | trombinoscopes et listes publiées recalculés |
| `service.*` (socle 6) | 6 | contrôle « cours sans intervenant » recalculé |
| `edt.import_realise` (socle 10) | 10 | écarts de structure proposés (STR-83) |

### 14.3 Événements ajoutés [personas]

| Événement | Sens | Charge utile | Consommateurs ou producteur |
|---|---|---|---|
| `referentiel.fermeture_exceptionnelle` (enrichi) | émis | + `services_touches[]`, `conditionnelle`, `levee_prevue_le` | 10, 12, 40, **41**, 43, 8 : chacun n'agit que sur les services cochés (R-S1) |
| `referentiel.ouverture_annulee` (étendu à STR-95) | émis | `annee_id`, compteurs de notifications déjà envoyées | tous (année de travail), 8 |
| `referentiel.rentree_publiee` | émis | `annee_id`, niveaux | 8, 29 (portail) |
| `referentiel.constat_arrete` | émis | `constat_id`, date de référence, version | 9, 47 |
| `referentiel.scenario_structure_retenu` | émis | `scenario_id` | 6 (répartition prévisionnelle), 46 |
| `referentiel.demande_structure_decidee` | émis | `demande_id`, type, état | 8 (réponse), 2 (chronologie) |
| `referentiel.groupe_scinde` / `_fusionne` | émis | groupes source et cibles | 6 (services à pourvoir), 10, 12, 21 |
| `referentiel.affectation_corrigee` | émis | `eleve_id`, cible erronée, cible juste, période | **12, 21** (revue des saisies), 2 ; **jamais 8** (pas de « changement » annoncé) |
| `referentiel.montee_revisee` | émis | `eleve_id`, `annee_id` | 3, 24, constitution |
| `constitution.rouverte`, `constitution.contributeur_invite` | émis | identifiants, compteurs | 8 |
| `referentiel.lieu_*` | émis | `lieu_id`, dates | 10, 42 |
| `referentiel.composition_modifiee` (étendu) | émis | + `cours_id` pour les inclusions et exclusions | 10, 12, 20, 21, **6** (missions sur élèves : AESH, R-P20) |
| mesure d'éloignement (nom à fixer par les socles 13 et 36) | consommé | deux `eleve_id`, dates, **aucun motif** | contrainte `mesure` (R-S7) |

---

## 15. Écrans clés

Leptos 0.8 (`docs/FRONTEND_LEPTOS.md`), actions proposées par le serveur, design system maison. Les six écrans actuels (`/web/referentiel/*` [F]) sont gardés et prolongés.

| Écran | Route [R] | Contenu | Pour qui |
|---|---|---|---|
| **Années et rentrée** (prolonge `/web/referentiel/annees`) | `/web/structure/annees` | frise des années (préparation, courante, terminée, clôturée) ; **tableau de bord de rentrée** : étapes du §4.2 avec contrôles regroupés par socle, compteurs, gravité, lien direct vers la correction ; aperçu d'ouverture et de clôture calculés par le serveur ; programmation | direction, secrétariat |
| **Établissement** | `/web/structure/etablissement` | profil, préréglages, sites, bâtiments, salles (tableau dense, filtres par type et capacité, indisponibilités), paramètres | administrateur |
| **Calendrier** (prolonge `/calendrier`) | `/web/structure/calendrier` | vue annuelle en mosaïque de mois ; calques (officiel, établissement, par site/niveau/classe) ; semaines A/B en bandeau ; ajout par glisser sur des jours ; **fermeture exceptionnelle** avec aperçu des effets ; comparaison N-1 ; publication | direction, secrétariat ; lecture pour tous |
| **Périodes et grilles** | `/web/structure/periodes`, `/grilles` | périodisations et leurs assignations (niveaux × périodisations en tableau) ; grilles versionnées, frise de journée | secrétariat |
| **Offre et maquette** | `/web/structure/offre` | par niveau : enseignements proposés (modalité, capacité, choix des élèves, effectif), règles de choix, maquette, **besoins / dotation** avec simulation | direction |
| **Plan de structure** | `/web/structure/annee/{id}` | **matrice** : une colonne par niveau, les classes en lignes, les groupes inter-classes en bandeaux qui traversent leurs classes d'origine ; effectifs, capacité, filles/garçons, pastilles d'anomalie ; panneau latéral de l'objet sélectionné (cours, intervenants, salle, membres) ; génération des groupes et des cours | direction, secrétariat |
| **Composition d'un groupe** | `/web/structure/groupes/{id}` | membres datés ; « devraient y être » selon la règle (avec bouton d'ajout) ; « ne devraient pas » ; historique des entrées/sorties ; pour un responsable d'atelier, la version limitée à son groupe | secrétariat, responsables |
| **Élève dans la structure** (onglet « Scolarité » du socle 2) | `/web/eleves/{id}/scolarite` | frise : classe, groupes, enseignements suivis ; actions : changer de classe (assistant avec conséquences), modifier les choix | secrétariat |
| **Constitution des classes** | `/web/structure/constitution/{campagne}` | colonnes = classes cibles, cartes élèves (prénom, nom, indicateurs, pastilles d'enseignement), glisser-déposer, verrou ; jauges d'équilibre en direct ; liste des contraintes violées ; comparaison de scénarios côte à côte ; rien d'imprimable sans journal | direction |
| **Avis de constitution** | `/web/structure/constitution/{campagne}/avis` | pour un professeur : ses élèves de l'année, indicateur et contraintes, saisie rapide au clavier | professeurs, CPE |
| **Mes cours et mes groupes** | `/web/mes-cours` | exactement le périmètre du §8 ; par cours : public à la date, mouvements récents | professeurs, responsables |
| **Fiche classe** | `/web/classes/{id}` | équipe (socle 6), effectifs et mouvements, cours, calendrier et EDT de la classe, trombinoscope | personnel |
| **Calendrier personnel** (portail) | `/web/mon-calendrier` | famille et élève : vacances, journées sans cours, rentrée de son niveau, stage, examens ; abonnement iCal | familles, élèves |

**Mobile** (socle 11) : calendrier personnel, « mes cours et mes groupes » en lecture, notification de fermeture exceptionnelle ; aucune gestion de structure sur mobile (tâche complexe, CLAUDE.md).

**Accessibilité** : la matrice de structure et la constitution ont une **alternative clavier et liste** (déplacer un élève par menu « vers la classe… ») ; glisser-déposer jamais seul moyen.

---

## 16. Sécurité et données personnelles

- **Cloisonnement** : clés composées, RLS, tests de séparation entre établissements sur toutes les nouvelles routes ; les tables globales (`ref_nomenclature`, `ref_calendrier_officiel`) sont en lecture seule pour les tenants et sans aucune donnée personnelle.
- **Visibilité** : la règle du §8 est le seul calcul ; la matrice D-022 fait partie de la matrice d'isolation.
- **Constitution des classes** : les contraintes peuvent révéler un conflit, un harcèlement, une situation familiale [H]. Mesures [R] : motif **codé** par défaut ; note libre facultative **scellée** (chiffrement applicatif du socle 1), lisible de la direction seule ; l'auteur ne voit que ses propres contraintes ; aucun export ; consultation journalisée ; **purge** automatique (§9.8) ; information des familles dans la notice de traitement (socle 27). Les souhaits des familles ne portent que sur des élèves du même niveau, sont visibles de la direction seule et purgés de même.
- **Indicateur de niveau** : facultatif, limité à la campagne, purgé, jamais affiché aux familles ni réutilisé ailleurs.
- **Publication des classes** : avant la date de publication, aucune famille ne voit la classe N+1 de son enfant, ni la composition d'une classe ; après, une famille voit la classe et l'équipe, jamais la liste des autres élèves [R] (sauf choix de l'établissement de publier la liste de classe aux élèves, désactivé par défaut).
- **Calendrier iCal** : lien personnel à jeton révocable ; aucune donnée nominative dans les événements.
- **Fermeture exceptionnelle** : action ★ pour éviter l'erreur de masse (des centaines de repas annulés, des familles prévenues).
- **Conservation** : la structure (classes, groupes, cours, calendrier) est conservée avec l'année archivée ; les affectations suivent la durée de conservation de la scolarité de l'élève (socle 27, DPO) ; journal des opérations d'année conservé.
- **Aucune IA** dans ce socle : le calcul de constitution est déterministe et explicable ; la résolution de noms par un modèle externe vue dans Next.js n'est pas reprise (CLAUDE.md : pas de fallback externe non autorisé).

---

## 17. Plan de migration depuis l'existant

Règles : migrations additives d'abord ; reprises idempotentes qui rangent l'imprévu en « à traiter » sans échouer ; répétition sur base de labo et jeux anonymisés, **jamais** sur la base de production ; ancienne interface `ui.rs` figée ; anciennes routes conservées puis dépréciées.

| Étape | Contenu | Reprise / compatibilité | Vérification |
|---|---|---|---|
| M0 Correctifs immédiats | aucune création implicite (années, classes) dans les lectures et le trigger (erreur explicite sur code inconnu, journalisée) ; garde d'état d'année (clôture de la courante refusée, ouverture qui ne clôt plus, ancienne route `courante=true` refusée, réouverture par trigger refusée) ; affectation jamais supprimée physiquement ; filtre de sortie dans `mes_groupes` et `designes` ; tests qui **échouent** sans base en CI | aucun schéma | tests des cinq défauts §2.2 n° 3, 4, 5, 17, 18 |
| M1 D-022 provisoire | branche EDT de `perimetre.rs` restreinte au groupe du service quand le créneau en a un ; `classes_du_prof` → « mes cours » ; matrice de séparation D-022 | — | tests classe mixte, demi-groupe, inter-classes |
| M2 Clés et contraintes | `UNIQUE (etablissement_id, id)` et `(etablissement_id, annee_id, id)` ; clés composées `NOT VALID` puis validées ; rapport d'orphelins ; `ent_edt_seance` ajoutée au plan Q14 | orphelins listés, non supprimés | requête d'orphelins à zéro en labo |
| M3 Profil et paramètres | `ent_etablissement_profil` créé pour chaque établissement (secteur « à préciser », fuseau `Europe/Paris`) ; `ent_param_etab` et `ent_sortie_param` → `ent_parametre` (clés connues ; inconnues rapportées) ; `heure_paris` → fuseau du profil | lectures des anciennes clés par une vue | égalité des valeurs |
| M4 Calendrier | `ent_periode_vacances` + `ent_jour_calendrier` → `ent_calendrier_evenement` (année déduite des dates ; `fermeture` conservée) ; `ent_edt_calendrier_ab` → `ent_semaine` ; `ref_calendrier_officiel` chargé ; fériés calculés comparés aux fériés saisis | vue `ent_jour_calendrier` de compatibilité | jours ouverts identiques avant / après sur données de labo, écarts expliqués |
| M5 Périodes et grilles | `ent_periodisation` par année (une « défaut » + une par `niveau_id` rencontré) ; `ent_note_periode` : unicité par année ; `ent_grille_horaire` → version « depuis toujours » | vues | périodes résolues identiques par classe |
| M6 Salles, matières, offre | `ent_salle.site` → `site_id` (rapprochement par code/libellé, rejets) ; matières enrichies ; offres créées depuis les codes d'`ent_eleve_option(_annee)` ; `ent_eleve_enseignement` repris (année texte → `annee_id`, années **non créées** : rejets rapportés) | vues de compatibilité des tables d'options | aucune option perdue |
| M7 Cours | chaque `ent_service_enseignement` → un cours (matière × cible, dédoublonné : deux professeurs sur la même cible et la même matière = un cours, deux services) + `cours_id` sur le service ; `ent_edt_creneau.cours_id` déduit de `service_id` ; `matiere_libelle` sans `matiere_id` → matière créée « à vérifier » | `ent_service_enseignement` reste la table des services jusqu'au socle 6 | comparaison périmètre professeur avant / après (identique sauf corrections D-022, listées) |
| M8 Groupes | `ent_groupe.classe_id` → `ent_groupe_classe` ; `nature` = `enseignement` si un service vise le groupe, `activite` si responsables sans service, sinon `liste` à revoir ; responsables datés | `classe_id` maintenu par la base | — |
| M9 Années | `statut`/`courante` → `etat` ; `precedente_id` chaîné ; journal d'opérations ouvert | vue `courante` | transitions testées |
| M10 Écritures | toutes les écritures de `referentiel_repo.rs` passent par les commandes du socle 5 (événements) ; génération des séances déplacée au socle 10 (code déplacé) ; puis test de garde des tables du socle 5 | anciennes routes en façade | grep des `INSERT/UPDATE ent_classe|ent_groupe|ent_affectation` hors module |
| M11 Retrait D-022 provisoire | branche EDT supprimée quand 100 % des créneaux ont un `cours_id` | — | compteur de créneaux sans cours à zéro |
| M12 Retrait des projections | après bascule des lecteurs (Q14) : textes `classe`, `niveau`, `salle`, `site`, `matiere_libelle` ; anciennes tables calendrier et options | migration dédiée après validation | grep des lectures à zéro |

---

## 18. Découpage en lots de code (équipe Sonnet)

Chaque lot livre : migrations, dépôt (`ent-db`), service en transaction unique, routes (`ent-api`), types et événements (`ent-types`), tests Postgres (isolation entre établissements, matrice de droits, matrice D-022 quand le lot touche la visibilité), écrans quand le lot en a, documentation (ce document, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; revue sécurité (skill `security-review`) sur S0, S1, S6, S9, S10. Sessions ou worktrees séparés (F-007), quatre compilations au plus en même temps (F-006). Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

| Lot | Contenu | Migrations | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|---|
| **S0 Correctifs** | M0, M2 | 330, 331 | — | les défauts §2.2 n° 3, 4, 5, 16, 17, 18, 19 corrigés et testés ; aucune création implicite (test) | M |
| **S1 Visibilité D-022** | M1 ; fonction unique `structure::perimetre` ; matrice D-022 dans la matrice d'isolation | — | S0 | professeur d'allemand ne voit aucun élève d'espagnol sur les 12 routes élève (appel, notes, appréciations, cahier, vie scolaire, dossier, messagerie élève…) | M |
| **S2 Profil, paramètres, lieux** | M3, STR-01 à 10 ; préréglages ; fuseau | 332, 337 | S0 | outre-mer : heure locale juste (test) ; salle typée liée à son site | M |
| **S3 Calendrier** | M4, STR-23 à 30 ; import officiel (après vérification du jeu de données et de la licence) ; fériés calculés ; port `jour()` ; fermeture exceptionnelle et son événement | 333, 334 | S2 | `jour()` exact sur une année de labo (vacances, fériés, ponts, cibles, A/B) ; fermeture → événement consommé par un consommateur de test | L |
| **S4 Cours** [fait en partie, §24 : objet, vacant, membres, reprise, contrat ; STR-56 et STR-58 non faits] | M7, STR-56 à 58 ; contrat avec le socle 6 (services) et 10 (`cours_id` des créneaux) | 341 | S1 | périmètre avant/après identique hors corrections D-022 listées ; poste vacant = cours sans service | M |
| **S5 Offre et enseignements suivis** | M6 (matières, offre, options), STR-36 à 44, 62 ; port pour le socle 3 | 338, 340 | S0 | aucune option perdue ; règles de choix appliquées ; offre lue par le socle 3 | M |
| **S6 Groupes et compositions** | M8, STR-50 à 55, 60 à 68 ; règles de composition ; changement de classe en un geste ; port de fermeture à la sortie | 339, 342 | S4, S5 | contrôle « choix sans groupe » ; changement de classe : groupes recalculés, événements, professeurs prévenus (test) | L |
| **S7 Périodes et grilles** | M5, STR-31 à 35 | 335, 336 | S2 | périodisation résolue par classe ; grille versionnée ; coupure lue par le socle 12 | M |
| **S8 Années et bascule** | M9, STR-11 à 22, 70, 80 ; contrôles `ControleRentree` / `ControleCloture` ; aperçu serveur ; ouverture programmée (tâche) ; réouverture | 344 | S3, S6, S7 | parcours complet N → N+1 sur labo : préparation atomique, montée par décisions, ouverture sans clôture, clôture contrôlée, réouverture bornée | L |
| **S9 Constitution des classes** | STR-71 à 79 ; algorithme déterministe ; purge | 343 | S6, S8 | 300 élèves en < 10 s (labo) ; contrainte de séparation respectée ; purge vérifiée (plus aucune contrainte en base) ; aucun accès famille avant publication | L |
| **S10 Dotation et maquette** | STR-45 ; besoins / dotation ; simulation | 345 | S4, S5 | simulation sans écriture (test) | M |
| **S11 Échanges** | STR-81 à 84 ; export de structure pour l'EDT ; écarts d'import ; exports d'effectifs (D-022) | — | S6 ; socle 9 (cadre d'import) | aller-retour sans perte sur un fichier d'exemple ; aucune création silencieuse | M |
| **S12 Écrans** | sous-lots : S12a années + tableau de bord ; S12b établissement + salles ; S12c calendrier ; S12d offre + maquette ; S12e plan de structure + groupes ; S12f constitution + avis ; S12g mes cours + fiche classe + calendrier personnel | — | lots back | rendu et parcours contre le vrai backend ; clavier, 375 px, axe-core | L (total) |
| **S13 RLS et retraits** | RLS stricte des tables du socle ; M10 à M12 (avec le socle 10 pour les séances) | 346–349 | tous | RLS testée sous rôle non propriétaire ; grep des lectures texte à zéro | M |

Ordre recommandé : **S0 → S1** (sécurité d'abord) → (S2, S5 en parallèle) → (S3, S4, S7) → S6 → S8 → (S9, S10, S11) → S12 au fil de l'eau → S13. **Parcours minimal vendable** (« une rentrée complète dans l'ENT ») : S0, S1, S2, S3, S4, S5, S6, S8 + S12a, c, e, g ; la constitution (S9) est l'argument commercial de juin et suit immédiatement.

**[personas] Place des ajouts dans les lots** (`PERSONAS_05-06.md`) : **avant S1 et S4**, trancher Q-S20 et Q-S21 (§0 de la passe, points A et B) ; **S4** : inclusions et exclusions (`ent_cours_membre_individuel`, R-S5) dans le modèle du cours dès le départ ; **S3** : fermeture à effets différenciés, levée programmée, services administratifs, examens nationaux, publication aux personnels (STR-93, 94, 114 ; R-S1, R-S2, R-S19) ; **S2** : lieux extérieurs et disponibilités, salles en lot (STR-109, 110) ; **S5** : admission sur avis ou test, enseignement quitté (R-S16) ; **S6** : inclusions, dédoublement de groupe, correction rétroactive, réorientation, demandes (STR-100 à 104, 108) ; **S8** : annulation d'ouverture, révision de la montée, informations de rentrée (STR-95, 96, 105) ; **S9** : campagne rouverte, contributeurs invités, fratries, mesures d'éloignement (STR-106, 107 ; R-S7, R-S8) ; **S10** : scénarios de structure, combinaisons et barrettes, constat (STR-97 à 99) ; **S11** : reprise datée, code de division (STR-113 ; R-S10) ; **S12** : sous-lot **S12h « listes »** (STR-111, 112, trames). Migrations : numéros à attribuer hors de la plage 330–349, déjà pleine.

Plage de migrations (registre `MIGRATIONS.md`) : 330 correctifs d'état et garde-fous ; 331 clés composées ; 332 profil et paramètres ; 333 calendrier ; 334 semaines ; 335 périodisations ; 336 grilles ; 337 bâtiments et salles ; 338 matières et offre ; 339 classes et groupes ; 340 enseignements suivis ; 341 cours ; 342 affectations ; 343 constitution ; 344 opérations d'année ; 345 maquette et dotation ; 346 RLS ; 347–349 retraits (après validation).

---

## 19. Questions à trancher par le porteur du projet

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q-S1** | Les **affectations** (classe, groupes) appartiennent au socle 5 (table et commandes), le socle 2 les lit. `SOCLES.md` dit « affectations des élèves → 2 » ; `02_ELEVE.md` §7.4 et `03_INSCRIPTIONS.md` §5.1 disent « référentiel / socle 5 » | 5 / 2 | **5** ; corriger `SOCLES.md` |
| **Q-S2** | **Choix ≠ groupe** : enseignements suivis (`ent_eleve_enseignement`) distincts des groupes, au lieu de « option = groupe » (`02_ELEVE.md` §7.4) ; propriété : socle 5 | séparer / fusionner | **séparer**, contrôle de cohérence entre les deux |
| **Q-S3** | **Cours ≠ service** : le cours (ce qui est enseigné à qui) au socle 5, le service (qui intervient) au socle 6 | séparer / garder `ent_service_enseignement` | **séparer**, avec reprise automatique |
| Q-S4 | Professeur principal et fonctions attachées à une classe : socle 6 (aujourd'hui REF-13 dans le référentiel [F]) | 5 / 6 | **6** (c'est une fonction d'une personne) ; la commande existante est déplacée, sans changer l'API |
| Q-S5 | Le professeur principal voit-il **toute** sa classe, même les groupes qu'il n'enseigne pas ? | oui / non | **oui** (il suit la classe : conseil, orientation) |
| Q-S6 | Visibilité après un changement de groupe : l'ancien professeur voit l'élève pour les **dates antérieures** | oui / non | **oui** (finir le trimestre honnêtement) |
| Q-S7 | Ouvrir N+1 **ne clôt plus** N (nouvel état `terminee`) | oui / non | **oui** (corrige le défaut n° 4) |
| Q-S8 | Le **secrétariat** garde-t-il le droit d'ouvrir et de clôturer l'année (comportement actuel via `referentiel.gerer`) ? | oui / direction seule | **direction seule** à terme ; à l'identique au départ, changement annoncé |
| Q-S9 | Réouverture d'une année clôturée : autorisée, bornée (15 jours, périmètre) | non / bornée | **bornée**, Admin seul |
| Q-S10 | Constitution des classes : souhaits des familles | jamais / paramétrable désactivé / activé | **paramétrable, désactivé par défaut** ; jamais de choix de professeur ; pas de saisie libre |
| Q-S11 | La génération des séances datées (aujourd'hui dans le référentiel [F]) part au socle 10 | oui / non | **oui** (code déplacé) |
| Q-S12 | Nomenclatures et calendrier officiel en **tables globales** tenues par ScolIA (hors tenant), en lecture seule | global / par établissement | **global** ; chaque établissement référence et adopte |
| Q-S13 | Calendrier officiel : source = jeu de données ouvert du ministère [NV] | — | **vérifier** jeu, licence, format, mise à jour avant S3 ; à défaut, saisie centrale par ScolIA une fois par an |
| Q-S14 | Les **listes de classe** (désignation des élèves) restent visibles de tout le personnel (annuaire, `02_ELEVE.md` Q-E3) | annuaire / portée | **annuaire**, le reste par la portée |
| Q-S15 | Publication des classes aux familles : la famille voit-elle la **liste** des élèves de la classe de son enfant ? | non / paramétrable | **non par défaut**, paramétrable |
| Q-S16 | Dotation horaire et maquette dans le socle 5 (et non dans 47 ou 48) | 5 / 47 | **5** (elles se calculent sur la structure) ; consolidation au 47 |
| Q-S17 | Les enseignements « hors contrat » d'un établissement sous contrat (option payante) : portés par l'offre, facturés par le socle 24 | oui / non | **oui** |
| Q-S18 | Vérification réglementaire avant S3, S5, S8, S10 : calendrier par zone et outre-mer, fériés d'Alsace-Moselle et d'outre-mer, volumes horaires de référence et règles de choix (LV2, spécialités), groupes de besoin, dotation et présentation aux instances, codes de modalité d'élection, formats SIECLE structures et STS (et son successeur), GAR | — | **consultation** des textes ; ajout à `INTEROPERABILITE_NATIONALE.md` §12 ; corriger au passage la mention « aucune colonne `uai` » (`INTEROPERABILITE_NATIONALE.md:159`), fausse : `ent_site.uai` existe [F `110:14-15`] |
| Q-S19 | Les migrations 112 (qui écrivent des données à leur application et créent années et classes) : neutraliser la création dans une migration correctrice (330) sans réécrire 112 | oui / non | **oui** |
| **Q-S20** [personas] | **Cours (socle 5) et service d'enseignement (socle 6) : un seul objet.** Les deux conceptions possèdent chacune « ce qui est enseigné à qui » (`ent_cours` ici, `ent_service_enseignement` enrichi au socle 6) | une table, socle 5 crée et structure, socle 6 porte les interventions / deux tables | **une table** (l'existant `ent_service_enseignement` devient le cours) ; création et public par le socle 5 ; interventions, heures dues par le socle 6 ; PRS-40 appelle STR-57 ; une seule reprise (fusion de M7 et de la migration 354 du socle 6). **Bloquant avant S4 et P3** |
| **Q-S21** [personas] | **Une seule fonction de périmètre** : `structure::perimetre` (ici) et `services::eleves_vus` (socle 6) | fonction du socle 5 qui lit les interventions / fonction du socle 6 | **socle 5**, qui lit les interventions (en saisie, en lecture, délai de grâce) fournies par le socle 6 ; un seul jeu de tests D-022. **Bloquant avant S1 et P4** |
| Q-S22 [personas] | Annuler une ouverture d'année (STR-95) : jusqu'à quand ? | jamais / jusqu'au premier jour de cours / sans limite | **jusqu'au premier jour de cours**, sans saisie dans N+1 (R-S3) |
| **Q-S23** [personas] | Membres individuels d'un cours (inclusions, exclusions, accueil temporaire) | oui / groupes seulement | **oui** : un groupe par élève inclus serait ingérable et les listes d'appel resteraient fausses (R-S5) |
| Q-S24 [personas] | Avis des enseignants des écoles d'origine (STR-107) | accès invité par jeton / import d'une fiche seulement / rien | **les deux**, accès invité désactivé par défaut |
| Q-S25 [personas] | Liste à la demande (STR-112) : catalogue de colonnes, ou listes fixes seulement (risque de « requêteur », D-023) | catalogue court / listes fixes | **douze listes prêtes + catalogue court**, réglage signalé S-43 |
| Q-S26 [personas] | Constats d'effectifs : dates officielles (enquêtes publiques, forfait d'externat dans le privé) [NV] | — | **vérifier** les dates et contenus attendus avant S10 ; ajouter à `INTEROPERABILITE_NATIONALE.md` §12 |

---

## 20. Récapitulatif : faits, hypothèses, contradictions

**Faits vérifiés [F]** (relevés par les explorations, constats décisifs relus) : année à états dérivés par trigger, une seule courante, réouverture sans garde (`092:6-19`, `110:73-98`) ; clôture qui ne vérifie pas que l'année est courante et ouverture qui clôt l'ancienne sans contrôle des notes (`referentiel_annee.rs:230-250, 297-335`) ; préparation en deux transactions (`referentiel_annee.rs:504`) ; reconduction sans responsables ni composition (`358-499`) ; montée par correspondance de codes (`648`) ; branche EDT de la portée au niveau de la classe (`perimetre.rs:127-139`) et `classes_du_prof` qui liste la classe entière pour un service de groupe (`referentiel_repo.rs:508-534`) ; lectures et triggers qui créent années et classes (`referentiel_repo.rs:287-335`, `111:57-109`, `112:60`) ; groupe à classe parente unique (`092:39-52`) ; service = professeur × matière × classe XOR groupe, PP unique (`092:87-116`) ; options en texte, année en texte (`015`, `018`) ; deux tables de jours fermés et lecture qui confond fermeture et vacances (`073`, `110:145-161`, `referentiel_structure.rs:932`) ; deux tables de périodes (`022:13`, `110:113-140`) ; salles et sites en texte (`076`, `021`, `065`, `110:207`) ; `ent_edt_seance.classe` texte hors du plan de rattachement (`110:202`) ; aucune zone, aucun secteur, aucun fuseau ; paramètres clé/valeur (`076:3-14`) ; actions `referentiel.lire/gerer`, `matieres.gerer`, `edt.gerer` (`actions.rs`) ; écritures anciennes sans événement (`referentiel_repo.rs`) ; tests ignorés sans base (`tests_referentiel_s2.rs:14-22`) ; écrans Leptos `/web/referentiel/{annees,structure,classes,calendrier,grille}` (`ent-web-app/src/lib.rs:85-90`) ; aucun écran salles ni matières. Next.js [F Next] : outil de répartition des classes avec avis des personnels, souhaits des parents (camarades, professeur), solveur, et résolution des saisies libres par un modèle externe (`app/lib/class-allocation-*.ts`) ; coefficients par matière × classe ; familles de matières ; grilles nommées ; zone de vacances par site ; calendrier 2025-2026 codé en dur ; table `annee_bascule` inutilisée.

**Non relu ligne à ligne** : corps de `ref_definir_periodes`, `ref_modifier_classe`, `planifier_seances`, `sync_affectation_classe`, `amorcer_referentiel` ; tests `ent-api/tests/referentiel.rs` ; écrans Leptos (lus par leur structure et `FRONTEND_CIBLE.md` §10) ; solveur Next.js (types et score seulement).

**Hypothèses [H] / non vérifié [NV]** : tout le calendrier réel d'une année (§4.1) ; dotation horaire globale et sa présentation aux instances ; usage de SIECLE et STS-Web dans le privé sous contrat ; contrat d'association / simple ; formats SIECLE structures, STS-Web et son successeur, GAR, logiciels d'EDT ; codes de modalité d'élection ; volumes horaires de référence, règles de choix (LV2, spécialités), groupes de besoin ; calendrier austral et fuseaux d'outre-mer ; fériés d'Alsace-Moselle et d'outre-mer ; jeu de données ouvert du calendrier scolaire et sa licence ; rythme de 4 ou 4,5 jours au primaire ; recomposition annuelle des classes au collège ; comparaison avec les logiciels du marché ; durée de calcul de la constitution ; efforts des lots.

**Contradictions signalées** :
- `SOCLES.md` §3 (socle 5, « affectations des élèves → 2 ») contre `02_ELEVE.md` §7.4 et `03_INSCRIPTIONS.md` §5.1 (affectation au référentiel / socle 5) — Q-S1.
- `02_ELEVE.md` §7.4 (« option = groupe ») contre ce document (choix ≠ groupe) — Q-S2.
- `SOCLE2_DOMAINES.md` REF-12 (service d'enseignement au référentiel) et REF-13 (PP) contre `SOCLES.md` (services et PP au socle 6) — Q-S3, Q-S4.
- `SOCLE2_DOMAINES.md` §6.3 « 9 tables texte » : il y en a **10** avec `ent_edt_seance.classe` (`110:202`).
- `INTEROPERABILITE_NATIONALE.md:159` (« aucune colonne `uai` ») contre `110:14-15` — Q-S18.
- D-022 (le professeur ne voit que ses groupes) contre le code actuel (branche EDT de `perimetre.rs`, `classes_du_prof`) — lot S1.
- `SOCLES.md` place les réservations de salles au socle 44 et ce document garde les salles (lieux, capacité, indisponibilités) au socle 5 : cohérent, mais l'indisponibilité pour travaux (5) et la réservation (44) devront partager le même calcul de disponibilité (contrat à écrire au socle 44).
- **[personas]** Ce document et `06_PERSONNELS.md` (écrit sans pouvoir lire celui-ci) **possèdent tous deux** l'objet « ce qui est enseigné à qui » (cours ici, service d'enseignement là-bas) et annoncent chacun la **seule** fonction de périmètre (`structure::perimetre` ici, `services::eleves_vus` là-bas) — Q-S20, Q-S21.
- **[personas]** Le socle 6 consomme `annee.ouverte`, `annee.cloturee`, `classe.archivee`, `groupe.archive` ; ce document émet `referentiel.annee_ouverte`, `referentiel.annee_cloturee`, `referentiel.classe_archivee`, `referentiel.groupe_ferme` — R-S23.
- **[personas]** Le principe 8 (« une fermeture se déclare une fois et produit tous ses effets ») annulerait les repas des internes présents : précisé par R-S1, sans retirer le principe.

## 21. Avancement du code — lots S0 et S1 (2026-11-08, branche `ent-socle-structure-s0-s1`)

**Niveau : implémenté + testé en local (PostgreSQL 16) ; non validé par la CI GitHub ni par Florian.** Migrations **330** (`330_structure_garde_fous`) et **331** (`331_cours_membres`) ; `MIGRATION_TAG` inchangé (`370`, la plus haute du répertoire).

**S0 — fait** : défaut n° 3 (la lecture « mes classes » ne crée plus d'année ; liste vide hors année) ; n° 4 (l'ouverture d'une année applique les contrôles de la clôture : périodes de notes ouvertes = refus, sauf `forcer`, tracé dans l'événement `annee_cloturee`) ; n° 5 (clôture de la courante sans successeur refusée sauf `forcer` ; `POST /referentiel/annees` avec `courante=true` refusé s'il existe une courante ; une année clôturée ne se rouvre pas : trigger en base) ; n° 16 (trigger de cohérence établissement / année sur `ent_affectation` et `ent_service_enseignement` — **à la place des clés étrangères composées, reportées au lot S13**) ; n° 17 (toute suppression d'affectation est archivée dans `ent_affectation_supprimee`) ; n° 18 (élèves sortis exclus des portées « mes groupes » et « désignés ») ; n° 19 (tests sur base : échec en CI si `ENT_DATABASE_URL` manque, via `url_labo_ou_echec`). **Reste de S0** : n° 3 pour le trigger `classe_id` et `sync_affectation_classe` (création implicite d'une classe sur code inconnu), qui supportent des flux existants — à traiter avec les appelants ; n° 17 pour les écritures sans événement (lot M10). Erreur UAI signalée par la conception : **corrigée dans la documentation** (`INTEROPERABILITE_NATIONALE.md` : `ent_site.uai` existe).

**S1 — fait** : *objet unique « cours »* = la ligne de `ent_service_enseignement` enrichie (recommandation Q-S20, **retenue par défaut, à confirmer**) ; table `ent_cours_membre` (inclusion / exclusion datées, motif en liste fermée, annulation sans suppression, refus des chevauchements, période dans l'année du cours) ; **une seule règle** : la fonction SQL `ent_public_cours_jour` (public = classe ou groupe + inclusions − exclusions, hors sortis), appelée par `enseigne_a`, `prof_suit_eleve_le`, `eleves_suivis_par_prof` (donc `eleves_autorises`), le remplacement et `mes_enseignements_for_actor` (effectif). Routes : `GET /referentiel/cours/{id}/public` (gestion ou professeur du cours, borné à ses élèves), `GET|POST /referentiel/cours/{id}/membres`, `POST /referentiel/cours-membres/{id}/annuler` (gestion seulement) ; événements `referentiel.cours_membre_ajoute` / `_annule` **sans motif ni identité** ; ajoutées à la matrice de séparation (`domaine_cours`, placeholder `{cours}`).

**Limites connues** : (1) un membre individuel s'attache à une ligne de service ; en cas de co-intervention (deux professeurs sur la même cible), il faut un membre par ligne jusqu'à la factorisation `ent_cours` / interventions (S4 et socle 6). (2) Une exclusion d'un cours de classe entière retire aussi l'élève au professeur principal qui n'a que ce cours (règle unique, voulue). (3) La branche emploi du temps historique de `perimetre.rs` subsiste pour les élèves sans affectation de classe (retrait au lot M11). (4) Les motifs (ulis, upe2a…) sont conservés tant que la ligne existe : politique de purge à fixer avec le socle 2. (5) Le passage de `ref_ouvrir_annee` à un refus sans `forcer` doit se refléter dans l'écran Leptos d'ouverture d'année (non modifié ici).

**Revue de sécurité S0/S1 (2026-11-08, auto-revue + relecture indépendante `review-sonnet`, statique ; pas une certification)** : aucun défaut critique ni élevé. Corrigés dans la migration 331 : membres de cours modifiables hors annulation (immuables désormais), modification en année clôturée (refusée), course sur le chevauchement (verrou). **Restent ouverts, documentés** : (M1) la branche « remplaçant » de `perimetre.rs` rattache le créneau à la classe texte et non au cours précis du titulaire — préexistant, élargi par les inclusions ; (F2) la branche EDT historique ignore les exclusions pour les élèves sans affectation de classe — les deux disparaissent au retrait M11 ; (F3) l'archive `ent_affectation_supprimee` n'a ni purge ni protection UPDATE/DELETE — politique de rétention à fixer ; (F4) `forcer` n'exige pas de droit dédié et la clôture ne prend pas le verrou d'établissement.

---

## 22. Avancement du code — lot S2 « profil, paramètres, lieux » (2026-11-08, branche `ent-socle-structure-s2`, base `ent-integration-lot9`)

**Niveau : implémenté + testé en local (PostgreSQL 16, rôle `ent_ci`) ; non validé par la CI GitHub ni par Florian ; pas d'écran (aucun écran existant n'a cassé).** Migrations **332** et **337** ; `MIGRATION_TAG` inchangé (`375_msg_fil_prive`, la plus haute).

**Fait (STR-01 à STR-10, STR-109, STR-110)** :
- *Profil* (`ent_etablissement_profil`, `PgEnt::structure_profil[_definir]`) : identité, UAI (format national, pas de vérification d'existence), secteur public / privé sous contrat / hors contrat, contrat d'association ou simple (seulement sous contrat), degrés, types, académie, département, zone de calendrier, régime local, tutelle, coordonnées, semaine type (jours ouvrés et demi-journées), version et concurrence optimiste (`version_attendue`). Lire ne crée jamais de ligne.
- *Fuseau* : **source unique** `ent_etablissement_parametre.fuseau` (migration 265), non dupliquée dans le profil (écart volontaire avec le schéma §9.1 qui y prévoyait une colonne : deux sources auraient pu diverger). `structure_fuseau_definir` valide le nom IANA par PostgreSQL ; changer de fuseau pendant une année courante exige `confirmer` (★ de STR-03).
- *Préréglages* (STR-02) : six profils (collège, lycée général, lycée professionnel, école, privé sous contrat, hors contrat) ; **aperçu** sans écriture, application **une seule fois** (`profil_preregle`), copie modifiable, ne crée ni site, ni salle, ni classe, n'écrase jamais un paramètre déjà personnalisé.
- *Paramètres* (STR-10) : registre de clés dans le code (`registre()`), clé inconnue refusée, valeur typée et bornée, version incrémentée, avant/après au journal. Quatre clés au départ (durée de séance, coupure matin/après-midi, trajet par défaut, salles réservables par défaut).
- *Journal* `ent_structure_journal` : ajout seul (déclencheur), RLS, lu par `GET /structure/journal`.
- *Sites* : degrés, couleur, exception de fuseau, UAI contrôlé, fermeture **datée** refusée s'il reste des classes ouvertes, des bâtiments ou des salles ouverts (STR-05). *Bâtiments* (STR-06). *Salles* typées (11 types), capacité 0–500, équipements, accessibilité PMR, réservable, étage, fermeture datée refusée si un créneau hebdomadaire l'utilise (STR-07).
- *Lieux extérieurs* (STR-109) : adresse, propriétaire, convention (identifiant de document du socle 7), temps et mode de trajet, fermeture ; une salle de lieu extérieur est de type `exterieur`.
- *Disponibilités et indisponibilités* (STR-09, STR-109, STR-110) : fenêtres hebdomadaires d'une salle ou d'un lieu (table `ent_disponibilite`, plus générique que `ent_salle_disponibilite` du §9.10) ; indisponibilités datées, **immuables** (annulées, jamais supprimées), **en lot** avec un même `lot_id` et un rapport des créneaux hebdomadaires touchés ; port `structure_disponibilite::disponible` (fermé, indisponible, hors fenêtres, règles du lieu pour ses salles).
- *Import de salles en lot* (STR-08) : CSV collé (`;` ou `,`, guillemets, BOM), en-tête obligatoire (`code;libelle;site` + colonnes facultatives), **aperçu = même chemin que l'import puis annulation**, rejets ligne à ligne avec raison, idempotent (clé = code, cellule vide = valeur conservée), colonne inconnue = fichier refusé, 2 000 lignes / 512 Kio au plus, caractères de commande refusés. Site, bâtiment et lieu doivent exister : **rien n'est créé par un import**.

**Critères de fin** : (1) outre-mer — test `outre_mer_heure_locale_juste_et_port_fuseau` (21 h UTC le 10 novembre = 01 h le 11 à La Réunion, 22 h le 10 à Paris, même source que `ent_aujourdhui_etab`) ; (2) salle liée à un site de son établissement — clé étrangère composite `(site_id, etablissement_id)` **et** déclencheur (site obligatoire à la création, jamais détaché), test `salle_toujours_liee_a_un_site_de_son_etablissement` (site d'un autre établissement, site texte inconnu : refus, aucun site créé) ; (3) aucune création implicite — tests de lecture à vide, de préréglage, d'import ; (4) port `structure::fuseau` — voir ci-dessous.

**Port et point de branchement du socle 6** : `ent_db::structure_etab::structure::{fuseau, aujourdhui, heure_locale, parametre}` prennent la connexion/transaction de l'appelant (`&mut PgConnection`, donc `&mut TenantTx`) ; `ent_db::structure_lieux::structure_disponibilite::disponible` pour les socles 10 et 42. **Le socle 6 (lot P0) doit remplacer son « Europe/Paris » en dur par `structure::fuseau(conn, etab)` (nom IANA) ou `structure::heure_locale` / `structure::aujourdhui` ; Europe/Paris reste la valeur par défaut d'un établissement sans réglage** (R-P2). Le socle 4 le fait déjà via `ent_aujourdhui_etab`.

**Droits** : lecture `referentiel.lire` (tout le personnel), écriture `referentiel.gerer` (administrateur, direction, administratif). L'action dédiée `etablissement.parametrer` du §13.2 **n'est pas créée** (le raccourci `referentiel.gerer` en tient lieu, comme prévu) : à introduire avec le catalogue du §13 pour permettre un réglage plus fin. Les familles et élèves n'ont aucun accès. Établissement toujours pris dans la session, jamais dans le corps.

**Cloisonnement** : toutes les tables du lot sont en RLS forcée (7 nouvelles : profil, paramètres, journal, bâtiments, lieux, disponibilités, indisponibilités → 82 au total, test `rls_politiques_presentes_et_forcees`), écritures par `TenantTx`, clés étrangères composites sur `etablissement_id`.

**Routes** (`ent-api/src/structure.rs`, sans logique) : `/structure/{profil, fuseau, prereglages, parametres, journal, sites, batiments, lieux, salles, salles/import, …/disponibilites, …/indisponibilites, indisponibilites/lot}`. L'import est un **aperçu par défaut** (il faut `apercu:false` pour écrire).

**Compatibilité** : l'ancienne route `POST /salles` (`creer_salle_for_actor`) n'a pas changé de signature mais exige désormais un site existant (texte = code ou libellé exact d'un site de l'établissement) ; sinon 400. Le labo (`ent-lab`) et `tests_edt_s2` déclarent explicitement leur site. Les salles historiques sont rattachées par correspondance **exacte** du texte avec un site existant du même établissement ; les autres restent « sans_site » (signalées dans la liste, rattachables par `POST /structure/salles/{id}/rattacher`).

**Limites connues / à décider** : (1) pas d'écran (S12b) ; (2) `adresse_id` (modèle d'adresse du socle 4) non branché : adresses en texte libre ; (3) `organisme_gestionnaire_id` non branché (socle 4 §8.2) ; (4) logo et convention : identifiants de document du socle 7, sans clé étrangère ; (5) `evenement_ref` d'une indisponibilité de lot : attend le calendrier (S3), « déplacées ou annulées avec l'événement » (STR-110) reste à faire à S3 ; (6) « séances touchées signalées au socle 10 » : le rapport donne un **nombre de créneaux hebdomadaires**, pas la liste des séances datées (il n'existe pas encore de séances liées aux salles) ; (7) l'ancienne route `PUT /referentiel/sites/{id}` peut encore passer `actif=false` sans date de fermeture ni contrôle : à retirer au profit de `/structure/sites/{id}/fermer` quand les écrans basculeront ; (8) les événements `referentiel.*` du lot n'ont pas encore de consommateur ; (9) l'unicité des codes est sensible à la casse en base (les codes de salle, bâtiment et lieu sont mis en majuscules par le service) ; (10) aucune revue de sécurité indépendante n'a été faite pour ce lot (auto-revue seulement).

---

## 23. Avancement du code — lot S5 « offre et enseignements suivis » (2026-11-08, branche `ent-socle-structure-s5`, base `ent-socle-structure-s2`)

**Niveau : implémenté + testé en local (PostgreSQL 16, rôle `ent_ci`) ; non validé par la CI GitHub ni par Florian ; pas d'écran (S12d).** Migrations **338** (`338_structure_offre`) et **340** (`340_eleve_enseignement`) ; `MIGRATION_TAG` inchangé (`375_msg_fil_prive`, la plus haute du répertoire).

**Fait (STR-36, 39, 41, 42, 43, 62, 68 ; R-S16 ; Q-S2)** :
- *Matières* (STR-36, 39) : `ent_matiere` enrichie (libellé court, couleur, famille, code national, **sous-matière à un seul niveau**, archivage). Code unique sans tenir compte de la casse, mis en majuscules ; le code ne change jamais. Archivage **refusé** si un cours ou une offre ouverts (année non terminée) l'utilisent, ou si elle a des sous-matières actives ; une offre ne se crée pas sur une matière archivée.
- *Offre par année* (STR-41) : `ent_offre_enseignement` (modalité, admission libre / sur avis / sur test / sur dossier, capacité, ouverture aux vœux, public classe ou groupe, date limite de choix, hors contrat), `ent_offre_cible` (niveau **ou** MEF ; aucune cible = tous), `ent_offre_groupe` (groupe porteur d'une offre ; un groupe ne porte qu'une offre, de la même année). Une offre ne se supprime jamais : elle se ferme ; une année clôturée gèle l'offre et les enseignements suivis (déclencheurs).
- *Règles de choix* (STR-42) : `ent_offre_regle_choix` (+ table de membres, pas de tableau d'UUID : clés étrangères vérifiées) — minimum / maximum d'enseignements suivis parmi une liste, par niveau, MEF ou tous, bloquante ou avertissement ; `ent_offre_incompatibilite` (paire non ordonnée, bloquante ou non, avec motif) ; `ent_offre_prerequis` (autre offre de la même année, ou **matière suivie l'année précédente**). Aucune règle nationale codée.
- **Un seul moteur** de vérification (`structure_offre::offre::verifier_choix`) : appelé par les enseignements suivis **et** par le port du socle 3. Les maximums, incompatibilités, prérequis, cibles, offres fermées et capacités sont vérifiés à chaque écriture ; **les minimums ne bloquent qu'avec `exiger_complet`** (un choix en cours de saisie n'est pas un choix incomplet), sinon ce sont des avertissements. Capacité : verrou sur l'offre le temps du calcul, dépassement possible par `forcer_capacite` (tracé au journal).
- *Enseignements suivis* (STR-62, `ent_eleve_enseignement`) : **distincts des groupes**. Cycle `choisi → accepté → quitté` ou `choisi → refusé | retiré`, tenu par la base : états terminaux immuables, pas de suppression, historique en ajout seul alimenté par déclencheur (`ent_eleve_enseignement_histo`), pas d'acceptation d'une offre non libre sans **avis favorable** (admission sur avis ou test, R-S16 pour la suite). Enseignement quitté = nature R-S16 (`changement_en_cours_annee`, `abandon_fin_premiere`, `fermeture_offre`, `depart`, `correction`), date de fin (dernier jour suivi, inclus) et motif ; il se rechoisit par une nouvelle ligne. Commande unique `structure_enseignements_modifier` (ajouter et quitter en une transaction, règles appliquées à l'**ensemble final**, aperçu sans écriture, refus bloquant = `ecrit:false`, HTTP 409 avec la liste des écarts). Décision d'admission : `structure_enseignement_decider`.
- *Fermeture d'un enseignement* (STR-43) : aperçu (élèves touchés) puis confirmation ; **direction et administrateur seulement** ; les enseignements acceptés sont quittés (nature `fermeture_offre`), les choix en attente retirés ; l'événement `referentiel.offre_fermee` ne porte **aucune identité**.
- *Contrôle « choix sans groupe »* (STR-68, fonction SQL `ent_enseignement_incoherences`, route `/structure/enseignements-incoherences`, **gestionnaires seulement**) : `sans_groupe`, `plusieurs_groupes`, `groupe_sans_choix`, `offre_sans_groupe` ; un élève qui a quitté l'enseignement reste suivi jusqu'à la date de fin incluse.
- *Reprise des options existantes* (critère « aucune option perdue ») : `ent_eleve_option` et `ent_eleve_option_annee` sont reprises en offres (créées depuis les codes rencontrés, `source = reprise_option`, **jamais ouvertes aux vœux**, associées au groupe homonyme s'il existe) et en enseignements suivis. Ce qui ne peut l'être (code vide, date ou année inconnue, erreur) est rangé dans `ent_eleve_enseignement_a_traiter` — **la reprise ne crée jamais d'année**. Correspondance conservée (`ent_eleve_enseignement_reprise`), reprise **idempotente**, pont à sens unique qui suit les écritures futures des anciennes tables sans jamais les faire échouer. Comparaison avant / après : `/structure/enseignements-reprise` (héritées = reprises + à traiter).
- *Port de lecture pour le socle 3* : `ent_db::structure_offre::offre::{offres_ouvertes_aux_voeux, lire, regles_applicables, verifier_choix}` prennent la connexion de l'appelant, **sans acteur** (l'appelant a déjà autorisé) ; places restantes calculées. Branché côté socle 3 : `PgEnt::enseignements_proposes_for_actor` (inscriptions, campagne + offre d'inscription → enseignements proposés pour l'année de la campagne et le niveau de l'offre).

**Critères de fin** : (1) aucune option perdue — test `options_historiques_reprises_sans_perte_et_sans_creation_d_annee` (huit lignes dont code vide, année inconnue, doublon ; reprise rejouée = 0 ; aucune année créée) ; (2) règles de choix appliquées — `regles_de_choix_incompatibilites_prerequis_capacite_et_cibles_appliquees`, `prerequis_et_admission_sur_avis_avec_historique` ; (3) offre lue par le socle 3 via le port — `offre_lue_par_le_port_pour_le_socle_3`, `socle_3_lit_l_offre_d_enseignements_d_une_campagne` ; (4) périmètre D-022 — `professeur_ne_voit_que_les_enseignements_de_son_perimetre_d022` (le professeur d'allemand lit les enseignements de ses élèves d'allemand, pas ceux d'espagnol ni d'un élève sans lien ; le professeur principal lit toute sa classe ; un autre établissement répond « introuvable »).

**Droits** : catalogue et règles en lecture `referentiel.lire` (tout le personnel), écriture `referentiel.gerer` (administrateur, direction, administratif) ; fermeture d'une offre : direction et administrateur. Lecture des enseignements d'**un élève** : gestionnaire, ou personnel dont la portée D-022 (`autoriser_eleve`, action `suivi.eleve`) contient l'élève ; hors périmètre = refus sans oracle d'existence ; le **motif libre** (texte) est masqué hors gestionnaires (il peut dire une raison de santé). Comptabilité et familles n'ont aucun accès aux enseignements d'un élève dans ce lot. Établissement toujours pris dans la session.

**Cloisonnement** : 11 tables nouvelles en RLS forcée (93 au total, test `rls_politiques_presentes_et_forcees` ; test strict `rls_strict_tables_du_lot_s5` sous rôle non propriétaire), clés étrangères **composites** `(id, etablissement_id)` pour l'année, la matière, le niveau, le MEF, le groupe, l'élève et l'offre ; journal `ent_structure_journal` étendu (objets `matiere`, `offre`, `offre_groupe`, `regle_choix`, `incompatibilite`, `prerequis`, `enseignements_suivis`, `enseignement_suivi`). Registre de fusion d'élèves (`TABLES_ELEVE`) : `ent_eleve_enseignement` *déplacée* (le conservé l'emporte en cas de conflit), `ent_eleve_enseignement_a_traiter` reste.

**Routes** (`ent-api/src/structure_offre.rs`, sans logique) : `/structure/{matieres, offres, regles-choix, incompatibilites, eleves/{id}/enseignements[/controle], enseignements/{id}/historique|decision, enseignements-incoherences, enseignements-reprise}` et leurs actions. Événements : `referentiel.matiere_creee|modifiee|archivee`, `referentiel.offre_modifiee`, `referentiel.offre_fermee`, `referentiel.enseignements_suivis_modifies` — sans identité d'élève.

**Limites connues / à décider** :
1. *Pas d'écran* (S12d) ; pas d'accès des **familles** et des **élèves** à leurs propres enseignements (viendra avec STR-100 et le socle 3 : le dépôt d'un choix par la famille n'est pas fait ici).
2. *Niveau de l'élève* : déduit de sa classe active de l'année (`ent_classe.niveau_id`) ; **le MEF de l'élève n'est pas connu** de ce lot : une cible ou une règle par MEF vaut pour le niveau de ce MEF (déduction documentée). Un élève sans classe n'a pas de contexte : cibles et règles par niveau ne sont pas vérifiées, avertissement `contexte_inconnu`.
3. *Groupes* : `ent_offre_groupe` est la seule liaison offre / groupe tant que S6 (composition, 339/342) n'existe pas ; l'association est explicite (ou automatique pour les offres reprises des options, par code homonyme) ; **aucune écriture de groupe n'est déclenchée par un choix** (« groupe proposé » du STR-62 reste à S6).
4. *Non faits de cette tranche* : STR-37 (import de matières depuis la nomenclature nationale : la table `ref_nomenclature` du socle 9 n'existe pas), STR-38 (fusion de matières ★ et défusion 30 jours), STR-44 / STR-45 (maquette horaire et dotation : lot S10), propositions de choix de remplacement envoyées aux familles à la fermeture (socle 8), demandes de changement (STR-100, S6), date limite de changement = avertissement seulement.
5. *Reprise des anciennes options* : dans `ent_eleve_option_reprendre()` (lot E4, migration 286, rejouée à chaque démarrage), une option dont la date ne tombe dans aucune année fait échouer le pont historique (`ent_assurer_annee` ne crée pas l'année et le groupe est créé sans année) : **défaut préexistant du lot E4**, non corrigé ici ; la reprise de ce lot, elle, range proprement ce cas « à traiter ».
6. *Admission* : l'avis est saisi par un gestionnaire (pas par le professeur concerné ni par le PP : STR-100) ; `sur_test` et `sur_dossier` ne se distinguent pas de `sur_avis` pour le moteur.
7. *Capacité* : seule l'acceptation occupe une place ; une offre à acceptation soumise à avis peut donc recevoir plus de demandes que de places (comportement voulu, la décision vérifie la capacité) ; baisser la capacité sous l'effectif n'évince personne (tracé).
8. *Retraits* : `ent_eleve_option` / `ent_eleve_option_annee` ne sont pas retirées (M10 à M12 : lot S13, après validation par Florian).
9. *Aucune revue de sécurité indépendante* (auto-revue seulement) ; la liste des écarts « choix sans groupe » nomme des élèves de tout l'établissement : réservée aux gestionnaires (le STR-68 prévoyait `referentiel.lire`).

**Vérifications (2026-11-08, local, PostgreSQL 16, rôle `ent_ci`)** : clippy du workspace avec et sans `--features ent-api/labo,ent-lab/labo` : 0 erreur, aucun avertissement sur les fichiers du lot ; `ent-db --features labo` : 12 tests du lot S5 (ent-db) verts, 115 verts sur le sous-ensemble ciblé (RLS, fusion d'élèves, référentiel, D-022, structure S1 à S5, élève socle et scolarité, inscriptions I2, A1) ; `ent-db` sans labo : 136 + 3 verts ; `ent-api --features labo` : test HTTP S5 et test HTTP S2 verts, matrice de séparation 43/43 (inventaire étendu à `structure.rs` et `structure_offre.rs`, sans quoi son test d'inventaire échouait déjà sur S2) ; `ent-api` sans labo : verts ; `domain-eleve` : 235 verts. **La suite complète d'`ent-db` n'a pas été menée à terme en local** (≈ 180 tests en 50 min sur une base partagée) : la CI GitHub fait foi.

**Échecs préexistants constatés sur la base (pas introduits par S5, non corrigés ici)** : (a) `domain-eleve fusion::tests::domaines_lisibles_et_proteges` attend le domaine « Inscriptions » pour `ent_eleve_inscription`, la règle donne « Scolarité » ; (b) `ent-db tests_eleve_fusion::garde_du_registre_tables_eleve` : `ent_cours_membre` (S1) et `ent_affectation_supprimee` (S0) portent `eleve_id` sans figurer au registre de fusion (stratégie à choisir : le déclencheur d'`ent_cours_membre` interdit de changer `eleve_id`, donc *Déplacer* exigerait de l'autoriser pendant une fusion) ; (c) `ent-db --test isolation_sql_statique` : 5 requêtes sans filtre d'établissement (`eleve_fusion.rs` ×2, `propositions.rs`, `structure_lieux.rs` ×2 — deux sont du lot S2) ; les deux fragments du lot S5 ont été corrigés (le filtre est dans la constante, comme demandé par le garde).

---

## 24. Avancement du code — lot S4 « cours » (2026-11-09, branche `ent-socle-structure-s4`, base `ent-socle-structure-s5`)

**Niveau : implémenté + testé en local (PostgreSQL 16, rôle `ent_ci`) ; non validé par la CI GitHub ni par Florian ; pas d'écran (S12g).** Migration **341** (`341_cours`) ; `MIGRATION_TAG` inchangé (`375_msg_fil_prive`, la plus haute du répertoire). Retouche de la migration 338 (voir « Points d'attention »).

**Fait (M7, STR-57, Q-S3, Q-S20, R-S5)** :
- *L'objet « cours »* (`ent_cours`) = **ce qui est enseigné à qui** : année, **public = une classe entière OU un groupe**, matière (référentiel ou libellé libre), offre d'enseignements de l'année (facultative, S5), volume hebdomadaire, semaines (`toutes`/`A`/`B`), ouverture et fermeture, reconduction, origine (`saisie`, `reprise`, `service`, `generation`), drapeau `a_verifier`. **Le service reste « qui l'enseigne »** (`ent_service_enseignement`, qui garde ses colonnes) et pointe vers son cours par `cours_id` : Q-S3 « cours ≠ service » est tranchée comme recommandé (**séparer, avec reprise automatique**). Cela remplace l'option « la ligne de service *est* le cours » retenue provisoirement au lot S1 (Q-S20 : la recommandation « une seule table » devient « un seul **objet** cours, que les services visent » ; le socle 6 pose ses interventions sur ce cours).
- *Invariants tenus par la base* : public, matière, année et ouverture ne changent jamais (« changer de public = fermer et ouvrir », STR-57 ; l'historique des notes reste sur l'ancien cours) ; un cours **se ferme** (définitif) et **ne se supprime pas** ; un seul cours **ouvert** par (année, public, matière), ou par libellé quand il n'y a pas de matière du référentiel ; classe et groupe de l'année du cours ; année clôturée = gel ; offre de la même année.
- *Poste vacant = cours sans service* : pas d'état stocké, un fait déduit (`ent_cours_vacants`, `vacant` dans la vue). Un cours vacant a un public (la classe) mais **aucun professeur ne le voit** (le filtre professeur passe par les services). Pourvoir un poste = poser un service sur le cours (`POST /structure/cours/{id}/services`) ; un second professeur sur le même cours est une co-intervention.
- *Écrivains historiques inchangés* : un déclencheur `ent_service_cours` trouve ou crée le cours d'un service inséré sans `cours_id` (donc `creer_service`, l'ouverture d'année, la création depuis l'EDT, les jeux de labo continuent de fonctionner) ; à l'inverse un service inséré **avec** `cours_id` en recopie la cible (c'est ainsi que le socle 6 posera ses services). Un créneau d'emploi du temps (`ent_edt_creneau.cours_id`) hérite du cours de son service ; il peut aussi viser un cours sans service.
- *Une seule règle de visibilité* : `ent_public_cours_jour` est **redéfinie sur le cours** (même signature qu'au lot S1) : public = (classe ou groupe à la date) + inclusions actives − exclusions actives, hors élèves sortis, dans la fenêtre d'ouverture du cours ; le filtre professeur = « a un service sur ce cours ». `perimetre.rs`, `eleves_autorises`, « mes enseignements » et le remplacement l'appellent toujours sans changement de code.
- *Membres individuels (R-S5) rattachés au cours* : `ent_cours_membre` (S1, **nom conservé** à la place de `ent_cours_membre_individuel` du §9.10) désigne désormais le **cours** : une inclusion ou une exclusion posée une fois vaut pour **tous** les professeurs du cours (la limite n° 1 du lot S1, « un membre par ligne de service », disparaît). Le journal de structure garde chaque pose et chaque annulation **sans le motif** (donnée sensible) ; les événements restent sans identité. Les anciens identifiants de service restent acceptés par les routes `/referentiel/cours/{id}/…`.
- *Reprise automatique (M7), idempotente, sans perte* — fonction `ent_cours_reprendre()` exécutée à chaque démarrage : chaque service reçoit son cours (clé de dédoublonnage : année, public, matière — ou libellé sans matière) ; **deux professeurs sur la même cible et la même matière = un cours, deux services** ; un service sans matière ni libellé donne un cours « Cours à préciser » marqué `a_verifier` (le service professeur principal vide en fait partie) ; les créneaux reçoivent le cours de leur service ; les membres sont **repointés de la ligne de service vers le cours** (ancienne référence gardée dans `service_origine_id`, aucune ligne perdue ni modifiée dans ses dates). Ce qui ne peut pas être repris est rangé dans `ent_cours_a_traiter`, sans faire échouer le démarrage. La reprise **ne crée jamais d'année** (les années clôturées sont reprises avec un drapeau local qui lève seulement le gel, pas les autres contrôles). Comparaison avant / après : `GET /structure/cours/reprise` (services avec cours, à traiter, membres sur un cours, créneaux sans cours — **compteur de M11**).
- *Commandes* (journal avant / après, événements sans identité) : créer (l'année est celle du public, jamais du corps), modifier (libellé, offre, volume, semaines), **fermer** (date incluse, nombre de créneaux touchés rendu), pourvoir, lister (année courante par défaut, filtres classe / groupe / vacants / fermés), vacants, détail avec services (gestion seulement). Événements `referentiel.cours_cree|modifie|ferme|pourvu`.
- *Contrat de lecture pour les autres socles* (`ent_db::structure_cours::cours`, fonctions qui prennent la connexion ou la transaction de l'appelant, sans acteur) : `lire`, `du_service`, `services_du_cours`, `vacants`, `public`, `poser_service`, `creneaux_sans_cours`. **Socle 6** : poser ses services / interventions SUR un cours (`poser_service` recopie la cible ; PRS-40 appelle la création du cours, STR-57) ; ne pas recalculer le périmètre (Q-S21) ; un cours sans service est un poste vacant. **Socle 10** : `ent_edt_creneau.cours_id` (rempli par la base à partir du service ; M11 se mesure par `creneaux_sans_cours`) ; les séances, devoirs (`ent_devoir.service_id`) et entrées du cahier de textes **gardent leur `service_id`** et retrouvent le cours par `ent_service_enseignement.cours_id` (aucune colonne ajoutée dans ces tables ici).

**Critères de fin** : (1) *périmètre avant / après identique* — test `reprise_idempotente_sans_perte_et_perimetre_identique` (les élèves suivis par trois professeurs sont identiques avant et après une reprise rejouée depuis un état « d'avant S4 », aucun service ni créneau ni membre perdu, reprise rejouée = 0) ; (2) *poste vacant représenté* — `poste_vacant_cours_sans_service` ; (3) *reprise idempotente* — idem + dédoublonnage de deux services d'avant (un cours, deux services, origine `reprise`) ; (4) *un professeur ne voit que les élèves de ses cours* — `professeur_ne_voit_que_les_eleves_de_ses_cours` (LV2 mixte allemand / espagnol, groupe, inclusion individuelle, exclusion individuelle, co-intervenant, annulation, chevauchement refusé) et la matrice D-022 existante (`tests_d022`, `tests_d022_bis`, `tests_structure_s1`) restée verte ; (5) HTTP — `routes_cours_droits_cloisonnement_poste_vacant_et_conflits` (401, 403 famille et professeur, 404 entre établissements, 409 de doublon, de pose et de fermeture) et la matrice de séparation étendue aux routes du cours.

**Droits** : lecture (liste, vacants) `referentiel.lire` (tout le personnel ; la liste ne nomme aucun professeur) ; détail avec professeurs, création, modification, fermeture, pose d'un service, état de la reprise : `referentiel.gerer` (administrateur, direction, administratif). Familles et élèves : aucun accès. Établissement toujours pris dans la session.

**Cloisonnement** : `ent_cours` et `ent_cours_a_traiter` en RLS forcée (97 tables au total), clés étrangères **composites** `(id, etablissement_id)` pour l'année, la matière, l'offre, la classe, le groupe, la reconduction, le service, le créneau et le membre ; commandes sous `TenantTx`.

**Décision prise (S5 la laissait ouverte) — fusion d'élèves et `ent_cours_membre`** : le test de garde du registre de fusion échouait parce que `ent_cours_membre` et `ent_affectation_supprimee` portent `eleve_id` sans être déclarées. **Choix le plus prudent : `Reste`** (les lignes restent sur la fiche absorbée, lisibles via la fusion, jamais perdues). Raisons : une inclusion ou une exclusion est une **décision individuelle datée** dont le motif est sensible (ULIS, UPE2A…) ; la déplacer sans regard ferait apparaître un motif de santé sur la fiche conservée et pourrait créer un chevauchement qui bloquerait la fusion ; l'archive des affectations supprimées est une trace d'audit. **Conséquence à connaître** : après une fusion, une inclusion qui concernait l'élève absorbé n'est plus appliquée à la fiche conservée — la gestion la ressaisit (le détail reste lisible). Le registre est **provisoire** : si Florian préfère « déplacer » (avec exception du drapeau local `ent.fusion_eleve` dans le déclencheur), c'est une décision à reprendre au lot de fusion. Pour que le test de clé du registre passe, `ent_affectation_supprimee` reçoit une clé primaire (`id`) en 341 (ignorée s'il existait des doublons).

**Points d'attention / limites** :
1. *Migrations 331 et 338 retouchées (rejeu à chaque démarrage)* : (a) la contrainte de liste fermée `ent_structure_journal_objet_check` est `NOT VALID` en 338 **et** en 341 (liste complète avec `cours`, `cours_membre`, `cours_service`). **Sans cela, tout redémarrage échouait** dès qu'une ligne de journal du lot S4 existait (338 contrôlait sa liste étroite contre les lignes écrites par 341) ; `NOT VALID` évite en plus de parcourir tout le journal sous verrou à chaque démarrage (les lignes existantes respectent déjà la liste, qui ne fait que s'allonger). **Règle pour les lots suivants** : un lot qui étend cette liste la redéfinit complète, en `NOT VALID`. (b) 331 ne redéfinit plus `ent_trg_cours_membre` ni `ent_public_cours_jour` quand `ent_cours` existe (la revue a relevé qu'à chaque rejeu, 331 remettait l'ancienne règle jusqu'à 341 : fenêtre pendant laquelle les exclusions n'étaient plus appliquées pour une instance déjà en service).
2. *Non fait de ce lot* : STR-56 (génération des cours depuis la maquette et la structure : la maquette est au lot S10), STR-58 (barrettes / alignements, lues par le socle 10), reconduction d'un cours d'une année à l'autre (colonne `reconduit_de_id` prévue, pas de commande ; lot S8), import / export de cours (S11), écrans (S12g), accès des professeurs à « leurs » cours autrement que par `mes-enseignements` (qui porte maintenant `cours_id`).
3. *Branche emploi du temps historique de `perimetre.rs`* (M1) : **non retirée** — il reste des créneaux sans `service_id` donc sans cours (compteur `creneaux_sans_cours`) ; le retrait (M11) attend que ce compteur soit à zéro. Les deux réserves de la revue S0/S1 (M1 : le remplaçant est rattaché à la classe texte et non au cours du titulaire ; F2 : la branche EDT ignore les exclusions pour les élèves sans affectation de classe) sont **inchangées** et disparaîtront avec M11.
4. *Corrections au périmètre D-022* : une seule, voulue (à confirmer par Florian) — un membre individuel posé une fois vaut pour **tous** les professeurs du cours (avant, il fallait un membre par ligne de service) : l'inclusion posée pour le professeur A est donc aussi visible du co-intervenant B, et l'exclusion le masque aussi à B. À la reprise, deux professeurs sur la même cible et la même matière partagent leurs membres. Aucune autre différence n'est connue. Les services n'ont pas de dates avant le socle 6 : un ancien titulaire ou un remplaçant garde sa vue tant que son service existe (inchangé).
4 bis. *Revue indépendante* (`review-sonnet`, statique, 2026-11-09) : aucun défaut critique ni fuite entre établissements ; traités : reprise des créneaux et des membres protégée contre l'échec (avertissement, le démarrage continue), propagation du cours aux créneaux, contrôle d'année clôturée à la pose d'un service, professeur au journal de la pose, drapeau `a_verifier` conservé par une modification vide. Restent : `structure_cours_modifier` ne peut pas vider volume, semaines ni offre ; un créneau n'est pas contrôlé contre la classe de son cours (seul le service l'impose) ; les lignes de `ent_cours_a_traiter` ne sont pas purgées ; un service « à traiter » n'est vu d'aucun professeur (échec fermé).
5. *Cours sans matière ni libellé* (service professeur principal historique vide) : cours « Cours à préciser », `a_verifier = true` ; la modification du libellé lève le drapeau.
6. *Le public d'un cours de groupe suit l'affectation au groupe* : tant que S6 (composition) n'existe pas, un changement de groupe en cours d'année passe par les affectations existantes ; aucune écriture de cours n'est déclenchée.
7. *Services d'une année clôturée* : repris avec le gel levé localement le temps de la reprise ; un nouveau service ne peut pas être posé sur un cours d'une année clôturée. Si un service change de cible ou de matière, il suit un autre cours (créé au besoin) et ses créneaux le suivent ; ses membres restent sur l'ancien cours (qui peut devenir vacant).
8. *Aucune revue de sécurité indépendante* (auto-revue seulement). Points à relire : déclencheur de reprise (drapeau `ent.cours_reprise` transactionnel, jamais posé par l'application), immutabilité des membres, filtre établissement de chaque requête de `structure_cours.rs`.

**Vérifications (2026-11-09, local, PostgreSQL 16, rôle `ent_ci`, base partagée entre tests)** : clippy du workspace avec et sans `--features ent-api/labo,ent-lab/labo` : aucune erreur, aucun avertissement sur les fichiers du lot (les avertissements restants sont ceux de `domain-eleve`, `ent-mail`… déjà présents) ; `ent-db --features labo` : `tests_structure` 34/34 après correction (S1 6/6, S2 11, S4 5 dont RLS strict sous rôle non propriétaire, S5 12), `tests_d022` 6/6, `tests_eleve_fusion` 7/7 (**le test de garde du registre de fusion, en échec sur S5, passe**), `tests_rls` 14/14 (97 tables en RLS forcée), `tests_referentiel` 10/10, `tests_communication` 16/16, `tests_edt` 10/10, `migrate` 1/1 ; `ent-db` sans labo 136/136 ; `ent-api --features labo` : test HTTP S4, S2 et S5 verts, **matrice de séparation 43/43** (inventaire étendu à `structure_cours.rs`, routes du cours ajoutées) ; `ent-api` sans labo verts. **Un test d'`ent-db` a dû être adapté** : `membres_garde_fous_de_saisie` (S1) attendait « valeur invalide » pour un cours inconnu, c'est désormais « introuvable » (l'identifiant est résolu avant toute écriture). **Échecs préexistants, hérités de S5 et non corrigés ici** : (a) `domain-eleve fusion::tests::domaines_lisibles_et_proteges` (domaine de `ent_eleve_inscription`) ; (c) `ent-db --test isolation_sql_statique` : 5 requêtes sans filtre d'établissement (`eleve_fusion.rs` ×2, `propositions.rs`, `structure_lieux.rs` ×2) — aucune du lot S4. La suite complète d'`ent-db` n'a pas été menée d'un trait (≈ 1 h) : la CI GitHub fait foi.
