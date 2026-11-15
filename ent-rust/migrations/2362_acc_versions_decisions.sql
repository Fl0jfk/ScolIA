-- Socle 18, lot FO1 — migration 2362 : versions de texte, usages couverts, décisions
-- (`docs/socles/18_FORMULAIRES.md` §8.2, §27 ; règles R-FC19, R-FC21, R-FC44, R-FC45, R-FC46, R-FC52 ; FD-13, FD-18).
--
-- Additif, rejouable, sans JSON, sans date UTC. Ajout seul : une version publiée est immuable, une décision ne se
-- modifie ni ne se supprime (une nouvelle décision remplace la précédente dans le calcul, l'historique reste).
--
-- Preuve de signature : `preuve_signature_id` référence la brique de signature du socle 7 (lot D6, à venir) par un
-- identifiant **sans clé étrangère** (le socle 7 n'a pas encore livré la table) ; tant qu'elle est NULL, le canal dit
-- ce qu'on sait de la décision (`en_ligne`, `guichet`, `papier`, `reprise_dossier`, `reprise_sans_preuve`). Le port
-- Rust `PortSignature` (ent-db/src/accords.rs) est le seul point d'appel : le socle 18 n'écrit aucune signature.

-- Versions de texte d'un objet. `publiee_le IS NULL` : brouillon (un seul par objet), modifiable.
CREATE TABLE IF NOT EXISTS ent_acc_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero >= 1),
    texte TEXT NOT NULL CHECK (char_length(btrim(texte)) BETWEEN 1 AND 10000),
    -- SHA-256 hexadécimal de (objet, numéro, texte, usages couverts), chaque élément précédé de sa longueur (R-FC45).
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    type_changement TEXT CHECK (type_changement IS NULL OR type_changement IN ('forme', 'fond', 'restriction')),
    publiee_le TIMESTAMPTZ,
    publiee_par UUID,
    -- Relecture du texte (juriste ou direction selon la nature, R-FC72) : tant qu'elle manque, le texte est « provisoire ».
    texte_relu_le TIMESTAMPTZ,
    texte_relu_par UUID,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_acc_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_acc_version_objet_id_uq UNIQUE (etablissement_id, objet_id, id),
    CONSTRAINT ent_acc_version_numero_uq UNIQUE (etablissement_id, objet_id, numero),
    CONSTRAINT ent_acc_version_objet_cfk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_version_changement_chk CHECK (type_changement IS NULL OR numero > 1),
    CONSTRAINT ent_acc_version_relue_chk CHECK ((texte_relu_le IS NULL) = (texte_relu_par IS NULL))
);
-- Un seul brouillon à la fois par objet.
CREATE UNIQUE INDEX IF NOT EXISTS ent_acc_version_brouillon_uq
    ON ent_acc_version (etablissement_id, objet_id) WHERE publiee_le IS NULL;

-- Usages couverts par une version (figés à la publication).
CREATE TABLE IF NOT EXISTS ent_acc_version_usage (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    usage_id UUID NOT NULL,
    PRIMARY KEY (version_id, usage_id),
    CONSTRAINT ent_acc_version_usage_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_acc_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_version_usage_usage_cfk FOREIGN KEY (etablissement_id, usage_id)
        REFERENCES ent_acc_usage (etablissement_id, id) ON DELETE RESTRICT
);

