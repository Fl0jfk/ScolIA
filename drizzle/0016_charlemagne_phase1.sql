-- Charlemagne Phase 1 — nomenclatures, notes config, groupes, facturation, EDT.
-- FKs internes + etablissement ; eleve_id / foyer_id / annee_scolaire_id sans FK (uuid libres).

ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "photo_key" text;
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "ref_nomenclature" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "type" text NOT NULL,
  "code" text NOT NULL,
  "libelle_court" text,
  "libelle_long" text,
  "source" text DEFAULT 'siecle' NOT NULL,
  "metadata_json" jsonb,
  "valid_from" date,
  "valid_to" date,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "ref_nomenclature_etab_type_code_uidx"
  ON "ref_nomenclature" ("etablissement_id", "type", "code");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "ref_nomenclature_etab_type_idx"
  ON "ref_nomenclature" ("etablissement_id", "type");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "ref_etablissement" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "code_rne" text NOT NULL,
  "code_nature" text,
  "code_type" text,
  "code_secteur" text,
  "sigle" text,
  "denom_princ" text,
  "denom_compl" text,
  "adresse" text,
  "date_ouverture" date,
  "date_fermeture" date,
  "source" text DEFAULT 'siecle' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "ref_etablissement_rne_uidx"
  ON "ref_etablissement" ("code_rne");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "nomenclature_import_log" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "fichier" text NOT NULL,
  "source" text DEFAULT 'siecle_xml' NOT NULL,
  "date_import" timestamp with time zone DEFAULT now() NOT NULL,
  "statut" text DEFAULT 'ok' NOT NULL,
  "nb_inserts" integer DEFAULT 0 NOT NULL,
  "nb_updates" integer DEFAULT 0 NOT NULL,
  "nb_deletes" integer DEFAULT 0 NOT NULL,
  "rapport_json" jsonb
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "nomenclature_import_log_etab_idx"
  ON "nomenclature_import_log" ("etablissement_id", "date_import");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "note_matiere" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "groupe_matiere" text,
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "note_matiere_etab_code_uidx"
  ON "note_matiere" ("etablissement_id", "code");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "note_matiere_etab_idx"
  ON "note_matiere" ("etablissement_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "note_periode" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "niveau_modele" text DEFAULT 'tous' NOT NULL,
  "date_debut" date,
  "date_fin" date,
  "statut" text DEFAULT 'ouverte' NOT NULL,
  "ordre" integer DEFAULT 1 NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "note_periode_etab_idx"
  ON "note_periode" ("etablissement_id");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "note_periode_etab_code_uidx"
  ON "note_periode" ("etablissement_id", "code", "annee_scolaire_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "note_type_devoir" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "coef_defaut" numeric(6, 2) DEFAULT '1',
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "note_type_devoir_etab_code_uidx"
  ON "note_type_devoir" ("etablissement_id", "code");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "note_matiere_classe" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "matiere_id" uuid NOT NULL REFERENCES "note_matiere"("id") ON DELETE CASCADE,
  "classe" text NOT NULL,
  "enseignant_user_id" text,
  "enseignant_nom" text,
  "coef" numeric(6, 2) DEFAULT '1' NOT NULL,
  "compte_dans_mg" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "note_matiere_classe_uidx"
  ON "note_matiere_classe" ("etablissement_id", "matiere_id", "classe");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "note_matiere_classe_classe_idx"
  ON "note_matiere_classe" ("etablissement_id", "classe");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "groupe_pedagogique" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "type" text DEFAULT 'autre' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "groupe_pedagogique_etab_code_uidx"
  ON "groupe_pedagogique" ("etablissement_id", "code");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "groupe_pedagogique_etab_idx"
  ON "groupe_pedagogique" ("etablissement_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "groupe_pedagogique_membre" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "groupe_id" uuid NOT NULL REFERENCES "groupe_pedagogique"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "groupe_pedagogique_membre_pk"
    PRIMARY KEY ("etablissement_id", "groupe_id", "eleve_id")
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "groupe_pedagogique_membre_eleve_idx"
  ON "groupe_pedagogique_membre" ("etablissement_id", "eleve_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "tarif" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "compte_produit" text,
  "tva_taux" numeric(5, 2) DEFAULT '0' NOT NULL,
  "periodicite" text DEFAULT 'mensuel' NOT NULL,
  "portee" text DEFAULT 'autre' NOT NULL,
  "portee_valeur" text,
  "prix_unitaire" numeric(12, 2) DEFAULT '0' NOT NULL,
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "tarif_etab_code_uidx"
  ON "tarif" ("etablissement_id", "code", "annee_scolaire_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "tarif_etab_idx"
  ON "tarif" ("etablissement_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "foyer_facturation" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "foyer_id" uuid NOT NULL,
  "code_auxiliaire" text,
  "categorie_quotient" text,
  "quotient_familial" numeric(12, 2),
  "iban" text,
  "bic" text,
  "rum" text,
  "mandat_date" date,
  "accepte_prelevement" boolean DEFAULT false NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "foyer_facturation_foyer_uidx"
  ON "foyer_facturation" ("etablissement_id", "foyer_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "foyer_facturation_etab_idx"
  ON "foyer_facturation" ("etablissement_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "facture" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "foyer_id" uuid NOT NULL,
  "annee_scolaire_id" uuid,
  "numero" text NOT NULL,
  "statut" text DEFAULT 'brouillon' NOT NULL,
  "date_emission" date,
  "date_echeance" date,
  "total_ht" numeric(12, 2) DEFAULT '0' NOT NULL,
  "total_ttc" numeric(12, 2) DEFAULT '0' NOT NULL,
  "pdf_key" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "facture_etab_numero_uidx"
  ON "facture" ("etablissement_id", "numero");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "facture_foyer_idx"
  ON "facture" ("etablissement_id", "foyer_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "facture_statut_idx"
  ON "facture" ("etablissement_id", "statut");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "facture_ligne" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "facture_id" uuid NOT NULL REFERENCES "facture"("id") ON DELETE CASCADE,
  "eleve_id" uuid,
  "tarif_id" uuid,
  "libelle" text NOT NULL,
  "periode" text,
  "quantite" numeric(10, 2) DEFAULT '1' NOT NULL,
  "prix_unitaire" numeric(12, 2) DEFAULT '0' NOT NULL,
  "remise" numeric(12, 2) DEFAULT '0' NOT NULL,
  "total_ht" numeric(12, 2) DEFAULT '0' NOT NULL,
  "total_ttc" numeric(12, 2) DEFAULT '0' NOT NULL,
  "ordre" integer DEFAULT 1 NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "facture_ligne_facture_idx"
  ON "facture_ligne" ("etablissement_id", "facture_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "encaissement" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "foyer_id" uuid NOT NULL,
  "mode" text DEFAULT 'virement' NOT NULL,
  "montant" numeric(12, 2) NOT NULL,
  "date_encaissement" date NOT NULL,
  "reference" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "encaissement_foyer_idx"
  ON "encaissement" ("etablissement_id", "foyer_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "facture_encaissement" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "facture_id" uuid NOT NULL REFERENCES "facture"("id") ON DELETE CASCADE,
  "encaissement_id" uuid NOT NULL REFERENCES "encaissement"("id") ON DELETE CASCADE,
  "montant" numeric(12, 2) NOT NULL,
  CONSTRAINT "facture_encaissement_pk"
    PRIMARY KEY ("etablissement_id", "facture_id", "encaissement_id")
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "calendrier_scolaire" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid,
  "label" text NOT NULL,
  "date_debut" date NOT NULL,
  "date_fin" date NOT NULL,
  "type" text DEFAULT 'vacances' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "calendrier_scolaire_etab_idx"
  ON "calendrier_scolaire" ("etablissement_id", "annee_scolaire_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "edt_creneau" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "annee_scolaire_id" uuid,
  "jour_semaine" integer NOT NULL,
  "heure_debut" text NOT NULL,
  "heure_fin" text NOT NULL,
  "classe" text,
  "groupe_id" uuid,
  "matiere_id" uuid,
  "enseignant_nom" text,
  "salle" text,
  "semaine" text DEFAULT 'AB' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "edt_creneau_classe_idx"
  ON "edt_creneau" ("etablissement_id", "classe");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "edt_creneau_jour_idx"
  ON "edt_creneau" ("etablissement_id", "jour_semaine");
