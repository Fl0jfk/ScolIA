-- Socle 11, lot MB0 (plage 2220–2239) — correctifs de sécurité mobile DS1, DS2, DS3, DS4.
-- Réf. : docs/socles/11_APPLICATIONS_MOBILES.md §9 (MB0), 11_MOBILE.md §27.1 (MO0).
-- Rejouable (une base qui a déjà appliqué une version antérieure de ce fichier est remise d'aplomb) ;
-- aucune migration versée n'est modifiée. Aucune colonne JSON (D-036).

-- DS1/DS3 : l'inscription du jeton de notification est liée à la session qui l'a faite ; l'empreinte
-- SHA-256 du jeton est le critère de recherche et d'unicité (aucune route ne renvoie le jeton).
-- Suppression d'une session : jamais d'effacement silencieux (SET NULL, et le déclencheur ci-dessous
-- révoque d'abord l'appareil avec le motif « expire »).
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS session_id UUID;
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS token_empreinte TEXT;
ALTER TABLE ent_push_device ADD COLUMN IF NOT EXISTS revoque_motif TEXT;
-- Historique d'envoi (règle 3, M4) : empreinte et référence à l'appareil, plus jamais le jeton.
ALTER TABLE ent_push_outbox ADD COLUMN IF NOT EXISTS token_empreinte TEXT;
ALTER TABLE ent_push_outbox ADD COLUMN IF NOT EXISTS device_id UUID;

-- M-A : `ADD COLUMN IF NOT EXISTS … REFERENCES` ne change pas une clé étrangère déjà posée par une
-- version antérieure de ce fichier (alors en CASCADE : la purge d'une session effaçait l'appareil et sa
-- trace). La clé est donc gérée à part : remplacée seulement si elle n'est pas déjà en SET NULL
-- (pas de verrou ni de revalidation à chaque rejeu).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_catalog.pg_constraint
                WHERE conname = 'ent_push_device_session_id_fkey'
                  AND conrelid = 'public.ent_push_device'::regclass AND confdeltype <> 'n') THEN
        ALTER TABLE public.ent_push_device DROP CONSTRAINT ent_push_device_session_id_fkey;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_constraint
                    WHERE conname = 'ent_push_device_session_id_fkey'
                      AND conrelid = 'public.ent_push_device'::regclass) THEN
        ALTER TABLE public.ent_push_device ADD CONSTRAINT ent_push_device_session_id_fkey
            FOREIGN KEY (session_id) REFERENCES public.ent_session (id) ON DELETE SET NULL;
    END IF;
END $$;

-- F-5 : la RLS est FORCÉE, y compris pour le propriétaire. Les mises à jour de données de ce fichier
-- sont donc faites établissement par établissement (contexte posé), pour ne rien laisser en route
-- si le lanceur de migrations tournait un jour avec `app.rls_strict` actif (0 ligne vue, 0 ligne
-- touchée, sans erreur). Bloc délimité par les deux repères ci-dessous (repris par le test
-- `tests_rls_stricte`) ; il n'utilise que des colonnes posées plus haut.
-- DEBUT-DONNEES
DO $$
DECLARE
    precedent TEXT := current_setting('app.etablissement_id', true);
    e UUID;
