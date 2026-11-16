-- Socle 18, lot FO0 — migration 2360 : correctifs urgents des formulaires, autorisations et consentements
-- (`docs/socles/18_FORMULAIRES.md` §2.2, §29 , défauts FD-01, FD-02, FD-05, FD-11, FD-12, FD-14, FD-15 ,
-- règles R-FC13, R-FC19, R-FC21, R-FC44 , actions FRM-103, FRM-271, FRM-272, FRM-273).
--
-- Additif, rejouable. Aucun JSON, aucune date UTC (`ent_aujourdhui_etab`). Aucune migration antérieure modifiée.
--
-- 1. Qualification d'une nouvelle version (R-FC19) : `changement` ∈ forme | fond | restriction (NULL pour la
--    version 1) , l'acteur de la publication et de l'archivage sont journalisés (FD-11).
-- 2. Capacité de décider seul (FD-05, R-FC13) : fonction canonique `ent_eleve_decide_seul` (majorité par
--    `ent_eleve_majorite_le`, émancipation, tutelle , date contestée ou approximative ⇒ majorité non présumée).
-- 3. Immuabilité en base (FD-12, FD-14, FD-15, R-FC21, R-FC44) : une décision ne s'écrit qu'une fois , un texte publié
--    ne change plus , un numéro de version ne recule pas , la nature d'un objet ne change jamais.

-- 1. Version et acteurs ------------------------------------------------------------------------
ALTER TABLE ent_formulaire ADD COLUMN IF NOT EXISTS changement TEXT;
ALTER TABLE ent_formulaire ADD COLUMN IF NOT EXISTS publie_par UUID;
ALTER TABLE ent_formulaire ADD COLUMN IF NOT EXISTS archive_par UUID;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_formulaire_changement_chk') THEN
        ALTER TABLE ent_formulaire ADD CONSTRAINT ent_formulaire_changement_chk
            CHECK (changement IS NULL OR (changement IN ('forme', 'fond', 'restriction') AND version > 1));
    END IF;
END $$;

-- 2. Capacité de décider seul ------------------------------------------------------------------
-- Vrai quand l'élève décide lui-même, à la date : émancipé (même mineur), ou majeur dont la date de naissance est
-- certaine (précision « jour », non contestée) et qui n'est pas sous tutelle. Sinon (mineur, majeur sous tutelle,
-- naissance approximative ou contestée) les titulaires décident. La curatelle laisse la décision à l'élève.
CREATE OR REPLACE FUNCTION ent_eleve_decide_seul(p_etab UUID, p_eleve UUID)
RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT CASE
        WHEN EXISTS (SELECT 1 FROM ent_eleve_capacite c
                      WHERE c.etablissement_id = p_etab AND c.eleve_id = p_eleve
                        AND c.nature = 'emancipation' AND c.levee_le IS NULL
                        AND c.du <= ent_aujourdhui_etab(p_etab)
                        AND (c.au IS NULL OR c.au >= ent_aujourdhui_etab(p_etab))) THEN true
        WHEN EXISTS (SELECT 1 FROM ent_eleve_capacite c
                      WHERE c.etablissement_id = p_etab AND c.eleve_id = p_eleve
                        AND c.nature = 'tutelle' AND c.levee_le IS NULL
                        AND c.du <= ent_aujourdhui_etab(p_etab)
                        AND (c.au IS NULL OR c.au >= ent_aujourdhui_etab(p_etab))) THEN false
        ELSE coalesce(
            (SELECT ent_eleve_majorite_le(e.date_naissance, e.date_naissance_precision, e.date_naissance_statut)
                    <= ent_aujourdhui_etab(p_etab)
               FROM ent_eleve e WHERE e.etablissement_id = p_etab AND e.id = p_eleve),
            false)
    END
$$;

-- 3. Immuabilité -------------------------------------------------------------------------------
-- Décisions : ajout seul. Seule exception, la fusion de deux fiches élève, qui réaffecte `eleve_id` (rien d'autre ne
-- bouge). Purge volontaire d'un établissement : `SET LOCAL ent.purge_etablissement = 'on'` (jamais posé par l'application).
CREATE OR REPLACE FUNCTION ent_formulaire_reponse_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE'
       AND NEW.id = OLD.id
       AND NEW.etablissement_id = OLD.etablissement_id
       AND NEW.formulaire_id = OLD.formulaire_id
       AND NEW.objet IS NOT DISTINCT FROM OLD.objet
       AND NEW.version = OLD.version
       AND NEW.signataire_user_id = OLD.signataire_user_id
       AND NEW.qualite = OLD.qualite
       AND NEW.decision = OLD.decision
       AND NEW.commentaire IS NOT DISTINCT FROM OLD.commentaire
       AND NEW.empreinte = OLD.empreinte
       AND NEW.decide_le = OLD.decide_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_formulaire_reponse : une décision est en ajout seul (écrire une nouvelle décision)'
        USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_formulaire_reponse_garde ON ent_formulaire_reponse;
