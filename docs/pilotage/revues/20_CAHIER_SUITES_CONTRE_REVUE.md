# Contre-revue des suites cahier C8/C9 (PR 140, tête ab521fd1) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** Les 4 points (REVOKE de `ent_cdt_reprise`, gel des
4 tables filles, gel qui échoue fermé, compte de requêtes) sont réellement corrigés avec tests discriminants. Règle 38
conforme (2 fonctions avec `search_path`, relations `public.`, SECURITY INVOKER, 2419 en fin de tableau, rejeu idempotent).
Aucun contournement du gel des filles trouvé (NEW et OLD examinés, mère absente → refus). Aucune assertion affaiblie.

## Moyens
1. **Fusion d'élève impossible** dès qu'une note d'accompagnement (et déjà une cible de travail ou une variante d'élève) est
   dans une année close : le gel (23514) fait échouer toute la transaction de fusion, et de défusion.
   → **Corrigé par le pilote** : `ent_cdt_note_accompagnement`, `ent_cdt_travail_cible`, `ent_cdt_variante_eleve` passent en
   `dc(…)` dans `domain-eleve/src/fusion.rs` : seules les lignes d'une année non figée suivent l'élève conservé, les autres
   restent sur l'absorbé. Test `c8_suites_fusion_ne_deplace_que_les_lignes_d_une_annee_ouverte`.
2. **Règle « jamais supprimé » ajoutée sans demande** sur groupe, appartenance, note d'accompagnement (données sensibles :
   minimisation, effacement). → **Décision D-069 du pilote** : retirée ; le gel d'année protège les années closes, les FK
   RESTRICT les liens. Test « hors gel » réécrit (ajout seul de `continuite_cours`, TRUNCATE refusé).

## Faibles
1. Pas de garde TRUNCATE sur les 4 tables → **corrigé par le pilote** (`ent_cdt_sans_truncate`, 42501).
2. Assertions DELETE de `verifier_gel` non discriminantes à cause de « jamais supprimé » → devenues discriminantes (règle retirée), sauf `continuite_cours` (ajout seul).
3. Aucun test de déplacement d'une ligne vers ou depuis une année close (le code l'examine) : en file.
4. Test F3 : n'exige que 23514, pas le message : en file.
5. Messages des filles non traduits en 409 (`cahier_c8::db` ; `cahier_c5` utilise `ref_commun::db`) : en file.
6. Branche `ent_app` du REVOKE non exercée ; nom codé en dur ; ajouter `rolname <> current_user` : en file.
7. Doc : plage 2400–2419 épuisée (C14 et retrait des anciennes tables sans numéro) : **à réattribuer par le pilote** (registre des plages).
8. `ent_cdt_echeance_eleve` et `ent_cdt_delai_demande` hors gel : à décider (prudent : les geler).
9. Test F3 : `ALTER TABLE ent_annee_scolaire ENABLE ROW LEVEL SECURITY` (verrou ACCESS EXCLUSIVE) sur une table commune : risque de blocages en parallèle.
10. F4 : égalité stricte des comptes de requêtes entre deux jeux de données : fragile, préférer une borne.
