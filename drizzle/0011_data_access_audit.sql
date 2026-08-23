CREATE TABLE IF NOT EXISTS "data_access_audit" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "user_id" text REFERENCES "user"("id") ON DELETE SET NULL,
  "resource_type" text NOT NULL,
  "resource_id" text,
  "action" text NOT NULL,
  "ip_address" text,
  "user_agent" text,
  "metadata" jsonb,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "data_access_audit_etab_created_idx" ON "data_access_audit" ("etablissement_id", "created_at");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "data_access_audit_resource_idx" ON "data_access_audit" ("etablissement_id", "resource_type", "created_at");
