-- Web Push serveur : derniers compteurs de « nouveautés de l'ENT » notifiés par utilisateur.
-- Purement additif : nouvelle table, rien d'existant n'est modifié.
CREATE TABLE IF NOT EXISTS "web_push_signal_state" (
  "user_id" text NOT NULL,
  "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
  "signal_id" text NOT NULL,
  "count" integer NOT NULL DEFAULT 0,
  "updated_at" timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY ("user_id", "signal_id")
);
