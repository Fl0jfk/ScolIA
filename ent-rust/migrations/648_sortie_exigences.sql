-- 648 — Socle 42, lot V1 (1/2) : exigences par participant (moteur), campagne d'autorisations (port vers le
-- socle 18), faits de signature (formulaire, guichet, attestation de reprise), dérogations, désaccords,
-- relances, ordres de mission. Réf. : 42_SORTIES_VOYAGES.md §9.3, §10, §11.2, §19.2, §19.3, §19.5,
-- VOY-139, VOY-149, VOY-152, VOY-171 à VOY-173, VOY-260, VOY-277 ; R-SV6, R-SV9, R-SV40 ; CV-04 à CV-09,
-- CV-25, CV-26.
--
-- Exigences E1 à E14 (PRA) : `etablissement_id` NOT NULL partout, clés étrangères COMPOSITES, UUID fournis par
-- l'application, aucun BYTEA (les scans vont au magasin d'objets), aucun JSON (D-036), horodatages, tables
-- d'ajout seul pour les faits, migration rejouable (aucune modification des migrations 640 à 647), RLS posée
-- en fin de fichier.
--
-- * Le socle 18 (formulaires et autorisations) est codé en parallèle : AUCUNE dépendance de code. La campagne
--   se demande par un fait sortant (`voyages.campagne_ouverte`) ; les décisions reviennent par la commande
--   d'entrée `voyages_autorisation_decision_recue` (port). Le guichet et l'attestation de reprise écrivent
--   les mêmes faits, avec leur source.
-- * Un fait de signature est en AJOUT SEUL ; l'état de l'exigence est recalculé par le serveur.

CREATE TABLE IF NOT EXISTS ent_sortie_campagne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    -- La participation est TOUJOURS demandée pour une sortie facultative ; image et soins sont des
    -- consentements distincts (un formulaire, des consentements séparés et révocables, §11.2).
    avec_image BOOLEAN NOT NULL DEFAULT false,
    avec_soins BOOLEAN NOT NULL DEFAULT false,
    statut TEXT NOT NULL DEFAULT 'ouverte' CHECK (statut IN ('ouverte', 'close')),
    -- Référence du formulaire côté socle 18, posée par lui à l'ouverture effective.
    reference_externe TEXT CHECK (reference_externe IS NULL OR length(reference_externe) <= 100),
    ouverte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    ouverte_par_user_id UUID,
    close_le TIMESTAMPTZ,
    CONSTRAINT ent_sortie_campagne_close_chk CHECK ((statut = 'close') = (close_le IS NOT NULL)),
    CONSTRAINT ent_sortie_campagne_sortie_uq UNIQUE (sortie_id),
    CONSTRAINT ent_sortie_campagne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_campagne_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);

