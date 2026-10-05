-- Régime demandé (DP | EXT | INT) — titre Google Agenda + suivi.
ALTER TABLE "rdv_inscription_booking"
  ADD COLUMN IF NOT EXISTS "regime" text;
