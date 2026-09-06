-- Portes ouvertes — tables Postgres (idempotent)
-- Appliquer via: psql "$DATABASE_URL" -f scripts/sql/portes-ouvertes.sql

CREATE TABLE IF NOT EXISTS portes_ouvertes_config (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  title text NOT NULL DEFAULT 'Portes ouvertes',
  intro text NOT NULL DEFAULT '',
  address text NOT NULL DEFAULT '',
  maps_url text,
  notify_email text,
  preinscription_url text,
  follow_up_delay_minutes integer NOT NULL DEFAULT 60,
  consent_label text NOT NULL DEFAULT 'J''accepte que mes coordonnées soient utilisées pour organiser ma visite et me recontacter si besoin.',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS portes_ouvertes_config_etab_uidx ON portes_ouvertes_config (etablissement_id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_config_etab_idx ON portes_ouvertes_config (etablissement_id);

CREATE TABLE IF NOT EXISTS portes_ouvertes_slot (
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  id text NOT NULL,
  cycle text NOT NULL,
  label text NOT NULL,
  start_at timestamptz NOT NULL,
  end_at timestamptz NOT NULL,
  max_places integer,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS portes_ouvertes_slot_etab_id_uidx ON portes_ouvertes_slot (etablissement_id, id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_slot_etab_cycle_idx ON portes_ouvertes_slot (etablissement_id, cycle);
CREATE INDEX IF NOT EXISTS portes_ouvertes_slot_etab_start_idx ON portes_ouvertes_slot (etablissement_id, start_at);

CREATE TABLE IF NOT EXISTS portes_ouvertes_registration (
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  id text NOT NULL,
  slot_id text NOT NULL,
  slot_label text,
  slot_start_at timestamptz,
  slot_end_at timestamptz,
  first_name text NOT NULL,
  last_name text NOT NULL,
  email text NOT NULL,
  phone text,
  children_info text,
  child_first_name text,
  child_last_name text,
  cycle text,
  classe_souhaitee text,
  consent boolean NOT NULL DEFAULT true,
  source text,
  recorded_by_user_id text,
  recorded_by_name text,
  last_modified_by_user_id text,
  last_modified_by_name text,
  visited_at timestamptz,
  follow_up_due_at timestamptz,
  follow_up_email_sent_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS portes_ouvertes_reg_etab_id_uidx ON portes_ouvertes_registration (etablissement_id, id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_reg_etab_idx ON portes_ouvertes_registration (etablissement_id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_reg_slot_idx ON portes_ouvertes_registration (etablissement_id, slot_id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_reg_cycle_idx ON portes_ouvertes_registration (etablissement_id, cycle);
CREATE INDEX IF NOT EXISTS portes_ouvertes_reg_email_idx ON portes_ouvertes_registration (etablissement_id, email);
CREATE INDEX IF NOT EXISTS portes_ouvertes_reg_followup_idx ON portes_ouvertes_registration (follow_up_due_at, follow_up_email_sent_at);

CREATE TABLE IF NOT EXISTS portes_ouvertes_slot_staff (
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  slot_id text NOT NULL,
  role text NOT NULL,
  ref_id text NOT NULL,
  display_name text NOT NULL,
  meta jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS portes_ouvertes_staff_etab_idx ON portes_ouvertes_slot_staff (etablissement_id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_staff_slot_idx ON portes_ouvertes_slot_staff (etablissement_id, slot_id);
CREATE INDEX IF NOT EXISTS portes_ouvertes_staff_role_idx ON portes_ouvertes_slot_staff (etablissement_id, slot_id, role);
CREATE UNIQUE INDEX IF NOT EXISTS portes_ouvertes_staff_unique_uidx ON portes_ouvertes_slot_staff (etablissement_id, slot_id, role, ref_id);

-- Colonnes ajoutées si table déjà créée sans elles
ALTER TABLE portes_ouvertes_config ADD COLUMN IF NOT EXISTS preinscription_url text;
ALTER TABLE portes_ouvertes_config ADD COLUMN IF NOT EXISTS follow_up_delay_minutes integer NOT NULL DEFAULT 60;
ALTER TABLE portes_ouvertes_registration ADD COLUMN IF NOT EXISTS visited_at timestamptz;
ALTER TABLE portes_ouvertes_registration ADD COLUMN IF NOT EXISTS follow_up_due_at timestamptz;
ALTER TABLE portes_ouvertes_registration ADD COLUMN IF NOT EXISTS follow_up_email_sent_at timestamptz;
