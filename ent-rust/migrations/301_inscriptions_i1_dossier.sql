-- Lot I1 (socle 3, §16 M1 et M2) — côté socle 3 (dossier) et socle 4 (liens en attente).
--
-- * Le dossier garde la **trace** de l'inscription annuelle prévue qu'a créée sa validation
--   (`inscription_annuelle_id`) et du parcours choisi (`parcours_demande`, texte affiché).
-- * L'origine `import` est admise pour `ent_lien_attente` : l'import passe désormais par la
--   commande `integrer_declaration` du socle 4 (il ne rapproche jamais de lui-même ; un lien en
--   attente ne peut donc venir que d'un choix humain, mais la contrainte reste cohérente).

ALTER TABLE ent_inscription_dossier
    ADD COLUMN IF NOT EXISTS parcours_demande TEXT,
    ADD COLUMN IF NOT EXISTS inscription_annuelle_id UUID;

DO $$
DECLARE
    c TEXT;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_parcours_chk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_parcours_chk
            CHECK (parcours_demande IS NULL OR char_length(parcours_demande) <= 120);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_insc_annuelle_fk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_insc_annuelle_fk
            FOREIGN KEY (etablissement_id, inscription_annuelle_id)
            REFERENCES ent_eleve_scolarite_annee (etablissement_id, id);
    END IF;
    -- Élargit le contrôle d'origine de `ent_lien_attente` sans présumer de son nom.
    FOR c IN SELECT conname FROM pg_constraint
              WHERE conrelid = 'ent_lien_attente'::regclass AND contype = 'c'
                AND pg_get_constraintdef(oid) ILIKE '%source%inscription%saisie%'
    LOOP
        EXECUTE format('ALTER TABLE ent_lien_attente DROP CONSTRAINT %I', c);
    END LOOP;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_lien_attente_source_chk') THEN
        ALTER TABLE ent_lien_attente
            ADD CONSTRAINT ent_lien_attente_source_chk
            CHECK (source IN ('inscription', 'saisie', 'import'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS ent_inscription_dossier_insc_annuelle_idx
    ON ent_inscription_dossier (etablissement_id, inscription_annuelle_id)
    WHERE inscription_annuelle_id IS NOT NULL;
