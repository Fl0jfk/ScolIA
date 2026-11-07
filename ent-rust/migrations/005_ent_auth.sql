-- Auth ENT 100 % Rust — users, membership multi-tenant, sessions Postgres.
-- Cible prod (tables ent_*). Pas Better-Auth ni NextAuth sur ce chemin.

CREATE TABLE IF NOT EXISTS ent_user (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_user_email_unique UNIQUE (email)
);

CREATE TABLE IF NOT EXISTS ent_membership (
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, etablissement_id)
);

CREATE INDEX IF NOT EXISTS ent_membership_etab_idx
    ON ent_membership (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_session (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    CONSTRAINT ent_session_token_unique UNIQUE (token_hash)
);

CREATE INDEX IF NOT EXISTS ent_session_user_idx
    ON ent_session (user_id);

CREATE INDEX IF NOT EXISTS ent_session_expires_idx
    ON ent_session (expires_at)
    WHERE revoked_at IS NULL;
