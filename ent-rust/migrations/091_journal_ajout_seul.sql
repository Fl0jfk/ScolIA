-- Journal des modifications / consultations : **ajout seul**.
-- Un UPDATE ou un DELETE sur ent_journal_modif est refusé, y compris pour le
-- rôle applicatif : une trace d'audit modifiable n'est pas une preuve.
-- (PostgreSQL >= 14 requis pour CREATE OR REPLACE TRIGGER ; la CI utilise postgres:18.)
-- (Une purge de rétention légitime devra passer par une procédure dédiée,
-- décidée et auditée, jamais par l'application courante.)

CREATE OR REPLACE FUNCTION ent_journal_modif_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'ent_journal_modif est en ajout seul (% interdit)', TG_OP
        USING ERRCODE = '42501';
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER ent_journal_modif_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_journal_modif
    FOR EACH ROW EXECUTE FUNCTION ent_journal_modif_ajout_seul();
