-- Date de naissance portée par le compte (user) : source principale des anniversaires du personnel.
-- Nullable : les comptes existants n'ont pas de date tant qu'elle n'est pas saisie ou importée.
ALTER TABLE "user" ADD COLUMN IF NOT EXISTS "date_naissance" date;
