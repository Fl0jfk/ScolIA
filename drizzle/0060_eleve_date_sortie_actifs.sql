-- Date de sortie établissement + alignement statut / scolarité (idempotent, sans DELETE).
--> statement-breakpoint

ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "date_sortie" date;
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "eleve_etablissement_date_sortie_idx"
  ON "eleve" ("etablissement_id", "date_sortie")
  WHERE "date_sortie" IS NOT NULL;
--> statement-breakpoint

UPDATE "eleve"
SET "status" = 'ancien', "updated_at" = now()
WHERE "status" = 'inscrit'
  AND "date_sortie" IS NOT NULL
  AND "date_sortie" <= CURRENT_DATE;
--> statement-breakpoint

UPDATE "eleve_scolarite" AS s
SET "statut" = 'terminee', "updated_at" = now()
WHERE s."statut" = 'en_cours'
  AND EXISTS (
    SELECT 1 FROM "eleve" e
    WHERE e."id" = s."eleve_id"
      AND e."etablissement_id" = s."etablissement_id"
      AND (
        e."status" IN ('ancien', 'archive')
        OR (e."date_sortie" IS NOT NULL AND e."date_sortie" <= CURRENT_DATE)
      )
  );
