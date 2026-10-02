-- Invitations : liste éligibles + situation + date naissance (match 2/3) + deadline RSVP.

ALTER TABLE "invitation_page"
  ADD COLUMN IF NOT EXISTS "require_eligible" integer NOT NULL DEFAULT 0;
ALTER TABLE "invitation_page"
  ADD COLUMN IF NOT EXISTS "ask_situation" text NOT NULL DEFAULT 'off';
ALTER TABLE "invitation_page"
  ADD COLUMN IF NOT EXISTS "rsvp_closes_at" timestamptz;

ALTER TABLE "invitation_rsvp"
  ADD COLUMN IF NOT EXISTS "situation_status" text;
ALTER TABLE "invitation_rsvp"
  ADD COLUMN IF NOT EXISTS "situation_detail" text NOT NULL DEFAULT '';
ALTER TABLE "invitation_rsvp"
  ADD COLUMN IF NOT EXISTS "situation_establishment" text NOT NULL DEFAULT '';
ALTER TABLE "invitation_rsvp"
  ADD COLUMN IF NOT EXISTS "birth_date" date;
ALTER TABLE "invitation_rsvp"
  ADD COLUMN IF NOT EXISTS "eligible_id" uuid;

CREATE TABLE IF NOT EXISTS "invitation_eligible" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "page_id" uuid NOT NULL REFERENCES "invitation_page"("id") ON DELETE CASCADE,
  "eleve_first_name" text NOT NULL,
  "eleve_last_name" text NOT NULL,
  "eleve_name_norm" text NOT NULL,
  "birth_date" date,
  "diploma" text,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE "invitation_eligible"
  ADD COLUMN IF NOT EXISTS "birth_date" date;

CREATE INDEX IF NOT EXISTS "invitation_eligible_etab_idx"
  ON "invitation_eligible" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "invitation_eligible_page_idx"
  ON "invitation_eligible" ("etablissement_id", "page_id");
CREATE INDEX IF NOT EXISTS "invitation_eligible_page_norm_idx"
  ON "invitation_eligible" ("page_id", "eleve_name_norm");
CREATE INDEX IF NOT EXISTS "invitation_eligible_page_birth_idx"
  ON "invitation_eligible" ("page_id", "birth_date");

-- Ancien unique (page, name_norm) trop strict si homonymes / variantes : on le retire si présent.
DROP INDEX IF EXISTS "invitation_eligible_page_norm_uidx";

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'invitation_rsvp_eligible_id_fk'
  ) THEN
    ALTER TABLE "invitation_rsvp"
      ADD CONSTRAINT "invitation_rsvp_eligible_id_fk"
      FOREIGN KEY ("eligible_id") REFERENCES "invitation_eligible"("id") ON DELETE SET NULL;
  END IF;
EXCEPTION
  WHEN duplicate_object THEN NULL;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS "invitation_rsvp_page_eligible_uidx"
  ON "invitation_rsvp" ("page_id", "eligible_id");