-- Une exigence par (participant, code) : son état est CALCULÉ par le serveur à partir des faits.
CREATE TABLE IF NOT EXISTS ent_sortie_exigence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN (
        'autorisation_participation', 'desaccord_titulaire', 'autorisation_soins', 'droit_image')),
    etat TEXT NOT NULL DEFAULT 'attendue' CHECK (etat IN (
        'attendue', 'remplie', 'derogee', 'refusee', 'a_revoir', 'levee')),
    bloquante BOOLEAN NOT NULL DEFAULT false,
    source TEXT CHECK (source IS NULL OR source IN (
        'formulaire', 'guichet', 'attestation_reprise', 'decision_direction', 'derogation')),
    signataire_personne_id UUID,
    signee_le DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_exigence_uq UNIQUE (participant_id, code),
    CONSTRAINT ent_sortie_exigence_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_exigence_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_exigence_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_exigence_signataire_fk FOREIGN KEY (etablissement_id, signataire_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_exigence_sortie_idx
    ON ent_sortie_exigence (etablissement_id, sortie_id, code, etat);

-- Faits d'une exigence, en AJOUT SEUL : ce qui a été signé (ou refusé), quand, par qui, avec quelle source.
-- La date de signature d'un coupon papier (`signee_le`) n'est pas la date de saisie (`cree_le`).
CREATE TABLE IF NOT EXISTS ent_sortie_exigence_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    exigence_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'formulaire', 'guichet', 'attestation_reprise', 'derogation', 'declaration_desaccord',
        'decision_desaccord')),
    decision TEXT CHECK (decision IS NULL OR decision IN ('accepte', 'refuse', 'retire')),
    signataire_personne_id UUID,
    signee_le DATE,
    acteur_user_id UUID,
    -- Motif d'une dérogation ou d'une décision de direction : jamais montré à l'organisateur.
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 1000),
    -- Scan d'une signature papier : objet du magasin, jamais en base.
    scan_cle_objet TEXT,
    scan_nom TEXT CHECK (scan_nom IS NULL OR length(scan_nom) BETWEEN 1 AND 200),
    scan_mime TEXT,
    scan_taille INTEGER CHECK (scan_taille IS NULL OR scan_taille > 0),
    scan_sha256 TEXT CHECK (scan_sha256 IS NULL OR scan_sha256 ~ '^[0-9a-f]{64}$'),
    reference_externe TEXT CHECK (reference_externe IS NULL OR length(reference_externe) <= 100),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_exigence_fait_signature_chk CHECK (
        (type IN ('formulaire', 'guichet', 'attestation_reprise')) = (decision IS NOT NULL AND signataire_personne_id IS NOT NULL
                                                                       AND signee_le IS NOT NULL)),
    CONSTRAINT ent_sortie_exigence_fait_motif_chk CHECK (
        type NOT IN ('derogation', 'decision_desaccord') OR motif IS NOT NULL),
    CONSTRAINT ent_sortie_exigence_fait_scan_chk CHECK (
        (scan_cle_objet IS NULL) = (scan_nom IS NULL AND scan_mime IS NULL AND scan_taille IS NULL AND scan_sha256 IS NULL)
        OR (scan_cle_objet IS NOT NULL AND scan_nom IS NOT NULL AND scan_mime IS NOT NULL
            AND scan_taille IS NOT NULL AND scan_sha256 IS NOT NULL)),
    CONSTRAINT ent_sortie_exigence_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_exigence_fait_exigence_fk FOREIGN KEY (etablissement_id, exigence_id)
        REFERENCES ent_sortie_exigence (etablissement_id, id),
    CONSTRAINT ent_sortie_exigence_fait_signataire_fk FOREIGN KEY (etablissement_id, signataire_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_exigence_fait_exigence_idx
    ON ent_sortie_exigence_fait (etablissement_id, exigence_id, cree_le);
DROP TRIGGER IF EXISTS ent_sortie_exigence_fait_immuable ON ent_sortie_exigence_fait;
CREATE TRIGGER ent_sortie_exigence_fait_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_exigence_fait
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Relances d'autorisations (J-21, J-10, J-5 et gestes manuels), en AJOUT SEUL : une relance automatique par
-- seuil et par sortie au plus (idempotence) ; `omise` = seuil dépassé rattrapé par un seuil plus proche.
CREATE TABLE IF NOT EXISTS ent_sortie_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    seuil TEXT NOT NULL CHECK (seuil IN ('J-21', 'J-10', 'J-5', 'manuelle')),
    omise BOOLEAN NOT NULL DEFAULT false,
    destinataires_comptes INTEGER NOT NULL DEFAULT 0 CHECK (destinataires_comptes >= 0),
    destinataires_sans_compte INTEGER NOT NULL DEFAULT 0 CHECK (destinataires_sans_compte >= 0),
    acteur_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_relance_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_relance_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_relance_seuil_uq
    ON ent_sortie_relance (sortie_id, seuil) WHERE seuil <> 'manuelle';
DROP TRIGGER IF EXISTS ent_sortie_relance_immuable ON ent_sortie_relance;
CREATE TRIGGER ent_sortie_relance_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_relance
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Ordres de mission et autorisations écrites des adultes (VOY-152) : documents PAR PERSONNE. Le type du
-- document s'étend et le document peut désigner l'adulte concerné (clé composite).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_document_type_v1_chk') THEN
        ALTER TABLE ent_sortie_document DROP CONSTRAINT IF EXISTS ent_sortie_document_type_v0c_chk;
        ALTER TABLE ent_sortie_document DROP CONSTRAINT IF EXISTS ent_sortie_document_type_check;
        ALTER TABLE ent_sortie_document ADD CONSTRAINT ent_sortie_document_type_v1_chk CHECK (
            type IN ('budget', 'circulaire', 'demande_devis', 'devis', 'bon_commande', 'avenant',
                     'ordre_mission', 'autorisation_adulte', 'liste_nominative'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = 'ent_sortie_document'
                      AND column_name = 'personne_id') THEN
        ALTER TABLE ent_sortie_document ADD COLUMN personne_id UUID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_document_personne_fk') THEN
        ALTER TABLE ent_sortie_document ADD CONSTRAINT ent_sortie_document_personne_fk
            FOREIGN KEY (etablissement_id, personne_id) REFERENCES ent_personne (etablissement_id, id);
    END IF;
END
$$;

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_campagne',
    'ent_sortie_exigence',
    'ent_sortie_exigence_fait',
    'ent_sortie_relance'
]);
