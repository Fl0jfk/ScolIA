-- Fiches de dialogue v3 : niveau unique, ciblage élèves, snapshot INE/MEF.

ALTER TABLE "fd_campagne" ADD COLUMN IF NOT EXISTS "niveau_actuel" text;
ALTER TABLE "fd_campagne" ADD COLUMN IF NOT EXISTS "eleve_ids_cibles" jsonb NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE "fd_fiche" ADD COLUMN IF NOT EXISTS "eleve_ine" text;
ALTER TABLE "fd_fiche" ADD COLUMN IF NOT EXISTS "eleve_mef" text;

COMMENT ON COLUMN "fd_campagne"."niveau_actuel" IS 'Niveau unique (6e, 5e, 4e, 3e, 2nde, 1re, Tle)';
COMMENT ON COLUMN "fd_campagne"."eleve_ids_cibles" IS 'IDs élèves ciblés (vide = toutes les classes cochées)';
