CREATE TABLE IF NOT EXISTS "request_org" (
	"etablissement_id" uuid PRIMARY KEY NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_org_attr" (
	"etablissement_id" uuid NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "request_org_attr_pk" PRIMARY KEY("etablissement_id","path")
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_org" ADD CONSTRAINT "request_org_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_org_attr" ADD CONSTRAINT "request_org_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
