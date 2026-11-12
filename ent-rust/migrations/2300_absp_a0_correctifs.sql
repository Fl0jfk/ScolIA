-- 2300 — Socle 15 (absences des personnels), lot A0 : correctifs urgents de l'existant
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §28.4, AR-06, AR-13, AR-15, AR-16, AR-17).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée sur les nouvelles tables.
--
-- 1. `ent_absence_rh` : l'auteur de la déclaration n'est plus écrasé par la validation (AR-13) ;
--    valideur, date de validation, refus et annulation en colonnes (R-AP5) ; statut `annulee` (AR-16) ;
--    déclencheur « le valideur n'est pas l'absent » (AR-06, R-AP10), en plus du contrôle de service.
-- 2. Levée d'une exception ou d'un déplacement d'EDT : plus de suppression physique (AR-15, R-AP8).
--    La ligne levée est COPIÉE dans une table d'archive datée avant d'être retirée de la table vivante :
--    les lecteurs de `ent_edt_exception` / `ent_edt_deplacement` n'ont donc rien à filtrer et une séance
--    levée ne peut pas rester « annulée » par oubli d'un filtre.

-- 1. Absence RH -----------------------------------------------------------------------------------
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS declare_par_user_id UUID;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS valide_par_user_id UUID;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS valide_le TIMESTAMPTZ;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS refuse_par_user_id UUID;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS refuse_le TIMESTAMPTZ;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS annule_par_user_id UUID;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS annule_le TIMESTAMPTZ;
-- Confirmée par l'agent lui-même (§28.4 point 4 : une absence seulement SIGNALÉE par un tiers ne
-- bloque plus la personne comme remplaçante). L'existant est réputé confirmé (comportement actuel).
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS confirmee BOOLEAN NOT NULL DEFAULT true;

-- Reprise AVANT tout écrasement futur. Avant ce lot, la validation écrasait `created_by_user_id` par
-- l'auteur de la validation : pour une ligne `validee`, la valeur lue est donc le VALIDEUR, et la
-- déclarante n'est plus connue (laissée NULL : on n'invente pas un auteur). Pour les autres lignes,
-- `created_by_user_id` est bien l'auteur de la déclaration.
-- Le déclencheur « valideur ≠ absent » (créé plus bas, et déjà présent aux rejeux) est suspendu pendant la
-- reprise : une ancienne auto-validation (NJ-19) ne doit pas faire échouer la migration.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'ent_absence_rh_valideur') THEN
        ALTER TABLE ent_absence_rh DISABLE TRIGGER ent_absence_rh_valideur;
    END IF;
END $$;

UPDATE ent_absence_rh
   SET declare_par_user_id = CASE WHEN statut = 'validee' THEN NULL ELSE created_by_user_id END,
       valide_par_user_id  = CASE WHEN statut = 'validee' THEN created_by_user_id END,
       valide_le           = CASE WHEN statut = 'validee' THEN created_at END
 WHERE declare_par_user_id IS NULL AND valide_par_user_id IS NULL;

DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'ent_absence_rh_valideur') THEN
        ALTER TABLE ent_absence_rh ENABLE TRIGGER ent_absence_rh_valideur;
    END IF;
END $$;

ALTER TABLE ent_absence_rh DROP CONSTRAINT IF EXISTS ent_absence_rh_statut_chk;
ALTER TABLE ent_absence_rh ADD CONSTRAINT ent_absence_rh_statut_chk
    CHECK (statut IN ('signalee', 'validee', 'refusee', 'annulee'));

-- Un valideur n'est jamais l'absent (R-AP10). Le contrôle ne porte que sur les lignes écrites :
-- une ancienne ligne auto-validée n'empêche pas la migration (elle reste visible pour contrôle).
CREATE OR REPLACE FUNCTION ent_trg_absence_rh_valideur() RETURNS trigger AS $$
BEGIN
    IF NEW.valide_par_user_id IS NOT NULL
       AND NEW.valide_par_user_id = NEW.user_id
       AND (TG_OP = 'INSERT' OR NEW.valide_par_user_id IS DISTINCT FROM OLD.valide_par_user_id) THEN
        RAISE EXCEPTION 'Personne ne valide sa propre absence (R-AP10)' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absence_rh_valideur ON ent_absence_rh;
CREATE TRIGGER ent_absence_rh_valideur BEFORE INSERT OR UPDATE ON ent_absence_rh
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absence_rh_valideur();

-- 2. Archives de levée ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_edt_exception_levee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    exception_id UUID NOT NULL,
    creneau_id UUID NOT NULL,
    jour DATE NOT NULL,
    kind TEXT NOT NULL,
    nature TEXT,
    remplacant_user_id UUID,
    nouvelle_salle TEXT,
    absence_rh_id UUID,
    lot_id UUID,
    pose_par_user_id UUID,
    pose_le TIMESTAMPTZ,
    -- `levee` : l'exception a été retirée ; `remplacee` : une autre décision l'a remplacée
    -- (l'état précédent est gardé, jamais écrasé).
    cause TEXT NOT NULL DEFAULT 'levee' CHECK (cause IN ('levee', 'remplacee')),
    levee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    levee_par_user_id UUID NOT NULL,
    CONSTRAINT ent_edt_exception_levee_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_edt_exception_levee_idx
    ON ent_edt_exception_levee (etablissement_id, creneau_id, jour);

CREATE TABLE IF NOT EXISTS ent_edt_deplacement_leve (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    deplacement_id UUID NOT NULL,
    creneau_id UUID NOT NULL,
    jour_origine DATE NOT NULL,
    nouveau_jour DATE NOT NULL,
    nouvelle_heure_debut TIME NOT NULL,
    nouvelle_heure_fin TIME NOT NULL,
    nouvelle_salle TEXT,
    levee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    levee_par_user_id UUID NOT NULL,
    CONSTRAINT ent_edt_deplacement_leve_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_edt_deplacement_leve_idx
    ON ent_edt_deplacement_leve (etablissement_id, creneau_id, jour_origine);

-- Ajout seul : une levée archivée ne se modifie ni ne se supprime.
CREATE OR REPLACE FUNCTION ent_trg_archive_levee_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_edt_exception_levee_ajout_seul ON ent_edt_exception_levee;
CREATE TRIGGER ent_edt_exception_levee_ajout_seul BEFORE UPDATE OR DELETE ON ent_edt_exception_levee
    FOR EACH ROW EXECUTE FUNCTION ent_trg_archive_levee_ajout_seul();
DROP TRIGGER IF EXISTS ent_edt_deplacement_leve_ajout_seul ON ent_edt_deplacement_leve;
CREATE TRIGGER ent_edt_deplacement_leve_ajout_seul BEFORE UPDATE OR DELETE ON ent_edt_deplacement_leve
    FOR EACH ROW EXECUTE FUNCTION ent_trg_archive_levee_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_edt_exception_levee', 'ent_edt_deplacement_leve']);
