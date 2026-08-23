ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "lieu_naissance" text;
ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "status" text DEFAULT 'inscrit' NOT NULL;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eleve_etablissement_status_idx" ON "eleve" ("etablissement_id", "status");

CREATE TABLE IF NOT EXISTS "eleve_scolarite" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid REFERENCES "annee_scolaire"("id") ON DELETE SET NULL,
  "site_id" text,
  "classe" text,
  "statut" text DEFAULT 'en_cours' NOT NULL,
  "demi_pension" boolean DEFAULT false NOT NULL,
  "repas_par_semaine" integer,
  "etablissement_precedent" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eleve_scolarite_eleve_idx" ON "eleve_scolarite" ("etablissement_id", "eleve_id");
CREATE INDEX IF NOT EXISTS "eleve_scolarite_site_idx" ON "eleve_scolarite" ("etablissement_id", "site_id");
CREATE INDEX IF NOT EXISTS "eleve_scolarite_annee_idx" ON "eleve_scolarite" ("etablissement_id", "annee_scolaire_id");

CREATE TABLE IF NOT EXISTS "foyer" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "label" text DEFAULT 'Foyer' NOT NULL,
  "adresse" text,
  "code_postal" text,
  "ville" text,
  "payeur_est_foyer" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "foyer_etablissement_idx" ON "foyer" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "foyer_responsable" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "foyer_id" uuid NOT NULL REFERENCES "foyer"("id") ON DELETE CASCADE,
  "nom" text NOT NULL,
  "prenom" text NOT NULL,
  "email" text,
  "telephone" text,
  "autorite_parentale" boolean DEFAULT false NOT NULL,
  "contact_urgence" boolean DEFAULT false NOT NULL,
  "payeur" boolean DEFAULT false NOT NULL,
  "rang" integer DEFAULT 1 NOT NULL,
  "user_id" text REFERENCES "user"("id") ON DELETE SET NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "foyer_responsable_foyer_idx" ON "foyer_responsable" ("etablissement_id", "foyer_id");
CREATE UNIQUE INDEX IF NOT EXISTS "foyer_responsable_foyer_rang_uidx" ON "foyer_responsable" ("foyer_id", "rang");

CREATE TABLE IF NOT EXISTS "eleve_foyer_link" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "foyer_id" uuid NOT NULL REFERENCES "foyer"("id") ON DELETE CASCADE,
  "relation" text DEFAULT 'principal' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "eleve_foyer_link_pk" PRIMARY KEY ("etablissement_id", "eleve_id", "foyer_id")
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eleve_foyer_link_eleve_idx" ON "eleve_foyer_link" ("etablissement_id", "eleve_id");

CREATE TABLE IF NOT EXISTS "eleve_document" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "tiroir" text NOT NULL,
  "title" text NOT NULL,
  "mime_type" text,
  "s3_key" text,
  "file_url" text,
  "annee_label" text,
  "confidentialite" text DEFAULT 'standard' NOT NULL,
  "source" text DEFAULT 'upload' NOT NULL,
  "created_by_user_id" text REFERENCES "user"("id") ON DELETE SET NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eleve_document_eleve_idx" ON "eleve_document" ("etablissement_id", "eleve_id");
CREATE INDEX IF NOT EXISTS "eleve_document_tiroir_idx" ON "eleve_document" ("etablissement_id", "eleve_id", "tiroir");

CREATE TABLE IF NOT EXISTS "document_access_request" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "document_id" uuid NOT NULL REFERENCES "eleve_document"("id") ON DELETE CASCADE,
  "requester_user_id" text NOT NULL REFERENCES "user"("id") ON DELETE CASCADE,
  "status" text DEFAULT 'pending' NOT NULL,
  "duration_days" integer DEFAULT 1 NOT NULL,
  "decided_by_user_id" text REFERENCES "user"("id") ON DELETE SET NULL,
  "decided_at" timestamp with time zone,
  "expires_at" timestamp with time zone,
  "note" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "document_access_request_doc_idx" ON "document_access_request" ("etablissement_id", "document_id");
CREATE INDEX IF NOT EXISTS "document_access_request_status_idx" ON "document_access_request" ("etablissement_id", "status");

CREATE TABLE IF NOT EXISTS "eleve_access_audit" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "actor_user_id" text REFERENCES "user"("id") ON DELETE SET NULL,
  "resource_type" text NOT NULL,
  "resource_id" text NOT NULL,
  "eleve_id" uuid REFERENCES "eleve"("id") ON DELETE SET NULL,
  "action" text NOT NULL,
  "metadata" jsonb,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "eleve_access_audit_etab_idx" ON "eleve_access_audit" ("etablissement_id", "created_at");
CREATE INDEX IF NOT EXISTS "eleve_access_audit_eleve_idx" ON "eleve_access_audit" ("etablissement_id", "eleve_id");

CREATE TABLE IF NOT EXISTS "preinscription" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "site_id" text,
  "niveau_vise" text,
  "filiere_visee" text,
  "nom" text NOT NULL,
  "prenom" text NOT NULL,
  "date_naissance" date,
  "lieu_naissance" text,
  "demi_pension" boolean DEFAULT false NOT NULL,
  "etablissement_precedent" text,
  "status" text DEFAULT 'pending' NOT NULL,
  "payload" jsonb,
  "eleve_id" uuid REFERENCES "eleve"("id") ON DELETE SET NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "preinscription_etab_idx" ON "preinscription" ("etablissement_id", "status");
CREATE INDEX IF NOT EXISTS "preinscription_site_idx" ON "preinscription" ("etablissement_id", "site_id");
