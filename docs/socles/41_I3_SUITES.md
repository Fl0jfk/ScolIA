# Socle 41, lot I3 — suites (M1 à M4, tiers autorisé, infirmerie, classes, faibles) : livraison

Branche `ent-socle-41-code-i3-suites`, partie d'`origin/ENT` **65daf461**. Migration **2827** `internat_i3_suites` (2824 à 2826 versées, **non modifiées**, règle 25). Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27). Documents lus : `CLAUDE.md`, `PILOTAGE.md` (règles 17, 25, 27, 28, 31, 33, 34, 35, 36), `41_INTERNAT.md` (I3), `41_INTERNAT_I3_LIVRAISON.md`, `41_I3_CORRECTIFS.md` (passes 1 à 4).

**Environnement des tests** : PostgreSQL 16 local, bases **neuves** (`ent_i3s`, puis une base neuve pour la suite large), `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, Rust **1.99.0**, `--locked`, un seul `CARGO_TARGET_DIR`. Aucun test ne dépend de l'heure réelle : jours fixes d'avril 2026, instants passés à chaque commande ; les faits de perte de droit, posés à l'instant de l'écriture, sont datés par le test (`dater_perte_droit`, qui lève le déclencheur d'ajout seul le temps d'une transaction de test).

## 1. Fait / non fait

| # | Point | État | Ce qui a été fait | Tests |
|---|---|---|---|---|
| 1 | **M3** tiers autorisé à récupérer | **fait** (décision du pilote ; Florian peut la revoir) | Table `ent_internat_tiers_autorise` : tiers (personne), titulaire demandeur (fiche), date de la demande écrite, du / au (**un an au plus**, les deux obligatoires), motif (300 car.), statut `actif` / `conteste` / `revoque`, jamais supprimée, champs fondateurs figés. Posé par le personnel (`internat.sorties.decider` : CPE, vie scolaire, direction, administration), **pas de nouvelle action**. **Porte « tiers autorisé » ajoutée dans l'unique fonction de remise** `ent_internat_remise` (donc contrôle de départ, revue à la minute, liste de remise, infirmerie, désignation de personne). Règles générales : une **interdiction bloque toujours** (chaîne de fusions) et empêche la pose ; **un acte tombe avec son auteur** (le titulaire demandeur visé ou privé de son droit ferme la porte ; un autre titulaire reprend la demande, motif écrit) ; **refus par défaut** ; un lien de la personne suspendu / en garde « aucune » / échu n'est pas contourné (la justice ne se contourne pas) ; **désaccord** : désaccord connu du socle 18 → posé « contesté » ; l'opposition d'un autre titulaire ferme la porte aussitôt (le plus restrictif) ; **la direction seule tranche** (lever avec motif obligatoire, ou maintenir = révoqué) ; le demandeur ou le personnel révoque (effet immédiat, sorties à venir revues). Historisé (6 faits `tiers_*` du journal I3). **Visible des familles** (titulaires, élève majeur) : personne, dates, état, « remettable aujourd'hui », **sans motif, sans demandeur, sans opposant** ; le personnel lit tout (lecture journalisée, objet `tiers`). 6 routes. Aucun nom ni texte libre dans les événements. | `s_m3_tiers_autorise_pose_remise_visibilite_interdiction_et_auteur_qui_tombe`, `s_m3_desaccord_opposition_arbitrage_et_revocation`, `s_m3_une_sortie_avec_le_tiers_ne_part_plus_apres_revocation` ; routes : `internat_api_absences_sorties_parcours_cpe_droits_et_bornes` (403, 404) |
| 2 | **M1** auteur du personnel | **fait** | `ent_internat_personnel_seul` : compte qui n'a **aucun** rôle parent ou élève dans l'établissement. `ent_internat_user_vise` n'applique plus la perte de droit « sans lien » à ce compte (le correspondant désigné par le CPE, un décideur du personnel, n'est plus suspendu à chaque revue). **Une interdiction nominative qui le viserait vaut toujours.** Un compte à la fois parent et agent n'est pas exempté (prudent). | `s_m1_un_auteur_du_personnel_n_est_pas_un_auteur_sans_lien` (désignation par le CPE, deux revues, témoin parent sans lien « visé », interdiction qui vise l'agent) |
| 3 | **M2** perte de droit datée par un fait immuable | **fait** | Table en ajout seul `ent_internat_perte_droit_fait` (`a` = instant d'écriture, `effet_le`) alimentée par un **déclencheur `AFTER INSERT OR UPDATE OF autorite_parentale, garde, suspendu_le, date_fin` sur `ent_eleve_lien`**, uniquement aux **transitions vers la perte** (autorité retirée, garde « aucune », suspension, pose ou changement de date de fin). `ent_internat_perte_droit` date par ces faits ; **plus aucun `updated_at`**. Reprise de l'existant datée comme avant (une levée ou une requalification déjà faite n'est donc pas annulée). La suspension est datée de l'instant d'écriture, **pas de la date saisie** (une suspension antidatée ne remonte pas le temps) ; la fin de lien du lendemain de `date_fin`. | `s_m2_ecriture_du_payeur_fusion_reactivation_ne_datent_pas_la_perte` (écriture du payeur, écriture de rattachement / fusion, réactivation : **aucun fait, levée conservée** ; ajout seul ; les quatre natures à la transition) ; `p4_r1`, `p4_r2`, `p4_r5` adaptés (dates fixées par `dater_perte_droit`) |
| 4 | **M4** garde des sorties scolaires | **fait** | La garde de sources (non récursive, avec littéraux) est **remplacée par une garde en base** : déclencheur `ent_internat_remise_participant_garde` sur `ent_sortie_participant` (INSERT ou passage vers `remis` / `parti_seul_autorise` refusé, `42501`). Aucun bloc d'un autre socle n'est modifié (déclencheur de garde seulement). | `p4_r8_la_base_refuse_d_ecrire_un_etat_de_remise_d_un_participant` (déclencheur posé et actif sur la vraie table ; comportement sur une table temporaire) |
| 5 | **Infirmerie** | **fait** | `personne_habilitee_a_recuperer` (`sante_acc.rs`) ne réécrit plus la règle : `personne_peut_prendre_l_eleve_tx` (point d'entrée unique de `internat_sorties.rs`) appelle `ent_internat_remise` : titulaire en exercice, personne autorisée, tiers autorisé, **interdictions (avant : ignorées !)**, chaîne de fusions, perte de droit, interdiction sans personne identifiée. Jour = jour du passage. | `s_infirmerie_la_remise_passe_par_la_fonction_unique_de_l_internat` |
| 6 | **Classes par code sans année** | **fait** (audit : voir §3) | Corrigés : `messagerie_m0b.rs::resoudre_classe` (tri explicite, année courante d'abord, ambiguïté seulement s'il n'y a pas exactement une classe de l'année courante), `edt_notifications.rs` (`LIMIT 1` sans ordre → `ORDER BY courante DESC, date_debut DESC, id`). `cahier.rs::exiger_classe_texte` l'était déjà (65daf461). Autres occurrences : voir tableau §3. | Les parcours existants ; pas de test dédié (voir §5) |
| 7a | Requalification d'un contact | **fait** | Limitée aux **élèves internes** (séjour d'internat couvrant le jour) ; **conflit d'intérêt** refusé : l'agent est la personne du contact, son déclarant (chaîne de fusions) ou un proche de l'élève (lien). | `s_requalification_limitee_aux_internes_et_sans_conflit_d_interet` |
| 7b | `tracing` sans texte sqlx | **fait** | `nature_erreur` : catégorie et code SQLSTATE seulement ; remplace `error = %e` dans la revue d'internat (actes.rs), la veille et les rappels d'appel. | — (lecture) |
| 7c | Tests manquants | **fait** | M1, M2, 403 des routes du tiers, sortie sous R1. | `s_m1_*`, `s_m2_*`, `s_p4_r1_la_sortie_dont_l_auteur_perd_son_droit_retombe_a_la_demande`, test API |

## 2. Décisions prises (à valider)

1. **Tiers : un an au plus, du / au obligatoires** ; renouvellement = nouvelle pose. Statut posé « contesté » d'emblée s'il existe un désaccord connu du socle 18.
2. **Tiers : lien écarté** = suspendu, garde « aucune », échu ou pas commencé (l'autorité retirée **n'écarte pas** : c'est le cas du proche). Un parent déchu de l'autorité par décision de justice **avec lien suspendu** ne se rétablit pas par ce statut ; un parent privé d'autorité *sans* suspension du lien peut en revanche être posé tiers — **à valider par Florian** (le sens prudent serait de l'exclure, mais rien ne le distingue d'un grand-parent sans libellé, R6).
3. **Seul le demandeur révoque** côté famille ; l'autre titulaire s'oppose. Seule la direction tranche.
4. **M1** : compte « personnel seul » = aucun rôle parent / élève dans l'établissement (aucun état d'accès lu).
5. **M2** : suspension datée de l'écriture, pas de `suspendu_le` ; ceci est plus strict qu'avant et sans effet sur un acte déjà couvert.
6. **Infirmerie** : le jour de la remise est celui du passage (le service n'a pas d'horloge injectée) ; une interdiction **sans** personne identifiée bloque aussi l'infirmerie (on ne peut pas vérifier qui est à la porte).

## 3. Classes cherchées par code sans année (audit, point 6)

Faits relevés (Haiku, lecture seule, puis contrôle) : `ent_classe` est unique par `(etablissement_id, annee_id, code)` ; un même code existe donc une fois par année. `ent_eleve.classe`, `ent_cahier_texte.classe`, `ent_conseil_classe.classe`, `ent_appel.classe` sont des textes sans année.

| Occurrence | Verdict |
|---|---|
| `service.rs::exiger_classe_dans_perimetre` (10 appelants) | **Ne lit pas `ent_classe`** : compare le code au `DISTINCT ent_eleve.classe` des élèves de la portée de l'agent ; pas de recherche de ligne, donc pas de tirage au hasard. Reste un point à vérifier hors périmètre : un code d'une année passée passerait si des élèves portent encore ce texte. |
| `cahier.rs::exiger_classe_texte` | Déjà corrigé (65daf461) : année courante d'abord, tri explicite. |
| `messagerie_m0b.rs::resoudre_classe` | **Corrigé** (voir ci-dessus). |
| `edt_notifications.rs` (sous-requête `classe_id`) | **Corrigé** (fenêtre de date, ordre explicite). |
| `edt_reactions.rs`, `edt_masse.rs`, `eleve_repo.rs::list_en_classe_au` | Année déterminée par la **date** (fenêtre `date_debut`/`date_fin`) ou par identifiants : pas de recherche ambiguë. Laissés. |
| `edt_admin.rs` (contrôle « classe connue ») | Booléen d'existence (`archivee_le IS NULL`), accepte volontairement une classe d'une année en préparation (emploi du temps de l'année suivante). Laissé. |
| Migrations (`ent_resoudre_classe`, `ent_assurer_annee`, backfills) | Filtrées par année ou `HAVING count(*) = 1`. |

**Origine de l'année « préparation » 2025-2026 du labo.** Le jeu de labo (`ent-lab`) **ne crée ni année scolaire ni année en préparation** (aucune insertion dans `ent_annee_scolaire`). L'année est créée par la **fonction SQL `ent_assurer_annee(établissement, jour)`** (migration 111) quand une écriture porte sur un jour qu'aucune année ne couvre : elle crée l'année 1ᵉʳ septembre → 31 août, **`courante` seulement si `current_date` tombe dans cette année et qu'aucune autre n'est courante**. Un jour historique (2025-2026) crée donc une année `courante = false`, et le déclencheur `ent_trg_annee_statut` (110) range tout non-courant non clôturé en **`preparation`**, même quand l'année est **passée**. Ce n'est donc **pas une erreur du jeu de labo** mais une propriété du modèle : « préparation » = « ni courante, ni clôturée », y compris pour une année révolue jamais clôturée. **Corrigé ? Non** : changer ce déclencheur ou `ent_assurer_annee` toucherait le socle du référentiel (autre socle, règle 21) et ses tests ; les recherches de classe par code sont maintenant robustes à cette situation (année courante d'abord). **À décider** avec le socle 3 : statut `cloturee` pour toute année dont `date_fin` est passée.

## 4. Migration, compteurs, registres (règle 28, recalculés à partir d'ENT 65daf461)

- **Migration 2827** enregistrée dans `migrate.rs` après 2826 et dans `docs/socles/MIGRATIONS.md`. **Numéros décalés d'un cran dans `41_INTERNAT.md`** : I4 = 2828–2829, I5/I6 = 2830, I7 = 2831, I8 = 2832, I9 = 2833, I11 = 2834, I12 = 2835, réserve 2836–2838, I14 = 2839 (inchangée). **À signaler aux sessions qui avaient réservé 2827 à 2834.**
- Tables **+2** : RLS `tests_rls.rs` **+2** (`… + 13 + 5 + 2 + 2`) ; registre des données **+2** ; `TABLES_ELEVE` **+2** (`ent_internat_perte_droit_fait`, `ent_internat_tiers_autorise`, stratégie « reste »). Toutes : RLS (`ent_rls_poser`), `etablissement_id`, clés étrangères composites, `BEFORE TRUNCATE`, aucune colonne JSON ni BYTEA.
- `ent_internat_i3_fait` : colonne `tiers_id`, deux contraintes remplacées. `Action::TOUTES` **inchangé (275)** ; `NOMS_SOURCES` / `SOURCES` **inchangés (68)** ; événements **inchangés** (action de plus sur `internat.correspondant`) ; routes d'I3 **+6 (43)**, ajoutées au test « aucune route sans session » ; `MIGRATION_TAG` **inchangé (3808)** ; `Cargo.lock` : aucune dépendance ajoutée.
- Fonctions SQL : +3 (`ent_internat_perte_droit_poser`, `_personnel_seul`, `_lien_ecarte`) ; 3 remplacées (`_perte_droit`, `_user_vise`, `_remise`).

## 5. Écarts, limites

- Pas de test dédié pour la résolution de classe (`resoudre_classe`) : modification de tri sans changement de comportement hors le cas d'ambiguïté multi-années.
- **Revue à la pose** (R7, passe 4) toujours non faite (blocs d'autres socles).
- Le tiers n'a **pas de notification** (M4 d'I2, destinataires du socle 8) ; aucun écran (règle 27).
- Un tiers désigné **correspondant** reste soumis à la règle existante (contact validé du socle 4) ; le statut de tiers ne suffit pas.

## 6. Tests

- `tests_internat_i3` : **78 verts** sur base neuve (63 + 11 de la passe 4 déjà là + nouveaux : `s_m1_*`, `s_m2_*`, `s_m3_*` ×3, `s_infirmerie_*`, `s_requalification_*`, `s_p4_r1_*`, `p4_r8` réécrit) ; `ent-api` `tests_internat` : **5 verts** (403 et 404 des 6 routes du tiers, 43 routes dans « aucune route sans session »).
- Suite large `ent-db` (`internat rls registre migrat fusion conservation rgpd accords d036 roles isolation jour_utc sante`, base neuve `ent_i3w`, un fil) : **381 tests, 3 rouges au premier passage, tous corrigés puis rejoués verts** : (1) ordre alphabétique du registre des données et de `TABLES_ELEVE` ; (2) `m9_seuls_les_participants_confirmes…` (I2) posait les états `remis` / `parti_seul_autorise` : désormais dans une transaction en rôle de réplication, ce seul passage ; (3) `tests_sante::passages_famille_informee…` : **vrai défaut de ma part**, le prénom d'une personne est facultatif (`Option`), corrigé comme `Personne::libelle`.
- Clippy 1.99 `-D warnings`, **les trois commandes de la CI propres** (deux passages, le second après les derniers correctifs) ; `cargo fmt -p ent-types --check` propre ; `tiers.rs` (nouveau fichier) formaté par `rustfmt` ; `cargo fmt --all` non lancé ; `web/Cargo.lock` : `metadata --locked` vert.
- **0 nouvel avertissement** (rustc et clippy, tests et labo compris). Aucun `#[ignore]`, aucun test à l'heure réelle.
- Non rejoué en local : suite complète d'`ent-db`, frontend (sauté, règle 27).
- Incident local : `ent-api` ne se compile pas en parallèle sur 16 Go (rustc tué par l'OOM) ni avec le dossier `incremental` (disque) : compilé en `-j1`, sans débogage ni incrémental.

## 7. Correctifs de revue

Passe de correctifs demandée par le pilote sur la revue de la PR 108 (brouillon), partie de la tête distante `2340ca32`. **La migration 2827 n'étant pas versée, elle est modifiée** (2824 à 2826 non touchées). Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend sauté (règle 27).

### 7.1 Fait / non fait

| # | Point | État | Ce qui a été fait | Tests |
|---|---|---|---|---|
| E1 (élevé) | Parent privé d'autorité posé tiers | **fait** | `ent_internat_lien_ecarte` : un lien est écarté s'il est suspendu, en garde « aucune », échu ou pas commencé, **ou** sans autorité parentale **et** de type `mere`, `pere` ou `tuteur`. **Une seule fonction** : la branche « contact » de `ent_internat_remise` l'appelle (plus de prédicat dupliqué) ; effet à la pose et à la remise (autorité retirée après coup = porte fermée). Proche de lien `autre` sans autorité : posable. | `s_e1_*` ; `p4_r6_*` et `fc_fd_ff_*` adaptés (voir 7.3) |
| E2 (élevé) | « Personnel seul » trop large | **fait** | `ent_internat_personnel_seul(etab, eleve, user)` : rôle de personnel **et** aucun lien (compte ou fiche de la chaîne de fusions), aucun compte élève sur cet élève, aucun rôle ni casquette parent/élève. `ent_internat_user_vise` passe `p_sans_lien_perdu := NOT personnel_seul` (plus de court-circuit). **Conflit d'intérêt** à la pose et à la reprise d'un tiers (l'agent n'est ni le tiers, ni le demandeur, ni un proche de l'élève). | `s_e2_*` (agent père, lien suspendu : visé ; casquette parent : visé ; témoins ; conflits à la pose et à la reprise) |
| M1 | Infirmerie : jour de la remise | **fait** | `cloturer_passage_a` (horloge injectée ; `_for_actor` l'appelle avec l'heure réelle) évalue la remise au **jour local de l'établissement à la clôture**. | `m1_m2_*` (`tests_sante.rs`) |
| M2 | `fiche_urgence` : `peut_recuperer` | **fait** | Calculé par `personne_peut_prendre_l_eleve_tx` pour les responsables **et** les contacts, jamais réécrit (`fiche_urgence_a`, horloge injectée). | `m1_m2_*` |
| M3 | Tiers : notification, reprise | **fait** | Événement dédié `internat.tiers` à chaque acte, `destinataires: titulaires` (tous les titulaires, résolus par le socle 8), **sans motif ni demandeur** ; `reprendre_tiers_a` refusée tant que le demandeur actuel est en exercice. | `s_m3_revue_*` |
| M4 | Motif et demandeur | **fait** | Lus seulement avec `InternatSortiesDecider` ; `InternatLire` / `InternatAppel` voient la liste sans ces champs. | `s_m4_m5_*` |
| M5 | Famille : tiers contesté ou fermé | **fait** | Statut « en_cours_d_examen », identique pour toute cause (opposition, décision, demandeur ou lien écarté, interdiction) ; le personnel voit l'état réel. | `s_m4_m5_*`, `s_m3_desaccord_*` adapté |
| M6 | Garde statique de P4-R8 | **fait** | Test sans base : la migration 2827 contient le déclencheur et la fonction de garde, enregistrée dans `migrate.rs` (le test à base reste). | `p4_r8_garde_statique_*` |
| M7 | Classes | **fait** | Messagerie : « la seule classe de l'année courante » d'abord, puis la seule classe avec élèves affectés, sinon ambiguïté. `edt_masse.rs` : jointure aussi sur l'année (`x.jour` dans l'année de la classe). | `s_m7_*` (messagerie) ; `edt_masse` : pas de test dédié |
| F1 | Ré-opposition illimitée après « lever » | **fait** | `opposer_tiers_a` : refusée si `decision_apres_tx(Tiers)` (fait `tiers_levee`) existe ; le demandeur ou le personnel peut révoquer. | `s_f1_*` |
| F2 | `ORDER BY` de la remise | **fait** | Rang par porte (titulaire, contact, tiers) puis relation, dans l'`UNION ALL … LIMIT 1`. | couvert par les tests de remise |
| F3 | Type d'événement des tiers | **fait** | `internat.tiers` (au lieu de `internat.correspondant`). | `s_m3_revue_*`, test de confidentialité des événements |
| F4 | Durée et alerte | **fait** | `au <= du + 365` (contrainte et service, avant 366) ; alerte unique avant échéance (15 jours) dans la veille (`tiers_echeance`, `VeilleSortiesDto::tiers_a_echeance`), `TiersDto::echeance_proche`. | `s_f4_*` |
| F5 | Export RGPD | **fait** | `ClasseTable::colonnes_hors_export` : `motif` et `demande_par_personne_id` ne sortent pas de l'export du tiers (l'assemblage de l'extrait, lot G3, les retire). | `s_f5_*` |

### 7.2 Compteurs (règle 28, recalculés depuis `2340ca32`)

Tables **+0** (RLS, registre, `TABLES_ELEVE` inchangés) ; routes **+0** (43) ; `Action::TOUTES` **inchangé (275)** ; `NOMS_SOURCES` / `SOURCES` **inchangés (68)** ; `MIGRATION_TAG` **inchangé (3808)** ; `Cargo.lock` : aucune dépendance. Événements **+1 type** (`internat.tiers`, aucun registre de types à tenir). Fonctions SQL : `ent_internat_personnel_seul` change de signature (3 paramètres, l'ancienne est supprimée par `DROP FUNCTION IF EXISTS` : la migration n'étant pas versée) ; `ent_internat_lien_ecarte`, `_user_vise`, `_remise` remplacées. Acte `tiers_echeance` ajouté à la contrainte du journal I3.

### 7.3 Points à signaler au pilote

1. **Conséquence de la fonction unique (E1)** : la porte « contact » ne lit plus « sans autorité » pour tout lien, mais seulement pour `mere`, `pere`, `tuteur`. Un proche de lien `autre` sans autorité (grand-mère inscrite comme responsable) **redevient remis comme contact validé** (passe 4 / R6 le refusait). Deux tests de la passe 4 ont été adaptés (`p4_r6_*`, `fc_fd_ff_*`) ; **à confirmer**.
2. **Limite connue** : `ent_eleve_lien.lien` est un texte libre. Un lien saisi « Maman » ou « Papa » (hors vocabulaire `mere` / `pere` / `tuteur`) sans autorité n'est pas écarté. Les imports et inscriptions imposent le vocabulaire ; le saisir à la main reste possible. Contrainte de domaine possible (autre socle).
3. Aucun test dédié pour `edt_masse` (jointure sur l'année, même comportement hors multi-années).

### 7.4 Tests

Environnement : PostgreSQL 16 local, bases **neuves** (`ent_rev`, puis `ent_revw` pour la suite large), `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, Rust **1.99.0**, `--locked`, un seul `CARGO_TARGET_DIR`. Aucun test à l'heure réelle ni au jour UTC : jours fixes d'avril 2026, horloge injectée (`cloturer_passage_a`, `fiche_urgence_a`).

- `tests_internat_i3` complet : **85 verts, 2 rouges au premier passage**, tous deux dus à E1 (la décision change la porte « contact » pour un proche de lien `autre`, voir 7.3) : `p4_r6_*` et `fc_fd_ff_*` adaptés, puis verts.
- Nouveaux tests : `s_e1_*`, `s_e2_*`, `s_m3_revue_*`, `s_m4_m5_*`, `s_f1_*`, `s_f4_*`, `s_f5_*`, `s_m7_*`, `p4_r8_garde_statique_*` (sans base), `m1_m2_*` (`tests_sante.rs`).
- Suite large `ent-db` (`internat rls registre migrat fusion conservation rgpd accords d036 roles isolation jour_utc sante m1_m2`, base neuve, un fil) : **391 verts, 0 rouge** (et les binaires d'intégration filtrés : verts).
- Clippy 1.99 `-D warnings`, **les trois commandes de la CI propres** ; `cargo fmt -p ent-types --check` propre ; `tiers.rs` formaté par `rustfmt` (fichier ciblé) ; `cargo fmt --all` non lancé ; `web/Cargo.lock` : `metadata --locked` vert. **0 nouvel avertissement**, aucun `#[ignore]`.
- Non rejoué en local : `ent-api` `tests_internat` (la CI les passe), frontend (sauté, règle 27).

### 7.5 Tête et CI

Voir le message au pilote (tête finale et état de la CI de la PR 108).
