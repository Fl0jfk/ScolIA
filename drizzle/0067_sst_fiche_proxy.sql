-- SST : traçabilité déposant / déclarant + personnes autorisées à déposer pour autrui.

ALTER TABLE "sst_fiche"
  ADD COLUMN IF NOT EXISTS "created_by_first_name" text NOT NULL DEFAULT '';
ALTER TABLE "sst_fiche"
  ADD COLUMN IF NOT EXISTS "created_by_last_name" text NOT NULL DEFAULT '';
ALTER TABLE "sst_fiche"
  ADD COLUMN IF NOT EXISTS "declarant_user_id" text NOT NULL DEFAULT '';

-- Historique : jusqu'ici déclarant = déposant.
UPDATE "sst_fiche"
SET
  "created_by_first_name" = COALESCE(NULLIF(TRIM("created_by_first_name"), ''), "declarant_first_name"),
  "created_by_last_name" = COALESCE(NULLIF(TRIM("created_by_last_name"), ''), "declarant_last_name"),
  "declarant_user_id" = COALESCE(NULLIF(TRIM("declarant_user_id"), ''), "created_by_user_id")
WHERE
  TRIM(COALESCE("created_by_first_name", '')) = ''
  OR TRIM(COALESCE("created_by_last_name", '')) = ''
  OR TRIM(COALESCE("declarant_user_id", '')) = '';

CREATE TABLE IF NOT EXISTS "sst_proxy_deposant" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "user_id" text NOT NULL,
  "first_name" text NOT NULL DEFAULT '',
  "last_name" text NOT NULL DEFAULT '',
  "email" text NOT NULL DEFAULT '',
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "sst_proxy_deposant_etab_user_uidx"
  ON "sst_proxy_deposant" ("etablissement_id", "user_id");
CREATE INDEX IF NOT EXISTS "sst_proxy_deposant_etab_idx"
  ON "sst_proxy_deposant" ("etablissement_id");
