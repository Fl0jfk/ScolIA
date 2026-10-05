-- Communication familles : pièces, notifs in-app, matrice, broadcast
ALTER TABLE "famille_thread" ADD COLUMN IF NOT EXISTS "is_broadcast" boolean DEFAULT false NOT NULL;

CREATE TABLE IF NOT EXISTS "famille_thread_attachment" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "message_id" uuid NOT NULL REFERENCES "famille_thread_message"("id") ON DELETE cascade,
  "file_name" text NOT NULL,
  "mime" text NOT NULL,
  "size" integer NOT NULL,
  "s3_key" text,
  "content_base64" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "famille_thread_attachment_message_idx" ON "famille_thread_attachment" ("message_id");
CREATE INDEX IF NOT EXISTS "famille_thread_attachment_etab_idx" ON "famille_thread_attachment" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "famille_messaging_settings" (
  "etablissement_id" uuid PRIMARY KEY REFERENCES "etablissement"("id") ON DELETE cascade,
  "roles_can_initiate" jsonb DEFAULT '["admin","cpe","direction","directeur","directrice","administratif"]'::jsonb NOT NULL,
  "prof_own_classes_only" boolean DEFAULT true NOT NULL,
  "allow_broadcast" boolean DEFAULT true NOT NULL,
  "allow_parent_attachments" boolean DEFAULT true NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_by_user_id" text
);

CREATE TABLE IF NOT EXISTS "famille_notif" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE cascade,
  "foyer_id" uuid NOT NULL,
  "thread_id" uuid REFERENCES "famille_thread"("id") ON DELETE cascade,
  "kind" text DEFAULT 'new_message' NOT NULL,
  "titre" text NOT NULL,
  "preview" text DEFAULT '' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "lu_at" timestamp with time zone,
  CONSTRAINT "famille_notif_kind_chk" CHECK ("kind" in ('new_message', 'broadcast', 'reply_staff'))
);
CREATE INDEX IF NOT EXISTS "famille_notif_foyer_idx" ON "famille_notif" ("etablissement_id","foyer_id","lu_at");
CREATE INDEX IF NOT EXISTS "famille_notif_thread_idx" ON "famille_notif" ("thread_id");
