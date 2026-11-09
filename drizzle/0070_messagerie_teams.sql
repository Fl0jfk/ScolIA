-- Messagerie du personnel « niveau Teams » : préférences par participant et métadonnées de message.
-- Purement additif : colonnes nullables ou avec valeur par défaut, aucune donnée existante modifiée.
ALTER TABLE "messaging_participant" ADD COLUMN IF NOT EXISTS "hidden_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "messaging_participant" ADD COLUMN IF NOT EXISTS "pinned_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "messaging_participant" ADD COLUMN IF NOT EXISTS "marked_unread_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "messaging_message" ADD COLUMN IF NOT EXISTS "mention_user_ids" text[] NOT NULL DEFAULT '{}'::text[];
--> statement-breakpoint
ALTER TABLE "messaging_message" ADD COLUMN IF NOT EXISTS "pinned_at" timestamptz;
--> statement-breakpoint
ALTER TABLE "messaging_message" ADD COLUMN IF NOT EXISTS "pinned_by_id" text;
