-- Socle élève : une classe en cours, préparation année suivante, régimes datés, journal métier.
--> statement-breakpoint

-- Doublons en_cours : on conserve la ligne la plus récente, les autres passent terminee (pas de DELETE).
UPDATE eleve_scolarite AS s
SET statut = 'terminee', updated_at = now()
WHERE s.statut = 'en_cours'
  AND s.id NOT IN (
    SELECT kept.id FROM (
      SELECT DISTINCT ON (etablissement_id, eleve_id) id
      FROM eleve_scolarite
      WHERE statut = 'en_cours'
      ORDER BY etablissement_id, eleve_id, updated_at DESC, created_at DESC
    ) AS kept
  );
--> statement-breakpoint

-- Doublons en_cours/prevue sur la même année : on conserve la plus récente.
UPDATE eleve_scolarite AS s
SET statut = 'terminee', updated_at = now()
WHERE s.statut IN ('en_cours', 'prevue')
  AND s.annee_scolaire_id IS NOT NULL
  AND s.id NOT IN (
    SELECT kept.id FROM (
      SELECT DISTINCT ON (etablissement_id, eleve_id, annee_scolaire_id) id
      FROM eleve_scolarite
      WHERE statut IN ('en_cours', 'prevue')
        AND annee_scolaire_id IS NOT NULL
      ORDER BY etablissement_id, eleve_id, annee_scolaire_id, updated_at DESC, created_at DESC
    ) AS kept
  );
--> statement-breakpoint

CREATE UNIQUE INDEX IF NOT EXISTS "eleve_scolarite_one_en_cours_uidx"
  ON "eleve_scolarite" ("etablissement_id", "eleve_id")
  WHERE "statut" = 'en_cours';
--> statement-breakpoint

CREATE UNIQUE INDEX IF NOT EXISTS "eleve_scolarite_one_active_per_year_uidx"
  ON "eleve_scolarite" ("etablissement_id", "eleve_id", "annee_scolaire_id")
  WHERE "statut" IN ('en_cours', 'prevue') AND "annee_scolaire_id" IS NOT NULL;
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "eleve_regime_periode" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "eleve_id" uuid NOT NULL REFERENCES "eleve"("id") ON DELETE CASCADE,
  "scolarite_id" uuid NOT NULL REFERENCES "eleve_scolarite"("id") ON DELETE CASCADE,
  "regime" text NOT NULL,
  "date_debut" date NOT NULL,
  "date_fin" date,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "eleve_regime_periode_eleve_idx"
  ON "eleve_regime_periode" ("etablissement_id", "eleve_id");
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "eleve_regime_periode_scolarite_idx"
  ON "eleve_regime_periode" ("etablissement_id", "scolarite_id");
--> statement-breakpoint

CREATE UNIQUE INDEX IF NOT EXISTS "eleve_regime_periode_open_uidx"
  ON "eleve_regime_periode" ("etablissement_id", "eleve_id", "scolarite_id")
  WHERE "date_fin" IS NULL;
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "metier_event" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "type" text NOT NULL,
  "aggregate" text NOT NULL,
  "aggregate_id" uuid NOT NULL,
  "eleve_id" uuid REFERENCES "eleve"("id") ON DELETE SET NULL,
  "payload" jsonb DEFAULT '{}'::jsonb NOT NULL,
  "actor_user_id" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "metier_event_etab_created_idx"
  ON "metier_event" ("etablissement_id", "created_at");
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "metier_event_etab_type_idx"
  ON "metier_event" ("etablissement_id", "type");
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "metier_event_eleve_idx"
  ON "metier_event" ("etablissement_id", "eleve_id");
--> statement-breakpoint

-- Backfill : période ouverte depuis le début d’année (ou 1er sept. du label) à partir du régime plat.
INSERT INTO "eleve_regime_periode" (
  "etablissement_id", "eleve_id", "scolarite_id", "regime", "date_debut"
)
SELECT
  s.etablissement_id,
  s.eleve_id,
  s.id,
  btrim(e.regime),
  COALESCE(
    a.starts_on,
    CASE
      WHEN a.label ~ '^\d{4}-\d{4}$' THEN make_date(split_part(a.label, '-', 1)::int, 9, 1)
      ELSE CURRENT_DATE
    END
  )
FROM eleve_scolarite s
INNER JOIN eleve e ON e.id = s.eleve_id AND e.etablissement_id = s.etablissement_id
LEFT JOIN annee_scolaire a ON a.id = s.annee_scolaire_id
WHERE s.statut = 'en_cours'
  AND e.regime IS NOT NULL
  AND btrim(e.regime) <> ''
  AND NOT EXISTS (
    SELECT 1 FROM eleve_regime_periode p
    WHERE p.scolarite_id = s.id AND p.date_fin IS NULL
  );
