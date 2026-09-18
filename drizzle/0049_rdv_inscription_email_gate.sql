-- Gate e-mail RDV inscription (double opt-in avant le formulaire).
CREATE TABLE IF NOT EXISTS "rdv_inscription_email_gate" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "direction_slug" text NOT NULL,
  "parent_email" text NOT NULL,
  "token" text NOT NULL,
  "expires_at" timestamp with time zone NOT NULL,
  "verified_at" timestamp with time zone,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "rdv_inscription_email_gate_token_uidx"
  ON "rdv_inscription_email_gate" ("token");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "rdv_inscription_email_gate_etab_email_idx"
  ON "rdv_inscription_email_gate" ("etablissement_id", "parent_email");
