-- Statut dédié pour les déclarations d'absence annulées à l'accueil (traçabilité sans DELETE).
-- `vs_absence_eleve.statut` est text sans CHECK : valeur `annulee` acceptée sans ALTER de contrainte.
-- Journal : 0061_vs_absence_eleve_annulee (when après 0060_metier_event).

UPDATE "vs_absence_eleve"
SET
  "statut" = 'annulee',
  "updated_at" = NOW()
WHERE
  "source" = 'accueil'
  AND "statut" = 'classee'
  AND "note_cpe" ILIKE 'Annulée par l%accueil%'
  AND "statut" IS DISTINCT FROM 'annulee';
