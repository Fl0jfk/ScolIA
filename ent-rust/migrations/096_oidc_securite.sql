-- Socle 1 / A7 — SSO OIDC : état lié au navigateur.
--
-- `binding_hash` : SHA-256 (hex) du secret posé dans un cookie `__Host-` court au
-- `start` ; le `complete` exige le même secret (anti login-CSRF / rejeu de
-- `code` + `state` depuis un autre navigateur). NULL = état antérieur à cette
-- migration : refusé au `complete` (les états vivent 10 minutes).
ALTER TABLE ent_oidc_state ADD COLUMN IF NOT EXISTS binding_hash TEXT;

-- `client_secret` reste dans la même colonne : valeur chiffrée au format
-- `enc:v1:…` (AES-256-GCM, clé `ENT_SECRETS_KEY`) ; une valeur sans ce préfixe
-- est un secret hérité en clair, relu tel quel puis rechiffré (voir
-- docs/SOCLE1_SECURITE.md, A7).
COMMENT ON COLUMN ent_oidc_provider.client_secret IS
    'enc:v1:<base64url(nonce||chiffré||tag)> (AES-256-GCM) ; héritage en clair toléré en lecture';
