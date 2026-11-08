# Registre des numéros de migration

Chaque session qui écrit des migrations reçoit une plage réservée ici, pour éviter les collisions entre branches (cf. `03_INSCRIPTIONS.md` §2.2 point 22). `MIGRATION_TAG` vaut toujours la plus haute migration présente (255 après le lot 7, car 250–259 sont plus hautes que les plages 170–219 ; la liste de `migrate.rs` reste en ordre croissant).

| Plage | Attribution | Branche | État (2026-11-06) |
|---|---|---|---|
| 001–134 | socle 1 et lots S2-0 à S2-4 | `ENT` | sur ENT |
| 140–145, 150 | S2-3 inscriptions, S2-6 cahier de textes | `ent-integration-lot5` | en attente du lot I0 |
| 170–179 | S2-5 emploi du temps, remplacements | `ent-socle2-edt` | 170–172 écrites ; fusionnée dans `ent-integration-lot7` |
| 180–189 | S2-7 évaluations | `ent-socle2-evaluations` | 180–182 écrites ; fusionnée (copie sans `target-s27`) dans `ent-integration-lot7` |
| 190–199 | S2-8 communication | `ent-socle2-communication` | 190–193 écrites ; fusionnée dans `ent-integration-lot7` |
| 200–209 | S2-9 finances | `ent-socle2-finances` | 200–206 écrites ; fusionnée dans `ent-integration-lot7` |
| 210–219 | S2-11 santé | `ent-socle2-sante` | 210–215 écrites ; **216 (module santé désactivé par défaut) ajoutée par l'intégration** ; fusionnée dans `ent-integration-lot7` |
| 240–249 | socle 2 élève, lots E0–E1 | `ent-socle-eleve-e0-e1` | en cours |
| 170–179 | S2-5 emploi du temps, remplacements | `ent-socle2-edt` | branche |
| 180–189 | S2-7 évaluations | `ent-socle2-evaluations` | branche |
| 190–199 | S2-8 communication | `ent-socle2-communication` | branche |
| 200–209 | S2-9 finances | `ent-socle2-finances` | branche |
| 210–219 | S2-11 santé | `ent-socle2-sante` | branche |
| 240–249 | socle 2 élève, lots E0–E1 | `ent-socle-eleve-e0-e1` | fait (240), dans E2 |
| 250–254 | socle 3, lot I0 (correctifs S2-3) | `ent-socle-inscriptions-i0` | 250 écrite (`250_inscriptions_i0.sql`) ; 251–254 libres ; fusionnée dans `ent-integration-lot6` |
| 255–259 | correctifs revue finale socle 1 (RF-01, RF-04) | `ent-socle1-revue-correctifs` | 255 écrite ; fusionnée dans `ent-integration-lot6` ; `MIGRATION_TAG` = `255_email_confirmation` |
| 260–279 | socle 4 dossier administratif (conception : `04_ADMINISTRATIF.md` §16) | `ent-socle-administratif-a0` (lot A0), `ent-socle-administratif-a0-bis` (lot A0 bis) | **260** écrite (A0) ; **265** `etablissement_fuseau`, **266** `personne_reserve`, **267** `adresse_protegee`, **268** `lien_confirmation_voie`, **269** `inscription_responsable_reserve` écrites (A0 bis) ; 261–264 et 275–279 réservées aux lots suivants (270–274 : lot A1, ligne suivante) (la numérotation du §16 de la conception est décalée en conséquence) ; `MIGRATION_TAG` = `269_inscription_responsable_reserve` sur cette branche |
| 270–274 | socle 4, lot A1 (droits et minimisation) | `ent-socle-administratif-a1` | **270** `conflit_interet`, **271** `eleve_interdiction`, **272** `contact_declarant` écrites ; 273–274 réservées (RLS du lot A14) ; `MIGRATION_TAG` = `272_contact_declarant` sur cette branche |
| 280–289 | socle 2 élève : 280 (lot E2, identité versionnée) ; **281–282 (lot E3 : identifiants, numéro)** ; **283–286 (lot E4 : scolarisations et statut dérivé, inscriptions, capacité et majorité, options)** ; 287–289 libres | `ent-socle-eleve-e2` (280) puis `ent-socle-eleve-e3-e4` (281–286) | 280 sur E2 ; 281–286 écrites, testées en local ; `MIGRATION_TAG` = `286_eleve_options` |
| 290–299 | socle 2 élève, lots E5 et suivants | à venir | réservé |

