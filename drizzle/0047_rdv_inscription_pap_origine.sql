-- RDV inscription : PAP + établissement d’origine.
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "has_pap" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "pap_s3_key" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "pap_file_name" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "pap_mime_type" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "pap_bring_to_rdv" integer NOT NULL DEFAULT 0;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "etablissement_origine_rne" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "etablissement_origine_label" text;
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "etablissement_origine_adresse" text;
