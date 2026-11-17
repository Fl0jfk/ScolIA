-- Socle 18, lot FO3 — migration 2366 : désaccords déclarés, objets créés par un autre socle, lien décision-soumission
-- (`docs/socles/18_FORMULAIRES.md` §7.1 point 4, §21.1 , règles R-FC7, R-FC14, R-FC44 , actions FRM-111, FRM-112, FRM-118).
--
-- Additif, rejouable, sans JSON et sans date UTC.
--
-- * `ent_acc_desaccord` : un désaccord **ouvert** rend le verdict « désaccord » (donc non autorisé) pour tous les objets de la
--   même famille (image, sortie, général) de l'élève. Il se lève seulement avec une pièce et un motif. Le socle 4 (table
--   `ent_eleve_desaccord`, lot A4) n'est pas livré : le désaccord est donc écrit ici et **publié** comme événement
--   `eleve.desaccord_declare` pour que le socle 4 le reprenne. Quand A4 existera, cette table deviendra sa projection.
-- * `ent_acc_objet_source` : l'objet d'accord créé **pour** un autre socle (sortie scolaire du socle 42, par exemple) garde
--   la trace de sa source. Une source n'a qu'un objet.
-- * `ent_acc_decision.formulaire_soumission_id` : la décision prise en répondant à une question « accord » d'un formulaire.

