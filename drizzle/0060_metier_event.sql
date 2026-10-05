-- Journal métier transversal (Pack A — clôture appel → attendance.call_completed).
-- Idempotent : safe si 0050_eleve_core_socle n'a jamais été joué en prod ou a échoué avant CREATE TABLE.
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "metier_event" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "type" text NOT NULL,
  "aggregate" text NOT NULL,
  "aggregate_id" uuid NOT NULL,
  "eleve_id" uuid REFERENCES "eleve"("id") ON DELETE SET NULL,
  "payload" jsonb DEFAULT '{}'::jsonb NOT NULL,
  "actor_user_id" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "metier_event_etab_created_idx"
  ON "metier_event" ("etablissement_id", "created_at");
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "metier_event_etab_type_idx"
  ON "metier_event" ("etablissement_id", "type");
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "metier_event_eleve_idx"
  ON "metier_event" ("etablissement_id", "eleve_id");
