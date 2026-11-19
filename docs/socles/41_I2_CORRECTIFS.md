# Socle 41 « Internat » — lot I2 : correctifs de la revue de sécurité

Branche `ent-socle-41-code-i2` (PR n° 102, brouillon). Point de départ : 2bf45cea (fusion d'ENT et inventaire des routes, 66 sources, faits par le pilote, conservés). Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote (M4 et M5 restent ouverts, voir `41_INTERNAT_I2_LIVRAISON.md` §5).

Migration modifiée : **2824** (non versée sur ENT). Aucune nouvelle migration, aucune table, aucune route, aucune action.

## Constat par constat

| Constat | État | Ce qui a été fait | Test(s) |
|---|---|---|---|
| E1 appel qui ne s'ouvre plus | **fait** | `ent_internat_attendu_calcule` : la branche `non_attendu` renvoie toujours `non_attendu` (motif NULL). | `e1_heberge_hors_rythme_avec_absence_annoncee_l_appel_s_ouvre` |
| E2 TRUNCATE | **fait** | Déclencheur `BEFORE TRUNCATE … FOR EACH STATEMENT` (42501) sur les 11 tables du lot et sur `ent_internat_presence_fait` (posé depuis 2824, 2820 intacte). | `e2_truncate_refuse_sur_les_tables_en_ajout_seul` (TRUNCATE … CASCADE par le déclencheur, SET de session puis UPDATE/TRUNCATE ; `ALTER TABLE … DISABLE TRIGGER` essayé avec le rôle applicatif **en mode strict seulement** : en mode propriétaire il serait légitime) |
| E2 garde d'insertion de la liste figée | **fait** | `BEFORE INSERT` sur `ent_internat_appel_attendu` : appel `ouvert` et `cree_le = transaction_timestamp()`. | `e2_liste_figee_insertion_seulement_a_la_creation_de_l_appel` |
| E2 garde de l'appel | **fait** | Seules transitions ouvert→clos, clos→valide ; `nb_ecarts`, `clos_par_user_id`, `clos_le` figés sur un appel clos ; `a_posteriori` et `cree_le` figés. | `e2_garde_de_l_appel_transitions_et_champs_figes` |
| E2 motif `reprise` | **fait** | Refusé (23514) pour un fait rattaché à un appel ; accepté sans `appel_id`. | `e2_motif_reprise_refuse_sur_un_fait_rattache_a_un_appel` |
| E3 numéros au surveillant | **fait** | Exclusion des personnes visées par une interdiction active de communication ou de contact (non levée, active à la date de la nuit, personnes fusionnées comprises) et des personnes à adresse protégée ; **élève protégé : aucun numéro**, `numeros_masques = true` (la vue renvoie à la direction / au CPE d'astreinte). Même filtre pour `confirme_par_personne_id` (refus). | `e3_numeros_sans_personne_interdite_ni_adresse_protegee_ni_eleve_protege`, `e3_garde_alternee_et_interdiction_visant_une_personne_non_identifiee` |
| M1 mots de santé | **fait** | Racines élargies dans `domain_eleve::mots_de_sante_detectes` (fièvre, crise, vomi, migraine, dépression, suicid, psy, TDAH, malade, angoisse, grippe, gastro, covid…), documentée comme **aide, pas garantie** ; appliquée à la note, au texte du motif de correction, aux notes du cas (étape, issue) et au motif de rectification. | `m1_controle_sante_…` + tests unitaires `domain-eleve` |
| M2 « Excusé » seulement | **fait** | `motif_public` : `sante` → `excuse` dans les vues (attendu, appel) et les événements (pose et annulation de retrait) ; la base garde le code réel ; PDF : « Excusé » seul et phrase de pied exacte (le motif de santé n'est pas détaillé ; « À l'infirmerie » = renvoi sans information médicale). | `m2_motif_sante_devient_excuse_…` |
| M3 transmission du matin | **fait** | Refus si un appel de la nuit est ouvert, si l'appel « soir » n'est pas validé, ou si la nuit n'est pas strictement passée. Table `ent_internat_transmission` versionnée (`version`, `motif_rectification`, `eleves_absents`) : rectificative version n+1 en ajout seul, motif obligatoire, précédente lisible ; seule la différence part vers la vie scolaire (absences annulées marquées `annulee`). Formule du petit-déjeuner unique (`effectif_repas_tx`) : dîner − absents − partis. | `m3_transmission_conditions_rectificative_versionnee_et_petit_dejeuner_aligne` |
| M6 ouverture ensembliste | **fait** | Deux requêtes (`INSERT … SELECT FROM ent_internat_attendu_calcule … WHERE périmètre` pour l'appel puis pour la liste figée), plus aucun recalcul par élève ; un écart entre les deux requêtes est refusé. | `e1_…` (liste figée = attendu calculé, zone, liste vide) |
| M7 a posteriori 48 h | **fait** | Au-delà de 48 h après la fin de la nuit (midi du lendemain, fuseau de l'établissement), `internat.appel.valider` est exigé. | `m7_appel_a_posteriori_borne_a_48_heures_sauf_droit_de_valider` |
| M8 conflit | **fait** | Mesuré sur `recu_le` ; tous les faits contradictoires reçus dans les 120 s avant le dernier sont signalés (`marques_en_conflit`). | `m8_conflit_mesure_sur_la_reception_…` (trois faits) |
| M9 sorties | **fait** | `ent_sortie_participant` : seuls `pret` (autorisation complète) et `parti` (départ effectif) sortent de l'attendu ; `inscrit`, `liste_attente`, `bloque`, `rentre`… dorment à l'internat. **Choix à valider** : `propose`, `remis`, `parti_seul_autorise` ne sont pas retirés non plus. | `m9_seuls_les_participants_confirmes_…` |
| M4, M5 | **hors lot, documentés** | Limites connues bloquant le « prêt pilote » dans `41_INTERNAT_I2_LIVRAISON.md` §5. | — |
| L1 | **fait** | Nuit exceptionnelle : un an au plus dans le futur (service). | `l1_…` |
| L2 | **fait** | Journal pour `lister_appels_a`, `lire_procedure_a`, `effectif_repas_a`, `lister_listes_a` et la lecture du journal ; colonne `eleve_ids UUID[]` pour `attendu` et `appel`. Les fonctions concernées reçoivent désormais `maintenant`. | `l2_lectures_journalisees_…` |
| L3 | **fait** | La garde du cas fige aussi `issue_lieu_code`, `issue_confirmee_par_personne_id`, `numero_intervention`, `appel_id`. | `l3_…` |
| L4 | **fait** | « Présent » ne ferme le cas que dans un appel couvrant la zone du lit ; « vu » ailleurs ferme avec le lieu réel `autre_zone`. | `l4_…` |
| L5 | **fait** | Une correction « absent » après clôture ouvre un cas. | `l5_…` |
| L6 | **fait** | Retrait `sante` de l'infirmerie : 30 nuits au plus, renouvelable. | `l6_…` |
| L7 | **fait** | `du_rythme` ne l'ignorait pas : corrigé (un élève dont `date_sortie` est antérieure à la nuit n'est plus attendu ; la date de sortie est son dernier jour). | `l7_…` |
| L9 | **fait** | Heure limite avant midi (ex. 00 h 30) = lendemain de la date de la nuit (la nuit court de midi à midi) ; l'ancien calcul rappelait aussitôt. | `l9_…` |

## Interprétations à valider

- M3 « nuit strictement passée » : le jour local de l'établissement est postérieur à la nuit (sinon la transmission de 7 h–8 h, nuit encore « courante » jusqu'à midi, serait impossible).
- E3 : une interdiction dont la cible n'est qu'un libellé (aucune personne identifiée) masque tous les numéros ; une interdiction levée après la nuit est traitée comme levée.
- Valeurs d'un retrait « santé » et événements : le code réel reste en base pour l'audit.

## Compteurs (règle 28)

Aucune table, route, action ni migration ajoutée : SOURCES / NOMS_SOURCES (66), compteur RLS, `Action::TOUTES` et `MIGRATION_TAG` **inchangés** ; les tests de garde correspondants (RLS, registre) sont verts en local.

## Vérifications locales (base de test locale uniquement, Rust 1.99.0, `--locked`)

- `tests_internat` (ent-db, feature `labo`) : 57 verts (dont les 25 nouveaux) ; tests RLS, registre et gardes partagés : 45 verts ; `domain-eleve` : 301 verts.
- Clippy `-D warnings` : les trois commandes de la CI propres ; `cargo fmt -p ent-types --check` propre.
- **Non rejoué en local** : les tests d'`ent-api` (l'édition de liens du binaire de test est tuée par manque de mémoire dans ce conteneur) et la suite complète d'`ent-db` : laissés à la CI.
- Changements d'API : `transmettre_matin_a` reçoit un motif facultatif (corps JSON facultatif de la route, pas de route nouvelle) ; `TransmissionDto.version`, `CasVueDto.numeros_masques`, `AppelLigneDto.marques_en_conflit`, `LectureSensibleDto.eleve_ids` (tous `serde(default)`). Frontend non touché (règle 27).

## Tête poussée et CI

- Commits des correctifs : 0286e8e5 (correctifs), 3278400a (fixture M9), 43e7ece6 (clippy d'un test), e3d8a329 (ce rapport), **591d8765** (test API de la nuit passé sur une nuit passée fixe : la transmission du matin refuse désormais une nuit non terminée, M3).
- Historique de la CI : run 423 (43e7ece6) remplacé par la poussée suivante (jobs annulés, sans signification) ; run 425 (e3d8a329) rouge pour **un seul test** (`ent-api` `internat_api_la_nuit_parcours_droits_et_pdf`, tranche labo 2/20, cause : test sur la nuit courante, corrigé en 591d8765 et rejoué en local : 3 tests `ent-api` internat verts) ; les 40 autres jobs étaient verts.
- Le pilote a ensuite fusionné ENT (b618a23e, socle 21) et poussé **7402da72**, qui contient 591d8765 : run 427 **vert** (formatage, audits, backend), selon le commentaire du bot de la PR n° 102. PR laissée en brouillon.
