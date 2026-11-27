-- Secret Santa à plusieurs : un organisateur, ses participants, un tirage à une heure précise.
-- Purement additif : deux nouvelles tables, rien d'existant n'est modifié.
CREATE TABLE IF NOT EXISTS "secret_santa" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "organizer_user_id" text NOT NULL,
  "organizer_name" text NOT NULL,
  "title" text NOT NULL,
  "budget" text NOT NULL DEFAULT '',
  "message" text NOT NULL DEFAULT '',
  "reveal_at" timestamptz NOT NULL,
  "drawn_at" timestamptz,
  "exclusions" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "secret_santa_organizer_idx" ON "secret_santa" ("etablissement_id", "organizer_user_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "secret_santa_participant" (
  "santa_id" uuid NOT NULL REFERENCES "secret_santa"("id") ON DELETE CASCADE,
  "user_id" text NOT NULL,
  "display_name" text NOT NULL,
  "email" text NOT NULL,
  "receiver_user_id" text,
  "invited_at" timestamptz,
  "opened_at" timestamptz,
  CONSTRAINT "secret_santa_participant_pk" PRIMARY KEY ("santa_id", "user_id")
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "secret_santa_participant_user_idx" ON "secret_santa_participant" ("user_id");
