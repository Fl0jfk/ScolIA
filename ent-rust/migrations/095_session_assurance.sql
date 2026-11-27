-- Socle 1 (A2, A3) : sessions à deux durées, niveau d'assurance, portée, appareils.
--
-- D-011 : web = 2 h d'inactivité et 12 h au plus ; mobile = longue, prolongée à
-- l'usage, révocable. D-009 : passkey obligatoire (politique par établissement) ;
-- tant qu'un compte soumis à l'obligation n'a pas de passkey, sa session est
-- limitée à l'enrôlement (`portee = 'enrolement'`).
-- Additif : les sessions existantes deviennent « web, mot de passe, complète ».

ALTER TABLE ent_session
    ADD COLUMN IF NOT EXISTS client TEXT NOT NULL DEFAULT 'web',
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS inactivite_max_secs INTEGER,
    ADD COLUMN IF NOT EXISTS assurance TEXT NOT NULL DEFAULT 'mot_de_passe',
    ADD COLUMN IF NOT EXISTS portee TEXT NOT NULL DEFAULT 'complete',
    ADD COLUMN IF NOT EXISTS appareil TEXT,
    ADD COLUMN IF NOT EXISTS ip_creation TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_session_client_chk') THEN
        ALTER TABLE ent_session ADD CONSTRAINT ent_session_client_chk
            CHECK (client IN ('web', 'mobile'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_session_assurance_chk') THEN
        ALTER TABLE ent_session ADD CONSTRAINT ent_session_assurance_chk
            CHECK (assurance IN ('mot_de_passe', 'passkey', 'sso'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_session_portee_chk') THEN
        ALTER TABLE ent_session ADD CONSTRAINT ent_session_portee_chk
            CHECK (portee IN ('complete', 'enrolement'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS ent_session_user_actives_idx
    ON ent_session (user_id) WHERE revoked_at IS NULL;

-- Politique de sécurité par établissement (passkey obligatoire par rôle).
CREATE TABLE IF NOT EXISTS ent_politique_securite (
    etablissement_id UUID PRIMARY KEY,
    passkey_obligatoire_roles TEXT[] NOT NULL DEFAULT '{}',
    modifie_par_user_id UUID REFERENCES ent_user (id),
    modifie_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Le parcours « mot de passe puis passkey » conserve le type de client choisi.
ALTER TABLE ent_passkey_challenge
    ADD COLUMN IF NOT EXISTS client TEXT NOT NULL DEFAULT 'web';
