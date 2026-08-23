CREATE TABLE "annee_scolaire" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"label" text NOT NULL,
	"starts_on" date,
	"ends_on" date,
	"is_current" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE "etablissement_site" (
	"etablissement_id" uuid NOT NULL,
	"site_id" text NOT NULL,
	"label" text NOT NULL,
	"kind" text,
	"director_name" text,
	"director_email" text,
	"director_external_user_id" text,
	"color_hex" text,
	"signature_s3_key" text,
	"grades" text,
	"role_slugs" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"active" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "etablissement_site_pk" PRIMARY KEY("etablissement_id","site_id")
);--> statement-breakpoint
CREATE TABLE "eleve" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"source_key" text NOT NULL,
	"ine" text,
	"nom" text NOT NULL,
	"prenom" text NOT NULL,
	"folder_name" text NOT NULL,
	"classe" text,
	"email" text,
	"parent_email" text,
	"parent1_email" text,
	"parent2_email" text,
	"parent_phone" text,
	"parent1_phone" text,
	"parent2_phone" text,
	"date_naissance" date,
	"mef" text,
	"secteur" text,
	"pilotage_key" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE "school_roster_meta" (
	"etablissement_id" uuid PRIMARY KEY NOT NULL,
	"teacher_catalog" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_by" text
);--> statement-breakpoint
CREATE TABLE "school_class_assignment" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"class_name" text NOT NULL,
	"class_key" text NOT NULL,
	"external_user_id" text NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE "personnel" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"external_user_id" text,
	"email" text DEFAULT '' NOT NULL,
	"email_perso" text,
	"email_pro" text,
	"first_name" text DEFAULT '' NOT NULL,
	"last_name" text DEFAULT '' NOT NULL,
	"display_name" text DEFAULT '' NOT NULL,
	"category" text NOT NULL,
	"job_title" text,
	"hire_date" text,
	"active" boolean DEFAULT true NOT NULL,
	"establishment_label" text,
	"manager_id" text,
	"payload" jsonb DEFAULT '{}'::jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
ALTER TABLE "annee_scolaire" ADD CONSTRAINT "annee_scolaire_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "etablissement_site" ADD CONSTRAINT "etablissement_site_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "eleve" ADD CONSTRAINT "eleve_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "school_roster_meta" ADD CONSTRAINT "school_roster_meta_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "school_class_assignment" ADD CONSTRAINT "school_class_assignment_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "personnel" ADD CONSTRAINT "personnel_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "annee_scolaire_etablissement_label_uidx" ON "annee_scolaire" USING btree ("etablissement_id","label");--> statement-breakpoint
CREATE INDEX "annee_scolaire_etablissement_idx" ON "annee_scolaire" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX "etablissement_site_etablissement_idx" ON "etablissement_site" USING btree ("etablissement_id");--> statement-breakpoint
CREATE UNIQUE INDEX "eleve_etablissement_source_uidx" ON "eleve" USING btree ("etablissement_id","source_key");--> statement-breakpoint
CREATE UNIQUE INDEX "eleve_etablissement_ine_uidx" ON "eleve" USING btree ("etablissement_id","ine") WHERE ine is not null and ine <> '';--> statement-breakpoint
CREATE INDEX "eleve_etablissement_classe_idx" ON "eleve" USING btree ("etablissement_id","classe");--> statement-breakpoint
CREATE INDEX "eleve_etablissement_idx" ON "eleve" USING btree ("etablissement_id");--> statement-breakpoint
CREATE UNIQUE INDEX "school_class_assignment_class_uidx" ON "school_class_assignment" USING btree ("etablissement_id","class_key");--> statement-breakpoint
CREATE INDEX "school_class_assignment_etablissement_idx" ON "school_class_assignment" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX "school_class_assignment_external_idx" ON "school_class_assignment" USING btree ("etablissement_id","external_user_id");--> statement-breakpoint
CREATE INDEX "personnel_etablissement_idx" ON "personnel" USING btree ("etablissement_id");--> statement-breakpoint
CREATE UNIQUE INDEX "personnel_etablissement_external_uidx" ON "personnel" USING btree ("etablissement_id","external_user_id") WHERE external_user_id is not null and external_user_id <> '';--> statement-breakpoint
CREATE INDEX "personnel_etablissement_email_idx" ON "personnel" USING btree ("etablissement_id","email");
