-- Occupancy 14.2 : brancher les participants voyage sur eleve.id live.
-- Snapshot nom/classe reste ; eleve_id nullable (revue des non-match, pas d'invention).
--> statement-breakpoint

ALTER TABLE "travel_participant" ADD COLUMN IF NOT EXISTS "eleve_id" uuid;
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "travel_participant"
    ADD CONSTRAINT "travel_participant_eleve_id_eleve_id_fk"
    FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id")
    ON DELETE set null ON UPDATE no action;
EXCEPTION
  WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "travel_participant_eleve_idx"
  ON "travel_participant" ("etablissement_id", "eleve_id");
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "travel_participant_key_idx"
  ON "travel_participant" ("etablissement_id", "travel_id", "eleve_key");
--> statement-breakpoint

-- Backfill strict INE → eleve.id (pas de matching flou).
UPDATE travel_participant AS tp
SET eleve_id = e.id
FROM eleve e
WHERE e.etablissement_id = tp.etablissement_id
  AND tp.eleve_id IS NULL
  AND btrim(COALESCE(tp.eleve_key, '')) <> ''
  AND upper(btrim(COALESCE(e.ine, ''))) = upper(btrim(tp.eleve_key));
