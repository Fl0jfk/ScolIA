# Registre des numéros de migration

Chaque session qui écrit des migrations reçoit une plage réservée ici, pour éviter les collisions entre branches (cf. `03_INSCRIPTIONS.md` §2.2 point 22). `MIGRATION_TAG` vaut toujours la plus haute migration présente.

| Plage | Attribution | Branche | État (2026-11-06) |
|---|---|---|---|
| 001–134 | socle 1 et lots S2-0 à S2-4 | `ENT` | sur ENT |
| 140–145, 150 | S2-3 inscriptions, S2-6 cahier de textes | `ent-integration-lot5` | en attente du lot I0 |
| 170–179 | S2-5 emploi du temps, remplacements | `ent-socle2-edt` | branche |
| 180–189 | S2-7 évaluations | `ent-socle2-evaluations` | branche |
| 190–199 | S2-8 communication | `ent-socle2-communication` | branche |
| 200–209 | S2-9 finances | `ent-socle2-finances` | branche |
| 210–219 | S2-11 santé | `ent-socle2-sante` | branche |
| 240–249 | socle 2 élève, lots E0–E1 | `ent-socle-eleve-e0-e1` | en cours |
| 250–254 | socle 3, lot I0 (correctifs S2-3) | `ent-socle-inscriptions-i0` | 250 écrite (`250_inscriptions_i0.sql`) ; 251–254 libres ; fusionnée dans `ent-integration-lot6` |
| 255–259 | correctifs revue finale socle 1 (RF-01, RF-04) | `ent-socle1-revue-correctifs` | 255 écrite ; fusionnée dans `ent-integration-lot6` ; `MIGRATION_TAG` = `255_email_confirmation` |
| 260–279 | socle 4 dossier administratif (conception : `04_ADMINISTRATIF.md` §16) | `ent-socle-administratif-a0` (lot A0), `ent-socle-administratif-a0-bis` (lot A0 bis) | **260** écrite (A0) ; **265** `etablissement_fuseau`, **266** `personne_reserve`, **267** `adresse_protegee`, **268** `lien_confirmation_voie`, **269** `inscription_responsable_reserve` écrites (A0 bis) ; 261–264 et 270–279 réservées aux lots A1 et suivants (la numérotation du §16 de la conception est décalée en conséquence) ; `MIGRATION_TAG` = `269_inscription_responsable_reserve` sur cette branche |
| 280–289 | socle 2 élève, lot E2 (identité versionnée) : 280 | `ent-socle-eleve-e2` | en cours (280 utilisée) |
| 290–299 | socle 2 élève, lots E3 et suivants | à venir | réservé |
| 300–329 | socle 3 inscriptions, lots I1 et suivants | `ent-socle-inscriptions-i1` (lot I1) | **300** `inscription_annuelle_prevue` (parcours, source_ref, sources `admission`/`reinscription` sur `ent_eleve_scolarite_annee`) et **301** `inscriptions_i1_dossier` (trace de l'inscription prévue sur le dossier, source `import` des liens en attente) écrites ; **302–304 libres** pour la suite du lot ; 305–329 réservées aux lots I2 et suivants ; `MIGRATION_TAG` = `301_inscriptions_i1_dossier` |
| 330–349 | socle 5 structure de l'établissement et année scolaire (conception : `05_STRUCTURE.md` §18) | `ent-socle-structure-conception` (conception seulement) | réservé |
| 350–369 | socle 6 personnels, services et équipes (conception : `06_PERSONNELS.md` §17 ; 350–351 et la partie accès de 357 touchent des tables du socle 1) | `ent-socle-personnels-conception` (conception seulement) | réservé |

Les plages proposées dans `02_ELEVE.md` (140–149, 160–169) et `03_INSCRIPTIONS.md` (200–219) sont remplacées par ce registre.
