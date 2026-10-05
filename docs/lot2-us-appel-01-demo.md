# Lot 2 — US-APPEL-01 (démo W2)

Outil de travail ScolIA — **pas** registre légal (Charlemagne noop). Proxy CPE démo : **`direction`** (`florian+direction@h-me.fr`), pas de compte `cpe`.

## Parcours

1. **Professeur** — `/vie-scolaire/absences?tab=appels`  
   Choisir un créneau du jour (EDT, **ses créneaux** via `enseignantNom`) → ouvrir l’appel → marquer ≥1 élève absent → enregistrer → **clôturer**.

2. **Système** — `vs_absence_eleve` (`source=appel`, lié `appel_id` + horaires créneau) ; événement `attendance.call_completed` dans `metier_event`.

3. **Direction (proxy CPE)** — `/vie-scolaire/absences?tab=consulter`  
   L’absence apparaît (« Appel de classe ») sans re-saisie.

4. **Brain** (optionnel) — `open_appel` / `save_appel_lignes` / `close_appel` (module `vs-appels`, confirmation UI).

## Règles métier Lot 2

- Retour **présent** : ne supprime pas une absence avec motif / traitement → `classee` + historique.
- **Anti-double** : un seul signal appel actif par élève / jour / créneau horaire (groupe + classe).
- Famille : hors détail Lot 2 (visibilité après qualification CPE — non démo ici).

## Tests

```bash
npm run test:vs-appels-slot
npm run test:vs-appels-close
npm run test:vs-absences-hub
```
