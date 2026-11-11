# Socle 13 — Discipline et suivi éducatif

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** le code des lots suivants, **après** deux générations de code Rust (P0 et S2-4) et un module Next.js partiel. Aucun code n'a été écrit ni modifié. Aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit).
**Branche :** `ent-socle-13-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas** (même jour) : `docs/socles/PERSONAS_13.md`. Ajouts faits **ici**, marqués **« ajout personas »**, regroupés au **§32** (et renvois ponctuels). **En cas de contradiction, le §32 l'emporte** ; rien n'a été supprimé.
**Frontières :** le carnet de correspondance appartient au socle **16** ; les informations préoccupantes, la cellule de veille, le psychologue et les questionnaires nominatifs R470 au socle **36** ; l'appel et l'assiduité au **12** ; l'accueil et la remise de l'élève au **14** ; les documents et la preuve de notification au **7** ; les avis au **8**.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-021, D-022, D-023, D-024, D-026, D-027, D-028, D-031, D-034, D-035, D-036, D-037 à D-041) ; `docs/SOCLES.md` §13 et §10 (T-12, T-22) ; `docs/VERIF_REGLEMENTAIRE.md` (R11, R13, R34, R35, R39, R41, R43) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (C07) ; `docs/SOCLE2_DOMAINES.md` (VS-14 à VS-18) ; conceptions `02`, `04`, `05` (R-S7), `06` (R-P6, missions), `07` (natures disciplinaires, D34, D36, D53, D79, D80, D82, DCT-149), `08` (natures `discipline.*`, R-N40), `10` (CL-64), `16` (carnet, CM-29, CM-72), `21` (R-02, CE-21, CE-49, CE-93), `34` (R-S51), `IA_COEUR.md` (§6.2, §7.2, IA-P12). Modèles de forme : `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`, `42_SORTIES_VOYAGES.md`.

**Conventions.** **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins relatifs à sa racine sur `origin/main`) ; **[V]** règle lue en entier (§5.1) ; **[V-ext]** lue en extrait ou sur une fiche officielle qui reprend une circulaire ; **[NT]** non trouvé ; **[NV]** non vérifié ; **[H]** hypothèse de terrain ; **[R]** recommandation.

| Élément | Numéros | Remarque |
|---|---|---|
| Défauts du code Rust | **DV-01**… | |
| Actions existantes et défauts de Next.js | **NX-01**…, **VD-01**… | propres à ce socle |
| Règles réglementaires | **RG-01**… | |
| Actions cibles | **DIS-100**… | DIS-01 à DIS-07 sont pris par la matrice Charlemagne ; VS-14 à VS-18 de `SOCLE2_DOMAINES.md` sont repris ici |
| Règles transverses | **R-DS1**… | |
| Cas limites | **CD-01**… | |
| Questions | **Q-DS1**…, **F-DS1**… (Florian) | |
| Réglages signalés | **S-1301 à S-1340** | plage libre vérifiée (plus haut S pris : S-466) |
| Manques (passe personas) | **M-1301 à M-1399** | plage libre vérifiée (plus haut M pris : M-2591 ; aucun M-13xx) |
| Lots | **D0 à D14** | |

**Plage de migrations : 2260–2279**, imposée par le pilote, vérifiée libre sur les 124 branches distantes le 2026-11-11 ; inscrite dans `MIGRATIONS.md`.

**Méthode.** (1) Lecture des décisions, de la carte, de la matrice, du registre des migrations et des conceptions citées. (2) **Trois recherches en parallèle** par des sous-agents : état des lieux du code Rust (environ 60 lectures) ; inventaire du module Next.js en lecture seule (environ 30 lectures) ; vérification réglementaire (environ 60 lectures, Code de l'éducation consolidé au 31-10-2026, fiches éduscol, IH2EF, instruction de tri, réponse ministérielle, jurisprudence). (3) Rédaction, puis passe personas (`PERSONAS_13.md`).

---

## 0. En bref

1. **L'existant est implémenté mais non conforme.** Deux générations de code Rust se superposent sans se parler [F] : la plus ancienne laisse **la CPE et le secrétariat prononcer et annuler des sanctions sans procédure** (DV-01) ; la procédure récente **n'impose aucun délai de défense** et accepte une convocation au conseil de discipline la veille (DV-05) ; l'effacement efface **le blâme et les exclusions trop tôt**, ne se déclenche **jamais seul** et laisse les textes dans la chronologie et les SMS (DV-02 à DV-04) ; une famille reçoit **la description libre de l'incident**, qui nomme d'autres élèves (DV-06), et un **SMS** donne la sanction et le nom de l'enfant à tous les liens, même ceux privés de la vie scolaire (DV-07). Le module Next.js (C07) est **partiel** : il confond punitions, sanctions et procédures, n'efface rien et ouvre tout à tout rôle ; on en garde les usages (carnet avec accusé de lecture, incidents d'internat à plusieurs élèves, compteurs du jour), pas le modèle.
2. **Lot D0 de correctifs urgents** (§27.1) : sanctions réservées à la direction et liées à une procédure ; table d'effacement R511-13 IV codée et tâche quotidienne ; effacement en profondeur ; délais planchers (2 jours ouvrables, 5 jours) et droit de se taire ; textes internes retirés de la vue famille ; SMS supprimé ; RLS réelle (`TenantTx`) ; alertes CPE fermées à la comptabilité ; commission « à points » gelée. **Il peut partir tout de suite.**
3. **Le réglementaire a beaucoup bougé depuis 2023** (§5, vérifié le 2026-11-11) : **droit de se taire** à notifier dès l'engagement (décret 2025-609) ; procédure **obligatoire** en cas de **harcèlement ou cyberharcèlement**, même si la victime est ailleurs ; **saisine obligatoire du conseil** si un élève porte ou introduit une **arme** ; mesure alternative **avec engagement signé**, en partie sur le temps scolaire (décret 2026-637) ; signalement « **Faits établissement** » sans délai (décret 2025-1260) ; appel au recteur non suspensif, décision **sous un mois**, classe relais après exclusion définitive (décret 2026-414) ; **dispositif interne de recueil** des violences et grilles annuelles harcèlement, privé compris (décret 2026-954) ; téléphone interdit **au lycée** (loi 2026-813). Dans le **privé sous contrat**, rien de R511/D511 ne s'impose : la discipline est **contractuelle**, le conseil y est souvent **consultatif**, le juge est **judiciaire** ; ScolIA y applique **par défaut les garanties du public** (§5.4).
4. **Le modèle** (§7, §25) : l'**incident** est un fait unique avec des **personnes et des rôles** (auteur présumé, victime, témoin) ; chaque élève a **sa** suite ; **punition** (tout personnel, retenues planifiées, exclusion de cours suivie jusqu'à l'arrivée) ≠ **procédure** (contradictoire calculé par le serveur) ≠ **sanction** (échelle fermée, sursis encadré, date d'effacement dès la décision) ; **suivi éducatif** et **commission éducative** qui ne sanctionnent pas ; **situations de harcèlement** cloisonnées avec mesures d'éloignement publiées sans motif ; **vigilance** déterministe qui alerte un adulte, jamais une sanction ni un score. Aucun JSON (D-036), toutes les exigences E1–E14.
5. **Ce que ScolIA apporte de plus [H]** : saisie en 30 secondes depuis l'appel ; une frise des délais qui empêche le vice de procédure ; des courriers prêts avec les bonnes mentions et les bons destinataires (parents séparés, adresse protégée, élève majeur) ; l'oubli organisé (effacement prouvé, rien qui ressurgit au conseil de classe ni dans l'IA) ; un professeur qui sait ce qu'est devenu son rapport ; un élève exclu de cours qui ne disparaît jamais dans un couloir ; une famille qui comprend ses droits.
6. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 136 (+ ajouts personas, §32) | DIS-100 à DIS-279 |
   | Règles réglementaires | 46 | RG-01 à RG-46 |
   | Règles transverses | 32 (+ personas) | R-DS1 à R-DS32 |
   | Cas limites | 60 (+ personas) | CD-01 à CD-60 |
   | Réglages signalés | 20 | S-1301 à S-1320 |
   | Lots | 15 | D0 à D14 |

7. **Décisions appliquées par défaut (D-031)** : 20 questions tranchées (§30.1). **Soumis à Florian** (§30.2) : l'usage réel de la discipline au lycée pilote pendant l'année de doublon ; le règlement intérieur du lycée (instance consultative ? recours interne ?) ; la reprise des sanctions depuis Charlemagne (données réelles) ; la recette juridique des trames (coût) ; l'arrêt des écrans Next.js.

---

## 1. Thèse : une sanction est la fin d'une procédure, et un incident est l'histoire de plusieurs élèves

1. **Éduquer d'abord, sanctionner en dernier.** Le domaine n'est pas une liste de punitions : c'est un **parcours gradué** — observation, dialogue, mesure de prévention, punition, commission éducative, sanction, conseil de discipline — où chaque marche est **datée, motivée et proportionnée**, et où le logiciel propose toujours la marche la plus basse qui suffit. Les écrans mettent en avant les **mesures éducatives** (entretien, engagement, fiche de suivi, réparation) au même niveau que les punitions.
2. **Un fait n'est pas une personne.** Un **incident** est un fait daté, situé, décrit **une seule fois**, auquel sont rattachées **des personnes avec un rôle** (auteur présumé, victime, témoin, signalant, impliqué). Chaque élève impliqué a **son propre fil** : ses suites (rien, entretien, punition, procédure) se décident **pour lui**, individuellement (principe d'individualisation). Aujourd'hui l'incident n'a qu'un seul élève [F] : un fait à trois élèves est saisi trois fois, ou mal.
3. **La procédure est un garde-fou, pas une case.** Le serveur **calcule** et **bloque** : délai de défense d'au moins deux jours ouvrables avant toute sanction ; convocation au conseil de discipline au moins cinq jours avant, par un mode qui vaut notification ; cas où l'engagement de la procédure est **obligatoire** ; échelle **fermée** des sanctions ; durée maximale d'une exclusion ; motivation écrite ; voies et délais de recours. Il dit **pourquoi** il bloque, et il ne laisse pas contourner un délai par un changement de date à la main sans trace.
4. **Le privé sous contrat n'est pas le public, mais il mérite les mêmes garanties.** Le Code de l'éducation encadre la procédure **dans les EPLE** ; dans le privé, c'est le **règlement intérieur** et le contrat de scolarisation qui la fixent (§5). ScolIA porte un **profil de procédure** par établissement : « EPLE » (règles du code, non modifiables) ou « privé » (règles du règlement intérieur, **préréglées par défaut sur les garanties du public** comme bonnes pratiques : contradictoire, délais, assistance, motivation, effacement), sans jamais affaiblir les droits de la défense en dessous de ce préréglage sans décision tracée de la direction.
5. **On partage le strict nécessaire.** Un parent voit **son** enfant, jamais le nom des autres élèves impliqués ni des témoins ; un professeur voit les faits **de ses cours** et les punitions qu'il a données, la **pastille** « suivi CPE en cours » ou « exclu jusqu'au … » pour ses élèves, jamais le détail d'une procédure ; la victime n'est pas citée dans la notification adressée à l'auteur ; un témoin peut demander à rester **anonyme pour les familles** (son témoignage reste au dossier, accessible aux personnes qui ont à en connaître et à la défense dans les formes du §12.4). La discipline est de la donnée **C2** ; le harcèlement et ce qui touche un mineur victime peut devenir **C3/C4** (protection de l'enfance, socle 36).
6. **L'oubli est organisé, pas subi.** Chaque sanction naît avec sa **date d'effacement** calculée par le serveur selon R511-13 IV ; l'effacement du **dossier administratif** de l'élève est fait **à la date, sans oubli**, y compris au **terme de la scolarité** dans le second degré et **à la demande** de l'élève qui change d'établissement. L'effacement n'est pas une destruction : ce qui doit être conservé pour les archives (procès-verbal du conseil de discipline, dossier d'un contentieux) l'est hors de la vue courante, selon la règle d'archives (socles 7, 27).
7. **On n'enferme pas un élève dans son passé.** Une sanction effacée **n'apparaît plus nulle part** pour les lecteurs courants (fiche, conseil de classe, bulletin, IA, statistiques nominatives) ; une alerte de vigilance s'éteint ; une appréciation qui cite une sanction effacée est signalée à son auteur (21, CE-93).
8. **La sûreté passe avant la procédure.** Un élève **exclu d'un cours** est un élève qui **marche dans le couloir** : le logiciel trace l'envoi, l'arrivée à la vie scolaire et alerte si l'élève n'est pas arrivé ; un élève **exclu temporairement** ou sous **mesure conservatoire** n'est pas « absent » à l'appel, il est attendu **ailleurs** (ou nulle part) et l'accueil le sait.
9. **L'IA aide à écrire, jamais à juger.** Elle met au propre un rapport dicté, propose une formulation neutre des faits, liste les délais et les pièces manquantes, repère une incohérence, prépare un courrier ; elle **ne propose jamais une sanction**, ne qualifie pas un élève, ne classe personne par « risque », ne lit aucune donnée de santé, et tout ce qu'elle prépare est relu et confirmé par une personne (N3 pour tout envoi et toute punition, `IA_COEUR.md` §6.2, IA-P12).

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust : deux générations qui ne se parlent pas [F]

Chemins relatifs à `ent-rust/`. Relevé en lecture seule le 2026-11-11 sur `origin/ent-mc0hoe` (sous-agent, environ 60 lectures).

| Génération | Où | Contient | Qualité |
|---|---|---|---|
| **P0** (premiers lots) | migrations `019_vs_sanctions.sql`, `054_vs_commission.sql`, `061_signal_commission.sql`, `069_alerte_cpe.sql`, `020`/`044` (carnet), `081` (éditions) ; `ent-db/src/service.rs:11618-12263` (incidents, sanctions, commission), `13349-13465` (alertes CPE), `15504-15606` (éditions), `16970-17260` (carnet) ; `ent-db/src/sanction_repo.rs` ; ancienne interface `ent-api/src/ui.rs` | types d'incidents, de sanctions, motifs ; incident (un élève) ; sanction (sans procédure) ; commission « VS » avec **permis à points** ; alertes CPE ; éditions (texte à trous) ; carnet | droits par **rôle** et non par action (D-017) ; événements **hors transaction** ; écrasement sans historique (`ON CONFLICT DO UPDATE`, `sanction_repo.rs:297, 369`) |
| **S2-4** | `133_vs_punition.sql`, `134_vs_discipline.sql` ; `ent-db/src/discipline_vs.rs` ; routes `ent-api/src/vie_scolaire.rs:44-61` ; écran Leptos `/web/discipline` (`web/crates/ent-web-app/src/modules/discipline_vs/`) | punitions **distinctes** des sanctions, catégories de l'échelle, règles d'effacement, procédure (chef / conseil), signalement d'assiduité | outbox **en transaction** (`tx.publier`), verrou `FOR UPDATE`, machine d'états ; mais **aucun accès en `TenantTx`** (23 appels `self.pool()`) |

**Tables** (toutes avec `etablissement_id NOT NULL`, UUID applicatifs, aucune `modifie_le`, **aucune FK composite**, pas de `CASCADE`) : `ent_incident_type`, `ent_sanction_type` (+ `categorie` NULL autorisé, 134:5-13), `ent_sanction_motif`, `ent_incident` (**un seul `eleve_id`, sans FK vers `ent_eleve`**, `description` illimitée, ni lieu, ni heure, ni témoins, 019:46-55), `ent_sanction` (`duree_minutes` alors qu'une exclusion se compte en jours, `note` libre, `effacee_at`, 019:60-80), `ent_regle_effacement` (durées **paramétrables de 1 à 120 mois**), `ent_procedure_disciplinaire` (`delai_convocation_jours DEFAULT 8`, valeur libre de 0 à 60, `decision TEXT` libre, ni mode d'envoi, ni preuve, ni appel, ni sursis), `ent_punition_type`, `ent_punition` (retenue sans salle, surveillant ni créneau), `ent_vs_commission`, `ent_vs_commission_dossier` (décisions `avertissement` / `exclusion_temporaire`, `points_retires`), `ent_alerte_cpe`, `ent_carnet_entree`, `ent_edition_vs`. RLS : politique du pilote 100 (`100_rls_pilote.sql:42-48`) **permissive tant que `app.etablissement_id` n'est pas posé** ; `ENT_RLS_MODE=off` par défaut (`ent-db/src/tenant_tx.rs:6-10`) ; tables reprises par 584 (punitions, procédure) et 587 (règles d'effacement).

**Droits** (`ent-auth/src/actions.rs`) : `punitions.donner` (Prof : ses cours, 1060 ; Admin, Direction, Administratif, CPE : établissement, 1131) ; `discipline.gerer` (Admin, Direction, 1137) ; `sanctions.prononcer` (**Admin, Direction, Administratif, CPE**, 1189, `roles.rs:251-256`) ; `incidents.declarer` (**Prof sur tout l'établissement**, 1069-1073, commentaire « à resserrer »).

**Événements** : `eleve.incident_declared`, `eleve.sanction_prononcee` (+ **SMS** aux parents), `eleve.sanction_annulee`, `vs.commission*`, `vs.alerte_cpe*` publiés **hors transaction** (`service.rs:11766, 11843, 11873, 12245, 13363`) ; `vs.punition_donnee`, `vs.procedure_engagee`, `vs.procedure_decidee`, `vs.conseil_discipline_decide`, `vs.sanction_effacee` **en transaction** (`discipline_vs.rs:334, 618, 688, 870`) mais **sans aucun consommateur** (ni chronologie, ni avis aux familles). Le conseil de classe compte directement `ent_sanction` non effacées (`evaluations_conseil.rs:471-556`) : correct.

### 2.2 Défauts du code Rust (DV-01 à DV-24)

| # | Gravité | Défaut | Où [F] | Correctif (lot) |
|---|---|---|---|---|
| **DV-01** | **critique** | **CPE et secrétariat prononcent et annulent des sanctions, sans aucune procédure** ; test de non-régression qui fige ce droit | `roles.rs:251-256`, `actions.rs:1189, 1460`, `service.rs:11803-11910, 12263` | **D0** : `sanctions.prononcer` réservé à la direction ; sanction R511-13 créée **uniquement** par la décision d'une procédure ; ancienne route gelée |
| **DV-02** | **critique** | **Effacement trop tôt** : blâme et mesure de responsabilisation effacés à la fin de l'année au lieu de la fin de l'année suivante ; exclusions temporaires après 12 mois au lieu de la fin de la deuxième année suivante ; exclusion définitive jamais (le terme « fin de scolarité » manque) ; test qui verrouille les règles fausses | `discipline_vs.rs:52-74`, `tests_vie_scolaire.rs:691-760` | **D0** : table R511-13 IV codée, non paramétrable |
| **DV-03** | **critique** | **Effacement jamais déclenché** (route et bouton seulement, aucune tâche planifiée) ; types sans catégorie **jamais** effacés ; durées modifiables hors de la loi | `discipline_vs.rs:517-584`, `punitions.rs:87-113` | **D0** : tâche quotidienne idempotente ; catégorie obligatoire (reprise) ; paramétrage supprimé |
| **DV-04** | **critique** | **Effacement partiel** : seule `note` est vidée ; restent la description de l'incident, les faits et la décision de la procédure, le dossier de commission, le motif de punition, l'entrée de chronologie (aucun filtre) et le **texte du SMS** en clair | `discipline_vs.rs:606-609`, `dossier_personnes.rs:954-975`, `service.rs:11873` | **D0** : effacement en profondeur, chronologie filtrée, purge du corps des SMS |
| **DV-05** | **critique** | **Pas de contradictoire** : aucun délai de défense (décision possible le jour même), délai de convocation réglable à 0 et compté en jours calendaires depuis l'**envoi**, mode d'envoi et preuve absents, aucun appel | `discipline_vs.rs:652-654, 724-840, 783` | **D0** : `informe_le`, plancher 2 jours ouvrables ; plancher 5 jours, mode et preuve ; **D3/D5** : modèle complet |
| **DV-06** | **critique** | **Fuite vers les familles** : un incident n'a qu'un élève, la `description` libre (qui nomme les autres impliqués et les témoins [H, usage]) et la `note` de sanction sont servies **telles quelles** aux parents et dans les applis mobiles famille | `019:46-55`, `dto.rs:3581-3634`, `routes.rs:4752-4775`, `apps/ios-famille/.../EntApiClient.swift:764-782`, `apps/android-famille/.../EntApiClient.kt` | **D0** : textes internes **retirés** de la vue famille (champ vide tant que D1 n'a pas créé le texte famille) ; **D1** : incident à plusieurs personnes |
| **DV-07** | **critique** | **SMS de sanction** : type, nom, prénom en clair, à **tous** les liens communicants, sans `peut_voir_vs` ni majorité, hors transaction | `service.rs:11868-11885`, `foyer_repo.rs:714-724` | **D0** : SMS supprimé, avis neutre par le socle 8 (lot N0) filtré par `peut_voir_vs` |
| DV-08 | élevée | **RLS inopérante** sur la génération S2-4, les alertes CPE, le carnet, les éditions (aucun `TenantTx`) : en mode strict ils liraient zéro ligne ; l'isolation repose sur les `WHERE` seuls | `discipline_vs.rs`, `alerte_cpe_repo.rs`, `carnet_repo.rs`, `edition_vs_repo.rs` | **D0** : passage en `TenantTx`, un test strict par table |
| DV-09 | élevée | **Comptabilité, infirmerie, psychologue lisent les alertes CPE** ; la famille reçoit `detail` et `cloture_note` | `service.rs:13448`, `alerte_cpe_repo.rs:116-123` | **D0** : action `discipline.alertes.lire` ; vue famille fermée |
| DV-10 | élevée | **Commission « permis à points »** qui « décide » avertissement ou exclusion (contraire à R511-19-1, §5), points jamais effacés ; dossiers visibles des familles sans `peut_voir_vs` | `054:38-40`, `domain-eleve/src/commission.rs:13, 238-263`, `service.rs:12175-12200` | **D0** : décisions « sanction » de la commission gelées, points gelés et masqués aux familles ; **D8** : commission éducative |
| DV-11 | élevée | **Éditions VS rendues pour n'importe quel élève par tout personnel** (comptabilité, professeur hors périmètre) | `service.rs:15542-15580` | **D0** : contrôle `autoriser_eleve` |
| DV-12 | élevée | Un professeur déclare un incident sur **tout** l'établissement | `actions.rs:1069-1073` | **D0** : portée « ses cours » + signalement à la vie scolaire hors périmètre (R-DS20) |
| DV-13 | moyenne | Procédures **invisibles** de la famille et de la CPE ; famille non informée de l'engagement | `discipline_vs.rs:916-934` | D3 |
| DV-14 | moyenne | Sanctions **annulées** encore servies aux familles | `sanction_repo.rs:448-460` | **D0** |
| DV-15 | moyenne | Événements P0 **hors transaction** ; décision de commission en deux écritures non atomiques | `service.rs:11766-12085` | D1 (refonte) ; D0 pour la sanction |
| DV-16 | moyenne | Écrasement **sans historique** des incidents et sanctions | `sanction_repo.rs:289-369` | D1 |
| DV-17 | moyenne | Pas de FK `eleve_id` ni de FK composites (type ou incident d'un autre établissement possibles en base) | 019 | D1 (nouvelles tables) |
| DV-18 | moyenne | Textes illimités, dates ni futures ni anciennes contrôlées | `domain-eleve/src/sanction.rs:184-230` | D1 |
| DV-19 | faible | Retenue confondue avec une sanction (`duree_minutes`, UI « 60 min ») | `ui.rs:40020` | D2 |
| DV-20 | faible | Aucun journal d'audit sur S2-4 ; `AggregateKind::Sanction` réutilisé pour punitions et procédures | `discipline_vs.rs:337` | D1 |
| DV-21 | faible | Aucun consommateur des événements `vs.*` | — | D1, D3 |
| DV-22 | faible | Carnet : entrée puis fil hors transaction | `service.rs:16980-17000` | socle 16 (M0a/M0b) |
| DV-23 | faible | Référentiels et alertes protégés par rôle, pas par action | `roles.rs:243, 299` | D1 |
| DV-24 | — | **Absents** : incidents à plusieurs, témoins, harcèlement, suivi éducatif, mesures conservatoires, sursis, responsabilisation détaillée, appel, courriers officiels | — | D1 à D11 |

**À réutiliser** [F] : la machine d'états et les gardes de la procédure S2-4 (`discipline_vs.rs:638-934`) ; les punitions (portée au groupe, outbox) ; la liste `CATEGORIES_SANCTION` et la fonction pure `date_effacement` (à étendre) ; le calcul des sections du dossier et la portée D-022 (`service.rs:4625`, `perimetre.rs`) ; le compteur du conseil de classe ; les contrôles du carnet (signature par titulaire, interdiction de communication) ; l'écran `/web/discipline` (structure, composants, refus 403).

### 2.3 Module Next.js « vie scolaire » (C07, volet discipline) [F, lecture seule]

Lecture en lecture seule d'une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit). Chemins relatifs à la racine Next.js.

- **Où** : `drizzle/0020_vs_sanctions.sql`, `0021_vs_carnet.sql` ; `db/schema-charlemagne-p4.ts:116-160`, `p5.ts:17-40` ; `app/lib/vs-sanctions-db.ts`, `vs-carnet-db.ts` ; API `app/api/vie-scolaire/{sanctions,carnet}`, `app/api/famille/{sanctions,carnet}` ; pages `app/(admin)/vie-scolaire/{sanctions,carnet}`, `app/quotidien/{sanctions,carnet}` ; incidents d'**internat** en JSON (`app/api/internat/incidents/route.ts`, `app/lib/internat-types.ts:293-331`, collection `internat__incidents`).
- **Maturité** : **partiel, tirant vers l'ébauche** ; classement T0 « perdable » confirmé (Charlemagne fait foi). On en reprend **les usages**, pas le modèle (D-035, précision 2).

**Actions existantes (NX-01 à NX-17)**

| # | Rôle | Action |
|---|---|---|
| NX-01 | direction, CPE, administratif, surveillant | rechercher un élève actif (nom, prénom, classe) |
| NX-02 | idem | saisir une « sanction » (élève, type, date, motif facultatif) |
| NX-03 | idem | créer en même temps une entrée de carnet « Sanction — <type> » visible de la famille (case cochée par défaut) |
| NX-04 | idem | lister les 100 dernières sanctions actives |
| NX-05 | idem | annuler une sanction (logique) |
| NX-06 | système | catalogue de 8 types créés automatiquement (`MOT_CARNET`, `AVERT`, `COLLE`, `EXCL_COURS`, `EXCL_TEMP`, `CONSEIL`, `AVERT_CD`, `BLAME`) avec gravité |
| NX-07 | + **professeur** | écrire une entrée de carnet pour n'importe quel élève |
| NX-08 | idem | lister le carnet, filtre « non signées » |
| NX-09 | parent | voir les sanctions actives de ses enfants (type, gravité, date, motif, auteur) |
| NX-10 | parent | voir le carnet et **signer** (accusé de lecture idempotent) |
| NX-11 | direction, CPE, administratif | fiche élève : 30 dernières sanctions et entrées de carnet (section masquée aux professeurs et surveillants) |
| NX-12 | rôles du module | tableau de bord : sanctions actives, du jour, carnet non signé |
| NX-13 | IA (lecture) | sanctions ou carnet d'un élève ; professeur limité à ses classes |
| NX-14 | internat | déclarer un incident touchant **plusieurs** internes, lieu, heure, témoins élèves et personnels, mesures prises ; courriel interne |
| NX-15 | internat | lister les incidents par interne ou période |
| NX-16 | internat | supprimer définitivement un incident |
| NX-17 | internat | statistiques sur 30 jours par nature (incident, remarque, sanction, **valorisation**) et par chambre |

**Défauts (VD-01 à VD-20)** — signalés pour mémoire, Next.js reste intouchable : tout rôle du module prononce n'importe quelle « sanction » (VD-01) ; un professeur lit le carnet de tout l'établissement, motifs de sanction compris (VD-02) ; rattachement parent–enfant aussi par simple égalité d'adresse e-mail [H] (VD-03) ; ni autorité parentale ni majorité (VD-04) ; aucun audit, suppression définitive des incidents d'internat (VD-05) ; établissement déduit de l'hôte [H] (VD-06) ; **aucun effacement** (VD-07) ; aucune procédure contradictoire (VD-08) ; punitions, sanctions et procédures confondues dans un catalogue (VD-09) ; motif libre montré à la famille, témoins nommés dans le courriel d'internat (VD-10) ; annuler une sanction ne retire pas l'entrée de carnet (VD-11) ; nature d'un incident d'internat forcée à « incident » (VD-12) ; réécriture complète du JSON à chaque saisie, courriel avant enregistrement (VD-13) ; dates non contrôlées (VD-14) ; suppression d'un élève qui efface son historique en cascade (VD-15) ; aucun test (VD-16) ; ergonomie : annulation sans motif, pas de pagination, carnet non modifiable, une seule signature pour tous les responsables, pas de saisie groupée (VD-17 à VD-20).

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

1. Le **carnet avec accusé de lecture** par la famille et le compteur « non signé » (→ socle 16, qui le possède ; le socle 13 s'y réfère pour l'avis de punition) — **une signature par titulaire** désormais.
2. Le **lien** punition → information de la famille, avec une annulation qui suit (corrige VD-11).
3. Le **catalogue propre à l'établissement** (code, gravité, ordre, actif), séparé en punitions et sanctions.
4. Les **sections du dossier selon le rôle** (professeurs et surveillants sans la section discipline complète).
5. L'**incident d'internat à plusieurs élèves**, avec témoins élèves et personnels, lieu, heure, mesures prises, **nature positive** (« valorisation ») : c'est le meilleur modèle d'incident existant ; il devient l'incident du socle 13 (lieu « internat »).
6. Les **compteurs du jour** de la direction et de la CPE ; le **nom de l'auteur** et la date sur chaque saisie.
7. Côté Rust : la procédure S2-4, les punitions, `date_effacement`, la portée D-022 (§2.2).

### 2.5 Verdict

On **garde et on étend** la génération S2-4 (punitions, procédure, effacement), on **remplace** la génération P0 (incident à un élève, sanction sans procédure, commission à points, alertes par rôle, SMS) par la brique `discipline` du §19, et on **corrige tout de suite** (lot D0) les sept défauts critiques, qui exposent l'établissement à des sanctions **annulables** (DV-01, DV-05), à un **oubli illégal** des effacements (DV-02 à DV-04) et à des **fuites** de données d'autres élèves vers les familles (DV-06, DV-07). Rien de Next.js n'est à reprendre en données (perdable) ; les incidents d'internat, s'ils sont utiles pour l'année en cours, sont repris par le socle 41 (Q-DS13).

---

## 3. Acteurs, besoins réels, irritants

Rôles du catalogue D-017 et missions du socle 6. **[H]** pour les irritants (pratique courante, à confirmer avec Florian et l'équipe du lycée pilote).

| Acteur | Ce qu'il fait réellement | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** | Décide les sanctions, engage les procédures, préside le conseil de discipline, signe les courriers, répond aux recours, rend compte à l'autorité (public) ou à la tutelle (privé) | Reconstituer un dossier épars (mails, mots, rapports Word) ; recompter les délais ; écrire chaque courrier à la main (D-023) | **Une sanction annulée pour vice de procédure** (délai, convocation, motivation) ; un parent qui dit « je n'ai rien reçu » ; être accusé de partialité ou de laxisme | Un **dossier de procédure** complet, une frise des délais calculés, des courriers prêts, une liste « ce qui manque avant de pouvoir décider » |
| **Adjoint(e)** | Mêmes actes par délégation ; souvent en première ligne sur le collège | Ne pas savoir ce qui a déjà été fait par un collègue | Décider sans avoir tout l'historique (récidive, sursis en cours) | L'historique non effacé, les sursis en cours, la récidive signalée |
| **CPE** | Reçoit les rapports d'incident, mène les entretiens, propose les suites, suit les élèves, anime la commission éducative, organise les retenues, fait le lien avec les familles | Ressaisir les rapports reçus par mail ; tenir le planning des retenues sur papier ; relancer les retenues non faites ; trier les « mots » des professeurs | Passer à côté d'un élève qui décroche ou d'un harcèlement ; un dossier qui fuit vers une famille ; être seul quand il est absent | Une **file de traitement** des incidents, le suivi éducatif de ses élèves au même endroit, les retenues planifiées et pointées seules, des alertes **expliquées** |
| **AED (assistant d'éducation)** | Surveille les retenues, accueille les élèves exclus de cours, saisit des incidents de cour, de cantine, d'internat | Faire l'appel de retenue sur une feuille ; ne pas savoir qui doit être là | Un élève exclu de cours qui n'arrive jamais | Sur mobile : la liste de la retenue, l'appel en trois gestes, « exclu de cours en route : Lucas, envoyé à 10 h 12 » |
| **Professeur** | Constate, donne des punitions, exclut ponctuellement d'un cours, rédige un rapport, demande une sanction, participe au conseil de discipline (élu) | Rédiger un rapport long ; ne pas savoir ce qu'il est advenu de son rapport | Être désavoué ; voir le nom de l'élève qui a témoigné circuler ; être pris à partie par une famille | Saisie en **30 secondes** depuis l'appel (téléphone) ; un retour « votre rapport a été traité : entretien CPE le … » ; ne voir que ses élèves |
| **Professeur principal** | Suit sa classe, est informé des punitions et sanctions de ses élèves, prépare le conseil de classe | Découvrir au conseil qu'un élève a été exclu trois jours | Arriver au conseil sans le contexte | Synthèse vie scolaire **par élève** pour le conseil (sans détail inutile, sans sanction effacée) |
| **AESH** | Accompagne un élève handicapé, présent lors d'incidents | N'être ni témoin officiel ni informé | Que l'élève accompagné soit sanctionné sans prise en compte de son handicap | Pouvoir signaler, être entendue, que le PPS soit **signalé** (pastille, sans contenu) à la personne qui décide |
| **Secrétariat** | Met en forme et envoie les courriers, les recommandés, tient le registre, prépare le conseil (convocations des membres) | Courriers Word, enveloppes, accusés de réception à classer | Un recommandé parti trop tard, une adresse protégée sur un courrier | Courriers générés, **bordereau de recommandés**, suivi des accusés, adresse due choisie par le socle 4 |
| **Accueil** | Reçoit les familles convoquées, remet des courriers en main propre contre signature, voit passer l'élève exclu | Ne pas savoir qu'un élève est exclu ou sous mesure conservatoire | Laisser entrer un élève interdit d'accès ; remettre un courrier à la mauvaise personne | « Exclu jusqu'au 14 : accès interdit » à l'écran de l'accueil ; remise contre signature sur tablette (7, D34) |
| **Gestionnaire, intendance** | Applique une exclusion de la demi-pension ou de l'internat ; constate des dégradations | Apprendre l'exclusion par le bouche-à-oreille | Facturer un repas à un élève exclu, ou ne pas savoir qu'il ne doit pas être servi | Effets automatiques vers la cantine et l'internat (40, 41) |
| **Infirmière, psychologue, assistant(e) social(e)** | Reçoivent l'élève blessé, l'élève victime, l'élève en souffrance ; participent à la commission éducative, à la cellule de veille | Être sollicités sans contexte ; voir leur information confidentielle recopiée dans un dossier disciplinaire | Que le secret soit rompu | Une **référence** croisée sans contenu (« suivi infirmier le jour de l'incident : oui ») ; jamais de donnée de santé au dossier disciplinaire |
| **Référent harcèlement, équipe pHARe** | Reçoivent les signalements, mènent la méthode de la préoccupation partagée, suivent les situations | Signalements dispersés (oral, mail, messagerie, 3018) | Exposer l'élève qui signale ; perdre la preuve ; une situation « oubliée » | Une **situation** suivie, datée, cloisonnée, avec rappels de points d'étape, et les mesures de protection publiées sans motif (5, 10) |
| **Membres du conseil de discipline** (personnels, parents, élèves élus) | Siègent, lisent le dossier, votent | Recevoir le dossier en papier ou par mail | Être exposés à des informations inutiles ; un dossier qui traîne sur un téléphone | Accès **temporaire en lecture seule**, filigrané, clos après la séance (7, D82) ; déport proposé en cas de lien personnel (6, R-P6) |
| **Familles (titulaires)** | Reçoivent les avis, sont convoquées, présentent leurs observations, consultent le dossier, se font assister, font appel | Apprendre une sanction par leur enfant ; courrier incompréhensible | Une décision arbitraire ; ne pas pouvoir se défendre ; que l'autre parent (séparé) ne soit pas informé, ou au contraire qu'il le soit malgré une décision du juge | Un avis clair, les **droits et délais écrits en langage simple**, la possibilité de répondre en ligne, un rendez-vous proposé |
| **Parent sans exercice de l'autorité parentale** | Doit être **informé** des choix importants (C. civ. 373-2-1 [V], VERIF R11) | Ne rien recevoir | Être écarté | Recevoir l'information due, sans pouvoir agir à la place des titulaires |
| **Élève mineur auteur présumé** | Est entendu, présente sa défense, exécute la punition ou la sanction | Ne pas comprendre ce qu'on lui reproche ni ce qui va se passer | L'injustice ; l'humiliation devant les autres ; être « fiché » | Savoir où il en est, ce qu'il doit faire, **quand ce sera effacé** ; un engagement qui compte |
| **Élève victime** | Signale, est protégé, est informé des suites **le concernant** | Répéter son récit cinq fois | Les représailles ; que l'auteur apprenne qui a parlé | Un seul récit recueilli, des mesures de protection visibles par les adultes concernés, un suivi |
| **Élève témoin** | Témoigne | Être identifié par les familles | Représailles | Anonymat vis-à-vis des familles et des autres élèves (§12.4) |
| **Élève majeur** | Reçoit lui-même les notifications, peut se défendre seul ; ses parents ne sont informés que dans les conditions du §5 | Que ses parents reçoivent tout sans qu'il le sache | — | Clarté sur qui est informé |
| **Personne qui assiste** (un parent, un délégué, un avocat, une personne de son choix) | Assiste l'élève lors de la procédure, reçoit la convocation | Ne pas être convoquée à temps | — | Convocation dans les délais, accès au dossier |
| **Autorité académique** (public : DASEN, recteur) | Statue sur l'appel ; réaffecte l'élève exclu définitivement soumis à l'obligation scolaire ; reçoit les signalements de faits graves | Dossiers incomplets | — | Dossier d'appel complet exporté proprement |
| **Tutelle, DDEC, organisme de gestion** (privé) | Est informée des exclusions définitives ; peut proposer une médiation [H] | — | Contentieux | Dossier clair |
| **Délégué à la protection des données** | Vérifie la base légale, la minimisation, les durées, les droits d'accès | Données disciplinaires partout, textes libres sans contrôle | Une sanction effacée qui survit dans un export, une IA nourrie de dossiers disciplinaires | Registre clair, effacement prouvé, journal des accès |

**Ce que montrent ces portraits [R].** (1) Le temps perdu est d'abord de la **ressaisie** (rapport reçu par mail, retenue sur papier, courrier Word) : la saisie unique depuis le lieu du fait est le premier gain. (2) La peur dominante des décideurs est **juridique** (vice de forme) : le logiciel doit porter la procédure. (3) La peur dominante des élèves et des familles est **l'arbitraire et l'exposition** : transparence sur *son* dossier, opacité sur celui des autres. (4) Les professeurs veulent un **retour** : un rapport sans suite visible décourage le signalement.

---

## 4. L'année de la discipline vue du terrain [H, à confirmer avec Florian]

| Moment | Ce qui se passe | Ce que ScolIA doit rendre facile |
|---|---|---|
| Juin – juillet | Règlement intérieur revu (CA dans le public, direction et tutelle dans le privé) ; échelle des punitions, mesures de prévention, composition de la commission éducative | Catalogue des punitions et mesures, profil de procédure, composition des instances **à la date** ; publication du règlement aux familles (7, 18) |
| Rentrée | Règlement signé par les familles ; élections au CA (public) dont découlent les membres élus du conseil de discipline | Prise de connaissance du règlement (7, S0) ; membres élus du conseil (46) |
| Septembre – octobre | Premiers incidents, oublis répétés, retenues du mercredi | Retenues planifiées, pointées ; observations ; premiers entretiens CPE |
| Toute l'année | Incidents de cour, de cantine, d'internat, de sortie ; signalements de harcèlement (campagne pHARe, questionnaires) ; téléphones confisqués | Saisie rapide, file de traitement, situations de harcèlement suivies, remise des téléphones |
| Avant chaque conseil de classe | Synthèse vie scolaire par élève | Synthèse **non effacée, proportionnée** (23) |
| Ponctuellement | Procédure contradictoire, sanction, parfois conseil de discipline (quelques-uns par an dans un lycée [H]) | Dossier de procédure guidé, délais, convocations, séance, PV, notification, appel |
| Fin d'année scolaire | Effacement des avertissements ; bilan ; préparation de la constitution des classes (séparer des élèves) | Effacement automatique et prouvé ; mesures d'éloignement transmises sans motif à la constitution (5, R-S7) |
| Fin de l'année suivante, de la deuxième année suivante | Effacement des blâmes, mesures de responsabilisation, exclusions | Idem, sans oubli, même si l'élève a changé de classe ou de cycle |
| Départ de l'élève | Effacement à la demande en cas de changement d'établissement ; effacement de tout au terme de la scolarité du second degré | Demande enregistrée, effacement fait, certificat sans mention disciplinaire |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

### 5.1 Sources et statut

- **Code de l'éducation consolidé**, fichier de données ouvertes de la DILA republié par codes.droit.org (`lastup="2026-10-31"`) : https://codes.droit.org/payloads/Code%20de%20l%27%C3%A9ducation.xml ; même méthode pour le Code pénal (`lastup` 2026-08-26) et le Code de procédure pénale. Lu en entier, article par article : **[V]**. Ce n'est pas Légifrance : **un juriste relit l'article sur Légifrance avant de coder une règle qui en dépend** (même réserve que `VERIF_REGLEMENTAIRE.md` §1).
- **Fiches éduscol « Ressource sur les sanctions disciplinaires » n° 1 à 15** (https://eduscol.education.fr/document/31039/download et suivantes, lues en entier) ; **IH2EF**, fiche « Procédures disciplinaires » (mise à jour du 04-12-2024) et page « Lutte contre le harcèlement scolaire » (https://www.ih2ef.gouv.fr/lutte-contre-le-harcelement-scolaire) ; elles **reprennent** des circulaires que le BO (403) n'a pas laissé lire : **[V-ext]**.
- **Instruction de tri 2005-003**, annexe du BO n° 24 du 16-06-2005 (https://www.education.gouv.fr/bo/BoAnnexes/2005/24/tableaux_encart24.pdf) : [V].
- Privé : réponse ministérielle AN QE n° 86829, JO du 05-04-2016 (https://questions.assemblee-nationale.fr/dyn/14/questions/QANR5L14QE86829.pdf) [V] ; TA Melun n° 1500164 (2015) [V] ; document APEL 2012 « La discipline dans l'enseignement catholique » (https://www.apel.fr/sites/default/files/2022-07/2012_La%20discipline_EC.pdf), qui cite le texte du CNEC du 17-03-2007 [V-ext].
- Décret 2026-637 (détail des modifications) : copie du texte sur https://kohenavocats.fr/?p=2045228 [V-ext], cohérente avec le texte consolidé.
- Recherche menée par un sous-agent (environ 60 lectures) ; Légifrance, education.gouv.fr/bo et plusieurs sites académiques ont renvoyé 403.
- **Textes récents** (postérieurs à la conception d'origine de la carte) relevés dans le texte consolidé : décrets **2025-609** (droit de se taire, port d'arme), **2025-1260** (signalement « Faits établissement »), **2026-414** (classe relais, commission d'appel, délai du recteur), **2026-637** (mesure alternative), **2026-687** (comportement d'un parent), **2026-954** (dispositif interne de recueil, questionnaires) ; loi **2026-813** (téléphone au lycée).

### 5.2 Ce qui commande le logiciel (EPLE)

| # | Règle | Source | Marque | Conséquence dans ScolIA |
|---|---|---|---|---|
| RG-01 | **Échelle fermée** : avertissement ; blâme ; mesure de responsabilisation ; exclusion temporaire **de la classe** (élève accueilli dans l'établissement) ≤ 8 jours ; exclusion temporaire **de l'établissement ou d'un service annexe** ≤ 8 jours ; exclusion définitive de l'établissement ou d'un service annexe. Le règlement intérieur « ne peut que la reproduire telle quelle » | C. éduc. **R511-13 I** (version du décret 2026-637, en vigueur le 18-07-2026) ; éduscol fiche 1 | [V] ; [V-ext] | catégories fixes, aucune sanction libre (R-DS4) ; durée ≤ 8 jours contrôlée (R-DS9) ; corrige DV-01 et l'ancienne catégorie `autre` |
| RG-02 | **Sursis** possible pour les sanctions 3° à 6°, **pas** pour l'avertissement ni le blâme ; durée de révocabilité fixée par l'autorité : au moins l'année en cours, au plus la durée d'inscription au dossier (exclusion définitive avec sursis : au plus la fin de la 2ᵉ année suivante) ; l'élève (et son représentant s'il est mineur) est averti des conséquences d'un nouveau manquement ; révocation **seulement** pour des faits passibles d'une sanction de niveau **égal ou supérieur**, seule ou avec une nouvelle sanction ; seul le conseil révoque le sursis d'une exclusion définitive ; **exécution cumulée plafonnée à 8 jours** d'exclusion | **R511-13-1** (décret 2019-906) | [V] | §8.7, R-DS13 ; contrôles du sursis (CD-18, CD-19) |
| RG-03 | **Mesure de responsabilisation** : activités de solidarité, culturelles ou de formation, hors des heures d'enseignement, **20 heures au plus** ; dignité, sans danger, adaptée à l'âge ; dans un établissement d'enseignement scolaire, une association, une collectivité, un groupement public ou une administration de l'État ; **convention** obligatoire avec la structure (clauses types : arrêté du 30-11-2011 [V-ext]) ; **accord** de l'élève ou de ses représentants s'il est mineur **en cas d'exécution en dehors du temps scolaire** ; un exemplaire de la convention remis | **R511-13 II** (décret 2026-637) | [V] | §10 ; plafond 20 h ; accord recueilli quand l'exécution est hors temps scolaire |
| RG-04 | **Mesure alternative** à une exclusion temporaire (4° ou 5°) : mesure de responsabilisation proposée par le chef ou le conseil, **qui peut avoir lieu en partie pendant les heures d'enseignement** (nouveau, faits commis à partir de la rentrée 2026), subordonnée à **la signature d'un engagement par l'élève** ; respectée : seule la mesure alternative est inscrite au dossier, effacée à la fin de l'année scolaire suivante ; non respectée : la sanction initiale est exécutée et inscrite | **R511-13 III** (décret 2026-637) | [V] | §10 ; engagement signé (document 7) ; double date d'effacement selon l'issue |
| RG-05 | **Inscription au dossier administratif et effacement** : sanctions inscrites, même avec sursis ; avertissement effacé à la fin de l'année scolaire ; blâme et responsabilisation à la fin de l'année scolaire suivante ; autres (sauf exclusion définitive) à la fin de la 2ᵉ année scolaire suivante ; **toutes** au terme de la scolarité dans le second degré ; effacement **sur demande** de l'élève lorsqu'il change d'établissement | **R511-13 IV** | [V] (déjà VERIF R34) | §9.3, DIS-220 à DIS-223 ; corrige DV-02 à DV-04 |
| RG-06 | **Portée de l'effacement** : la sanction **et les pièces de procédure** (convocation, compte rendu, courriers) sont **retirées et archivées, pas détruites** ; les documents relatifs **aux faits** (rapports, témoignages) peuvent être conservés et pris en compte pour des fautes ultérieures ; une personne qui a connu une sanction effacée ne peut pas en faire état ; la demande d'effacement au changement d'établissement **ne vise pas l'exclusion définitive**, peut être refusée, et le **silence de deux mois vaut acceptation** | éduscol fiches 11 et 12 | [V-ext] | R-DS25 précisée : retrait de la vue courante + archive à accès restreint ; incidents conservés selon leur propre durée ; échéance de 2 mois sur la demande (DIS-223) ; à confirmer par un juriste (Q-DS3) |
| RG-07 | **Mesure éducative d'abord** : hors des cas où l'engagement est obligatoire et avant la procédure, le chef et l'équipe éducative « recherchent, dans la mesure du possible, toute mesure utile de nature éducative » | **R511-12** | [V] | parcours gradué (§8) ; la direction voit les mesures déjà tentées |
| RG-08 | **Droit de se taire** : quand le chef d'établissement engage une procédure, il informe l'élève qu'il dispose du droit de garder le silence pour l'ensemble de la procédure (étendu au conseil de discipline par la fiche éduscol 7) | **R511-12-1** (décret 2025-609) ; QPC du 26-06-2024 citée par l'IH2EF | [V] ; [V-ext] | **mention obligatoire** dans l'information préalable et la convocation (§16) ; le compte rendu d'audition le rappelle ; contrôle bloquant de la trame |
| RG-09 | **Qui prononce** : le chef d'établissement **seul** pour les sanctions 1° à 5° ; l'**exclusion définitive** relève du conseil de discipline ; atteinte aux principes de la République (laïcité) : le DASEN peut, à la demande motivée du chef, engager la procédure et prononcer seul les sanctions 1° à 5° | **R511-14** (décret 2023-782) ; R511-27 | [V] | R-DS9 ; la CPE ne sanctionne pas (aucun texte, [NT]) : corrige DV-01 |
| RG-10 | **Délégation** : signature déléguée aux adjoints ; suppléance par l'adjoint, présidence des instances comprise ; sanctions 1° à 5° prononcées par le chef « ou son adjoint par délégation » | **R421-13 III** ; R421-9 3° ; IH2EF | [V] ; [V-ext] | `sanctions.prononcer` : chef et adjoint (délégation datée, socle 6) |
| RG-11 | **Engagement obligatoire** de la procédure : a) violence verbale envers un membre du personnel ; b) acte grave envers un membre du personnel ou un autre élève ; c) atteinte grave aux principes de la République, notamment la laïcité ; d) **harcèlement, notamment cyberharcèlement, envers un autre élève, y compris scolarisé dans un autre établissement** | **R421-10** (décret 2025-609, version du 03-07-2025) | [V] | R-DS11 : tâche bloquante ; **aucun renoncement possible** dans ces cas (DIS-162 réservé aux autres cas) ; harcèlement : une situation confirmée déclenche l'engagement contre chaque auteur |
| RG-12 | **Saisine obligatoire du conseil de discipline** : un membre du personnel victime de **violence physique** ; un élève qui **introduit une arme** dans l'établissement ou **porte une arme** sur lui | **R421-10** | [V] | blocage : la décision du chef seul est refusée, seule la saisine est possible (R-DS11) |
| RG-13 | **Contradictoire devant le chef seul** : l'élève est informé **sans délai** des faits reprochés et du délai pour présenter sa défense **oralement, par écrit ou en se faisant assister** par une personne de son choix ; délai fixé par le chef, **au moins deux jours ouvrables** ; même communication au représentant légal d'un **mineur** ; consultation du dossier par l'élève, son représentant et la personne qui l'assiste | **R421-10-1** (décret 2019-906) | [V] | §8.6, R-DS10 ; corrige DV-05 |
| RG-14 | **Jours ouvrables** : le samedi est ouvrable ; le délai **court pendant les vacances** | éduscol fiche 7 | [V-ext] | moteur de dates (§9) : lundi–samedi hors fériés ; **avertissement** (pas blocage) si le délai court pendant des vacances : la direction peut fixer un délai plus long |
| RG-15 | **Mesure conservatoire** : (chef seul) interdiction d'accès pendant le délai de défense ; (avant le conseil) interdiction d'accès en attendant la comparution, l'élève mineur **remis à son représentant légal** ; elle n'est pas une sanction, suppose une procédure engagée et doit être notifiée ; prolongeable pendant le délai d'appel ou jusqu'à la décision du recteur | **R421-10-1**, **D511-33**, **D511-50** ; éduscol 13 | [V] ; [V-ext] | DIS-167 : impossible sans procédure engagée ; durée bornée (fin du délai de défense, date de la séance, fin de l'appel) ; remise au représentant (14) |
| RG-16 | **Refus de saisine** : si un membre de la communauté éducative demande **par écrit** la saisine du conseil et que le chef refuse, il notifie une décision motivée ; le chef informe le DASEN avant de saisir le conseil si l'élève a déjà été exclu définitivement dans l'année ; la décision d'engager ou non n'est pas attaquable en annulation | **D511-30** ; éduscol 6 | [V] ; [V-ext] | DIS-160 : demande écrite tracée, réponse motivée obligatoire ; bilan au CA (RG-27) |
| RG-17 | **Signalement « Faits établissement »** : le chef informe **sans délai** l'autorité académique des violences subies par élèves ou personnels, de tout fait grave mettant en danger leur sécurité ou leur intégrité, de toute atteinte aux valeurs de la République, par le système d'information du ministère ; extension au privé sous contrat annoncée | **R421-10-2** (décret 2025-1260) ; guide ministériel 2024 | [V] ; [V-ext] | DIS-135 : **rappel** « signalement Faits établissement à faire » avec les éléments à reporter ; ScolIA **n'envoie rien** (pas d'interface) ; trace « signalé le … par … » (Q-DS12) |
| RG-18 | **Conseil de discipline** : 14 membres (chef président, adjoint, CPE désigné par le CA, gestionnaire, 5 personnels dont 4 enseignement et éducation et 1 ATSS, parents 3 en collège / 2 en lycée, élèves 2 en collège / 3 en lycée) ; élus chaque année au sein du CA, élèves de lycée par l'assemblée des délégués, un suppléant par élu ; expert à voix consultative possible (laïcité), présidence confiée au DASEN sur demande | **R511-20**, **R511-20-1**, **R511-21**, **R511-22** | [V] | §12.1 : composition contrôlée ; membres du socle 46 (ou saisie locale) |
| RG-19 | **Compétence et lieu** : conseil de l'établissement d'inscription quel que soit le lieu de la faute ; délocalisation dans un autre établissement ou à la DSDEN ; partage des voix : voix prépondérante du président | **D511-25**, **R511-27** | [V] | DIS-205 ; résultat de vote avec voix prépondérante |
| RG-20 | **Convocations** : élève, représentant légal (mineur), personne qui l'assiste **par pli recommandé ou remise en main propre contre signature**, au moins **5 jours** avant la séance ; membres, personne ayant demandé la comparution, témoins (et leur représentant s'ils sont mineurs) **par tout moyen, courriel compris**, au moins 5 jours avant | **D511-31** (décret 2019-908) | [V] (VERIF R35) | §12.2 ; l'ouverture ENT ne vaut pas convocation ; corrige DV-05 |
| RG-21 | **Faits et droits communiqués**, dossier consultable par les membres, l'élève, son représentant et la personne qui l'assiste ; droit d'être entendus pour le représentant et la personne qui assiste | **D511-32** | [V] | DIS-166, DIS-204 |
| RG-22 | **Incompatibilités** : un parent dont l'enfant comparaît est remplacé par son suppléant ; un élève sous procédure ne siège pas jusqu'à la décision définitive ; un élève exclu temporairement ne siège plus jusqu'à la fin de l'année ; le membre qui a demandé la comparution est remplacé | **D511-34** | [V] | DIS-201 : déport **automatique** proposé (pas seulement R-P6 du 06) |
| RG-23 | **Quorum** : majorité des membres ; à défaut, nouvelle réunion sous 5 à 10 jours, qui délibère quel que soit le nombre de présents (délai réductible en cas d'urgence) | **D511-35** | [V] | DIS-210 : report avec fenêtre 5–10 jours calculée |
| RG-24 | **Déroulé** : secrétaire de séance désigné parmi les membres ; délégués mineurs se retirent si les deux tiers des membres le demandent ; lecture du rapport motivant la proposition ; auditions (élève, représentant et personne qui assiste à leur demande, deux professeurs de la classe, deux délégués de la classe, toute personne utile, témoins) ; débats contradictoires « avec le souci de donner … une portée éducative » | **D511-36 à D511-40** | [V] | mode séance guidé (§12.5) |
| RG-25 | **Vote** : seuls les membres à voix délibérative présents ; **bulletins secrets**, majorité des suffrages exprimés (abstentions, blancs, nuls non comptés) ; **secret** pour toutes les personnes présentes | **D511-41** | [V] | R-DS24 : comptes seulement ; accès membres clos après la séance |
| RG-26 | **Notification et procès-verbal** : décision notifiée **aussitôt** à l'élève et à son représentant, **confirmée par recommandé le jour même** avec les voies et délais d'appel ; PV (présents, griefs, réponses de l'élève, observations de la personne qui assiste, décision) signé du président et du secrétaire, aux archives, **copie au recteur sous 5 jours** | **D511-42** (décret 2019-1558) | [V] | DIS-208, DIS-209 : recommandé du jour même au bordereau ; tâche « copie du PV au recteur » à J+5 |
| RG-27 | **Registre des sanctions** et bilan annuel au CA (décisions disciplinaires, suites données aux demandes écrites de saisine) ; registre **anonymisé** (faits et sanction) | **R421-20 3°** ; IH2EF | [V] ; [V-ext] | DIS-264 : registre généré (sans nom) ; bilan annuel préparé |
| RG-28 | **Exclusion définitive d'un élève soumis à l'obligation scolaire** : le recteur ou le DASEN, informé immédiatement, l'inscrit dans un autre établissement ou, à titre transitoire et pour une année au plus, dans une **classe relais** | **D511-43** (décret 2026-414) | [V] | DIS-191 : tâche « informer l'autorité » le jour de la décision |
| RG-29 | **Poursuites pénales** sur les mêmes faits avec contestation sérieuse de leur matérialité : l'action disciplinaire peut être suspendue jusqu'au jugement ; jonction possible de faits distincts et d'une infraction à la mesure conservatoire | **D511-47**, **D511-48** | [V] | état `suspendue` de la procédure ; jonction (une décision pour plusieurs incidents d'un même élève) |
| RG-30 | **Conseil de discipline départemental** (DASEN président, 10 membres) : saisine possible pour atteinte grave par un élève déjà exclu définitivement ou poursuivi pénalement, si la sérénité ou la sécurité est compromise, ou pour atteinte grave aux principes de la République | **R511-44**, **R511-45**, **D511-46** | [V] | DIS-205 : saisine tracée, dossier exporté |
| RG-31 | **Appel** d'une décision du conseil au **recteur dans les 8 jours** de sa notification écrite, par le représentant légal, l'élève majeur ou le chef ; **non suspensif** (« immédiatement exécutoire ») ; commission académique (recteur + 3 membres) ; convocation de l'élève et de son représentant 5 jours avant ; **décision du recteur dans le mois** ; recours préalable obligatoire avant le juge administratif | **R511-49**, **D511-50**, **D511-51**, **D511-52** (décret 2026-414), **R511-53** | [V] | DIS-187 à DIS-189 : échéances calculées ; effets **non suspendus** (l'exclusion s'exécute) ; départ (3) ouvert à la date d'effet ; décision d'appel sous un mois suivie |
| RG-32 | **Sanction du chef seul** : décision **écrite et motivée** (faits et droit), voies et délais de recours ; recours gracieux ou hiérarchique, puis tribunal administratif dans les **2 mois** ; reste exécutoire | éduscol 5 ; IH2EF | [V-ext] | §8.7 : trame de notification ; échéance « recours possible jusqu'au » |
| RG-33 | **Commission éducative** : présidée par le chef ou son représentant ; au moins un professeur et au moins un parent ; composition arrêtée par le CA et inscrite au règlement intérieur ; examine la situation d'un élève, recherche une réponse éducative personnalisée, consultée en cas d'incidents impliquant **plusieurs élèves** ; **assure le suivi** des mesures de prévention, d'accompagnement, de responsabilisation et des mesures alternatives ; réunie aussi pour l'absentéisme ; **ne prononce aucune sanction** | **R511-19-1** ; R131-7 ; éduscol 10 | [V] ; [V-ext] | §11, R-DS17 ; corrige DV-10 |
| RG-34 | **Punitions scolaires** : données par les professeurs, personnels de direction, d'éducation ou de surveillance (le chef sur proposition d'un ATSS) ; liste indicative : rapport sur le carnet, excuse orale ou écrite, devoir supplémentaire, retenue ; **exclusion ponctuelle d'un cours « dans des cas très exceptionnels »**, avec prise en charge de l'élève et un travail demandé ; **toute punition fait l'objet d'une information écrite des parents** ; **zéro pour motif exclusivement disciplinaire proscrit** ; mesures d'ordre intérieur (recours auprès du chef), **non inscrites au dossier administratif** ; **punitions collectives interdites** (une même punition pour plusieurs élèves reste possible si elle est individualisée) | circulaire 2014-059 du 27-05-2014 (BO n° 22), reprise par éduscol fiche 1 et IH2EF | [V-ext] | §8.3–8.5, R-DS2 ; information écrite = avis (8) ; « lignes » : aucun texte trouvé [NT], laissé au règlement intérieur, non proposé par défaut |
| RG-35 | **Règlement intérieur** adopté par le CA : chapitre discipline, reproduit l'échelle, prévoit les mesures de prévention, de responsabilisation et d'accompagnement (notamment après réintégration d'un élève exclu pour violence) ; plan de prévention de la violence avec un programme contre le harcèlement | **R421-5**, **R421-20 5° et 12°** | [V] | DIS-109 ; mesure d'accompagnement au retour d'exclusion proposée (CD-22) |
| RG-36 | **Principes** : légalité, contradictoire et droits de la défense (droit de se taire compris), individualisation, proportionnalité (« pas de barème »), non bis in idem (les antécédents peuvent être pris en compte pour la gravité), motivation, discrétion ; faute **hors établissement** sanctionnable si liée à la qualité d'élève, réseaux sociaux compris | IH2EF ; éduscol 1, 7, 8 | [V-ext] | R-DS1, R-DS3, R-DS8, R-DS14 (« pas de barème » confirme : pas de permis à points qui décide) ; lieu « en ligne » (DIS-122) |
| RG-37 | **Harcèlement** : « Aucun élève … ne doit subir de faits de harcèlement » (ancien L511-3-1, **devenu L111-6**) ; public **et privé** : prévention, détection, réponse rapide et coordonnée, orientation des victimes, témoins et auteurs, information annuelle ; projet d'établissement : procédures de traitement (L543-1) ; délit de harcèlement scolaire (3 à 10 ans) | **L111-6**, **L543-1**, C. pén. **222-33-2-3** | [V] | §14 ; R-DS23 |
| RG-38 | **Dispositif interne de recueil** des atteintes à l'intégrité physique ou morale des élèves et des personnels (violence, harcèlement, agissements sexistes, menaces…), public et privé, avec un dispositif spécifique pour l'internat et les voyages avec nuitées ; **questionnaire annuel** sur les violences sexistes et sexuelles (grande section → terminale) et **grille d'auto-évaluation** harcèlement (CE2 → terminale), nom facultatif, réponses nominatives examinées par des **personnels de santé ou sociaux formés**, entretien si inquiétant, parents informés et pouvant s'opposer ; responsable de traitement : le ministre (public), **le chef d'établissement (privé sous contrat)** | **R470-1 à R470-4** (décret 2026-954 du 14-10-2026) | [V] | §14 : le **recueil** est au socle 13 (DIS-250) ; les **questionnaires nominatifs** relèvent du socle 36 (personnels sociaux et de santé, C3), le 13 ne reçoit qu'une situation ouverte par eux ; à concevoir avec le DPO (Q-DS15) |
| RG-39 | **Protocole pHARe** dans chaque école, collège et lycée (circulaire du 2 février 2024, BO n° 6 du 08-02-2024, qui abroge 2013-100) : formation, prévention, protocole national de détection et de prise en charge | IH2EF | [V-ext] | étapes préréglées dans le public ; le privé choisit son protocole (préréglage pHARe proposé) |
| RG-40 | **Téléphone** : utilisation interdite dans les écoles, collèges **et lycées** et pendant les activités d'enseignement hors de l'enceinte ; modalités et exceptions par le règlement intérieur ; élèves handicapés ou atteints d'un trouble de santé : équipements autorisés non concernés ; **confiscation** par un membre de la direction, de l'enseignement, de l'éducation ou de la surveillance, modalités de restitution par le règlement intérieur | **L511-5** (loi 2026-813) | [V] | DIS-136 ; usage médical jamais sanctionné (34 R-S51) ; application au privé sous contrat : Q-DS16 |
| RG-41 | **Comportement d'un parent** menaçant un membre de la communauté éducative : le DASEN peut affecter l'élève dans un autre établissement (second degré), après dialogue ; suspension conservatoire de l'accès de 5 jours au plus | **R421-12-1** (décret 2026-687) | [V] | hors échelle des sanctions de l'élève : incident avec un **adulte auteur** (rôle `auteur_presume` d'une personne non élève), aucune sanction de l'élève (CD-34) |
| RG-42 | **Signalement au procureur** : tout fonctionnaire qui acquiert la connaissance d'un crime ou d'un délit en donne avis sans délai ; obligation pour **quiconque** d'informer des mauvais traitements ou atteintes sexuelles sur un mineur | C. proc. pén. **40** ; C. pén. **434-3** | [V] | DIS-256 : trace de l'avis, pièce hors dossier disciplinaire (CD-36) |
| RG-43 | **Absentéisme** : réponse éducative et administrative (commission éducative, engagement, avertissement du DASEN, procureur) ; le manquement à l'assiduité **peut** fonder une procédure disciplinaire (L511-1, R421-5) ; aucun texte trouvé qui interdise d'exclure pour absentéisme [NT] | **L131-8**, **R131-7**, **L511-1** | [V] | R-DS18 corrigée : **pas d'exclusion automatique** ; une procédure pour absentéisme est possible mais signalée « l'exclusion aggrave l'absence » [R] |
| RG-44 | **Élève majeur** : la communication au représentant légal n'est prévue que si l'élève est mineur ; appel par l'élève majeur ; accord de responsabilisation par lui ; notification à lui ; **aucun texte** sur l'information de ses parents [NT] | R421-10-1, D511-31, D511-32, R511-49, R511-13 II ; éduscol 5 | [V] ; [NT] | §15 : parents d'un majeur **non** destinataires par défaut [R, Q-DS8] |
| RG-45 | **Suivi municipal de l'obligation scolaire** : le traitement de la commune enregistre la mention, la date et la durée d'une **exclusion temporaire ou définitive** | **R131-10-2 6°** | [V] | export vers la mairie (élèves soumis à l'obligation scolaire) à prévoir avec les socles 3 et 9 [NV : circuit, Q-DS17] |
| RG-46 | **Conservation** (EPLE) : conseils de discipline (PV, correspondance, notifications) **10 ans, tri, ne conserver que le PV** ; commissions d'appel 10 ans, tri ; **cahiers de punitions 1 an, destruction** ; **registres des sanctions 10 ans, conservation** ; dossiers scolaires (y compris éléments sur les sanctions) 10 ou 50 ans, tri | instruction de tri 2005-003 | [V] (VERIF R41) ; remplacement éventuel [NT] | §21.3 corrigée |

### 5.3 Premier degré (en marge)
Pas de sanctions de l'échelle (R511-13 vise les collèges et lycées) [V] ; réprimandes portées à la connaissance des représentants, jamais d'atteinte à l'intégrité, l'élève jamais laissé seul, équipe éducative en cas de trouble grave (circulaire 2014-088, règlement type [V-ext]) ; **harcèlement** : mesures éducatives, suspension conservatoire de 5 jours au plus, radiation et inscription dans une autre école sur demande du DASEN au maire (**R411-11-1**) [V] ; comportement d'un parent (**R411-11-1-1**) [V]. ScolIA : profil `premier_degre` au socle 50 (observations, réprimandes, équipe éducative, situations de harcèlement), **sans** procédure ni échelle.

### 5.4 Privé sous contrat (l'établissement pilote)
- **Ce qui ne s'applique pas** : aucun article n'étend R511-12 à R511-53 ni D511-* au privé ; ils visent « les collèges et lycées relevant du ministre » et les EPLE [V, lecture du champ]. Les mesures disciplinaires du privé « relèvent des rapports contractuels de droit privé » ; « le juge civil peut être saisi … il vérifie alors qu'elle ne porte pas un trouble manifestement illicite aux principes fondamentaux du droit disciplinaire » (AN QE 86829, citant Cass. 11-03-2010, n° 09-12453) [V] ; une exclusion définitive prononcée dans un établissement sous contrat d'association relève du **juge judiciaire** (TA Melun 2015) [V].
- **Ce qui s'applique** : L111-6 (harcèlement), R470-1 à R470-4 (le chef d'établissement est responsable de traitement), le Code pénal et l'article 434-3 ; L511-5 (téléphone) à confirmer [V ; Q-DS16]. Le chef d'établissement « assume la responsabilité de l'établissement et de la vie scolaire » (R442-39, R442-55) [V].
- **Enseignement catholique** (document de 2012 citant le texte du CNEC de 2007) [V-ext] : textes du public non applicables ; conseil de discipline **consultatif** (« le chef d'établissement prend la responsabilité de la décision après avoir recueilli l'avis du conseil ») ; composition et convocation (5 jours ouvrés) par le règlement intérieur ; avocat « pas de droit » ; appel seulement s'il est prévu par le règlement intérieur. Statut actuel du SGEC [NT].
- **Ce que fait ScolIA [R]** (comme le socle 42 pour les sorties) : profil `prive_sous_contrat` **préréglé sur les garanties du public** — échelle reprise (le règlement intérieur peut la nommer autrement, la catégorie reste), information préalable avec **droit de se taire** et assistance, délai de défense de deux jours ouvrables, convocation 5 jours (jours **ouvrés** si le règlement le dit, S-1302), motivation, voies de recours **du règlement intérieur** (appel interne s'il existe, médiation de la tutelle, juge judiciaire), effacement R511-13 IV appliqué comme bonne pratique (cohérent avec le dossier unique de l'élève) ; **conseil consultatif** : le vote produit un **avis**, la décision est un acte distinct du chef (DIS-211) ; aucune transmission au recteur ; R421-10-2 (Faits établissement) seulement si l'extension au privé est confirmée.

### 5.5 Ce qui reste à vérifier [NV] ou à faire trancher
Voir §30.2 (questions au juriste et au DPO). Principaux : garanties à imposer dans le privé et doctrine actuelle du SGEC ; L511-5 dans le privé ; circulaire plus récente que 2014-059 ; données disciplinaires et article 10 du RGPD quand les faits sont pénalement qualifiables ; instruction de tri 2005 toujours en vigueur et articulation avec l'effacement ; effacement à la demande et exclusion définitive ; délégation au-delà des adjoints et conduite du contradictoire par la CPE ; information des parents d'un élève majeur ; internat comme « service annexe » [NT] ; dispositions transitoires des décrets 2026-414 et 2026-637 ; articulation de R470 avec le socle 36.

---

## 6. Branchements sur le cœur et sur les autres socles

### 6.1 Qui possède quoi

| Donnée | Propriétaire | Le socle 13 |
|---|---|---|
| Élève, identité affichée, scolarisation à la date, majorité, émancipation | 2 | lit (désignation, classe du jour, `est_majeur_a(date)`) ; **jamais** recopiée |
| Titulaires de l'autorité parentale, parent sans exercice, interdictions de communication, adresse **due** et adresse protégée, personne de confiance | 4 | lit les **destinataires dus** d'une information disciplinaire (`destinataires(eleve, nature, date)`) ; ne choisit jamais lui-même une adresse |
| Classes, groupes, cours, salles, calendrier, jours ouvrés | 5 | lit ; calcule les **jours ouvrables** avec le calendrier de l'établissement (§9.3) ; publie des **mesures d'éloignement** sans motif (R-S7 du 05) |
| Agents, missions (CPE référent, référent harcèlement, PP), délégations de signature, liens personnels déclarés | 6 | lit ; propose le **déport** (R-P6 du 06) ; vérifie le **pouvoir de signer** (`peut_signer(agent, "discipline", date)`, 07) |
| Documents : information préalable, notification, convocation, PV, convention de responsabilisation, engagement ; preuve de remise, date de notification effective, gel, effacement du dossier visible | 7 | **déclare** les natures et fournit leurs données (lecture publiée typée) ; demande l'émission **dans sa transaction** ; lit la date de notification effective (`etat()`) |
| Avis aux familles et aux personnels | 8 | déclare les natures `discipline.*` (enveloppe, C2) ; ne compose jamais un SMS ni un e-mail lui-même |
| Séances datées, modification d'une séance pour un élève | 10 | lit les séances (lieu du fait, exclusion de cours) ; demande la **disponibilité d'une salle** pour une séance de retenue ; publie l'exclusion-inclusion (CL-64 du 10) |
| Appel, absences, demi-journées, retards | 12 | **publie** des **états attendus** par élève et par période (« exclu temporairement », « mesure conservatoire », « en retenue », « exclu de cours, pris en charge ») que l'appel pré-remplit ; **lit** l'assiduité pour les règles de vigilance ; **jamais** une absence ne devient une sanction automatiquement |
| Accueil, accès, remise de l'élève | 14 | publie « accès interdit du … au … » (mesure conservatoire, exclusion de l'établissement) et « élève exclu de cours en route vers la vie scolaire » |
| Messagerie, carnet de correspondance | 16 | ne possède pas le carnet ; un mot du carnet peut **référencer** une punition ; un message signalé « harcèlement » (CM-29, CM-72 du 16) ouvre une **situation** au socle 13 |
| Rendez-vous, entretiens | 19 | demande un créneau d'entretien (CPE, famille) ; garde le compte rendu chez lui (§14) |
| Travail à faire pendant une exclusion | 20 | publie la période d'exclusion pour que les professeurs déposent le travail ; l'élève exclu garde l'accès au cahier de textes |
| Notes, fraude présumée (CE-49), absence à une évaluation pendant une exclusion (CE-21) | 21 | reçoit la fraude comme **incident** ; publie « exclusion » comme justification d'absence à une évaluation, sans motif ; **le zéro n'est jamais une punition** (21 R-02) |
| Synthèse vie scolaire du conseil de classe | 23 | fournit une **lecture minimisée** : nombre de punitions et d'exclusions de cours, sanctions **non effacées** de l'année, suivi en cours oui/non ; **aucune** sanction effacée, aucun fait d'un autre élève |
| Conservation, archives, demandes de droits | 27 | déclare ses catégories de conservation ; applique l'effacement R511-13 IV ; transmet les pièces à archiver |
| Santé | 34 | **aucune** donnée de santé dans le socle 13 ; référence « passage à l'infirmerie à l'occasion de l'incident : oui » (lien sans contenu) ; usage médical du téléphone : **jamais** sanctionné (R-S51 du 34) |
| Besoins particuliers (PAP, PPS, AESH) | 35 | lit une **pastille** « besoins particuliers à prendre en compte » pour la personne qui décide, jamais le contenu ; l'AESH peut être témoin |
| Informations préoccupantes, cellule de veille, psychologue, service social | 36 | **transmet** une situation (harcèlement, violence subie) quand elle devient une question de protection de l'enfance ; ne garde **pas** l'information préoccupante |
| Fraude aux examens nationaux | 38 | hors procédure de l'établissement (instance académique) [NV] ; le 13 ne fait qu'enregistrer l'incident constaté |
| Cantine, internat : exclusion d'un service annexe | 40, 41 | publie l'exclusion du service (dates) ; les incidents d'internat sont des **incidents** du 13, lieu « internat » |
| Sortie ou voyage | 42 | un incident peut se rattacher à une sortie ; le 13 ne décide pas de l'exclusion d'une sortie comme punition (cas limite CD-41) |
| Membres élus des instances (CA, conseil de discipline, commission éducative) | 46 | lit la composition **à la date** ; à défaut de socle 46, saisie locale de la composition au socle 13 (lot D5) |
| Indicateurs | 47 | expose des agrégats sans nom (§21.4) |
| Départ après exclusion définitive | 3 | publie `discipline.exclusion_definitive_prononcee` (date d'effet) : le 3 ouvre le départ (03 §… « origine discipline ») ; l'appel au recteur **n'est pas suspensif** (D511-50 [V], RG-31) : le départ s'ouvre à la date d'effet ; une décision d'appel qui annule ou réforme rouvre la scolarité (§6.4) |

### 6.2 Ports consommés

| Port | Socle | Usage |
|---|---|---|
| `eleve::designation(eleve_id, date, lecteur)` | 2 | affichage, prénom d'usage (R17) |
| `eleve::est_majeur_a(eleve_id, date)`, `emancipe` | 2 | destinataires et signataires des pièces de procédure |
| `administratif::destinataires(eleve_id, nature, date)` | 4 | qui reçoit l'information préalable, la convocation, la notification ; raison figée |
| `administratif::adresse_due(personne_id, nature)` | 4 | adresse du recommandé, sans révéler une adresse protégée |
| `structure::jours_ouvrables(etab, de, nb)` et `calendrier::est_ouvre(date)` | 5 | délais (R-DS10) |
| `structure::cours_de(agent, date)`, `eleves_du_cours` | 5 | portée du professeur (D-022) |
| `personnels::missions_actives(type, perimetre, date)` | 6 | CPE référent, référent harcèlement, PP |
| `personnels::peut_signer(agent, "discipline", date)` | 6/7 | signature des notifications |
| `personnels::lien_personnel(agent, eleve)` | 6 | déport au conseil de discipline et à la commission |
| `documents::emettre(nature, donnees, destinataires, tx)` et `documents::etat(document_id)` | 7 | courriers et preuve de notification |
| `notifications::declarer_avis(nature, …, tx)` | 8 | avis internes et aux familles |
| `edt::seances(eleve|agent, periode)`, `edt::salle_disponible(salle, creneau)` | 10 | lieu du fait, retenues |
| `assiduite::compteurs(eleve, periode)` | 12 | règles de vigilance (lecture agrégée) |

### 6.3 Ports exposés par le socle 13

| Port | Consommateurs | Contenu |
|---|---|---|
| `discipline::etat_attendu(eleve, date, demi_journee)` | 12, 14, 40, 41, 20, 21 | `exclu_etablissement`, `exclu_classe` (présent, hors cours), `mesure_conservatoire`, `exclu_service(cantine|internat)`, `en_retenue`, `exclu_cours_pris_en_charge` ; **sans motif** |
| `discipline::pastille(eleve, lecteur, date)` | fiche 2, appel 12, 23 | « suivi éducatif en cours » (pour l'équipe), « exclu jusqu'au … » ; rien si le lecteur n'a pas la portée |
| `discipline::synthese_conseil(eleve, periode, lecteur)` | 23 | nombres et sanctions **non effacées** de l'année, sans texte libre |
| `discipline::chronologie(eleve, lecteur)` | 2 | entrées à **titre neutre** (« Punition », « Procédure disciplinaire », « Sanction »), filtrées par droit ; les effacées disparaissent |
| `discipline::eloignements(etab, date)` | 5, 10, 42 | paires d'élèves à séparer, **identifiants seulement**, origine `mesure`, sans motif |
| `discipline::faits_agreges(etab, periode)` | 47, 9 (enquêtes) | comptages par catégorie, niveau, lieu ; seuil de secret statistique (§21.4) |
| Lectures publiées typées des natures de documents | 7 | données de chaque courrier (faits **rédigés pour la famille**, jamais le texte interne) |

### 6.4 Répercussions automatiques (le cœur bouge, la discipline suit)

| Événement | Effet |
|---|---|
| L'élève change de classe ou de groupe | les punitions données gardent leur auteur ; la **portée** du nouveau professeur s'applique aux faits **futurs** ; l'ancien professeur garde la vue de ce qu'il a lui-même saisi (lecture seule, R-DS27) |
| L'élève devient **majeur** pendant une procédure | les pièces suivantes lui sont adressées ; les titulaires ne reçoivent plus que ce que §5 permet ; signalé au décideur (CD-30) |
| Un titulaire perd l'autorité parentale, une interdiction de communication est posée | les destinataires des pièces **suivantes** sont recalculés ; les pièces déjà notifiées ne sont pas renvoyées ; alerte si une convocation en cours est touchée |
| L'élève **quitte** l'établissement | procédures en cours : alerte à la direction (abandon motivé ou poursuite si l'élève revient) ; effacement à la demande si changement d'établissement (R511-13 IV) ; au terme de la scolarité du second degré, effacement de toutes les sanctions |
| Fusion de deux fiches élève (02 E6) | les faits suivent le conservé ; les rôles dans un incident sont dédoublonnés |
| Fermeture exceptionnelle (05 S3) | les retenues de ce jour sont **reportées** (proposition) ou annulées, élèves et familles prévenus ; un délai de procédure est recalculé si le jour n'est plus ouvrable |
| Absence du surveillant de retenue (15) | séance proposée au remplacement ou au report |
| Sanction **réformée ou annulée** sur recours | notification initiale marquée (07 D79), effets annulés ou recalculés (appel, cantine, internat), effacement recalculé |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition | Table |
|---|---|---|
| **Observation** | Remarque datée d'un adulte sur un élève, positive ou négative, sans valeur de punition (« oubli de matériel », « très bonne participation »), visible de la famille si l'auteur le choisit | `ent_dis_observation` |
| **Incident** | Fait daté et situé, décrit une fois, avec des **personnes impliquées** et leur rôle | `ent_dis_incident`, `ent_dis_implique` |
| **Rôle dans un incident** | `auteur_presume`, `victime`, `temoin`, `signalant`, `implique` (rôle non qualifié), pour un élève ou un adulte | `ent_dis_implique.role` |
| **Punition scolaire** | Mesure d'ordre intérieur, prononcée par **tout personnel** (professeur, CPE, AED, direction) selon le règlement intérieur : inscription au carnet, excuse, devoir supplémentaire, retenue, exclusion ponctuelle d'un cours ; **jamais** de procédure | `ent_dis_punition` |
| **Retenue** | Punition exécutée lors d'une **séance de retenue** (date, heure, salle, surveillant, travail) | `ent_dis_retenue_seance`, `ent_dis_retenue_presence` |
| **Exclusion ponctuelle de cours** | Punition exceptionnelle : l'élève est **pris en charge** (vie scolaire), avec un rapport écrit | `ent_dis_exclusion_cours` |
| **Procédure disciplinaire** | Ce qui précède **toute** sanction : faits reprochés, information de l'élève et de ses représentants, délai de défense, observations, éventuellement conseil de discipline, décision motivée | `ent_dis_procedure` |
| **Sanction disciplinaire** | Une mesure de l'**échelle fermée** (avertissement, blâme, mesure de responsabilisation, exclusion temporaire de la classe, exclusion temporaire de l'établissement ou d'un service annexe, exclusion définitive de l'établissement ou d'un service annexe), avec ou sans sursis | `ent_dis_sanction` |
| **Mesure conservatoire** | Interdiction d'accès décidée par le chef d'établissement, le temps de la procédure, dans les cas prévus (§5) ; **n'est pas** une sanction | `ent_dis_mesure_conservatoire` |
| **Mesure de prévention, d'accompagnement, de réparation** | Mesures éducatives prévues par le règlement intérieur (confiscation d'un objet dangereux, engagement, tutorat, travail de réparation) | `ent_dis_mesure` |
| **Mesure alternative** à une sanction d'exclusion | Mesure de responsabilisation proposée à la place d'une exclusion ; son exécution éteint l'exclusion (§5) | `ent_dis_sanction.alternative_*` |
| **Commission éducative** | Instance de l'établissement qui examine la situation d'un élève et propose des réponses éducatives ; elle ne sanctionne pas | `ent_dis_commission*` |
| **Conseil de discipline** | Instance qui prononce les sanctions les plus lourdes ; séance, membres, vote | `ent_dis_conseil*` |
| **Suivi éducatif** | Dossier de suivi d'un élève par la CPE ou une équipe : objectifs, entretiens, engagements, bilans | `ent_dis_suivi*` |
| **Situation de harcèlement** | Signalement traité selon le protocole de l'établissement (pHARe dans le public), avec des personnes, des étapes et des mesures de protection | `ent_dis_situation*` |
| **Mesure d'éloignement** | Contrainte « ces deux élèves ne doivent pas être ensemble » (classe, groupe, sortie, chambre), publiée **sans motif** | `ent_dis_eloignement` |
| **Effacement** | Retrait d'une sanction du **dossier administratif** de l'élève à la date réglementaire ; la ligne devient invisible des lecteurs courants, son texte libre est vidé, l'existence est tracée pour la preuve | `ent_dis_sanction.efface_le`, `ent_dis_effacement` |
| **Règle de vigilance** | Règle déterministe et lisible (« 3 punitions en 15 jours ») qui crée une **alerte** pour un adulte désigné, jamais une sanction | `ent_dis_vigilance_regle`, `ent_dis_alerte` |
| **Profil de procédure** | Cadre juridique de l'établissement : `eple` (code de l'éducation), `prive_sous_contrat` (règlement intérieur, garanties du public par défaut), `prive_hors_contrat` | `ent_dis_profil` |

### 7.2 Distinctions qui manquent aujourd'hui (Rust et Next.js) [F]

1. **Incident ≠ élève** : l'incident Rust a un `eleve_id` unique (`019_vs_sanctions.sql`) ; Next.js n'a pas d'incident hors de l'internat (JSON).
2. **Punition ≠ sanction** : séparées en Rust depuis 133 ; confondues dans Next.js (« colle », « exclusion de cours », « conseil de discipline » et « mot carnet » rangés dans les sanctions, VD-09 du §2.2).
3. **Procédure ≠ sanction** : la sanction Rust peut exister sans procédure ; Next.js traite « conseil de discipline » comme un type de sanction.
4. **Texte interne ≠ texte pour la famille** : un seul champ `motif`/`note` montré tel quel (Next.js VD-10 ; Rust à vérifier §2).
5. **Effacement ≠ annulation** : une sanction annulée (recours) n'a jamais existé juridiquement ; une sanction effacée a existé et a été exécutée.
6. **Observation positive** : absente partout, alors que l'internat Next.js l'avait prévue (« valorisation », non enregistrée à cause de VD-12).

---

## 8. Du fait à la suite : le parcours gradué

### 8.1 Saisir (corrige l'incident à un seul élève et la ressaisie)

- **Depuis l'endroit du fait** : depuis l'appel (séance en cours : la séance, la salle, les élèves du cours sont proposés), depuis la fiche élève, depuis la cour (mobile, lieu choisi dans une liste courte), depuis l'internat (41), une sortie (42), la cantine (40), la messagerie (« signaler », 16).
- **Trois niveaux de saisie, un seul objet** :
  - **observation** (30 secondes : élève(s), catégorie, une phrase, visible famille oui/non) ;
  - **incident** (faits, lieu, heure, personnes et rôles, mesures immédiates prises, pièces) ;
  - **rapport d'incident** demandé par la CPE ou la direction (incident enrichi : récit circonstancié, témoins entendus).
- **Deux textes** dès la saisie (R-DS5) : les **faits internes** (pour les personnels qui ont à en connaître) et, plus tard, la **rédaction pour la famille** produite au moment d'une punition, d'une information préalable ou d'une notification (sans nom d'autre élève, contrôlée par R-D11 du 07).
- **Contrôle de contenu** à l'enregistrement : nom d'un autre élève de l'établissement dans un texte destiné à une famille, donnée de santé apparente, insulte recopiée sans guillemets : avertissement à l'auteur, pas de blocage pour le texte interne, blocage pour le texte famille (R-DS6).
- **Ce qui part tout de suite** : rien vers les familles (une observation « visible famille » part en avis U1 enveloppe, 8) ; l'incident entre dans la **file de traitement** de la vie scolaire (CPE référent du niveau, à défaut toute la vie scolaire) ; une **gravité** « immédiate » (violence physique, arme, intrusion, mise en danger) prévient la direction par avis interne U2.

### 8.2 Traiter (la file de la vie scolaire)

Pour chaque incident et **chaque élève impliqué** : prise en charge (qui), entretien(s), suites proposées, décision, clôture. Suites possibles pour un **auteur présumé** : aucune suite (motif), observation, mesure de prévention ou d'accompagnement, punition, saisine de la commission éducative, **engagement d'une procédure disciplinaire** (direction), transmission (36 : protection de l'enfance ; signalement au procureur, §5). Pour une **victime** : mesures de protection (éloignement, information de la famille, orientation vers infirmerie, psychologue), suivi. Pour un **témoin** : rien par défaut, protection si besoin.

Le professeur qui a saisi voit **l'état** de son rapport (« pris en charge par M. X le … », « suite donnée : entretien CPE et retenue »), jamais la procédure en détail ni ce qui concerne les autres élèves (R-DS21).

### 8.3 Punir (punitions scolaires, tout personnel)

- Types du **catalogue de l'établissement** (règlement intérieur), avec une **nature** fixée par ScolIA (`inscription_carnet`, `excuse`, `devoir_supplementaire`, `retenue`, `exclusion_cours`, `travail_reparation`, `confiscation`, `autre`) ; **interdits** codés (R-DS2) : aucune nature « zéro », « lignes », « punition collective » ; un type « autre » exige une description et reste soumis au contrôle de la CPE.
- **Individualisée** : une punition pour **un** élève ; la saisie « même punition pour plusieurs » crée **une punition par élève**, chacune avec son motif personnel (corrige VD-20 de Next.js sans recréer une punition collective).
- **Portée** : un professeur punit les élèves de **ses cours** (D-022) et, hors cours (couloir, cour, sortie), tout élève de l'établissement s'il a l'action `punitions.donner` en portée `etablissement` — le modèle par défaut « Professeur » l'a **pour le seul cas du fait constaté** (R-DS20, Q-DS4).
- **Information** de la famille : avis `discipline.punition` (8 : U1, enveloppe), sauf inscription au carnet (déjà un mot) ; élève informé dans l'ENT.
- **Annulation** et **levée** : annulée (erreur, motif obligatoire, l'avis est corrigé) ; levée (la CPE décide qu'elle n'est plus à faire) ; **non effectuée** (retenue manquée) → suite proposée, jamais automatique (R-DS15).

### 8.4 Retenues

- **Séances de retenue** créées par la vie scolaire (gabarit récurrent : « mercredi 13 h – 15 h, salle B12, surveillant selon planning ») ; capacité ; salle réservée au socle 10 ; surveillant (agent du socle 6) ; liste calculée des élèves affectés.
- Une punition de nature `retenue` est **affectée** à une séance : par le donneur (choix d'une séance compatible : pas de cours de l'élève à cette heure, pas d'exclusion, pas d'interdiction de transport [H]) ou par la vie scolaire ; **conflit** détecté (élève en sortie, en stage, en examen, interne qui rentre le mercredi, demi-pensionnaire ou non) ; famille informée de la **date et l'heure** (avis U1) au moins N jours avant (S-1305, défaut 2 jours ouvrés [H]).
- **Appel de retenue** sur mobile (AED) : présent, absent, retard, excusé ; travail rendu ; absence à la retenue : avis à la CPE, suite **proposée** (report, nouvelle punition) ; l'absence à une retenue **n'est pas** une absence au sens de l'assiduité (pas de demi-journée), mais elle est visible de la famille.
- **Report** collectif (fermeture, surveillant absent) : tous les élèves de la séance proposés sur une autre séance, familles prévenues en une fois.

### 8.5 Exclusion ponctuelle d'un cours (sûreté)

1. Le professeur l'enregistre **depuis l'appel** (état de l'élève « exclu de cours » à l'heure H) avec un motif court ; ScolIA crée la punition et un **suivi de prise en charge**.
2. La vie scolaire voit « Lucas, exclu de 4B par Mme X à 10 h 12, en route » ; à l'arrivée, l'AED confirme « arrivé à 10 h 15 » ; **sans confirmation dans un délai** (S-1306, défaut 10 minutes), alerte U2 à la vie scolaire et au professeur (CD-12).
3. Le **rapport écrit** du professeur est demandé (incident lié) ; rappel s'il manque en fin de journée.
4. L'élève reste **présent** pour l'assiduité ; il n'est pas absent du cours au sens du décompte (§5) ; la famille est informée (avis U1).
5. Répétition (S-1307, défaut 3 dans la période) : alerte de vigilance à la CPE et au PP.

### 8.6 Engager une procédure disciplinaire

- **Qui** : le chef d'établissement (ou son adjoint par délégation, R421-13 [V-ext]) ; la CPE **propose** (demande de procédure), elle n'engage pas (Q-DS5).
- **Engagement obligatoire** (RG-11) : violence verbale envers un personnel, acte grave envers un personnel ou un élève, atteinte grave aux principes de la République, **harcèlement ou cyberharcèlement** envers un élève (même scolarisé ailleurs) : l'incident ainsi catégorisé crée une **tâche bloquante** pour la direction, **sans renoncement possible** ; **saisine du conseil imposée** (RG-12) en cas de violence physique envers un personnel ou d'**arme** introduite ou portée : la décision du chef seul est refusée (R-DS11). Hors de ces cas, le chef peut renoncer motivément ; une **demande écrite** de saisine du conseil refusée appelle une décision motivée (RG-16).
- **Contenu** : élève (une procédure par élève, R-DS1), incidents rattachés (un ou plusieurs), **faits reprochés** rédigés (texte que l'élève et ses représentants liront), instance envisagée (chef seul / conseil de discipline), pièces du dossier (rapports, témoignages anonymisés pour la famille si demandé, §12.4).
- **Information préalable** (document 7, nature `discipline.information_prealable`) : faits reprochés, **droit de garder le silence pendant toute la procédure** (R511-12-1, RG-08, mention bloquante de la trame), droit de présenter sa défense oralement ou par écrit, droit de se faire **assister** par une personne de son choix, délai, possibilité de **consulter le dossier** ; destinataires : l'élève, et ses **représentants légaux** s'il est mineur (destinataires calculés par le socle 4) ; **date de notification effective** lue au socle 7 (R-N40 du 08 ; D80 du 07).
- **Délai de défense** (EPLE : au moins deux jours ouvrables, §5) : le serveur calcule la **date au plus tôt** de la décision à partir de la notification effective (R-DS10) ; avant cette date, l'action « prononcer » est **refusée** avec l'explication.
- **Défense** : observations orales (rendez-vous, compte rendu signé ou non), écrites (dépôt par la famille dans l'ENT, papier scanné), consultation du dossier (date, présents, copie remise, DCT-149 du 07), personne qui assiste (nom, qualité).
- **Mesure conservatoire** (RG-15) : seulement une fois la procédure engagée ; interdiction d'accès pendant le délai de défense (chef seul) ou jusqu'à la comparution (conseil), prolongeable pendant l'appel ; motivée, notifiée ; l'élève mineur est **remis à son représentant légal** (14) ; effet immédiat sur l'appel (état attendu) et l'accueil ; elle **n'est pas** une sanction et ne figure pas au dossier comme telle.

### 8.7 Décider (chef d'établissement seul)

- **Liste « avant de pouvoir décider »** : notification effective faite, délai écoulé, observations recueillies ou délai expiré sans réponse, aucune saisine obligatoire du conseil de discipline en suspens.
- **Décision** : sanction de l'échelle **à laquelle le chef a droit seul** (§5), durée, sursis (total ou partiel, durée du sursis), **mesure alternative** proposée pour une exclusion, **motivation** (texte obligatoire, faits retenus, raisons du choix) ; ou **aucune sanction** (classement motivé) ; ou **saisine du conseil de discipline**.
- **Contrôles** : même élève que la procédure ; pas deux sanctions pour les mêmes faits (non bis in idem, R-DS3) ; sursis en cours détecté (« le sursis de l'exclusion du 12 mars court jusqu'au 12 septembre : révocation ? ») ; exclusion > durée maximale refusée ; élève en situation de handicap signalé (pastille 35) ; élève de moins de 16 ans et exclusion de l'établissement : rappel de l'accueil (§5) ; élève majeur : destinataires.
- **Notification** (document 7, `discipline.notification_sanction`) : sanction, motivation, date d'effet, **voies et délais de recours** (EPLE : selon §5 ; privé : règlement intérieur), date d'effacement prévue ; destinataires dus ; date de notification effective tracée.

### 8.8 Conseil de discipline
Voir §12.

### 8.9 Exécuter (effets sur la journée et les services)

| Sanction | Effets automatiques (après notification, à la date d'effet) |
|---|---|
| Avertissement, blâme | aucun effet sur la journée ; ligne au dossier, date d'effacement |
| Mesure de responsabilisation | exécution suivie (§10) ; heures, lieu, structure, accord recueilli ; non-exécution → retour au décideur |
| Exclusion temporaire **de la classe** | élève **présent** dans l'établissement, retiré de ses cours (exclusion-inclusion, 10 CL-64) : état attendu « exclu de la classe, accueilli en … » ; travail déposé (20) ; appel : pas d'absence |
| Exclusion temporaire **de l'établissement** | état attendu « exclu » sur l'appel (pas d'absence non justifiée, pas de signalement d'assiduité) ; accueil : accès interdit ; cantine et internat : non servis, non facturés [H, socle 40] ; travail (20) ; évaluations manquées (21 CE-21) |
| Exclusion temporaire **d'un service annexe** | cantine ou internat seulement ; cours normaux |
| Exclusion définitive | décision **immédiatement exécutoire**, appel non suspensif (RG-31) : événement vers le 3 (départ) à la date d'effet ; élève soumis à l'obligation scolaire : l'autorité est informée **immédiatement** et réaffecte (autre établissement ou classe relais, RG-28) ; privé : information de la famille de son obligation, accompagnement proposé |
| Sursis (3° à 6° seulement) | aucun effet tant qu'il n'est pas révoqué ; **échéance** de révocabilité suivie (au moins l'année en cours, au plus la durée d'inscription) ; révocation = décision nouvelle, après procédure, pour des faits passibles d'une sanction de niveau **égal ou supérieur** ; exécution cumulée plafonnée à 8 jours (RG-02, R-DS13) |

### 8.10 Recours

- EPLE : décision du **chef** : recours gracieux ou hiérarchique puis tribunal administratif sous 2 mois (RG-32) ; décision du **conseil** : appel au recteur sous **8 jours** de la notification écrite, non suspensif, décision du recteur sous **un mois**, recours préalable obligatoire avant le juge (RG-31) ; le dossier d'appel est **exporté** (pièces, PV, notification, preuves de remise) ; l'appel est enregistré (date, auteur), la décision d'appel aussi (confirmée, réformée, annulée) avec ses effets (§6.4).
- Privé : recours interne prévu par le règlement intérieur (s'il existe), médiation de la tutelle [H] ; contentieux devant le juge judiciaire [à vérifier §5].
- **Gel** du dossier pendant un recours ou un contentieux (07 D53) : ni effacement ni élimination tant que le gel court ; l'effacement prévu est **reporté** et la raison tracée.

---

## 9. Délais et calendrier (le moteur de dates)

### 9.1 Principes
1. **Un délai part d'un fait daté au serveur** (notification effective, remise contre signature, première présentation d'un recommandé selon la règle retenue), jamais d'une date saisie librement (R-DS10).
2. **Jours ouvrables** : du lundi au **samedi** hors jours fériés ; le délai **court pendant les vacances** (éduscol 7 [V-ext], RG-14) ; ScolIA affiche un **avertissement** quand il court pendant des vacances et propose un délai plus long (la direction fixe le délai, au moins deux jours). Profil privé : jours **ouvrés** si le règlement intérieur le dit (S-1302).
3. **Le délai minimal se calcule, la date choisie se contrôle** : la décision ou la séance ne peut être fixée avant la date au plus tôt ; ScolIA **propose** la première date utile et la suivante (ex. « conseil possible à partir du jeudi 19 ; prochain jour sans cours du président : vendredi 20 »).
4. **Horodatage** de chaque fait qui fait courir un délai, à la minute, dans le fuseau de l'établissement (04 A0 bis, `etablissement_fuseau`).

### 9.2 Délais portés par le profil EPLE (valeurs du §5, non modifiables)
Défense avant sanction ; convocation au conseil de discipline ; appel ; durée maximale des exclusions temporaires ; durée de la mesure conservatoire ; délais d'effacement. **Profil privé** : mêmes valeurs **préréglées**, modifiables par la direction **vers le haut seulement** sans décision tracée (un délai plus long protège) ; vers le bas, décision motivée de la direction, signalée (S-1301).

### 9.3 Effacement (R511-13 IV)
Calculé à la décision : **date d'effacement** = fin de l'année scolaire pour l'avertissement ; fin de l'année scolaire **suivante** pour le blâme et la mesure de responsabilisation ; fin de la **deuxième** année scolaire suivante pour les autres sanctions, sauf l'exclusion définitive ; mesure alternative : à l'issue de l'année suivante ; **toutes** au terme de la scolarité dans le second degré ; **sur demande** au changement d'établissement. « Fin de l'année scolaire » = date de fin de l'année du référentiel (05), pas le 31 août par défaut [à confirmer, Q-DS3]. **Portée** (RG-06) : la sanction et les **pièces de procédure** sont retirées de la vue courante et **archivées** (accès restreint), les documents relatifs aux **faits** suivent leur propre durée ; la demande d'effacement au changement d'établissement ne vise pas l'exclusion définitive, peut être refusée, et vaut acceptation après **deux mois** de silence.

---

## 10. Mesures éducatives, responsabilisation, alternatives

- **Mesures de prévention** (règlement intérieur) : confiscation d'un objet (téléphone, L511-5 : rendu à la famille ou à l'élève selon la règle, **jamais** l'usage médical, 34 R-S51), engagement écrit, fiche de suivi, tutorat par un adulte.
- **Mesures de réparation** : travail d'intérêt collectif dans l'établissement, réparation d'un dommage (jamais une pénalité financière, R-DS16 ; la réparation civile d'un dommage relève d'une démarche distincte, hors socle, Q-DS9).
- **Mesure de responsabilisation** (sanction) : dans l'établissement ou dans une **structure partenaire** liée par **convention** (document 7, signé par l'établissement, la structure, l'élève et ses représentants s'il est mineur) ; **accord** de l'élève (et des représentants) recueilli quand le texte l'exige (§5) ; heures prévues, réalisées, attestation de la structure ; **annuaire des structures** (associations, collectivités) partagé dans l'établissement.
- **Alternative à l'exclusion** : proposée au moment de la sanction ; acceptée ou refusée par l'élève et ses représentants ; si elle est exécutée, l'exclusion n'est pas mise à exécution et la mention suit la règle d'effacement propre (§5) ; si elle est refusée ou non exécutée, l'exclusion s'applique (décision tracée).

---

## 11. Commission éducative (EPLE ; équivalent optionnel dans le privé)

- **Composition** à la date (règlement intérieur ; CA dans le public) : chef d'établissement ou son représentant (président), CPE, professeurs, au moins un parent, autres membres ; membres suppléants (46 ou saisie locale).
- **Saisine** : par le chef d'établissement, sur proposition de la CPE ou d'un professeur ; motif (comportement répété, difficulté d'intégration, préalable à une procédure) ; élève et représentants **invités** (convocation simple, 19).
- **Séance** : cas examinés (plusieurs élèves possibles, **un par un**, les familles ne se croisent pas : horaires distincts), propositions (engagement, mesure de responsabilisation, tutorat, fiche de suivi, orientation vers un professionnel), **suivi** des mesures avec échéances et bilan.
- **Ce qu'elle n'est pas** : elle ne prononce **aucune** sanction ; ses comptes rendus ne figurent pas au dossier administratif comme une sanction et suivent leur propre durée de conservation (21.3).
- Elle sert aussi au **suivi des mesures** alternatives et de responsabilisation (§5).

---

## 12. Conseil de discipline

### 12.1 Composition et membres
- **EPLE** : composition fixée par le code (§5), membres élus issus du CA (socle 46) ou désignés, titulaires et suppléants, à la date de la séance. ScolIA vérifie la **composition complète** et propose les suppléants ; il **propose le déport** d'un membre ayant un lien personnel déclaré avec l'élève ou la victime (06 R-P6), ou qui est lui-même témoin ou victime (CD-25).
- **Conseil de discipline départemental** et **délocalisation** (public, §5) : saisine tracée, séance hors de l'établissement, dossier transmis.
- **Privé** : instance définie par le règlement intérieur (conseil de discipline, « conseil d'éducation » [H]) ; composition saisie ; mêmes garanties par défaut.

### 12.2 Convocations
- Destinataires : élève, représentants légaux (mineur), personne qui l'assiste, membres, témoins, victime ou ses représentants si le règlement le prévoit [NV] ; **natures** de document distinctes (§16).
- **Modes** : pour l'élève, ses représentants et la personne qui assiste, **pli recommandé** ou **remise en main propre contre signature** (§5) — l'ouverture dans l'ENT ne vaut pas convocation (VERIF R35) ; pour les membres et témoins, tout moyen y compris courrier électronique.
- **Délai** : contrôlé sur la date de remise ou de présentation (Q-DS2), séance refusée si trop tôt ; **bordereau de recommandés** pour le secrétariat ; saisie du numéro d'envoi et de l'avis de réception ; remise en main propre sur tablette (07 D34).

### 12.3 Dossier
- Pièces rassemblées par la procédure (rapports, témoignages, observations, antécédents **non effacés**, mesures éducatives antérieures) ; **consultation** par l'élève, ses représentants, la personne qui l'assiste ; **accès des membres** temporaire, lecture seule, filigrané, clos deux jours après la séance (07 D82).
- **Rien d'effacé** ne peut entrer au dossier (R-DS25) ; aucune donnée de santé (R-DS7) ; une pastille « besoins particuliers » peut être portée **à la connaissance du président** (35), pas des membres sans accord de la famille [R, Q-DS10].

### 12.4 Témoins et anonymat
- Un témoignage écrit peut être **anonymisé pour la défense** quand la sécurité du témoin l'exige [NV, à valider juriste : équilibre entre contradictoire et protection, Q-DS6] ; l'identité reste connue du chef d'établissement ; l'anonymisation est une **décision motivée** tracée.
- Un élève témoin n'est jamais nommé dans une pièce adressée à une famille autre que la sienne.

### 12.5 Séance
- Déroulé guidé (§5) : ouverture, quorum, rappel des faits, audition de l'élève, de la personne qui l'assiste, des témoins, délibéré **hors** présence de l'élève, **vote à bulletin secret**, majorité, proclamation.
- Saisie en séance sur tablette par le secrétaire de séance (ou après) : présents, auditions (résumés), **résultats de vote** (pour / contre / blancs et nuls, par proposition, **jamais** le vote nominatif, R-DS24), décision.
- **Procès-verbal** (document 7) signé par le président et le secrétaire de séance ; **notification** de la décision (document 7), voies et délais de recours.

### 12.6 Après
- Exécution (§8.9), recours (§8.10), archives (le PV suit la règle d'archives : conservation selon l'instruction de tri, §21.3), effacement de la **sanction** au dossier administratif selon R511-13 IV.

---

## 13. Suivi éducatif et vigilance

### 13.1 Suivi éducatif
- **Ouvert** par la CPE (ou PP, référent) pour un élève : motif **codé** (comportement, décrochage, relations, travail, autre), objectifs, **référent**, échéance de bilan, participants (équipe).
- **Entrées** : entretien (date, présents : élève, famille, adulte), observation, engagement (document 7 éventuellement signé), point d'étape, bilan ; **visibilité** par entrée : `cpe` (auteur et CPE), `equipe` (personnels désignés du suivi), `famille` (rédigée pour elle). Le **texte confidentiel** de la CPE n'est lu que par l'auteur et la direction sur motif tracé (R-DS22).
- Pour un professeur de l'élève : **pastille** « suivi éducatif en cours, référent : Mme X », pas le contenu.
- Clôture avec bilan ; durée de conservation propre (21.3).

### 13.2 Règles de vigilance (détection précoce explicable, A6)
- **Déterministes**, lisibles, peu nombreuses, préréglées (S-1310 à S-1313) : « 3 punitions en 15 jours », « 2 exclusions de cours en 30 jours », « 5 observations négatives en 30 jours », « absences répétées le même jour + punitions » (lecture 12), « baisse de moyenne + punitions » (lecture 21, sans note affichée).
- **Alerte** au **référent** (CPE du niveau, PP), avec **les raisons** (« 3 punitions : 3/11, 7/11, 12/11 ») ; prise en charge, suite, classement ; **jamais** visible de la famille ni de l'élève comme « alerte » ; **jamais** de sanction automatique ; **jamais** de classement des élèves par score (IA-P12).
- **Permis à points** (T-22 de la matrice ; pratique de certains établissements [H]) : non retenu comme mécanique de sanction automatique (R-DS14) ; une règle de vigilance « barème » peut le **remplacer** comme aide à la décision, désactivée par défaut, **réglage signalé** (S-1314, Q-DS11).

---

## 14. Harcèlement entre élèves (protocole)

- **Signalement** de toutes sources : élève (formulaire ENT discret, CPE, infirmière), famille, personnel, témoin, messagerie (16 : message signalé → situation), questionnaire de campagne, 3018 (information reçue) ; accusé de réception à l'auteur du signalement, **sans** révéler la suite aux autres.
- **Situation** : personnes et rôles (élève cible, auteurs présumés, témoins), **référents** (équipe pHARe ou désignée, mission du 06), étapes du protocole de l'établissement (dans le public, celui du programme pHARe [V-ext, §5] : entretien de l'élève cible, évaluation, entretiens des auteurs selon la méthode retenue, information des familles, mesures de protection, points de suivi à échéances), **mesures de protection** (éloignement publié aux 5, 10, 42 sans motif ; adulte référent ; aménagement des trajets [H]), suites disciplinaires : dès que les faits de harcèlement sont établis, l'engagement d'une **procédure contre chaque auteur est obligatoire** (R421-10 d, RG-11), même si la victime est scolarisée ailleurs ; une procédure par auteur, l'élève cible n'est jamais convoqué avec eux.
- **Cloisonnement** : la situation est lisible des seuls référents et de la direction ; un professeur reçoit une **consigne** (« ne pas placer A et B ensemble ») sans le motif ; la famille de l'élève cible est informée des mesures **pour son enfant**, jamais des sanctions prononcées contre un autre élève (§5, à vérifier) ; la famille d'un auteur présumé n'apprend jamais le nom de l'élève qui a signalé.
- **Transmission** : quand la situation relève de la protection de l'enfance ou d'une infraction (cyberharcèlement grave, violences), elle est **transmise** au socle 36 (information préoccupante) ou fait l'objet d'un **avis au procureur** (C. proc. pén. 40 al. 2 [V, 16 R-17]) ; ScolIA trace la transmission, **ne l'envoie pas lui-même** sans action humaine.
- **Preuves** : captures et messages joints sont **gelés** (16 : gel ; 7 : gel D53) ; aucune suppression pendant la situation.
- **Pilotage** : nombre de situations, délais de prise en charge, sans nom (47).

---

## 15. Ce que voient les familles et les élèves

| Qui | Voit | Ne voit jamais |
|---|---|---|
| Titulaire de l'autorité parentale (mineur) | observations visibles, punitions et leur exécution (date de retenue), procédures **de son enfant** et leurs pièces, sanctions **non effacées**, mesures, suivi (entrées « famille »), dates d'effacement, voies de recours | autres élèves impliqués, témoins, victime, texte interne, alertes, situations de harcèlement où son enfant est auteur présumé avant l'information prévue, votes, sanctions effacées |
| Parent sans exercice de l'autorité parentale | **information** des sanctions et des décisions importantes (373-2-1 [V]) ; pas les pièces de défense, pas les actes (il ne signe pas, ne consulte pas le dossier comme partie) [R, Q-DS7] | idem + ce qu'une décision du juge lui interdit (interdiction de communication, 04) |
| Élève mineur | ses propres observations, punitions, procédures (il est **partie** : il reçoit les pièces), sanctions, dates d'effacement, ses engagements | idem |
| Élève majeur | tout ce qui le concerne ; il est le destinataire des pièces | — ; ses parents ne sont informés que selon §5 (Q-DS8) |
| Victime (et sa famille) | les mesures prises **pour la protéger** ; l'information qu'une suite a été donnée [NV : aucun texte trouvé sur ce qui peut être dit ; secret du conseil, RG-25] | la sanction nominative de l'auteur [R, Q-DS18] |

---

## 16. Documents produits (natures déclarées au socle 7)

Trames prêtes (D-023, 3 ou 4 par nature), contenu rempli par les données, **contrôle de contenu** R-D11 du 07, série `DIS`.

| Nature | Destinataires | Mode qui vaut notification | Conservation (21.3) |
|---|---|---|---|
| `discipline.avis_punition` | titulaires (mineur), élève | ouverture (S0) | courte |
| `discipline.convocation_retenue` (inclus dans l'avis) | idem | ouverture | courte |
| `discipline.rapport_incident` (interne) | — | — | dossier de procédure ou courte |
| `discipline.information_prealable` | élève, titulaires | ouverture authentifiée, remise contre signature, courrier (selon nature, 07 D80) | dossier disciplinaire |
| `discipline.compte_rendu_defense` | interne, copie à la famille si demandée | — | dossier disciplinaire |
| `discipline.mesure_conservatoire` | élève, titulaires | remise contre signature ou recommandé (recommandé si urgence) | dossier disciplinaire |
| `discipline.notification_sanction` | élève, titulaires, parent sans exercice (information) | selon nature | dossier disciplinaire, effacement R511-13 |
| `discipline.convocation_conseil` (élève, représentant, personne qui assiste) | — | **recommandé ou remise en main propre contre signature** | dossier disciplinaire |
| `discipline.convocation_conseil_membre`, `_temoin` | membres, témoins | tout moyen | courte |
| `discipline.proces_verbal_conseil` | archives | — | **10 ans, tri** (instruction 2005, VERIF R41) |
| `discipline.notification_decision_conseil` | élève, titulaires | recommandé ou main propre [NV] | dossier disciplinaire |
| `discipline.convention_responsabilisation` | établissement, structure, élève, titulaires | signatures (07 S2) | dossier disciplinaire |
| `discipline.engagement` | élève, titulaires | prise de connaissance ou signature | suivi éducatif |
| `discipline.convocation_commission` | élève, titulaires | ouverture | suivi |
| `discipline.compte_rendu_commission` | équipe, famille (extrait) | — | suivi |
| `discipline.dossier_appel` (export) | autorité académique | — | dossier disciplinaire |
| `discipline.attestation_effacement` [R] | élève majeur ou titulaires, sur demande | — | courte |

---

## 17. Cas limites (CD-01 à CD-60)

### 17.1 Faits, personnes, saisie

| # | Cas | Réponse |
|---|---|---|
| CD-01 | Bagarre à trois dans la cour, deux auteurs présumés et un élève qui s'interposait | un incident, trois rôles ; une suite par élève ; la famille de chacun ne voit que son enfant (R-DS1, R-DS5) |
| CD-02 | Le même fait est déclaré par deux professeurs | doublon proposé (même lieu, heure, élèves) ; fusion par la CPE (DIS-133) |
| CD-03 | Un rapport cite nommément un élève témoin | texte interne inchangé ; le texte famille est **bloqué** tant qu'il contient le nom (R-DS6) |
| CD-04 | Auteur inconnu (tag sur un mur) | incident sans auteur ; rôle ajouté plus tard ; pas de suite tant qu'il est inconnu |
| CD-05 | Fait commis **hors de l'établissement** (réseau social, arrêt de bus) | lieu « en ligne » / « abords » ; sanctionnable si lié à la qualité d'élève (RG-36) ; avertissement à la direction sur ce critère |
| CD-06 | Victime **scolarisée dans un autre établissement** (cyberharcèlement) | personne externe décrite sans compte ; procédure obligatoire (RG-11 d) ; aucune donnée transmise à l'autre établissement sans base légale (Q-DS14) |
| CD-07 | Adulte extérieur auteur (parent qui menace un professeur) | rôle d'une personne non élève ; aucune sanction de l'élève ; rappel RG-41 (DASEN) et dépôt de plainte hors ScolIA |
| CD-08 | Un AED est lui-même mis en cause | incident visible de la direction seule (personnel impliqué) ; relève du socle 6 / 49, pas de l'échelle |
| CD-09 | Incident saisi pour un élève **hors** des cours du professeur, vu dans le couloir | fait constaté (R-DS20) : saisie permise, lecture de l'historique refusée |
| CD-10 | Professeur qui veut punir toute la classe | refus ; proposition « une punition par élève identifié » (R-DS2) |
| CD-11 | Saisie depuis le téléphone **sans réseau** (sortie) | brouillon local chiffré, envoyé au retour ; heure du fait conservée, heure de saisie tracée |
| CD-12 | Élève exclu de cours **n'arrive pas** à la vie scolaire | alerte U2 après S-1306 ; recherche ; l'appel du créneau montre « exclu de cours, non arrivé » ; procédure de sûreté (14) |
| CD-13 | Exclusion de cours pour un élève avec **AESH** | l'AESH est prévenue ; pastille « besoins particuliers » au CPE ; le PPS peut prévoir l'accompagnement (35) |
| CD-14 | Téléphone confisqué non rendu le soir | rappel au confiscateur et à la CPE ; trace de remise (DIS-136) |
| CD-15 | Téléphone utilisé pour une **application médicale** (diabète) | confiscation et punition refusées si la consigne santé existe (34 R-S51, RG-40) |

### 17.2 Procédure et délais

| # | Cas | Réponse |
|---|---|---|
| CD-16 | Information préalable ouverte le vendredi à 17 h | délai à partir de la notification effective (07 D80) ; samedi compté (RG-14) ; décision possible le mardi au plus tôt si le chef retient deux jours |
| CD-17 | Délai qui court pendant les vacances d'hiver | avertissement, proposition d'un délai plus long ; décision non bloquée (RG-14) |
| CD-18 | Élève déjà sous **sursis** d'exclusion de 3 jours, nouveau fait de niveau « blâme » | révocation **refusée** (niveau inférieur, RG-02) ; sanction nouvelle possible |
| CD-19 | Révocation d'un sursis de 5 jours + nouvelle exclusion de 5 jours | exécution cumulée plafonnée à 8 jours (RG-02) : le serveur bloque au-delà |
| CD-20 | Sanction envisagée pour des faits déjà punis d'une retenue | avertissement non bis in idem (R-DS3) ; la direction motive (fait distinct, aggravation) |
| CD-21 | Deux incidents distincts d'un même élève | jonction en une procédure et une décision (RG-29) |
| CD-22 | Retour d'un élève exclu pour violence | mesure d'accompagnement proposée (RG-35), entretien de réintégration planifié |
| CD-23 | Parents qui ne viennent pas et n'écrivent pas | délai expiré sans observations : décision possible ; trace « aucune observation reçue » |
| CD-24 | Famille qui demande un report de l'entretien | report accordé ou refusé motivé ; nouvelle date au-delà du délai minimal |
| CD-25 | Membre du conseil de discipline qui est **la victime** ou **le témoin** | déport imposé (RG-22, R-P6) ; suppléant convoqué |
| CD-26 | Parent élu dont l'enfant comparaît | remplacé par son suppléant (RG-22) |
| CD-27 | Quorum non atteint | séance reportée dans 5 à 10 jours, délibère quel que soit le nombre (RG-23) ; nouvelles convocations |
| CD-28 | Recommandé non réclamé | la date de **première présentation** fait foi [NV, Q-DS2] ; remise en main propre proposée en complément |
| CD-29 | Adresse protégée d'un parent (ordonnance de protection) | recommandé à l'adresse due choisie par le socle 4 ; jamais l'adresse sur une pièce lue par l'autre parent (04 C7) |
| CD-30 | L'élève devient **majeur** pendant la procédure | pièces suivantes à lui seul ; appel par lui (RG-44) ; parents informés selon Q-DS8 ; signalé au décideur |
| CD-31 | Élève **émancipé** | traité comme majeur (C. civ. 413-6, VERIF R13) |
| CD-32 | Parents séparés, un seul titulaire présent à l'entretien | l'information préalable est due **aux deux** titulaires (R-DS12) ; la présence d'un seul suffit à l'entretien |
| CD-33 | Faits faisant l'objet de **poursuites pénales** contestées | procédure suspendue (RG-29), date de reprise suivie |
| CD-34 | Comportement menaçant d'un **parent** | RG-41 : hors échelle ; dossier transmis à la direction et au DASEN (public) |
| CD-35 | Atteinte à la **laïcité** | DASEN peut prononcer à la place du chef (RG-09) ; procédure exportable |
| CD-36 | Faits délictueux (violences, racket) | rappel article 40 (RG-42) ; trace de l'avis **hors** dossier disciplinaire (pièce restreinte, domaine `justice` du 07) |
| CD-37 | Élève de 15 ans exclu définitivement | information immédiate de l'autorité, classe relais possible (RG-28) ; le 3 ne clôt la scolarité qu'à la date d'effet |
| CD-38 | Appel au recteur formé le 9ᵉ jour | enregistré « hors délai » (l'autorité juge) ; échéances affichées |
| CD-39 | Décision réformée en appel | courriers marqués, effets recalculés, effacement recalculé (§6.4) |

### 17.3 Exécution, effacement, données

| # | Cas | Réponse |
|---|---|---|
| CD-40 | Exclusion temporaire pendant un **examen blanc** ou un **bac** | conflit signalé ; examen national : hors pouvoir de l'établissement d'en priver l'élève [NV, Q-DS19] ; convocation maintenue par défaut |
| CD-41 | Professeur qui veut « priver de sortie » un élève | une sortie **obligatoire** ne peut servir de punition [R] ; facultative : décision de la direction, motivée, jamais automatique (42) |
| CD-42 | Interne exclu de l'internat un mardi | exclusion d'un **service annexe** (RG-01) : appel du soir (41) attend « exclu », famille prévenue de reprendre l'élève, cours maintenus |
| CD-43 | Demi-pensionnaire exclu de la cantine | repas non servis et non facturés [H, 40] ; la famille est prévenue de la prise en charge du midi |
| CD-44 | Sanction prononcée le 30 juin, effacement « fin d'année » | avertissement effacé à la fin de **cette** année : effacement quasi immédiat ; affiché à la décision |
| CD-45 | Élève qui redouble ou change de cycle | effacement suit les années scolaires, pas le niveau |
| CD-46 | Élève qui quitte le lycée en terminale (fin du second degré) | toutes les sanctions effacées (RG-05) |
| CD-47 | Élève qui change d'établissement et **demande** l'effacement | demande tracée ; décision dans deux mois, silence = acceptation (RG-06) ; exclusion définitive exclue |
| CD-48 | Recours contentieux en cours à la date d'effacement | **gel** : effacement reporté (07 D53) |
| CD-49 | Sanction effacée citée dans une appréciation | avertissement à l'auteur (21 CE-93) |
| CD-50 | Export complet de l'établissement (portabilité, D-041) | sanctions effacées **exclues** de la vue métier ; archives exportées dans leur catégorie, double validation |
| CD-51 | Demande d'accès RGPD d'un parent | extraction sans les autres élèves ni les témoins anonymisés (RGPD 15 §4) |
| CD-52 | Fusion de deux fiches élève avec des sanctions | sanctions réunies, effacement recalculé sur le conservé (02 E6) |
| CD-53 | Réquisition judiciaire du dossier | gel, export tracé, décision de la direction ; hors ScolIA pour la transmission |

### 17.4 Harcèlement, protection

| # | Cas | Réponse |
|---|---|---|
| CD-54 | Élève cible qui demande que ses parents ne soient pas informés | information des parents « sous réserve de l'intérêt supérieur de l'enfant » (logique de R470-2) ; décision motivée du référent et de la direction, tracée ; transmission au 36 si danger [NV, Q-DS20] |
| CD-55 | Signalement anonyme (grille R470-3 ou formulaire) | situation ouverte sans signalant ; action de sensibilisation si non nominatif (RG-38) |
| CD-56 | Auteur présumé et cible dans la même classe | éloignement publié sans motif (5, 10) ; consigne aux professeurs ; changement de classe proposé à la direction |
| CD-57 | Situation où la cible devient auteur (conflit réciproque) | rôles multiples dans la situation, une suite par élève |
| CD-58 | Situation de harcèlement **sexuel** ou violence sexiste | catégorie sensible ; transmission 36 ; rappel 434-3 (RG-42) ; accès encore plus restreint |
| CD-59 | Le référent harcèlement est absent longtemps | référent suppléant (mission 06) ; aucune situation orpheline (R-DS28) |
| CD-60 | Une famille publie l'incident sur un réseau social | rien au socle 13 ; la direction peut ouvrir un incident « en ligne » ; communication hors ScolIA |

---

## 18. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Met au propre** un rapport dicté ou tapé vite (N1 brouillon), sans ajouter de fait ; marque ce qui est une interprétation.
- **Propose le texte famille** à partir du texte interne, sans nom d'autre élève, ton neutre, faits datés (N1) ; le contrôle de contenu repasse dessus.
- **Prépare** un avis de punition ou un courrier (N3 : aperçu, destinataires calculés, ré-authentification).
- **Liste ce qui manque** d'une procédure (pièces, délais, destinataires, mention du droit de se taire) — **d'abord déterministe** (aucun modèle), l'IA ne fait que reformuler.
- **Résume** un dossier pour la direction (faits, chronologie, mesures déjà tentées) en C2 pseudonymisé.
- **Repère des incohérences** (date d'un fait postérieure à la saisie, élève majeur traité comme mineur, sursis ignoré, catégorie obligatoire non signalée).
- **Traduit** une pièce pour une famille allophone, marquée « traduction automatique, le texte français fait foi ».

### 18.2 Ce que l'IA ne fait pas [R, IA-P12]
- Ne **propose aucune sanction**, aucune durée, aucune punition par défaut ; ne qualifie pas un élève (« violent », « à risque ») ; ne calcule aucun score.
- Ne lit **aucune** donnée de santé, de besoins particuliers détaillés, de situation de harcèlement ni de protection de l'enfance (C3/C4) ; les **situations** du §14 sont **fermées à l'IA** ; les textes confidentiels du suivi (`cpe`) aussi.
- Ne lit **aucune sanction effacée** (index purgé à l'effacement, événement consommé par le 30).
- N'envoie rien, ne signe rien, ne prononce rien ; **engager**, **décider**, **mesure conservatoire**, **révoquer un sursis**, **annuler** sont **fermés à l'IA** (⛔ du §19).

### 18.3 Capacités déclarées (registre `ent-ia`)

| Capacité | Type | Droit | Classe | Niveau |
|---|---|---|---|---|
| `discipline.incident.lire` | lecture | `discipline.incidents.lire` | C2 | — |
| `discipline.eleve.synthese` | lecture | portée du lecteur | C2 | — |
| `discipline.procedure.manques` | lecture (déterministe) | `discipline.procedures.lire` | C2 | — |
| `discipline.rapport.mettre_au_propre` | brouillon | auteur | C2 | N1 |
| `discipline.texte_famille.proposer` | brouillon | `discipline.incidents.traiter` | C2 | N1 |
| `discipline.punition.preparer` | commande | `punitions.donner` | C2 | **N3** |
| `discipline.avis.preparer` | commande (envoi) | selon nature | C2 | **N3** |
| `discipline.observation.preparer` | commande | `discipline.observer` | C2 | N2 |
| engagement, décision, conservatoire, sursis, annulation, situations, suivis `cpe` | — | — | — | **fermé** |

### 18.4 Cadre
Tests de garde (`IA_COEUR.md` §5.4) : équivalence écran / IA ; isolation entre établissements ; refus d'une demande « propose une sanction » (test de jeu d'évaluation) ; absence de toute sanction effacée dans le contexte ; pseudonymisation des élèves cités. Jeu d'évaluation du socle : 30 rapports fictifs (dictées bruitées, noms d'autres élèves à retirer, insultes à citer entre guillemets). Étude juridique (règlement européen sur l'IA) avant mise en service des fonctions à modèle de langage, comme 21 D-032.

---

## 19. Actions métier (DIS-100 à DIS-279)

DIS-01 à DIS-07 sont les lignes de la matrice Charlemagne ; VS-14 à VS-18 de `SOCLE2_DOMAINES.md` sont reprises ici. Colonnes : **Qui** (rôle par défaut · action D-017 du §21.1), **Règles**, **Événement**, **Lot**. ★ = action N3 si préparée par l'IA ; ⛔ = fermée à l'IA.

### 19.1 Paramétrage et référentiels

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-100 | Choisir le **profil de procédure** (EPLE, privé sous contrat, privé hors contrat) | DIR · `discipline.parametrer` | imposé à `eple` pour un établissement public (05 profil) ; changement tracé | `discipline.profil_modifie` | D1 |
| DIS-101 | Tenir le catalogue des **catégories de faits** (comportement, violence verbale, physique, atteinte aux biens, harcèlement, téléphone, fraude, intrusion, usage de substances, autre) | DIR, CPE · `discipline.parametrer` | catégories ScolIA fixes (codes) + libellés et sous-catégories de l'établissement ; **drapeau légal** « engagement obligatoire » posé par ScolIA, non modifiable (R-DS11) | — | D1 |
| DIS-102 | Tenir le catalogue des **punitions** (nature fixe, libellé, durée par défaut, besoin d'une séance) | DIR, CPE · `discipline.parametrer` | natures interdites refusées (R-DS2) ; règlement intérieur cité | — | D1 |
| DIS-103 | Tenir le catalogue des **mesures** de prévention, d'accompagnement, de réparation | idem | — | — | D8 |
| DIS-104 | Tenir l'**annuaire des structures** partenaires de responsabilisation | DIR · `discipline.parametrer` | coordonnées professionnelles seulement | — | D4 |
| DIS-105 | Définir la **composition** du conseil de discipline et de la commission éducative à la date | DIR · `discipline.instances.gerer` | EPLE : composition du code contrôlée ; lecture du socle 46 si présent | `discipline.instance_composee` | D5, D8 |
| DIS-106 | Gabarits de **séances de retenue** (jour, heure, salle, surveillant, capacité) | CPE · `discipline.retenues.gerer` | salle réservée au 10 | — | D2 |
| DIS-107 | Activer et régler les **règles de vigilance** (préréglages) | CPE, DIR · `discipline.parametrer` | seuils bornés ; aucune règle ne sanctionne (R-DS14) | — | D10 |
| DIS-108 | Choisir les **trames** des courriers (3 ou 4 par nature) et les textes encadrés | DIR · `documents.trames` | D-023 | — | D3 |
| DIS-109 | Indiquer le **règlement intérieur** en vigueur (document 7) auquel renvoient punitions et sanctions | DIR | version datée ; prise de connaissance des familles (07 S0) | — | D1 |
| DIS-110 | Désigner les **référents** (CPE par niveau, référent harcèlement, équipe pHARe) | DIR · missions (06) | lu par la file et les situations | — | D1, D9 |

### 19.2 Observations et incidents

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-120 | **Observation** rapide (positive ou négative) sur un ou plusieurs élèves, depuis l'appel, la fiche, le mobile | PRF · `discipline.observer` (ses cours) ; CPE, AED (établissement) | une observation par élève ; texte court ; visible famille au choix de l'auteur, défaut **non** pour une observation négative [R, S-1308] | `discipline.observation_faite` | D1 |
| DIS-121 | Corriger ou retirer sa propre observation (24 h), avec trace | auteur | après 24 h : CPE seulement | — | D1 |
| DIS-122 | **Déclarer un incident** : faits, date et heure, lieu (salle, cour, internat, sortie, transport, en ligne), séance, catégories, gravité, personnes et rôles, mesures immédiates | PRF · `discipline.incidents.declarer` (ses cours ; hors cours : fait constaté, R-DS20) ; CPE, AED, DIR, AESH (signalement) | texte interne ; contrôle de contenu ; personnes **choisies dans la base** ; adulte extérieur décrit sans compte | `discipline.incident_declare` (identifiants seulement) | D1 |
| DIS-123 | **Signaler** un fait concernant un élève hors de ses cours (transmis à la vie scolaire) | tout personnel | le signalant ne voit que l'état de son signalement | `discipline.incident_declare` | D1 |
| DIS-124 | Ajouter une **personne** (auteur présumé, victime, témoin…) ou changer son rôle | CPE, DIR · `discipline.incidents.traiter` | historique des rôles ; un rôle ne s'efface pas, il se clôt | `discipline.implique_modifie` | D1 |
| DIS-125 | Joindre des **pièces** (photo, capture, témoignage écrit, rapport) | déclarant, CPE | document 7 domaine `disciplinaire` ; capture d'écran **gelée** | — | D1 |
| DIS-126 | Demander un **rapport circonstancié** à un adulte témoin | CPE, DIR | tâche à échéance ; relance | `discipline.rapport_demande` | D1 |
| DIS-127 | Recueillir un **témoignage** d'élève (écrit ou oral résumé), avec option « anonyme pour les familles » | CPE, DIR | décision d'anonymisation motivée (§12.4) | — | D1 |
| DIS-128 | **Prendre en charge** un incident (file de traitement) | CPE, AED, DIR | une prise en charge par élève impliqué | `discipline.incident_pris_en_charge` | D1 |
| DIS-129 | Décider la **suite pour chaque élève** (aucune, observation, mesure, punition, commission, demande de procédure, transmission) | CPE, DIR | individualisation (R-DS1) | `discipline.suite_decidee` | D1 |
| DIS-130 | **Clore** l'incident (toutes les suites décidées) ou le **classer sans suite** (motif) | CPE, DIR | — | `discipline.incident_clos` | D1 |
| DIS-131 | Rattacher l'incident à une **sortie** (42), à l'**internat** (41), à la **cantine** (40), à un **stage** (39) | déclarant | lieu typé | — | D1 |
| DIS-132 | Indiquer qu'un élève a été **vu à l'infirmerie** à l'occasion (lien sans contenu) | CPE, INF | aucune donnée de santé (R-DS7) | — | D1 |
| DIS-133 | Fusionner deux incidents déclarés en double | CPE | rôles réunis, trace | — | D1 |
| DIS-134 | Retour au **déclarant** : voir l'état de son rapport | déclarant | état seulement (R-DS21) | — | D1 |
| DIS-135 | Incident **grave** : avis immédiat à la direction ; liste des démarches (police, famille de la victime, rectorat) | DIR | catégories « immédiates » ; aucune démarche envoyée automatiquement | `discipline.incident_grave` | D1 |
| DIS-136 | **Confisquer** un objet (téléphone…) et en tracer la remise | PRF, CPE, AED | L511-5 ; usage médical jamais sanctionné (34 R-S51) ; remise à l'élève ou à la famille selon le règlement ; rappel en fin de journée | `discipline.objet_confisque`, `_rendu` | D2 |
| DIS-137 | Rédiger les faits **à l'intention de la famille** (texte famille) à partir du texte interne | CPE, DIR | contrôle de contenu bloquant (R-DS6) ; IA : proposition N1 | — | D1 |

### 19.3 Punitions, retenues, exclusions de cours

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-140 | **Donner une punition** à un élève (type, motif, échéance, travail) ★ | PRF · `punitions.donner` (ses cours ; fait constaté hors cours) ; CPE, AED, DIR | catalogue ; interdits (R-DS2) ; une par élève | `discipline.punition_donnee` | D2 (D0 pour la reprise) |
| DIS-141 | Même punition à **plusieurs élèves** (une par élève, motif personnel) | idem | pas de punition collective (R-DS2) | idem | D2 |
| DIS-142 | **Affecter** une retenue à une séance (ou laisser la vie scolaire choisir) | donneur, CPE | conflits (cours, sortie, stage, examen, exclusion, régime) | `discipline.retenue_planifiee` | D2 |
| DIS-143 | Créer, modifier, **annuler** une séance de retenue | CPE · `discipline.retenues.gerer` | report proposé aux élèves affectés | `discipline.seance_retenue_*` | D2 |
| DIS-144 | **Appel de retenue** (présent, absent, retard, excusé ; travail rendu) | surveillant de la séance, AED, CPE | mobile, hors ligne court | `discipline.retenue_effectuee` / `_manquee` | D2 |
| DIS-145 | Suite d'une **retenue manquée** (report, autre punition, entretien) | CPE | proposée, jamais automatique (R-DS15) | — | D2 |
| DIS-146 | Marquer une punition **effectuée** (devoir rendu, excuse faite) | donneur, CPE | — | `discipline.punition_effectuee` | D2 |
| DIS-147 | **Annuler** une punition (erreur) avec motif ; **lever** une punition | donneur (24 h), CPE | avis corrigé (corrige VD-11) | `discipline.punition_annulee` | D2 |
| DIS-148 | **Exclure ponctuellement d'un cours** depuis l'appel (heure, motif) | PRF de la séance | état « exclu de cours, pris en charge » ; suivi d'arrivée (§8.5) | `discipline.exclusion_cours` | D2 |
| DIS-149 | Confirmer l'**arrivée** à la vie scolaire d'un élève exclu de cours | AED, CPE | alerte si absente après S-1306 | `discipline.exclusion_cours_arrivee` | D2 |
| DIS-150 | Rédiger le **rapport** d'exclusion de cours | PRF | rappel en fin de journée | — | D2 |
| DIS-151 | Voir ses punitions données (professeur) et leur exécution | PRF | les siennes seulement | — | D2 |
| DIS-152 | **Planning des retenues** de la semaine (vie scolaire) et liste du surveillant | CPE, AED | — | — | D2 |
| DIS-153 | Prévenir les familles de la date de retenue (avis) ; relance la veille [R] | système | 8 : U1 enveloppe | — | D2 |
| DIS-154 | Imprimer la **liste de retenue** et la feuille d'appel de secours | AED | sans motif | — | D2 |

### 19.4 Procédure disciplinaire (chef d'établissement)

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-160 | **Demander** l'engagement d'une procédure (proposition motivée) | CPE, PRF (via rapport), ADJ | tâche pour la direction | `discipline.procedure_demandee` | D3 |
| DIS-161 | **Engager** une procédure pour **un** élève, rattacher les incidents, rédiger les faits reprochés ⛔ (IA : aide à la rédaction N1 seulement) | DIR · `discipline.procedures.engager` | une procédure par élève ; contrôle de contenu ; cas obligatoires (R-DS11) | `discipline.procedure_engagee` | D0 (gardes), D3 |
| DIS-162 | **Renoncer** motivément à engager une procédure dans un cas où le texte le permet | DIR | motif obligatoire ; impossible quand l'engagement est imposé sans exception (§5) | `discipline.engagement_renonce` | D3 |
| DIS-163 | Émettre l'**information préalable** (faits, droits, délai, assistance, consultation du dossier) ★ | DIR (signature) ; SEC (mise en forme) | destinataires dus (4) ; nature 7 ; date de notification effective | `discipline.information_emise` | D0 (date), D3 |
| DIS-164 | Voir la **frise des délais** (notifiée le, décision possible à partir du, conseil possible à partir du, appel jusqu'au) | DIR, CPE, SEC | calcul serveur (R-DS10) | — | D3 |
| DIS-165 | Enregistrer les **observations** orales (compte rendu) ou écrites (dépôt), la **personne qui assiste** | DIR, CPE ; famille (dépôt en ligne) | dépôt famille daté | `discipline.defense_recue` | D3 |
| DIS-166 | Organiser la **consultation du dossier** (rendez-vous, présents, copie remise) | DIR, CPE | 07 DCT-149 | — | D3 |
| DIS-167 | Prononcer une **mesure conservatoire** (interdiction d'accès), la lever ★ ⛔ | DIR · `discipline.mesures_conservatoires` | conditions et durée du §5 ; effets 12 et 14 | `discipline.mesure_conservatoire` | D3 |
| DIS-168 | **Décider** : sanction de l'échelle (dans la compétence du chef), durée, sursis, alternative proposée, motivation ⛔ | DIR · `sanctions.prononcer` | liste « avant de pouvoir décider » ; non bis in idem ; durées max ; récidive et sursis en cours affichés | `discipline.sanction_prononcee` | D0 (réservé direction, gardes), D3 |
| DIS-169 | Décider **aucune sanction** (classement motivé) | DIR | — | `discipline.procedure_classee` | D3 |
| DIS-170 | **Saisir le conseil de discipline** | DIR | obligatoire dans les cas du §5 | `discipline.conseil_saisi` | D5 |
| DIS-171 | Émettre la **notification** de la sanction ★ | DIR (signature) | voies et délais de recours ; date d'effacement indiquée | `discipline.sanction_notifiee` | D3 |
| DIS-172 | **Abandonner** une procédure (élève parti, erreur) | DIR | motif | `discipline.procedure_abandonnee` | D3 |
| DIS-173 | Voir les procédures **en cours** de l'établissement, triées par échéance | DIR, ADJ, CPE | — | — | D3 |
| DIS-174 | Voir **sa** procédure (élève, titulaires) : faits, droits, dates, pièces qui lui sont destinées, dépôt d'observations | FAM, ELV | §15 ; jamais les autres élèves | — | D3 |
| DIS-175 | Prendre rendez-vous pour présenter ses observations (19) | FAM, ELV | — | — | D3 |
| DIS-176 | Ajouter une **personne qui assiste** (nom, qualité) et lui donner accès au dossier | FAM, ELV ; DIR | accès temporaire (07) | — | D3 |

### 19.5 Sanctions : exécution, sursis, alternatives, recours

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-180 | Publier les **effets** d'une exclusion (appel, accueil, cantine, internat, cahier de textes, évaluations) | système | §8.9 ; à la date d'effet | `discipline.exclusion_effet` | D4 |
| DIS-181 | Accueillir un élève **exclu de la classe** (lieu, adulte, travail) | CPE, AED | exclusion-inclusion (10 CL-64) | — | D4 |
| DIS-182 | Déposer le **travail** d'un élève exclu | PRF (ses cours) | via 20 | — | D4 |
| DIS-183 | Suivre une **mesure de responsabilisation** (structure, convention, heures, accord, attestation) | CPE, DIR | §10 ; accord quand le texte l'exige | `discipline.responsabilisation_*` | D4 |
| DIS-184 | Proposer et suivre une **mesure alternative** (accord, exécution, extinction de l'exclusion) | DIR, CPE | §5 | `discipline.alternative_*` | D4 |
| DIS-185 | **Révoquer un sursis** (nouvelle procédure) ⛔ | DIR | décision motivée après procédure (R-DS13) | `discipline.sursis_revoque` | D4 |
| DIS-186 | Échéance d'un **sursis** (fin) | système | rappel au décideur ; la sanction reste au dossier selon R511-13 | `discipline.sursis_echu` | D4 |
| DIS-187 | Enregistrer un **recours** (appel au recteur, recours interne privé, recours gracieux, contentieux) | DIR, SEC | date, auteur, délai vérifié ; gel (07 D53) | `discipline.recours_forme` | D5 |
| DIS-188 | Exporter le **dossier d'appel** | DIR | export tracé (pièces, PV, preuves) | — | D5 |
| DIS-189 | Enregistrer la **décision sur recours** (confirmée, réformée, annulée) et ses effets | DIR | §6.4 ; courriers marqués (07 D79) | `discipline.sanction_reformee` / `_annulee` | D5 |
| DIS-190 | **Annuler** une sanction (erreur matérielle, décision du juge) ⛔ | DIR | motif ; effets annulés ; distincte de l'effacement | `discipline.sanction_annulee` | D0, D3 |
| DIS-191 | Exclusion définitive : informer l'autorité (public) pour la réaffectation ; accompagner la famille (privé) | DIR | élève soumis à l'obligation scolaire (§5) | `discipline.exclusion_definitive_prononcee` | D5 |

### 19.6 Conseil de discipline

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-200 | Créer une **séance** (date, lieu, président) pour une ou plusieurs procédures | DIR · `discipline.conseil.gerer` | date ≥ date au plus tôt de chaque procédure | `discipline.conseil_planifie` | D5 |
| DIS-201 | Vérifier la **composition** et désigner les suppléants ; proposer les **déports** | DIR | 06 R-P6 ; membre témoin ou victime écarté (CD-25) | — | D5 |
| DIS-202 | Émettre les **convocations** (élève, représentants, personne qui assiste : recommandé ou remise contre signature ; membres, témoins : tout moyen) ★ | DIR, SEC | délai contrôlé à la remise ; bordereau | `discipline.convocation_emise` | D0 (plancher), D5 |
| DIS-203 | Saisir le **numéro de recommandé**, l'**avis de réception**, la **remise contre signature** | SEC, ACC | 07 D34 | `discipline.convocation_remise` | D5 |
| DIS-204 | Ouvrir l'**accès au dossier** des membres (temporaire, filigrané) | DIR | 07 D82 | — | D5 |
| DIS-205 | **Délocaliser** la séance ou saisir le **conseil départemental** (public) | DIR | §5 | `discipline.conseil_delocalise` | D5 |
| DIS-206 | Tenir la **séance** : présences, quorum, auditions, délibéré | président, secrétaire de séance | déroulé guidé ; tablette | — | D5 |
| DIS-207 | Enregistrer les **résultats de vote** (par proposition, comptes seulement) | secrétaire de séance | **jamais** de vote nominatif (R-DS24) | `discipline.conseil_decide` | D5 |
| DIS-208 | Produire et faire signer le **procès-verbal** | président, secrétaire | document 7 | — | D5 |
| DIS-209 | **Notifier** la décision du conseil ★ | président | voies et délais de recours | `discipline.sanction_notifiee` | D5 |
| DIS-210 | Reporter une séance (quorum non atteint, demande de la famille) | DIR | nouvelles convocations, délai recalculé | — | D5 |
| DIS-211 | Conseil de discipline **privé** (instance du règlement intérieur) | DIR | profil privé ; mêmes garanties par défaut | — | D6 |

### 19.7 Effacement et conservation

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-220 | Calculer la **date d'effacement** de chaque sanction à sa décision | système | R511-13 IV (§9.3) | — | D0 |
| DIS-221 | **Effacer** les sanctions échues chaque jour (tâche idempotente) | système | effacement en profondeur (R-DS25) ; reporté si gel | `discipline.sanction_effacee` | D0 |
| DIS-222 | Effacer **toutes** les sanctions au **terme de la scolarité** du second degré | système (sur sortie du 3 / 2) | sauf gel | idem | D0 (règle), D7 |
| DIS-223 | Enregistrer une **demande d'effacement** au changement d'établissement et l'exécuter | ELV majeur, titulaires ; DIR | R511-13 IV | idem | D7 |
| DIS-224 | Voir les effacements **à venir** et faits (preuve) | DIR, DPO | sans contenu effacé | — | D7 |
| DIS-225 | **Geler** un dossier (recours, contentieux, réquisition) et lever le gel | DIR | 07 D53 ; motif, revue | `discipline.gel_*` | D7 |
| DIS-226 | Verser les pièces à **archiver** (PV, dossier contentieux) selon la règle d'archives | système, DIR | 27 ; visa d'élimination (VERIF R40) | — | D7 |
| DIS-227 | Purger les **observations**, punitions, suivis, situations à leur durée (21.3) | système | durées validées par le DPO | — | D7 |
| DIS-228 | Attestation d'effacement (à la demande) | DIR | — | — | D7 |

### 19.8 Suivi éducatif, commission éducative, vigilance

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-230 | **Ouvrir un suivi éducatif** (motif codé, objectifs, référent, échéance, équipe) | CPE, PP, DIR · `discipline.suivi.gerer` | pastille pour l'équipe | `discipline.suivi_ouvert` | D8 |
| DIS-231 | Ajouter une **entrée** (entretien, observation, point d'étape, bilan) avec sa visibilité | membres du suivi | `cpe` / `equipe` / `famille` | — | D8 |
| DIS-232 | Faire signer un **engagement** (élève, titulaires) | CPE | document 7 | — | D8 |
| DIS-233 | **Clore** un suivi (bilan) | référent | — | `discipline.suivi_clos` | D8 |
| DIS-234 | Planifier un **entretien** avec la famille (19) | CPE, PP | convocation `vie_scolaire.convocation_entretien` (08) | — | D8 |
| DIS-235 | **Saisir la commission éducative** pour un élève | DIR (sur proposition) | elle ne sanctionne pas (R-DS17) | `discipline.commission_saisie` | D8 |
| DIS-236 | Tenir une **séance** de commission (cas un par un, propositions, suivi) | président | familles à horaires distincts | `discipline.commission_tenue` | D8 |
| DIS-237 | Suivre les **mesures** proposées par la commission (échéances, bilan) | CPE | — | — | D8 |
| DIS-238 | Recevoir une **alerte de vigilance** avec ses raisons ; la prendre en charge, la classer | CPE, PP (référent) | jamais visible de la famille (R-DS14) | `discipline.alerte_*` | D10 |
| DIS-239 | **Suivre un élève** (avis internes de ses punitions et incidents, durée limitée) | PP, CPE | 08 NTF-153 | — | D10 |
| DIS-240 | Voir la **synthèse vie scolaire** d'un élève pour le conseil de classe | PP, DIR, PRF (fenêtre du conseil, 21 D-032) | §6.3 ; non effacé seulement | — | D10 |
| DIS-241 | Voir le **tableau de bord** de la vie scolaire (incidents du jour, à traiter, retenues, procédures à échéance, suivis à bilan) | CPE, DIR | tuiles Bento | — | D10 |

### 19.9 Harcèlement

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-250 | **Signaler** une situation de harcèlement (élève, famille, personnel, témoin) | tous les comptes ; formulaire élève discret | accusé de réception ; 3018 affiché | `discipline.situation_signalee` (sans nom) | D9 |
| DIS-251 | Ouvrir une **situation** depuis un incident, un message signalé (16), un questionnaire | référent | — | idem | D9 |
| DIS-252 | Désigner les **personnes** (cible, auteurs présumés, témoins) et les **référents** | référent | cloisonné (R-DS23) | — | D9 |
| DIS-253 | Suivre les **étapes** du protocole avec échéances (entretiens, information des familles, points de suivi) | référents | protocole de l'établissement (pHARe préréglé dans le public) | `discipline.situation_etape` | D9 |
| DIS-254 | Poser une **mesure d'éloignement** (classe, groupe, cantine, sortie, chambre, transport) | référent, DIR | publiée sans motif (5, 10, 42) | `discipline.eloignement_pose` | D9 |
| DIS-255 | Informer la **famille** de l'élève cible des mesures prises pour lui | référent | sans nom de l'auteur sauf nécessité (§5) | — | D9 |
| DIS-256 | **Transmettre** au socle 36 (information préoccupante) ou tracer un **avis au procureur** | DIR | jamais automatique ; trace | `discipline.situation_transmise` | D9 |
| DIS-257 | Ouvrir les **suites disciplinaires** contre un auteur (incident, procédure) | DIR | une par auteur | — | D9 |
| DIS-258 | **Clore** une situation (résolue, transmise, non confirmée) avec bilan à échéance | référent | point de suivi après clôture [R] | `discipline.situation_close` | D9 |
| DIS-259 | Bilan anonyme des situations (pilotage) | DIR | seuil de secret | — | D10 |

### 19.10 Lecture, exports, système, IA

| # | Action | Qui | Règles | Événement | Lot |
|---|---|---|---|---|---|
| DIS-260 | Section **Discipline** de la fiche élève (360°) selon le lecteur | selon §21.2 | pastilles, titres neutres, effacé invisible | — | D1 |
| DIS-261 | Chronologie : entrées neutres, filtrées, effacées retirées | système | corrige DV-04 | — | D0 |
| DIS-262 | **Recherche** et filtres (élève, classe, période, catégorie, état, lieu) | CPE, DIR | portée | — | D1 |
| DIS-263 | **Export** nominatif (dossier d'un élève, liste de punitions) | DIR · `discipline.exporter` | tracé ; jamais un professeur (D-022) ; effacé exclu | — | D10 |
| DIS-264 | **Indicateurs** sans nom (47) ; enquêtes officielles sur les faits graves (public) [NV] | DIR | seuil de secret ; Q-DS12 | — | D10 |
| DIS-265 | Droit d'accès RGPD : extraction du dossier disciplinaire d'un élève **sans** données d'autres élèves | DIR, DPO | RGPD 15 §4 | — | D7 |
| DIS-266 | **IA** : mettre au propre un rapport dicté ou écrit vite | auteur | N1 brouillon ; §18 | — | D12 |
| DIS-267 | **IA** : proposer le texte famille à partir du texte interne (sans nom d'autre élève) | CPE, DIR | N1 ; contrôle de contenu | — | D12 |
| DIS-268 | **IA** : liste « ce qui manque » d'une procédure (pièces, délais, destinataires) | DIR | N0, déterministe d'abord | — | D12 |
| DIS-269 | **IA** : préparer un avis de punition ou un courrier ★ | auteur | N3 ; relu | — | D12 |
| DIS-270 | **IA** : résumé d'un dossier pour la direction (faits, chronologie, sans santé) | DIR | N0 ; C2 pseudonymisé | — | D12 |
| DIS-271 | **IA** : repérer incohérences (dates, rôles, élève majeur, sursis en cours) | DIR, CPE | N0 | — | D12 |
| DIS-272 | **Mobile** : observation, incident, punition, exclusion de cours depuis l'appel ; appel de retenue ; avis famille | PRF, AED, FAM, ELV | socle 11 ; hors ligne court (appel de retenue) | — | D11 |
| DIS-273 | **Reprise** des anciennes tables (P0 et S2-4) dans la brique | système | §26 | — | D13 |
| DIS-274 | Saisie d'une sanction **prononcée hors ScolIA** (année de doublon, historique Charlemagne) | DIR | mode « enregistrée », date de décision et autorité, effacement calculé, aucun courrier | `discipline.sanction_enregistree` | D13 |
| DIS-275 | Journal des **accès** au dossier disciplinaire d'un élève (qui a lu) | DIR, DPO ; famille (A11, à terme) | — | — | D7 |
| DIS-276 | Transfert d'information au **nouvel établissement** (sur demande, sanctions non effacées seulement, jamais les incidents) [R] | DIR | base légale à confirmer (Q-DS14) | — | D7 |
| DIS-277 | Aide en ligne : **« vos droits »** en langage simple pour les familles et l'élève à chaque pièce | système | — | — | D3 |
| DIS-278 | Traduction de l'information préalable (familles allophones) avec mention « traduction automatique, le texte français fait foi » | système | 16 CM-77 | — | D12 |
| DIS-279 | Vue **accueil** : élèves exclus ou sous mesure conservatoire aujourd'hui (accès interdit), sans motif | ACC | 14 | — | D4 |

**Total : 136 actions** (DIS-100 à DIS-279 ; les numéros sautés sont libres pour les ajouts).

---

## 20. Règles transverses (R-DS1 à R-DS32) [R, sauf mention]

| # | Règle |
|---|---|
| R-DS1 | **Individualisation** : une punition, une procédure, une sanction concerne **un** élève ; un incident à plusieurs produit autant de suites que d'élèves. |
| R-DS2 | **Interdits codés** : aucune nature de punition « zéro », « lignes », « punition collective » ; une évaluation ne sert jamais de punition (21 R-02) ; aucune pénalité financière (R-DS16). Catalogue refusé s'il en contient. [V-ext, circulaire 2014-059, §5] |
| R-DS3 | **Non bis in idem** : un même fait (incident × élève) ne peut fonder deux sanctions ; une punition déjà exécutée pour un fait est **signalée** au décideur d'une sanction pour ce même fait [à vérifier, §5]. |
| R-DS4 | **Légalité** : une sanction est toujours une catégorie de l'**échelle fermée** (pas de type libre) ; une punition appartient au catalogue tiré du règlement intérieur. |
| R-DS5 | **Deux textes** : faits internes (personnels habilités) et texte famille (rédigé pour la famille, sans nom d'autre élève). Le texte interne n'est **jamais** servi aux familles ni à l'élève (corrige DV-06). |
| R-DS6 | **Contrôle de contenu** (07 R-D11) : bloquant pour un texte famille, avertissement pour un texte interne. |
| R-DS7 | **Aucune donnée de santé** dans le socle 13 ; un lien « vu à l'infirmerie » sans contenu ; pastille « besoins particuliers » sans contenu (35). |
| R-DS8 | **Motivation obligatoire** de toute sanction, de tout classement, de toute mesure conservatoire, de tout renoncement. |
| R-DS9 | **Échelle et compétence** : le serveur refuse une sanction hors de la compétence de l'auteur (chef seul / conseil, §5) et une durée au-delà du maximum. |
| R-DS10 | **Délais calculés par le serveur** à partir de faits horodatés (notification effective, remise, présentation), en jours ouvrables du calendrier de l'établissement ; la décision et la séance sont **refusées** avant la date au plus tôt ; une date antidatée est impossible (horodatage serveur), une date saisie *a posteriori* (« entretien tenu hier ») est marquée « déclarée » et tracée. |
| R-DS11 | **Engagement obligatoire** : les catégories de faits que le code désigne (§5) créent une tâche bloquante pour la direction (engager, ou renoncer motivément quand c'est permis, ou saisir le conseil quand c'est imposé). |
| R-DS12 | **Information due aux deux parents** : toute information d'une sanction part aux destinataires calculés par le socle 4 (titulaires, parent sans exercice pour information, jamais une personne interdite) ; élève majeur : §5 et Q-DS8. |
| R-DS13 | **Sursis** : sa révocation est une **décision nouvelle** après procédure ; jamais automatique à la punition suivante. |
| R-DS14 | **Vigilance ≠ sanction** : une règle de vigilance n'a pour effet qu'une alerte à un adulte ; pas de score affiché, pas de classement, pas de « permis à points » qui déclenche une sanction. |
| R-DS15 | **Pas d'escalade automatique** : retenue manquée, retards cumulés, observations répétées → **proposition** à la CPE, jamais une punition ou une sanction créée seule. |
| R-DS16 | **Pas de pénalité financière** ; la réparation d'un dommage matériel par la famille relève d'une démarche distincte (assurance, responsabilité civile), hors de l'échelle (Q-DS9). |
| R-DS17 | **La commission éducative ne sanctionne pas** ; ses propositions sont des mesures éducatives ; une sanction ne naît que d'une procédure. |
| R-DS18 | **Absence ≠ discipline** : l'absentéisme relève d'abord du socle 12 (commission éducative, engagement, signalement) ; une procédure pour manquement à l'assiduité est possible (RG-43) mais jamais automatique, et une exclusion proposée pour absentéisme est **signalée** au décideur (« l'exclusion aggrave l'absence ») [R]. |
| R-DS19 | **État attendu** : un élève exclu, sous mesure conservatoire, en retenue ou exclu de cours n'est pas compté absent par l'appel ; l'appel affiche l'état sans motif. |
| R-DS20 | **Portée du professeur** : ses cours (D-022) pour observer, punir, déclarer ; **hors cours**, il peut punir ou déclarer un **fait qu'il a constaté** lui-même (couloir, sortie) sur tout élève, l'auteur étant tracé ; il ne **lit** jamais l'historique d'un élève hors de ses cours. |
| R-DS21 | **Retour au déclarant** : état du traitement visible (pris en charge, suite donnée en un mot), jamais le détail d'une procédure ni ce qui concerne d'autres élèves. |
| R-DS22 | **Confidentialité du suivi** : les entrées `cpe` ne sont lues que par leur auteur et la direction sur motif tracé ; les entrées `equipe` par les membres du suivi ; les entrées `famille` par la famille. |
| R-DS23 | **Cloisonnement du harcèlement** : une situation est lue par ses référents et la direction ; les autres adultes reçoivent des **consignes** sans motif ; jamais le nom du signalant à la famille d'un auteur présumé. |
| R-DS24 | **Vote secret** : seuls les comptes par proposition sont enregistrés. |
| R-DS25 | **Effacement en profondeur** : à l'effacement, la sanction disparaît de toute lecture courante (fiche, chronologie, conseil de classe, bulletin, IA, statistiques nominatives, exports, avis futurs) ; son texte libre, celui de sa procédure et de ses courriers visibles est vidé ou déplacé hors de la vue courante ; l'existence (catégorie, dates) est gardée pour la preuve, lisible du seul DPO et de la direction sur motif tracé ; les pièces d'archives (PV) suivent la règle d'archives ; le **gel** reporte l'effacement. |
| R-DS26 | **Annulation ≠ effacement** : une sanction annulée (recours, erreur) cesse ses effets et n'est plus servie ; son existence est gardée au journal. |
| R-DS27 | **Ce qu'on a saisi reste à soi** : un professeur garde la lecture de ses propres saisies (observations, punitions, rapports) après le départ de l'élève de son cours, **sans** les suites. |
| R-DS28 | **Destinataires par la fonction, pas par le nom** : un avis à la vie scolaire va à la boîte de service (16) et aux référents (06), pas à une personne absente. |
| R-DS29 | **Mineur protégé** : un élève n'est jamais destinataire d'une pièce qui nomme un autre élève ; un élève victime n'est pas informé des sanctions d'un autre au-delà de ce que §5 permet. |
| R-DS30 | **Journal en ajout seul** de toute commande du socle (acteur, origine écran ou `ia:<brouillon>`, motif) ; instantané dans `ent_journal_modif` (exception D-036 existante). |
| R-DS31 | **Tout en transaction** : fait métier, journal, événement outbox, émission de document et avis dans la même transaction (07 §5.3, 08 Q-N1). |
| R-DS32 | **Public / privé** : le profil `eple` applique le code sans réglage ; le profil privé part des mêmes valeurs ; tout écart en dessous est une décision motivée et signalée (S-1301). |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]

| Action | Portées admises | Modèles par défaut |
|---|---|---|
| `discipline.observer` | ses cours, établissement | Professeur (ses cours), CPE, AED, Direction (établissement) |
| `discipline.incidents.declarer` | ses cours + fait constaté, établissement | Professeur, AESH (signalement), CPE, AED, Direction |
| `discipline.incidents.traiter` | établissement, niveau | CPE, Direction ; AED (prise en charge sans décision) |
| `discipline.incidents.lire` | ses cours (sans suites), établissement | Professeur (ses saisies et faits de ses cours), CPE, AED, Direction |
| `punitions.donner` (existante) | ses cours + fait constaté, établissement | Professeur, CPE, AED, Direction |
| `discipline.retenues.gerer` | établissement | CPE, AED (appel) |
| `discipline.procedures.engager` | établissement | Direction (chef, adjoint) |
| `sanctions.prononcer` (existante, **resserrée**) | établissement | **Direction seule** (retiré de CPE et Administratif, DV-01) |
| `discipline.procedures.lire` | établissement | Direction, CPE (en lecture), Secrétariat (pièces à mettre en forme, sans les faits internes) |
| `discipline.mesures_conservatoires` | établissement | Direction |
| `discipline.conseil.gerer`, `discipline.instances.gerer` | établissement | Direction |
| `discipline.suivi.gerer` | mes élèves suivis, établissement | CPE, PP (ses élèves), Direction |
| `discipline.harcelement.gerer` | établissement | Référents (mission), Direction |
| `discipline.alertes.lire` | mes élèves, établissement | CPE, PP (ses élèves) ; **jamais** comptabilité, infirmerie, psychologue par défaut (DV-09) |
| `discipline.parametrer` | établissement | Direction, CPE |
| `discipline.exporter` | établissement | Direction (jamais professeur, D-022) |
| `discipline.effacement.voir` | établissement | Direction, DPO (preuve sans contenu) |

Familles et élèves : hors rôles (D-017), droits issus du lien (04) et de `peut_voir_vs`.

### 21.2 Ce que voit chacun (projections) [R]

| Lecteur | Incidents | Punitions | Procédures, sanctions | Suivi, harcèlement |
|---|---|---|---|---|
| Professeur (élève de ses cours) | ses saisies ; faits survenus dans ses cours (texte interne **sans** les autres élèves s'il n'en est pas l'auteur [R]) | les siennes ; « retenue prévue le … » des autres sans motif | pastille « exclu jusqu'au … » ; sanctions non effacées **pendant la fenêtre du conseil** (21 D-032), sans motivation | pastille « suivi en cours » ; consignes d'éloignement |
| Professeur principal | idem + incidents de ses élèves (texte interne) | toutes celles de ses élèves | sanctions non effacées et motivation ; procédures en cours (état) | suivis de ses élèves (entrées `equipe`) |
| CPE | tout (portée) | tout | tout en lecture ; propose | tout (sauf entrées `cpe` d'un collègue sans motif) ; situations si référent |
| AED | incidents qu'il prend en charge | celles à exécuter (retenues) | état « exclu » | — |
| Direction | tout | tout | tout | tout, lecture motivée des entrées `cpe` |
| Secrétariat | — | — | pièces à mettre en forme (texte famille, destinataires, adresses dues) | — |
| Accueil | — | — | « accès interdit du … au … » | — |
| Gestionnaire | — | — | exclusions de service annexe (dates) | — |
| Infirmerie, psychologue, social | — | — | — | situation transmise (36) |
| Famille, élève | §15 | §15 | §15 | entrées `famille` ; mesures prises pour la victime |

### 21.3 Conservation [R, durées soumises au DPO]

| Donnée | Durée proposée | Source |
|---|---|---|
| Sanction au dossier administratif | jusqu'à l'effacement R511-13 IV | [V] VERIF R34 |
| Trace d'existence d'une sanction effacée (catégorie, dates) | fin de l'année de l'effacement + 1 an, puis purge [R] | preuve en cas de contestation ; DPO |
| Dossier de procédure (pièces, notifications) | retiré de la vue courante à l'effacement, **archivé** (accès restreint) jusqu'à 10 ans (tri, RG-46) ; si contentieux : gel jusqu'à son terme | éduscol 11-12 [V-ext], 07 D53 |
| Procès-verbal du conseil de discipline | **10 ans**, tri (EPLE) | instruction de tri 2005, VERIF R41 [V] ; privé : J (VERIF R39) |
| Incidents sans suite disciplinaire, observations | fin de l'année scolaire suivante [R] | DPO |
| Punitions (« cahiers de punitions ») | **1 an**, destruction | instruction de tri 2005 [V], RG-46 |
| Registre des sanctions (anonymisé, R421-20 3°) | **10 ans**, conservation | instruction de tri 2005 [V] |
| Suivi éducatif, commission éducative | fin de l'année suivant la clôture [R] | DPO |
| Situations de harcèlement | 3 ans après la clôture [R] (preuve, récidive), sauf transmission (36) | DPO, J |
| Journal des accès | 6 mois à 1 an (VERIF R43) | DPO |
| Corps des avis (SMS, e-mail) | durée du socle 8 ; aucun contenu disciplinaire hors enveloppe | 08 |

### 21.4 Indicateurs (47) et secret statistique
Agrégats par catégorie, niveau, lieu, mois : incidents, punitions, exclusions de cours, sanctions par catégorie, conseils tenus, délais moyens de traitement, situations de harcèlement ; **seuil** : aucune case inférieure à 5 élèves affichée nominativement déductible [R] ; jamais de taux par professeur (06 : indicateurs d'activité désactivés).

### 21.5 Pièges à tester
1. Parent d'un auteur présumé qui lit l'incident : ne voit ni la victime ni les témoins (texte famille seulement).
2. Professeur d'un groupe de LV2 : ne voit pas l'historique disciplinaire des élèves de l'autre groupe (D-022).
3. Comptabilité : aucune alerte, aucun incident (DV-09).
4. Sanction effacée : absente de la chronologie, du conseil de classe, de l'IA, d'un export, d'un avis renvoyé (DV-04).
5. Parent sans `peut_voir_vs` : aucun avis de punition (DV-07).
6. Établissement B : aucune ligne de A en RLS stricte (DV-08).
7. Élève majeur : les pièces lui sont adressées ; ses parents selon Q-DS8.
8. Personne interdite de communication : jamais destinataire d'une convocation.
9. Témoin anonymisé : son nom n'apparaît dans aucune pièce rendue à une famille.
10. IA : refuse de proposer une sanction ; ne lit aucune donnée C3.

---

## 22. Événements (outbox, dans la transaction du fait) [R]

| Événement | Charge (identifiants seulement) | Consommateurs |
|---|---|---|
| `discipline.observation_faite` | observation_id, eleve_id, visible_famille | 8 (avis si visible), 2 (chronologie) |
| `discipline.incident_declare` | incident_id, gravite, lieu_type | file vie scolaire, 8 (avis interne), 47 |
| `discipline.incident_grave` | incident_id | 8 (U2 direction) |
| `discipline.punition_donnee` / `_annulee` / `_effectuee` | punition_id, eleve_id, nature | 8, 2, 12 (retenue), 23 |
| `discipline.retenue_planifiee` / `_manquee` | punition_id, seance_id | 8, 12 |
| `discipline.exclusion_cours` / `_arrivee` | punition_id, seance_id, heure | 12, 14 (sûreté), 8 |
| `discipline.procedure_engagee` / `_classee` / `_abandonnee` | procedure_id, eleve_id | 8 (avis interne), 2 |
| `discipline.information_emise`, `discipline.convocation_emise` | procedure_id, document_id | 7 (déjà émis en transaction), 8 |
| `discipline.mesure_conservatoire` | eleve_id, debut, fin | 12, 14, 40, 41 |
| `discipline.sanction_prononcee` / `_notifiee` | sanction_id, eleve_id, categorie, debut, fin | 12, 14, 20, 21, 23, 40, 41, 2, 8 |
| `discipline.exclusion_definitive_prononcee` | eleve_id, date_effet | 3 (départ), 9 [NV] |
| `discipline.sanction_reformee` / `_annulee` | sanction_id | 7 (D79), 12, 14, 40, 41, 23, 2 |
| `discipline.sanction_effacee` | sanction_id, eleve_id | 2, 7 (D36), 23, 30 (index IA), 47 |
| `discipline.eloignement_pose` / `_leve` | paire d'eleve_id, portee, debut, fin | 5, 10, 42, 41 |
| `discipline.situation_signalee` / `_transmise` / `_close` | situation_id | référents (8), 36 |
| `discipline.suivi_ouvert` / `_clos` | suivi_id, eleve_id | pastilles (2, 12) |
| `discipline.alerte_creee` | alerte_id, eleve_id, regle | 8 (avis au référent) |

Types déclarés dans la brique (D-026), pas dans l'énumération partagée ; `AggregateKind` propre (corrige DV-20).

---

## 23. Notifications (natures déclarées au socle 8) [R]

Compatible avec 08 §… (`discipline.punition`, `discipline.information_prealable`, `convocation_conseil`, `notification_sanction` déjà réservées).

| Nature | Urgence | Classe | Destinataires | Canaux | Preuve |
|---|---|---|---|---|---|
| `discipline.observation` | U0 | C2 | titulaires, élève | in-app, résumé | — |
| `discipline.punition` | U1 | C2 | titulaires (+ élève) | in-app, e-mail enveloppe | ouverture |
| `discipline.retenue_rappel` | U1 | C2 | idem | in-app, push enveloppe | — |
| `discipline.exclusion_cours` | U1 | C2 | titulaires | in-app, enveloppe | ouverture |
| `discipline.information_prealable`, `convocation_conseil`, `notification_sanction`, `notification_decision_conseil`, `mesure_conservatoire` | U1 | C2 | titulaires (+ élève, + élève majeur) ; parent sans exercice pour la notification | coffre + enveloppe sur tous les canaux ; courrier si aucun | **notification effective** (08 R-N40) ; **ne vaut pas convocation** au conseil (R35) |
| `discipline.incident_interne` | U1 (U2 si grave) | C2 | référents, direction | in-app, push | — |
| `discipline.exclusion_cours_non_arrivee` | U2 | C1 | vie scolaire, professeur | push | prise en charge |
| `discipline.alerte_vigilance` | U0 | C2 | référent | in-app, résumé | — |
| `discipline.situation` | U1 | C3 | référents | in-app seulement | — |
| `discipline.accuse_signalement` | U0 | C1 | signalant | in-app | — |

Aucun motif, aucun nom d'élève, aucune catégorie de sanction dans une enveloppe (corrige DV-07) : « Une information de vie scolaire concernant Léa est disponible dans l'ENT ».

---

## 24. Écrans (maquette décrite, `DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)

