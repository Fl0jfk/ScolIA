-- Secret Santa : incompatibilités confidentielles, réservées à l'administrateur. Nouvelle table seulement.
CREATE TABLE IF NOT EXISTS "secret_santa_incompatibility" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "user_a" text NOT NULL REFERENCES "user"("id") ON DELETE CASCADE,
  "user_b" text NOT NULL REFERENCES "user"("id") ON DELETE CASCADE,
  "created_by" text NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT "secret_santa_incompatibility_pk" PRIMARY KEY ("etablissement_id", "user_a", "user_b")
);
