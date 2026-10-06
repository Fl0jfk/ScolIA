-- Socle messaging_* (historiquement créé via ensure-messaging-tables.ts, absent du journal avant 0056).
-- Exécuté par apply-migrations-direct avant 0056_messaging_delivery sur base vierge uniquement (IF NOT EXISTS).
CREATE TABLE IF NOT EXISTS messaging_conversation (
  id text PRIMARY KEY,
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  user_a_id text NOT NULL,
  user_b_id text NOT NULL,
  last_message_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS messaging_conversation_pair_uidx
  ON messaging_conversation (etablissement_id, user_a_id, user_b_id);
CREATE INDEX IF NOT EXISTS messaging_conversation_etab_idx
  ON messaging_conversation (etablissement_id);
CREATE INDEX IF NOT EXISTS messaging_conversation_last_msg_idx
  ON messaging_conversation (etablissement_id, last_message_at);

CREATE TABLE IF NOT EXISTS messaging_participant (
  id text PRIMARY KEY,
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  conversation_id text NOT NULL REFERENCES messaging_conversation(id) ON DELETE CASCADE,
  user_id text NOT NULL,
  last_read_at timestamptz NOT NULL DEFAULT now(),
  last_read_message_id text,
  muted boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS messaging_participant_conv_user_uidx
  ON messaging_participant (conversation_id, user_id);
CREATE INDEX IF NOT EXISTS messaging_participant_user_idx
  ON messaging_participant (etablissement_id, user_id);
CREATE INDEX IF NOT EXISTS messaging_participant_etab_idx
  ON messaging_participant (etablissement_id);

CREATE TABLE IF NOT EXISTS messaging_message (
  id text PRIMARY KEY,
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  conversation_id text NOT NULL REFERENCES messaging_conversation(id) ON DELETE CASCADE,
  sender_id text NOT NULL,
  type text NOT NULL DEFAULT 'text',
  body text,
  reply_to_id text,
  forwarded_from_id text,
  edited_at timestamptz,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS messaging_message_conv_created_idx
  ON messaging_message (etablissement_id, conversation_id, created_at);
CREATE INDEX IF NOT EXISTS messaging_message_etab_idx
  ON messaging_message (etablissement_id);
CREATE INDEX IF NOT EXISTS messaging_message_sender_idx
  ON messaging_message (etablissement_id, sender_id);

CREATE TABLE IF NOT EXISTS messaging_attachment (
  id text PRIMARY KEY,
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  message_id text NOT NULL REFERENCES messaging_message(id) ON DELETE CASCADE,
  s3_key text NOT NULL,
  mime text NOT NULL,
  size integer NOT NULL,
  file_name text NOT NULL,
  width integer,
  height integer,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS messaging_attachment_message_idx
  ON messaging_attachment (message_id);
CREATE INDEX IF NOT EXISTS messaging_attachment_etab_idx
  ON messaging_attachment (etablissement_id);

CREATE TABLE IF NOT EXISTS messaging_reaction (
  id text PRIMARY KEY,
  etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
  message_id text NOT NULL REFERENCES messaging_message(id) ON DELETE CASCADE,
  user_id text NOT NULL,
  emoji text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS messaging_reaction_unique_uidx
  ON messaging_reaction (message_id, user_id, emoji);
CREATE INDEX IF NOT EXISTS messaging_reaction_message_idx
  ON messaging_reaction (message_id);
CREATE INDEX IF NOT EXISTS messaging_reaction_etab_idx
  ON messaging_reaction (etablissement_id);
