-- RS3-04 : une exception d'EDT ne peut référencer qu'une absence RH du même
-- établissement (clé étrangère composite, en plus du contrôle applicatif).
-- NOT VALID : les lignes existantes ne sont pas revérifiées (aucune migration
-- ne doit échouer sur des données historiques) ; toute nouvelle ligne l'est.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absence_rh_etab_id_uq
    ON ent_absence_rh (etablissement_id, id);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_exception_absence_etab_fk'
    ) THEN
        ALTER TABLE ent_edt_exception
            ADD CONSTRAINT ent_edt_exception_absence_etab_fk
            FOREIGN KEY (etablissement_id, absence_rh_id)
            REFERENCES ent_absence_rh (etablissement_id, id) NOT VALID;
    END IF;
END $$;
