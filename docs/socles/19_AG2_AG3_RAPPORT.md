# Socle 19 — rapport des lots AG2 (réduit) et AG3 (code)

**Date :** 2026-11-12 — **Branche :** `ent-socle-19-code-2` (partie de `origin/ENT` 50f7e0eb ; rien poussé sur `ENT`).
**Niveau atteint :** *implémenté et testé en local* ; CI : voir le run en bas (une seule, règle 16). **Non validé** par Florian, **pas prêt pilote**, rien déployé, aucune donnée de production touchée.

## 1. Livré

| Élément | Où |
|---|---|
| Migration **2386** : `ent_agenda_lieu_regle` (mode par salle, valideur, délai, durée max, horizon), `ent_agenda_reservation` (**EXCLUDE gist** anti-double réservation, statuts demandee/confirmee/refusee/annulee/tombee, FK composites), `ent_agenda_reservation_fait` (ajout seul) ; RLS posée, registre des données à jour | `migrations/2386_…` |
| Service : recherche de salle libre, planning borné, mes réservations, réserver (unitaire/série, rapport par occurrence), annuler, valider/refuser, règle par salle, révision D-058 | `ent-db/src/agenda_lieux.rs` |
| Actions `agenda.lieux.{lire,reserver,valider,gerer,parametrer}` | `ent-auth` |
| Routes `/agenda/lieux/*`, `/agenda/reservations*`, `/agenda/disponibilites`, `/agenda/creneau-commun` ; revue D-058 dans la boucle des rappels | `ent-api` |
| Écran `/web/salles` (recherche, réservation, mes réservations, annulation) + lien depuis « Mon agenda » | `web/…/agenda/salles.rs` |
| AG2 réduit : libre/occupé et créneau commun **calculés** depuis `edt_plages_libres` ; santé et social (`infirmerie`, `psy`) → `non_communique` (R-AG37) | `agenda_libre_occupe`, `agenda_creneau_commun` |

## 2. D-058 et ports du socle 10

Aucune copie de l'EDT : `edt_occupation_lieu`, `edt_occupe`, `edt_salles_liberees` et `edt_plages_libres` sont lus à chaque décision. Toutes les salles du référentiel sont réservables, classes comprises ; sans ligne de règle, une salle est « toujours » (le drapeau `reservable` du socle 5 n'est pas consulté). Un cours prime : réservation refusée si un cours occupe le créneau ; si un remplaçant est nommé après coup, la réservation passe à `tombee`, le demandeur reçoit le motif **public** (« le cours de 4B est rétabli ») et trois salles libres de rechange. Le motif d'absence n'apparaît nulle part (test).

## 3. Règle 14 — défauts du Next.js fermés

| Défaut | Corrigé par | Test |
|---|---|---|
| **NJ-62** toutes les réservations lisibles par tout compte, sans borne | plage de 42 jours au plus, droit `agenda.lieux.lire` (parent refusé), commentaire réservé au demandeur, bénéficiaire et gestionnaire | `nj62_planning_borne_et_commentaire_reserve_aux_parties` |
| **NJ-76** tout compte peut annuler/modifier la réservation d'un autre | seuls demandeur, bénéficiaire, gestionnaire ; tout autre : 404 (annulation et décision) ; gestionnaire : motif obligatoire, communiqué | `nj76_annuler_reservation_d_autrui_est_404_et_gestionnaire_motive` |
| ND-01 double réservation, ND-02 succès partiel silencieux, ND-03 dates passées | exclusion en base + verrou par salle ; rapport par occurrence, écriture partielle seulement sur confirmation ; créneau passé et fin avant début refusés | `conflits_serie_a_rapport_et_creneau_passe` |
| règle par salle, validation, cours prioritaire | modes toujours / sur validation / jamais ; valideur désigné | `regles_par_salle_et_cours_prioritaire` |
| D-058 de bout en bout | annulation → salle libre → réservation → remplaçant → `tombee` + avis | `cours_supprime_libere_puis_remplacant_fait_tomber_la_reservation` |
| AG2 | libre/occupé sans motif, santé non communiquée, parent refusé | `disponibilites_calculees_sans_motif_et_sans_sante` |

## 4. Limites honnêtes

* **AG2 réduit** : pas de disponibilités déclarées (migration 2383 non construite) ; le calcul repose sur l'EDT seul (pas d'événements ni de rendez-vous dans le libre/occupé).
* Seules les **annulations** libèrent une salle ; « déplacée » et « neutralisée » viendront avec T3/T4 du socle 10.
* Pas de modification d'une réservation (annuler puis réserver), pas d'écran de gestion des règles ni de validation côté web (routes prêtes), pas de notification de rappel avant une réservation.
* Écran non essayé au navigateur (axe-core, lecteur d'écran) ; la révision D-058 tourne toutes les 5 minutes, pas à l'instant de la nomination.
* Test local `rs3_02` (finance) échoue dans un enchaînement sur la base partagée et passe seul : interférence locale, hors périmètre.
