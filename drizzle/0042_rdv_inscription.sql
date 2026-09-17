-- RDV inscriptions (Google Agenda multi-directrices)
CREATE TABLE IF NOT EXISTS "rdv_inscription_config" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "enabled" integer DEFAULT 0 NOT NULL,
  "title" text DEFAULT 'Rendez-vous d’inscription' NOT NULL,
  "intro" text DEFAULT '' NOT NULL,
  "event_title_pattern" text DEFAULT 'rendez-vous inscription' NOT NULL,
  "notify_email" text,
  "location" text DEFAULT '' NOT NULL,
  "consent_label" text DEFAULT 'J’accepte que mes coordonnées soient utilisées pour organiser ce rendez-vous d’inscription.' NOT NULL,
  "horizon_days" integer DEFAULT 60 NOT NULL,
  "google_linked" integer DEFAULT 0 NOT NULL,
  "google_linked_email" text,
  "google_linked_at" timestamptz,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_config_etab_uidx" ON "rdv_inscription_config" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "rdv_inscription_config_etab_idx" ON "rdv_inscription_config" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "rdv_inscription_direction" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "slug" text NOT NULL,
  "label" text NOT NULL,
  "google_calendar_id" text DEFAULT '' NOT NULL,
  "directrice_display_name" text,
  "active" integer DEFAULT 1 NOT NULL,
  "sort_order" integer DEFAULT 0 NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_dir_etab_slug_uidx" ON "rdv_inscription_direction" ("etablissement_id", "slug");
CREATE INDEX IF NOT EXISTS "rdv_inscription_dir_etab_idx" ON "rdv_inscription_direction" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "rdv_inscription_dir_active_idx" ON "rdv_inscription_direction" ("etablissement_id", "active");

CREATE TABLE IF NOT EXISTS "rdv_inscription_booking" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "direction_id" uuid NOT NULL REFERENCES "rdv_inscription_direction"("id") ON DELETE cascade,
  "direction_slug" text NOT NULL,
  "google_event_id" text NOT NULL,
  "google_calendar_id" text NOT NULL,
  "google_html_link" text,
  "start_at" timestamptz NOT NULL,
  "end_at" timestamptz NOT NULL,
  "student_first_name" text NOT NULL,
  "student_last_name" text NOT NULL,
  "parent_email" text NOT NULL,
  "parent_phone" text NOT NULL,
  "status" text DEFAULT 'confirmed' NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_booking_etab_event_uidx"
  ON "rdv_inscription_booking" ("etablissement_id", "google_calendar_id", "google_event_id");
CREATE INDEX IF NOT EXISTS "rdv_inscription_booking_etab_idx" ON "rdv_inscription_booking" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "rdv_inscription_booking_dir_idx" ON "rdv_inscription_booking" ("etablissement_id", "direction_id");
CREATE INDEX IF NOT EXISTS "rdv_inscription_booking_status_idx" ON "rdv_inscription_booking" ("etablissement_id", "status");
CREATE INDEX IF NOT EXISTS "rdv_inscription_booking_start_idx" ON "rdv_inscription_booking" ("etablissement_id", "start_at");
