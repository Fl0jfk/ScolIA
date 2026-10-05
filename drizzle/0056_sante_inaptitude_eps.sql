-- Inaptitude EPS → extrait. Ajout uniquement. Pas de DELETE.
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "sante_inaptitude_eps" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "date_debut" date NOT NULL,
  "date_fin" date,
  "motif" text DEFAULT '' NOT NULL,
  "libelle_extrait" text NOT NULL,
  "document_id" uuid,
  "extrait_id" uuid,
  "actif" boolean DEFAULT true NOT NULL,
  "auteur_user_id" text,
  "auteur_nom" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "sante_inaptitude_eps" ADD CONSTRAINT "sante_inaptitude_eps_etablissement_id_etablissement_id_fk"
    FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_inaptitude_eps" ADD CONSTRAINT "sante_inaptitude_eps_eleve_id_eleve_id_fk"
    FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_inaptitude_eps" ADD CONSTRAINT "sante_inaptitude_eps_document_id_eleve_document_id_fk"
    FOREIGN KEY ("document_id") REFERENCES "public"."eleve_document"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_inaptitude_eps" ADD CONSTRAINT "sante_inaptitude_eps_extrait_id_sante_extrait_id_fk"
    FOREIGN KEY ("extrait_id") REFERENCES "public"."sante_extrait"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "sante_inaptitude_eps_eleve_idx"
  ON "sante_inaptitude_eps" ("etablissement_id", "eleve_id", "date_debut");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_inaptitude_eps_actif_idx"
  ON "sante_inaptitude_eps" ("etablissement_id", "actif");
--> statement-breakpoint

-- Nuit internat : même passage, autre contexte d’horaire.
ALTER TABLE "infirmerie_passage" ADD COLUMN IF NOT EXISTS "contexte" text DEFAULT 'journee' NOT NULL;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "infirmerie_passage" ADD CONSTRAINT "infirmerie_passage_contexte_chk"
    CHECK ("contexte" in ('journee', 'nuit_internat'));
EXCEPTION WHEN duplicate_object THEN null; END $$;
