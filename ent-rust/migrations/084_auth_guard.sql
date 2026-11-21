-- 084_auth_guard.sql — Garde anti-force brute sur la connexion (multi-instance).
-- Le compteur vit en Postgres : plusieurs conteneurs ent-api partagent la même
-- politique de verrouillage (pas d'état en mémoire par process).

CREATE TABLE IF NOT EXISTS ent_login_guard (
    email TEXT PRIMARY KEY,
    failed_count INT NOT NULL DEFAULT 0,
    window_start TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until TIMESTAMPTZ,
    last_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_login_guard_last_attempt_idx
    ON ent_login_guard (last_attempt_at);
