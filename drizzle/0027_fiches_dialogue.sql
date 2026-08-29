-- Fiches de dialogue (orientation) — campagnes configurables
CREATE TABLE IF NOT EXISTS "fd_campagne" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid,
  "label" text NOT NULL,
  "annee_label" text NOT NULL,
  "site_key" text,
  "calendrier_mode" text DEFAULT 'trimestre' NOT NULL,
  "template_key" text,
  "statut" text DEFAULT 'brouillon' NOT NULL,
  "catalogue" jsonb DEFAULT '{"destinations":[],"options":[],"fields":[]}'::jsonb NOT NULL,
  "appel_config" jsonb DEFAULT '{"enabled":true}'::jsonb NOT NULL,
  "delai_famille_jours" integer DEFAULT 7 NOT NULL,
  "classes_cibles" jsonb DEFAULT '[]'::jsonb NOT NULL,
  "created_by_user_id" text,
  "opened_at" timestamptz,
  "closed_at" timestamptz,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "fd_campagne_etablissement_idx" ON "fd_campagne" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "fd_campagne_annee_idx" ON "fd_campagne" ("etablissement_id","annee_label");
CREATE INDEX IF NOT EXISTS "fd_campagne_statut_idx" ON "fd_campagne" ("etablissement_id","statut");

CREATE TABLE IF NOT EXISTS "fd_etape" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "campagne_id" uuid NOT NULL REFERENCES "fd_campagne"("id") ON DELETE CASCADE,
  "ordre" integer NOT NULL,
  "kind" text NOT NULL,
  "label" text NOT NULL,
  "description" text,
  "opens_at" timestamptz,
  "closes_at" timestamptz,
  "conseil_date" date,
  "optionnelle" boolean DEFAULT false NOT NULL,
  "gelee" boolean DEFAULT false NOT NULL,
  "frozen_at" timestamptz,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "fd_etape_campagne_idx" ON "fd_etape" ("campagne_id");
CREATE UNIQUE INDEX IF NOT EXISTS "fd_etape_campagne_ordre_uidx" ON "fd_etape" ("campagne_id","ordre");
CREATE INDEX IF NOT EXISTS "fd_etape_etablissement_idx" ON "fd_etape" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "fd_fiche" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "campagne_id" uuid NOT NULL REFERENCES "fd_campagne"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL,
  "eleve_nom" text NOT NULL,
  "eleve_prenom" text NOT NULL,
  "classe_actuelle" text DEFAULT '' NOT NULL,
  "options_actuelles" jsonb DEFAULT '[]'::jsonb NOT NULL,
  "parent_emails" jsonb DEFAULT '[]'::jsonb NOT NULL,
  "statut" text DEFAULT 'a_envoyer' NOT NULL,
  "etape_courante_id" uuid,
  "last_sent_at" timestamptz,
  "last_reminder_at" timestamptz,
  "reminder_count" integer DEFAULT 0 NOT NULL,
  "acceptation" jsonb,
  "accepted_at" timestamptz,
  "refused_at" timestamptz,
  "closed_at" timestamptz,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "fd_fiche_campagne_eleve_uidx" ON "fd_fiche" ("campagne_id","eleve_id");
CREATE INDEX IF NOT EXISTS "fd_fiche_campagne_idx" ON "fd_fiche" ("campagne_id");
CREATE INDEX IF NOT EXISTS "fd_fiche_eleve_idx" ON "fd_fiche" ("eleve_id");
CREATE INDEX IF NOT EXISTS "fd_fiche_statut_idx" ON "fd_fiche" ("etablissement_id","statut");
CREATE INDEX IF NOT EXISTS "fd_fiche_classe_idx" ON "fd_fiche" ("campagne_id","classe_actuelle");

CREATE TABLE IF NOT EXISTS "fd_reponse" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "fiche_id" uuid NOT NULL REFERENCES "fd_fiche"("id") ON DELETE CASCADE,
  "etape_id" uuid NOT NULL REFERENCES "fd_etape"("id") ON DELETE CASCADE,
  "auteur_role" text NOT NULL,
  "auteur_user_id" text,
  "auteur_label" text,
  "payload" jsonb NOT NULL,
  "submitted_at" timestamptz DEFAULT now() NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "fd_reponse_fiche_idx" ON "fd_reponse" ("fiche_id");
CREATE INDEX IF NOT EXISTS "fd_reponse_etape_idx" ON "fd_reponse" ("etape_id");
CREATE UNIQUE INDEX IF NOT EXISTS "fd_reponse_fiche_etape_auteur_uidx" ON "fd_reponse" ("fiche_id","etape_id","auteur_role");

CREATE TABLE IF NOT EXISTS "fd_signature" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "fiche_id" uuid NOT NULL REFERENCES "fd_fiche"("id") ON DELETE CASCADE,
  "etape_id" uuid NOT NULL REFERENCES "fd_etape"("id") ON DELETE CASCADE,
  "role" text NOT NULL,
  "signer_name" text NOT NULL,
  "signer_email" text,
  "method" text DEFAULT 'pad' NOT NULL,
  "signature_png_base64" text,
  "signed_at" timestamptz DEFAULT now() NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "fd_signature_fiche_idx" ON "fd_signature" ("fiche_id");
CREATE UNIQUE INDEX IF NOT EXISTS "fd_signature_fiche_etape_role_uidx" ON "fd_signature" ("fiche_id","etape_id","role");

CREATE TABLE IF NOT EXISTS "fd_token" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "fiche_id" uuid NOT NULL REFERENCES "fd_fiche"("id") ON DELETE CASCADE,
  "etape_id" uuid,
  "token" text NOT NULL,
  "secure_code" text,
  "email" text,
  "purpose" text DEFAULT 'saisie' NOT NULL,
  "expires_at" timestamptz,
  "used_at" timestamptz,
  "revoked_at" timestamptz,
  "created_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "fd_token_token_uidx" ON "fd_token" ("token");
CREATE INDEX IF NOT EXISTS "fd_token_fiche_idx" ON "fd_token" ("fiche_id");
CREATE INDEX IF NOT EXISTS "fd_token_email_code_idx" ON "fd_token" ("email","secure_code");

CREATE TABLE IF NOT EXISTS "fd_pdf_archive" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "fiche_id" uuid NOT NULL REFERENCES "fd_fiche"("id") ON DELETE CASCADE,
  "etape_id" uuid,
  "kind" text NOT NULL,
  "title" text NOT NULL,
  "s3_key" text,
  "eleve_document_id" uuid,
  "created_at" timestamptz DEFAULT now() NOT NULL
);
CREATE INDEX IF NOT EXISTS "fd_pdf_archive_fiche_idx" ON "fd_pdf_archive" ("fiche_id");
CREATE INDEX IF NOT EXISTS "fd_pdf_archive_etablissement_idx" ON "fd_pdf_archive" ("etablissement_id");
