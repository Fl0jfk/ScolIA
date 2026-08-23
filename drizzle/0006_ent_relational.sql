-- Convert jsonb → text[] when still jsonb (idempotent, sans subquery dans USING).
DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'etablissement_site'
      AND column_name = 'role_slugs'
      AND udt_name = 'jsonb'
  ) THEN
    ALTER TABLE "etablissement_site" ADD COLUMN "role_slugs_txt" text[] DEFAULT '{}'::text[] NOT NULL;
    UPDATE "etablissement_site" SET "role_slugs_txt" = COALESCE(
      (SELECT array_agg(x) FROM jsonb_array_elements_text("role_slugs") AS t(x)),
      '{}'::text[]
    );
    ALTER TABLE "etablissement_site" DROP COLUMN "role_slugs";
    ALTER TABLE "etablissement_site" RENAME COLUMN "role_slugs_txt" TO "role_slugs";
    ALTER TABLE "etablissement_site" ALTER COLUMN "role_slugs" SET DEFAULT '{}'::text[];
  END IF;
END $$;--> statement-breakpoint
DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'school_roster_meta'
      AND column_name = 'teacher_catalog'
      AND udt_name = 'jsonb'
  ) THEN
    ALTER TABLE "school_roster_meta" ADD COLUMN "teacher_catalog_txt" text[] DEFAULT '{}'::text[] NOT NULL;
    UPDATE "school_roster_meta" SET "teacher_catalog_txt" = COALESCE(
      (SELECT array_agg(x) FROM jsonb_array_elements_text("teacher_catalog") AS t(x)),
      '{}'::text[]
    );
    ALTER TABLE "school_roster_meta" DROP COLUMN "teacher_catalog";
    ALTER TABLE "school_roster_meta" RENAME COLUMN "teacher_catalog_txt" TO "teacher_catalog";
    ALTER TABLE "school_roster_meta" ALTER COLUMN "teacher_catalog" SET DEFAULT '{}'::text[];
  END IF;
