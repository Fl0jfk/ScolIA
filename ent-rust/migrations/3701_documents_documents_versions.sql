-- Socle 7, lot D2 — migration 3701 : documents, versions immuables, données figées, exemplaires nominatifs.
-- Réf. : docs/socles/07_DOCUMENTS.md §6, §7.1, §8.3, §8.14 (domaine R-D6, versions particulières), §4.2 (D7 à D10).
--
-- Additive et rejouable (E8). Pas de JSON, de tableau ni de BYTEA (D-036) : les données figées d'une édition
-- sont des lignes clé/valeur typées. Clés étrangères composites en RESTRICT. Aucune clé étrangère vers une autre
-- brique pour le fait d'origine : (source_brique, source_type, source_id) est vérifié par lecture publiée.
-- Les octets ne sont JAMAIS en base : une version pointe un objet du registre (ent_objet, migration 500).

-- ───────────────────────── Le document ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_document (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL
        CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    entite_id UUID NOT NULL,
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne', 'agent', 'etablissement', 'demarche')),
    sujet_id UUID NOT NULL,
    -- Dénormalisés pour le filtre de portée et l'isolation (§8.3) ; cohérents avec le sujet (contrainte ci-dessous).
    eleve_id UUID,
    personne_id UUID,
    agent_id UUID,
    annee_scolaire_id UUID,
    origine TEXT NOT NULL CHECK (origine IN ('emis', 'depose', 'recu', 'numerise', 'importe')),
    sensibilite TEXT NOT NULL CHECK (sensibilite IN ('C0', 'C1', 'C2', 'C3', 'C4')),
    -- Domaine (R-D6) : sans « rh » et « financier », les dossiers des agents et les avis d'imposition seraient lus
    -- de tout le secrétariat.
    domaine TEXT NOT NULL DEFAULT 'scolarite'
        CHECK (domaine IN ('scolarite', 'sante', 'justice', 'financier', 'rh', 'social', 'disciplinaire')),
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    -- Servi à qui n'a pas le droit de lire le titre (RP1-05) : jamais d'indice sur la nature d'une sanction ou d'une
    -- pièce de santé.
    titre_neutre TEXT NOT NULL CHECK (length(btrim(titre_neutre)) BETWEEN 1 AND 200),
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN (
        'brouillon', 'emis', 'depose', 'accepte', 'refuse', 'annule', 'gele', 'elimine', 'verse')),
    -- Numérotation (migration 3702), attribuée à l'émission : année civile de l'établissement, série, rang.
    serie_code TEXT CHECK (serie_code IS NULL OR serie_code IN (
        'SCO', 'ADM', 'VS', 'DIS', 'PED', 'STG', 'FIN', 'PER', 'ETB', 'CRA', 'CRD')),
    numero_annee INT CHECK (numero_annee IS NULL OR numero_annee BETWEEN 2000 AND 2999),
    numero INT CHECK (numero IS NULL OR numero >= 1),
    -- Code de vérification publique : 12 caractères sans ambiguïté visuelle (≥ 60 bits), tiré par le service.
    code_verification TEXT CHECK (code_verification IS NULL OR code_verification ~ '^[A-HJ-NP-Z2-9]{12}$'),
    -- Mode de vérification publique déclaré par la nature (§9.7) ; copié ici pour que la page publique ne relise
    -- jamais la déclaration d'une brique.
    verification_publique TEXT NOT NULL DEFAULT 'authenticite_seule'
        CHECK (verification_publique IN ('complete', 'authenticite_seule', 'aucune')),
    version_courante INT NOT NULL DEFAULT 0 CHECK (version_courante >= 0),
    source_brique TEXT CHECK (source_brique IS NULL OR source_brique ~ '^[a-z][a-z0-9_]{0,39}$'),
    source_type TEXT CHECK (source_type IS NULL OR source_type ~ '^[a-z][a-z0-9_]{0,39}$'),
    source_id UUID,
    categorie_conservation TEXT NOT NULL CHECK (categorie_conservation ~ '^[a-z][a-z0-9_]{0,59}$'),
    annule_le TIMESTAMPTZ,
    annule_par UUID,
    annule_motif TEXT CHECK (annule_motif IS NULL OR length(btrim(annule_motif)) BETWEEN 3 AND 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par UUID,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_document_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_document_entite_cfk FOREIGN KEY (etablissement_id, entite_id)
        REFERENCES ent_doc_entite (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_document_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_document_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_document_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_document_annee_cfk FOREIGN KEY (etablissement_id, annee_scolaire_id)
        REFERENCES ent_annee_scolaire (etablissement_id, id) ON DELETE RESTRICT,
    -- Le sujet désigne l'une des trois colonnes dénormalisées, qui lui est égale.
    CONSTRAINT ent_doc_document_sujet_chk CHECK (
        (sujet_type <> 'eleve' OR eleve_id = sujet_id)
        AND (sujet_type <> 'personne' OR personne_id = sujet_id)
        AND (sujet_type <> 'agent' OR agent_id = sujet_id)),
    -- Un brouillon n'a ni numéro, ni code, ni version ; un document émis a un numéro complet et au moins une version
    -- (le code peut manquer seulement pour une nature sans vérification publique).
    CONSTRAINT ent_doc_document_numero_chk CHECK (
        (etat = 'brouillon' AND serie_code IS NULL AND numero_annee IS NULL AND numero IS NULL
            AND code_verification IS NULL AND version_courante = 0)
        OR (etat <> 'brouillon' AND origine <> 'emis')
        OR (etat <> 'brouillon' AND serie_code IS NOT NULL AND numero_annee IS NOT NULL AND numero IS NOT NULL
            AND version_courante >= 1
            AND (code_verification IS NOT NULL OR verification_publique = 'aucune'))),
    CONSTRAINT ent_doc_document_numero_ensemble_chk CHECK (
        (serie_code IS NULL) = (numero IS NULL) AND (numero IS NULL) = (numero_annee IS NULL)),
    CONSTRAINT ent_doc_document_annulation_chk CHECK (
        (etat = 'annule') = (annule_le IS NOT NULL)
        AND (annule_le IS NULL) = (annule_par IS NULL)
        AND (annule_le IS NULL) = (annule_motif IS NULL))
);
-- Un seul document par numéro dans l'entité émettrice, la série et l'année (les trous sont impossibles, les doublons aussi).
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_document_numero_uq
    ON ent_doc_document (etablissement_id, entite_id, serie_code, numero_annee, numero) WHERE numero IS NOT NULL;
-- Recherche publique par code : un code désigne un seul document, toutes entités confondues.
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_document_code_uq
    ON ent_doc_document (code_verification) WHERE code_verification IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_document_eleve_idx
    ON ent_doc_document (etablissement_id, eleve_id, cree_le DESC) WHERE eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_document_personne_idx
    ON ent_doc_document (etablissement_id, personne_id, cree_le DESC) WHERE personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_document_agent_idx
    ON ent_doc_document (etablissement_id, agent_id, cree_le DESC) WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_document_nature_idx
    ON ent_doc_document (etablissement_id, nature_code, etat);
CREATE INDEX IF NOT EXISTS ent_doc_document_source_idx
    ON ent_doc_document (etablissement_id, source_brique, source_type, source_id) WHERE source_id IS NOT NULL;

-- Un document émis ne se réécrit pas (§7.1) : seuls l'état, la version courante, l'annulation et la date de mise à
-- jour bougent, et la numérotation ne s'attribue qu'une fois (NULL → valeur). Aucune suppression.
CREATE OR REPLACE FUNCTION ent_doc_document_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_document : suppression interdite (le sort final passe par la conservation, lot D9)'
            USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.nature_code IS DISTINCT FROM OLD.nature_code
       OR NEW.entite_id IS DISTINCT FROM OLD.entite_id
       OR NEW.sujet_type IS DISTINCT FROM OLD.sujet_type
       OR NEW.sujet_id IS DISTINCT FROM OLD.sujet_id
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
       OR NEW.annee_scolaire_id IS DISTINCT FROM OLD.annee_scolaire_id
       OR NEW.origine IS DISTINCT FROM OLD.origine
       OR NEW.sensibilite IS DISTINCT FROM OLD.sensibilite
       OR NEW.domaine IS DISTINCT FROM OLD.domaine
       OR NEW.titre IS DISTINCT FROM OLD.titre
       OR NEW.titre_neutre IS DISTINCT FROM OLD.titre_neutre
       OR NEW.verification_publique IS DISTINCT FROM OLD.verification_publique
       OR NEW.source_brique IS DISTINCT FROM OLD.source_brique
       OR NEW.source_type IS DISTINCT FROM OLD.source_type
       OR NEW.source_id IS DISTINCT FROM OLD.source_id
       OR NEW.categorie_conservation IS DISTINCT FROM OLD.categorie_conservation
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le
       OR NEW.cree_par IS DISTINCT FROM OLD.cree_par THEN
        RAISE EXCEPTION 'ent_doc_document : identité du document immuable' USING ERRCODE = '23001';
    END IF;
    -- Numéro et code : attribués une seule fois, à l'émission.
    IF (OLD.numero IS NOT NULL AND (NEW.numero IS DISTINCT FROM OLD.numero
                                    OR NEW.numero_annee IS DISTINCT FROM OLD.numero_annee
                                    OR NEW.serie_code IS DISTINCT FROM OLD.serie_code))
       OR (OLD.code_verification IS NOT NULL AND NEW.code_verification IS DISTINCT FROM OLD.code_verification) THEN
        RAISE EXCEPTION 'ent_doc_document : numéro et code de vérification immuables' USING ERRCODE = '23001';
    END IF;
    -- Transitions admises à ce lot (les états de pièces, gel et sort final viennent avec D5 et D9).
    IF NEW.etat IS DISTINCT FROM OLD.etat
       AND NOT ((OLD.etat = 'brouillon' AND NEW.etat IN ('emis', 'annule'))
             OR (OLD.etat = 'emis' AND NEW.etat IN ('annule', 'gele'))
             OR (OLD.etat = 'gele' AND NEW.etat = 'emis')) THEN
        RAISE EXCEPTION 'ent_doc_document : transition % → % interdite', OLD.etat, NEW.etat USING ERRCODE = '23001';
    END IF;
    -- La version courante ne fait qu'avancer, d'une version à la fois (jamais en retrait).
    IF NEW.version_courante < OLD.version_courante OR NEW.version_courante > OLD.version_courante + 1 THEN
        RAISE EXCEPTION 'ent_doc_document : la version courante avance d''un pas' USING ERRCODE = '23001';
    END IF;
    -- Un document annulé ne reçoit plus de version.
    IF OLD.etat = 'annule' AND NEW.version_courante IS DISTINCT FROM OLD.version_courante THEN
        RAISE EXCEPTION 'ent_doc_document : document annulé' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_document_garde_trg ON ent_doc_document;
CREATE TRIGGER ent_doc_document_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_document
    FOR EACH ROW EXECUTE FUNCTION ent_doc_document_garde();

-- ───────────────────────── La version (immuable) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    document_id UUID NOT NULL,
    numero_version INT NOT NULL CHECK (numero_version >= 1),
    -- Le PDF émis : un objet du registre (clé opaque préfixée par l'établissement, jamais de nom de fichier).
    cle_objet TEXT NOT NULL,
    empreinte_sha256 TEXT NOT NULL CHECK (empreinte_sha256 ~ '^[0-9a-f]{64}$'),
    nb_pages INT NOT NULL CHECK (nb_pages >= 1),
    langue TEXT NOT NULL DEFAULT 'fr' CHECK (langue ~ '^[a-z]{2}$'),
    trame_code TEXT CHECK (trame_code IS NULL OR trame_code ~ '^[a-z][a-z0-9_.]{0,79}$'),
    trame_version INT CHECK (trame_version IS NULL OR trame_version >= 1),
    style_code TEXT CHECK (style_code IS NULL OR style_code IN ('classique', 'contemporaine', 'epuree', 'institutionnelle')),
    identite_version INT CHECK (identite_version IS NULL OR identite_version >= 1),
    type_version TEXT NOT NULL DEFAULT 'originale'
        CHECK (type_version IN ('originale', 'occultee', 'papier_original', 'importee')),
    version_source_id UUID,
    -- Motif obligatoire dès la deuxième version (réémission, D9).
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 3 AND 500),
    -- Identifiant de la clé du cachet de l'établissement qui a scellé cette version (PAdES, §10.7) : le registre dit
    -- quelles versions ont été scellées par quelle clé (révocation, rotation).
    cachet_cle_ref TEXT CHECK (cachet_cle_ref IS NULL OR length(cachet_cle_ref) BETWEEN 1 AND 120),
    emise_par UUID NOT NULL,
    emise_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    au_nom_de_agent_id UUID,
    par_delegation BOOLEAN NOT NULL DEFAULT false,
    CONSTRAINT ent_doc_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_version_numero_uq UNIQUE (document_id, numero_version),
    CONSTRAINT ent_doc_version_document_cfk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_version_objet_cfk FOREIGN KEY (etablissement_id, cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_version_source_cfk FOREIGN KEY (etablissement_id, version_source_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_version_agent_cfk FOREIGN KEY (etablissement_id, au_nom_de_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_version_motif_chk CHECK (numero_version = 1 OR motif IS NOT NULL),
    CONSTRAINT ent_doc_version_delegation_chk CHECK (NOT par_delegation OR au_nom_de_agent_id IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_doc_version_empreinte_idx ON ent_doc_version (etablissement_id, empreinte_sha256);

-- Ajout seul : ni UPDATE ni DELETE (modèle 182). L'empreinte de la version est celle de l'objet du registre : deux
-- références indépendantes qui doivent s'accorder (même contrôle que `lire_objet_empreinte`).
CREATE OR REPLACE FUNCTION ent_doc_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : ajout seul (% interdit)', TG_TABLE_NAME, TG_OP USING ERRCODE = '23001';
END
$$;
CREATE OR REPLACE FUNCTION ent_doc_version_coherente() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_sha TEXT;
BEGIN
    SELECT sha256 INTO v_sha FROM ent_objet
     WHERE etablissement_id = NEW.etablissement_id AND cle_objet = NEW.cle_objet;
    IF v_sha IS DISTINCT FROM NEW.empreinte_sha256 THEN
        RAISE EXCEPTION 'ent_doc_version : l''empreinte de la version ne correspond pas à celle de l''objet'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_version_ajout_seul_trg ON ent_doc_version;
CREATE TRIGGER ent_doc_version_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_version
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();
DROP TRIGGER IF EXISTS ent_doc_version_coherente_trg ON ent_doc_version;
CREATE TRIGGER ent_doc_version_coherente_trg
    BEFORE INSERT ON ent_doc_version
    FOR EACH ROW EXECUTE FUNCTION ent_doc_version_coherente();

-- ───────────────────────── Données figées d'une version ─────────────────────────
-- Ce qui a servi au rendu (nom, naissance, classe, année…), en lignes clé/valeur. Les valeurs de données
-- personnelles sont scellées par le service (champs_chiffres, préfixe `enc:v1:`) ; le rendu se rejoue à
-- l'identique à partir d'elles. Ajout seul.
CREATE TABLE IF NOT EXISTS ent_doc_version_donnee (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    cle TEXT NOT NULL CHECK (cle ~ '^[a-z][a-z0-9_.]{0,59}$'),
    valeur TEXT NOT NULL CHECK (length(valeur) <= 4000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (version_id, cle),
    CONSTRAINT ent_doc_version_donnee_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_doc_version_donnee_ajout_seul_trg ON ent_doc_version_donnee;
CREATE TRIGGER ent_doc_version_donnee_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_version_donnee
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

-- ───────────────────────── Exemplaires nominatifs ─────────────────────────
-- Version nominative pour un destinataire (parents séparés : deux exemplaires, même numéro de document, rang
-- distinct, D8). Le fichier de l'exemplaire porte l'en-tête et, au plus, l'adresse de SON destinataire (D7).
CREATE TABLE IF NOT EXISTS ent_doc_exemplaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    rang INT NOT NULL CHECK (rang >= 1),
    destinataire_personne_id UUID,
    destinataire_agent_id UUID,
    -- Personne dont l'adresse postale est imprimée sur cet exemplaire (NULL : aucune adresse). Elle ne peut être que
    -- celle du destinataire : un document ne porte jamais l'adresse d'un autre, protégée ou non (D7, ADM-25).
    adresse_de_personne_id UUID,
    cle_objet TEXT NOT NULL,
    empreinte_sha256 TEXT NOT NULL CHECK (empreinte_sha256 ~ '^[0-9a-f]{64}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_exemplaire_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_exemplaire_rang_uq UNIQUE (version_id, rang),
    CONSTRAINT ent_doc_exemplaire_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_exemplaire_personne_cfk FOREIGN KEY (etablissement_id, destinataire_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_exemplaire_agent_cfk FOREIGN KEY (etablissement_id, destinataire_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_exemplaire_objet_cfk FOREIGN KEY (etablissement_id, cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_exemplaire_destinataire_chk CHECK (num_nonnulls(destinataire_personne_id, destinataire_agent_id) <= 1),
    CONSTRAINT ent_doc_exemplaire_adresse_chk CHECK (
        adresse_de_personne_id IS NULL OR adresse_de_personne_id = destinataire_personne_id)
);
CREATE INDEX IF NOT EXISTS ent_doc_exemplaire_personne_idx
    ON ent_doc_exemplaire (etablissement_id, destinataire_personne_id) WHERE destinataire_personne_id IS NOT NULL;
DROP TRIGGER IF EXISTS ent_doc_exemplaire_ajout_seul_trg ON ent_doc_exemplaire;
CREATE TRIGGER ent_doc_exemplaire_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_exemplaire
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();
DROP TRIGGER IF EXISTS ent_doc_exemplaire_coherent_trg ON ent_doc_exemplaire;
CREATE TRIGGER ent_doc_exemplaire_coherent_trg
    BEFORE INSERT ON ent_doc_exemplaire
    FOR EACH ROW EXECUTE FUNCTION ent_doc_version_coherente();

SELECT ent_rls_poser(ARRAY[
    'ent_doc_document',
    'ent_doc_version',
    'ent_doc_version_donnee',
    'ent_doc_exemplaire'
]);
