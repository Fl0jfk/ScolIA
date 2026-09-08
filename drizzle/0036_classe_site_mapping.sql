CREATE TABLE IF NOT EXISTS "classe_site_mapping" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"class_key" text NOT NULL,
	"class_name" text NOT NULL,
	"site_id" text NOT NULL,
	"siecle_code" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "classe_site_mapping_class_uidx" ON "classe_site_mapping" ("etablissement_id","class_key");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "classe_site_mapping_etablissement_idx" ON "classe_site_mapping" ("etablissement_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "classe_site_mapping_site_idx" ON "classe_site_mapping" ("etablissement_id","site_id");
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "classe_site_mapping" ADD CONSTRAINT "classe_site_mapping_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
