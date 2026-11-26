-- « Fil du voyage » : fil privé des familles des élèves participants (remplace les groupes WhatsApp pendant les sorties).
-- Purement additif : nouvelles tables, rien d'existant n'est modifié. Tout est purgé 7 jours après le retour (job cyclique).
CREATE TABLE IF NOT EXISTS "travel_feed_settings" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "trip_id" text NOT NULL,
  "opened_at" timestamptz NOT NULL DEFAULT now(),
  "opened_by" text,
  "allow_download" boolean NOT NULL DEFAULT false,
  "emails_sent" integer NOT NULL DEFAULT 0,
  "return_date" text,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT "travel_feed_settings_pk" PRIMARY KEY ("etablissement_id", "trip_id")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_feed_access" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "trip_id" text NOT NULL,
  "email" text NOT NULL,
  "ines" text[] NOT NULL DEFAULT '{}'::text[],
  "family_key" text NOT NULL,
  "token_hashes" text[] NOT NULL DEFAULT '{}'::text[],
  "first_sent_at" timestamptz,
  "send_failures" integer NOT NULL DEFAULT 0,
  "last_notified_at" timestamptz,
  "notify_count" integer NOT NULL DEFAULT 0,
  "pending_messages" integer NOT NULL DEFAULT 0,
  "pending_photos" integer NOT NULL DEFAULT 0,
  "created_at" timestamptz NOT NULL DEFAULT now()
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "travel_feed_access_trip_email_uidx" ON "travel_feed_access" ("etablissement_id", "trip_id", "email");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_feed_access_trip_idx" ON "travel_feed_access" ("etablissement_id", "trip_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_feed_post" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "trip_id" text NOT NULL,
  "kind" text NOT NULL,
  "body" text,
  "delay_minutes" integer,
  "author_user_id" text,
  "author_name" text,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "hidden_at" timestamptz,
  "hidden_by" text
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_feed_post_trip_idx" ON "travel_feed_post" ("etablissement_id", "trip_id", "created_at");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_feed_photo" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "trip_id" text NOT NULL,
  "post_id" uuid NOT NULL REFERENCES "travel_feed_post"("id") ON DELETE CASCADE,
  "storage_key" text NOT NULL,
  "width" integer NOT NULL,
  "height" integer NOT NULL,
  "bytes" integer NOT NULL,
  "position" integer NOT NULL DEFAULT 0,
  "created_at" timestamptz NOT NULL DEFAULT now()
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_feed_photo_post_idx" ON "travel_feed_photo" ("etablissement_id", "post_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_feed_reaction" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "post_id" uuid NOT NULL REFERENCES "travel_feed_post"("id") ON DELETE CASCADE,
  "trip_id" text NOT NULL,
  "family_key" text NOT NULL,
  "emoji" text NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT "travel_feed_reaction_pk" PRIMARY KEY ("post_id", "family_key", "emoji")
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_feed_reaction_trip_idx" ON "travel_feed_reaction" ("etablissement_id", "trip_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_feed_audit" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "trip_id" text NOT NULL,
  "actor_user_id" text,
  "actor_name" text,
  "action" text NOT NULL,
  "post_id" uuid,
  "detail" text,
  "created_at" timestamptz NOT NULL DEFAULT now()
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_feed_audit_trip_idx" ON "travel_feed_audit" ("etablissement_id", "trip_id", "created_at");
