-- Comptabilité dans l’ENT : partie simple (échéances, dépenses, caisse / banque).
-- Pas de plan comptable. Pas de partie double. Ajout uniquement.
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "facture_echeance" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "facture_id" uuid NOT NULL,
  "date_echeance" date NOT NULL,
  "montant" numeric(12, 2) NOT NULL,
  "ordre" integer DEFAULT 1 NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "tresorerie_compte" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "libelle" text NOT NULL,
  "nature" text NOT NULL,
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "tresorerie_compte_nature_chk" CHECK ("nature" in ('caisse', 'banque'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "depense" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "date_depense" date NOT NULL,
  "libelle" text NOT NULL,
  "fournisseur" text,
  "portee" text DEFAULT 'autre' NOT NULL,
  "montant" numeric(12, 2) NOT NULL,
  "statut" text DEFAULT 'prevue' NOT NULL,
  "travel_id" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "depense_portee_chk" CHECK ("portee" in ('cantine', 'internat', 'voyage', 'scolarite', 'autre')),
  CONSTRAINT "depense_statut_chk" CHECK ("statut" in ('prevue', 'payee'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "mouvement_tresorerie" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "compte_id" uuid NOT NULL,
  "date_mouvement" date NOT NULL,
  "sens" text NOT NULL,
  "montant" numeric(12, 2) NOT NULL,
  "libelle" text NOT NULL,
  "encaissement_id" uuid,
  "depense_id" uuid,
  "facture_id" uuid,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "mouvement_tresorerie_sens_chk" CHECK ("sens" in ('entree', 'sortie')),
  CONSTRAINT "mouvement_tresorerie_montant_chk" CHECK ("montant" > 0)
);
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "facture_echeance" ADD CONSTRAINT "facture_echeance_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "facture_echeance" ADD CONSTRAINT "facture_echeance_facture_id_facture_id_fk" FOREIGN KEY ("facture_id") REFERENCES "public"."facture"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "tresorerie_compte" ADD CONSTRAINT "tresorerie_compte_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "depense" ADD CONSTRAINT "depense_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "depense" ADD CONSTRAINT "depense_travel_id_travel_id_fk" FOREIGN KEY ("travel_id") REFERENCES "public"."travel"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "mouvement_tresorerie" ADD CONSTRAINT "mouvement_tresorerie_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "mouvement_tresorerie" ADD CONSTRAINT "mouvement_tresorerie_compte_id_tresorerie_compte_id_fk" FOREIGN KEY ("compte_id") REFERENCES "public"."tresorerie_compte"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "mouvement_tresorerie" ADD CONSTRAINT "mouvement_tresorerie_encaissement_id_encaissement_id_fk" FOREIGN KEY ("encaissement_id") REFERENCES "public"."encaissement"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "mouvement_tresorerie" ADD CONSTRAINT "mouvement_tresorerie_depense_id_depense_id_fk" FOREIGN KEY ("depense_id") REFERENCES "public"."depense"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "mouvement_tresorerie" ADD CONSTRAINT "mouvement_tresorerie_facture_id_facture_id_fk" FOREIGN KEY ("facture_id") REFERENCES "public"."facture"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "facture_echeance_facture_idx" ON "facture_echeance" ("etablissement_id", "facture_id");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "facture_echeance_ordre_uidx" ON "facture_echeance" ("facture_id", "ordre");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "tresorerie_compte_libelle_uidx" ON "tresorerie_compte" ("etablissement_id", "libelle");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "depense_date_idx" ON "depense" ("etablissement_id", "date_depense");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "depense_portee_idx" ON "depense" ("etablissement_id", "portee");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "mouvement_tresorerie_compte_idx" ON "mouvement_tresorerie" ("etablissement_id", "compte_id", "date_mouvement");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "mouvement_tresorerie_depense_idx" ON "mouvement_tresorerie" ("etablissement_id", "depense_id");
--> statement-breakpoint

COMMENT ON VIEW "export_comptable_famille" IS 'Sortie pour le cabinet. La comptabilité vit dans facture, encaissement, depense, mouvement_tresorerie.';
