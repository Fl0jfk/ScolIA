-- Messagerie, correctifs de la revue des suites M12 (`docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md`).
-- Plage 480–499 : 491 est prise ici ; 482–490 sont versées et ne changent pas. Rejouable.
--
-- E1 : corriger puis annuler un message différé échouait. `ent_msg_version` (482) a une clé étrangère vers `ent_msg`
-- sans `ON DELETE` et un déclencheur d'ajout seul : la suppression du message violait la clé (23503), la transaction
-- d'annulation était défaite et le message restait planifié (avec le texte corrigé). L'exception bornée de 490
-- (`ent.msg_annulation`) est étendue à `ent_msg_version` : une version n'est supprimable que dans la transaction qui
-- annule **ce** message, jamais remis (`remise_prevue_le > now()`). Tout autre UPDATE ou DELETE reste refusé.
--
-- M1 : curseur du flux M12 fondé sur les identifiants de transaction (`xid8`), plus sur `pg_stat_activity` ni sur
-- l'horloge murale. Chaque message porte l'identifiant de la transaction qui l'a inséré ; le retrait, celui de la
-- transaction qui l'a posé (déclencheur). Borne du curseur : `pg_snapshot_xmin(pg_current_snapshot())` : toute
-- transaction d'identifiant inférieur est terminée, donc aucun message ne peut plus apparaître sous la borne.

CREATE OR REPLACE FUNCTION ent_msg_ajout_seul_sauf_annulation() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_cible TEXT := nullif(current_setting('ent.msg_annulation', true), '');
    v_msg UUID;
BEGIN
    IF TG_OP = 'DELETE' AND v_cible IS NOT NULL THEN
        IF TG_TABLE_NAME IN ('ent_msg_carte_document', 'ent_msg_version') THEN
            v_msg := OLD.message_id;
        ELSIF TG_TABLE_NAME = 'ent_msg_piece_acces' THEN
            SELECT p.message_id INTO v_msg FROM ent_msg_piece p
             WHERE p.etablissement_id = OLD.etablissement_id AND p.id = OLD.piece_id;
        END IF;
        IF v_msg IS NOT NULL AND v_msg::text = v_cible
           AND EXISTS (SELECT 1 FROM ent_msg m
                        WHERE m.etablissement_id = OLD.etablissement_id AND m.id = v_msg
                          AND m.remise_prevue_le > now()) THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION '% : ajout seul (ni modification ni suppression)', TG_TABLE_NAME
        USING ERRCODE = 'check_violation';
END
$$;

DROP TRIGGER IF EXISTS ent_msg_version_ajout_seul ON ent_msg_version;
DROP TRIGGER IF EXISTS ent_msg_version_ajout_seul_trg ON ent_msg_version;
CREATE TRIGGER ent_msg_version_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_msg_version
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul_sauf_annulation();

-- M1 : identifiants de transaction. Les lignes existantes reçoivent celui de cette migration (volatile : la table est
-- réécrite une fois) : tous inférieurs à la borne de tout appel postérieur, donc relus normalement par la reprise.
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS msg_xid xid8 NOT NULL DEFAULT pg_current_xact_id();
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS retire_xid xid8;

CREATE OR REPLACE FUNCTION ent_msg_retire_xid() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.retire_le IS NOT NULL AND OLD.retire_le IS NULL THEN
        NEW.retire_xid := pg_current_xact_id();
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_msg_retire_xid_trg ON ent_msg;
CREATE TRIGGER ent_msg_retire_xid_trg BEFORE UPDATE OF retire_le ON ent_msg
    FOR EACH ROW EXECUTE FUNCTION ent_msg_retire_xid();

-- Retraits antérieurs : l'identifiant de la migration (même raison que ci-dessus).
UPDATE ent_msg SET retire_xid = pg_current_xact_id() WHERE retire_le IS NOT NULL AND retire_xid IS NULL;

CREATE INDEX IF NOT EXISTS ent_msg_xid_idx ON ent_msg (etablissement_id, msg_xid);
CREATE INDEX IF NOT EXISTS ent_msg_retire_xid_idx ON ent_msg (etablissement_id, retire_xid) WHERE retire_xid IS NOT NULL;
