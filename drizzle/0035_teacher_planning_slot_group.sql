ALTER TABLE "teacher_planning_slot" ADD COLUMN IF NOT EXISTS "group_id" text;
--> statement-breakpoint
ALTER TABLE "teacher_planning_slot" ADD COLUMN IF NOT EXISTS "group_label" text;
