-- PAI tenu par l’infirmerie. Ajout uniquement. Pas de DELETE.
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "sante_pai" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "statut" text DEFAULT 'brouillon' NOT NULL,
  "protocole" text DEFAULT '' NOT NULL,
  "traitements_autorises" text DEFAULT '' NOT NULL,
  "document_id" uuid,
  "date_debut" date,
  "date_fin" date,
  "valide_at" timestamp with time zone,
  "valide_par_user_id" text,
  "valide_par_nom" text,
  "notes" text DEFAULT '' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "sante_pai" ADD CONSTRAINT "sante_pai_statut_chk"
    CHECK ("statut" in ('brouillon', 'valide', 'expire', 'revoque'));
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "sante_pai" ADD CONSTRAINT "sante_pai_etablissement_id_etablissement_id_fk"
    FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_pai" ADD CONSTRAINT "sante_pai_eleve_id_eleve_id_fk"
    FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_pai" ADD CONSTRAINT "sante_pai_document_id_eleve_document_id_fk"
    FOREIGN KEY ("document_id") REFERENCES "public"."eleve_document"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "sante_pai_eleve_idx" ON "sante_pai" ("etablissement_id", "eleve_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_pai_statut_idx" ON "sante_pai" ("etablissement_id", "statut");