-- 1. Lien décision-soumission ----------------------------------------------------------------------------------
ALTER TABLE ent_acc_decision ADD COLUMN IF NOT EXISTS formulaire_soumission_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_acc_decision_soumission_cfk') THEN
        ALTER TABLE ent_acc_decision ADD CONSTRAINT ent_acc_decision_soumission_cfk
            FOREIGN KEY (etablissement_id, formulaire_soumission_id)
            REFERENCES ent_frm_soumission (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END $$;
CREATE INDEX IF NOT EXISTS ent_acc_decision_soumission_idx
    ON ent_acc_decision (etablissement_id, formulaire_soumission_id) WHERE formulaire_soumission_id IS NOT NULL;

-- La garde d'ajout seul (2362) ne connaissait pas la nouvelle colonne : elle doit rester inchangée aussi.
CREATE OR REPLACE FUNCTION ent_acc_decision_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE'
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.objet_id = OLD.objet_id
       AND NEW.version_id = OLD.version_id
       AND NEW.sujet_personne_id IS NOT DISTINCT FROM OLD.sujet_personne_id
       AND NEW.decideur_user_id = OLD.decideur_user_id AND NEW.qualite = OLD.qualite
       AND NEW.decision = OLD.decision AND NEW.canal = OLD.canal
       AND NEW.preuve_signature_id IS NOT DISTINCT FROM OLD.preuve_signature_id
       AND NEW.date_signature_papier IS NOT DISTINCT FROM OLD.date_signature_papier
       AND NEW.saisie_par_user_id IS NOT DISTINCT FROM OLD.saisie_par_user_id
       AND NEW.annule_decision_id IS NOT DISTINCT FROM OLD.annule_decision_id
       AND NEW.motif_annulation IS NOT DISTINCT FROM OLD.motif_annulation
       AND NEW.decide_le = OLD.decide_le AND NEW.empreinte = OLD.empreinte
       AND NEW.source_reprise IS NOT DISTINCT FROM OLD.source_reprise
       AND NEW.source_reprise_id IS NOT DISTINCT FROM OLD.source_reprise_id
       AND NEW.formulaire_soumission_id IS NOT DISTINCT FROM OLD.formulaire_soumission_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_acc_decision : une décision est en ajout seul' USING ERRCODE = '42501';
END $$;

-- 2. Désaccords déclarés ----------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_acc_desaccord (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sujet_eleve_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (objet IN ('image', 'sortie', 'general')),
    -- Objet d'accord visé (un refus après un accord sur cet objet) , NULL : tous les objets de la famille.
    objet_acc_id UUID,
    source TEXT NOT NULL CHECK (source IN ('accord', 'declaration', 'etablissement')),
    declarant_user_id UUID NOT NULL,
    declare_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    piece_document_id UUID,
    leve_le TIMESTAMPTZ,
    leve_par UUID,
    leve_motif TEXT CHECK (leve_motif IS NULL OR char_length(btrim(leve_motif)) BETWEEN 1 AND 300),
    CONSTRAINT ent_acc_desaccord_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_acc_desaccord_eleve_cfk FOREIGN KEY (etablissement_id, sujet_eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_desaccord_objet_cfk FOREIGN KEY (etablissement_id, objet_acc_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_desaccord_leve_chk CHECK (
        (leve_le IS NULL) = (leve_par IS NULL) AND (leve_le IS NULL) = (leve_motif IS NULL)
        AND (leve_le IS NULL OR piece_document_id IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_acc_desaccord_eleve_idx ON ent_acc_desaccord (etablissement_id, sujet_eleve_id) WHERE leve_le IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_acc_desaccord_ouvert_uq
    ON ent_acc_desaccord (etablissement_id, sujet_eleve_id, objet, COALESCE(objet_acc_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE leve_le IS NULL;

-- Un désaccord ne se modifie pas et ne se supprime pas : il se lève une fois, avec pièce et motif.
CREATE OR REPLACE FUNCTION ent_acc_desaccord_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.leve_le IS NULL AND NEW.leve_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.sujet_eleve_id = OLD.sujet_eleve_id
       AND NEW.objet = OLD.objet AND NEW.objet_acc_id IS NOT DISTINCT FROM OLD.objet_acc_id AND NEW.source = OLD.source
       AND NEW.declarant_user_id = OLD.declarant_user_id AND NEW.declare_le = OLD.declare_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_acc_desaccord : un désaccord se lève une fois (pièce et motif), il ne se modifie ni ne se supprime'
        USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_acc_desaccord_garde ON ent_acc_desaccord;
CREATE TRIGGER ent_acc_desaccord_garde BEFORE UPDATE OR DELETE ON ent_acc_desaccord
    FOR EACH ROW EXECUTE FUNCTION ent_acc_desaccord_garde();

-- 3. Objets créés pour un autre socle ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_acc_objet_source (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet_id UUID NOT NULL,
    source_socle SMALLINT NOT NULL CHECK (source_socle BETWEEN 1 AND 60),
    source_type TEXT NOT NULL CHECK (source_type ~ '^[a-z][a-z0-9_]{1,39}$'),
    source_id UUID NOT NULL,
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, objet_id),
    CONSTRAINT ent_acc_objet_source_uq UNIQUE (etablissement_id, source_type, source_id),
    CONSTRAINT ent_acc_objet_source_objet_cfk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT
);
CREATE OR REPLACE FUNCTION ent_acc_objet_source_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    RAISE EXCEPTION 'ent_acc_objet_source : table en ajout seul' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_acc_objet_source_garde ON ent_acc_objet_source;
CREATE TRIGGER ent_acc_objet_source_garde BEFORE UPDATE OR DELETE ON ent_acc_objet_source
    FOR EACH ROW EXECUTE FUNCTION ent_acc_objet_source_garde();

-- 4. Nouveaux actes du journal des accords -------------------------------------------------------------------------
ALTER TABLE ent_acc_journal DROP CONSTRAINT IF EXISTS ent_acc_journal_acte_check;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_acc_journal_acte_fo3_chk') THEN
        ALTER TABLE ent_acc_journal ADD CONSTRAINT ent_acc_journal_acte_fo3_chk CHECK (acte IN (
            'objet_active', 'objet_cree', 'objet_desactive', 'version_redigee', 'version_publiee', 'version_relue',
            'decision', 'retrait', 'lecture_detaillee', 'verification', 'reprise',
            'regle_relevee', 'desaccord_ouvert', 'desaccord_leve', 'objet_pour_source', 'document_lie', 'dossier_preuve_lu'));
    END IF;
END $$;

SELECT ent_rls_poser(ARRAY['ent_acc_desaccord', 'ent_acc_objet_source']);
