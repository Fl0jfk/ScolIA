# Socle 41 « Internat » — lot I2 : suites de la contre-revue de sécurité du pilote

Branche `ent-socle-41-code-i2-suites` (PR n° 104, brouillon vers `ENT`), partie de `origin/ENT` 7402da72. Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote (M4, M5 : voir `41_INTERNAT_I2_LIVRAISON.md` §5). Frontend non touché (règle 27).

**Migration 2825 `internat_i2_suites`** (rejouable : `CREATE OR REPLACE`, `ADD COLUMN IF NOT EXISTS`), enregistrée dans `migrate.rs` après 2824 et inscrite dans `docs/socles/MIGRATIONS.md`. 2824 n'est pas modifiée (règle 25). La 2825 que le lot I3 devait prendre est consommée ici : I3 prend 2826.

## Point par point

| # | Point | État | Ce qui a été fait | Tests (`tests_internat_i2` sauf mention) |
|---|---|---|---|---|
| 1 | L7 inversé | **fait** | 2825 : `el.date_sortie > nuit` (la date de sortie est le premier jour d'absence). | `l7_un_eleve_sorti_n_est_plus_attendu_des_son_premier_jour_d_absence` (sortie le 28 : attendu le 27, pas le 28) |
| 2 | M9 sorties | **fait** | 2825 : `remis` et `parti_seul_autorise` quittent l'attendu (avec `pret`, `parti`) ; `absent_depart` et `rentre` restent attendus. | `m9_…` étendu : remis, parti_seul_autorise (retirés, motif `sejour_scolaire`), absent_depart (attendu) |
| 3 | E3 dates | **fait** | Exclusion : interdiction non levée et non terminée avant la nuit, `du` ignoré ; objet `remise` ajouté. Même règle pour le masque « personne non identifiée ». | `e3_interdiction_non_levee_et_non_terminee_exclut_quelle_que_soit_sa_date_de_debut` (du postérieur, au expiré, levée, au = jour de la nuit) |
| 4 | E3 confirmation | **fait** | `rentre_famille` : autorité parentale, garde ≠ aucune, dates du lien, personne non fusionnée, masque élève protégé / interdiction sans personne, objet `remise`. | `e3_confirmation_rentre_famille_memes_filtres_que_la_liste_des_numeros` |
| 5 | Fusions en chaîne | **fait** | CTE récursive sur `fusionnee_dans` (interdictions et adresses protégées). | `e3_fusions_en_chaine_portent_les_mesures_de_toute_la_chaine` (A→B→C) |
| 6 | M3 couverture | **fait** | Transmission refusée tant qu'un attendu/ajouté n'est dans la liste figée d'aucun soir validé (logique des rappels, partagée). | `m3_transmission_refusee_tant_qu_un_attendu_n_est_dans_aucun_soir_valide` ; `m3_un_eleve_ajoute_hors_zone_bloque_…` |
| 7 | M3 rectificative | **fait** | L'idempotence compare les ensembles d'absents et de partis (colonne `eleves_partis`, 2825) et les repas. | `m3_rectificative_quand_l_ensemble_des_absents_ou_des_partis_change_a_totaux_egaux` |
| 8 | Test M1 vide | **fait** | Appel du soir validé, nuit passée, v1 transmise, chiffres changés : le refus du motif « diabète » est vérifié sur son message (« aucune donnée de santé »), et le même banc accepte un motif sans santé. | `m1_controle_sante_…` |
| 9 | `est_refus` | **fait** | Remplacé par `refus_de(r, fragment)` : message (ou variante de domaine) attendu ; plus aucun `Domain(_)` accepté. 50 appels repris avec leur message réel. Deux tests qui passaient pour une mauvaise raison ont été corrigés (annulation « déjà clos », isolation inter-établissements). | tout le module |
| 10 | Assertion PDF | **fait** | `contenu_pdf_appel` (lignes avant rendu) testé : « Excusé » seul, aucune cellule/paragraphe avec « sant » hors phrase de pied fixe ; le rendu part de ce contenu. | `m2_motif_sante_devient_excuse_…` |
| 11 | Note perdue | **fait** | Colonne `issue_note` (≤ 300, figée par la garde du cas, 2825), exposée dans `CasVueDto.issue_note` ; `annule_erreur` exige un motif écrit. | `la_note_de_cloture_d_un_cas_sans_lieu_est_conservee_et_l_annulation_exige_un_motif` |
| 12 | Portée de valider | **fait** | Appliquée à l'élève : correction après clôture, annulation pour erreur, ouverture tardive (périmètre entier dans la portée). | `la_portee_de_valider_s_applique_…` |
| 13 | Repas | **fait** | Nuit transmise : dîner et petit-déjeuner sur la liste figée ; la transmission elle-même compte sur la liste figée (pas de rectificative artificielle). | `le_diner_d_une_nuit_deja_transmise_se_compte_sur_la_liste_figee` |
| 14 | M1 | **fait** | Repli ASCII écrit à la main (marques combinantes, Latin-1/Latin étendu A, œ, æ, ß, pleine chasse, caractères invisibles) ; racines ajoutées (hospital, fractur, bless, douleur, nause, plaie, automutil, anorex). Toujours « une aide, pas une garantie ». | `domain-eleve` (376 verts) + notes M1 |
| 15 | Bornes | **fait** | `confie_a_user_id` doit avoir une adhésion ouverte ; routes de nuit : 400 hors 2000–2100 ; corps de la transmission lu en octets (vide ou `{}` accepté avec `Content-Type: application/json`, JSON invalide = 400). | `une_liste_ne_se_confie_qu_a_un_agent_de_l_etablissement` ; ent-api `internat_api_dates_de_nuit_hors_bornes_refusees_en_400`, `internat_api_la_nuit_parcours_droits_et_pdf` |

## Écarts et choix à valider

- **NFKD** : pas de dépendance ajoutée (D-062, `unicode-normalization` n'est pas dans le graphe de `domain-eleve`) ; équivalent écrit à la main. À remplacer par la crate si vous préférez la justification écrite d'une dépendance.
- **Couverture M3** : « présent dans une liste figée de soir validé » (comme les rappels), quel que soit le statut figé. Un retrait annulé après l'appel n'est donc pas rattrapé par ce contrôle.
- **Repas** : avant transmission, la lecture suit l'attendu calculé ; dès qu'une transmission existe, la liste figée.
- Le masque « interdiction sans personne identifiée » suit aussi la règle de dates pessimiste (extension du point 3).
- Les bornes de date ne couvrent que les routes de nuit et l'ouverture d'appel ; retraits et nuits exceptionnelles gardent leurs contrôles métier.

## Compteurs (règle 28)

Aucune table, route, action ni source ajoutée : SOURCES / NOMS_SOURCES, compteur RLS, `Action::TOUTES`, registre `TABLES_ELEVE` et `MIGRATION_TAG` (3808) inchangés. Colonnes ajoutées seulement.

## Vérifications locales (bases locales neuves, Rust 1.99.0, `--locked`)

- `tests_internat_i2` : 49 verts (base `ent_i2s`) ; gardes de registre, RLS (46), migrations (14), registre (19), fusion (20), `tests_d036` (7), actions (7) : verts sur une base neuve distincte.
- `domain-eleve` : 376 verts. ent-api `internat_api*` : 4 verts. `web/Cargo.lock --locked` : ok.
- Clippy `-D warnings` : les trois commandes de la CI propres (0 avertissement) ; `cargo fmt -p ent-types --check` propre.
- Non rejoué en local : suite complète d'`ent-db` et d'`ent-api` (laissée à la CI).

## Tête et CI

Voir le message au pilote (tête finale et état de la CI de la PR n° 104).
