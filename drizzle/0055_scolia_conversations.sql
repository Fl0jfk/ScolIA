-- Conversations ScolIA (historique multi-appareils).

CREATE TABLE IF NOT EXISTS "scolia_conversation" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "user_id" text NOT NULL,
  "title" text NOT NULL DEFAULT 'Nouvelle conversation',
  "state" jsonb NOT NULL DEFAULT '{}'::jsonb,
  "last_message_at" timestamptz,
  "archived_at" timestamptz,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "scolia_conversation_user_idx"
  ON "scolia_conversation" ("etablissement_id", "user_id", "updated_at");
CREATE INDEX IF NOT EXISTS "scolia_conversation_etab_idx"
  ON "scolia_conversation" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "scolia_message" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "conversation_id" text NOT NULL REFERENCES "scolia_conversation"("id") ON DELETE CASCADE,
  "role" text NOT NULL,
  "content" text NOT NULL,
  "seq" integer NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "scolia_message_conv_idx"
  ON "scolia_message" ("conversation_id", "seq");
CREATE INDEX IF NOT EXISTS "scolia_message_etab_idx"
  ON "scolia_message" ("etablissement_id");
