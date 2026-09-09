-- Sépare les nomenclatures Siècle collège / lycée (codes nationaux peuvent se chevaucher).
ALTER TABLE "ref_nomenclature" ADD COLUMN IF NOT EXISTS "cycle" text NOT NULL DEFAULT '';
--> statement-breakpoint
DROP INDEX IF EXISTS "ref_nomenclature_etab_type_code_uidx";
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "ref_nomenclature_etab_type_code_cycle_uidx"
  ON "ref_nomenclature" ("etablissement_id", "type", "code", "cycle");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "ref_nomenclature_etab_type_cycle_idx"
  ON "ref_nomenclature" ("etablissement_id", "type", "cycle");
