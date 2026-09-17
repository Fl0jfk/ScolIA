-- RDV inscription : lien élève, niveau demandé, reconfirmation J-7.
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "niveau_id" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "niveau_label" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "eleve_id" uuid;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "match_status" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "create_new" integer NOT NULL DEFAULT 0;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "reconfirm_token" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "reconfirm_status" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "reconfirm_mail_sent_at" timestamptz;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "reconfirmed_at" timestamptz;

CREATE INDEX IF NOT EXISTS "rdv_inscription_booking_eleve_idx"
  ON "rdv_inscription_booking" ("etablissement_id", "eleve_id");

CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_booking_reconfirm_token_uidx"
  ON "rdv_inscription_booking" ("reconfirm_token")
  WHERE "reconfirm_token" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "rdv_inscription_booking_reconfirm_pending_idx"
  ON "rdv_inscription_booking" ("etablissement_id", "status", "reconfirm_status", "start_at");
