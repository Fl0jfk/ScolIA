ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "regime" text;
ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "sexe" text;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eleve_etablissement_regime_idx" ON "eleve" ("etablissement_id", "regime");
