CREATE TABLE IF NOT EXISTS "travel_parent_blog" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "travel_id" text NOT NULL REFERENCES "travel"("id") ON DELETE cascade,
  "token" text NOT NULL,
  "enabled" boolean DEFAULT true NOT NULL,
  "activated_at" timestamptz NOT NULL,
  "expires_at" timestamptz NOT NULL,
  "activated_by_user_id" text,
  "parents_notified_at" timestamptz,
  "purged_at" timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS "travel_parent_blog_token_uidx" ON "travel_parent_blog" ("token");
CREATE UNIQUE INDEX IF NOT EXISTS "travel_parent_blog_travel_uidx" ON "travel_parent_blog" ("etablissement_id", "travel_id");
CREATE INDEX IF NOT EXISTS "travel_parent_blog_expires_idx" ON "travel_parent_blog" ("expires_at");

CREATE TABLE IF NOT EXISTS "travel_parent_blog_post" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "travel_id" text NOT NULL REFERENCES "travel"("id") ON DELETE cascade,
  "author_user_id" text DEFAULT '' NOT NULL,
  "author_name" text DEFAULT '' NOT NULL,
  "body" text DEFAULT '' NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "travel_parent_blog_post_travel_idx" ON "travel_parent_blog_post" ("etablissement_id", "travel_id");

CREATE TABLE IF NOT EXISTS "travel_parent_blog_photo" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "post_id" text NOT NULL REFERENCES "travel_parent_blog_post"("id") ON DELETE cascade,
  "travel_id" text NOT NULL REFERENCES "travel"("id") ON DELETE cascade,
  "s3_key" text NOT NULL,
  "content_type" text DEFAULT 'image/jpeg' NOT NULL,
  "sort_order" integer DEFAULT 0 NOT NULL
);
CREATE INDEX IF NOT EXISTS "travel_parent_blog_photo_post_idx" ON "travel_parent_blog_photo" ("etablissement_id", "post_id");
CREATE INDEX IF NOT EXISTS "travel_parent_blog_photo_travel_idx" ON "travel_parent_blog_photo" ("etablissement_id", "travel_id");
