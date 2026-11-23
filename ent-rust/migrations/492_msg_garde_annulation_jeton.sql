-- Messagerie, passe 2 de la contre-revue E1 (`docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md`, « Passe 2 »).
-- Plage 482–499 du socle 16 : 492 est prise ici ; 482–491 sont versées et ne changent pas (règle 25). Rejouable.
--
-- M-3 : la garde d'annulation de 490 et 491 (`ent_msg_ajout_seul_sauf_annulation`) reposait sur un réglage libre
-- (`ent.msg_annulation`) que toute session du rôle applicatif peut poser, et sur des tables non qualifiées cherchées
-- d'abord dans `pg_temp` : une table temporaire `ent_msg` homonyme pouvait satisfaire le `EXISTS`. Ici :
--   1. la fonction est recréée avec `SET search_path = pg_catalog, public, pg_temp` et des tables **qualifiées** ;
--   2. le réglage libre est remplacé par un **jeton lié à la transaction** : `ent_msg_annulation_ouvrir` inscrit
--      `pg_current_xact_id()` et l'identifiant du message dans `ent_msg_annulation_garde` (seulement pour un message
--      de l'établissement courant, dont la remise est à venir) ; la garde vérifie les deux, plus `remise_prevue_le`
--      dans le futur. Un jeton ne vaut que dans sa transaction : l'identifiant ne se rejoue pas.
-- La fonction du déclencheur `ent_msg_retire_xid` (491) reçoit le même `search_path`.

CREATE TABLE IF NOT EXISTS ent_msg_annulation_garde (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    message_id UUID NOT NULL,
    xid xid8 NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (xid, message_id)
);
CREATE INDEX IF NOT EXISTS ent_msg_annulation_garde_etab_idx
    ON ent_msg_annulation_garde (etablissement_id, message_id);

DO $$
BEGIN
    ALTER TABLE ent_msg_annulation_garde ENABLE ROW LEVEL SECURITY;
    ALTER TABLE ent_msg_annulation_garde FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS ent_rls_etablissement ON ent_msg_annulation_garde;
    CREATE POLICY ent_rls_etablissement ON ent_msg_annulation_garde FOR ALL
        USING (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
        WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
END
$$;

CREATE OR REPLACE FUNCTION ent_msg_annulation_ouvrir(p_etab UUID, p_msg UUID) RETURNS void
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    -- Jetons des transactions terminées (toute transaction d'identifiant inférieur à `xmin` l'est) : plus rien ne les
    -- lit ; la purge borne la taille de la table sans tâche de fond.
    DELETE FROM public.ent_msg_annulation_garde g
     WHERE g.xid < pg_catalog.pg_snapshot_xmin(pg_catalog.pg_current_snapshot());
    -- Le jeton ne se demande que pour l'établissement du contexte courant (aucun jeton « ailleurs », même sans RLS).
    IF public.ent_rls_etablissement() IS NOT NULL AND public.ent_rls_etablissement() <> p_etab THEN
        RAISE EXCEPTION 'ent_msg : annulation refusée (autre établissement)'
            USING ERRCODE = 'check_violation';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM public.ent_msg m
                    WHERE m.etablissement_id = p_etab AND m.id = p_msg
                      AND m.remise_prevue_le > pg_catalog.now()) THEN
        RAISE EXCEPTION 'ent_msg : annulation refusée (message remis ou inconnu)'
            USING ERRCODE = 'check_violation';
    END IF;
    INSERT INTO public.ent_msg_annulation_garde (etablissement_id, message_id, xid)
    VALUES (p_etab, p_msg, pg_catalog.pg_current_xact_id())
    ON CONFLICT DO NOTHING;
END
$$;

CREATE OR REPLACE FUNCTION ent_msg_ajout_seul_sauf_annulation() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    v_msg UUID;
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF TG_TABLE_NAME IN ('ent_msg_carte_document', 'ent_msg_version') THEN
            v_msg := OLD.message_id;
        ELSIF TG_TABLE_NAME = 'ent_msg_piece_acces' THEN
            SELECT p.message_id INTO v_msg FROM public.ent_msg_piece p
             WHERE p.etablissement_id = OLD.etablissement_id AND p.id = OLD.piece_id;
        END IF;
        IF v_msg IS NOT NULL
           AND EXISTS (SELECT 1 FROM public.ent_msg_annulation_garde g
                        WHERE g.etablissement_id = OLD.etablissement_id AND g.message_id = v_msg
                          AND g.xid = pg_catalog.pg_current_xact_id())
           AND EXISTS (SELECT 1 FROM public.ent_msg m
                        WHERE m.etablissement_id = OLD.etablissement_id AND m.id = v_msg
                          AND m.remise_prevue_le > pg_catalog.now()) THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION '% : ajout seul (ni modification ni suppression)', TG_TABLE_NAME
        USING ERRCODE = 'check_violation';
END
$$;

CREATE OR REPLACE FUNCTION ent_msg_retire_xid() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF NEW.retire_le IS NOT NULL AND OLD.retire_le IS NULL THEN
        NEW.retire_xid := pg_catalog.pg_current_xact_id();
    END IF;
    RETURN NEW;
END
$$;
