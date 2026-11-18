-- Messagerie, correctifs de la revue de sécurité du lot M5-M6 (`docs/socles/16_MESSAGERIE_M5_M6_LIVRAISON.md`).
-- Plage 480–499 : 490 est prise ici (488 et 489, non versées, ne sont pas modifiées). Rejouable.
--
-- M4 (R-M48) : l'annulation d'un message différé échouait en 500, parce que les cartes « document » et le journal des
-- accès aux pièces sont en ajout seul (déclencheur de 488) et que les lectures pointent le message. Une exception
-- **explicite et bornée** : une suppression n'est permise que dans la transaction qui annule le message, désigné par le
-- réglage local `ent.msg_annulation`, et seulement si ce message n'a **jamais été remis** (`remise_prevue_le > now()`).
-- Tout autre UPDATE ou DELETE reste refusé.

CREATE OR REPLACE FUNCTION ent_msg_ajout_seul_sauf_annulation() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_cible TEXT := nullif(current_setting('ent.msg_annulation', true), '');
    v_msg UUID;
BEGIN
    IF TG_OP = 'DELETE' AND v_cible IS NOT NULL THEN
        IF TG_TABLE_NAME = 'ent_msg_carte_document' THEN
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

DROP TRIGGER IF EXISTS ent_msg_piece_acces_ajout_seul_trg ON ent_msg_piece_acces;
CREATE TRIGGER ent_msg_piece_acces_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_msg_piece_acces
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul_sauf_annulation();
DROP TRIGGER IF EXISTS ent_msg_carte_document_ajout_seul_trg ON ent_msg_carte_document;
CREATE TRIGGER ent_msg_carte_document_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_msg_carte_document
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul_sauf_annulation();

-- M8 : l'**ouverture** d'un lien pré-signé (GET /fichiers/{jeton}) est journalisée à côté de l'émission.
ALTER TABLE ent_msg_piece_acces DROP CONSTRAINT IF EXISTS ent_msg_piece_acces_via_check;
ALTER TABLE ent_msg_piece_acces DROP CONSTRAINT IF EXISTS ent_msg_piece_acces_via_chk;
DO $$
DECLARE c text;
BEGIN
    FOR c IN SELECT conname FROM pg_constraint
              WHERE conrelid = 'ent_msg_piece_acces'::regclass AND contype = 'c'
                AND pg_get_constraintdef(oid) LIKE '%via%' LOOP
        EXECUTE format('ALTER TABLE ent_msg_piece_acces DROP CONSTRAINT %I', c);
    END LOOP;
END
$$;
ALTER TABLE ent_msg_piece_acces ADD CONSTRAINT ent_msg_piece_acces_via_chk
    CHECK (via IN ('lien', 'ouverture', 'telechargement'));
