-- VS Phase 3 — carnet de correspondance (établissement → famille + accusé).

CREATE TABLE IF NOT EXISTS "vs_carnet_entree" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "date_entree" date NOT NULL,
  "categorie" text DEFAULT 'correspondance' NOT NULL,
  "titre" text NOT NULL,
  "corps" text NOT NULL,
  "visible_famille" boolean DEFAULT true NOT NULL,
  "created_by_user_id" text,
  "created_by_nom" text,
  "signe_at" timestamptz,
  "signe_par_user_id" text,
  "signe_par_nom" text,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS "vs_carnet_etab_date_idx"
  ON "vs_carnet_entree" ("etablissement_id", "date_entree");
CREATE INDEX IF NOT EXISTS "vs_carnet_eleve_idx"
  ON "vs_carnet_entree" ("etablissement_id", "eleve_id");
CREATE INDEX IF NOT EXISTS "vs_carnet_signe_idx"
  ON "vs_carnet_entree" ("etablissement_id", "signe_at");
