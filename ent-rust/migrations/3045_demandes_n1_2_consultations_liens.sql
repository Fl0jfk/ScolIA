-- Socle N1 (« Demandes et flux de travail »), lot N1-2 — migration 3045 : consultations, liens, personnes visées,
-- requalification.
-- Réf. : docs/socles/N1_DEMANDES.md §11.6 (consulter), §28.2 (ent_dem_consultation, ent_dem_lien), R-DM47
-- (DEM-360 : la personne visée n'est jamais destinataire), CDM-01 et CDM-11 (requalification), NJ-81.
--
-- Additive et rejouable (E8). Aucune donnée semi-structurée, aucun tableau, aucun contenu binaire (D-036).
-- Clés étrangères composites et ON DELETE RESTRICT partout (NJ-89).

-- ───────────────────────── Consultations ─────────────────────────
-- Un tiers (agent nommé, dans ses droits) est consulté par le traitant : il voit la QUESTION et ce que le traitant
-- choisit de partager (pièces nommées, et le texte de la demande seulement si la nature n'est pas C2+) ; sa
-- réponse est un échange de consultation ; la demande reste chez le traitant (§11.6).
CREATE TABLE IF NOT EXISTS ent_dem_consultation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    consulte_agent_id UUID,
    consulte_file_id UUID,
    question TEXT NOT NULL CHECK (length(question) BETWEEN 1 AND 30000),
    question_scellee BOOLEAN NOT NULL DEFAULT false,
    partage_demande BOOLEAN NOT NULL DEFAULT false,
    echeance DATE NOT NULL,
    demandee_par_personne_id UUID NOT NULL,
    repondue_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_consultation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_consultation_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_consultation_agent_cfk FOREIGN KEY (etablissement_id, consulte_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_consultation_file_cfk FOREIGN KEY (etablissement_id, consulte_file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_consultation_par_cfk FOREIGN KEY (etablissement_id, demandee_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_consultation_un_consulte_chk CHECK (num_nonnulls(consulte_agent_id, consulte_file_id) = 1)
);
CREATE INDEX IF NOT EXISTS ent_dem_consultation_demande_idx ON ent_dem_consultation (etablissement_id, demande_id);
CREATE INDEX IF NOT EXISTS ent_dem_consultation_agent_idx
    ON ent_dem_consultation (etablissement_id, consulte_agent_id) WHERE consulte_agent_id IS NOT NULL;

-- Une consultation ne change pas : seule la date de réponse s'écrit, une fois ; rien ne se supprime.
CREATE OR REPLACE FUNCTION ent_dem_consultation_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_consultation : suppression interdite (NJ-89)' USING ERRCODE = '23001';
    END IF;
    -- Requalification en nature confidentielle : la question écrite en clair est SCELLÉE, rien d'autre ne change.
    IF ent_dem_requalif_en_cours() AND NOT OLD.question_scellee AND NEW.question_scellee
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.demande_id = OLD.demande_id
       AND NEW.consulte_agent_id IS NOT DISTINCT FROM OLD.consulte_agent_id
       AND NEW.consulte_file_id IS NOT DISTINCT FROM OLD.consulte_file_id
       AND NEW.partage_demande = OLD.partage_demande AND NEW.echeance = OLD.echeance
       AND NEW.demandee_par_personne_id = OLD.demandee_par_personne_id
       AND NEW.repondue_le IS NOT DISTINCT FROM OLD.repondue_le AND NEW.cree_le = OLD.cree_le THEN
        RETURN NEW;
    END IF;
    IF OLD.repondue_le IS NULL AND NEW.repondue_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.demande_id = OLD.demande_id
       AND NEW.consulte_agent_id IS NOT DISTINCT FROM OLD.consulte_agent_id
       AND NEW.consulte_file_id IS NOT DISTINCT FROM OLD.consulte_file_id
       AND NEW.question = OLD.question AND NEW.question_scellee = OLD.question_scellee
       AND NEW.partage_demande = OLD.partage_demande AND NEW.echeance = OLD.echeance
       AND NEW.demandee_par_personne_id = OLD.demandee_par_personne_id AND NEW.cree_le = OLD.cree_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_dem_consultation : une consultation ne se modifie pas' USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_consultation_garde_trg ON ent_dem_consultation;
CREATE TRIGGER ent_dem_consultation_garde_trg
    BEFORE UPDATE OR DELETE ON ent_dem_consultation
    FOR EACH ROW EXECUTE FUNCTION ent_dem_consultation_garde();

-- Pièces PARTAGÉES avec un consulté : sans ligne ici, un consulté ne lit aucune pièce (NJ-83).
CREATE TABLE IF NOT EXISTS ent_dem_consultation_partage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    consultation_id UUID NOT NULL,
    piece_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_consultation_partage_uq UNIQUE (etablissement_id, consultation_id, piece_id),
    CONSTRAINT ent_dem_consultation_partage_consultation_cfk FOREIGN KEY (etablissement_id, consultation_id)
        REFERENCES ent_dem_consultation (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_consultation_partage_piece_cfk FOREIGN KEY (etablissement_id, piece_id)
        REFERENCES ent_dem_piece (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_dem_consultation_partage_sans_suppression_trg ON ent_dem_consultation_partage;
CREATE TRIGGER ent_dem_consultation_partage_sans_suppression_trg
    BEFORE DELETE ON ent_dem_consultation_partage
    FOR EACH ROW EXECUTE FUNCTION ent_dem_sans_suppression();

-- Le message de consultation (migration 3044) rejoint sa consultation.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_dem_message_consultation_cfk') THEN
        ALTER TABLE ent_dem_message ADD CONSTRAINT ent_dem_message_consultation_cfk
            FOREIGN KEY (etablissement_id, consultation_id)
            REFERENCES ent_dem_consultation (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;

-- ───────────────────────── Liens entre demandes ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('doublon_de', 'issue_de_conversation', 'reouverture_de', 'avenant_de', 'reprise_de')),
    cible_demande_id UUID,
    cible_ref UUID,
    cree_par_personne_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_lien_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_lien_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_lien_cible_cfk FOREIGN KEY (etablissement_id, cible_demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_lien_par_cfk FOREIGN KEY (etablissement_id, cree_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    -- Une cible : une demande, ou une référence externe (le fil du socle 16 pour `issue_de_conversation`).
    CONSTRAINT ent_dem_lien_cible_chk CHECK (num_nonnulls(cible_demande_id, cible_ref) = 1),
    CONSTRAINT ent_dem_lien_pas_soi_chk CHECK (cible_demande_id IS NULL OR cible_demande_id <> demande_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_lien_uq
    ON ent_dem_lien (etablissement_id, demande_id, type, COALESCE(cible_demande_id, cible_ref));
DROP TRIGGER IF EXISTS ent_dem_lien_sans_suppression_trg ON ent_dem_lien;
CREATE TRIGGER ent_dem_lien_sans_suppression_trg
    BEFORE DELETE ON ent_dem_lien
    FOR EACH ROW EXECUTE FUNCTION ent_dem_sans_suppression();

-- ───────────────────────── Personnes visées (DEM-360, R-DM47) ─────────────────────────
-- « Cette demande concerne une personne de ce service » : la personne désignée par le demandeur n'est JAMAIS
-- destinataire ni lectrice de la demande, même si elle est membre de la file. Sans motif demandé ; ajout seul.
CREATE TABLE IF NOT EXISTS ent_dem_personne_visee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    designee_par_personne_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_personne_visee_uq UNIQUE (etablissement_id, demande_id, personne_id),
    CONSTRAINT ent_dem_personne_visee_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_personne_visee_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_personne_visee_par_cfk FOREIGN KEY (etablissement_id, designee_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_dem_personne_visee_personne_idx ON ent_dem_personne_visee (etablissement_id, personne_id);
CREATE OR REPLACE FUNCTION ent_dem_personne_visee_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_dem_personne_visee : ajout seul (R-DM47) ; une désignation ne se retire pas en silence'
        USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_personne_visee_ajout_seul_trg ON ent_dem_personne_visee;
CREATE TRIGGER ent_dem_personne_visee_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_dem_personne_visee
    FOR EACH ROW EXECUTE FUNCTION ent_dem_personne_visee_ajout_seul();

-- ───────────────────────── Requalification (CDM-01, CDM-11, DEM-153) ─────────────────────────
-- La nature (code et version) d'une demande est figée par la garde de la migration 3040, SAUF pendant une
-- requalification : la garde accepte alors le changement, sous un réglage LOCAL à la transaction posé par le seul
-- chemin de requalification (comme la purge). Tout le reste de la garde est repris à l'identique.
CREATE OR REPLACE FUNCTION ent_dem_demande_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_demande : suppression interdite (R-DM28, NJ-89) ; purge par l''ordonnanceur seulement'
            USING ERRCODE = '23001';
    END IF;
    IF OLD.purgee_le IS NOT NULL THEN
        RAISE EXCEPTION 'ent_dem_demande : demande purgée, plus aucune écriture' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.numero IS DISTINCT FROM OLD.numero
       OR NEW.annee_id IS DISTINCT FROM OLD.annee_id
       OR NEW.deposant_personne_id IS DISTINCT FROM OLD.deposant_personne_id
       OR NEW.canal_depot IS DISTINCT FROM OLD.canal_depot
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'ent_dem_demande : identité de la demande figée' USING ERRCODE = '23001';
    END IF;
    IF (NEW.nature_code IS DISTINCT FROM OLD.nature_code OR NEW.nature_version IS DISTINCT FROM OLD.nature_version)
       AND NOT ent_dem_requalif_en_cours() THEN
        RAISE EXCEPTION 'ent_dem_demande : la nature ne change que par une requalification (CDM-01)'
            USING ERRCODE = '23001';
    END IF;
    IF NEW.purgee_le IS NULL AND NEW.demandeur_personne_id IS DISTINCT FROM OLD.demandeur_personne_id THEN
        RAISE EXCEPTION 'ent_dem_demande : le demandeur ne change jamais (R-DM4)' USING ERRCODE = '23001';
    END IF;
    IF NEW.purgee_le IS NOT NULL AND NOT ent_dem_purge_en_cours() THEN
        RAISE EXCEPTION 'ent_dem_demande : purge réservée à l''ordonnanceur (R-DM29)' USING ERRCODE = '23001';
    END IF;
    IF NEW.version <> OLD.version + 1 THEN
        RAISE EXCEPTION 'ent_dem_demande : version périmée ou non incrémentée (% -> %)', OLD.version, NEW.version
            USING ERRCODE = '40001';
    END IF;
    NEW.maj_le := now();
    RETURN NEW;
END
$$;

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_consultation',
    'ent_dem_consultation_partage',
    'ent_dem_lien',
    'ent_dem_personne_visee'
]);
