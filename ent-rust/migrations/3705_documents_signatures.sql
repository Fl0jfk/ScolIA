-- Socle 7, lot D6 — migration 3705 : la brique de signature (circuits, signataires, signatures chaînées, défis,
-- horodatage RFC 3161 sélectif). Réf. : 07_DOCUMENTS.md §8.7, §9.6, §10 ; CARNET_IDEES I-013 et I-014 (décisions de
-- Florian du 16-11).
--
-- Principes inscrits en base (la base garde les garde-fous même si le service est contourné) :
--   * un circuit porte sur UNE version : il ne change jamais de version ; une nouvelle version le rend « obsolète »
--     (le service le fait dans la transaction de la réémission) ; un circuit clos ne se rouvre pas ;
--   * les signatures sont en AJOUT SEUL et CHAÎNÉES : chaque entrée porte le sceau SHA-256 de la précédente du même
--     circuit (rang, empreinte précédente et empreinte de l'entrée sont contrôlés par un déclencheur) ; une entrée ne
--     se modifie ni ne se supprime ; un signataire ne signe qu'une fois ;
--   * un défi (passkey, code à usage unique) se consomme UNE fois (rejeu refusé) ;
--   * l'horodatage qualifié est désactivé par défaut ; le relevé de consommation est un journal d'ajout (une ligne par
--     jeton demandé, résultat inscrit une seule fois) ;
--   * D-036 : ni JSON, ni tableau, ni BYTEA ; aucune date « du jour » en base (le service passe le jour de l'établissement).

-- ───────────────────────── Réglages par établissement ─────────────────────────
-- Menu des modes de signature d'une nature (1 à 4 choix, un seul par défaut). Absent = menu par défaut de la nature
-- (déclaration de code).
CREATE TABLE IF NOT EXISTS ent_doc_signature_menu (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL
        CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    mode TEXT NOT NULL CHECK (mode IN ('passkey', 'code_sms', 'guichet', 'papier')),
    rang SMALLINT NOT NULL CHECK (rang BETWEEN 1 AND 4),
    defaut BOOLEAN NOT NULL DEFAULT false,
    maj_par UUID,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, nature_code, mode),
    CONSTRAINT ent_doc_signature_menu_rang_uq UNIQUE (etablissement_id, nature_code, rang)
);
-- Un seul mode par défaut par nature.
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_signature_menu_defaut_uq
    ON ent_doc_signature_menu (etablissement_id, nature_code) WHERE defaut;

-- Horodatage sélectif par nature : absent = défaut de la nature (déclaration de code) ; l'établissement le règle.
CREATE TABLE IF NOT EXISTS ent_doc_horodatage_nature (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL
        CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    actif BOOLEAN NOT NULL,
    maj_par UUID,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, nature_code)
);

-- Service d'horodatage RFC 3161 de l'établissement : DÉSACTIVÉ par défaut (aucune ligne = désactivé).
--   mode 'mutualise' : le compte de ScolIA (configuré hors base, un seul pour tous) ; aucune adresse ici ;
--   mode 'propre'    : le prestataire de l'établissement (adresse du service, identifiant, secret scellé par le service).
CREATE TABLE IF NOT EXISTS ent_doc_horodatage_service (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    mode TEXT NOT NULL CHECK (mode IN ('mutualise', 'propre')),
    actif BOOLEAN NOT NULL DEFAULT false,
    url TEXT CHECK (url IS NULL OR (url ~ '^https://' AND length(url) <= 300)),
    identifiant TEXT CHECK (identifiant IS NULL OR length(identifiant) BETWEEN 1 AND 200),
    secret_scelle TEXT CHECK (secret_scelle IS NULL OR length(secret_scelle) BETWEEN 1 AND 1000),
    maj_par UUID,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_horodatage_service_propre_chk CHECK (mode = 'mutualise' OR url IS NOT NULL)
);

