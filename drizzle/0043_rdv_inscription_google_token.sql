ALTER TABLE "rdv_inscription_config"
  ADD COLUMN IF NOT EXISTS "google_refresh_token" text;
