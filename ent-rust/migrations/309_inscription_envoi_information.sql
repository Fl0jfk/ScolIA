-- Lot I2 (socle 3) — la file d'envoi (250) sert aussi aux informations aux familles (prolongation
-- d'échéance, fermeture du secrétariat) et peut annuler un envoi pas encore parti (marque « à ne
-- pas solliciter » posée après coup, INS-57 : aucun message, y compris déjà programmé).
ALTER TABLE ent_inscription_envoi DROP CONSTRAINT IF EXISTS ent_inscription_envoi_kind_check;
ALTER TABLE ent_inscription_envoi DROP CONSTRAINT IF EXISTS ent_inscription_envoi_kind_i2_chk;
ALTER TABLE ent_inscription_envoi
    ADD CONSTRAINT ent_inscription_envoi_kind_i2_chk
    CHECK (kind IN ('invitation', 'accueil', 'information'));
ALTER TABLE ent_inscription_envoi DROP CONSTRAINT IF EXISTS ent_inscription_envoi_statut_check;
ALTER TABLE ent_inscription_envoi DROP CONSTRAINT IF EXISTS ent_inscription_envoi_statut_i2_chk;
ALTER TABLE ent_inscription_envoi
    ADD CONSTRAINT ent_inscription_envoi_statut_i2_chk
    CHECK (statut IN ('a_envoyer', 'envoye', 'echec', 'annule'));
