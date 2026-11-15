-- Socle 28, lot MS0 (MSE-D02) : annulation réelle d'un lot d'import prévisualisé.
-- L'état `annule` existait dans le CHECK de `ent_import_lot` mais n'était jamais posé.
-- On ajoute qui a annulé et quand ; un lot annulé porte obligatoirement les deux.
-- Migration additive et rejouable. Aucun JSON, aucune date relative.

ALTER TABLE ent_import_lot ADD COLUMN IF NOT EXISTS annule_par UUID REFERENCES ent_user (id) ON DELETE SET NULL;
ALTER TABLE ent_import_lot ADD COLUMN IF NOT EXISTS annule_le TIMESTAMPTZ;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ent_import_lot_annule_coherent'
    ) THEN
        ALTER TABLE ent_import_lot ADD CONSTRAINT ent_import_lot_annule_coherent
            CHECK (statut <> 'annule' OR annule_le IS NOT NULL);
    END IF;
END $$;
