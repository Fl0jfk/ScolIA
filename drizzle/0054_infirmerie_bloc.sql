-- Bloc infirmerie : soin du passage, fiche, médicaments, accidents.
-- Ajout uniquement. Pas de DELETE. Extraits déjà en 0052.
--> statement-breakpoint

ALTER TABLE "infirmerie_passage" ADD COLUMN IF NOT EXISTS "suite" text;
--> statement-breakpoint
ALTER TABLE "infirmerie_passage" ADD COLUMN IF NOT EXISTS "soins_notes" text;
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "infirmerie_passage" ADD CONSTRAINT "infirmerie_passage_suite_chk"
    CHECK ("suite" is null or "suite" in ('repos', 'retour_cours', 'renvoi_famille', 'urgence', 'autre'));
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "infirmerie_fiche" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "antecedents" text DEFAULT '' NOT NULL,
  "personnes_a_prevenir" text DEFAULT '' NOT NULL,
  "notes" text DEFAULT '' NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "sante_medicament_prise" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "medicament" text NOT NULL,
  "dose" text,
  "pris_at" timestamp with time zone NOT NULL,
  "auteur_user_id" text,
  "auteur_nom" text,
  "document_id" uuid,
  "notes" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "sante_accident" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "date_accident" date NOT NULL,
  "circonstances" text DEFAULT '' NOT NULL,
  "soins" text DEFAULT '' NOT NULL,
  "suite" text DEFAULT '' NOT NULL,
  "lieu" text,
  "auteur_user_id" text,
  "auteur_nom" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "infirmerie_fiche" ADD CONSTRAINT "infirmerie_fiche_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "infirmerie_fiche" ADD CONSTRAINT "infirmerie_fiche_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_medicament_prise" ADD CONSTRAINT "sante_medicament_prise_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_medicament_prise" ADD CONSTRAINT "sante_medicament_prise_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_medicament_prise" ADD CONSTRAINT "sante_medicament_prise_document_id_eleve_document_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."eleve_document"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_accident" ADD CONSTRAINT "sante_accident_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_accident" ADD CONSTRAINT "sante_accident_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

CREATE UNIQUE INDEX IF NOT EXISTS "infirmerie_fiche_eleve_uidx" ON "infirmerie_fiche" ("etablissement_id", "eleve_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_medicament_prise_eleve_idx" ON "sante_medicament_prise" ("etablissement_id", "eleve_id", "pris_at");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_accident_eleve_idx" ON "sante_accident" ("etablissement_id", "eleve_id", "date_accident");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_accident_date_idx" ON "sante_accident" ("etablissement_id", "date_accident");
