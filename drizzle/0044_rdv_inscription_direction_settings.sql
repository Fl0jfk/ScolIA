-- Paramètres RDV (titre, intro, motif Google, notif, lieu, consentement, horizon) par direction.
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "title" text DEFAULT 'Rendez-vous d’inscription' NOT NULL;
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "intro" text DEFAULT '' NOT NULL;
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "event_title_pattern" text DEFAULT 'rendez-vous inscription' NOT NULL;
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "notify_email" text;
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "location" text DEFAULT '' NOT NULL;
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "consent_label" text DEFAULT 'J’accepte que mes coordonnées soient utilisées pour organiser ce rendez-vous d’inscription.' NOT NULL;
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "horizon_days" integer DEFAULT 60 NOT NULL;

-- Reprise des valeurs globales existantes (si déjà configurées).
UPDATE "rdv_inscription_direction" AS d
SET
  "title" = COALESCE(NULLIF(TRIM(c."title"), ''), d."title"),
  "intro" = COALESCE(c."intro", d."intro"),
  "event_title_pattern" = COALESCE(NULLIF(TRIM(c."event_title_pattern"), ''), d."event_title_pattern"),
  "notify_email" = COALESCE(c."notify_email", d."notify_email"),
  "location" = COALESCE(c."location", d."location"),
  "consent_label" = COALESCE(NULLIF(TRIM(c."consent_label"), ''), d."consent_label"),
  "horizon_days" = COALESCE(NULLIF(c."horizon_days", 0), d."horizon_days")
FROM "rdv_inscription_config" AS c
WHERE d."etablissement_id" = c."etablissement_id";
