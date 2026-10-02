-- Accusés de distribution (delivered) pour la messagerie interne.
ALTER TABLE messaging_participant
  ADD COLUMN IF NOT EXISTS last_delivered_at timestamptz;
ALTER TABLE messaging_participant
  ADD COLUMN IF NOT EXISTS last_delivered_message_id text;