BEGIN
    FOR e IN SELECT id FROM public.ent_etablissement
             UNION SELECT etablissement_id FROM public.ent_membership LOOP
        PERFORM set_config('app.etablissement_id', e::text, true);
        UPDATE public.ent_push_device
           SET token_empreinte = encode(sha256(convert_to(token, 'UTF8')), 'hex')
         WHERE token_empreinte IS NULL;
        -- Les inscriptions antérieures au lot n'ont pas de session : révoquées à 30 jours sans
        -- réinscription (`ent_push_actif` ignore déjà celles qui n'ont pas été revues depuis 30 jours).
        UPDATE public.ent_push_device SET revoked_at = now(), revoque_motif = 'expire'
         WHERE session_id IS NULL AND revoked_at IS NULL AND last_seen_at < now() - interval '30 days';
        -- Règle 3 (M4) : le jeton en clair n'existe qu'à un seul endroit (la ligne d'inscription active).
        UPDATE public.ent_push_outbox
           SET token_empreinte = COALESCE(token_empreinte, encode(sha256(convert_to(token, 'UTF8')), 'hex')),
               token = NULL
         WHERE token IS NOT NULL;
    END LOOP;
    PERFORM set_config('app.etablissement_id', COALESCE(precedent, ''), true);
END $$;
-- FIN-DONNEES

-- L'empreinte est toujours cohérente avec le jeton (y compris pour les écritures qui ne la posent pas).
CREATE OR REPLACE FUNCTION ent_trg_push_device_empreinte() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $$
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
    revoque_motif IS NULL OR revoque_motif IN
        ('session_revoquee', 'reaffecte', 'desinscription', 'expire', 'restauration', 'acces_ferme')
);

-- Règle 1 (M-D) : un appareil ne reçoit rien qui ne soit adossé à une session ACCEPTÉE PAR L'API
-- (mêmes conditions que `resolve_session`, `auth_repo.rs`) ; sans session (labo, anciennes
-- inscriptions) : réinscription de moins de 30 jours ET accès ouvert. UNE seule définition de chaque
-- condition, appelée par tous les chemins d'envoi : un accès échu, fermé, suspendu ou une casquette
-- parent fermée ne reçoit plus rien, sans attendre la révocation de la session.
-- Fonctions « invoker » (aucun privilège supplémentaire) : leur `search_path` garde `public`, car les
-- fonctions d'accès appelées (`ent_acces_ouvert`, `ent_aujourdhui_etab`, `ent_agent_*`…) lisent leurs tables sans les qualifier.
DROP FUNCTION IF EXISTS ent_push_actif(TIMESTAMPTZ, UUID, TIMESTAMPTZ);

-- Les conditions d'accès (état ouvert, date de fin au jour de l'établissement, agent non suspendu) ne
-- sont PAS redéfinies ici : `ent_acces_ouvert` (migrations 350 et 352) est la définition unique, la même
-- que celle de `resolve_session`.

CREATE OR REPLACE FUNCTION ent_session_vivante(p_session_id UUID) RETURNS boolean
LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT EXISTS (
        SELECT 1 FROM public.ent_session s
         WHERE s.id = p_session_id AND s.revoked_at IS NULL AND s.expires_at > now()
           AND (s.inactivite_max_secs IS NULL
                OR s.last_seen_at > now() - make_interval(secs => s.inactivite_max_secs))
           AND s.portee = 'complete'
           AND public.ent_acces_ouvert(s.user_id, s.etablissement_id)
           -- Une session = une casquette : parent encore ouverte ; ailleurs, pas d'agent avant son arrivée
           -- (accès réduit côté API).
           AND CASE WHEN s.casquette = 'parent'
                    THEN public.ent_casquette_ouverte(s.user_id, s.etablissement_id, 'parent')
                    ELSE NOT public.ent_agent_avant_arrivee(s.user_id, s.etablissement_id) END)
$$;

CREATE OR REPLACE FUNCTION ent_push_actif(
    p_revoked_at TIMESTAMPTZ, p_session_id UUID, p_last_seen_at TIMESTAMPTZ,
    p_user_id UUID, p_etablissement_id UUID)
RETURNS boolean LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT p_revoked_at IS NULL
       AND CASE WHEN p_session_id IS NULL
                THEN p_last_seen_at > now() - interval '30 days'
                     AND public.ent_acces_ouvert(p_user_id, p_etablissement_id)
                ELSE public.ent_session_vivante(p_session_id) END
$$;

-- Les mises à jour de ent_push_device par les déclencheurs ci-dessous s'exécutent dans le contexte
-- d'établissement de la session concernée (RLS forcée). Règle 3 (M-C) : `search_path` réduit, objets
-- qualifiés, exécution retirée à PUBLIC (grants en fin de fichier).
-- DS1 : révoquer une session coupe, dans la même transaction, les notifications de son appareil,
-- quel que soit le chemin de révocation (déconnexion, appareil perdu, accès fermé, révocation globale).
CREATE OR REPLACE FUNCTION ent_trg_session_revoque_push() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    precedent TEXT := pg_catalog.current_setting('app.etablissement_id', true);
BEGIN
    IF NEW.revoked_at IS NOT NULL AND OLD.revoked_at IS NULL THEN
        PERFORM pg_catalog.set_config('app.etablissement_id', NEW.etablissement_id::text, true);
        UPDATE public.ent_push_device
           SET revoked_at = pg_catalog.now(), revoque_motif = 'session_revoquee'
         WHERE session_id = NEW.id AND revoked_at IS NULL;
        PERFORM pg_catalog.set_config('app.etablissement_id', COALESCE(precedent, ''), true);
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
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    precedent TEXT := pg_catalog.current_setting('app.etablissement_id', true);
BEGIN
    PERFORM pg_catalog.set_config('app.etablissement_id', OLD.etablissement_id::text, true);
    UPDATE public.ent_push_device
       SET revoked_at = pg_catalog.now(),
           revoque_motif = CASE WHEN OLD.revoked_at IS NULL THEN 'expire' ELSE 'session_revoquee' END
     WHERE session_id = OLD.id AND revoked_at IS NULL;
    PERFORM pg_catalog.set_config('app.etablissement_id', COALESCE(precedent, ''), true);
    RETURN OLD;
