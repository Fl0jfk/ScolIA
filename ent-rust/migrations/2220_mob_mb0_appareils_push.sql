-- Socle 11, lot MB0 (plage 2220–2239) — correctifs de sécurité mobile DS1, DS2, DS3, DS4.
-- Réf. : docs/socles/11_APPLICATIONS_MOBILES.md §9 (MB0), 11_MOBILE.md §27.1 (MO0).
-- Additive et rejouable ; aucune migration versée n'est modifiée. Aucune colonne JSON (D-036).

-- DS1/DS3 : l'inscription du jeton de notification est liée à la session qui l'a faite ; l'empreinte
-- SHA-256 du jeton est le critère de recherche et d'unicité (aucune route ne renvoie le jeton).
-- Suppression d'une session : jamais d'effacement silencieux (SET NULL, et le déclencheur ci-dessous
-- révoque d'abord l'appareil avec le motif « expire »).
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS session_id UUID REFERENCES ent_session (id) ON DELETE SET NULL;
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS token_empreinte TEXT;
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS revoque_motif TEXT;

UPDATE ent_push_device
   SET token_empreinte = encode(sha256(convert_to(token, 'UTF8')), 'hex')
 WHERE token_empreinte IS NULL;

-- L'empreinte est toujours cohérente avec le jeton (y compris pour les écritures qui ne la posent pas).
CREATE OR REPLACE FUNCTION ent_trg_push_device_empreinte() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.token_empreinte := encode(sha256(convert_to(NEW.token, 'UTF8')), 'hex');
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_push_device_empreinte ON ent_push_device;
CREATE TRIGGER ent_push_device_empreinte
    BEFORE INSERT OR UPDATE OF token ON ent_push_device
    FOR EACH ROW EXECUTE FUNCTION ent_trg_push_device_empreinte();

-- DS3 / F4 : l'unicité porte sur l'empreinte des inscriptions actives (par établissement) ; l'index
-- sur le jeton en clair disparaît.
ALTER TABLE ent_push_device DROP CONSTRAINT IF EXISTS ent_push_device_token_uq;
DROP INDEX IF EXISTS ent_push_device_token_actif_uq;
CREATE UNIQUE INDEX IF NOT EXISTS ent_push_device_empreinte_actif_uq
    ON ent_push_device (etablissement_id, token_empreinte) WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS ent_push_device_empreinte_idx
    ON ent_push_device (token_empreinte) WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS ent_push_device_session_idx
    ON ent_push_device (session_id) WHERE session_id IS NOT NULL AND revoked_at IS NULL;

ALTER TABLE ent_push_device DROP CONSTRAINT IF EXISTS ent_push_device_revoque_motif_chk;
ALTER TABLE ent_push_device ADD CONSTRAINT ent_push_device_revoque_motif_chk CHECK (
    revoque_motif IS NULL OR revoque_motif IN ('session_revoquee', 'reaffecte', 'desinscription', 'expire')
);

-- Règle 1 : un appareil ne reçoit rien qui ne soit adossé à une session vivante (non révoquée, non
-- expirée, inactivité respectée) ou, sans session, à une réinscription de moins de 30 jours.
-- UNE seule définition, appelée par tous les chemins d'envoi.
CREATE OR REPLACE FUNCTION ent_session_vivante(p_session_id UUID) RETURNS boolean
LANGUAGE sql STABLE SET search_path = pg_catalog, public AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_session s
         WHERE s.id = p_session_id AND s.revoked_at IS NULL AND s.expires_at > now()
           AND (s.inactivite_max_secs IS NULL
                OR s.last_seen_at > now() - make_interval(secs => s.inactivite_max_secs)))
$$;

CREATE OR REPLACE FUNCTION ent_push_actif(p_revoked_at TIMESTAMPTZ, p_session_id UUID, p_last_seen_at TIMESTAMPTZ)
RETURNS boolean LANGUAGE sql STABLE SET search_path = pg_catalog, public AS $$
    SELECT p_revoked_at IS NULL
       AND CASE WHEN p_session_id IS NULL THEN p_last_seen_at > now() - interval '30 days'
                ELSE ent_session_vivante(p_session_id) END
$$;

-- Les mises à jour de ent_push_device par les déclencheurs ci-dessous s'exécutent dans le contexte
-- d'établissement de la session concernée (RLS forcée).
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

