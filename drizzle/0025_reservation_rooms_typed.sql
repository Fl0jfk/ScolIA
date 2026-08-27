CREATE TABLE IF NOT EXISTS "reservation_room" (
  "etablissement_id" uuid NOT NULL,
  "id" text NOT NULL,
  "name" text NOT NULL,
  "building" text,
  "sort_order" integer DEFAULT 0 NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "reservation_room_pk" PRIMARY KEY("etablissement_id","id")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "reservation_room_booking" (
  "etablissement_id" uuid NOT NULL,
  "id" text NOT NULL,
  "room_id" text NOT NULL,
  "group_id" text,
  "user_id" text DEFAULT '' NOT NULL,
  "first_name" text DEFAULT '' NOT NULL,
  "last_name" text DEFAULT '' NOT NULL,
  "booked_by_first_name" text DEFAULT '' NOT NULL,
  "booked_by_last_name" text DEFAULT '' NOT NULL,
  "booked_for_other" boolean DEFAULT false NOT NULL,
  "email" text,
  "subject" text,
  "class_name" text,
  "comment" text,
  "starts_at" text NOT NULL,
  "ends_at" text NOT NULL,
  "status" text DEFAULT 'CONFIRMED' NOT NULL,
  "cancelled_at" text,
  "cancelled_by" text,
  "cancel_reason" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "reservation_room_booking_pk" PRIMARY KEY("etablissement_id","id")
);
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "reservation_room" ADD CONSTRAINT "reservation_room_etablissement_id_etablissement_id_fk"
    FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "reservation_room_booking" ADD CONSTRAINT "reservation_room_booking_etablissement_id_etablissement_id_fk"
    FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reservation_room_etab_idx" ON "reservation_room" ("etablissement_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reservation_room_booking_etab_idx" ON "reservation_room_booking" ("etablissement_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reservation_room_booking_room_idx" ON "reservation_room_booking" ("etablissement_id","room_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reservation_room_booking_starts_idx" ON "reservation_room_booking" ("etablissement_id","starts_at");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reservation_room_booking_status_idx" ON "reservation_room_booking" ("etablissement_id","status");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reservation_room_booking_group_idx" ON "reservation_room_booking" ("etablissement_id","group_id");
