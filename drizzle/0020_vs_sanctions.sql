-- VS Phase 2 — sanctions catalogue court + saisies CPE.

CREATE TABLE IF NOT EXISTS "vs_sanction_type" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "gravite" integer DEFAULT 1 NOT NULL,
  "actif" boolean DEFAULT true NOT NULL,
  "ordre" integer DEFAULT 1 NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS "vs_sanction_type_uidx"
  ON "vs_sanction_type" ("etablissement_id", "code");
CREATE INDEX IF NOT EXISTS "vs_sanction_type_etab_idx"
  ON "vs_sanction_type" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "vs_sanction" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "type_id" uuid NOT NULL REFERENCES "vs_sanction_type"("id") ON DELETE RESTRICT,
  "date_sanction" date NOT NULL,
  "motif" text,
  "statut" text DEFAULT 'active' NOT NULL,
  "created_by_user_id" text,
  "created_by_nom" text,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS "vs_sanction_etab_date_idx"
  ON "vs_sanction" ("etablissement_id", "date_sanction");
CREATE INDEX IF NOT EXISTS "vs_sanction_eleve_idx"
  ON "vs_sanction" ("etablissement_id", "eleve_id");
CREATE INDEX IF NOT EXISTS "vs_sanction_statut_idx"
  ON "vs_sanction" ("etablissement_id", "statut");
