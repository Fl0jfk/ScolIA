# Contre-revue du lot V3 voyages (PR 117), rapport du relecteur

Rapport intégral de la contre-revue `review-sonnet` des correctifs V3 (copie de travail de l'époque : chemins et numéros de ligne indicatifs, à revérifier sur ENT). Lecture seule, aucun test exécuté par le relecteur.


**Verdict : aucun constat Élevé confirmé, aucun défaut confirmé qui expose un mineur interne dans un chemin atteignable aujourd'hui.** Il reste 1 risque potentiel à lever avant la V3b (M-1) et 5 constats Moyens. Je n'ai exécuté aucun test : tout est lu dans le code. Les chemins qui se complètent entre eux sont cités ligne par ligne.

## Moyen

**M-1 (risque potentiel, bloquant pour la V3b) : le départ constaté sans resynchronisation fait disparaître l'élève, et le CPE ne peut pas le rétablir.**
- Fait vérifié : grep sur tout `crates/` hors tests, aucune requête n'écrit `etat = 'parti'`. Le départ constaté n'est donc pas atteignable aujourd'hui. `voyages_exigences.rs:284` confirme que le moteur d'exigences ne touche jamais « parti ».
- Le jour où la V3b l'écrira sans appeler `internat_resynchroniser` (`voyages_argent_v3.rs:842`), la branche directe de `ent_internat_attendu_calcule` (654, « sejour_scolaire », vers la ligne 4496 du patch) retire l'élève dès que la sortie est « prête » ou « en cours ». C'est conforme à la règle 1, mais aucune ligne `ent_internat_retrait` n'existe. L'annulation par le CPE (`annuler_retrait_a`, `internat_nuit.rs:849`) a besoin d'un `id` de retrait : elle est impossible. M2 ne protège donc pas ce cas, et la seule parade est la route retour-anticipé, qui exige `VoyagesParticipants` (porteur ou direction). Un élève retiré à tort n'est cherché par personne : c'est le sens non sûr.
- Correctif :
  - Exiger dans la V3b l'appel de `internat_resynchroniser` dans la même transaction, avec un test qui échoue sans lui.
  - Mieux : ne garder qu'une source de vérité, c'est-à-dire que la branche directe exige un retrait effectif posé. Sinon, donner au CPE une marque « rétabli » qui ne dépend pas d'une ligne de retrait.
  - Le rapport §5.5 dit « attendu correct sans retrait » : c'est faux pour un élève parti, il faut le corriger.

**M-2 : le seuil de seconde validation se contourne par fractionnement.**
- `voyages_argent_v3.rs:718` compare chaque demande isolément (`e.montant > seuil`).
- Scénario : une seule personne, avec le droit, enchaîne 149,99 € × N (sources « apel » ou « autre ») sur le même participant et le même payeur. Aucune seconde personne n'intervient.
- Correctif : seuil sur le cumul (aides appliquées + demandes en attente) par sortie, participant et payeur.
- Autre contournement : la détection « fonds social » repose sur `source` saisie par le demandeur. Une aide étiquetée « autre » échappe à la règle fonds social.

**M-3 : la règle 3 ne couvre que l'aide et le report.**
- `exiger_decideur_libre` / `ent_sortie_argent_decideur_interdit` n'apparaissent que dans `voyages_argent_v3.rs` (grep).
- Restent sans la règle « jamais pour son propre enfant » : `voyage_plan_accorder`, `voyage_avoir_decider`, `voyage_avoirs_annulation_collective`, `voyage_versement_annuler` (`voyages_argent_repo.rs:1143`, 1252, 1456, 1533), le pointage et les versements, les frais réels, les acomptes.
- Le périmètre était peut-être volontairement limité ; la formulation « un pouvoir sur l'argent » est générale. Il faut le décider explicitement (voir Questions ci-dessous).

**M-4 (potentiel) : la date d'annulation d'une sortie est lue dans `modifie_le`.**
- `voyages_argent_v3.rs:869-894` : `au_sortie = min(au, jour_modif - 1)` avec `jour_modif = ent_aujourdhui_etab(s.modifie_le)`.
- Aujourd'hui `modifie_le` ne bouge plus après l'annulation, car porteur (`voyages_repo.rs:2056`) et étapes (`:2119`) sont refusés sur un état terminal. Si un futur UPDATE de la sortie le décale puis qu'une synchronisation repasse, les nuits suivant l'annulation seraient retirées. Un élève serait alors absent de l'appel alors que la sortie est annulée.
- Correctif : colonne `annulee_le`, ou lecture de l'instant dans le journal de la sortie ; test avec `modifie_le` déplacé.

**M-5 (conditionnel) : modification en place des migrations 653 et 654.**
- `migrate.rs:576-587` refuse un fichier modifié (`FichierModifie`) sur toute base où l'ancienne version a été appliquée. L'échec est bruyant, pas silencieux.
- « Non versées » n'est pas vérifiable par moi : les migrations 653 et 654 sont absentes de l'arbre `/home/user/ScolIA` (Glob négatif). À confirmer qu'aucune base persistante (labo, pilote, cache de CI) n'a appliqué l'ancienne version, sinon il faut une migration de rattrapage.

## Faible

- **F-a. Verrou avant droit** : `verrou_sortie` est pris avant l'autorisation sur les nouvelles routes (`voyages_argent_v3.rs:1534`, 1591, 1673, 1723) ; n'importe quel compte authentifié peut tenir brièvement le verrou consultatif d'une sortie. `consigne_motif` (`voyages_argent.rs:218`) n'a aucun contrôle de droit (texte statique, sans fuite). Ordre cible : personnel → droit → verrou → existence.
- **F-b. Retour anticipé**
  - La date de retour est l'instant de saisie (`:1735-1741`, et 654 `ent_sortie_retrait_effectif`). Une saisie tardive laisse l'élève retiré les nuits intermédiaires. Prévoir un `retour_le` borné (≤ maintenant, ≥ départ), tracé.
  - Aucune vérification de l'état de la sortie : l'appel passe aussi sur une sortie « rentrée », « close » ou « annulée » (la synchronisation sort sans rien faire).
  - La famille « remis » / « parti seul » n'est pas couverte (déjà reconnu au §5.5).
- **F-c. Demandes en attente** : aucune expiration, et elles peuvent être validées des mois plus tard. Les montants et l'état sont revérifiés à la validation, pas le contexte. L'annulation d'une aide ou d'un report (`:1141`, `:1333`) se fait seul, sans seuil, quel que soit le montant. Les refus par la règle 3 ne sont pas journalisés.
- **F-d. Défense en profondeur de la base**
  - `ent_sortie_internat_retrait_fin.retrait_id` n'a pas de clé étrangère.
  - Le FK `ent_sortie_argent_demande_participant_fk` ne lie pas le participant à `sortie_id`.
  - `ent_sortie_aide.demande_id` n'est pas contrôlé en cohérence (participant, montant) avec la demande par la base.
- **F-e. F4 incomplet**
  - `voyage_modifier`, `voyage_commande` et `changer_etat_participant` appellent la resynchronisation avec `Utc::now()` (`voyages_repo.rs:1584`, 1960, 2539) et `modifie_le = now()` côté base. Elles ne sont donc pas testables à instant fixe.
  - Le test `internat_annulee_apres_le_depart…` (`tests_voyages_v3.rs:1173`) utilise `now()` de la base ; il peut vaciller autour de minuit, heure de Paris.
  - Ce n'est pas du « jour UTC » : `ent_aujourdhui_etab` est utilisé partout.
- **F-f. Sortie « rentrée » avec participant resté « parti »** : `internat_resynchroniser` sort sans effet (`:857`). Le retrait jusqu'à `au` subsiste alors que le groupe est rentré. Incohérence de données, sens non sûr.
- **F-g. Retour arrière de la reprise** : `voyages_reprise_repo.rs:916` annule la sortie sans resynchroniser. Inatteignable aujourd'hui (refusé s'il existe des faits de participants, participants « inscrit » ou « rentré »), mais cela contredit « une seule fonction appelée partout ».
- **F-h. F3 partiel** : `DROP TRIGGER` + `CREATE TRIGGER` rejoués sur les tables de 653 (ACCESS EXCLUSIVE à chaque rejeu). Le test de rejeu n'est pas exécuté en local.
- **F-i. 654 duplique le corps de la fonction de 2826** : une divergence future est écrasée silencieusement. Ajouter un test qui compare les deux corps hors des deux conditions « 654 ».
- **F-j. M2 tient pour tout le séjour** : une annulation CPE bloque la repose du retrait du participant pour toute la sortie (`ecartes`, `:950`). C'est le sens sûr (faux « attendu » possibles, jamais faux « retiré »), mais cela déclenche de fausses alertes si l'élève repart réellement.