| Écran | Route | Contenu |
|---|---|---|
| **Vie scolaire — aujourd'hui** | `/web/discipline` (refonte) | tuiles Bento : incidents à traiter, exclusions de cours en cours (avec temps écoulé), retenues du jour, procédures à échéance, suivis à bilan, alertes ; file de traitement à droite |
| **File des incidents** | `/web/discipline/incidents` | tableau dense (date, lieu, catégories, personnes par rôle, état par élève, pris en charge par) ; filtres ; saisie rapide |
| **Incident** | `/web/discipline/incidents/{id}` | faits (interne), personnes et rôles, une **ligne par élève** avec sa suite, pièces, journal |
| **Saisie rapide** (dialogue) | partout | observation / incident / punition en un dialogue ; élèves du cours proposés ; dictée possible (IA, N1) |
| **Retenues** | `/web/discipline/retenues` | semaine en colonnes, séances, capacité, élèves ; glisser une punition vers une séance ; appel |
| **Procédure** | `/web/discipline/procedures/{id}` | **frise des délais** en tête (notifiée, décision possible, conseil possible, recours jusqu'au) ; liste « avant de pouvoir décider » ; pièces ; défense ; décision ; courriers |
| **Conseil de discipline** | `/web/discipline/conseils/{id}` | composition, convocations et preuves (bordereau), dossier, **mode séance** sur tablette (déroulé, votes), PV |
| **Suivi éducatif** | `/web/discipline/suivis/{id}` | objectifs, entrées par visibilité, engagements, bilan |
| **Situations de harcèlement** | `/web/discipline/situations` | accès restreint ; étapes à échéances ; mesures ; transmissions |
| **Effacements** | `/web/discipline/effacements` | à venir, faits, gelés (direction, DPO) |
| **Fiche élève, section Discipline** | `/web/eleves/{id}` | selon le lecteur (§21.2) |
| **Famille et élève** | `/web/famille/vie-scolaire` | punitions, procédures en cours avec **« vos droits »** et dépôt d'observations, sanctions non effacées avec date d'effacement |
| **Accueil** | `/web/accueil` | bandeau « accès interdit aujourd'hui » (14) |

**Mobile (socle 11)** : professeur — observation, incident, punition, exclusion de cours **depuis l'appel** ; AED — appel de retenue, arrivée des exclus de cours, file du jour ; CPE — file et alertes ; famille et élève — avis, retenue à venir, procédure (lecture, dépôt d'observations par photo). Les tâches complexes (conseil, procédure) restent sur le web (D-003).