-- ───────────────────────── Circuits ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_circuit (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    document_id UUID NOT NULL,
    version_id UUID NOT NULL,
    nature_code TEXT NOT NULL
        CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    ordre TEXT NOT NULL CHECK (ordre IN ('parallele', 'sequentiel')),
    regle_titulaires TEXT NOT NULL CHECK (regle_titulaires IN ('aucun', 'un', 'tous')),
    eleve_cosigne BOOLEAN NOT NULL DEFAULT false,
    echeance DATE,
    relances_max SMALLINT NOT NULL DEFAULT 2 CHECK (relances_max BETWEEN 0 AND 5),
    -- Horodatage qualifié demandé à la complétion (décidé au lancement, d'après la nature et le réglage).
    horodater BOOLEAN NOT NULL DEFAULT false,
    etat TEXT NOT NULL DEFAULT 'ouvert'
        CHECK (etat IN ('ouvert', 'complet', 'refuse', 'expire', 'annule', 'obsolete')),
    lance_par UUID NOT NULL,
    lance_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    clos_le TIMESTAMPTZ,
    clos_motif TEXT CHECK (clos_motif IS NULL OR length(btrim(clos_motif)) BETWEEN 3 AND 500),
    -- À la complétion : PDF signé (page de signatures ajoutée, cachet de l'établissement), dossier de preuve (PDF
    -- lisible, cacheté), tête de la chaîne. Écrits UNE fois, avec le passage à « complet ».
    pdf_signe_cle_objet TEXT,
    pdf_signe_sha256 TEXT CHECK (pdf_signe_sha256 IS NULL OR pdf_signe_sha256 ~ '^[0-9a-f]{64}$'),
    preuve_cle_objet TEXT,
    preuve_sha256 TEXT CHECK (preuve_sha256 IS NULL OR preuve_sha256 ~ '^[0-9a-f]{64}$'),
    tete_chaine_sha256 TEXT CHECK (tete_chaine_sha256 IS NULL OR tete_chaine_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT ent_doc_circuit_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_circuit_document_cfk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_circuit_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_circuit_signe_objet_cfk FOREIGN KEY (etablissement_id, pdf_signe_cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_circuit_preuve_objet_cfk FOREIGN KEY (etablissement_id, preuve_cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_circuit_clos_chk CHECK ((etat = 'ouvert') = (clos_le IS NULL)),
    CONSTRAINT ent_doc_circuit_complet_chk CHECK (
        (etat = 'complet') = (preuve_cle_objet IS NOT NULL AND preuve_sha256 IS NOT NULL AND tete_chaine_sha256 IS NOT NULL)),
    CONSTRAINT ent_doc_circuit_signe_ensemble_chk CHECK ((pdf_signe_cle_objet IS NULL) = (pdf_signe_sha256 IS NULL)),
    CONSTRAINT ent_doc_circuit_motif_chk CHECK (etat NOT IN ('refuse', 'annule', 'obsolete') OR clos_motif IS NOT NULL)
);
-- Un seul circuit ouvert par version.
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_circuit_ouvert_uq
    ON ent_doc_circuit (etablissement_id, version_id) WHERE etat = 'ouvert';
CREATE INDEX IF NOT EXISTS ent_doc_circuit_document_idx ON ent_doc_circuit (etablissement_id, document_id, lance_le DESC);
CREATE INDEX IF NOT EXISTS ent_doc_circuit_ouvert_echeance_idx
    ON ent_doc_circuit (etablissement_id, echeance) WHERE etat = 'ouvert';
CREATE INDEX IF NOT EXISTS ent_doc_circuit_signe_sha_idx ON ent_doc_circuit (pdf_signe_sha256) WHERE pdf_signe_sha256 IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_circuit_preuve_sha_idx ON ent_doc_circuit (preuve_sha256) WHERE preuve_sha256 IS NOT NULL;

CREATE OR REPLACE FUNCTION ent_doc_circuit_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_circuit : suppression interdite (un circuit se clôt avec un motif)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.document_id IS DISTINCT FROM OLD.document_id
       OR NEW.version_id IS DISTINCT FROM OLD.version_id
       OR NEW.nature_code IS DISTINCT FROM OLD.nature_code
       OR NEW.ordre IS DISTINCT FROM OLD.ordre
       OR NEW.regle_titulaires IS DISTINCT FROM OLD.regle_titulaires
       OR NEW.eleve_cosigne IS DISTINCT FROM OLD.eleve_cosigne
       OR NEW.horodater IS DISTINCT FROM OLD.horodater
       OR NEW.lance_par IS DISTINCT FROM OLD.lance_par
       OR NEW.lance_le IS DISTINCT FROM OLD.lance_le THEN
        RAISE EXCEPTION 'ent_doc_circuit : identité du circuit immuable (une nouvelle version ouvre un nouveau circuit)'
            USING ERRCODE = '23001';
    END IF;
    IF OLD.etat <> 'ouvert' THEN
        -- Un circuit clos est final : seule l'échéance (jamais) ni rien d'autre ne bouge.
        IF ROW(NEW.*) IS DISTINCT FROM ROW(OLD.*) THEN
            RAISE EXCEPTION 'ent_doc_circuit : circuit clos (%), plus aucune modification', OLD.etat USING ERRCODE = '23001';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.etat IS DISTINCT FROM OLD.etat AND NEW.etat NOT IN ('complet', 'refuse', 'expire', 'annule', 'obsolete') THEN
        RAISE EXCEPTION 'ent_doc_circuit : transition % → % interdite', OLD.etat, NEW.etat USING ERRCODE = '23001';
    END IF;
    IF NEW.etat = 'ouvert' AND (NEW.pdf_signe_cle_objet IS NOT NULL OR NEW.preuve_cle_objet IS NOT NULL) THEN
        RAISE EXCEPTION 'ent_doc_circuit : les pièces de preuve ne s''écrivent qu''à la complétion' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_circuit_garde_trg ON ent_doc_circuit;
CREATE TRIGGER ent_doc_circuit_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_circuit
    FOR EACH ROW EXECUTE FUNCTION ent_doc_circuit_garde();

-- ───────────────────────── Signataires ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_signataire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    circuit_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang >= 1),
    role TEXT NOT NULL CHECK (role IN ('titulaire', 'eleve', 'agent', 'etablissement', 'externe')),
    libelle_role TEXT NOT NULL CHECK (length(btrim(libelle_role)) BETWEEN 1 AND 80),
    -- Exactement un : une personne (titulaire, tuteur ayant un compte…), un agent, un élève, ou un externe (nom saisi).
    personne_id UUID,
    agent_id UUID,
    eleve_id UUID,
    externe_nom TEXT CHECK (externe_nom IS NULL OR length(btrim(externe_nom)) BETWEEN 2 AND 120),
    externe_organisation TEXT CHECK (externe_organisation IS NULL OR length(externe_organisation) <= 120),
    -- Contact de l'externe : scellé par le service (champs sensibles) ; jamais relu hors de l'envoi du code.
    externe_contact_scelle TEXT CHECK (externe_contact_scelle IS NULL OR length(externe_contact_scelle) <= 1000),
    -- Qui choisit le mode : l'émetteur (menu fermé à un choix) ou le signataire lui-même (parmi les modes autorisés).
    choix_par TEXT NOT NULL DEFAULT 'emetteur' CHECK (choix_par IN ('emetteur', 'signataire')),
    -- Lien nominatif du signataire externe : jeton aléatoire, JAMAIS stocké (seulement son empreinte), durée limitée.
    lien_jeton_sha256 TEXT CHECK (lien_jeton_sha256 IS NULL OR lien_jeton_sha256 ~ '^[0-9a-f]{64}$'),
    lien_expire_le TIMESTAMPTZ,
    requis BOOLEAN NOT NULL DEFAULT true,
    etat TEXT NOT NULL DEFAULT 'attendu'
        CHECK (etat IN ('attendu', 'invite', 'papier_depose', 'signe', 'refuse', 'retire', 'remplace', 'expire')),
    refus_motif TEXT CHECK (refus_motif IS NULL OR length(btrim(refus_motif)) BETWEEN 3 AND 500),
    papier_cle_objet TEXT,
    papier_depose_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    etat_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_signataire_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_signataire_circuit_cfk FOREIGN KEY (etablissement_id, circuit_id)
        REFERENCES ent_doc_circuit (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signataire_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signataire_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signataire_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signataire_papier_cfk FOREIGN KEY (etablissement_id, papier_cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signataire_identite_chk CHECK (num_nonnulls(personne_id, agent_id, eleve_id, externe_nom) = 1),
    CONSTRAINT ent_doc_signataire_role_chk CHECK (
        (role = 'eleve') = (eleve_id IS NOT NULL)
        AND (role = 'externe') = (externe_nom IS NOT NULL)
        AND (role NOT IN ('agent', 'etablissement') OR agent_id IS NOT NULL)
        AND (role <> 'titulaire' OR personne_id IS NOT NULL)),
    CONSTRAINT ent_doc_signataire_refus_chk CHECK (refus_motif IS NULL OR etat = 'refuse'),
    CONSTRAINT ent_doc_signataire_papier_chk CHECK ((papier_cle_objet IS NULL) = (papier_depose_le IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_signataire_rang_uq
    ON ent_doc_signataire (circuit_id, rang) WHERE etat NOT IN ('remplace', 'retire');
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_signataire_lien_uq
    ON ent_doc_signataire (lien_jeton_sha256) WHERE lien_jeton_sha256 IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_signataire_circuit_idx ON ent_doc_signataire (etablissement_id, circuit_id);
CREATE INDEX IF NOT EXISTS ent_doc_signataire_personne_idx
    ON ent_doc_signataire (etablissement_id, personne_id) WHERE personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_signataire_agent_idx
    ON ent_doc_signataire (etablissement_id, agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_signataire_eleve_idx
    ON ent_doc_signataire (etablissement_id, eleve_id) WHERE eleve_id IS NOT NULL;

CREATE OR REPLACE FUNCTION ent_doc_signataire_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_signataire : suppression interdite (un signataire se remplace ou se retire)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.circuit_id IS DISTINCT FROM OLD.circuit_id
       OR NEW.rang IS DISTINCT FROM OLD.rang
       OR NEW.role IS DISTINCT FROM OLD.role
       OR NEW.libelle_role IS DISTINCT FROM OLD.libelle_role
       OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.externe_nom IS DISTINCT FROM OLD.externe_nom
       OR NEW.externe_organisation IS DISTINCT FROM OLD.externe_organisation
       OR NEW.externe_contact_scelle IS DISTINCT FROM OLD.externe_contact_scelle
       OR NEW.choix_par IS DISTINCT FROM OLD.choix_par
       OR NEW.lien_jeton_sha256 IS DISTINCT FROM OLD.lien_jeton_sha256
       OR NEW.lien_expire_le IS DISTINCT FROM OLD.lien_expire_le
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'ent_doc_signataire : identité du signataire immuable (le remplacer par un autre)' USING ERRCODE = '23001';
    END IF;
    -- Un état final (signé, refusé, retiré, remplacé, expiré) ne bouge plus ; « requis » peut passer à faux (un parent
    -- suffit) mais jamais à vrai après coup.
    IF OLD.etat IN ('signe', 'refuse', 'retire', 'remplace', 'expire')
       AND (NEW.etat IS DISTINCT FROM OLD.etat OR NEW.refus_motif IS DISTINCT FROM OLD.refus_motif
            OR NEW.papier_cle_objet IS DISTINCT FROM OLD.papier_cle_objet) THEN
        RAISE EXCEPTION 'ent_doc_signataire : état % final', OLD.etat USING ERRCODE = '23001';
    END IF;
    IF NEW.requis AND NOT OLD.requis THEN
        RAISE EXCEPTION 'ent_doc_signataire : un signataire non bloquant ne redevient pas requis' USING ERRCODE = '23001';
    END IF;
    IF NEW.etat IS DISTINCT FROM OLD.etat THEN
        NEW.etat_le := now();
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_signataire_garde_trg ON ent_doc_signataire;
CREATE TRIGGER ent_doc_signataire_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_signataire
    FOR EACH ROW EXECUTE FUNCTION ent_doc_signataire_garde();

-- Modes autorisés d'un signataire (sous-ensemble du menu de la nature ; 1 à 4).
CREATE TABLE IF NOT EXISTS ent_doc_signataire_mode (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    signataire_id UUID NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('passkey', 'code_sms', 'guichet', 'papier')),
    PRIMARY KEY (signataire_id, mode),
    CONSTRAINT ent_doc_signataire_mode_signataire_cfk FOREIGN KEY (etablissement_id, signataire_id)
        REFERENCES ent_doc_signataire (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_doc_signataire_mode_ajout_seul_trg ON ent_doc_signataire_mode;
CREATE TRIGGER ent_doc_signataire_mode_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_signataire_mode
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

-- ───────────────────────── Défis (passkey, code à usage unique) ─────────────────────────
-- Un défi lie, côté serveur, ce qui va être signé (empreinte de la version, intention affichée, signataire, nonce) au
-- défi d'authentification. Le standard WebAuthn traite le défi comme des octets opaques tirés par l'authentifieur
-- côté serveur : le lien `lien_sha256` = SHA-256(défi ‖ empreinte ‖ signataire ‖ intention ‖ nonce) est écrit ici
-- puis scellé dans la chaîne des signatures. Se consomme UNE fois.
CREATE TABLE IF NOT EXISTS ent_doc_defi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    circuit_id UUID NOT NULL,
    signataire_id UUID NOT NULL,
    version_id UUID NOT NULL,
    methode TEXT NOT NULL CHECK (methode IN ('passkey', 'code_sms')),
    empreinte_signee TEXT NOT NULL CHECK (empreinte_signee ~ '^[0-9a-f]{64}$'),
    intention_texte TEXT NOT NULL CHECK (length(intention_texte) BETWEEN 10 AND 600),
    nonce TEXT NOT NULL CHECK (nonce ~ '^[0-9a-f]{32}$'),
    -- passkey : défi WebAuthn (base64url) et état d'authentification sérialisé ; code : empreinte du code.
    defi_webauthn TEXT,
    etat_webauthn TEXT,
    code_sha256 TEXT CHECK (code_sha256 IS NULL OR code_sha256 ~ '^[0-9a-f]{64}$'),
    canal_masque TEXT CHECK (canal_masque IS NULL OR length(canal_masque) <= 60),
    lien_sha256 TEXT CHECK (lien_sha256 IS NULL OR lien_sha256 ~ '^[0-9a-f]{64}$'),
    tentatives SMALLINT NOT NULL DEFAULT 0 CHECK (tentatives BETWEEN 0 AND 10),
    -- Au guichet : l'agent qui a lancé le défi (le code part au téléphone de la personne, il est saisi sur la tablette).
    agent_user_id UUID,
    demande_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    expire_le TIMESTAMPTZ NOT NULL,
    consomme_le TIMESTAMPTZ,
    CONSTRAINT ent_doc_defi_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_defi_signataire_cfk FOREIGN KEY (etablissement_id, signataire_id)
        REFERENCES ent_doc_signataire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_defi_circuit_cfk FOREIGN KEY (etablissement_id, circuit_id)
        REFERENCES ent_doc_circuit (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_defi_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_defi_methode_chk CHECK (
        (methode = 'passkey' AND defi_webauthn IS NOT NULL AND etat_webauthn IS NOT NULL AND code_sha256 IS NULL)
        OR (methode = 'code_sms' AND code_sha256 IS NOT NULL AND defi_webauthn IS NULL AND etat_webauthn IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_doc_defi_signataire_idx ON ent_doc_defi (etablissement_id, signataire_id, cree_le DESC);

CREATE OR REPLACE FUNCTION ent_doc_defi_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_defi : suppression interdite' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.circuit_id IS DISTINCT FROM OLD.circuit_id
       OR NEW.signataire_id IS DISTINCT FROM OLD.signataire_id
       OR NEW.version_id IS DISTINCT FROM OLD.version_id
       OR NEW.methode IS DISTINCT FROM OLD.methode
       OR NEW.empreinte_signee IS DISTINCT FROM OLD.empreinte_signee
       OR NEW.intention_texte IS DISTINCT FROM OLD.intention_texte
       OR NEW.nonce IS DISTINCT FROM OLD.nonce
       OR NEW.defi_webauthn IS DISTINCT FROM OLD.defi_webauthn
       OR NEW.etat_webauthn IS DISTINCT FROM OLD.etat_webauthn
       OR NEW.code_sha256 IS DISTINCT FROM OLD.code_sha256
       OR NEW.lien_sha256 IS DISTINCT FROM OLD.lien_sha256
       OR NEW.agent_user_id IS DISTINCT FROM OLD.agent_user_id
       OR NEW.demande_par IS DISTINCT FROM OLD.demande_par
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le
       OR NEW.expire_le IS DISTINCT FROM OLD.expire_le THEN
        RAISE EXCEPTION 'ent_doc_defi : seuls les essais et la consommation bougent' USING ERRCODE = '23001';
    END IF;
    IF OLD.consomme_le IS NOT NULL AND (NEW.consomme_le IS DISTINCT FROM OLD.consomme_le OR NEW.tentatives <> OLD.tentatives) THEN
        RAISE EXCEPTION 'ent_doc_defi : défi déjà consommé (rejeu refusé)' USING ERRCODE = '23001';
    END IF;
    IF NEW.tentatives < OLD.tentatives THEN
        RAISE EXCEPTION 'ent_doc_defi : les essais ne se remettent pas à zéro' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_defi_garde_trg ON ent_doc_defi;
CREATE TRIGGER ent_doc_defi_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_defi
    FOR EACH ROW EXECUTE FUNCTION ent_doc_defi_garde();

-- ───────────────────────── Signatures (ajout seul, chaînées) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_signature (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    circuit_id UUID NOT NULL,
    signataire_id UUID NOT NULL,
    version_id UUID NOT NULL,
    rang_chaine INT NOT NULL CHECK (rang_chaine >= 1),
    empreinte_signee TEXT NOT NULL CHECK (empreinte_signee ~ '^[0-9a-f]{64}$'),
    methode TEXT NOT NULL CHECK (methode IN ('passkey', 'code_sms', 'guichet', 'papier')),
    -- Niveau de preuve obtenu : S1 (accord simple : code), S2 (passkey), G (guichet, agent + confirmation),
    -- G0 (tablette sans confirmation : simple, affichée comme telle), P (papier contrôlé par un humain).
    niveau TEXT NOT NULL CHECK (niveau IN ('S1', 'S2', 'G', 'G0', 'P')),
    intention_texte TEXT NOT NULL CHECK (length(intention_texte) BETWEEN 10 AND 600),
    intention_sha256 TEXT NOT NULL CHECK (intention_sha256 ~ '^[0-9a-f]{64}$'),
    horodatage_serveur TIMESTAMPTZ NOT NULL,
    defi_id UUID,
    lien_sha256 TEXT CHECK (lien_sha256 IS NULL OR lien_sha256 ~ '^[0-9a-f]{64}$'),
    -- Passkey : les éléments bruts de l'assertion, pour expertise (le défi figure dans `client_data_json`).
    passkey_credential_id TEXT CHECK (passkey_credential_id IS NULL OR length(passkey_credential_id) <= 400),
    client_data_json TEXT CHECK (client_data_json IS NULL OR length(client_data_json) <= 4000),
    authenticator_data_b64 TEXT CHECK (authenticator_data_b64 IS NULL OR length(authenticator_data_b64) <= 2000),
    assertion_signature_b64 TEXT CHECK (assertion_signature_b64 IS NULL OR length(assertion_signature_b64) <= 2000),
    -- Code : canal masqué seulement (« 06 ** ** ** 12 »).
    code_canal_masque TEXT CHECK (code_canal_masque IS NULL OR length(code_canal_masque) <= 60),
    -- Guichet : agent identifié, identité vue (type de pièce, jamais copiée), confirmation, tracé (objet scellé).
    constate_par_user_id UUID,
    identite_vue TEXT CHECK (identite_vue IS NULL OR identite_vue IN
        ('carte_identite', 'passeport', 'titre_sejour', 'permis', 'carte_professionnelle', 'autre')),
    confirmation TEXT CHECK (confirmation IS NULL OR confirmation IN ('code', 'aucune')),
    trace_sha256 TEXT CHECK (trace_sha256 IS NULL OR trace_sha256 ~ '^[0-9a-f]{64}$'),
    trace_cle_objet TEXT,
    -- Papier : scan contrôlé (objet), contrôleur.
    papier_cle_objet TEXT,
    papier_sha256 TEXT CHECK (papier_sha256 IS NULL OR papier_sha256 ~ '^[0-9a-f]{64}$'),
    -- Signature de l'établissement par délégation : la formule imprimée.
    formule_delegation TEXT CHECK (formule_delegation IS NULL OR length(formule_delegation) <= 200),
    compte_user_id UUID,
    precedent_sha256 TEXT NOT NULL CHECK (precedent_sha256 ~ '^[0-9a-f]{64}$'),
    entree_sha256 TEXT NOT NULL CHECK (entree_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT ent_doc_signature_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_signature_rang_uq UNIQUE (circuit_id, rang_chaine),
    CONSTRAINT ent_doc_signature_signataire_uq UNIQUE (signataire_id),
    CONSTRAINT ent_doc_signature_entree_uq UNIQUE (entree_sha256),
    CONSTRAINT ent_doc_signature_circuit_cfk FOREIGN KEY (etablissement_id, circuit_id)
        REFERENCES ent_doc_circuit (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signature_signataire_cfk FOREIGN KEY (etablissement_id, signataire_id)
        REFERENCES ent_doc_signataire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signature_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signature_defi_cfk FOREIGN KEY (etablissement_id, defi_id)
        REFERENCES ent_doc_defi (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signature_trace_cfk FOREIGN KEY (etablissement_id, trace_cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signature_papier_cfk FOREIGN KEY (etablissement_id, papier_cle_objet)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_signature_methode_niveau_chk CHECK (
        (methode = 'passkey' AND niveau = 'S2' AND passkey_credential_id IS NOT NULL AND client_data_json IS NOT NULL
            AND authenticator_data_b64 IS NOT NULL AND assertion_signature_b64 IS NOT NULL AND defi_id IS NOT NULL)
        OR (methode = 'code_sms' AND niveau = 'S1' AND code_canal_masque IS NOT NULL AND defi_id IS NOT NULL)
        OR (methode = 'guichet' AND niveau IN ('G', 'G0') AND constate_par_user_id IS NOT NULL AND identite_vue IS NOT NULL
            AND confirmation IS NOT NULL AND (niveau = 'G') = (confirmation = 'code') AND (niveau = 'G') = (defi_id IS NOT NULL))
        OR (methode = 'papier' AND niveau = 'P' AND papier_cle_objet IS NOT NULL AND papier_sha256 IS NOT NULL
            AND constate_par_user_id IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_doc_signature_circuit_idx ON ent_doc_signature (etablissement_id, circuit_id, rang_chaine);

-- Chaînage : le rang suit le dernier du circuit et l'empreinte précédente est celle de cette dernière entrée (la
-- première porte SHA-256 du texte « scolia.circuit: » suivi de l'identifiant du circuit, calculé par le service).
-- Le contrôle du calcul de `entree_sha256` lui-même est refait par `verifier_chaine` (service) : la base garantit la
-- continuité, ajout seul compris, le service le calcul.
CREATE OR REPLACE FUNCTION ent_doc_signature_chainage() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_rang INT;
    v_prec TEXT;
BEGIN
    -- Sérialise les signatures d'un même circuit (deux signatures simultanées ne se partagent jamais un rang).
    PERFORM pg_advisory_xact_lock(hashtextextended('ent_doc_signature:' || NEW.circuit_id::text, 0));
    SELECT rang_chaine, entree_sha256 INTO v_rang, v_prec
      FROM ent_doc_signature WHERE circuit_id = NEW.circuit_id ORDER BY rang_chaine DESC LIMIT 1;
    IF v_rang IS NULL THEN
        IF NEW.rang_chaine <> 1 THEN
            RAISE EXCEPTION 'ent_doc_signature : la première entrée d''un circuit porte le rang 1' USING ERRCODE = '23001';
        END IF;
    ELSE
        IF NEW.rang_chaine <> v_rang + 1 OR NEW.precedent_sha256 <> v_prec THEN
            RAISE EXCEPTION 'ent_doc_signature : chaîne rompue (rang % attendu, empreinte précédente à reprendre)', v_rang + 1
                USING ERRCODE = '23001';
        END IF;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_doc_circuit c
                    WHERE c.etablissement_id = NEW.etablissement_id AND c.id = NEW.circuit_id
                      AND c.version_id = NEW.version_id AND c.etat = 'ouvert') THEN
        RAISE EXCEPTION 'ent_doc_signature : circuit clos ou version différente (une signature porte sur la version du circuit)'
            USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_signature_chainage_trg ON ent_doc_signature;
CREATE TRIGGER ent_doc_signature_chainage_trg
    BEFORE INSERT ON ent_doc_signature
    FOR EACH ROW EXECUTE FUNCTION ent_doc_signature_chainage();
DROP TRIGGER IF EXISTS ent_doc_signature_ajout_seul_trg ON ent_doc_signature;
CREATE TRIGGER ent_doc_signature_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_signature
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

-- ───────────────────────── Horodatage qualifié (relevé de consommation) ─────────────────────────
-- Une ligne par jeton DEMANDÉ au service (mutualisé ou propre) : le relevé de consommation par établissement se lit
-- ici. Le jeton RFC 3161 (DER, hexadécimal) est conservé tel quel, pour une vérification ultérieure par un outil tiers.
CREATE TABLE IF NOT EXISTS ent_doc_horodatage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    circuit_id UUID NOT NULL,
    empreinte_horodatee TEXT NOT NULL CHECK (empreinte_horodatee ~ '^[0-9a-f]{64}$'),
    mode TEXT NOT NULL CHECK (mode IN ('mutualise', 'propre')),
    etat TEXT NOT NULL DEFAULT 'en_attente' CHECK (etat IN ('en_attente', 'obtenu', 'echec')),
    jeton_hex TEXT CHECK (jeton_hex IS NULL OR length(jeton_hex) <= 80000),
    instant_jeton TIMESTAMPTZ,
    tentatives SMALLINT NOT NULL DEFAULT 0 CHECK (tentatives BETWEEN 0 AND 20),
    derniere_erreur TEXT CHECK (derniere_erreur IS NULL OR length(derniere_erreur) <= 300),
    demande_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    obtenu_le TIMESTAMPTZ,
    CONSTRAINT ent_doc_horodatage_circuit_cfk FOREIGN KEY (etablissement_id, circuit_id)
        REFERENCES ent_doc_circuit (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_horodatage_obtenu_chk CHECK ((etat = 'obtenu') = (jeton_hex IS NOT NULL AND obtenu_le IS NOT NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_horodatage_circuit_uq ON ent_doc_horodatage (circuit_id);
CREATE INDEX IF NOT EXISTS ent_doc_horodatage_releve_idx ON ent_doc_horodatage (etablissement_id, demande_le DESC);

CREATE OR REPLACE FUNCTION ent_doc_horodatage_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_horodatage : suppression interdite (relevé de consommation)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.circuit_id IS DISTINCT FROM OLD.circuit_id
       OR NEW.empreinte_horodatee IS DISTINCT FROM OLD.empreinte_horodatee
       OR NEW.mode IS DISTINCT FROM OLD.mode
       OR NEW.demande_le IS DISTINCT FROM OLD.demande_le THEN
        RAISE EXCEPTION 'ent_doc_horodatage : identité de la demande immuable' USING ERRCODE = '23001';
    END IF;
    IF OLD.etat = 'obtenu' AND ROW(NEW.*) IS DISTINCT FROM ROW(OLD.*) THEN
        RAISE EXCEPTION 'ent_doc_horodatage : jeton obtenu, plus aucune modification' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_horodatage_garde_trg ON ent_doc_horodatage;
CREATE TRIGGER ent_doc_horodatage_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_horodatage
    FOR EACH ROW EXECUTE FUNCTION ent_doc_horodatage_garde();

SELECT ent_rls_poser(ARRAY[
    'ent_doc_signature_menu',
    'ent_doc_horodatage_nature',
    'ent_doc_horodatage_service',
    'ent_doc_circuit',
    'ent_doc_signataire',
    'ent_doc_signataire_mode',
    'ent_doc_defi',
    'ent_doc_signature',
    'ent_doc_horodatage'
]);