CREATE TRIGGER ent_formulaire_reponse_garde
    BEFORE UPDATE OR DELETE ON ent_formulaire_reponse
    FOR EACH ROW EXECUTE FUNCTION ent_formulaire_reponse_garde();

-- Formulaires : un brouillon se modifie et se supprime , un texte publié est figé (seuls le statut et l'archivage
-- évoluent : publié → archivé) , la nature d'un objet ne change jamais (FD-15) , un numéro de version ne recule
-- pas (FD-14).
CREATE OR REPLACE FUNCTION ent_formulaire_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    autre_nature TEXT;
    plus_recent INTEGER;
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF OLD.statut = 'brouillon' THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION 'ent_formulaire : seul un brouillon jamais publié se supprime (archiver)' USING ERRCODE = '42501';
    END IF;

    IF TG_OP = 'INSERT' THEN
        IF NEW.objet IS NOT NULL THEN
            SELECT f.type INTO autre_nature FROM ent_formulaire f
             WHERE f.etablissement_id = NEW.etablissement_id AND f.objet = NEW.objet LIMIT 1;
            IF autre_nature IS NOT NULL AND autre_nature <> NEW.type THEN
                RAISE EXCEPTION 'ent_formulaire : la nature d''un objet ne change jamais (%)', NEW.objet
                    USING ERRCODE = '23514';
            END IF;
        END IF;
        RETURN NEW;
    END IF;

    -- UPDATE
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.type IS DISTINCT FROM OLD.type
       OR NEW.objet IS DISTINCT FROM OLD.objet
       OR NEW.version IS DISTINCT FROM OLD.version
       OR NEW.cree_par IS DISTINCT FROM OLD.cree_par THEN
        RAISE EXCEPTION 'ent_formulaire : nature, objet, numéro de version et auteur sont immuables' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut <> 'brouillon' THEN
        IF NEW.titre IS DISTINCT FROM OLD.titre
           OR NEW.texte IS DISTINCT FROM OLD.texte
           OR NEW.audience IS DISTINCT FROM OLD.audience
           OR NEW.cible_ids IS DISTINCT FROM OLD.cible_ids
           OR NEW.changement IS DISTINCT FROM OLD.changement
           OR NEW.publie_le IS DISTINCT FROM OLD.publie_le
           OR NEW.publie_par IS DISTINCT FROM OLD.publie_par THEN
            RAISE EXCEPTION 'ent_formulaire : un texte publié est immuable (créer une nouvelle version)'
                USING ERRCODE = '42501';
        END IF;
        IF OLD.statut = 'archive' AND NEW.statut <> 'archive' THEN
            RAISE EXCEPTION 'ent_formulaire : un formulaire archivé ne se rouvre pas' USING ERRCODE = '42501';
        END IF;
        IF OLD.statut = 'publie' AND NEW.statut = 'brouillon' THEN
            RAISE EXCEPTION 'ent_formulaire : un formulaire publié ne redevient pas brouillon' USING ERRCODE = '42501';
        END IF;
    END IF;
    IF OLD.statut = 'brouillon' AND NEW.statut = 'publie' AND NEW.objet IS NOT NULL THEN
        SELECT max(f.version) INTO plus_recent FROM ent_formulaire f
         WHERE f.etablissement_id = NEW.etablissement_id AND f.objet = NEW.objet
           AND f.id <> NEW.id AND f.statut IN ('publie', 'archive');
        IF plus_recent IS NOT NULL AND plus_recent > NEW.version THEN
            RAISE EXCEPTION 'ent_formulaire : une version plus récente (%) existe déjà pour %', plus_recent, NEW.objet
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_formulaire_garde ON ent_formulaire;
CREATE TRIGGER ent_formulaire_garde
    BEFORE INSERT OR UPDATE OR DELETE ON ent_formulaire
    FOR EACH ROW EXECUTE FUNCTION ent_formulaire_garde();
