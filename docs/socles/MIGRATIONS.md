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
| 250–254 | socle 3, lot I0 (correctifs S2-3) | `ent-socle-inscriptions-i0` | en cours |
| 255–259 | correctifs revue finale socle 1 (RF-01, RF-04) | `ent-socle1-revue-correctifs` | en cours |
| 260–279 | socle 4 dossier administratif (conception : `04_ADMINISTRATIF.md` §16) | `ent-socle-administratif-conception` (conception seulement) | réservé |
| 280–299 | socle 2 élève, lots E2 et suivants | à venir | réservé |
| 300–329 | socle 3 inscriptions, lots I1 et suivants | à venir | réservé |

Les plages proposées dans `02_ELEVE.md` (140–149, 160–169) et `03_INSCRIPTIONS.md` (200–219) sont remplacées par ce registre.
