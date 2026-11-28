-- Socle 21, lot V14 « cohabitation de l'an 1 » (`docs/socles/21_EVALUATIONS.md` §16, §17.15, EVA-233 à EVA-237, CE-89 ;
-- livraison : `docs/socles/21_EVALUATIONS_V14_LIVRAISON.md`). Plage d'extension 701–719 du socle 21 ; posée en FIN du tableau
-- de `migrate.rs` (règle 38). Additive et rejouable ; aucune migration versée n'est modifiée (règle 25).
--
-- L'an 1 du lycée pilote (D-032), notes et bulletins restent dans le logiciel tiers (Charlemagne). ScolIA :
--   * reçoit les bulletins PDF produits par ce logiciel (un dépôt par période, un fichier par élève) ;
--   * les rapproche des élèves (INE, puis nom, prénom et date de naissance ; sinon file « à rapprocher », CE-89) ;
--   * les distribue SEUL aux familles selon les droits du lien (`peut_voir_notes`), et à l'élève ;
--   * exporte l'assiduité de la période pour le bulletin du logiciel tiers (journal de chaque export) ;
--   * affiche aux familles où consulter les notes pendant la cohabitation (bandeau réglé par la direction).
-- Un bulletin non rapproché n'est JAMAIS distribué : la base le refuse (contraintes ci-dessous).