| 300–329 | socle 3 inscriptions, lots I1 et suivants | `ent-socle-inscriptions-i1` (lot I1) | **300** `inscription_annuelle_prevue` (parcours, source_ref, sources `admission`/`reinscription` sur `ent_eleve_scolarite_annee`) et **301** `inscriptions_i1_dossier` (trace de l'inscription prévue sur le dossier, source `import` des liens en attente) écrites ; **302–304 libres** pour la suite du lot ; 305–329 réservées aux lots I2 et suivants ; `MIGRATION_TAG` = `301_inscriptions_i1_dossier` |
| 330–349 | socle 5 structure de l'établissement et année scolaire (conception : `05_STRUCTURE.md` §18) | `ent-socle-structure-conception` (conception seulement) | réservé |
| 350–369 | socle 6 personnels, services et équipes (conception : `06_PERSONNELS.md` §17 ; 350–351 et la partie accès de 357 touchent des tables du socle 1) | `ent-socle-personnels-conception` (conception seulement) | réservé |
| 370–374 | D-022 : portée du professeur au groupe (370 : `ent_edt_creneau.groupe_id`) | `ent-d022-portee-groupes` | 370 écrite ; `MIGRATION_TAG` = `370_edt_creneau_groupe` ; 371–374 libres |

Les plages proposées dans `02_ELEVE.md` (140–149, 160–169) et `03_INSCRIPTIONS.md` (200–219) sont remplacées par ce registre.

## Mise à jour du 2026-11-07 (contradicteur du pilotage)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 216 | module santé désactivé par défaut | `ent-integration-lot7` | fait |
| 250 | inscriptions I0 | sur ENT (lot 6) | fait |
| 255 | changement d'e-mail confirmé (RF-01) | sur ENT (lot 6) | fait |
| 256–259 | failles faibles socle 1 | `ent-socle1-revue-faibles` | à vérifier |
| 260–269 | socle 4 A0 et A0 bis | `ent-socle-administratif-a0-bis` | fait |
| 270–274 | socle 4 A1 | `ent-socle-administratif-a1` | en cours |
| 280 | socle 2 E2 | `ent-socle-eleve-e2` | fait |
| 281–289 | socle 2 E3–E4 | `ent-socle-eleve-e3-e4` | en cours |
| 300–304 | socle 3 I1 | `ent-socle-inscriptions-i1` | en cours |
| 370 | D-022 (créneau rattaché à un groupe) — **pris sans réservation, empiète sur la plage du socle 6 (350–369) : 370–374 sont désormais réservés à D-022 et D-022 bis, et le socle 6 reste 350–369** | `ent-d022-portee-groupes`, `ent-d022-bis-enseignements` | fait / en cours |
| 3700–3799 | socle 7 documents, pièces et signatures (conception : `07_DOCUMENTS.md` §23), **si la règle « 3000 + 100 × socle » (Q-B2 d'`ARCHITECTURE_BRIQUES.md`) est adoptée** ; 3700–3712 attribuées, 3713–3789 réserve, 3790–3799 retrait des anciennes tables. La consigne de la session indiquait 4000–4099, qui reviendrait au socle 10 (Q-D1) | `ent-socle-documents-conception` (conception seulement) | réservé |
| 380–399 | socle 7, **repli** si Q-B2 n'est pas adoptée (380–392 attribuées, 399 retrait) | `ent-socle-documents-conception` (conception seulement) | réservé |

Règle : aucune session ne démarre sans sa ligne ici. À terme, `MIGRATION_TAG` sera calculé automatiquement à partir du répertoire des migrations (voir D-026).

## Mise à jour du 2026-11-07 (intégration du lot 8 bis)
Intégrées dans `ent-integration-lot8b` : 240 ; 260–269 (A0, A0 bis) ; 270–272 (A1 : 273–274 restent réservées) ; 280–286 (E2, E3, E4) ; 300–301 (I1) ; 370 (D-022). Liste de `migrate.rs` = répertoire `migrations/`, ordre croissant, aucune collision ; `MIGRATION_TAG` = `370_edt_creneau_groupe`. D-022 bis n'a pas ajouté de migration au-delà de 370 (371–374 restent réservées).

## Mise à jour du 2026-11-07 (conception du socle 8)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 3800–3899 | socle 8 notifications et diffusion (conception : `08_NOTIFICATIONS.md` §19 et §27), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 3800–3810 attribuées (paramètres, avis et destinataires, remises et résumés, préférences et appareils, diffusions, urgences, budget SMS et SMS entrants, retours fournisseurs, gels et liens, vues, reprise), 3811–3889 réserve, 3890–3899 retrait des anciennes tables (055, 057, 058 pour les journaux d'envoi, `ent_notification` de 102, 190, 191) après la durée d'archive | `ent-socle-notifications-conception` (conception seulement) | réservé |
| 400–419 | socle 8, **repli** si Q-B2 n'est pas adoptée (400–410 attribuées, 419 retrait) | `ent-socle-notifications-conception` (conception seulement) | réservé |
