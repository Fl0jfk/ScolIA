-- Traitement administratif après validation direction (rectorat / RH).

ALTER TABLE "absence"
  ADD COLUMN IF NOT EXISTS "admin_treated_at" timestamp with time zone,
  ADD COLUMN IF NOT EXISTS "admin_treated_by" text,
  ADD COLUMN IF NOT EXISTS "admin_note" text;