-- 1. Dépôt (un lot = une période, un logiciel) ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS eva_tiers_lot (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    logiciel TEXT NOT NULL DEFAULT 'charlemagne' CHECK (logiciel IN ('charlemagne', 'autre')),
    libelle TEXT NOT NULL CHECK (char_length(libelle) BETWEEN 1 AND 120),
    depose_par UUID NOT NULL,
    depose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_tiers_lot_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT eva_tiers_lot_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_tiers_lot_periode_idx ON eva_tiers_lot (etablissement_id, periode_id, depose_le DESC);

-- 2. Un fichier déposé (un bulletin d'un élève) --------------------------------------------------------------------------
-- statut : a_rapprocher → rapproche → distribue → (remplace | retire) ; ecarte depuis a_rapprocher ou rapproche.
-- Indices : ce que le dépôt a annoncé (saisi par la secrétaire ou lu dans le nom du fichier) ; ils ne servent qu'au
-- rapprochement et ne sont jamais montrés à une famille.
CREATE TABLE IF NOT EXISTS eva_tiers_bulletin (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lot_id UUID NOT NULL,
    periode_id UUID NOT NULL,
    nom_fichier TEXT NOT NULL CHECK (char_length(nom_fichier) BETWEEN 1 AND 255),
    storage_key TEXT NOT NULL,
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    taille INTEGER NOT NULL CHECK (taille > 0),
    indice_ine TEXT CHECK (indice_ine IS NULL OR char_length(indice_ine) BETWEEN 1 AND 20),
    indice_nom TEXT CHECK (indice_nom IS NULL OR char_length(indice_nom) BETWEEN 1 AND 120),
    indice_prenom TEXT CHECK (indice_prenom IS NULL OR char_length(indice_prenom) BETWEEN 1 AND 120),
    indice_naissance DATE,
    eleve_id UUID,
    rapprochement TEXT CHECK (rapprochement IS NULL OR rapprochement IN ('ine', 'identite', 'manuel')),
    rapproche_par UUID,
    rapproche_le TIMESTAMPTZ,
    statut TEXT NOT NULL DEFAULT 'a_rapprocher'
        CHECK (statut IN ('a_rapprocher', 'rapproche', 'distribue', 'remplace', 'retire', 'ecarte')),
    motif TEXT CHECK (motif IS NULL OR char_length(motif) BETWEEN 3 AND 500),
    document_id UUID,
    distribue_par UUID,
    distribue_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_tiers_bulletin_lot_fk FOREIGN KEY (lot_id, etablissement_id)
        REFERENCES eva_tiers_lot (id, etablissement_id),
    CONSTRAINT eva_tiers_bulletin_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_tiers_bulletin_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT eva_tiers_bulletin_document_fk FOREIGN KEY (document_id) REFERENCES ent_eleve_document (id),
    -- CE-89 : « à rapprocher » n'a pas d'élève ; rapproché, distribué, remplacé ou retiré en a un, avec sa voie.
    CONSTRAINT eva_tiers_bulletin_a_rapprocher_chk CHECK (statut <> 'a_rapprocher' OR eleve_id IS NULL),
    CONSTRAINT eva_tiers_bulletin_rapproche_chk CHECK (
        statut NOT IN ('rapproche', 'distribue', 'remplace', 'retire')
        OR (eleve_id IS NOT NULL AND rapprochement IS NOT NULL AND rapproche_le IS NOT NULL)),
    -- Distribué (et ce qui en découle) : date, auteur et document du coffre ; jamais avant.
    CONSTRAINT eva_tiers_bulletin_distribue_chk CHECK (
        (statut IN ('distribue', 'remplace', 'retire'))
        = (distribue_le IS NOT NULL AND distribue_par IS NOT NULL AND document_id IS NOT NULL)),
    CONSTRAINT eva_tiers_bulletin_motif_chk CHECK (statut NOT IN ('ecarte', 'retire') OR motif IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS eva_tiers_bulletin_lot_idx ON eva_tiers_bulletin (etablissement_id, lot_id);
CREATE INDEX IF NOT EXISTS eva_tiers_bulletin_eleve_idx ON eva_tiers_bulletin (etablissement_id, eleve_id, periode_id);
-- Un seul bulletin distribué par élève et par période (une nouvelle version remplace l'ancienne).
CREATE UNIQUE INDEX IF NOT EXISTS eva_tiers_bulletin_distribue_uq
    ON eva_tiers_bulletin (etablissement_id, eleve_id, periode_id) WHERE statut = 'distribue';
-- Le même fichier n'est pas déposé deux fois pour une période (sauf s'il a été écarté).
CREATE UNIQUE INDEX IF NOT EXISTS eva_tiers_bulletin_empreinte_uq
    ON eva_tiers_bulletin (etablissement_id, periode_id, empreinte) WHERE statut <> 'ecarte';

-- Fichier, période, lot, empreinte et date de dépôt ne changent jamais ; un fichier déposé ne se supprime pas ;
-- un bulletin distribué ne revient jamais en arrière (seulement remplacé ou retiré, avec motif pour le retrait).
CREATE OR REPLACE FUNCTION eva_tiers_bulletin_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_tiers_bulletin : suppression interdite (écarter avec un motif)' USING ERRCODE = '42501';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.lot_id IS DISTINCT FROM OLD.lot_id
       OR NEW.periode_id IS DISTINCT FROM OLD.periode_id
       OR NEW.storage_key IS DISTINCT FROM OLD.storage_key
       OR NEW.empreinte IS DISTINCT FROM OLD.empreinte
       OR NEW.taille IS DISTINCT FROM OLD.taille
       OR NEW.nom_fichier IS DISTINCT FROM OLD.nom_fichier
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'eva_tiers_bulletin : fichier, lot, période et date de dépôt sont figés' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut IN ('remplace', 'retire', 'ecarte') AND NEW IS DISTINCT FROM OLD THEN
        RAISE EXCEPTION 'eva_tiers_bulletin : un bulletin % ne change plus', OLD.statut USING ERRCODE = '42501';
    END IF;
    IF OLD.statut = 'distribue' AND (NEW.statut NOT IN ('distribue', 'remplace', 'retire')
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.document_id IS DISTINCT FROM OLD.document_id
       OR NEW.distribue_le IS DISTINCT FROM OLD.distribue_le) THEN
        RAISE EXCEPTION 'eva_tiers_bulletin : un bulletin distribué se remplace ou se retire, jamais ne se rapproche à nouveau'
            USING ERRCODE = '42501';
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_tiers_bulletin_garde_trg ON eva_tiers_bulletin;
CREATE TRIGGER eva_tiers_bulletin_garde_trg
    BEFORE UPDATE OR DELETE ON eva_tiers_bulletin
    FOR EACH ROW EXECUTE FUNCTION eva_tiers_bulletin_garde();

-- 3. Destinataires figés à la distribution (qui a reçu quoi, et à quel titre) --------------------------------------------
CREATE TABLE IF NOT EXISTS eva_tiers_destinataire (
    bulletin_id UUID NOT NULL REFERENCES eva_tiers_bulletin (id),
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    user_id UUID NOT NULL,
    qualite TEXT NOT NULL CHECK (qualite IN ('responsable', 'eleve')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (bulletin_id, user_id)
);
CREATE INDEX IF NOT EXISTS eva_tiers_destinataire_user_idx ON eva_tiers_destinataire (etablissement_id, user_id);

-- 4. Journal (ajout seul) : dépôt, rapprochement, écart, distribution, remplacement, retrait ----------------------------
CREATE TABLE IF NOT EXISTS eva_tiers_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    bulletin_id UUID NOT NULL REFERENCES eva_tiers_bulletin (id),
    action TEXT NOT NULL CHECK (action IN ('depose', 'rapproche', 'ecarte', 'distribue', 'remplace', 'retire')),
    eleve_id UUID,
    voie TEXT CHECK (voie IS NULL OR voie IN ('ine', 'identite', 'manuel')),
    acteur UUID NOT NULL,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 500),
    le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS eva_tiers_journal_bulletin_idx ON eva_tiers_journal (etablissement_id, bulletin_id, le);

-- 5. Journal des exports d'assiduité vers le logiciel tiers (EVA-233, R-X24 : export tracé) -----------------------------
CREATE TABLE IF NOT EXISTS eva_tiers_export (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    classe_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE NOT NULL CHECK (au >= du),
    eleves INTEGER NOT NULL CHECK (eleves >= 0),
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    exporte_par UUID NOT NULL,
    exporte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_tiers_export_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_tiers_export_etab_idx ON eva_tiers_export (etablissement_id, exporte_le DESC);

CREATE OR REPLACE FUNCTION eva_tiers_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP USING ERRCODE = '42501';
END;
$$;
DROP TRIGGER IF EXISTS eva_tiers_journal_ajout_seul_trg ON eva_tiers_journal;
CREATE TRIGGER eva_tiers_journal_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON eva_tiers_journal
    FOR EACH ROW EXECUTE FUNCTION eva_tiers_ajout_seul();
DROP TRIGGER IF EXISTS eva_tiers_journal_sans_truncate_trg ON eva_tiers_journal;
CREATE TRIGGER eva_tiers_journal_sans_truncate_trg
    BEFORE TRUNCATE ON eva_tiers_journal
    FOR EACH STATEMENT EXECUTE FUNCTION eva_tiers_ajout_seul();
DROP TRIGGER IF EXISTS eva_tiers_export_ajout_seul_trg ON eva_tiers_export;
CREATE TRIGGER eva_tiers_export_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON eva_tiers_export
    FOR EACH ROW EXECUTE FUNCTION eva_tiers_ajout_seul();
DROP TRIGGER IF EXISTS eva_tiers_destinataire_ajout_seul_trg ON eva_tiers_destinataire;
CREATE TRIGGER eva_tiers_destinataire_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON eva_tiers_destinataire
    FOR EACH ROW EXECUTE FUNCTION eva_tiers_ajout_seul();

-- 6. Bandeau « où sont les notes » (EVA-237) : un réglage par établissement ---------------------------------------------
CREATE TABLE IF NOT EXISTS eva_cohabitation (
    etablissement_id UUID PRIMARY KEY CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    actif BOOLEAN NOT NULL DEFAULT false,
    logiciel_libelle TEXT NOT NULL CHECK (char_length(logiciel_libelle) BETWEEN 1 AND 80),
    -- Lien vers le portail de l'éditeur : https seulement (jamais javascript:, data: ni http en clair).
    url TEXT CHECK (url IS NULL OR (url ~ '^https://[^\s<>"'']+$' AND char_length(url) <= 500)),
    message TEXT NOT NULL CHECK (char_length(message) BETWEEN 1 AND 600),
    modifie_par UUID NOT NULL,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

SELECT ent_rls_poser(ARRAY[
    'eva_tiers_lot',
    'eva_tiers_bulletin',
    'eva_tiers_destinataire',
    'eva_tiers_journal',
    'eva_tiers_export',
    'eva_cohabitation']);
