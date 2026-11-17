-- Socle 18, lot FO2 — migration 2364 : le moteur de formulaires (création par les personnels)
-- (`docs/socles/18_FORMULAIRES.md` §11, §21.2, §27 , règles R-FC22, R-FC33 , actions FRM-120 à FRM-139, FRM-149).
--
-- Additif, rejouable, sans JSON (D-036) et sans date UTC. Plage 2360-2379 : 2364 formulaires et questions · 2365 cibles,
-- soumissions et valeurs · 2366 désaccords et lien décision-soumission · 2367 dossier de preuve (brique de signature).
--
-- * Un formulaire a des **versions** : une version publiée est immuable (questions, options, conditions figées). Une
--   modification après publication passe par une nouvelle version en brouillon (un seul brouillon à la fois).
-- * La seule retouche permise sur une version publiée est la correction de **forme** d'un libellé ou d'une aide, tracée
--   dans `ent_frm_correction` (en ajout seul) et autorisée par le service seul (réglage de transaction).
-- * Une condition d'affichage ne dépend jamais d'une question située après elle : la boucle est donc impossible, et la
--   base le vérifie (déclencheur) en plus du code.
-- * L'audience est écrite en critères (lignes, pas de JSON) puis résolue en cibles à la publication (migration 2365).

CREATE TABLE IF NOT EXISTS ent_frm_formulaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    titre TEXT NOT NULL CHECK (char_length(btrim(titre)) BETWEEN 1 AND 160),
    description TEXT CHECK (description IS NULL OR char_length(description) <= 2000),
    nature TEXT NOT NULL CHECK (nature IN ('information', 'collecte', 'mixte')),
    unite TEXT NOT NULL CHECK (unite IN ('eleve_un_titulaire', 'eleve_chaque_titulaire', 'foyer', 'personne', 'eleve_lui_meme')),
    confidentialite TEXT NOT NULL DEFAULT 'nominatif' CHECK (confidentialite IN ('nominatif', 'confidentiel')),
    destinataire_confidentiel_user_id UUID,
    statut TEXT NOT NULL DEFAULT 'brouillon' CHECK (statut IN ('brouillon', 'soumis', 'publie', 'clos', 'archive')),
    modele_code TEXT CHECK (modele_code IS NULL OR modele_code ~ '^[a-z][a-z0-9_]{1,39}$'),
    ouverture TIMESTAMPTZ,
    echeance TIMESTAMPTZ,
    relances TEXT NOT NULL DEFAULT 'j7_j3_j1' CHECK (relances IN ('j7_j3_j1', 'j3_j1', 'aucune')),
    -- Un formulaire de santé relève du socle 34 et reste désactivé sans hébergeur HDS (D-034).
    sante BOOLEAN NOT NULL DEFAULT false CHECK (sante = false),
    -- Effectif de l'audience résolue au dépôt (validation au-delà du seuil).
    audience_nb INTEGER CHECK (audience_nb IS NULL OR audience_nb >= 0),
    auteur_user_id UUID NOT NULL,
    soumis_le TIMESTAMPTZ,
    valide_par UUID,
    valide_le TIMESTAMPTZ,
    publie_par UUID,
    publie_le TIMESTAMPTZ,
    clos_le TIMESTAMPTZ,
    archive_par UUID,
    archive_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_frm_formulaire_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_formulaire_echeance_chk CHECK (echeance IS NULL OR ouverture IS NULL OR echeance > ouverture),
    CONSTRAINT ent_frm_formulaire_confid_chk CHECK ((confidentialite = 'confidentiel') = (destinataire_confidentiel_user_id IS NOT NULL)),
    CONSTRAINT ent_frm_formulaire_publie_chk CHECK (statut NOT IN ('publie', 'clos') OR (publie_par IS NOT NULL AND publie_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_frm_formulaire_etab_idx ON ent_frm_formulaire (etablissement_id, statut, cree_le DESC);
CREATE INDEX IF NOT EXISTS ent_frm_formulaire_auteur_idx ON ent_frm_formulaire (etablissement_id, auteur_user_id);

-- Co-auteurs (FRM-139) : lisent et modifient le brouillon, ne valident jamais leur propre travail.
CREATE TABLE IF NOT EXISTS ent_frm_auteur (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    formulaire_id UUID NOT NULL,
    user_id UUID NOT NULL,
    ajoute_par UUID NOT NULL,
    ajoute_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (formulaire_id, user_id),
    CONSTRAINT ent_frm_auteur_formulaire_cfk FOREIGN KEY (etablissement_id, formulaire_id)
        REFERENCES ent_frm_formulaire (etablissement_id, id) ON DELETE RESTRICT
);

-- Versions : `publiee_le` nul = brouillon (un seul par formulaire).
CREATE TABLE IF NOT EXISTS ent_frm_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    formulaire_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero >= 1),
    -- SHA-256 hexadécimal de toute la structure, calculé à la publication seulement.
    empreinte TEXT CHECK (empreinte IS NULL OR empreinte ~ '^[0-9a-f]{64}$'),
    type_changement TEXT CHECK (type_changement IS NULL OR type_changement IN ('forme', 'fond')),
    publiee_le TIMESTAMPTZ,
    publiee_par UUID,
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_frm_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_version_formulaire_id_uq UNIQUE (etablissement_id, formulaire_id, id),
    CONSTRAINT ent_frm_version_numero_uq UNIQUE (etablissement_id, formulaire_id, numero),
    CONSTRAINT ent_frm_version_formulaire_cfk FOREIGN KEY (etablissement_id, formulaire_id)
        REFERENCES ent_frm_formulaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_version_publiee_chk CHECK ((publiee_le IS NULL) = (empreinte IS NULL) AND (publiee_le IS NULL) = (publiee_par IS NULL)),
    CONSTRAINT ent_frm_version_changement_chk CHECK (type_changement IS NULL OR numero > 1)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_frm_version_brouillon_uq
    ON ent_frm_version (etablissement_id, formulaire_id) WHERE publiee_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_frm_section (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    ordre INTEGER NOT NULL CHECK (ordre BETWEEN 1 AND 100),
    titre TEXT NOT NULL CHECK (char_length(btrim(titre)) BETWEEN 1 AND 160),
    aide TEXT CHECK (aide IS NULL OR char_length(aide) <= 1000),
    CONSTRAINT ent_frm_section_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_section_version_id_uq UNIQUE (etablissement_id, version_id, id),
    CONSTRAINT ent_frm_section_ordre_uq UNIQUE (etablissement_id, version_id, ordre),
    CONSTRAINT ent_frm_section_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_frm_version (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_frm_question (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    section_id UUID NOT NULL,
    ordre INTEGER NOT NULL CHECK (ordre BETWEEN 1 AND 200),
    type TEXT NOT NULL CHECK (type IN (
        'texte_court', 'texte_long', 'choix_unique', 'choix_multiple', 'liste', 'oui_non', 'nombre', 'date', 'heure',
        'echelle', 'grille', 'classement', 'fichier', 'accord', 'prise_connaissance', 'creneau', 'choix_tarifie', 'information')),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 500),
    aide TEXT CHECK (aide IS NULL OR char_length(aide) <= 1000),
    obligatoire BOOLEAN NOT NULL DEFAULT false,
    -- C0 ordinaire, C1 personnelle, C2 sensible (art. 9, direction seule). C3 (santé) est refusée avant l'écriture.
    sensibilite TEXT NOT NULL DEFAULT 'C0' CHECK (sensibilite IN ('C0', 'C1', 'C2')),
    min_nombre NUMERIC,
    max_nombre NUMERIC,
    longueur_max INTEGER CHECK (longueur_max IS NULL OR longueur_max BETWEEN 1 AND 4000),
    -- Question « accord » : l'objet d'accord du socle 18 (le même moteur, §6.2).
    objet_accord_id UUID,
    CONSTRAINT ent_frm_question_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_question_version_id_uq UNIQUE (etablissement_id, version_id, id),
    CONSTRAINT ent_frm_question_ordre_uq UNIQUE (etablissement_id, section_id, ordre),
    CONSTRAINT ent_frm_question_section_cfk FOREIGN KEY (etablissement_id, version_id, section_id)
        REFERENCES ent_frm_section (etablissement_id, version_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_question_objet_cfk FOREIGN KEY (etablissement_id, objet_accord_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_question_accord_chk CHECK ((type = 'accord') = (objet_accord_id IS NOT NULL)),
    CONSTRAINT ent_frm_question_bornes_chk CHECK (min_nombre IS NULL OR max_nombre IS NULL OR min_nombre <= max_nombre),
    CONSTRAINT ent_frm_question_sans_reponse_chk CHECK (type <> 'information' OR NOT obligatoire)
);
CREATE INDEX IF NOT EXISTS ent_frm_question_version_idx ON ent_frm_question (etablissement_id, version_id);
CREATE INDEX IF NOT EXISTS ent_frm_question_objet_idx ON ent_frm_question (etablissement_id, objet_accord_id) WHERE objet_accord_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS ent_frm_option (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    question_id UUID NOT NULL,
    ordre INTEGER NOT NULL CHECK (ordre BETWEEN 1 AND 100),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 200),
    CONSTRAINT ent_frm_option_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_option_ordre_uq UNIQUE (etablissement_id, question_id, ordre),
    CONSTRAINT ent_frm_option_question_cfk FOREIGN KEY (etablissement_id, question_id)
        REFERENCES ent_frm_question (etablissement_id, id) ON DELETE RESTRICT
);

-- Lignes d'une grille (les colonnes sont les options de la question).
CREATE TABLE IF NOT EXISTS ent_frm_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    question_id UUID NOT NULL,
    ordre INTEGER NOT NULL CHECK (ordre BETWEEN 1 AND 100),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 200),
    CONSTRAINT ent_frm_ligne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_frm_ligne_ordre_uq UNIQUE (etablissement_id, question_id, ordre),
    CONSTRAINT ent_frm_ligne_question_cfk FOREIGN KEY (etablissement_id, question_id)
        REFERENCES ent_frm_question (etablissement_id, id) ON DELETE RESTRICT
);

-- Condition d'affichage : la question n'apparait que si la réponse à `depend_de` est (ou n'est pas) l'option donnée.
CREATE TABLE IF NOT EXISTS ent_frm_condition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    question_id UUID NOT NULL,
    depend_de_question_id UUID NOT NULL,
    option_id UUID NOT NULL,
    operateur TEXT NOT NULL CHECK (operateur IN ('egal', 'different')),
    CONSTRAINT ent_frm_condition_uq UNIQUE (etablissement_id, question_id, depend_de_question_id, option_id, operateur),
    CONSTRAINT ent_frm_condition_question_cfk FOREIGN KEY (etablissement_id, question_id)
        REFERENCES ent_frm_question (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_condition_depend_cfk FOREIGN KEY (etablissement_id, depend_de_question_id)
        REFERENCES ent_frm_question (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_condition_option_cfk FOREIGN KEY (etablissement_id, option_id)
        REFERENCES ent_frm_option (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_condition_soi_chk CHECK (question_id <> depend_de_question_id)
);

-- Audience : critères en lignes (même forme que les campagnes). `inclure` ajoute, `exclure` retire (exclus fondus).
CREATE TABLE IF NOT EXISTS ent_frm_audience_critere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    formulaire_id UUID NOT NULL,
    mode TEXT NOT NULL DEFAULT 'inclure' CHECK (mode IN ('inclure', 'exclure')),
    type TEXT NOT NULL CHECK (type IN ('etablissement', 'niveau', 'classe', 'groupe', 'eleve')),
    cible_id UUID,
    CONSTRAINT ent_frm_audience_cible_chk CHECK ((type = 'etablissement') = (cible_id IS NULL)),
    CONSTRAINT ent_frm_audience_critere_uq UNIQUE (etablissement_id, formulaire_id, mode, type, cible_id),
    CONSTRAINT ent_frm_audience_formulaire_cfk FOREIGN KEY (etablissement_id, formulaire_id)
        REFERENCES ent_frm_formulaire (etablissement_id, id) ON DELETE RESTRICT
);

-- Corrections de forme d'une version publiée (FRM-135) : libellé ou aide, ancienne valeur conservée, ajout seul.
CREATE TABLE IF NOT EXISTS ent_frm_correction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    formulaire_id UUID NOT NULL,
    question_id UUID NOT NULL,
    ancien_libelle TEXT NOT NULL,
    nouveau_libelle TEXT NOT NULL,
    ancienne_aide TEXT,
    nouvelle_aide TEXT,
    par_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_frm_correction_formulaire_cfk FOREIGN KEY (etablissement_id, formulaire_id)
        REFERENCES ent_frm_formulaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_frm_correction_question_cfk FOREIGN KEY (etablissement_id, question_id)
        REFERENCES ent_frm_question (etablissement_id, id) ON DELETE RESTRICT
);

-- Journal des formulaires (ajout seul) : codes courts, jamais une réponse.
CREATE TABLE IF NOT EXISTS ent_frm_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    acte TEXT NOT NULL CHECK (acte IN (
        'cree', 'modele', 'modifie', 'soumis', 'valide', 'refuse_validation', 'publie', 'version', 'correction', 'prolonge',
        'clos', 'rouvert', 'archive', 'duplique', 'co_auteur', 'supprime', 'reponse')),
    formulaire_id UUID,
    version_id UUID,
    acteur_user_id UUID NOT NULL,
    nb INTEGER CHECK (nb IS NULL OR nb >= 0),
    detail TEXT CHECK (detail IS NULL OR detail ~ '^[a-z0-9_.:-]{1,80}$'),
    le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_frm_journal_etab_idx ON ent_frm_journal (etablissement_id, le DESC);
CREATE INDEX IF NOT EXISTS ent_frm_journal_formulaire_idx ON ent_frm_journal (etablissement_id, formulaire_id, le DESC);

-- Gardes ---------------------------------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION ent_frm_purge() RETURNS boolean
LANGUAGE sql STABLE AS $$ SELECT current_setting('ent.purge_etablissement', true) = 'on' $$;

-- Formulaire : nature, unité et confidentialité se verrouillent à la première publication. Un brouillon jamais publié est
-- le seul à se supprimer.
CREATE OR REPLACE FUNCTION ent_frm_formulaire_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF ent_frm_purge() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF OLD.statut = 'brouillon' AND OLD.publie_le IS NULL THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION 'ent_frm_formulaire : seul un brouillon jamais publié se supprime' USING ERRCODE = '42501';
    END IF;
    IF OLD.publie_le IS NOT NULL THEN
        IF NEW.nature IS DISTINCT FROM OLD.nature OR NEW.unite IS DISTINCT FROM OLD.unite
           OR NEW.confidentialite IS DISTINCT FROM OLD.confidentialite
           OR NEW.destinataire_confidentiel_user_id IS DISTINCT FROM OLD.destinataire_confidentiel_user_id
           OR NEW.auteur_user_id IS DISTINCT FROM OLD.auteur_user_id OR NEW.publie_le IS DISTINCT FROM OLD.publie_le THEN
            RAISE EXCEPTION 'ent_frm_formulaire : nature, unité, confidentialité et auteur ne changent plus après la publication'
                USING ERRCODE = '42501';
        END IF;
    END IF;
    IF OLD.statut = 'archive' AND NEW.statut <> 'archive' THEN
        RAISE EXCEPTION 'ent_frm_formulaire : un formulaire archivé reste archivé' USING ERRCODE = '42501';
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_frm_formulaire_garde ON ent_frm_formulaire;
CREATE TRIGGER ent_frm_formulaire_garde BEFORE UPDATE OR DELETE ON ent_frm_formulaire
    FOR EACH ROW EXECUTE FUNCTION ent_frm_formulaire_garde();

-- Version : publiée, elle est immuable. Le numéro d'une version publiée dépasse celui des versions déjà publiées.
CREATE OR REPLACE FUNCTION ent_frm_version_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    plus_haut INTEGER;
    cycle_trouve BOOLEAN;
BEGIN
    IF ent_frm_purge() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF OLD.publiee_le IS NULL THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION 'ent_frm_version : une version publiée ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.publiee_le IS NOT NULL THEN
        RAISE EXCEPTION 'ent_frm_version : une version publiée est immuable (créer une nouvelle version)' USING ERRCODE = '42501';
    END IF;
    IF NEW.publiee_le IS NOT NULL THEN
        SELECT max(numero) INTO plus_haut FROM ent_frm_version
         WHERE etablissement_id = NEW.etablissement_id AND formulaire_id = NEW.formulaire_id
           AND publiee_le IS NOT NULL AND id <> NEW.id;
        IF plus_haut IS NOT NULL AND plus_haut >= NEW.numero THEN
            RAISE EXCEPTION 'ent_frm_version : numéro % non supérieur à la dernière version publiée (%)', NEW.numero, plus_haut
                USING ERRCODE = '23514';
        END IF;
        -- Aucune boucle de conditions dans la version publiée (la base le vérifie aussi).
        WITH RECURSIVE arete AS (
            SELECT c.question_id AS de, c.depend_de_question_id AS vers
              FROM ent_frm_condition c
              JOIN ent_frm_question q ON q.etablissement_id = c.etablissement_id AND q.id = c.question_id
             WHERE q.etablissement_id = NEW.etablissement_id AND q.version_id = NEW.id
        ), chemin(origine, courant, profondeur) AS (
            SELECT de, vers, 1 FROM arete
            UNION ALL
            SELECT p.origine, a.vers, p.profondeur + 1 FROM chemin p JOIN arete a ON a.de = p.courant
             WHERE p.profondeur < 300
        )
        SELECT EXISTS (SELECT 1 FROM chemin WHERE origine = courant) INTO cycle_trouve;
        IF cycle_trouve THEN
            RAISE EXCEPTION 'ent_frm_version : boucle dans les conditions d''affichage' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_frm_version_garde ON ent_frm_version;
CREATE TRIGGER ent_frm_version_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_frm_version
    FOR EACH ROW EXECUTE FUNCTION ent_frm_version_garde();

-- Sections, questions, options, lignes, conditions : modifiables tant que leur version est un brouillon, figées ensuite.
-- Exception : la correction de forme (libellé ou aide d'une question) que le service autorise pour la durée d'une transaction.
CREATE OR REPLACE FUNCTION ent_frm_contenu_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    version_cible UUID;
    publiee TIMESTAMPTZ;
    ligne RECORD;
BEGIN
    IF ent_frm_purge() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    ligne := CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    IF TG_TABLE_NAME = 'ent_frm_section' OR TG_TABLE_NAME = 'ent_frm_question' THEN
        version_cible := ligne.version_id;
    ELSIF TG_TABLE_NAME = 'ent_frm_option' OR TG_TABLE_NAME = 'ent_frm_ligne' THEN
        SELECT version_id INTO version_cible FROM ent_frm_question WHERE etablissement_id = ligne.etablissement_id AND id = ligne.question_id;
    ELSE
        SELECT version_id INTO version_cible FROM ent_frm_question WHERE etablissement_id = ligne.etablissement_id AND id = ligne.question_id;
    END IF;
    SELECT publiee_le INTO publiee FROM ent_frm_version WHERE etablissement_id = ligne.etablissement_id AND id = version_cible;
    IF publiee IS NULL THEN
        RETURN ligne;
    END IF;
    IF TG_OP = 'UPDATE' AND TG_TABLE_NAME = 'ent_frm_question' AND current_setting('ent.frm_correction', true) = 'on' THEN
        IF NEW.id = OLD.id AND NEW.version_id = OLD.version_id AND NEW.section_id = OLD.section_id AND NEW.ordre = OLD.ordre
           AND NEW.type = OLD.type AND NEW.obligatoire = OLD.obligatoire AND NEW.sensibilite = OLD.sensibilite
           AND NEW.min_nombre IS NOT DISTINCT FROM OLD.min_nombre AND NEW.max_nombre IS NOT DISTINCT FROM OLD.max_nombre
           AND NEW.longueur_max IS NOT DISTINCT FROM OLD.longueur_max
           AND NEW.objet_accord_id IS NOT DISTINCT FROM OLD.objet_accord_id THEN
            RETURN NEW;
        END IF;
    END IF;
    RAISE EXCEPTION '% : la version est publiée, ses questions sont figées (créer une nouvelle version)', TG_TABLE_NAME
        USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_frm_section_garde ON ent_frm_section;
CREATE TRIGGER ent_frm_section_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_frm_section
    FOR EACH ROW EXECUTE FUNCTION ent_frm_contenu_garde();
DROP TRIGGER IF EXISTS ent_frm_question_garde ON ent_frm_question;
CREATE TRIGGER ent_frm_question_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_frm_question
    FOR EACH ROW EXECUTE FUNCTION ent_frm_contenu_garde();
DROP TRIGGER IF EXISTS ent_frm_option_garde ON ent_frm_option;
CREATE TRIGGER ent_frm_option_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_frm_option
    FOR EACH ROW EXECUTE FUNCTION ent_frm_contenu_garde();
DROP TRIGGER IF EXISTS ent_frm_ligne_garde ON ent_frm_ligne;
CREATE TRIGGER ent_frm_ligne_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_frm_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_frm_contenu_garde();
DROP TRIGGER IF EXISTS ent_frm_condition_garde ON ent_frm_condition;
CREATE TRIGGER ent_frm_condition_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_frm_condition
    FOR EACH ROW EXECUTE FUNCTION ent_frm_contenu_garde();

-- Condition : la question dépendante et celle dont elle dépend sont de la même version, la seconde placée avant la
-- première (section puis ordre), et l'option appartient à la seconde. Aucune boucle n'est donc possible.
CREATE OR REPLACE FUNCTION ent_frm_condition_coherence() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    q_version UUID;
    d_version UUID;
    q_rang INTEGER[];
    d_rang INTEGER[];
    o_question UUID;
BEGIN
    SELECT q.version_id, ARRAY[s.ordre, q.ordre] INTO q_version, q_rang
      FROM ent_frm_question q JOIN ent_frm_section s ON s.etablissement_id = q.etablissement_id AND s.id = q.section_id
     WHERE q.etablissement_id = NEW.etablissement_id AND q.id = NEW.question_id;
    SELECT q.version_id, ARRAY[s.ordre, q.ordre] INTO d_version, d_rang
      FROM ent_frm_question q JOIN ent_frm_section s ON s.etablissement_id = q.etablissement_id AND s.id = q.section_id
     WHERE q.etablissement_id = NEW.etablissement_id AND q.id = NEW.depend_de_question_id;
    SELECT question_id INTO o_question FROM ent_frm_option WHERE etablissement_id = NEW.etablissement_id AND id = NEW.option_id;
    IF q_version IS DISTINCT FROM d_version THEN
        RAISE EXCEPTION 'ent_frm_condition : les deux questions doivent être de la même version' USING ERRCODE = '23514';
    END IF;
    IF NOT (d_rang < q_rang) THEN
        RAISE EXCEPTION 'ent_frm_condition : une question ne dépend que d''une question placée avant elle (aucune boucle)'
            USING ERRCODE = '23514';
    END IF;
    IF o_question IS DISTINCT FROM NEW.depend_de_question_id THEN
        RAISE EXCEPTION 'ent_frm_condition : l''option n''appartient pas à la question dont on dépend' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_frm_condition_coherence ON ent_frm_condition;
CREATE TRIGGER ent_frm_condition_coherence BEFORE INSERT OR UPDATE ON ent_frm_condition
    FOR EACH ROW EXECUTE FUNCTION ent_frm_condition_coherence();

-- Corrections et journal : ajout seul.
CREATE OR REPLACE FUNCTION ent_frm_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF ent_frm_purge() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    RAISE EXCEPTION '% : table en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_frm_correction_garde ON ent_frm_correction;
CREATE TRIGGER ent_frm_correction_garde BEFORE UPDATE OR DELETE ON ent_frm_correction
    FOR EACH ROW EXECUTE FUNCTION ent_frm_ajout_seul();
DROP TRIGGER IF EXISTS ent_frm_journal_garde ON ent_frm_journal;
CREATE TRIGGER ent_frm_journal_garde BEFORE UPDATE OR DELETE ON ent_frm_journal
    FOR EACH ROW EXECUTE FUNCTION ent_frm_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_frm_formulaire', 'ent_frm_auteur', 'ent_frm_version', 'ent_frm_section', 'ent_frm_question', 'ent_frm_option',
    'ent_frm_ligne', 'ent_frm_condition', 'ent_frm_audience_critere', 'ent_frm_correction', 'ent_frm_journal']);
