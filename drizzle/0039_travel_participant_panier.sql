ALTER TABLE "travel_participant" ADD COLUMN IF NOT EXISTS "droit_image_ok" boolean DEFAULT true NOT NULL;
ALTER TABLE "travel_participant" ADD COLUMN IF NOT EXISTS "panier_repas" boolean DEFAULT false NOT NULL;