END $$;--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "enseignant" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"nom" text NOT NULL,
	"prenom" text NOT NULL,
	"folder_name" text NOT NULL,
	"secteur" text NOT NULL,
	"email" text,
	"email_pro" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "absence" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"created_at" timestamp with time zone NOT NULL,
	"updated_at" timestamp with time zone NOT NULL,
	"source" text NOT NULL,
	"display_name" text NOT NULL,
	"calendar_visible" boolean DEFAULT true NOT NULL,
	"created_by_user_id" text DEFAULT '' NOT NULL,
	"created_by_name" text DEFAULT '' NOT NULL,
	"created_by_email" text DEFAULT '' NOT NULL,
	"created_by_roles" text[] DEFAULT '{}'::text[] NOT NULL,
	"scope" text NOT NULL,
	"site_label" text,
	"period_type" text,
	"start_date" date NOT NULL,
	"end_date" date NOT NULL,
	"start_time" text,
	"end_time" text,
	"start_at" timestamp with time zone NOT NULL,
	"end_at" timestamp with time zone NOT NULL,
	"reason" text DEFAULT '' NOT NULL,
	"details" text DEFAULT '' NOT NULL,
	"source_document" text,
	"document_keys" text[] DEFAULT '{}'::text[] NOT NULL,
	"confidence" double precision,
	"workflow_status" text NOT NULL,
	"manager_decision" text NOT NULL,
	"closed_at" timestamp with time zone,
	"justification_file_name" text,
	"justification_file_url" text,
	"justification_uploaded_at" timestamp with time zone,
	"justification_uploaded_by" text,
	"manager_note" text,
	"hours_treatment" text,
	"justificatif_relance_at" timestamp with time zone,
	"privacy_reason_redacted" boolean DEFAULT false NOT NULL,
	"privacy_documents_purged_at" timestamp with time zone
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "absence_history" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"absence_id" text NOT NULL,
	"at" timestamp with time zone NOT NULL,
	"by" text DEFAULT '' NOT NULL,
	"action" text NOT NULL,
	"note" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"type" text NOT NULL,
	"status" text NOT NULL,
	"owner_name" text,
	"owner_email" text,
	"owner_id" text,
	"created_at" timestamp with time zone,
	"updated_at" timestamp with time zone,
	"image_url" text,
	"image_config_id" text,
	"title" text,
	"destination" text,
	"site_label" text,
	"classes" text,
	"start_date" text,
	"end_date" text,
	"start_time" text,
	"end_time" text,
	"nb_eleves" text,
	"nb_accompagnateurs" text,
	"liste_eleves_status" text
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_attr" (
	"etablissement_id" uuid NOT NULL,
	"travel_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "travel_attr_pk" PRIMARY KEY("etablissement_id","travel_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_participant" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"travel_id" text NOT NULL,
	"eleve_key" text DEFAULT '' NOT NULL,
	"nom" text DEFAULT '' NOT NULL,
	"prenom" text DEFAULT '' NOT NULL,
	"classe" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_history" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"travel_id" text NOT NULL,
	"at" text DEFAULT '' NOT NULL,
	"by" text DEFAULT '' NOT NULL,
	"action" text DEFAULT '' NOT NULL,
	"note" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "travel_message" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"travel_id" text NOT NULL,
	"user_label" text DEFAULT '' NOT NULL,
	"role" text DEFAULT '' NOT NULL,
	"body" text DEFAULT '' NOT NULL,
	"at" text DEFAULT '' NOT NULL,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"created_at" timestamp with time zone NOT NULL,
	"updated_at" timestamp with time zone NOT NULL,
	"status" text NOT NULL,
	"category" text DEFAULT '' NOT NULL,
	"subject" text DEFAULT '' NOT NULL,
	"description" text DEFAULT '' NOT NULL,
	"requester_first_name" text DEFAULT '' NOT NULL,
	"requester_last_name" text DEFAULT '' NOT NULL,
	"requester_full_name" text DEFAULT '' NOT NULL,
	"requester_email" text DEFAULT '' NOT NULL,
	"requester_phone" text DEFAULT '' NOT NULL,
	"requester_user_id" text,
	"assigned_unit" text DEFAULT '' NOT NULL,
	"assigned_role_label" text DEFAULT '' NOT NULL,
	"assigned_email" text DEFAULT '' NOT NULL,
	"assigned_route_id" text,
	"purge_at" timestamp with time zone
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_attr" (
	"etablissement_id" uuid NOT NULL,
	"request_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "request_attr_pk" PRIMARY KEY("etablissement_id","request_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_comment" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"request_id" text NOT NULL,
	"at" text DEFAULT '' NOT NULL,
	"by_name" text DEFAULT '' NOT NULL,
	"by_email" text DEFAULT '' NOT NULL,
	"by_user_id" text,
	"body" text DEFAULT '' NOT NULL,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_history" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"request_id" text NOT NULL,
	"at" text DEFAULT '' NOT NULL,
	"by" text DEFAULT '' NOT NULL,
	"action" text DEFAULT '' NOT NULL,
	"note" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_attachment" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"request_id" text NOT NULL,
	"comment_id" text,
	"file_name" text DEFAULT '' NOT NULL,
	"s3_key" text DEFAULT '' NOT NULL,
	"content_type" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_routing" (
	"etablissement_id" uuid PRIMARY KEY NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "request_routing_attr" (
	"etablissement_id" uuid NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "request_routing_attr_pk" PRIMARY KEY("etablissement_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "domain_planning_domain" (
	"id" text NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"color" text,
	"coordinator_external_user_ids" text[] DEFAULT '{}'::text[] NOT NULL,
	CONSTRAINT "domain_planning_domain_pk" PRIMARY KEY("etablissement_id","id")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "domain_planning_booking" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"domain_id" text NOT NULL,
	"title" text,
	"starts_at" text,
	"ends_at" text,
	"assignee_external_user_id" text,
	"notes" text
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "domain_planning_session" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"title" text DEFAULT '' NOT NULL,
	"starts_at" text,
	"ends_at" text,
	"constraint_kind" text,
	"capacity" integer
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "domain_planning_signup" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"session_id" text NOT NULL,
	"external_user_id" text,
	"display_name" text,
	"email" text,
	"created_at" text
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_leave" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"personnel_id" text NOT NULL,
	"personnel_name" text DEFAULT '' NOT NULL,
	"type" text NOT NULL,
	"start_date" text NOT NULL,
	"end_date" text NOT NULL,
	"reason" text,
	"status" text NOT NULL,
	"created_at" text NOT NULL,
	"decided_at" text,
	"decided_by" text,
	"decision_note" text
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_shared_doc" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"name" text NOT NULL,
	"file_url" text DEFAULT '' NOT NULL,
	"uploaded_at" text DEFAULT '' NOT NULL,
	"uploaded_by" text DEFAULT '' NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_document" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"personnel_id" text NOT NULL,
	"kind" text DEFAULT '' NOT NULL,
	"name" text DEFAULT '' NOT NULL,
	"file_url" text DEFAULT '' NOT NULL,
	"uploaded_at" text,
	"expires_at" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_formation" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"personnel_id" text NOT NULL,
	"title" text DEFAULT '' NOT NULL,
	"organism" text,
	"completed_at" text,
	"expires_at" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_habilitation" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"personnel_id" text NOT NULL,
	"title" text DEFAULT '' NOT NULL,
	"issued_at" text,
	"expires_at" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_entretien" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"personnel_id" text NOT NULL,
	"kind" text DEFAULT '' NOT NULL,
	"status" text DEFAULT '' NOT NULL,
	"completed_at" text,
	"next_due_at" text,
	"notes" text,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "personnel_attr" (
	"etablissement_id" uuid NOT NULL,
	"personnel_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "personnel_attr_pk" PRIMARY KEY("etablissement_id","personnel_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_offer" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"title" text DEFAULT '' NOT NULL,
	"status" text,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_offer_attr" (
	"etablissement_id" uuid NOT NULL,
	"offer_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "stage_offer_attr_pk" PRIMARY KEY("etablissement_id","offer_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_convention" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"status" text,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_convention_attr" (
	"etablissement_id" uuid NOT NULL,
	"convention_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "stage_convention_attr_pk" PRIMARY KEY("etablissement_id","convention_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_application" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"offer_id" text,
	"status" text,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_application_attr" (
	"etablissement_id" uuid NOT NULL,
	"application_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "stage_application_attr_pk" PRIMARY KEY("etablissement_id","application_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_token" (
	"token" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"kind" text DEFAULT '' NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stage_token_attr" (
	"etablissement_id" uuid NOT NULL,
	"token" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "stage_token_attr_pk" PRIMARY KEY("etablissement_id","token","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "certificate_program" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"title" text DEFAULT '' NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "certificate_program_attr" (
	"etablissement_id" uuid NOT NULL,
	"program_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "certificate_program_attr_pk" PRIMARY KEY("etablissement_id","program_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "certificate_award" (
	"id" text PRIMARY KEY NOT NULL,
	"etablissement_id" uuid NOT NULL,
	"program_id" text,
	"status" text,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "certificate_award_attr" (
	"etablissement_id" uuid NOT NULL,
	"award_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "certificate_award_attr_pk" PRIMARY KEY("etablissement_id","award_id","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "tenant_setting_section" (
	"etablissement_id" uuid NOT NULL,
	"section" text NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "tenant_setting_section_pk" PRIMARY KEY("etablissement_id","section")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "tenant_setting_attr" (
	"etablissement_id" uuid NOT NULL,
	"section" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "tenant_setting_attr_pk" PRIMARY KEY("etablissement_id","section","path")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "ent_collection_record" (
	"etablissement_id" uuid NOT NULL,
	"collection" text NOT NULL,
	"record_id" text NOT NULL,
	"status" text,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "ent_collection_record_pk" PRIMARY KEY("etablissement_id","collection","record_id")
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "ent_collection_attr" (
	"etablissement_id" uuid NOT NULL,
	"collection" text NOT NULL,
	"record_id" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "ent_collection_attr_pk" PRIMARY KEY("etablissement_id","collection","record_id","path")
);--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "enseignant" ADD CONSTRAINT "enseignant_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "absence" ADD CONSTRAINT "absence_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "absence_history" ADD CONSTRAINT "absence_history_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "absence_history" ADD CONSTRAINT "absence_history_absence_id_absence_id_fk" FOREIGN KEY ("absence_id") REFERENCES "public"."absence"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel" ADD CONSTRAINT "travel_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_attr" ADD CONSTRAINT "travel_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_attr" ADD CONSTRAINT "travel_attr_travel_id_travel_id_fk" FOREIGN KEY ("travel_id") REFERENCES "public"."travel"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_participant" ADD CONSTRAINT "travel_participant_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_participant" ADD CONSTRAINT "travel_participant_travel_id_travel_id_fk" FOREIGN KEY ("travel_id") REFERENCES "public"."travel"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_history" ADD CONSTRAINT "travel_history_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_history" ADD CONSTRAINT "travel_history_travel_id_travel_id_fk" FOREIGN KEY ("travel_id") REFERENCES "public"."travel"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_message" ADD CONSTRAINT "travel_message_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "travel_message" ADD CONSTRAINT "travel_message_travel_id_travel_id_fk" FOREIGN KEY ("travel_id") REFERENCES "public"."travel"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request" ADD CONSTRAINT "request_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_attr" ADD CONSTRAINT "request_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_attr" ADD CONSTRAINT "request_attr_request_id_request_id_fk" FOREIGN KEY ("request_id") REFERENCES "public"."request"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_comment" ADD CONSTRAINT "request_comment_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_comment" ADD CONSTRAINT "request_comment_request_id_request_id_fk" FOREIGN KEY ("request_id") REFERENCES "public"."request"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_history" ADD CONSTRAINT "request_history_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_history" ADD CONSTRAINT "request_history_request_id_request_id_fk" FOREIGN KEY ("request_id") REFERENCES "public"."request"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_attachment" ADD CONSTRAINT "request_attachment_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_attachment" ADD CONSTRAINT "request_attachment_request_id_request_id_fk" FOREIGN KEY ("request_id") REFERENCES "public"."request"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_routing" ADD CONSTRAINT "request_routing_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "request_routing_attr" ADD CONSTRAINT "request_routing_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "domain_planning_domain" ADD CONSTRAINT "domain_planning_domain_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "domain_planning_booking" ADD CONSTRAINT "domain_planning_booking_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "domain_planning_session" ADD CONSTRAINT "domain_planning_session_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "domain_planning_signup" ADD CONSTRAINT "domain_planning_signup_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_leave" ADD CONSTRAINT "personnel_leave_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_shared_doc" ADD CONSTRAINT "personnel_shared_doc_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_document" ADD CONSTRAINT "personnel_document_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_formation" ADD CONSTRAINT "personnel_formation_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_habilitation" ADD CONSTRAINT "personnel_habilitation_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_entretien" ADD CONSTRAINT "personnel_entretien_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "personnel_attr" ADD CONSTRAINT "personnel_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_offer" ADD CONSTRAINT "stage_offer_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_offer_attr" ADD CONSTRAINT "stage_offer_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_offer_attr" ADD CONSTRAINT "stage_offer_attr_offer_id_stage_offer_id_fk" FOREIGN KEY ("offer_id") REFERENCES "public"."stage_offer"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_convention" ADD CONSTRAINT "stage_convention_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_convention_attr" ADD CONSTRAINT "stage_convention_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_convention_attr" ADD CONSTRAINT "stage_convention_attr_convention_id_stage_convention_id_fk" FOREIGN KEY ("convention_id") REFERENCES "public"."stage_convention"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_application" ADD CONSTRAINT "stage_application_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_application_attr" ADD CONSTRAINT "stage_application_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_application_attr" ADD CONSTRAINT "stage_application_attr_application_id_stage_application_id_fk" FOREIGN KEY ("application_id") REFERENCES "public"."stage_application"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_token" ADD CONSTRAINT "stage_token_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_token_attr" ADD CONSTRAINT "stage_token_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "stage_token_attr" ADD CONSTRAINT "stage_token_attr_token_stage_token_token_fk" FOREIGN KEY ("token") REFERENCES "public"."stage_token"("token") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "certificate_program" ADD CONSTRAINT "certificate_program_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "certificate_program_attr" ADD CONSTRAINT "certificate_program_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "certificate_program_attr" ADD CONSTRAINT "certificate_program_attr_program_id_certificate_program_id_fk" FOREIGN KEY ("program_id") REFERENCES "public"."certificate_program"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "certificate_award" ADD CONSTRAINT "certificate_award_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "certificate_award_attr" ADD CONSTRAINT "certificate_award_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "certificate_award_attr" ADD CONSTRAINT "certificate_award_attr_award_id_certificate_award_id_fk" FOREIGN KEY ("award_id") REFERENCES "public"."certificate_award"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "tenant_setting_section" ADD CONSTRAINT "tenant_setting_section_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "tenant_setting_attr" ADD CONSTRAINT "tenant_setting_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "ent_collection_record" ADD CONSTRAINT "ent_collection_record_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "ent_collection_attr" ADD CONSTRAINT "ent_collection_attr_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "enseignant_etablissement_idx" ON "enseignant" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "enseignant_etablissement_secteur_idx" ON "enseignant" USING btree ("etablissement_id","secteur");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "absence_etablissement_idx" ON "absence" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "absence_etablissement_status_idx" ON "absence" USING btree ("etablissement_id","workflow_status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "absence_etablissement_start_idx" ON "absence" USING btree ("etablissement_id","start_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "absence_history_absence_idx" ON "absence_history" USING btree ("etablissement_id","absence_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_etablissement_idx" ON "travel" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_etablissement_status_idx" ON "travel" USING btree ("etablissement_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_attr_travel_idx" ON "travel_attr" USING btree ("etablissement_id","travel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_participant_travel_idx" ON "travel_participant" USING btree ("etablissement_id","travel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_history_travel_idx" ON "travel_history" USING btree ("etablissement_id","travel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "travel_message_travel_idx" ON "travel_message" USING btree ("etablissement_id","travel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "request_etablissement_idx" ON "request" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "request_etablissement_status_idx" ON "request" USING btree ("etablissement_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "request_comment_request_idx" ON "request_comment" USING btree ("etablissement_id","request_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "request_history_request_idx" ON "request_history" USING btree ("etablissement_id","request_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "request_attachment_request_idx" ON "request_attachment" USING btree ("etablissement_id","request_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "domain_planning_booking_etab_idx" ON "domain_planning_booking" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "domain_planning_session_etab_idx" ON "domain_planning_session" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "domain_planning_signup_etab_idx" ON "domain_planning_signup" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "personnel_leave_etab_idx" ON "personnel_leave" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "personnel_shared_doc_etab_idx" ON "personnel_shared_doc" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "personnel_document_personnel_idx" ON "personnel_document" USING btree ("etablissement_id","personnel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "personnel_formation_personnel_idx" ON "personnel_formation" USING btree ("etablissement_id","personnel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "personnel_habilitation_personnel_idx" ON "personnel_habilitation" USING btree ("etablissement_id","personnel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "personnel_entretien_personnel_idx" ON "personnel_entretien" USING btree ("etablissement_id","personnel_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "stage_offer_etab_idx" ON "stage_offer" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "stage_convention_etab_idx" ON "stage_convention" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "stage_application_etab_idx" ON "stage_application" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "stage_token_etab_idx" ON "stage_token" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "certificate_program_etab_idx" ON "certificate_program" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "certificate_award_etab_idx" ON "certificate_award" USING btree ("etablissement_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "ent_collection_record_coll_idx" ON "ent_collection_record" USING btree ("etablissement_id","collection");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "ent_collection_record_status_idx" ON "ent_collection_record" USING btree ("etablissement_id","collection","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "ent_collection_attr_rec_idx" ON "ent_collection_attr" USING btree ("etablissement_id","collection","record_id");
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "platform_document" (
	"doc_key" text PRIMARY KEY NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "platform_document_attr" (
	"doc_key" text NOT NULL,
	"path" text NOT NULL,
	"value" text NOT NULL,
	CONSTRAINT "platform_document_attr_pk" PRIMARY KEY("doc_key","path")
);--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "platform_document_attr" ADD CONSTRAINT "platform_document_attr_doc_key_platform_document_doc_key_fk" FOREIGN KEY ("doc_key") REFERENCES "public"."platform_document"("doc_key") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
