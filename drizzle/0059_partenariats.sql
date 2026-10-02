-- Partenariats & offres (catalogue public + coupon numérique).

CREATE TABLE IF NOT EXISTS "partenariat_offre" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "slug" text NOT NULL,
  "title" text NOT NULL,
  "short_description" text NOT NULL DEFAULT '',
  "body" text NOT NULL DEFAULT '',
  "logo_s3_key" text,
  "kind" text NOT NULL DEFAULT 'info',
  "cycles" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "niveaux" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "category_label" text NOT NULL DEFAULT '',
  "contact_name" text NOT NULL DEFAULT '',
  "contact_role" text NOT NULL DEFAULT '',
  "contact_email" text NOT NULL DEFAULT '',
  "contact_phone" text NOT NULL DEFAULT '',
  "partner_contact_name" text NOT NULL DEFAULT '',
  "partner_contact_role" text NOT NULL DEFAULT '',
  "partner_contact_email" text NOT NULL DEFAULT '',
  "partner_contact_phone" text NOT NULL DEFAULT '',
  "cta_links" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "tarifs" jsonb NOT NULL DEFAULT '[]'::jsonb,
  "demarche" text NOT NULL DEFAULT '',
  "engagement_text" text NOT NULL DEFAULT 'Je m’engage à régler les sommes dues liées à cette inscription selon les modalités communiquées par l’établissement.',
  "require_signature" integer NOT NULL DEFAULT 1,
  "notify_email" text,
  "max_places" integer,
  "inscription_opens_at" timestamptz,
  "inscription_closes_at" timestamptz,
  "enabled" integer NOT NULL DEFAULT 0,
  "sort_order" integer NOT NULL DEFAULT 0,
  "published_at" timestamptz,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "partenariat_offre_etab_slug_uidx"
  ON "partenariat_offre" ("etablissement_id", "slug");
CREATE INDEX IF NOT EXISTS "partenariat_offre_etab_idx"
  ON "partenariat_offre" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "partenariat_offre_etab_enabled_idx"
  ON "partenariat_offre" ("etablissement_id", "enabled");
CREATE INDEX IF NOT EXISTS "partenariat_offre_etab_sort_idx"
  ON "partenariat_offre" ("etablissement_id", "sort_order");

CREATE TABLE IF NOT EXISTS "partenariat_evenement" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "offre_id" uuid NOT NULL REFERENCES "partenariat_offre"("id") ON DELETE CASCADE,
  "title" text NOT NULL,
  "starts_at" timestamptz NOT NULL,
  "ends_at" timestamptz,
  "location" text NOT NULL DEFAULT '',
  "notes" text NOT NULL DEFAULT '',
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "partenariat_evenement_etab_idx"
  ON "partenariat_evenement" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "partenariat_evenement_offre_idx"
  ON "partenariat_evenement" ("etablissement_id", "offre_id");
CREATE INDEX IF NOT EXISTS "partenariat_evenement_starts_idx"
  ON "partenariat_evenement" ("offre_id", "starts_at");

CREATE TABLE IF NOT EXISTS "partenariat_inscription" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "offre_id" uuid NOT NULL REFERENCES "partenariat_offre"("id") ON DELETE CASCADE,
  "eleve_first_name" text NOT NULL,
  "eleve_last_name" text NOT NULL,
  "eleve_niveau" text NOT NULL DEFAULT '',
  "eleve_classe" text NOT NULL DEFAULT '',
  "eleve_birth_date" text,
  "parent_first_name" text NOT NULL,
  "parent_last_name" text NOT NULL,
  "parent_email" text NOT NULL,
  "parent_phone" text NOT NULL DEFAULT '',
  "engagement_accepted_at" timestamptz NOT NULL,
  "signature_s3_key" text,
  "status" text NOT NULL DEFAULT 'soumise',
  "admin_note" text NOT NULL DEFAULT '',
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "partenariat_inscription_etab_idx"
  ON "partenariat_inscription" ("etablissement_id");
CREATE INDEX IF NOT EXISTS "partenariat_inscription_offre_idx"
  ON "partenariat_inscription" ("etablissement_id", "offre_id");
CREATE INDEX IF NOT EXISTS "partenariat_inscription_status_idx"
  ON "partenariat_inscription" ("offre_id", "status");
CREATE INDEX IF NOT EXISTS "partenariat_inscription_email_idx"
  ON "partenariat_inscription" ("offre_id", "parent_email");
