ALTER TABLE "reservation_room" ADD COLUMN IF NOT EXISTS "kind" text DEFAULT 'facility' NOT NULL;
--> statement-breakpoint
ALTER TABLE "reservation_room" ADD COLUMN IF NOT EXISTS "bookable" boolean DEFAULT true NOT NULL;
