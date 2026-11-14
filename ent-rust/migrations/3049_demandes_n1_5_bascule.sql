-- Socle N1 (« Demandes et flux de travail »), lot N1-5 — migration 3049 : gardes de la bascule.
-- Réf. : docs/socles/N1_DEMANDES.md §29, DEM-349 (guichet en lecture seule), DEM-365, CDM-50, CDM-62.
--
-- Trois choses, toutes additives :
-- 1. ent_dem_guichet : l'état « lecture seule » du guichet de ScolIA pendant la bascule ou un incident (le geste côté
--    Next.js, lui, est celui de Florian, hors de ce dépôt). Une ligne par établissement ; absente = guichet ouvert.
-- 2. La garde de la demande accepte UN seul changement de demandeur de plus : le rapprochement, par le secrétariat, d'un
--    demandeur « externe non vérifié » de la reprise vers la personne retrouvée (sous un réglage LOCAL à la transaction,
--    comme la purge et la requalification). Tout le reste de la garde est repris à l'identique de la migration 3045.
-- 3. Une HSE reprise comme « transmise hors ScolIA » (déjà envoyée au rectorat par e-mail depuis le Next.js, CDM-50) ne
--    peut recevoir AUCUNE séance : sinon une attestation ultérieure créerait au socle 15 des heures déjà payées.

-- ───────────────────────── 1. Guichet en lecture seule ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_guichet (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lecture_seule BOOLEAN NOT NULL DEFAULT false,
    raison TEXT NOT NULL DEFAULT 'bascule' CHECK (raison IN ('bascule', 'incident', 'maintenance')),
    depuis_le TIMESTAMPTZ,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par_user_id UUID,
    CONSTRAINT ent_dem_guichet_depuis_chk CHECK (NOT lecture_seule OR depuis_le IS NOT NULL)
);

-- Le journal des réglages connaît désormais le guichet et la reprise (mêmes valeurs qu'en 3046, plus deux).
ALTER TABLE ent_dem_reglage_journal DROP CONSTRAINT IF EXISTS ent_dem_reglage_journal_sujet_check;
ALTER TABLE ent_dem_reglage_journal ADD CONSTRAINT ent_dem_reglage_journal_sujet_check CHECK (sujet IN (
    'repli_rh', 'nature', 'file', 'membres', 'routage', 'reponse_type', 'palier', 'hse', 'reprographie',
    'guichet', 'reprise'));

-- ───────────────────────── 2. Rapprochement d'un demandeur de reprise ─────────────────────────
CREATE OR REPLACE FUNCTION ent_dem_rapprochement_en_cours() RETURNS boolean
LANGUAGE sql STABLE AS $$
    SELECT COALESCE(current_setting('ent.dem_rapprochement', true), '') = 'on'
$$;

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
    -- Le demandeur ne change jamais (R-DM4), SAUF le rapprochement d'une demande REPRISE (canal « reprise »).
    IF NEW.purgee_le IS NULL AND NEW.demandeur_personne_id IS DISTINCT FROM OLD.demandeur_personne_id
       AND NOT (OLD.canal_depot = 'reprise' AND ent_dem_rapprochement_en_cours()) THEN
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

-- ───────────────────────── 3. HSE « transmise hors ScolIA » : aucune séance (CDM-50) ─────────────────────────
CREATE OR REPLACE FUNCTION ent_dem_hse_seance_hors_scolia() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ent_dem_reprise_correspondance c
         WHERE c.etablissement_id = NEW.etablissement_id AND c.source = 'demandes_hse'
           AND c.cible_id = NEW.demande_id AND c.raison_code = 'hse_transmise_hors_scolia') THEN
        RAISE EXCEPTION 'ent_dem_hse_seance : HSE déjà transmise hors ScolIA (reprise), aucune séance ne peut être ajoutée (CDM-50)'
            USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_dem_hse_seance_hors_scolia_trg ON ent_dem_hse_seance;
CREATE TRIGGER ent_dem_hse_seance_hors_scolia_trg BEFORE INSERT ON ent_dem_hse_seance
    FOR EACH ROW EXECUTE FUNCTION ent_dem_hse_seance_hors_scolia();

SELECT ent_rls_poser(ARRAY['ent_dem_guichet']);
