-- Saisie accueil : source / horaires / auteur sur absences élèves,
-- liens métier sur absences RH (professeurs + personnel OGEC).

ALTER TABLE "vs_absence_eleve"
  ADD COLUMN IF NOT EXISTS "heure_debut" text,
  ADD COLUMN IF NOT EXISTS "heure_fin" text,
  ADD COLUMN IF NOT EXISTS "source" text DEFAULT 'appel' NOT NULL,
  ADD COLUMN IF NOT EXISTS "created_by_user_id" text,
  ADD COLUMN IF NOT EXISTS "created_by_nom" text,
  ADD COLUMN IF NOT EXISTS "canal" text;

CREATE INDEX IF NOT EXISTS "vs_absence_eleve_etab_source_date_idx"
  ON "vs_absence_eleve" ("etablissement_id", "source", "date_debut");

ALTER TABLE "absence"
  ADD COLUMN IF NOT EXISTS "personnel_id" text,
  ADD COLUMN IF NOT EXISTS "enseignant_id" text;

CREATE INDEX IF NOT EXISTS "absence_etablissement_personnel_idx"
  ON "absence" ("etablissement_id", "personnel_id");
CREATE INDEX IF NOT EXISTS "absence_etablissement_enseignant_idx"
  ON "absence" ("etablissement_id", "enseignant_id");
