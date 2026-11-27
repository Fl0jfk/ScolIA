-- Photos de profil du personnel. Purement additif : une nouvelle table.
CREATE TABLE IF NOT EXISTS "user_avatar" (
  "user_id" text PRIMARY KEY REFERENCES "user"("id") ON DELETE CASCADE,
  "image" bytea NOT NULL,
  "updated_by" text NOT NULL,
  "updated_at" timestamptz NOT NULL DEFAULT now()
);
