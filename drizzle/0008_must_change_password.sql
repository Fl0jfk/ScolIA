ALTER TABLE "user" ADD COLUMN IF NOT EXISTS "must_change_password" boolean DEFAULT true NOT NULL;
-- Compte admin déjà provisionné avec MDP personnel : ne pas forcer.
UPDATE "user" SET "must_change_password" = false WHERE lower(email) = 'florian@h-me.fr';
