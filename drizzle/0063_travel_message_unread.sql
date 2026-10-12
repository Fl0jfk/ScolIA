CREATE TABLE IF NOT EXISTS "travel_message_read" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "travel_id" text NOT NULL,
  "user_id" text NOT NULL,
  "last_read_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_message_read" ADD CONSTRAINT "travel_message_read_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_message_read" ADD CONSTRAINT "travel_message_read_travel_id_travel_id_fk" FOREIGN KEY ("travel_id") REFERENCES "public"."travel"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "travel_message_read_user_trip_uidx" ON "travel_message_read" USING btree ("etablissement_id","travel_id","user_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_message_read_user_idx" ON "travel_message_read" USING btree ("etablissement_id","user_id");
