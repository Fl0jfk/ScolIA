-- Fix MFA abandonnée : verified DEFAULT true empêchait Better-Auth de poser two_factor_enabled.
ALTER TABLE "two_factor" ALTER COLUMN "verified" SET DEFAULT false;
--> statement-breakpoint
-- Setups incomplets (secret généré, MFA jamais finalisée) : repartir à zéro.
DELETE FROM "two_factor" AS tf
USING "user" AS u
WHERE tf."user_id" = u."id"
  AND u."two_factor_enabled" = false;
--> statement-breakpoint
-- Comptes déjà en MFA : s’assurer que verified est true pour le challenge login.
UPDATE "two_factor" AS tf
SET "verified" = true
FROM "user" AS u
WHERE tf."user_id" = u."id"
  AND u."two_factor_enabled" = true
  AND tf."verified" IS DISTINCT FROM true;