## Conforme (vérifié)

1. **Règle 1**
   - La branche directe (654) exige `p.etat IN ('parti','remis','parti_seul_autorise')`, sans « pret ».
   - `ent_sortie_retrait_effectif` impose « parti » ou « rentré » avec nuits antérieures au jour de retour.
   - Le test I2 est corrigé (`pret → attendu`), et le nouveau test échouerait avec l'ancienne branche.
2. **Règle 2** : appel de la resynchronisation dans la même transaction pour :
   - `voyage_modifier` (`voyages_repo.rs:1583`, sur dates ou état) ;
   - `voyage_commande` (`:1959`, sur changement d'état) ;
   - désistement et retrait d'un participant (`:2539`) ;
   - retour anticipé et route de synchronisation.

   Les ajouts et réintégrations de participants n'en ont pas besoin (aucun retrait). La réouverture de liste prête → ouverte reste dans les états actifs. Les exigences ne touchent jamais « parti ». `ETATS_SORTIE_ACTIFS` = ouverte, prête, en cours ; une sortie annulée ne garde que les nuits déjà passées dehors. Les dates modifiées passent par l'API (`voyage_modifier`, test `:1208`).
3. **Règle 3**
   - Contrôle applicatif plus déclencheur de base (23514) sur demande, aide, report, annulation et issue « appliquée ».
   - La seconde personne est distincte du demandeur (applicatif et déclencheur), a le droit `voyages.argent` (même établissement via TenantTx / RLS) et n'est pas liée à l'élève ni au compte.
   - Une demande déjà tranchée donne un 409 : issue unique (UNIQUE sur `demande_id`) et verrou de sortie.
   - Les montants sont revérifiés à la validation.
   - Un 202 n'écrit aucune aide ni aucun report : seules les lignes appliquées comptent.
   - Seuil strictement supérieur à 150 € ; fonds social toujours soumis par défaut ; motif obligatoire.
   - Tests présents : demandeur refusé, parent refusé, doublon refusé.
4. **M1** : contrôle de net négatif sous verrou de sortie (`voyages_argent_repo.rs:1178`), test présent. **M4** : aide, report et pointage refusés sur sortie close, annulée ou refusée (`exiger_sortie_ouverte_a_l_argent`), test pour les trois états ; les annulations restent permises.
5. **M2, qui peut l'utiliser** : `annuler_retrait_a` exige `InternatGerer` ; l'infirmerie est limitée au motif « sante ». Le seul effet possible est de rendre l'élève attendu : impossible de s'en servir pour le faire disparaître. Il est tracé (acteur, événement sans nom ni montant). Le retour anticipé agit aussi dans le sens sûr (« rentré » ⇒ attendu) et l'instant vient du serveur (non falsifiable).
6. **F1** : clé 8 à 64 caractères, contenu comparé, concurrence testée (`tokio::join!`) ; validation, refus et annulation en compare-et-écrit. **F5** : trace de l'acteur sur « avoirs à revoir ».
7. **RLS et tables** : 4 tables en 653 et 2 en 654 passent par `ent_rls_poser` ; `tests_rls.rs` passe à +6 ; registre des données à jour ; ajout seul (UPDATE, DELETE, TRUNCATE) ; `etablissement_id` dans toutes les clés étrangères.
8. **Entrées** : motif ≤ 500, référence ≤ 40, clé validée, montant borné, `motif_code` sur liste blanche, tout en `invalide()` (pas de 500). Le test d'isolation entre établissements existe.
9. **Autre** : la migration 654 passe après 2826 dans `migrate.rs` (ordre OK) ; aucun `etablissement_id` n'est accepté du client.

## Tests

- **Prouvent la règle (échoueraient sans)** : règle 1 (prêt → attendu), règle 2 (désistement, retrait, annulation, réouverture, dates via l'API), M2 (branche « live » via `etat = 'en_cours'`), M1, M4, F1, règle 3 (seuil 150,00 / 150,01, fonds social, 202 sans écriture).
- **Heure** : instant fixe `2031-03-12T11:00Z`, nuits relatives au jour du cadre, aucun jour UTC. Exceptions en F-e.
- **Manquent**
  - aucun test HTTP (`ent-api`) pour les 5 nouvelles routes (201/202/403/404/422) ; seul `ent-db` est testé ;
  - fractionnement du seuil ;
  - annulée + `modifie_le` déplacé ;
  - retour anticipé sur sortie close ;
  - saisie tardive du retour ;
  - départ « parti » sans resynchronisation (M-1) ;
  - cohérence 654 / 2826.

## Questions

- La règle 3 doit-elle s'étendre aux plans, avoirs, annulations de versement, frais et acomptes (M-3) ?
- Les migrations 653 et 654 ont-elles été appliquées sur une base persistante (M-5) ?


## Réponses du pilote (D-069)
- **M-3** : oui. La règle « jamais pour son propre enfant » s'étend à **tout** pouvoir sur l'argent : plans, avoirs (décision et annulation collective), annulation de versement, pointage et versements, frais réels, acomptes.
- **M-5** : sans objet. 653 et 654 sont versées sur ENT et ne changent plus ; toute correction passe par une nouvelle migration.
