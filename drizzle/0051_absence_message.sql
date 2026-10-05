CREATE TABLE IF NOT EXISTS "absence_message" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "absence_id" text NOT NULL,
  "at" timestamp with time zone NOT NULL,
  "user_id" text NOT NULL DEFAULT '',
  "user_name" text NOT NULL DEFAULT '',
  "role_label" text NOT NULL DEFAULT '',
  "text" text NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "absence_message" ADD CONSTRAINT "absence_message_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "absence_message" ADD CONSTRAINT "absence_message_absence_id_absence_id_fk" FOREIGN KEY ("absence_id") REFERENCES "public"."absence"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "absence_message_absence_idx" ON "absence_message" USING btree ("etablissement_id","absence_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "absence_message_absence_at_idx" ON "absence_message" USING btree ("etablissement_id","absence_id","at");
