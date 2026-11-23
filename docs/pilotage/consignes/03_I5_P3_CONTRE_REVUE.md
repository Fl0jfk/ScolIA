# Contre-revue de la passe 3 d'I5 (PR 127, tête d495fe98) — synthèse du pilote

**Aucun élevé.** Correct :
- `begin_lecture_coherente`, avec une exception de garde étroite ;
- 314 : gardes à `search_path` fixé, `setval`, caducité ;
- guichet ;
- refus d'écarter une proposition liée à une décision ;
- horloge des tests.

## Moyens
- **M1 : interblocage encore possible.** `compacter_rangs_tx` (`inscriptions_attente.rs` ~92-111, appelée après le verrou de l'offre) met à jour **tous** les dossiers de la liste derrière le partant. Le lot verrouillé à l'avance n'est qu'une fenêtre (`dispo.clamp(0,50)+5`).
  - Scénario : A (départ du rang 3) tient l'offre et compacte Y (rang 20). Pendant ce temps, B (décision sur Y) tient Y et attend l'offre.
- **M1b** : `publier_ou_programmer_tx` verrouille les dossiers au fil des décisions (`ORDER BY d.id`), sans tri par UUID de dossier.
- **M1c** : `modifier_offre_for_actor` verrouille l'offre puis la campagne, alors que `decider_tx` verrouille la campagne puis l'offre (antérieur au lot).
- **M1d** : les tests de concurrence (`tokio::join!` sans barrière) n'ont pas valeur de preuve. Le test des déplacements croisés est sérialisé par la campagne, même sur l'ancien code.
- **M2** : la réparation de chaîne de 314 est inatteignable : 313 échoue d'abord, en 23505, sur un historique partiel. **Décision** : pas de dérogation à la règle 25 ; la limite est documentée (aucune base de production n'a jamais porté l'ENT).
- **M3** : export. Un vœu créé par `retenir_voeu_tx` pour l'offre décidée sort dans l'export (`offre_id`, `rang`) **avant publication**. Le test d'export reste en partie tautologique (`colonnes_hors_export` « contient »).
- **M4** : export des conditions trop restrictif. Une décision programmée puis remplacée (ni publiée ni caduque) bloque à jamais l'export des conditions du dossier : il faut ne regarder que le bout de chaîne.
- **M5** : le test de la garde à `search_path` fixé ne distingue rien (table temporaire vide, prédécesseur aléatoire).
- **M6** : la caducité est trop stricte (égalité stricte avec `etat_cible(sens)`). Un `ajourne` dont le dossier passe en `attente_famille`, ou un `admis` qui devient `confirme`, est déclaré caduc : la famille n'est jamais informée, et la direction n'est pas alertée. **Décision D-069** : comparer à une **famille d'états compatibles** par sens, et alerter la direction quand une décision programmée devient caduque.

## Faibles
- F1 : `dossier_dto` garde une connexion tout en en demandant une autre, et l'état est lu avant le snapshot.
- F2 : `NOWAIT` dans `apres_transition_tx`, sans réessai.
- F3 : les lots sont verrouillés avant le contrôle d'écart. `campagne FOR UPDATE` → `FOR NO KEY UPDATE`.
- F4 : asymétrie guichet / famille sur `repondre_avant`. **Décision** : la famille peut répondre tant que l'expiration n'est pas constatée, comme aujourd'hui.
- F5 : 55P03 et 40P01 rendent tous deux `ConflitDossier` : un test qui accepte le conflit accepte aussi un interblocage.
- F6 : `place_no`, `origine` et `designee_le` sont exportés.
- F7 : le test 314 prend un verrou ACCESS EXCLUSIVE.
- F8 : aucun test direct de `begin_lecture_coherente` en RLS stricte.

## Revoir le plan (règle « trois tentatives »)
Trois passes n'ont pas suffi à fermer l'ordre des verrous. La passe 4 ne recolle pas de verrous : elle **change de modèle**.
1. Plus de compactage physique de `rang_attente` : le rang se calcule en lecture (`row_number()` sur `ordre_attente`, avec les seules sorties publiées côté famille).
2. Un verrou **unique et grossier par offre**, pris en premier pour toute écriture qui touche la liste d'attente ou les places (verrou consultatif transactionnel sur l'offre, ou `FOR NO KEY UPDATE` de la campagne puis de l'offre). Les dossiers viennent ensuite, triés par UUID.
3. Des tests de concurrence **déterministes** : une connexion externe tient un verrou, l'opération attend, puis aboutit ; plus une liste de plus de 6 dossiers.
