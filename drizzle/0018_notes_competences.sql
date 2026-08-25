-- Notes Phase 2b — compétences / LSU collège (Option B plan Charlemagne).

CREATE TABLE IF NOT EXISTS "note_competence_domaine" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "cycle" text DEFAULT 'college' NOT NULL,
  "ordre" integer DEFAULT 1 NOT NULL,
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS "note_competence_domaine_uidx"
  ON "note_competence_domaine" ("etablissement_id", "code");

CREATE INDEX IF NOT EXISTS "note_competence_domaine_etab_idx"
  ON "note_competence_domaine" ("etablissement_id");

CREATE TABLE IF NOT EXISTS "note_competence_item" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "domaine_id" uuid NOT NULL REFERENCES "note_competence_domaine"("id") ON DELETE CASCADE,
  "matiere_id" uuid REFERENCES "note_matiere"("id") ON DELETE SET NULL,
  "code" text NOT NULL,
  "libelle" text NOT NULL,
  "ordre" integer DEFAULT 1 NOT NULL,
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS "note_competence_item_uidx"
  ON "note_competence_item" ("etablissement_id", "domaine_id", "code");

CREATE INDEX IF NOT EXISTS "note_competence_item_domaine_idx"
  ON "note_competence_item" ("etablissement_id", "domaine_id");

CREATE TABLE IF NOT EXISTS "note_competence_valeur" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "item_id" uuid NOT NULL REFERENCES "note_competence_item"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL,
  "periode_id" uuid NOT NULL REFERENCES "note_periode"("id") ON DELETE CASCADE,
  "niveau" text,
  "appreciation" text,
  "updated_at" timestamptz DEFAULT now() NOT NULL,
  CONSTRAINT "note_competence_valeur_pk" PRIMARY KEY ("etablissement_id", "item_id", "eleve_id", "periode_id")
);

CREATE INDEX IF NOT EXISTS "note_competence_valeur_eleve_idx"
  ON "note_competence_valeur" ("etablissement_id", "eleve_id");

CREATE INDEX IF NOT EXISTS "note_competence_valeur_periode_idx"
  ON "note_competence_valeur" ("etablissement_id", "periode_id");
