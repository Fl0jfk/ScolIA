-- Socle 11, lot MB0 (plage 2220–2239) — correctifs de sécurité mobile DS1, DS2, DS3, DS4.
-- Réf. : docs/socles/11_APPLICATIONS_MOBILES.md §9 (MB0), 11_MOBILE.md §27.1 (MO0).
-- Additive et rejouable ; aucune migration versée n'est modifiée. Aucune colonne JSON (D-036).

-- DS1/DS3 : l'inscription du jeton de notification est liée à la session qui l'a faite ; l'empreinte
-- SHA-256 du jeton permet de le retrouver sans le relire (aucune route ne le renvoie plus).
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS session_id UUID REFERENCES ent_session (id) ON DELETE CASCADE;
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS token_empreinte TEXT;
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS revoque_motif TEXT;

UPDATE ent_push_device
   SET token_empreinte = encode(sha256(convert_to(token, 'UTF8')), 'hex')
 WHERE token_empreinte IS NULL;

-- DS3 : réaffecter un jeton = révoquer l'ancienne inscription (trace) puis en créer une nouvelle ;
-- l'unicité ne porte donc que sur les inscriptions actives.
ALTER TABLE ent_push_device DROP CONSTRAINT IF EXISTS ent_push_device_token_uq;
CREATE UNIQUE INDEX IF NOT EXISTS ent_push_device_token_actif_uq
    ON ent_push_device (etablissement_id, token) WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS ent_push_device_session_idx
    ON ent_push_device (session_id) WHERE session_id IS NOT NULL AND revoked_at IS NULL;

ALTER TABLE ent_push_device DROP CONSTRAINT IF EXISTS ent_push_device_revoque_motif_chk;
ALTER TABLE ent_push_device ADD CONSTRAINT ent_push_device_revoque_motif_chk CHECK (
    revoque_motif IS NULL OR revoque_motif IN ('session_revoquee', 'reaffecte', 'desinscription', 'expire')
);

-- DS1 : révoquer une session coupe, dans la même transaction, les notifications de son appareil,
-- quel que soit le chemin de révocation (déconnexion, appareil perdu, accès fermé, révocation globale).
CREATE OR REPLACE FUNCTION ent_trg_session_revoque_push() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    precedent TEXT := current_setting('app.etablissement_id', true);
BEGIN
    IF NEW.revoked_at IS NOT NULL AND OLD.revoked_at IS NULL THEN
        PERFORM set_config('app.etablissement_id', NEW.etablissement_id::text, true);
        UPDATE ent_push_device
           SET revoked_at = now(), revoque_motif = 'session_revoquee'
         WHERE session_id = NEW.id AND revoked_at IS NULL;
        PERFORM set_config('app.etablissement_id', COALESCE(precedent, ''), true);
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_session_revoque_push ON ent_session;
CREATE TRIGGER ent_session_revoque_push
    AFTER UPDATE OF revoked_at ON ent_session
    FOR EACH ROW EXECUTE FUNCTION ent_trg_session_revoque_push();

-- Les inscriptions antérieures au lot n'ont pas de session : elles sont révoquées à 30 jours sans
-- réinscription (les requêtes d'envoi ignorent déjà celles qui n'ont pas été revues depuis 30 jours).
UPDATE ent_push_device SET revoked_at = now(), revoque_motif = 'expire'
 WHERE session_id IS NULL AND revoked_at IS NULL AND last_seen_at < now() - interval '30 days';

-- DS2 : les sessions « mobile » ouvertes sans passkey (durée déclarée par le client) sont ramenées
-- à la durée standard du web.
UPDATE ent_session
   SET client = 'web',
       expires_at = LEAST(expires_at, now() + interval '12 hours'),
       inactivite_max_secs = COALESCE(inactivite_max_secs, 7200)
 WHERE client = 'mobile' AND assurance <> 'passkey' AND revoked_at IS NULL;
