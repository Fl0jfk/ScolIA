-- Messagerie familles ↔ établissement (canal dédié, cloisonné du Messenger staff)
CREATE TABLE IF NOT EXISTS "famille_thread" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "foyer_id" uuid NOT NULL,
  "eleve_id" uuid,
  "sujet" text NOT NULL,
  "created_by_user_id" text,
  "created_by_nom" text,
  "last_message_at" timestamp with time zone DEFAULT now() NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS "famille_thread_etab_foyer_idx" ON "famille_thread" ("etablissement_id","foyer_id");
CREATE INDEX IF NOT EXISTS "famille_thread_etab_last_idx" ON "famille_thread" ("etablissement_id","last_message_at");

CREATE TABLE IF NOT EXISTS "famille_thread_message" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "thread_id" uuid NOT NULL REFERENCES "famille_thread"("id") ON DELETE cascade,
  "auteur_cote" text NOT NULL,
  "auteur_user_id" text,
  "auteur_nom" text,
  "corps" text NOT NULL,
  "lu_at" timestamp with time zone,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "famille_thread_message_cote_chk" CHECK ("auteur_cote" in ('staff', 'parent'))
);

CREATE INDEX IF NOT EXISTS "famille_thread_message_thread_idx" ON "famille_thread_message" ("etablissement_id","thread_id","created_at");