END $$;

DROP TRIGGER IF EXISTS ent_session_supprimee_push ON ent_session;
CREATE TRIGGER ent_session_supprimee_push
    BEFORE DELETE ON ent_session
    FOR EACH ROW EXECUTE FUNCTION ent_trg_session_supprimee_push();

-- Règle 2 (M2, M-B) : un jeton physique n'a qu'un titulaire. Révoque (motif « reaffecte ») toute
-- inscription active de cette empreinte détenue par une AUTRE personne, dans tous les établissements
-- (la RLS étant forcée, on parcourt les établissements). La trace de la révocation reste dans
-- l'établissement d'origine (la ligne est conservée, `revoked_at` + motif). Aucune information ne
-- traverse les établissements : la fonction ne rend QUE les anciens titulaires de l'établissement
-- appelant ; ceux des autres établissements sont révoqués sans que l'appelant en apprenne l'identité,
-- ni même l'existence. Verrou consultatif : deux inscriptions simultanées du même jeton sont
-- sérialisées jusqu'à la fin de la transaction. Le filtre `etablissement_id = e` est explicite (et non
-- laissé à la RLS) : un propriétaire superutilisateur contourne la RLS, et la boucle révoquerait alors tout
-- d'un coup en rendant une liste vide pour l'établissement appelant.
-- SECURITY DEFINER (propriétaire des tables) : seule porte d'écriture entre établissements ; le
-- `search_path` est réduit, tous les objets sont qualifiés, l'exécution est retirée à PUBLIC.
DROP FUNCTION IF EXISTS ent_push_reaffecter(TEXT, UUID);

CREATE OR REPLACE FUNCTION ent_push_reaffecter(p_empreinte TEXT, p_user_id UUID, p_etablissement_id UUID)
RETURNS SETOF UUID
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    precedent TEXT := pg_catalog.current_setting('app.etablissement_id', true);
    e UUID;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('ent_push:' || p_empreinte, 0));
    FOR e IN SELECT id FROM public.ent_etablissement
             UNION SELECT etablissement_id FROM public.ent_membership
             UNION SELECT p_etablissement_id LOOP
        PERFORM pg_catalog.set_config('app.etablissement_id', e::text, true);
        IF e = p_etablissement_id THEN
            RETURN QUERY
                WITH u AS (
                    UPDATE public.ent_push_device
                       SET revoked_at = pg_catalog.now(), revoque_motif = 'reaffecte'
                     WHERE etablissement_id = e
                       AND token_empreinte = p_empreinte AND revoked_at IS NULL AND user_id <> p_user_id
                    RETURNING user_id)
                SELECT user_id FROM u;
        ELSE
            UPDATE public.ent_push_device
               SET revoked_at = pg_catalog.now(), revoque_motif = 'reaffecte'
             WHERE etablissement_id = e
               AND token_empreinte = p_empreinte AND revoked_at IS NULL AND user_id <> p_user_id;
        END IF;
    END LOOP;
    PERFORM pg_catalog.set_config('app.etablissement_id', COALESCE(precedent, ''), true);
    RETURN;
END $$;

-- Règle 3 (M4) : le jeton en clair n'existe qu'à un seul endroit (la ligne d'inscription active).
-- L'historique d'envoi ne garde que l'empreinte et la référence à l'appareil ; les lignes déjà
-- présentes sont neutralisées.
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

-- Règle 3 (M-C) : exécution retirée à PUBLIC pour les fonctions du lot qui écrivent ou changent de
-- contexte ; accordée au seul rôle applicatif (`ent_app`, docs/RLS_ISOLATION.md) quand il existe. Le
-- propriétaire (rôle de migration) garde l'exécution. Les fonctions de déclencheur n'ont pas besoin
-- d'être appelables : un déclencheur ne contrôle l'exécution qu'à sa création.
REVOKE ALL ON FUNCTION ent_push_reaffecter(TEXT, UUID, UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION ent_trg_push_device_empreinte() FROM PUBLIC;
REVOKE ALL ON FUNCTION ent_trg_session_revoque_push() FROM PUBLIC;
REVOKE ALL ON FUNCTION ent_trg_session_supprimee_push() FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ent_app') THEN
        GRANT EXECUTE ON FUNCTION public.ent_push_reaffecter(TEXT, UUID, UUID) TO ent_app;
    END IF;
END $$;
