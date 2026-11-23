-- Portes ouvertes : archive d'une session terminée (inscrits, équipes, créneaux tels qu'ils étaient).
-- Purement additif : une nouvelle table, rien d'existant n'est modifié.
CREATE TABLE IF NOT EXISTS "portes_ouvertes_archive" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "title" text NOT NULL DEFAULT '',
  "session_start_at" timestamptz,
  "session_end_at" timestamptz,
  "registrations_count" integer NOT NULL DEFAULT 0,
  "staff_count" integer NOT NULL DEFAULT 0,
  "slots" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "registrations" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "staff" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "archived_by_user_id" text,
  "archived_by_name" text,
  "archived_at" timestamptz NOT NULL DEFAULT now()
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "portes_ouvertes_archive_etab_idx" ON "portes_ouvertes_archive" ("etablissement_id", "archived_at");
