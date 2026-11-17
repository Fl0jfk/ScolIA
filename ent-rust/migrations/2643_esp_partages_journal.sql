-- Socle 32, lot ET1 — migration 2643 : partages nominatifs et journal en ajout seul
-- (`32_BUREAUTIQUE.md` §8.3, §15 point 6, §24, R-ET2, R-ET9, R-ET13, ND32-03, ND32-08, ND32-11).
--
-- * `ent_espace_partage` : accès donné à une **personne nommée** de l'établissement, sur l'**identifiant** d'un élément
--   ou d'un dossier (jamais un chemin : un déplacement ne casse pas le partage, R-ET13), en lecture ou écriture, daté.
--   Le destinataire est une `personne` du même établissement (clé composite : ND32-03, « membres non vérifiés »).
--   Aucun lien public (R-ET9) : un partage n'existe que vers une personne nommée.
-- * `ent_espace_journal` : journal **en ajout seul** (E7) : création, partage, changement de membres, téléchargement,
--   corbeille, restauration, purge. Codes courts, jamais de nom de fichier ni de texte libre (minimisation).
-- Sans JSON, sans tableau, sans BYTEA. Additif, rejouable.

CREATE TABLE IF NOT EXISTS ent_espace_partage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    element_id UUID,
    dossier_id UUID,
    destinataire_personne_id UUID NOT NULL,
    droit TEXT NOT NULL CHECK (droit IN ('lecture', 'ecriture')),
    debut DATE NOT NULL,
    fin DATE,
    par_personne_id UUID NOT NULL,
    -- Badge « non vu » (NX32-19) : levé par une commande explicite.
    vu_le TIMESTAMPTZ,
    retire_le TIMESTAMPTZ,
    retire_par_personne_id UUID,
    retire_motif_code TEXT CHECK (retire_motif_code IS NULL OR retire_motif_code IN ('auteur', 'destinataire', 'animateur', 'espace')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_partage_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_partage_cible_chk CHECK ((element_id IS NOT NULL) <> (dossier_id IS NOT NULL)),
    CONSTRAINT ent_espace_partage_dates_chk CHECK (fin IS NULL OR fin >= debut),
    CONSTRAINT ent_espace_partage_retrait_chk CHECK (
        (retire_le IS NULL) = (retire_par_personne_id IS NULL) AND (retire_le IS NULL) = (retire_motif_code IS NULL)),
    CONSTRAINT ent_espace_partage_element_fk FOREIGN KEY (etablissement_id, espace_id, element_id)
        REFERENCES ent_espace_element (etablissement_id, espace_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_partage_dossier_fk FOREIGN KEY (etablissement_id, espace_id, dossier_id)
        REFERENCES ent_espace_dossier (etablissement_id, espace_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_partage_destinataire_fk FOREIGN KEY (etablissement_id, destinataire_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_partage_par_fk FOREIGN KEY (etablissement_id, par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
-- Un seul partage actif par cible et destinataire (modifier = remplacer, pas empiler).
CREATE UNIQUE INDEX IF NOT EXISTS ent_espace_partage_element_uq
    ON ent_espace_partage (etablissement_id, element_id, destinataire_personne_id)
    WHERE retire_le IS NULL AND element_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_espace_partage_dossier_uq
    ON ent_espace_partage (etablissement_id, dossier_id, destinataire_personne_id)
    WHERE retire_le IS NULL AND dossier_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_espace_partage_destinataire_idx
    ON ent_espace_partage (etablissement_id, destinataire_personne_id) WHERE retire_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_espace_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID,
    element_id UUID,
    acteur_personne_id UUID,
    action_code TEXT NOT NULL CHECK (action_code ~ '^[a-z0-9_.]{1,60}$'),
    detail_code TEXT CHECK (detail_code IS NULL OR detail_code ~ '^[a-z0-9_.:-]{1,80}$'),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_journal_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_journal_element_fk FOREIGN KEY (etablissement_id, element_id)
        REFERENCES ent_espace_element (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_journal_acteur_fk FOREIGN KEY (etablissement_id, acteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_espace_journal_espace_idx ON ent_espace_journal (etablissement_id, espace_id, le DESC);
CREATE INDEX IF NOT EXISTS ent_espace_journal_element_idx ON ent_espace_journal (etablissement_id, element_id, le DESC);

CREATE OR REPLACE FUNCTION ent_espace_journal_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'ent_espace_journal : journal en ajout seul' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_espace_journal_garde ON ent_espace_journal;
CREATE TRIGGER ent_espace_journal_garde BEFORE UPDATE OR DELETE ON ent_espace_journal
    FOR EACH ROW EXECUTE FUNCTION ent_espace_journal_garde();

SELECT ent_rls_poser(ARRAY['ent_espace_partage', 'ent_espace_journal']);
