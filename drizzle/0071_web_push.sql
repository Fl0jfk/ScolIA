-- Notifications navigateur (Web Push) : abonnements par appareil et préférences par utilisateur.
-- Purement additif : nouvelles tables, rien d'existant n'est modifié.
CREATE TABLE IF NOT EXISTS "web_push_subscription" (
  "id" text PRIMARY KEY,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "user_id" text NOT NULL,
  "endpoint" text NOT NULL,
  "p256dh" text NOT NULL,
  "auth" text NOT NULL,
  "user_agent" text,
  "failure_count" integer NOT NULL DEFAULT 0,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "last_success_at" timestamptz
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "web_push_subscription_endpoint_uidx" ON "web_push_subscription" ("endpoint");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "web_push_subscription_user_idx" ON "web_push_subscription" ("etablissement_id", "user_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "web_push_preference" (
  "user_id" text PRIMARY KEY,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "muted_kinds" text[] NOT NULL DEFAULT '{}'::text[],
  "quiet_start_minute" integer,
  "quiet_end_minute" integer,
  "updated_at" timestamptz NOT NULL DEFAULT now()
);
