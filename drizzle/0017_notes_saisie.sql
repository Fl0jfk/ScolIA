-- Notes Phase 2 — devoirs, notes, moyennes cache.

CREATE TABLE IF NOT EXISTS "note_devoir" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "matiere_id" uuid NOT NULL REFERENCES "note_matiere"("id") ON DELETE CASCADE,
  "periode_id" uuid NOT NULL REFERENCES "note_periode"("id") ON DELETE CASCADE,
  "type_devoir_id" uuid REFERENCES "note_type_devoir"("id") ON DELETE SET NULL,
  "classe" text NOT NULL,
  "groupe_id" uuid,
  "libelle" text NOT NULL,
  "date_devoir" date,
  "coefficient" numeric(6, 2) DEFAULT '1' NOT NULL,
  "created_by_user_id" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "note_devoir_classe_idx"
  ON "note_devoir" ("etablissement_id", "classe");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "note_devoir_periode_idx"
  ON "note_devoir" ("etablissement_id", "periode_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "note_valeur" (
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "devoir_id" uuid NOT NULL REFERENCES "note_devoir"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL,
  "valeur" numeric(5, 2),
  "absent" boolean DEFAULT false NOT NULL,
  "dispense" boolean DEFAULT false NOT NULL,
  "appreciation" text,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "note_valeur_pk" PRIMARY KEY("etablissement_id","devoir_id","eleve_id")
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "note_valeur_eleve_idx"
  ON "note_valeur" ("etablissement_id", "eleve_id");
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "note_moyenne_eleve" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL,
  "matiere_id" uuid NOT NULL REFERENCES "note_matiere"("id") ON DELETE CASCADE,
  "periode_id" uuid NOT NULL REFERENCES "note_periode"("id") ON DELETE CASCADE,
  "moyenne" numeric(5, 2),
  "nb_notes" integer DEFAULT 0 NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "note_moyenne_eleve_uidx"
  ON "note_moyenne_eleve" ("etablissement_id", "eleve_id", "matiere_id", "periode_id");
