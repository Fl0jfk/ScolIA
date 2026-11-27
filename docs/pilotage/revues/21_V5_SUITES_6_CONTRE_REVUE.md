# Contre-revue — V5 passe 6 (PR 148)

Relecteur : review-sonnet, lecture seule, sur `wt-v5s6` (tête 67382631). Aucun test exécuté par le relecteur.

## Vérifié sans défaut
- Colonnes des conditions `export_si` (ent_note_archive, ent_devoir_archive, eva_moyenne_externe, eva_enseignement_externe) : existent (migrations 008, 022, 180, 472) ; `hors_export` contrôlé par le test d'assemblage.
- Cloisonnement par `etablissement_id` dans chaque sous-requête ; conditions statiques, élève lié en `$1` : pas d'injection.
- Moyen 2 : `eva_moyenne_externe` reprend exactement la lecture du bulletin (`evaluations_v1.rs:1618-1629`).
- Moyen 5 : règle de publication de `ent_note_archive` = règle de reprise (472) ; note sans devoir exclue ; auteurs retirés ; `ent_note_correction`, `ent_note_doublon_archive` hors export.

## Constats
- **Bloquant** : aucun.
- **M-A** : le garde « hors extrait par élève » ne refusait que `eleve*` ; une table rattachée par `resultat_id`, `enseignement_externe_id`, `rattrapage_id`, `user_id`, `personne_id`… aurait pu s'y cacher ; le test saute en silence sans base. Justification « contient élève » faible.
- **M-B** : `ent_devoir_archive` exportée sans condition (pas d'`eleve_id` : titres, classes, auteurs de tous les devoirs, y compris non publiés) ; invisible des gardes (`est_table_evaluation` ne couvrait pas `ent_devoir_`). Latent (pas d'assembleur G3), mais déclaration fausse.
- **M-C** : les tests du Moyen 2 n'isolaient ni `statut = 'moyenne'` ni `compte_moyenne_generale` (le cas négatif cumulait non publié et non compté).
- **M-D** : `ent_note_archive` suit la publication figée de l'archive, pas celle de l'évaluation reprise : un retrait de publication dans la nouvelle grille laisserait sortir la note.
- **F-1** (décision) : `ent_note_correction` / `eva_resultat_historique` non exportées alors que la famille a pu voir l'ancienne valeur : restriction à acter (droits des tiers/auteurs).
- **F-2** : `libelle` d'`ent_note_archive` exporté (texte libre enseignant), limité aux notes publiées : à documenter.
- **F-3** : pas de cas positif « publication explicite + `publie_le` posé » ni des trois statuts de note.
- **F-4** : `ent_note_periode` marquée `perso().export()` sans donnée personnelle ; `ent_eva_export` jamais testée dans un extrait du personnel.
- **F-5** : `est_colonne_d_etat` heuristique par nom.

## Verdict du relecteur
Versable après correctifs : M-B, M-A, M-C ; M-D à trancher (condition ou justification).

## Suite donnée par le pilote
- M-A, M-B, M-C, M-D corrigés par le pilote sur la branche (voir la section du même nom dans `21_EVALUATIONS_V3E_V5_LIVRAISON.md`).
  - M-D : condition ajoutée (si l'évaluation reprise existe et n'est plus publiée, la note ne sort plus) ; un devoir refusé par la reprise garde sa note exportée (D-069 : rien de visible ne disparaît).
  - M-A : liste de colonnes de rattachement partagée ; le test échoue en CI (`GITHUB_ACTIONS`) si la base manque.
- F-1 : décision D-069 du pilote : restriction maintenue (journaux internes, données d'auteurs et de valeurs non publiées) ; la valeur corrigée visible est exportée ; réserve DPO notée, comme pour `eva_dispense_port`.
- F-2 à F-5 : en file pour une passe ultérieure.
