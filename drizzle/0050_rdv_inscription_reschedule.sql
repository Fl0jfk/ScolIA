-- Demande de rechoix admin : motif libre + token lien parent.
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "admin_cancel_note" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "reschedule_token" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "reschedule_token_expires_at" timestamptz;

CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_booking_reschedule_token_uidx"
  ON "rdv_inscription_booking" ("reschedule_token");