---

## 25. Modèle de données cible (brique `discipline`) [R]

**Règles appliquées à toutes les tables** (E1–E14 de `PRA_SAUVEGARDES.md` §18.4, D-036) : `id UUID` généré par l'application (E3) ; `etablissement_id UUID NOT NULL` y compris dans les tables filles (E1) ; contrainte `UNIQUE (etablissement_id, id)` et **FK composites** `(etablissement_id, x_id) → (etablissement_id, id)` vers toute ligne d'établissement (E2) ; `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()`, `modifie_le` sur les tables modifiables (E6) ; **aucun `DELETE`** d'un objet à historique : états datés (E6) ; **aucune colonne JSON/JSONB, aucun tableau** (D-036, E10) ; fichiers = documents du socle 7 (`document_id`), aucun `BYTEA` (E5) ; RLS standard (`ent_rls_poser`, 580) sur chaque table (E13) ; classement au registre des tables (E12) ; files d'effets = outbox existante (E9) ; aucun nom ni texte libre dans un journal applicatif (E14) ; textes libres bornés par `CHECK (char_length(...) <= n)`. Numéros lisibles par **compteur par établissement** (E3).

```sql
-- Profil et référentiels -------------------------------------------------
ent_dis_profil (etablissement_id PK, cadre TEXT CHECK IN ('eple','prive_sous_contrat','prive_hors_contrat','premier_degre'),
                delai_defense_jours SMALLINT NOT NULL DEFAULT 2 CHECK (>= 2 si eple), jours_mode TEXT CHECK IN ('ouvrables','ouvres'),
                delai_convocation_jours SMALLINT NOT NULL DEFAULT 5, conseil_consultatif BOOLEAN NOT NULL,   -- vrai pour le privé
                recours_interne BOOLEAN, protocole_harcelement TEXT CHECK IN ('phare','etablissement'),
                exclusion_cours_alerte_minutes SMALLINT DEFAULT 10, modifie_par, cree_le, modifie_le)
ent_dis_categorie_fait (id, etablissement_id, code_scolia TEXT CHECK IN ('comportement','violence_verbale_personnel','violence_verbale_eleve',
                'violence_physique_personnel','violence_physique_eleve','acte_grave','principes_republique','harcelement','arme',
                'atteinte_biens','vol','telephone','fraude','substances','intrusion','autre'),
                libelle, actif, ordre)               -- les drapeaux légaux sont DÉDUITS du code_scolia dans le code (RG-11, RG-12), pas stockés
ent_dis_punition_type (id, etablissement_id, nature TEXT CHECK IN ('inscription_carnet','excuse','devoir_supplementaire','retenue',
                'exclusion_cours','travail_reparation','autre'), libelle, duree_defaut_minutes, actif, ordre)
ent_dis_mesure_type (id, etablissement_id, nature TEXT CHECK IN ('prevention','accompagnement','reparation'), libelle, actif)
ent_dis_structure (id, etablissement_id, nom, nature TEXT CHECK IN ('etablissement_scolaire','association','collectivite',
                'groupement_public','administration_etat'), contact_professionnel, actif)     -- RG-03
ent_dis_compteur (etablissement_id, annee_id, serie TEXT CHECK IN ('INC','PROC','SAN','CD'), dernier INT, PK (etab, annee_id, serie))
ent_dis_journal (id, etablissement_id, objet_type TEXT, objet_id UUID, action TEXT, acteur_user_id, origine TEXT CHECK IN ('ecran','mobile','ia','systeme'),
                 ia_brouillon_id NULL, motif TEXT NULL CHECK (<= 500), cree_le)     -- ajout seul (déclencheur) ; instantané dans ent_journal_modif (exception D-036 existante)

-- Observations et incidents ----------------------------------------------
ent_dis_observation (id, etablissement_id, eleve_id, auteur_agent_id, seance_id NULL, sens TEXT CHECK IN ('positive','negative'),
                     categorie TEXT, texte TEXT CHECK (<= 300), visible_famille BOOLEAN, etat TEXT CHECK IN ('active','retiree'), cree_le, modifie_le)
ent_dis_incident (id, etablissement_id, numero TEXT, survenu_le TIMESTAMPTZ, precision TEXT CHECK IN ('minute','heure','demi_journee','jour'),
                  lieu_type TEXT CHECK IN ('salle','cour','couloir','cantine','internat','sortie','transport','abords','en_ligne','stage','autre'),
                  salle_id NULL, seance_id NULL, sortie_id NULL, lieu_texte TEXT CHECK (<= 120),
                  faits_internes TEXT CHECK (<= 8000), mesures_immediates TEXT CHECK (<= 2000),
                  gravite TEXT CHECK IN ('ordinaire','importante','immediate'), declarant_personne_id, etat TEXT CHECK IN
                  ('declare','pris_en_charge','clos','classe_sans_suite','fusionne'), fusionne_dans NULL, cree_le, modifie_le)
ent_dis_incident_categorie (etablissement_id, incident_id, categorie_id, PK (incident_id, categorie_id))
ent_dis_implique (id, etablissement_id, incident_id, eleve_id NULL, personne_id NULL, personne_externe_libelle TEXT NULL,
                  role TEXT CHECK IN ('auteur_presume','victime','temoin','signalant','implique'),
                  temoin_anonyme_familles BOOLEAN DEFAULT false, anonymat_motif TEXT NULL, debut_le, fin_le NULL,   -- un rôle se clôt, il ne s'efface pas
                  CHECK (num_nonnulls(eleve_id, personne_id, personne_externe_libelle) = 1))
ent_dis_suite (id, etablissement_id, implique_id, nature TEXT CHECK IN ('aucune','observation','mesure','punition','commission',
               'demande_procedure','procedure','transmission'), objet_id NULL, decidee_par, decidee_le, motif TEXT CHECK (<= 1000))
ent_dis_incident_piece (etablissement_id, incident_id, document_id, nature TEXT CHECK IN ('rapport','temoignage','capture','photo','autre'), gele BOOLEAN)
ent_dis_rapport_demande (id, etablissement_id, incident_id, agent_id, echeance, rendu_le NULL)
ent_dis_texte_famille (id, etablissement_id, incident_id NULL, procedure_id NULL, implique_id NULL, texte TEXT CHECK (<= 4000),
                       redige_par, valide_le, controle_contenu TEXT CHECK IN ('ok','averti'), version SMALLINT)   -- R-DS5 ; versions en ajout seul
ent_dis_confiscation (id, etablissement_id, eleve_id, objet TEXT CHECK IN ('telephone','autre'), confisque_par, confisque_le, rendu_a TEXT CHECK IN ('eleve','famille'), rendu_le NULL, rendu_par NULL)

-- Punitions ----------------------------------------------------------------
ent_dis_punition (id, etablissement_id, numero, eleve_id, type_id, incident_id NULL, motif_famille TEXT CHECK (<= 500), motif_interne TEXT NULL CHECK (<= 1000),
                  donnee_par_agent_id, donnee_le, echeance DATE NULL, travail TEXT NULL CHECK (<= 1000), duree_minutes NULL,
                  etat TEXT CHECK IN ('donnee','planifiee','effectuee','non_effectuee','annulee','levee'), etat_motif NULL, cree_le, modifie_le)
ent_dis_retenue_seance (id, etablissement_id, debut TIMESTAMPTZ, fin TIMESTAMPTZ, salle_id, surveillant_agent_id, capacite SMALLINT,
                        etat TEXT CHECK IN ('planifiee','tenue','annulee','reportee'), reportee_vers NULL, cree_le, modifie_le)
ent_dis_retenue_affectation (id, etablissement_id, seance_id, punition_id, eleve_id, presence TEXT NULL CHECK IN ('present','absent','retard','excuse'),
                             travail_rendu BOOLEAN NULL, pointe_par NULL, pointe_le NULL, UNIQUE (punition_id) WHERE active)
ent_dis_exclusion_cours (punition_id PK, etablissement_id, seance_id, exclu_le TIMESTAMPTZ, accompagne_par TEXT CHECK IN ('seul','delegue','adulte'),
                         arrivee_le NULL, arrivee_confirmee_par NULL, rapport_document_id NULL, alerte_emise_le NULL)

-- Procédures et sanctions ----------------------------------------------------
ent_dis_procedure (id, etablissement_id, numero, eleve_id, cadre TEXT (copie du profil à l'engagement), instance TEXT CHECK IN
                   ('chef','conseil','conseil_departemental','dasen'),
                   etat TEXT CHECK IN ('demandee','engagee','information_emise','defense_ouverte','pret_a_decider','conseil_saisi',
                   'conseil_convoque','decidee','notifiee','en_recours','close','classee','abandonnee','suspendue'),
                   faits_reproches TEXT CHECK (<= 4000), engagement_obligatoire TEXT NULL CHECK IN ('a','b','c','d'),
                   saisine_obligatoire TEXT NULL CHECK IN ('violence_physique_personnel','arme'),
                   engagee_par, engagee_le TIMESTAMPTZ, droit_silence_informe BOOLEAN NOT NULL DEFAULT false,
                   notification_effective_le TIMESTAMPTZ NULL, delai_defense_jours SMALLINT, decision_possible_le DATE NULL,  -- calculées par le serveur
                   suspendue_motif NULL, demande_ecrite_par NULL, demande_ecrite_le NULL, refus_saisine_motif NULL, cree_le, modifie_le)
ent_dis_procedure_incident (etablissement_id, procedure_id, incident_id, PK)
ent_dis_procedure_destinataire (id, etablissement_id, procedure_id, personne_id, qualite TEXT CHECK IN ('eleve','representant_legal',
                   'parent_information','personne_assistante','membre','temoin','demandeur'), raison TEXT (figée par le socle 4))
ent_dis_piece_emise (id, etablissement_id, procedure_id, destinataire_id, nature TEXT, document_id, mode TEXT CHECK IN
                   ('coffre','remise_main_propre','recommande','courriel','courrier_simple'), emise_le, numero_envoi NULL,
                   presente_le NULL, remise_le NULL, ar_document_id NULL, etat TEXT CHECK IN ('emise','remise','non_reclamee','retournee'))
ent_dis_defense (id, etablissement_id, procedure_id, nature TEXT CHECK IN ('orale','ecrite','consultation_dossier','assistance','silence'),
                 personne_id NULL, assistant_libelle NULL, tenue_le, document_id NULL, saisie_par, saisie_declaree BOOLEAN)   -- R-DS10
ent_dis_mesure_conservatoire (id, etablissement_id, procedure_id, eleve_id, debut, fin, fondement TEXT CHECK IN ('R421-10-1','D511-33','D511-50','reglement_interieur'),
                 motif TEXT CHECK (<= 1000), document_id, levee_le NULL, levee_motif NULL)
ent_dis_sanction (id, etablissement_id, numero, eleve_id, procedure_id NOT NULL,        -- jamais sans procédure (DV-01)
                  categorie TEXT NOT NULL CHECK IN ('avertissement','blame','responsabilisation','exclusion_classe',
                  'exclusion_etablissement','exclusion_service','exclusion_definitive','exclusion_definitive_service'),
                  service TEXT NULL CHECK IN ('demi_pension','internat','autre'), duree_jours SMALLINT NULL CHECK (duree_jours BETWEEN 1 AND 8),
                  debut DATE NULL, fin DATE NULL, heures_responsabilisation SMALLINT NULL CHECK (<= 20),
                  sursis TEXT CHECK IN ('aucun','total','partiel'), sursis_jours SMALLINT NULL, revocable_jusqu_au DATE NULL,
                  CHECK (sursis = 'aucun' OR categorie NOT IN ('avertissement','blame')),
                  alternative_proposee BOOLEAN, alternative_issue TEXT NULL CHECK IN ('acceptee','refusee','respectee','non_respectee'),
                  prononcee_par_instance TEXT CHECK IN ('chef','conseil','conseil_departemental','dasen','recteur','hors_scolia'),
                  prononcee_par_user_id NULL, prononcee_le, motivation TEXT NOT NULL CHECK (char_length BETWEEN 20 AND 4000),
                  notifiee_le NULL, recours_jusqu_au DATE NULL,
                  etat TEXT CHECK IN ('prononcee','notifiee','en_execution','executee','annulee','reformee','effacee'),
                  efface_prevu_le DATE NOT NULL, efface_le NULL, effacement_motif NULL CHECK IN ('delai','fin_scolarite','demande','alternative'),
                  gel_id NULL, revoque_sanction_id NULL, cree_le, modifie_le)
ent_dis_conseil_vote (id, etablissement_id, procedure_id, seance_id, tour SMALLINT, proposition TEXT, pour SMALLINT, contre SMALLINT,
                  blancs_nuls SMALLINT, voix_preponderante BOOLEAN, adoptee BOOLEAN)       -- comptes seulement (R-DS24)
ent_dis_recours (id, etablissement_id, sanction_id, nature TEXT CHECK IN ('gracieux','hierarchique','appel_recteur','interne_prive',
                  'mediation_tutelle','contentieux_administratif','contentieux_judiciaire'), forme_par TEXT CHECK IN ('eleve_majeur','representant','chef'),
                  forme_le, dans_le_delai BOOLEAN, issue TEXT NULL CHECK IN ('confirmee','reformee','annulee','irrecevable'), issue_le NULL, document_id NULL)

-- Exécution ----------------------------------------------------------------
ent_dis_responsabilisation (id, etablissement_id, sanction_id, structure_id NULL, convention_document_id NULL, engagement_document_id NULL,
                  hors_temps_scolaire BOOLEAN, accord_recueilli_le NULL, heures_prevues SMALLINT CHECK (<= 20), heures_realisees SMALLINT,
                  debut, fin, attestation_document_id NULL, etat TEXT CHECK IN ('prevue','en_cours','realisee','non_realisee'))
ent_dis_etat_attendu (id, etablissement_id, eleve_id, source_type TEXT CHECK IN ('sanction','mesure_conservatoire','retenue','exclusion_cours'),
                  source_id, etat TEXT CHECK IN ('exclu_etablissement','exclu_classe','exclu_service','mesure_conservatoire','en_retenue','exclu_cours'),
                  service TEXT NULL, debut TIMESTAMPTZ, fin TIMESTAMPTZ, lieu_accueil_id NULL, annule_le NULL)   -- port etat_attendu (12, 14, 40, 41)

-- Conseil de discipline et commission éducative ------------------------------
ent_dis_instance_membre (id, etablissement_id, instance TEXT CHECK IN ('conseil','commission'), personne_id, qualite TEXT CHECK IN
                  ('president','adjoint','cpe','gestionnaire','personnel_enseignement','personnel_atss','parent','eleve','autre'),
                  titulaire BOOLEAN, suppleant_de NULL, debut, fin)
ent_dis_seance (id, etablissement_id, instance TEXT CHECK IN ('conseil','commission','conseil_departemental'), debut TIMESTAMPTZ, lieu_texte,
                  delocalisee BOOLEAN, president_personne_id, secretaire_personne_id NULL, quorum_atteint BOOLEAN NULL,
                  etat TEXT CHECK IN ('planifiee','convoquee','tenue','reportee','annulee'), reportee_vers NULL, pv_document_id NULL,
                  copie_recteur_le NULL, cree_le, modifie_le)
ent_dis_seance_procedure (etablissement_id, seance_id, procedure_id, ordre, PK)
ent_dis_seance_presence (id, etablissement_id, seance_id, personne_id, qualite, present BOOLEAN, deport_motif TEXT NULL CHECK IN
                  ('lien_personnel','parent_eleve_concerne','eleve_sous_procedure','eleve_exclu','demandeur','victime_temoin','autre'))
ent_dis_audition (id, etablissement_id, seance_id, procedure_id, qualite TEXT, personne_id NULL, resume TEXT CHECK (<= 4000))
ent_dis_commission_cas (id, etablissement_id, seance_id, eleve_id, motif TEXT CHECK IN ('comportement','plusieurs_eleves','absenteisme',
                  'suivi_mesures','autre'), propositions TEXT CHECK (<= 2000), compte_rendu_document_id NULL)

-- Suivi éducatif, mesures, vigilance ------------------------------------------
ent_dis_suivi (id, etablissement_id, eleve_id, motif TEXT CHECK IN ('comportement','decrochage','relations','travail','reintegration','autre'),
               objectifs TEXT CHECK (<= 2000), referent_agent_id, bilan_prevu_le, etat TEXT CHECK IN ('ouvert','clos'), clos_le NULL, bilan TEXT NULL)
ent_dis_suivi_membre (etablissement_id, suivi_id, agent_id, debut, fin NULL, PK)
ent_dis_suivi_entree (id, etablissement_id, suivi_id, nature TEXT CHECK IN ('entretien','observation','point','engagement','bilan'),
               tenue_le, auteur_agent_id, visibilite TEXT CHECK IN ('cpe','equipe','famille'), texte TEXT CHECK (<= 4000), document_id NULL)
ent_dis_mesure (id, etablissement_id, eleve_id, type_id, origine_type TEXT CHECK IN ('incident','procedure','commission','suivi'), origine_id,
               debut, fin NULL, echeance_bilan NULL, etat TEXT CHECK IN ('prevue','en_cours','realisee','abandonnee'), bilan TEXT NULL)
ent_dis_vigilance_regle (id, etablissement_id, indicateur TEXT CHECK IN ('punitions','exclusions_cours','observations_negatives',
               'retenues_manquees','incidents','absences_et_punitions'), seuil SMALLINT CHECK (seuil BETWEEN 2 AND 20),
               fenetre_jours SMALLINT CHECK (BETWEEN 7 AND 120), destinataire_mission TEXT, actif BOOLEAN)
ent_dis_alerte (id, etablissement_id, eleve_id, regle_id, constatee_le, valeur SMALLINT, etat TEXT CHECK IN ('ouverte','prise_en_charge','close'),
               pris_en_charge_par NULL, clos_motif NULL)
ent_dis_alerte_fait (etablissement_id, alerte_id, objet_type, objet_id, PK)           -- les raisons : faits comptés, pas un JSON

-- Harcèlement et éloignements -----------------------------------------------
ent_dis_situation (id, etablissement_id, numero, source TEXT CHECK IN ('eleve','famille','personnel','temoin','messagerie','questionnaire',
               'telephone_3018','autre'), signalant_personne_id NULL, signalee_le, nature TEXT CHECK IN ('harcelement','cyberharcelement',
               'violence_sexiste_sexuelle','autre'), etat TEXT CHECK IN ('signalee','evaluation','prise_en_charge','suivi','close','transmise','non_confirmee'),
               protocole TEXT, cree_le, modifie_le)
ent_dis_situation_personne (id, etablissement_id, situation_id, eleve_id NULL, personne_externe_libelle NULL, role TEXT CHECK IN
               ('cible','auteur_presume','temoin'), debut, fin NULL)
ent_dis_situation_referent (etablissement_id, situation_id, agent_id, debut, fin NULL, PK)
ent_dis_situation_etape (id, etablissement_id, situation_id, etape TEXT CHECK IN ('accueil_cible','evaluation','entretien_auteur',
               'information_famille_cible','information_famille_auteur','mesure_protection','point_suivi','transmission','cloture'),
               prevue_le NULL, faite_le NULL, auteur_agent_id NULL, note TEXT NULL CHECK (<= 2000))
ent_dis_eloignement (id, etablissement_id, eleve_a_id, eleve_b_id, portee TEXT CHECK IN ('classe','groupe','cantine','sortie','chambre','transport','tout'),
               origine_type TEXT CHECK IN ('situation','incident','procedure','suivi'), origine_id, debut, fin NULL, CHECK (eleve_a_id < eleve_b_id))

-- Effacement, gel, reprise ----------------------------------------------------
ent_dis_demande_effacement (id, etablissement_id, eleve_id, demandee_par, demandee_le, motif TEXT CHECK IN ('changement_etablissement'),
               reponse_due_le DATE (demandee_le + 2 mois), decision TEXT NULL CHECK IN ('acceptee','refusee','acceptee_silence'), decidee_le NULL)
ent_dis_effacement (id, etablissement_id, sanction_id, effectue_le, motif, objets_retires SMALLINT, par TEXT CHECK IN ('systeme','direction'))   -- preuve sans contenu
ent_dis_gel (id, etablissement_id, eleve_id, motif TEXT CHECK IN ('recours','contentieux','requisition','autre'), debut, revue_le, leve_le NULL)
ent_dis_reprise_correspondance (etablissement_id, ancienne_table TEXT, ancien_id UUID, nouvel_objet_type TEXT, nouvel_id UUID, reprise_le, PK)
```

