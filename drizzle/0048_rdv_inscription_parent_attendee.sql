-- Parent présent au RDV + identité (nom différent de l'élève possible).
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "parent_first_name" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "parent_last_name" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "rdv_attendee" text;
