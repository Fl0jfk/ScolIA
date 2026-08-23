CREATE TABLE IF NOT EXISTS "teacher_planning" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "external_user_id" text NOT NULL,
  "source" text,
  "source_file_name" text,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_by" text DEFAULT '' NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "teacher_planning_etab_user_uidx" ON "teacher_planning" ("etablissement_id", "external_user_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "teacher_planning_etablissement_idx" ON "teacher_planning" ("etablissement_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "teacher_planning_slot" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "planning_id" uuid NOT NULL REFERENCES "teacher_planning"("id") ON DELETE CASCADE,
  "week_type" text NOT NULL,
  "day" integer NOT NULL,
  "start_time" text NOT NULL,
  "end_time" text NOT NULL,
  "subject" text NOT NULL,
  "classes" text[] DEFAULT '{}'::text[] NOT NULL,
  "room" text
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "teacher_planning_slot_planning_idx" ON "teacher_planning_slot" ("planning_id", "week_type");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "teacher_planning_slot_etab_idx" ON "teacher_planning_slot" ("etablissement_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "teacher_planning_slot_class_gin_idx" ON "teacher_planning_slot" USING gin ("classes");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "teacher_planning_replacement" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "planning_id" uuid NOT NULL REFERENCES "teacher_planning"("id") ON DELETE CASCADE,
  "date" date NOT NULL,
  "start_time" text NOT NULL,
  "end_time" text NOT NULL,
  "subject" text NOT NULL,
  "classes" text[] DEFAULT '{}'::text[] NOT NULL,
  "room" text,
  "note" text,
  "created_by" text DEFAULT '' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "teacher_planning_repl_planning_idx" ON "teacher_planning_replacement" ("planning_id", "date");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "teacher_planning_repl_etab_date_idx" ON "teacher_planning_replacement" ("etablissement_id", "date");
