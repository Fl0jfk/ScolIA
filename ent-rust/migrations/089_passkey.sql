-- Passkeys (WebAuthn) : 2ᵉ facteur et connexion sans mot de passe.
--
-- Choix : l'ENT n'accepte QUE des passkeys (WebAuthn), jamais de TOTP ni
-- d'application d'authentification tierce. Le secret privé reste sur
-- l'appareil ; le serveur ne conserve que la clé publique et un compteur
-- d'usage (détection de clonage).
--
-- `cred_id` : identifiant de credential en base64url — unique au monde, il
-- empêche qu'un même appareil s'enrôle deux fois (au sein d'un même compte
-- comme entre comptes).
--
-- `passkey` : la structure WebAuthn sérialisée (clé publique COSE comprise),
-- telle que produite/vérifiée par le moteur. On ne « réinvente » pas le
-- format : on stocke l'objet opaque du vérificateur.
CREATE TABLE IF NOT EXISTS ent_passkey_credential (
    id               UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    user_id          UUID NOT NULL,
    cred_id          TEXT NOT NULL,
    passkey          JSONB NOT NULL,
    name             TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at     TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_passkey_credential_cred_id_uniq
    ON ent_passkey_credential (cred_id);

CREATE INDEX IF NOT EXISTS ent_passkey_credential_user_idx
    ON ent_passkey_credential (etablissement_id, user_id);

-- Challenges en cours (enrôlement / authentification). Usage unique : la ligne
-- est consommée (supprimée) à la fin du parcours, ce qui bloque le rejeu.
-- `token_hash` n'est jamais renvoyé au client : seul le token opaque l'est.
CREATE TABLE IF NOT EXISTS ent_passkey_challenge (
    id               UUID PRIMARY KEY,
    token_hash       TEXT NOT NULL UNIQUE,
    etablissement_id UUID NOT NULL,
    user_id          UUID NOT NULL,
    kind             TEXT NOT NULL CHECK (kind IN ('register', 'authenticate')),
    state            JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at       TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS ent_passkey_challenge_user_idx
    ON ent_passkey_challenge (user_id, kind);
