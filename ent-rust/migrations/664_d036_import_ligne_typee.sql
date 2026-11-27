-- D-036 (lot 2) — `ent_import_ligne.donnees` (JSONB : la ligne du fichier CSV, clés = colonnes du
-- format `scolia_eleves_v1`) devient dix-sept colonnes TEXT typées, une par colonne du format.
-- Le registre des colonnes reste celui du code (`imports::COLONNES`) ; une valeur vide est NULL.
-- Le JSON de transit entre le code et la base ne sert qu'à passer la ligne (`jsonb_to_record`).
--
-- Reprise idempotente : seules les lignes dont TOUTES les colonnes typées sont NULL et dont
-- l'ancienne colonne est renseignée sont reprises. L'ancienne colonne devient facultative et en
-- LECTURE SEULE (déclencheur) ; elle sera retirée au lot de retrait.
-- Corrige aussi E2 (références composites vers le lot et l'élève).

ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS nom TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS prenom TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS date_naissance TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS sexe TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS ine TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS classe TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS regime TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp1_nom TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp1_prenom TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp1_email TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp1_telephone TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp1_lien TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp2_nom TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp2_prenom TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp2_email TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp2_telephone TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS resp2_lien TEXT;
ALTER TABLE ent_import_ligne ADD COLUMN IF NOT EXISTS cree_le TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE ent_import_ligne ALTER COLUMN donnees DROP NOT NULL;

ALTER TABLE ent_import_ligne DROP CONSTRAINT IF EXISTS ent_import_ligne_valeurs_chk;
ALTER TABLE ent_import_ligne ADD CONSTRAINT ent_import_ligne_valeurs_chk CHECK (
    length(coalesce(nom, '')) <= 4000 AND length(coalesce(prenom, '')) <= 4000
    AND length(coalesce(date_naissance, '')) <= 4000 AND length(coalesce(sexe, '')) <= 4000
    AND length(coalesce(ine, '')) <= 4000 AND length(coalesce(classe, '')) <= 4000
    AND length(coalesce(regime, '')) <= 4000
    AND length(coalesce(resp1_nom, '')) <= 4000 AND length(coalesce(resp1_prenom, '')) <= 4000
    AND length(coalesce(resp1_email, '')) <= 4000 AND length(coalesce(resp1_telephone, '')) <= 4000
    AND length(coalesce(resp1_lien, '')) <= 4000
    AND length(coalesce(resp2_nom, '')) <= 4000 AND length(coalesce(resp2_prenom, '')) <= 4000
    AND length(coalesce(resp2_email, '')) <= 4000 AND length(coalesce(resp2_telephone, '')) <= 4000
    AND length(coalesce(resp2_lien, '')) <= 4000);

-- Reprise (valeurs non textuelles, vides ou trop longues ignorées).
UPDATE ent_import_ligne l SET
    nom = nullif(l.donnees ->> 'nom', ''), prenom = nullif(l.donnees ->> 'prenom', ''),
    date_naissance = nullif(l.donnees ->> 'date_naissance', ''), sexe = nullif(l.donnees ->> 'sexe', ''),
    ine = nullif(l.donnees ->> 'ine', ''), classe = nullif(l.donnees ->> 'classe', ''),
    regime = nullif(l.donnees ->> 'regime', ''),
    resp1_nom = nullif(l.donnees ->> 'resp1_nom', ''), resp1_prenom = nullif(l.donnees ->> 'resp1_prenom', ''),
    resp1_email = nullif(l.donnees ->> 'resp1_email', ''), resp1_telephone = nullif(l.donnees ->> 'resp1_telephone', ''),
    resp1_lien = nullif(l.donnees ->> 'resp1_lien', ''),
    resp2_nom = nullif(l.donnees ->> 'resp2_nom', ''), resp2_prenom = nullif(l.donnees ->> 'resp2_prenom', ''),
    resp2_email = nullif(l.donnees ->> 'resp2_email', ''), resp2_telephone = nullif(l.donnees ->> 'resp2_telephone', ''),
    resp2_lien = nullif(l.donnees ->> 'resp2_lien', '')
 WHERE jsonb_typeof(l.donnees) = 'object'
   AND l.nom IS NULL AND l.prenom IS NULL AND l.date_naissance IS NULL AND l.sexe IS NULL
   AND l.ine IS NULL AND l.classe IS NULL AND l.regime IS NULL
   AND l.resp1_nom IS NULL AND l.resp1_prenom IS NULL AND l.resp1_email IS NULL
   AND l.resp1_telephone IS NULL AND l.resp1_lien IS NULL
   AND l.resp2_nom IS NULL AND l.resp2_prenom IS NULL AND l.resp2_email IS NULL
   AND l.resp2_telephone IS NULL AND l.resp2_lien IS NULL
   AND NOT EXISTS (SELECT 1 FROM jsonb_each(l.donnees) e
                    WHERE jsonb_typeof(e.value) = 'string' AND length(e.value #>> '{}') > 4000);

-- Ligne au format d'échange du code (clés des colonnes non vides).
CREATE OR REPLACE FUNCTION ent_import_ligne_donnees(l ent_import_ligne) RETURNS jsonb
LANGUAGE sql IMMUTABLE AS $$
    SELECT jsonb_strip_nulls(jsonb_build_object(
        'nom', l.nom, 'prenom', l.prenom, 'date_naissance', l.date_naissance, 'sexe', l.sexe,
        'ine', l.ine, 'classe', l.classe, 'regime', l.regime,
        'resp1_nom', l.resp1_nom, 'resp1_prenom', l.resp1_prenom, 'resp1_email', l.resp1_email,
        'resp1_telephone', l.resp1_telephone, 'resp1_lien', l.resp1_lien,
        'resp2_nom', l.resp2_nom, 'resp2_prenom', l.resp2_prenom, 'resp2_email', l.resp2_email,
        'resp2_telephone', l.resp2_telephone, 'resp2_lien', l.resp2_lien))
$$;

CREATE OR REPLACE FUNCTION ent_import_ligne_donnees_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.donnees IS NOT NULL AND (TG_OP = 'INSERT' OR NEW.donnees IS DISTINCT FROM OLD.donnees) THEN
        RAISE EXCEPTION 'ent_import_ligne.donnees est en lecture seule (D-036) : utiliser les colonnes typées'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_import_ligne_donnees_lecture_seule ON ent_import_ligne;
CREATE TRIGGER ent_import_ligne_donnees_lecture_seule
    BEFORE INSERT OR UPDATE OF donnees ON ent_import_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_import_ligne_donnees_lecture_seule();

-- E2 : références composites.
SELECT ent_d036_contrainte('ent_import_lot', 'ent_import_lot_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_retirer_fk_simple('ent_import_ligne', 'lot_id');
SELECT ent_d036_contrainte('ent_import_ligne', 'ent_import_ligne_lot_fk',
    'FOREIGN KEY (etablissement_id, lot_id) REFERENCES ent_import_lot (etablissement_id, id) ON DELETE CASCADE');
