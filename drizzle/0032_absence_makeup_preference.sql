ALTER TABLE "absence" ADD COLUMN IF NOT EXISTS "staff_preferred_treatment" text;
ALTER TABLE "absence" ADD COLUMN IF NOT EXISTS "staff_preferred_makeup_slots" text;
ALTER TABLE "absence" ADD COLUMN IF NOT EXISTS "direction_confirmed_makeup_slots" text;
