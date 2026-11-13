-- Socle 20, lot C1 — migration 2411 : archive annuelle d'un cahier de cours (document figé, empreinte stable).
-- Réf. : 20_CAHIER_DE_TEXTES.md §14.3, §24.3, R-C19, R-C28. La production de l'archive (PDF/A, socle 7) est le lot C8.
-- Conservation : lisible comme cahier actif jusqu'à la fin de l'année suivante, puis archive de cinq ans (R-06, à valider
-- par le DPO, Q-CDT3) ; aucune destruction sans la décision du chef d'établissement (`decision`, `decide_par_user_id`).

CREATE TABLE IF NOT EXISTS ent_cdt_archive (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    cours_id UUID NOT NULL,
    objet_id UUID NOT NULL,
    empreinte CHAR(64) NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    produite_le TIMESTAMPTZ NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    echeance_archive DATE NOT NULL,
    decision TEXT NOT NULL DEFAULT 'a_decider' CHECK (decision IN ('a_decider', 'versee', 'detruite')),
    decide_par_user_id UUID,
    decide_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_archive_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_archive_version_uq UNIQUE (etablissement_id, annee_id, cours_id, version),
    CONSTRAINT ent_cdt_archive_decision_chk CHECK ((decision = 'a_decider') = (decide_le IS NULL)),
    CONSTRAINT ent_cdt_archive_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_archive_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_archive_objet_fk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_objet (etablissement_id, id) ON DELETE RESTRICT
);
-- Une archive ne se réécrit pas : seules les colonnes de décision changent.
CREATE OR REPLACE FUNCTION ent_cdt_archive_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_cdt_archive : suppression interdite (decision)' USING ERRCODE = '23514';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id OR NEW.annee_id IS DISTINCT FROM OLD.annee_id
       OR NEW.cours_id IS DISTINCT FROM OLD.cours_id OR NEW.objet_id IS DISTINCT FROM OLD.objet_id
       OR NEW.empreinte IS DISTINCT FROM OLD.empreinte OR NEW.produite_le IS DISTINCT FROM OLD.produite_le
       OR NEW.version IS DISTINCT FROM OLD.version OR NEW.echeance_archive IS DISTINCT FROM OLD.echeance_archive THEN
        RAISE EXCEPTION 'ent_cdt_archive : archive immuable (seule la décision change)' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_archive_garde_trg ON ent_cdt_archive;
CREATE TRIGGER ent_cdt_archive_garde_trg BEFORE UPDATE OR DELETE ON ent_cdt_archive
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_archive_garde();

SELECT ent_rls_poser(ARRAY['ent_cdt_archive']);