**Archive de l'effacé.** À l'effacement, les colonnes de texte de la sanction (`motivation`) et de sa procédure (`faits_reproches`), les textes famille liés et les comptes rendus de défense sont **déplacés** dans `ent_dis_archive_texte (etablissement_id, objet_type, objet_id, colonne, texte_chiffre, archive_le, sort_final_le)` — chiffré (clé inventoriée, E11), accès direction et DPO sur motif tracé, élimination au sort final (10 ans pour le PV, RG-46 ; le reste selon le DPO). Les incidents (faits) suivent leur propre durée (RG-06).

**Classement des tables (registre E12)** : toutes `etablissement`, export `double` (domaine nominatif sensible) sauf `ent_dis_situation*` et `ent_dis_archive_texte` en `particuliere` (double validation et accès restreint, D-041) ; aucune file d'effets propre (outbox commune).

**Exceptions D-036 déclarées** : aucune nouvelle. Le journal du socle n'a que des colonnes texte ; l'instantané avant/après passe par `ent_journal_modif.payload` (exception existante).

---

## 26. Reprise des données existantes

1. **Anciennes tables Rust** (P0 et S2-4), base de démonstration et futures bases de recette : reprise idempotente par `ent_dis_reprise_correspondance` (rejouable, E8) :
   - `ent_incident` → `ent_dis_incident` + un `ent_dis_implique` (auteur présumé) ; `description` → `faits_internes` ; **aucun texte famille** créé (à rédiger si l'incident est encore ouvert) ;
   - `ent_sanction` avec `categorie` connue → `ent_dis_sanction` + une procédure `hors_scolia` (« sanction enregistrée ») ; sans catégorie → **file de requalification** pour la direction (aucune sanction reprise sans catégorie) ; `effacee_at` repris ; date d'effacement **recalculée** selon RG-05 — une sanction effacée à tort trop tôt par DV-02 **reste effacée** (on ne ressuscite pas une sanction effacée), signalé dans le rapport ;
   - `ent_punition` → `ent_dis_punition` (`motif` → `motif_famille` si visible, sinon interne) ;
   - `ent_procedure_disciplinaire` → `ent_dis_procedure` (états mappés ; `decision` libre → motivation de la sanction liée si elle existe, sinon note au journal) ;
   - `ent_vs_commission*` → séances et cas de commission éducative ; `decision` et `points_retires` **non repris** comme sanctions (rapport à la direction) ;
   - `ent_alerte_cpe` → alertes `close` avec raison « reprise » (pas de règle) ;
   - `ent_edition_vs` → abandonnée (natures du §16, 07).
2. **Charlemagne** (lycée pilote) : historique des **sanctions non effacées** des élèves présents, par export fourni par Florian (F-01 à F-04, données anonymisées pour les essais) → `DIS-274` (« prononcée hors ScolIA », catégorie, date, durée, autorité) ; les incidents et punitions **ne sont pas repris** [R].
3. **Next.js C07** : perdable (T0) ; rien n'est repris. Incidents d'internat : repris par le socle 41 s'il le décide (Q-DS13).
4. **Retrait** des anciennes tables (2279) après la durée d'archive et une recette de non-régression.

---

## 27. Lots de code (équipe Sonnet) et migrations

### 27.1 Tableau des lots

| Lot | Contenu | Corrige | Dépend de | Migration |
|---|---|---|---|---|
| **D0 — correctifs urgents** | (1) `sanctions.prononcer` retiré de CPE et Administratif, ancienne route `POST /eleves/{id}/sanctions` limitée à la direction et **exigeant une procédure décidée** ; test R2 modifié ; (2) table R511-13 IV **codée** (fin d'année N, N+1, N+2, fin de scolarité ; alternative N+1), paramétrage supprimé, catégorie obligatoire (reprise : types sans catégorie → `autre` bloqué, file de requalification), test d'effacement corrigé ; (3) **tâche quotidienne** idempotente d'effacement ; (4) effacement **en profondeur** (textes de l'incident lié, faits et décision de la procédure, chronologie filtrée, corps des SMS purgés) ; (5) procédure : `informe_le` + `decision ≥ informe_le + 2 jours ouvrables` (lundi–samedi hors fériés) ; `delai_convocation_jours ≥ 5` (CHECK), mode d'envoi et date de remise exigés avant la séance ; mention du **droit de se taire** dans le texte de l'édition ; (6) **vue famille** : `description` et `note` **retirées** des réponses famille (web, iOS, Android) ; sanctions annulées non servies ; (7) **SMS de sanction supprimé** (avis neutre quand le socle 8 N0 sera là ; d'ici là : aucun envoi externe, avis in-app seulement) ; (8) `TenantTx` sur `discipline_vs.rs`, `alerte_cpe_repo.rs`, `carnet_repo.rs`, `edition_vs_repo.rs` + un test RLS strict par table ; (9) alertes CPE : action `discipline.alertes.lire`, retirée de comptabilité, infirmerie, psychologue ; vue famille fermée ; (10) commission : décisions « avertissement / exclusion » et points **gelés** (plus d'écriture, masqués aux familles) ; (11) éditions VS : `autoriser_eleve` ; (12) incident : portée « ses cours » du professeur + fait constaté | DV-01 à DV-12, DV-14 | — (peut partir tout de suite) | **2260** (catégorie NOT NULL après reprise, CHECK des délais, colonnes `informe_le`, `convocation_mode`, `convocation_remise_le`, `sanction.duree_jours`, purge des corps de SMS) ; 2261 réservée (second temps de D0 si nécessaire) |
| **D1 — brique et incidents** | crate `discipline` (D-026) ; profil, catégories de faits, compteurs, journal ; observations ; **incident à plusieurs personnes**, suites par élève, pièces, rapports demandés, texte famille, contrôle de contenu, file de traitement, retour au déclarant, chronologie neutre ; actions D-017 ; événements dans la brique | DV-06, DV-13, DV-15 à DV-18, DV-20, DV-21, DV-23 | 2, 4, 5, 6, 7, 8 (existants) | **2262**, **2263** |
| **D2 — punitions et retenues** | catalogue par nature, interdits ; punitions individualisées (saisie multiple) ; **séances de retenue** (salle, surveillant, capacité, conflits), appel de retenue, report ; **exclusion de cours** avec suivi d'arrivée ; confiscations ; avis | DV-19 | D1, 10, 12 | **2264** |
| **D3 — procédure du chef d'établissement et sanctions** | procédure complète (destinataires, pièces émises, preuves, frise des délais, défense, consultation, droit de se taire, cas obligatoires, refus de saisine motivé), mesure conservatoire, **sanction structurée** (catégorie, jours ≤ 8, sursis, alternative, motivation, date d'effacement), notification, vue famille et dépôt d'observations, « vos droits » | DV-05 (complet) | D1, 7 (natures, `etat()`), 8 | **2265** |
| **D4 — exécution** | états attendus (12, 14, 40, 41), exclusion-inclusion (10), travail (20), responsabilisation (structures, convention, accord, heures ≤ 20), alternative (engagement signé, issue), sursis (révocabilité, révocation, plafond 8 jours), vue accueil | — | D3, 10, 12, 14 | **2266** |
| **D5 — conseil de discipline (EPLE)** | composition, membres, incompatibilités et déports, séances, convocations (recommandé / main propre, bordereau), accès des membres, quorum et report, mode séance, votes, PV, notification et recommandé du jour, copie au recteur, appel (8 jours, non suspensif, décision sous un mois), départemental, délocalisation, exclusion définitive (3, autorité) | — | D3, 46 (ou saisie locale) | **2267** |
| **D6 — profil privé sous contrat** | conseil **consultatif** (avis puis décision du chef), recours interne du règlement, médiation, jours ouvrés, trames privées ; profil hors contrat | — | D5 | **2268** |
| **D7 — effacement, gel, archives, droits** | demandes d'effacement (2 mois), fin de scolarité, gel, archive chiffrée de l'effacé, sort final, preuve d'effacement, accès RGPD expurgé, journal des accès, transfert au nouvel établissement (si Q-DS14) | DV-02 à DV-04 (complet) | D3, 7, 27 | **2269** |
| **D8 — suivi éducatif et commission éducative** | suivis, membres, entrées par visibilité, engagements ; mesures ; commission (composition, séances, cas, propositions, suivi des mesures) ; requalification de l'ancienne commission | DV-10 (complet) | D1, 19 | **2270** |
| **D9 — harcèlement** | situations, personnes, référents, étapes (pHARe préréglé), mesures de protection, **éloignements** publiés (5, 10, 42, 41), transmissions (36, procureur tracé), recueil (R470-1), suites disciplinaires obligatoires | — | D1, D3, 16 (signalement), 36 | **2271** |
| **D10 — vigilance et pilotage** | règles déterministes et alertes expliquées, suivi d'un élève, synthèse pour le conseil de classe (23), tableau de bord, registre anonymisé des sanctions et bilan au CA, indicateurs (47), rappel « Faits établissement » | DV-09 (complet) | D1–D4 | **2272** |
| **D11 — mobile** | professeur (observation, incident, punition, exclusion de cours depuis l'appel), AED (retenue, arrivée), CPE (file), familles et élèves (avis, retenue, procédure, dépôt) | — | 11, D1–D3 | — |
| **D12 — IA** | capacités du §18.3, tests de garde, jeu d'évaluation | — | 30 (cœur IA), D1–D3 | — |
| **D13 — reprise** | §26 (anciennes tables, Charlemagne, sanctions « hors ScolIA ») | — | D1–D3 | **2273** |
| **D14 — RLS stricte et retraits** | RLS stricte (test strict de toutes les tables du socle) ; retrait des anciennes tables après archive | DV-08 (final) | tous | **2278**, **2279** |

**Parcours minimal** [R] : **D0 tout de suite** (avant la démo de décembre 2026 si possible : ce sont des défauts juridiques et des fuites) ; puis **D1, D2, D3, D4, D7** pour la V1 de fin janvier – février 2027 (D-034) ; **D5, D8, D9, D10** pendant l'épaississement (février – août 2027) ; D6 avant le doublon (le lycée pilote est privé) ; D11, D12, D13 selon les socles 11 et 30.

**Année de doublon 2027-2028** (D-034) [R, Q-DS1] : la discipline **n'est pas** un module de l'add-on Next.js en usage réel (C07 perdable) ; mais ses usages quotidiens (observations, incidents, punitions, retenues, exclusions de cours, suivi CPE) ne font l'objet d'aucun échange officiel : **usage réel recommandé dès la rentrée 2027** pour eux (gain immédiat pour la CPE, pas de double saisie dans Charlemagne si l'établissement l'accepte) ; les **sanctions et procédures** restent officielles dans Charlemagne (ou sur papier) et sont **enregistrées** dans ScolIA (DIS-274) pour que l'effacement, la synthèse du conseil de classe et l'historique fonctionnent ; bascule de la procédure complète après la recette juridique (relecture des trames par un juriste).

### 27.2 Migrations (registre `MIGRATIONS.md`)

Plage **imposée 2260–2279** (message de lancement), vérifiée libre sur **les 124 branches distantes** le 2026-11-11 (aucun fichier de migration ni réservation 226x–227x ; `git grep` du répertoire `ent-rust/migrations` et de `MIGRATIONS.md` de chaque `origin/*`). Elle **remplace** les plages hypothétiques « 3000 + 100 × socle » (4300–4399) et de repli qui auraient pu être proposées : il n'y en a qu'une.

| Numéro | Lot | Objet |
|---|---|---|
| 2260 | D0 | correctifs urgents (colonnes et contraintes sur les anciennes tables, reprise des catégories, purge des corps de SMS de sanction) |
| 2261 | D0 | réserve de D0 |
| 2262 | D1 | profil, catégories de faits, compteurs, journal, observations |
| 2263 | D1 | incidents, impliqués, suites, pièces, rapports demandés, textes famille, confiscations |
| 2264 | D2 | types et punitions refondus, séances et affectations de retenue, exclusions de cours |
| 2265 | D3 | procédures, destinataires, pièces émises, défense, mesures conservatoires, sanctions |
| 2266 | D4 | responsabilisation, structures, états attendus |
| 2267 | D5 | membres d'instance, séances, présences, auditions, votes, recours |
| 2268 | D6 | colonnes et contraintes du profil privé |
| 2269 | D7 | demandes d'effacement, effacements, gel, archive des textes effacés |
| 2270 | D8 | suivis, membres, entrées, mesures, cas de commission |
| 2271 | D9 | situations, personnes, référents, étapes, éloignements |
| 2272 | D10 | règles de vigilance, alertes et leurs faits |
| 2273 | D13 | correspondance de reprise |
| 2274–2277 | — | réserve (ajouts personas, §32) |
| 2278 | D14 | RLS stricte |
| 2279 | D14 | retrait des anciennes tables (`ent_incident`, `ent_sanction`, `ent_sanction_type`, `ent_sanction_motif`, `ent_incident_type`, `ent_punition*`, `ent_procedure_disciplinaire`, `ent_regle_effacement`, `ent_vs_commission*`, `ent_alerte_cpe`, `ent_edition_vs`) |

Règles : migrations **additives et rejouables** (E8) ; 2279 est la seule destructrice, après décision, sauvegarde vérifiée et note dans `MIGRATIONS.md`. `MIGRATION_TAG` : 2260 deviendra la plus haute migration du répertoire tant qu'aucune plage plus haute n'est écrite.

---

## 28. Tests et recette

1. **Isolation** : matrice inter-établissements sur chaque route (générée depuis la déclaration des routes, D-026) ; RLS stricte sur chaque table.
2. **Minimisation** : les dix pièges du §21.5, en tests d'API et d'écran (famille, professeur de groupe, comptabilité, élève majeur, interdiction de communication, témoin anonymisé, sanction effacée, IA).
3. **Délais** : tests de propriété du moteur de dates (samedi, fériés, vacances, fuseau, changement d'heure, notification un vendredi soir, recommandé non réclamé) ; refus avant la date au plus tôt.
4. **Échelle et sursis** : sanctions hors catégorie refusées, 9 jours refusés, sursis sur un blâme refusé, révocation pour un niveau inférieur refusée, cumul > 8 jours refusé, exclusion définitive par le chef seul refusée, décision du chef seul refusée en cas d'arme.
5. **Effacement** : table complète R511-13 IV (une sanction de chaque catégorie, chaque date limite, fin de scolarité, demande avec silence de deux mois, alternative respectée / non respectée, gel) ; vérification qu'aucune lecture courante ne la retrouve (fiche, chronologie, conseil, export, IA, avis) ; idempotence de la tâche.
6. **Procédure de bout en bout** (chef seul et conseil) : information préalable avec droit de se taire, défense, décision, notification, appel ; contrôle des trames (mentions obligatoires).
7. **Transactions** : fait + journal + outbox + document + avis ou rien (pannes injectées).
8. **Reprise** : rejouable, idempotente, rapport ; sanctions sans catégorie non reprises.
9. **Mobile** : appel de retenue hors ligne, exclusion de cours depuis l'appel.
10. **IA** : tests de garde et jeu d'évaluation (§18.4).
11. **Recette juridique** : relecture des trames de l'information préalable, de la notification, de la convocation et du PV par un juriste avant tout usage réel (Q-DS21).

---

## 29. Réglages signalés (D-023 : peu de réglages, signalés) — S-1301 à S-1320

| # | Réglage | Défaut | Pourquoi il existe | Candidat à l'allègement ? |
|---|---|---|---|---|
| S-1301 | Profil privé : délais **en dessous** des valeurs du public | interdit sans décision motivée | règlement intérieur du privé | oui (le garder fermé) |
| S-1302 | Privé : jours ouvrables ou ouvrés | ouvrables | règlement intérieur | non |
| S-1303 | Catalogue des punitions de l'établissement | 6 types préréglés | règlement intérieur | non |
| S-1304 | Catégories de faits : libellés et sous-catégories | préréglées | vocabulaire local | oui |
| S-1305 | Préavis minimal d'une retenue aux familles | 2 jours ouvrés [H] | organisation des familles | non |
| S-1306 | Délai d'alerte d'un exclu de cours non arrivé | 10 minutes | sûreté | non |
| S-1307 | Seuil d'exclusions de cours avant alerte | 3 dans la période | vigilance | oui |
| S-1308 | Observation négative visible des familles par défaut | non | ton de l'établissement | oui |
| S-1309 | Professeur : punir un fait constaté hors de ses cours | oui | pratique courante | non |
| S-1310 à S-1313 | Seuils des quatre règles de vigilance | préréglés | vigilance | oui (un seul réglage « sensibilité » à trois positions, comme 21 S-118) |
| S-1314 | Barème de points (vigilance seulement) | **désactivé** | pratique de certains établissements (T-22) | **oui** (proposition : ne pas l'offrir, Q-DS11) |
| S-1315 | Protocole harcèlement : pHARe ou protocole propre | pHARe (public), pHARe proposé (privé) | — | non |
| S-1316 | Témoin anonymisé pour les familles : qui décide | direction | sécurité des témoins | non |
| S-1317 | Durées de conservation hors règle d'archives | §21.3 | DPO | non |
| S-1318 | Rappel de retenue la veille | oui | — | oui |
| S-1319 | Trames des courriers | 3 ou 4 par nature (D-023) | identité | non |
| S-1320 | Parents d'un élève majeur informés des sanctions | **non** (Q-DS8) | absence de texte | non |

---

## 30. Décisions appliquées par défaut (D-031) et questions

### 30.1 Appliquées par défaut (D-031), Florian peut revenir dessus

| # | Question | Décision appliquée |
|---|---|---|
| Q-DS1 | Discipline pendant l'année de doublon | usage réel pour observations, incidents, punitions, retenues, exclusions de cours, suivi ; sanctions et procédures officielles dans Charlemagne, **enregistrées** dans ScolIA (DIS-274) ; bascule après recette juridique (§27.1) — **à confirmer par Florian** car cela touche l'organisation réelle du lycée (voir 30.2) |
| Q-DS2 | Jours ouvrables ; point de départ d'un recommandé | lundi–samedi hors fériés, vacances comprises avec avertissement (RG-14) ; recommandé : date de **première présentation** si connue, sinon date de remise ; remise en main propre proposée en complément |
| Q-DS3 | « Fin de l'année scolaire » | date de fin de l'année du référentiel (05) |
| Q-DS4 | Professeur et fait constaté hors de ses cours | permis (saisie et punition), sans lecture de l'historique (R-DS20, S-1309) |
| Q-DS5 | Qui engage une procédure | chef et adjoint par délégation ; la CPE **propose** |
| Q-DS6 | Témoignage anonymisé pour la défense | possible sur décision motivée de la direction ; identité connue du chef ; **à faire valider par un juriste** avant usage réel |
| Q-DS7 | Parent sans exercice de l'autorité parentale | reçoit l'**information** des sanctions et décisions importantes, pas les pièces de défense |
| Q-DS8 | Parents d'un élève **majeur** | **non** destinataires par défaut (aucun texte ; RG-44) ; l'élève majeur peut choisir de les associer (lien « informer mes parents ») |
| Q-DS9 | Réparation financière d'un dommage | hors socle : jamais une sanction ni une punition ; démarche civile séparée (assurance) |
| Q-DS10 | Besoins particuliers au conseil de discipline | pastille au président ; membres informés seulement avec l'accord de la famille |
| Q-DS11 | Permis à points (T-22) | **non retenu** comme mécanique ; barème de vigilance désactivé par défaut (S-1314) ; ancienne commission à points gelée (D0) |
| Q-DS12 | « Faits établissement » (R421-10-2) | rappel et trace dans ScolIA, aucune interface avec le système du ministère au lancement |
| Q-DS13 | Incidents d'internat de Next.js | décision laissée au socle 41 ; le 13 fournit le modèle d'incident |
| Q-DS14 | Transfert au nouvel établissement | aucun transfert automatique ; sur demande de l'établissement d'accueil, seulement les sanctions non effacées, après accord de la direction, tracé — **base légale à confirmer par le DPO** |
| Q-DS18 | Information de la victime sur la sanction de l'auteur | « des suites ont été données », sans nom de sanction |
| Q-DS19 | Exclusion et examen national | convocation à l'examen maintenue (l'établissement n'en prive pas l'élève) |
| Q-DS22 | Plage de migrations | **2260–2279** (imposée), seule plage du socle |
| Q-DS23 | Observations négatives visibles des familles | non par défaut, au choix de l'auteur (S-1308) ; toute **punition** fait l'objet d'une information écrite (RG-34) |
| Q-DS24 | Lignes, punitions humiliantes | non proposées par défaut ; « autre » soumis à la CPE |
| Q-DS25 | Exclusion de cours : qui confirme l'arrivée | AED ou CPE ; alerte après 10 min |

### 30.2 Soumis à Florian (engagent l'établissement, des données réelles ou un tiers)

| # | Point | Pourquoi Florian |
|---|---|---|
| F-DS1 | **Q-DS1** : la vie scolaire du lycée pilote utilisera-t-elle ScolIA en usage réel dès la rentrée 2027 pour les punitions, retenues et incidents (et plus dans Charlemagne) ? | organisation réelle, accord de l'équipe et de la direction |
| F-DS2 | Règlement intérieur du lycée pilote : catalogue des punitions, instance disciplinaire (conseil consultatif ? « conseil d'éducation » ?), recours interne, délais | fait que seul Florian connaît ; trames privées |
| F-DS3 | Reprise des sanctions non effacées depuis Charlemagne (export F-01 à F-04) | données réelles : accord écrit et DPO (D-034, D-035) |
| F-DS4 | Recette juridique des trames (information préalable, notification, convocation, PV) | coût d'un juriste |
| F-DS5 | Arrêt éventuel des écrans « sanctions » et « carnet » de Next.js | décision et geste de Florian (D-035 point 3) |

### 30.3 Questions au juriste (par priorité)
1. **Privé sous contrat** : garanties à imposer par défaut (contradictoire, droit de se taire, délais, assistance, appel interne) ; conseil consultatif ; doctrine actuelle du SGEC.
2. Anonymisation d'un témoignage vis-à-vis de la défense (Q-DS6).
3. Point de départ du délai pour un recommandé non réclamé (Q-DS2).
4. Effacement à la demande : exclusion définitive exclue ? silence de deux mois (RG-06) ; portée exacte (pièces archivées, faits conservés).
5. L511-5 (téléphone) dans le privé sous contrat ; nature de la confiscation.
6. Information des parents d'un élève majeur (Q-DS8).
7. Délégation au-delà des adjoints ; conduite du contradictoire par la CPE.
8. Internat comme « service annexe » ; sanctions d'internat dans le privé.
9. Information de la victime sur les suites (Q-DS18).
10. Dispositions transitoires des décrets 2026-414 et 2026-637 (procédures en cours).
11. Valeur d'une remise contre signature électronique (déjà VERIF R35).

### 30.4 Questions au DPO
1. Données disciplinaires et article 10 du RGPD quand les faits sont pénalement qualifiables ; mentions de poursuites (RG-29).
2. Durées du §21.3 (incidents, observations, suivis, situations, trace d'effacement, archive des textes effacés).
3. Instruction de tri 2005 pour le privé (archives publiques du privé sous contrat, VERIF R39).
4. R470 : articulation des questionnaires nominatifs (socle 36) et des situations (socle 13) ; responsable de traitement dans le privé (le chef d'établissement).
5. Transfert au nouvel établissement (Q-DS14) ; export vers la mairie (RG-45, Q-DS17).
6. AIPD : le socle 13 entre dans l'AIPD de l'ENT (mineurs, données sensibles, IA).

---

## 31. Faits, hypothèses, contradictions

**Faits [F]** : code Rust lu sur `origin/ent-mc0hoe` (2026-11-11) ; code Next.js lu en lecture seule sur une copie de `origin/main` ; plages et numéros vérifiés sur les 124 branches distantes.

**Vérifié [V]** : RG-01 à RG-05, RG-07 à RG-13, RG-15 (code), RG-16 à RG-31, RG-33 (code), RG-35, RG-37, RG-38, RG-40 à RG-46, §5.3, §5.4 (code et jurisprudence citée). **[V-ext]** : RG-06, RG-14, RG-32, RG-34, RG-36, RG-39, enseignement catholique. **[NT]** : interdiction nominative des lignes, interdiction d'exclure pour absentéisme, texte sur les parents d'un élève majeur, internat « service annexe », circulaire plus récente que 2014-059, doctrine CNIL sur l'article 10, statut actuel du SGEC.

**Hypothèses [H]** : irritants des acteurs (§3) et année type (§4) ; nombre de conseils de discipline par an ; préavis de retenue ; effets cantine (non facturation d'un exclu).

**Contradictions relevées**
- `SOCLES.md` §13 classe l'existant « I, L » : c'est vrai pour les punitions et la procédure S2-4, mais la génération P0 contredit le Code (DV-01 à DV-07) ; le statut réel est **« implémenté, non conforme »**.
- `SOCLE2_DOMAINES.md` (§ S2-4, lignes « Sanctions : catégories et effacement » et « Procédure disciplinaire ») décrit des défauts [H] (délai de 8 jours, effacement « N mois ») que le §5 montre **faux** : délai minimal de 5 jours (convocation), 2 jours ouvrables (défense), effacement en années scolaires.
- `16_MESSAGERIE.md` R-16 dit « L. 511-3-1 abrogé » : plus exactement, le contenu est **devenu L111-6** (RG-37).
- `07_DOCUMENTS.md` (lignes 156, 158) garde « délai cité 5 ou 8 jours », « [NV] » : tranché ici (5 jours, D511-31 [V] ; deux jours ouvrables, R421-10-1 [V]) ; à reporter au 07 lors de sa prochaine mise à jour.
- `08_NOTIFICATIONS.md` §143 prévoit `discipline.punition` « titulaires + élève » : cohérent ; l'information écrite des parents est **due** pour toute punition (RG-34) : le refus d'un canal ne supprime pas l'information (comme VERIF R3).
- Test `actions.rs:1460` (« non-régression R2 ») **fige** un droit contraire au Code (CPE et secrétariat prononcent) : il est modifié par D0, ce n'est pas une régression.

---
