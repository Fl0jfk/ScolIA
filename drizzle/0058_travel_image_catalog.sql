-- Catalogue images voyages partagé (manuel + enrichissement Wikimedia / Unsplash).
CREATE TABLE IF NOT EXISTS "travel_image_catalog" (
  "id" text PRIMARY KEY NOT NULL,
  "label" text NOT NULL,
  "url" text NOT NULL,
  "keywords" text DEFAULT '' NOT NULL,
  "normalize_key" text NOT NULL,
  "source" text DEFAULT 'manual' NOT NULL,
  "author" text,
  "license" text,
  "attribution_url" text,
  "source_page_url" text,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "travel_image_catalog_normalize_key_uidx"
  ON "travel_image_catalog" ("normalize_key");
CREATE INDEX IF NOT EXISTS "travel_image_catalog_source_idx"
  ON "travel_image_catalog" ("source");
