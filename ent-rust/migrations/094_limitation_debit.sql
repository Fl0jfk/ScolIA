-- 094_limitation_debit.sql — Limitation de débit des routes d'authentification
-- publiques (anti-abus : bombardement de mails, challenges en boucle, etc.).
--
-- Seau à jetons partagé entre instances ent-api : une seule requête atomique
-- (INSERT ... ON CONFLICT ... DO UPDATE ... RETURNING) recharge le seau, retire
-- un jeton s'il en reste et renvoie la décision. La clé est une empreinte
-- (route + type de clé + valeur) : aucune adresse e-mail ni IP en clair.
-- Table volatile (exclue des inventaires PRA), purgée de façon opportuniste.

CREATE TABLE IF NOT EXISTS ent_rate_limit (
    bucket_key TEXT PRIMARY KEY,
    tokens DOUBLE PRECISION NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_ok BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX IF NOT EXISTS ent_rate_limit_updated_idx
    ON ent_rate_limit (updated_at);
