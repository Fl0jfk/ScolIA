-- Socle 2 (l'élève), lot E6 : ce que la fusion (292) a le droit de faire aux tables gardées.
--
-- Quatre tables du socle 2 refusent par déclencheur qu'on change l'élève d'une ligne
-- (scolarisation, identifiant, capacité) ou réagissent à un UPDATE par un pont vers d'anciennes
-- tables (options). Une fusion doit pouvoir REPOINTER `eleve_id` — et seulement cela — sans
-- rien ouvrir d'autre. Le drapeau `ent.fusion_eleve` (local à la transaction, posé par le seul
-- code de fusion/défusion) autorise ce repointage ET exige que tout le reste de la ligne soit
-- identique. Hors fusion, le comportement est strictement celui d'avant (mêmes gardes).
--
-- Limite assumée (comme les gardes d'identité de la migration 280) : ce drapeau protège contre
-- l'erreur de code, pas contre un compte applicatif propriétaire des tables ni une injection SQL.

CREATE OR REPLACE FUNCTION ent_fusion_en_cours() RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT coalesce(current_setting('ent.fusion_eleve', true), '') = 'on'
$$;

-- Scolarisation : élève et identifiant immuables… sauf repointage de fusion.
CREATE OR REPLACE FUNCTION ent_eleve_scolarisation_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'scolarisation : ne se supprime pas (la clôturer)' USING ERRCODE = '42501';
    END IF;
    IF ent_fusion_en_cours() AND (to_jsonb(NEW) - 'eleve_id') = (to_jsonb(OLD) - 'eleve_id') THEN
        RETURN NEW;
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.cree_le)
       IS DISTINCT FROM (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.cree_le) THEN
        RAISE EXCEPTION 'scolarisation : élève et identifiant immuables' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;

-- Identifiant : élève, système, valeur et provenance immuables… sauf repointage de fusion.
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_immuable() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'identifiant : une ligne ne se supprime pas (révoquer)' USING ERRCODE = '42501';
    END IF;
    IF ent_fusion_en_cours() AND (to_jsonb(NEW) - 'eleve_id') = (to_jsonb(OLD) - 'eleve_id') THEN
        RETURN NEW;
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.systeme, NEW.valeur, NEW.provenance, NEW.valide_du,
        NEW.enregistre_le, NEW.enregistre_par)
       IS DISTINCT FROM
       (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.systeme, OLD.valeur, OLD.provenance, OLD.valide_du,
        OLD.enregistre_le, OLD.enregistre_par) THEN
        RAISE EXCEPTION 'identifiant : élève, système, valeur et provenance sont immuables'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;

-- Capacité : une mesure est immuable (seule sa levée se pose)… sauf repointage de fusion.
CREATE OR REPLACE FUNCTION ent_eleve_capacite_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'capacité : une mesure ne se supprime pas (la lever)' USING ERRCODE = '42501';
    END IF;
    IF ent_fusion_en_cours() AND (to_jsonb(NEW) - 'eleve_id') = (to_jsonb(OLD) - 'eleve_id') THEN
        RETURN NEW;
    END IF;
    IF OLD.levee_le IS NULL AND NEW.levee_le IS NOT NULL
       AND (to_jsonb(NEW) - 'levee_le' - 'levee_par' - 'au') = (to_jsonb(OLD) - 'levee_le' - 'levee_par' - 'au') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'capacité : une mesure est immuable (déclarer ou lever)' USING ERRCODE = '42501';
END $$;

-- Options : le pont vers les affectations ne doit pas rejouer pendant une fusion (les
-- affectations sont déplacées par la fusion elle-même, table par table).
CREATE OR REPLACE FUNCTION ent_eleve_option_pont() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF ent_fusion_en_cours() THEN
        RETURN NULL;
    END IF;
    IF TG_TABLE_NAME = 'ent_eleve_option' THEN
        PERFORM ent_eleve_option_vers_affectation(NEW);
    ELSE
        PERFORM ent_eleve_option_annee_vers_affectation(NEW);
    END IF;
    RETURN NULL;
END $$;
