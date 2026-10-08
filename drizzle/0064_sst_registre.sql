-- Registre Santé & Sécurité au Travail (émargement annuel + fiches).

CREATE TABLE IF NOT EXISTS "sst_campagne" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_label" text NOT NULL,
  "title" text NOT NULL,
  "active" boolean NOT NULL DEFAULT true,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "sst_campagne_etab_annee_uidx"
  ON "sst_campagne" ("etablissement_id", "annee_label");
CREATE INDEX IF NOT EXISTS "sst_campagne_etab_idx"
  ON "sst_campagne" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "sst_emargement" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "campagne_id" uuid NOT NULL REFERENCES "sst_campagne"("id") ON DELETE CASCADE,
  "user_id" text NOT NULL,
  "first_name" text NOT NULL DEFAULT '',
  "last_name" text NOT NULL DEFAULT '',
  "fonction" text NOT NULL DEFAULT '',
  "signed_at" timestamptz NOT NULL DEFAULT now(),
  "signature_png_base64" text NOT NULL,
  "remarques" text NOT NULL DEFAULT '',
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "sst_emargement_campagne_user_uidx"
  ON "sst_emargement" ("campagne_id", "user_id");
CREATE INDEX IF NOT EXISTS "sst_emargement_etab_idx"
  ON "sst_emargement" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "sst_emargement_user_idx"
  ON "sst_emargement" ("etablissement_id", "user_id");

CREATE TABLE IF NOT EXISTS "sst_fiche" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "campagne_id" uuid NOT NULL REFERENCES "sst_campagne"("id") ON DELETE CASCADE,
  "numero" integer NOT NULL,
  "created_by_user_id" text NOT NULL,
  "declarant_first_name" text NOT NULL DEFAULT '',
  "declarant_last_name" text NOT NULL DEFAULT '',
  "declarant_fonction" text NOT NULL DEFAULT '',
  "observed_date" text NOT NULL,
  "observed_time" text NOT NULL DEFAULT '',
  "lieu" text NOT NULL DEFAULT '',
  "observations" text NOT NULL,
  "suggestions" text NOT NULL DEFAULT '',
  "signature_png_base64" text,
  "status" text NOT NULL DEFAULT 'ouverte',
  "workflow" jsonb NOT NULL DEFAULT '{}'::jsonb,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "sst_fiche_etab_numero_uidx"
  ON "sst_fiche" ("etablissement_id", "numero");
CREATE INDEX IF NOT EXISTS "sst_fiche_etab_idx"
  ON "sst_fiche" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "sst_fiche_campagne_idx"
  ON "sst_fiche" ("campagne_id");
CREATE INDEX IF NOT EXISTS "sst_fiche_status_idx"
  ON "sst_fiche" ("etablissement_id", "status");

CREATE TABLE IF NOT EXISTS "sst_consultation" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "campagne_id" uuid NOT NULL REFERENCES "sst_campagne"("id") ON DELETE CASCADE,
  "user_id" text NOT NULL,
  "first_name" text NOT NULL DEFAULT '',
  "last_name" text NOT NULL DEFAULT '',
  "fonction" text NOT NULL DEFAULT '',
  "consulted_at" timestamptz NOT NULL DEFAULT now(),
  "comments" text NOT NULL DEFAULT '',
  "signature_png_base64" text,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "sst_consultation_etab_idx"
  ON "sst_consultation" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "sst_consultation_campagne_idx"
  ON "sst_consultation" ("campagne_id");
