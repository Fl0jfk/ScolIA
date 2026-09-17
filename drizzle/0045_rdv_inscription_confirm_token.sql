-- Double opt-in RDV inscription : token de validation + statut pending.
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "confirm_token" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "confirm_expires_at" timestamptz;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "confirmed_at" timestamptz;

CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_booking_confirm_token_uidx"
  ON "rdv_inscription_booking" ("confirm_token")
  WHERE "confirm_token" IS NOT NULL;

-- Un seul hold actif (pending non expiré ou confirmed) par événement Google.
DROP INDEX IF EXISTS "rdv_inscription_booking_etab_event_uidx";
CREATE UNIQUE INDEX "rdv_inscription_booking_etab_event_active_uidx"
  ON "rdv_inscription_booking" ("etablissement_id", "google_calendar_id", "google_event_id")
  WHERE "status" IN ('pending', 'confirmed');
