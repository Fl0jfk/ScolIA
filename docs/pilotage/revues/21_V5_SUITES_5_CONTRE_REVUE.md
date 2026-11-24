# Contre-revue des suites V5, passe 5 (PR 143, tête f6cbfd2f) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé** (aucune fuite d'une note non publiée ni d'un champ
interne par les tables `eva_*` ; `extrait_eleve` exécute réellement la condition et retire les colonnes). Conditions
`export_si` de M3 et M5 correctes ; M4, F4 justifiés ; `eva_resultat_externe` aligné sur la lecture famille.

## Moyens
1. **Moyennes externes exportées inintelligibles** : `eva_enseignement_externe` retirée (F3) → l'export donne la valeur sans
   matière ni organisme, alors que la famille la voit avec sa matière au bulletin. → **Corrigé par le pilote** : table
   rouverte, `export_si` sur l'existence d'une moyenne externe **publiée**, `hors_export` (`referent`, `declare_par`,
   `compte_moyenne_generale`).
2. `eva_moyenne_externe` : `export_si` sur `publiee_le` seul, la lecture bulletin ajoute `compte_moyenne_generale` et
   `statut = 'moyenne'` : à aligner ou justifier (en file).
3. Justifications de M3/M5 (« communiquées à la famille ») non étayées : aucune lecture famille de `eva_rattrapage` ni de
   `eva_moyenne_statut`. **Décision D-069 du pilote** : on les garde exportées au titre du **droit d'accès (art. 15)**, car ce
   sont des données personnelles de l'élève, à condition qu'elles ne révèlent rien de non publié (conditions actuelles) ;
   les commentaires sont à reformuler (en file).
4. `eva_dispense_port` retirée : données personnelles (éventuellement de santé) sans canal d'accès tant que G3 n'existe pas →
   **réserve DPO** : « retirée de l'export automatique, fournie sur demande manuelle » (à acter au §12, en file).
5. `ent_note_correction` (motifs, auteurs), `ent_note_archive` (notes de reprise, éventuellement jamais publiées),
   `ent_note_doublon_archive` (copies écartées) exportées sans condition (antérieur) → à retirer ou conditionner ; étendre
   M1/M2 aux `ent_note_*` (en file, priorité avant G3). `ent_note_periode` : sans `eleve_id`, à déclarer hors export par élève.

## Faibles (en file)
6. M2 : pas de marqueur « non publié » pour `eva_resultat_item` et `eva_evaluation_restriction` ; auteurs `w.direction` des
   externes non interdits.
7. M1 : une prose ≥ 20 caractères suffit ; état détecté par noms de colonnes ; test réel silencieux sans base.
8. `ent_eva_export` (journal du personnel) exporté `tout_exportable` : à déclarer hors export par élève.
9. Test `la_table_de_reprise_des_appreciations_n_est_pas_exportee_sans_condition` tautologique.
