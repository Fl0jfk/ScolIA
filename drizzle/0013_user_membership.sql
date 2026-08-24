-- Appartenance compte → établissement (login sans choix d’école).
CREATE TABLE IF NOT EXISTS "user_membership" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "user_id" text NOT NULL REFERENCES "user"("id") ON DELETE CASCADE,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "context" text DEFAULT 'staff' NOT NULL,
  "active" boolean DEFAULT true NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS "user_membership_user_etablissement_uidx"
  ON "user_membership" ("user_id", "etablissement_id");
CREATE INDEX IF NOT EXISTS "user_membership_user_idx" ON "user_membership" ("user_id");
CREATE INDEX IF NOT EXISTS "user_membership_etablissement_idx"
  ON "user_membership" ("etablissement_id");

-- Backfill : chaque user existant appartient à son etablissement_id.
INSERT INTO "user_membership" ("user_id", "etablissement_id", "context", "active")
SELECT u."id", u."etablissement_id", 'staff', true
FROM "user" u
WHERE u."etablissement_id" IS NOT NULL
ON CONFLICT ("user_id", "etablissement_id") DO NOTHING;
