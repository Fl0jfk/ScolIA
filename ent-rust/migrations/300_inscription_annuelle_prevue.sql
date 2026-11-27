-- Lot I1 (socle 3, `docs/socles/03_INSCRIPTIONS.md` §16 M2) — côté socle 2.
--
-- La validation d'un dossier ne crée plus d'affectation en classe : elle enregistre une
-- **inscription annuelle prévue** (année, niveau, parcours, régime) par la commande
-- `ent-db::eleve_ports::enregistrer_inscription_annuelle_tx`. Sur le schéma actuel (avant le
-- lot E4 du socle 2 : `ent_eleve_inscription`), la « prévue » est une ligne `projetee` de
-- `ent_eleve_scolarite_annee` ; la commande ne changera pas de contrat quand E4 arrivera.
--
-- Additif : deux colonnes facultatives, une clé d'identification composée, deux valeurs de
-- `source` admises. Rien n'est supprimé ni réécrit.

ALTER TABLE ent_eleve_scolarite_annee
    ADD COLUMN IF NOT EXISTS parcours TEXT,
    ADD COLUMN IF NOT EXISTS source_ref UUID;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_scolarite_parcours_chk') THEN
        ALTER TABLE ent_eleve_scolarite_annee
            ADD CONSTRAINT ent_eleve_scolarite_parcours_chk
            CHECK (parcours IS NULL OR char_length(parcours) <= 120);
    END IF;
    -- Origines admises : `admission` (validation d'un dossier d'admission) et `reinscription`
    -- (validation d'une réinscription du socle 3), en plus de celles déjà reconnues.
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_scolarite_source_chk') THEN
        ALTER TABLE ent_eleve_scolarite_annee DROP CONSTRAINT ent_eleve_scolarite_source_chk;
    END IF;
    ALTER TABLE ent_eleve_scolarite_annee
        ADD CONSTRAINT ent_eleve_scolarite_source_chk
        CHECK (source IN ('import', 'saisie', 'montee_rapide', 'reinscription_parent',
                          'sync_courante', 'admission', 'reinscription'));
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_scolarite_annee_etab_id_uq') THEN
        ALTER TABLE ent_eleve_scolarite_annee
            ADD CONSTRAINT ent_eleve_scolarite_annee_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END $$;
