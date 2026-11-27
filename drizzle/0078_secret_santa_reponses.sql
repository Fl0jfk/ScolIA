-- Secret Santa : réponses des invités, rendez-vous, désistement après tirage. Colonnes ajoutées seulement.
ALTER TABLE "secret_santa" ADD COLUMN IF NOT EXISTS "event_place" text NOT NULL DEFAULT '';
--> statement-breakpoint
ALTER TABLE "secret_santa" ADD COLUMN IF NOT EXISTS "event_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "secret_santa" ADD COLUMN IF NOT EXISTS "updates_sent_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "secret_santa_participant" ADD COLUMN IF NOT EXISTS "response" text NOT NULL DEFAULT 'pending';
--> statement-breakpoint
ALTER TABLE "secret_santa_participant" ADD COLUMN IF NOT EXISTS "responded_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "secret_santa_participant" ADD COLUMN IF NOT EXISTS "changed_at" timestamptz;
--> statement-breakpoint
-- Un tirage déjà fait avec l'ancienne règle garde ses participants.
UPDATE "secret_santa_participant" SET "response" = 'accepted' WHERE "receiver_user_id" IS NOT NULL AND "response" = 'pending';
