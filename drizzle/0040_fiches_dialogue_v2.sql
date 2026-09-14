-- Fiches de dialogue v2 : options/LV élève, starter mode, snapshot identité, statuts enrichis.

ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "lv1" text;
ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "lv2" text;
ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "options" jsonb DEFAULT '[]'::jsonb;

ALTER TABLE "fd_campagne" ADD COLUMN IF NOT EXISTS "starter_mode" text NOT NULL DEFAULT 'famille_dabord';
ALTER TABLE "fd_campagne" ADD COLUMN IF NOT EXISTS "contact_pp_label" text;

ALTER TABLE "fd_fiche" ADD COLUMN IF NOT EXISTS "eleve_date_naissance" date;
ALTER TABLE "fd_fiche" ADD COLUMN IF NOT EXISTS "eleve_photo_key" text;
ALTER TABLE "fd_fiche" ADD COLUMN IF NOT EXISTS "parent_accord" jsonb;
ALTER TABLE "fd_fiche" ADD COLUMN IF NOT EXISTS "conflict_payload" jsonb;

COMMENT ON COLUMN "fd_campagne"."starter_mode" IS 'conseil_dabord | famille_dabord';
COMMENT ON COLUMN "fd_campagne"."contact_pp_label" IS 'Libellé canal contact PP (École Directe, Pronote…)';
