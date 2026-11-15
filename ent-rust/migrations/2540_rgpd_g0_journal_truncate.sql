-- Socle 27, lot G0 (RG-D02) : le journal des modifications / consultations est en
-- ajout seul. 091 refuse UPDATE et DELETE ligne par ligne ; TRUNCATE n'appelle pas
-- les déclencheurs de ligne. Un déclencheur de niveau instruction le refuse aussi.
-- Aucune autre modification de schéma (les traces bloquantes et le registre sont du code).

CREATE OR REPLACE FUNCTION ent_journal_modif_sans_truncate() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'ent_journal_modif est en ajout seul (TRUNCATE interdit)'
        USING ERRCODE = '42501';
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER ent_journal_modif_sans_truncate_trg
    BEFORE TRUNCATE ON ent_journal_modif
    FOR EACH STATEMENT EXECUTE FUNCTION ent_journal_modif_sans_truncate();