-- E1 : supprimer une session (purge des expirées, suppression d'un compte) révoque d'abord ses
-- appareils (motif « expire », trace gardée) ; la clé étrangère met ensuite session_id à NULL.
CREATE OR REPLACE FUNCTION ent_trg_session_supprimee_push() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    precedent TEXT := current_setting('app.etablissement_id', true);
BEGIN
    PERFORM set_config('app.etablissement_id', OLD.etablissement_id::text, true);
    UPDATE ent_push_device
       SET revoked_at = now(), revoque_motif = CASE WHEN OLD.revoked_at IS NULL THEN 'expire' ELSE 'session_revoquee' END
     WHERE session_id = OLD.id AND revoked_at IS NULL;
    PERFORM set_config('app.etablissement_id', COALESCE(precedent, ''), true);
    RETURN OLD;
END $$;

DROP TRIGGER IF EXISTS ent_session_supprimee_push ON ent_session;
CREATE TRIGGER ent_session_supprimee_push
    BEFORE DELETE ON ent_session
    FOR EACH ROW EXECUTE FUNCTION ent_trg_session_supprimee_push();

-- Règle 2 (M2) : un jeton physique n'a qu'un titulaire. Révoque (motif « reaffecte », trace) toute
-- inscription active de cette empreinte détenue par une AUTRE personne, dans tous les établissements
-- (la RLS étant forcée, on parcourt les établissements). Verrou consultatif : deux inscriptions
-- simultanées du même jeton sont sérialisées jusqu'à la fin de la transaction. Rend les anciens titulaires.
CREATE OR REPLACE FUNCTION ent_push_reaffecter(p_empreinte TEXT, p_user_id UUID) RETURNS SETOF UUID
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    precedent TEXT := current_setting('app.etablissement_id', true);
    e UUID;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('ent_push:' || p_empreinte, 0));
    FOR e IN SELECT id FROM ent_etablissement UNION SELECT etablissement_id FROM ent_session LOOP
        PERFORM set_config('app.etablissement_id', e::text, true);
        RETURN QUERY
            WITH u AS (
                UPDATE ent_push_device SET revoked_at = now(), revoque_motif = 'reaffecte'
                 WHERE token_empreinte = p_empreinte AND revoked_at IS NULL AND user_id <> p_user_id
                RETURNING user_id)
            SELECT user_id FROM u;
    END LOOP;
    PERFORM set_config('app.etablissement_id', COALESCE(precedent, ''), true);
    RETURN;
END $$;

-- Les inscriptions antérieures au lot n'ont pas de session : elles sont révoquées à 30 jours sans
-- réinscription (ent_push_actif ignore déjà celles qui n'ont pas été revues depuis 30 jours).
UPDATE ent_push_device SET revoked_at = now(), revoque_motif = 'expire'
 WHERE session_id IS NULL AND revoked_at IS NULL AND last_seen_at < now() - interval '30 days';

-- Règle 3 (M4) : le jeton en clair n'existe qu'à un seul endroit (la ligne d'inscription active).
-- L'historique d'envoi ne garde que l'empreinte et la référence à l'appareil ; les lignes déjà
-- présentes sont neutralisées.
ALTER TABLE ent_push_outbox ADD COLUMN IF NOT EXISTS token_empreinte TEXT;
ALTER TABLE ent_push_outbox ADD COLUMN IF NOT EXISTS device_id UUID;
UPDATE ent_push_outbox
   SET token_empreinte = COALESCE(token_empreinte, encode(sha256(convert_to(token, 'UTF8')), 'hex')),
       token = NULL
 WHERE token IS NOT NULL;
ALTER TABLE ent_push_outbox DROP CONSTRAINT IF EXISTS ent_push_outbox_sans_jeton_chk;
ALTER TABLE ent_push_outbox ADD CONSTRAINT ent_push_outbox_sans_jeton_chk CHECK (token IS NULL);

-- DS2 / F3 : les sessions « mobile » ouvertes sans passkey sont ramenées à la durée standard du web,
-- plafond compté depuis la création ; une session déjà au-delà du plafond est close (les appareils
-- liés sont révoqués par le déclencheur).
UPDATE ent_session
   SET client = 'web',
       expires_at = LEAST(expires_at, created_at + interval '12 hours'),
       inactivite_max_secs = COALESCE(inactivite_max_secs, 7200),
       revoked_at = CASE WHEN created_at + interval '12 hours' <= now() THEN now() ELSE revoked_at END
 WHERE client = 'mobile' AND assurance <> 'passkey' AND revoked_at IS NULL;
