-- Socle 18, lot FO2/FO3 — migration 2365 : cibles résolues, soumissions et valeurs typées
-- (`docs/socles/18_FORMULAIRES.md` §11.3, §11.4, §27 , règles R-FC2, R-FC16 , actions FRM-110, FRM-130, FRM-160).
--
-- Additif, rejouable, sans JSON (D-036) et sans date UTC.
--
-- * `ent_frm_cible` : l'audience résolue à la publication (un élève par ligne). Une famille ou un élève ne répond qu'à
--   un formulaire dont l'élève est cible.
-- * `ent_frm_soumission` : une réponse envoyée. Elle ne se modifie pas : une correction est une nouvelle soumission qui
--   remplace la précédente (qui garde sa date).
-- * `ent_frm_valeur` : une ligne par réponse, des colonnes typées, **une seule valeur scalaire** renseignée et cohérente
--   avec le type de la question (déclencheur). Ce n'est pas une table clé-valeur libre : chaque ligne référence sa
--   question par clé étrangère et toute requête (compter les choix, moyenner une échelle) est du SQL ordinaire.

CREATE TABLE IF NOT EXISTS ent_frm_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    formulaire_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    ajoute_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retire_le TIMESTAMPTZ,
    CONSTRAINT ent_frm_cible_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_cible_uq UNIQUE (etablissement_id, formulaire_id, eleve_id),
    CONSTRAINT ent_frm_cible_formulaire_cfk FOREIGN KEY (etablissement_id, formulaire_id)
        REFERENCES ent_frm_formulaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_cible_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_frm_cible_eleve_idx ON ent_frm_cible (etablissement_id, eleve_id) WHERE retire_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_frm_soumission (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    formulaire_id UUID NOT NULL,
    version_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    repondant_user_id UUID NOT NULL,
    qualite TEXT NOT NULL CHECK (qualite IN ('titulaire', 'eleve', 'eleve_majeur')),
    canal TEXT NOT NULL DEFAULT 'en_ligne' CHECK (canal IN ('en_ligne', 'guichet', 'papier')),
    saisie_par_user_id UUID,
    statut TEXT NOT NULL DEFAULT 'envoyee' CHECK (statut IN ('envoyee', 'remplacee', 'annulee_par_etablissement')),
    envoyee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    remplace_id UUID,
    CONSTRAINT ent_frm_soumission_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_soumission_version_uq UNIQUE (etablissement_id, version_id, id),
    CONSTRAINT ent_frm_soumission_version_cfk FOREIGN KEY (etablissement_id, formulaire_id, version_id)
        REFERENCES ent_frm_version (etablissement_id, formulaire_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_soumission_cible_cfk FOREIGN KEY (etablissement_id, formulaire_id, eleve_id)
        REFERENCES ent_frm_cible (etablissement_id, formulaire_id, eleve_id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_soumission_remplace_cfk FOREIGN KEY (etablissement_id, remplace_id)
        REFERENCES ent_frm_soumission (etablissement_id, id) ON DELETE RESTRICT
);
-- Une seule réponse en cours par répondant et par élève (la précédente est « remplacée » en une transaction).
CREATE UNIQUE INDEX IF NOT EXISTS ent_frm_soumission_en_cours_uq
    ON ent_frm_soumission (etablissement_id, formulaire_id, eleve_id, repondant_user_id) WHERE statut = 'envoyee';
CREATE INDEX IF NOT EXISTS ent_frm_soumission_repondant_idx ON ent_frm_soumission (etablissement_id, repondant_user_id, envoyee_le DESC);

CREATE TABLE IF NOT EXISTS ent_frm_valeur (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    soumission_id UUID NOT NULL,
    version_id UUID NOT NULL,
    question_id UUID NOT NULL,
    option_id UUID,
    ligne_id UUID,
    rang SMALLINT CHECK (rang IS NULL OR rang BETWEEN 1 AND 100),
    valeur_texte TEXT CHECK (valeur_texte IS NULL OR char_length(valeur_texte) <= 4000),
    valeur_nombre NUMERIC,
    valeur_date DATE,
    valeur_heure TIME,
    CONSTRAINT ent_frm_valeur_une_seule_chk CHECK (num_nonnulls(valeur_texte, valeur_nombre, valeur_date, valeur_heure) <= 1),
    CONSTRAINT ent_frm_valeur_soumission_cfk FOREIGN KEY (etablissement_id, version_id, soumission_id)
        REFERENCES ent_frm_soumission (etablissement_id, version_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_valeur_question_cfk FOREIGN KEY (etablissement_id, version_id, question_id)
        REFERENCES ent_frm_question (etablissement_id, version_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_valeur_option_cfk FOREIGN KEY (etablissement_id, option_id)
        REFERENCES ent_frm_option (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_valeur_ligne_cfk FOREIGN KEY (etablissement_id, ligne_id)
        REFERENCES ent_frm_ligne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_frm_valeur_soumission_idx ON ent_frm_valeur (etablissement_id, soumission_id);
CREATE INDEX IF NOT EXISTS ent_frm_valeur_question_idx ON ent_frm_valeur (etablissement_id, question_id);

-- Cohérence d'une valeur avec le type de sa question : une seule forme de valeur par type, option et ligne de la bonne question.
CREATE OR REPLACE FUNCTION ent_frm_valeur_coherence() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    q_type TEXT;
    q_max INTEGER;
    o_question UUID;
    l_question UUID;
BEGIN
    SELECT type, longueur_max INTO q_type, q_max FROM ent_frm_question WHERE etablissement_id = NEW.etablissement_id AND id = NEW.question_id;
    IF NEW.option_id IS NOT NULL THEN
        SELECT question_id INTO o_question FROM ent_frm_option WHERE etablissement_id = NEW.etablissement_id AND id = NEW.option_id;
        IF o_question IS DISTINCT FROM NEW.question_id THEN
            RAISE EXCEPTION 'ent_frm_valeur : option d''une autre question' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NEW.ligne_id IS NOT NULL THEN
        SELECT question_id INTO l_question FROM ent_frm_ligne WHERE etablissement_id = NEW.etablissement_id AND id = NEW.ligne_id;
        IF l_question IS DISTINCT FROM NEW.question_id THEN
            RAISE EXCEPTION 'ent_frm_valeur : ligne d''une autre question' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF q_type IN ('texte_court', 'texte_long') THEN
        IF NEW.valeur_texte IS NULL OR NEW.option_id IS NOT NULL OR NEW.ligne_id IS NOT NULL OR NEW.rang IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse texte porte un texte et rien d''autre' USING ERRCODE = '23514';
        END IF;
        IF char_length(NEW.valeur_texte) > COALESCE(q_max, CASE WHEN q_type = 'texte_court' THEN 200 ELSE 4000 END) THEN
            RAISE EXCEPTION 'ent_frm_valeur : texte trop long' USING ERRCODE = '23514';
        END IF;
    ELSIF q_type IN ('nombre', 'echelle', 'prise_connaissance') THEN
        IF NEW.valeur_nombre IS NULL OR NEW.option_id IS NOT NULL OR NEW.ligne_id IS NOT NULL OR NEW.rang IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse numérique porte un nombre et rien d''autre' USING ERRCODE = '23514';
        END IF;
    ELSIF q_type = 'date' THEN
        IF NEW.valeur_date IS NULL OR NEW.option_id IS NOT NULL OR NEW.ligne_id IS NOT NULL OR NEW.rang IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse date porte une date et rien d''autre' USING ERRCODE = '23514';
        END IF;
    ELSIF q_type = 'heure' THEN
        IF NEW.valeur_heure IS NULL OR NEW.option_id IS NOT NULL OR NEW.ligne_id IS NOT NULL OR NEW.rang IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse heure porte une heure et rien d''autre' USING ERRCODE = '23514';
        END IF;
    ELSIF q_type IN ('choix_unique', 'choix_multiple', 'liste', 'oui_non') THEN
        IF NEW.option_id IS NULL OR num_nonnulls(NEW.valeur_texte, NEW.valeur_nombre, NEW.valeur_date, NEW.valeur_heure) > 0
           OR NEW.ligne_id IS NOT NULL OR NEW.rang IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse à choix porte une option et rien d''autre' USING ERRCODE = '23514';
        END IF;
    ELSIF q_type = 'grille' THEN
        IF NEW.option_id IS NULL OR NEW.ligne_id IS NULL OR num_nonnulls(NEW.valeur_texte, NEW.valeur_nombre, NEW.valeur_date, NEW.valeur_heure) > 0
           OR NEW.rang IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse de grille porte une ligne et une option' USING ERRCODE = '23514';
        END IF;
    ELSIF q_type = 'classement' THEN
        IF NEW.option_id IS NULL OR NEW.rang IS NULL OR num_nonnulls(NEW.valeur_texte, NEW.valeur_nombre, NEW.valeur_date, NEW.valeur_heure) > 0
           OR NEW.ligne_id IS NOT NULL THEN
            RAISE EXCEPTION 'ent_frm_valeur : une réponse de classement porte une option et un rang' USING ERRCODE = '23514';
        END IF;
    ELSE
        RAISE EXCEPTION 'ent_frm_valeur : ce type de question ne porte pas de valeur ici (%)', q_type USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_frm_valeur_coherence ON ent_frm_valeur;
CREATE TRIGGER ent_frm_valeur_coherence BEFORE INSERT ON ent_frm_valeur
    FOR EACH ROW EXECUTE FUNCTION ent_frm_valeur_coherence();

-- Soumissions et valeurs : ajout seul, sauf le passage d'une soumission à « remplacée » ou « annulée par l'établissement ».
CREATE OR REPLACE FUNCTION ent_frm_soumission_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF ent_frm_purge() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.statut = 'envoyee' AND NEW.statut IN ('remplacee', 'annulee_par_etablissement')
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.formulaire_id = OLD.formulaire_id
       AND NEW.version_id = OLD.version_id AND NEW.eleve_id = OLD.eleve_id AND NEW.repondant_user_id = OLD.repondant_user_id
       AND NEW.qualite = OLD.qualite AND NEW.canal = OLD.canal AND NEW.envoyee_le = OLD.envoyee_le
       AND NEW.saisie_par_user_id IS NOT DISTINCT FROM OLD.saisie_par_user_id
       AND NEW.remplace_id IS NOT DISTINCT FROM OLD.remplace_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_frm_soumission : une soumission est en ajout seul' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_frm_soumission_garde ON ent_frm_soumission;
CREATE TRIGGER ent_frm_soumission_garde BEFORE UPDATE OR DELETE ON ent_frm_soumission
    FOR EACH ROW EXECUTE FUNCTION ent_frm_soumission_garde();
DROP TRIGGER IF EXISTS ent_frm_valeur_garde ON ent_frm_valeur;
CREATE TRIGGER ent_frm_valeur_garde BEFORE UPDATE OR DELETE ON ent_frm_valeur
    FOR EACH ROW EXECUTE FUNCTION ent_frm_ajout_seul();

-- Cibles : l'élève ajouté ou retiré par la résolution, jamais supprimé (un élève retiré garde sa ligne datée).
CREATE OR REPLACE FUNCTION ent_frm_cible_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF ent_frm_purge() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_frm_cible : une cible se retire (retire_le), elle ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.formulaire_id = OLD.formulaire_id
       AND NEW.eleve_id = OLD.eleve_id AND NEW.ajoute_le = OLD.ajoute_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_frm_cible : seule la date de retrait change' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_frm_cible_garde ON ent_frm_cible;
CREATE TRIGGER ent_frm_cible_garde BEFORE UPDATE OR DELETE ON ent_frm_cible
    FOR EACH ROW EXECUTE FUNCTION ent_frm_cible_garde();

SELECT ent_rls_poser(ARRAY['ent_frm_cible', 'ent_frm_soumission', 'ent_frm_valeur']);
