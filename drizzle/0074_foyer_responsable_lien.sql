-- Foyers : lien de parenté d'un responsable avec chaque élève (Mère, Père, Beau-père…).
-- Purement additif : une nouvelle table, rien d'existant n'est modifié.
CREATE TABLE IF NOT EXISTS "foyer_responsable_lien" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "responsable_id" uuid NOT NULL REFERENCES "foyer_responsable"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "lien" text NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT "foyer_responsable_lien_pk" PRIMARY KEY ("responsable_id", "eleve_id")
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "foyer_responsable_lien_etab_idx" ON "foyer_responsable_lien" ("etablissement_id", "eleve_id");
