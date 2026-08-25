-- VS Phase 2 — appels de classe & absences élèves.

CREATE TABLE IF NOT EXISTS "vs_appel" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "date_appel" date NOT NULL,
  "creneau_id" uuid REFERENCES "edt_creneau"("id") ON DELETE SET NULL,
  "classe" text NOT NULL,
  "heure_debut" text,
  "heure_fin" text,
  "matiere_libelle" text,
  "enseignant_user_id" text,
  "enseignant_nom" text,
  "statut" text DEFAULT 'en_cours' NOT NULL,
  "clos_at" timestamptz,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS "vs_appel_etab_date_idx"
  ON "vs_appel" ("etablissement_id", "date_appel");
CREATE INDEX IF NOT EXISTS "vs_appel_classe_idx"
  ON "vs_appel" ("etablissement_id", "classe", "date_appel");
CREATE UNIQUE INDEX IF NOT EXISTS "vs_appel_creneau_date_uidx"
  ON "vs_appel" ("etablissement_id", "creneau_id", "date_appel");

CREATE TABLE IF NOT EXISTS "vs_appel_ligne" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "appel_id" uuid NOT NULL REFERENCES "vs_appel"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "statut" text DEFAULT 'present' NOT NULL,
  "retard_minutes" integer,
  "note" text,
  "updated_at" timestamptz DEFAULT now() NOT NULL,
  CONSTRAINT "vs_appel_ligne_pk" PRIMARY KEY ("etablissement_id", "appel_id", "eleve_id")
);

CREATE INDEX IF NOT EXISTS "vs_appel_ligne_eleve_idx"
  ON "vs_appel_ligne" ("etablissement_id", "eleve_id");
CREATE INDEX IF NOT EXISTS "vs_appel_ligne_statut_idx"
  ON "vs_appel_ligne" ("etablissement_id", "statut");

CREATE TABLE IF NOT EXISTS "vs_absence_eleve" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "appel_id" uuid REFERENCES "vs_appel"("id") ON DELETE SET NULL,
  "date_debut" date NOT NULL,
  "date_fin" date NOT NULL,
  "type" text DEFAULT 'absence' NOT NULL,
  "statut" text DEFAULT 'a_traiter' NOT NULL,
  "justifie" boolean DEFAULT false NOT NULL,
  "motif" text,
  "justificatif_key" text,
  "relance_at" timestamptz,
  "traite_par_user_id" text,
  "traite_at" timestamptz,
  "note_cpe" text,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS "vs_absence_eleve_etab_statut_idx"
  ON "vs_absence_eleve" ("etablissement_id", "statut");
CREATE INDEX IF NOT EXISTS "vs_absence_eleve_eleve_idx"
  ON "vs_absence_eleve" ("etablissement_id", "eleve_id");
CREATE INDEX IF NOT EXISTS "vs_absence_eleve_date_idx"
  ON "vs_absence_eleve" ("etablissement_id", "date_debut");
CREATE UNIQUE INDEX IF NOT EXISTS "vs_absence_eleve_appel_uidx"
  ON "vs_absence_eleve" ("etablissement_id", "appel_id", "eleve_id");
