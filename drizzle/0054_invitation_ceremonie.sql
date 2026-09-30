-- Invitations cérémonies (RSVP pages publiques multi-instances).

CREATE TABLE IF NOT EXISTS "invitation_page" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "slug" text NOT NULL,
  "title" text NOT NULL DEFAULT 'Invitation',
  "intro" text NOT NULL DEFAULT '',
  "theme" text NOT NULL DEFAULT 'remise_diplome',
  "enabled" integer NOT NULL DEFAULT 0,
  "starts_at" timestamptz,
  "ends_at" timestamptz,
  "location" text NOT NULL DEFAULT '',
  "diploma_mode" text NOT NULL DEFAULT 'both',
  "max_total_persons" integer NOT NULL DEFAULT 200,
  "max_persons_per_eleve" integer NOT NULL DEFAULT 4,
  "notify_email" text,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "invitation_page_etab_slug_uidx"
  ON "invitation_page" ("etablissement_id", "slug");
CREATE INDEX IF NOT EXISTS "invitation_page_etab_idx"
  ON "invitation_page" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "invitation_page_etab_enabled_idx"
  ON "invitation_page" ("etablissement_id", "enabled");

CREATE TABLE IF NOT EXISTS "invitation_rsvp" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "page_id" uuid NOT NULL REFERENCES "invitation_page"("id") ON DELETE CASCADE,
  "eleve_first_name" text NOT NULL,
  "eleve_last_name" text NOT NULL,
  "eleve_name_norm" text NOT NULL,
  "response" text NOT NULL,
  "present_count" integer NOT NULL DEFAULT 0,
  "parent_email" text NOT NULL,
  "diploma" text,
  "duplicate_group_id" uuid,
  "duplicate_dismissed_at" timestamptz,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "invitation_rsvp_etab_idx"
  ON "invitation_rsvp" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "invitation_rsvp_page_idx"
  ON "invitation_rsvp" ("etablissement_id", "page_id");
CREATE INDEX IF NOT EXISTS "invitation_rsvp_page_norm_idx"
  ON "invitation_rsvp" ("page_id", "eleve_name_norm");
CREATE INDEX IF NOT EXISTS "invitation_rsvp_page_response_idx"
  ON "invitation_rsvp" ("page_id", "response");
CREATE INDEX IF NOT EXISTS "invitation_rsvp_dup_group_idx"
  ON "invitation_rsvp" ("page_id", "duplicate_group_id");
