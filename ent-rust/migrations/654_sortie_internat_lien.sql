-- 654 — Socle 42, lot V3 « argent relié » (2/2) : lien entre un séjour et l'internat (socle 41). Un interne qui part en
-- voyage ne dort pas à l'internat : l'attendu de la nuit (I2) doit le savoir sans que personne ne le ressaisisse, et
-- l'inverse quand le participant se désiste. Réf. : 42_SORTIES_VOYAGES.md §6.2, §6.4 ; 41_INTERNAT.md (adaptateur
-- `ent_internat_retrait`, motif « sejour_scolaire »).
--
-- Additive : 2824 (versée) n'est pas modifiée. On ajoute seulement la source « voyage » à la liste des sources
-- autorisées d'un retrait, par un remplacement de la contrainte (même nom, liste élargie) ; rien d'autre ne change
-- dans les tables du socle 41. La trace du retrait posé pour un participant vit dans le socle 42 (ajout seul).

DO $$
DECLARE
    c TEXT;
BEGIN
    FOR c IN SELECT conname FROM pg_constraint
              WHERE conrelid = 'ent_internat_retrait'::regclass AND contype = 'c'
                AND pg_get_constraintdef(oid) LIKE '%absence_internat%' LOOP
        EXECUTE format('ALTER TABLE ent_internat_retrait DROP CONSTRAINT %I', c);
    END LOOP;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_retrait_source_v3_chk') THEN
        ALTER TABLE ent_internat_retrait ADD CONSTRAINT ent_internat_retrait_source_v3_chk CHECK (source IN (
            'cpe', 'direction', 'vie_scolaire', 'infirmerie', 'absence_internat', 'reprise', 'voyage'));
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS ent_sortie_internat_retrait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    retrait_id UUID NOT NULL,
    du_nuit DATE NOT NULL,
    au_nuit DATE NOT NULL,
    cree_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_internat_retrait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_internat_retrait_periode_chk CHECK (au_nuit >= du_nuit),
    CONSTRAINT ent_sortie_internat_retrait_retrait_uq UNIQUE (etablissement_id, retrait_id),
    CONSTRAINT ent_sortie_internat_retrait_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_internat_retrait_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_internat_retrait_retrait_fk FOREIGN KEY (etablissement_id, retrait_id)
        REFERENCES ent_internat_retrait (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_internat_retrait_part_idx
    ON ent_sortie_internat_retrait (etablissement_id, sortie_id, participant_id);

DROP TRIGGER IF EXISTS ent_sortie_internat_retrait_immuable ON ent_sortie_internat_retrait;
CREATE TRIGGER ent_sortie_internat_retrait_immuable BEFORE UPDATE OR DELETE ON ent_sortie_internat_retrait
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_internat_retrait_vidage ON ent_sortie_internat_retrait;
CREATE TRIGGER ent_sortie_internat_retrait_vidage BEFORE TRUNCATE ON ent_sortie_internat_retrait
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_sortie_internat_retrait']);
