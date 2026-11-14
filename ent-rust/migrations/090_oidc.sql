-- SSO OIDC : fédération d'identité par établissement (Google Workspace,
-- Microsoft Entra, Keycloak, tout IdP conforme OpenID Connect).
--
-- Choix : les *endpoints* résolus à la découverte sont **stockés** dans la
-- ligne du fournisseur. La connexion ne dépend donc pas d'un appel réseau pour
-- démarrer (seul l'échange de code et le JWKS sortent sur le réseau), et une
-- panne de l'IdP ne casse pas l'affichage de la page de connexion.
--
-- Le portail public ne voit que `slug` + `libelle` (jamais le secret).
CREATE TABLE IF NOT EXISTS ent_oidc_provider (
    id                     UUID PRIMARY KEY,
    etablissement_id       UUID NOT NULL,
    slug                   TEXT NOT NULL,
    libelle                TEXT NOT NULL,
    issuer                 TEXT NOT NULL,
    client_id              TEXT NOT NULL,
    client_secret          TEXT NOT NULL,
    scopes                 TEXT NOT NULL DEFAULT 'openid email profile',
    redirect_uri           TEXT NOT NULL,
    authorization_endpoint TEXT NOT NULL,
    token_endpoint         TEXT NOT NULL,
    jwks_uri               TEXT NOT NULL,
    actif                  BOOLEAN NOT NULL DEFAULT true,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_oidc_provider_slug_uniq
    ON ent_oidc_provider (etablissement_id, slug);

-- Rattachement d'un compte local (`ent_user`) à un `sub` d'IdP. La clé est
-- `(provider, subject)` : l'identité externe est stable, l'e-mail peut changer.
CREATE TABLE IF NOT EXISTS ent_oidc_identity (
    id          UUID PRIMARY KEY,
    provider_id UUID NOT NULL REFERENCES ent_oidc_provider (id) ON DELETE CASCADE,
    subject     TEXT NOT NULL,
    user_id     UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_oidc_identity_subject_uniq
    ON ent_oidc_identity (provider_id, subject);

CREATE INDEX IF NOT EXISTS ent_oidc_identity_user_idx
    ON ent_oidc_identity (user_id);

-- État du parcours d'autorisation (PKCE + nonce), à **usage unique** : la ligne
-- est consommée au retour de l'IdP, ce qui interdit le rejeu d'un `state`.
CREATE TABLE IF NOT EXISTS ent_oidc_state (
    id               UUID PRIMARY KEY,
    token_hash       TEXT NOT NULL UNIQUE,
    provider_id      UUID NOT NULL,
    etablissement_id UUID NOT NULL,
    code_verifier    TEXT NOT NULL,
    nonce            TEXT NOT NULL,
    redirect_uri     TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at       TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS ent_oidc_state_etab_idx
    ON ent_oidc_state (etablissement_id);
