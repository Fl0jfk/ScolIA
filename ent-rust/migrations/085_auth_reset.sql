-- 085_auth_reset.sql — Réinitialisation de mot de passe en libre-service.
-- Jeton opaque à usage unique : seul le SHA-256 est stocké ; l'expiration et
-- l'usage unique sont vérifiés en base (UPDATE ... RETURNING atomique).
-- Ne stocke pas l'e-mail en clair côté table (le lien porte le jeton brut).

CREATE TABLE IF NOT EXISTS ent_password_reset (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES ent_user(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_password_reset_user_idx
    ON ent_password_reset (user_id);

CREATE INDEX IF NOT EXISTS ent_password_reset_expires_idx
    ON ent_password_reset (expires_at);
