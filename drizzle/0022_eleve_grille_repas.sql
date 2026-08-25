-- Grille repas Lun–Ven sur scolarité (Administratif → Passage).
ALTER TABLE "eleve_scolarite" ADD COLUMN IF NOT EXISTS "grille_repas" jsonb;