-- Décisions : ajout seul. Un sujet (élève ou personne), un décideur (compte), une qualité, une version, des usages.
CREATE TABLE IF NOT EXISTS ent_acc_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet_id UUID NOT NULL,
    version_id UUID NOT NULL,
    sujet_eleve_id UUID,
    sujet_personne_id UUID,
    -- Compte qui a décidé (identifiant sans clé étrangère : un compte purgé ne détruit pas la preuve, FD-13).
    decideur_user_id UUID NOT NULL,
    qualite TEXT NOT NULL CHECK (qualite IN (
        'titulaire', 'eleve_avis', 'eleve_cosignataire', 'eleve_majeur', 'tuteur', 'curateur_assiste',
        'service_gardien', 'delegataire_image', 'personne_soi')),
    decision TEXT NOT NULL CHECK (decision IN ('accorde', 'refuse', 'retire', 'annulee_par_etablissement')),
    canal TEXT NOT NULL CHECK (canal IN (
        'en_ligne', 'code', 'passkey', 'guichet', 'papier', 'attestation_reprise', 'reprise_dossier', 'reprise_sans_preuve')),
    preuve_signature_id UUID,
    date_signature_papier DATE,
    saisie_par_user_id UUID,
    annule_decision_id UUID,
    motif_annulation TEXT CHECK (motif_annulation IS NULL OR char_length(btrim(motif_annulation)) BETWEEN 1 AND 300),
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- SHA-256 de (version, sujet, décideur, qualité, décision, usages, date), chaque élément précédé de sa longueur.
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    -- Reprise d'une table antérieure : une ligne d'origine n'est reprise qu'une fois.
    source_reprise TEXT CHECK (source_reprise IS NULL OR source_reprise IN ('formulaire_193', 'personne_consentement_275')),
    source_reprise_id UUID,
    CONSTRAINT ent_acc_decision_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_acc_decision_objet_cfk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_decision_version_cfk FOREIGN KEY (etablissement_id, objet_id, version_id)
        REFERENCES ent_acc_version (etablissement_id, objet_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_decision_eleve_cfk FOREIGN KEY (etablissement_id, sujet_eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_decision_personne_cfk FOREIGN KEY (etablissement_id, sujet_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_decision_annule_cfk FOREIGN KEY (etablissement_id, annule_decision_id)
        REFERENCES ent_acc_decision (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_decision_sujet_chk CHECK (num_nonnulls(sujet_eleve_id, sujet_personne_id) = 1),
    CONSTRAINT ent_acc_decision_papier_chk CHECK (canal <> 'papier' OR date_signature_papier IS NOT NULL),
    CONSTRAINT ent_acc_decision_annulation_chk CHECK (
        (decision = 'annulee_par_etablissement') = (annule_decision_id IS NOT NULL AND motif_annulation IS NOT NULL)),
    CONSTRAINT ent_acc_decision_reprise_chk CHECK ((source_reprise IS NULL) = (source_reprise_id IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_acc_decision_reprise_uq
    ON ent_acc_decision (source_reprise, source_reprise_id) WHERE source_reprise IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_acc_decision_eleve_idx
    ON ent_acc_decision (etablissement_id, sujet_eleve_id, objet_id, decide_le DESC) WHERE sujet_eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_acc_decision_personne_idx
    ON ent_acc_decision (etablissement_id, sujet_personne_id, objet_id, decide_le DESC) WHERE sujet_personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_acc_decision_decideur_idx
    ON ent_acc_decision (etablissement_id, decideur_user_id, decide_le DESC);

-- Ce que la décision couvre, usage par usage (`accorde` faux : refus ou retrait de cet usage).
CREATE TABLE IF NOT EXISTS ent_acc_decision_usage (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    decision_id UUID NOT NULL,
    usage_id UUID NOT NULL,
    accorde BOOLEAN NOT NULL,
    PRIMARY KEY (decision_id, usage_id),
    CONSTRAINT ent_acc_decision_usage_decision_cfk FOREIGN KEY (etablissement_id, decision_id)
        REFERENCES ent_acc_decision (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_decision_usage_usage_cfk FOREIGN KEY (etablissement_id, usage_id)
        REFERENCES ent_acc_usage (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_acc_decision_usage_usage_idx ON ent_acc_decision_usage (etablissement_id, usage_id);

-- Immuabilité ------------------------------------------------------------------------------------
-- Version : publiée, elle ne change plus (seule la relecture du texte se pose, une fois). Le numéro d'une version
-- publiée dépasse toujours celui des versions déjà publiées (R-FC21, FD-14).
CREATE OR REPLACE FUNCTION ent_acc_version_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    plus_haut INTEGER;
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF OLD.publiee_le IS NULL THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION 'ent_acc_version : une version publiée ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.publiee_le IS NOT NULL THEN
            SELECT max(numero) INTO plus_haut FROM ent_acc_version
             WHERE etablissement_id = NEW.etablissement_id AND objet_id = NEW.objet_id AND publiee_le IS NOT NULL;
            IF plus_haut IS NOT NULL AND plus_haut >= NEW.numero THEN
                RAISE EXCEPTION 'ent_acc_version : numéro % non supérieur à la dernière version publiée (%)', NEW.numero, plus_haut
                    USING ERRCODE = '23514';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    IF OLD.publiee_le IS NOT NULL THEN
        IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
           OR NEW.objet_id IS DISTINCT FROM OLD.objet_id OR NEW.numero IS DISTINCT FROM OLD.numero
           OR NEW.texte IS DISTINCT FROM OLD.texte OR NEW.empreinte IS DISTINCT FROM OLD.empreinte
           OR NEW.type_changement IS DISTINCT FROM OLD.type_changement
           OR NEW.publiee_le IS DISTINCT FROM OLD.publiee_le OR NEW.publiee_par IS DISTINCT FROM OLD.publiee_par
           OR (OLD.texte_relu_le IS NOT NULL AND (NEW.texte_relu_le IS DISTINCT FROM OLD.texte_relu_le
                                                  OR NEW.texte_relu_par IS DISTINCT FROM OLD.texte_relu_par)) THEN
            RAISE EXCEPTION 'ent_acc_version : une version publiée est immuable (publier une nouvelle version)' USING ERRCODE = '42501';
        END IF;
    ELSIF NEW.publiee_le IS NOT NULL THEN
        SELECT max(numero) INTO plus_haut FROM ent_acc_version
         WHERE etablissement_id = NEW.etablissement_id AND objet_id = NEW.objet_id
           AND publiee_le IS NOT NULL AND id <> NEW.id;
        IF plus_haut IS NOT NULL AND plus_haut >= NEW.numero THEN
            RAISE EXCEPTION 'ent_acc_version : numéro % non supérieur à la dernière version publiée (%)', NEW.numero, plus_haut
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_acc_version_garde ON ent_acc_version;
CREATE TRIGGER ent_acc_version_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_acc_version
    FOR EACH ROW EXECUTE FUNCTION ent_acc_version_garde();

-- Usages couverts : modifiables tant que la version est un brouillon, figés ensuite.
CREATE OR REPLACE FUNCTION ent_acc_version_usage_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    publiee TIMESTAMPTZ;
    cible UUID;
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    cible := CASE WHEN TG_OP = 'DELETE' THEN OLD.version_id ELSE NEW.version_id END;
    SELECT publiee_le INTO publiee FROM ent_acc_version WHERE id = cible;
    IF publiee IS NOT NULL THEN
        RAISE EXCEPTION 'ent_acc_version_usage : les usages d''une version publiée sont figés' USING ERRCODE = '42501';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;
DROP TRIGGER IF EXISTS ent_acc_version_usage_garde ON ent_acc_version_usage;
CREATE TRIGGER ent_acc_version_usage_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_acc_version_usage
    FOR EACH ROW EXECUTE FUNCTION ent_acc_version_usage_garde();

-- Décisions et usages décidés : ajout seul. Seule la fusion de deux fiches élève réaffecte `sujet_eleve_id`.
CREATE OR REPLACE FUNCTION ent_acc_decision_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE' AND TG_TABLE_NAME = 'ent_acc_decision'
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
       AND NEW.source_reprise_id IS NOT DISTINCT FROM OLD.source_reprise_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION '% : une décision est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_acc_decision_garde ON ent_acc_decision;
CREATE TRIGGER ent_acc_decision_garde BEFORE UPDATE OR DELETE ON ent_acc_decision
    FOR EACH ROW EXECUTE FUNCTION ent_acc_decision_garde();
DROP TRIGGER IF EXISTS ent_acc_decision_usage_garde ON ent_acc_decision_usage;
CREATE TRIGGER ent_acc_decision_usage_garde BEFORE UPDATE OR DELETE ON ent_acc_decision_usage
    FOR EACH ROW EXECUTE FUNCTION ent_acc_decision_garde();

SELECT ent_rls_poser(ARRAY['ent_acc_version', 'ent_acc_version_usage', 'ent_acc_decision', 'ent_acc_decision_usage']);
