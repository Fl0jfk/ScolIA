-- Délai minimum (heures avant le RDV) pour remettre un créneau libre quand la famille
-- annule ou modifie son RDV. En deçà, le créneau est retiré (« ANNULÉ ») pour la direction.
ALTER TABLE "rdv_inscription_direction"
  ADD COLUMN IF NOT EXISTS "release_min_hours" integer DEFAULT 48 NOT NULL;
