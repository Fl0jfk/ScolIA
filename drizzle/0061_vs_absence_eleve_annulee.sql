-- Statut dédié pour les déclarations d'absence annulées à l'accueil (traçabilité sans DELETE).
-- Numéro 0061 : volontairement au-dessus de 0060_metier_event.sql (doublons 0053–0058 traités ailleurs).

UPDATE "vs_absence_eleve"
SET
  "statut" = 'annulee',
  "updated_at" = NOW()
WHERE
  "source" = 'accueil'
  AND "statut" = 'classee'
  AND "note_cpe" ILIKE 'Annulée par l%accueil%'
  AND "statut" IS DISTINCT FROM 'annulee';
