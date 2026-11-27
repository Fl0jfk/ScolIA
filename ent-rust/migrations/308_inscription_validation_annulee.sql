-- Lot I2 (socle 3, INS-68, R-I11) — annulation d'une validation erronée.
--
-- Le dossier garde la trace de l'annulation (qui, quand, pourquoi) ; l'état revient à `confirme`
-- (admission, arrivée en cours d'année), `confirmee` (réinscription) ou `complete` (élèves
-- affectés). Rien n'est effacé : les liens créés par la validation sont suspendus par le socle 4,
-- la fiche créée par la validation est archivée par le socle 2, l'inscription annuelle prévue est
-- retirée par le socle 2. Le délai est un paramètre de la campagne (302).
ALTER TABLE ent_inscription_dossier
    ADD COLUMN IF NOT EXISTS validation_annulee_le TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS validation_annulee_par UUID,
    ADD COLUMN IF NOT EXISTS validation_annulee_motif TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_annulation_chk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_annulation_chk
            CHECK (validation_annulee_motif IS NULL OR char_length(validation_annulee_motif) <= 1000);
    END IF;
END $$;
