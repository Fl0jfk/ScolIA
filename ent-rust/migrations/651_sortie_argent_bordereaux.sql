-- 651 — Socle 42, lot V2a (2/2) : bordereau de remise de chèques (VOY-266, R-SV45, S-438), acomptes fournisseurs
-- (VOY-270), exports vers la comptabilité (VOY-268), accès nominatif aux paiements (VOY-223, §21.2).
-- Réf. : 42_SORTIES_VOYAGES.md §12.3, §21.2, §32.5, §32.7.
--
-- Mêmes exigences que 650 : `etablissement_id` NOT NULL, clés étrangères composites, UUID applicatifs, aucun JSON,
-- aucun BYTEA, centimes entiers bornés, rejouable, RLS en fin de fichier.
--
-- * Un bordereau est saisi par l'ORGANISATEUR avec le nombre et le total des chèques, nominatif (élève, numéro de
--   chèque, montant), puis remis au SECRÉTARIAT le jour même (jour de l'établissement). L'écart entre bordereau et
--   remise et le retard sont calculés PAR LA BASE (déclencheur) : aucun code ne peut les omettre.
-- * Les acomptes fournisseurs sont des engagements de ScolIA envers l'agence ou l'hébergeur, à suivre (à payer,
--   payé, en retard) ; l'alerte avant échéance est posée une seule fois.
-- * Les accès nominatifs (liste de paiements, export) sont journalisés en ajout seul.

CREATE TABLE IF NOT EXISTS ent_sortie_bordereau (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    -- Jour de la collecte = jour de la saisie, dans le fuseau de l'établissement (posé par le serveur).
    jour_collecte DATE NOT NULL,
    nb_cheques INTEGER NOT NULL CHECK (nb_cheques BETWEEN 1 AND 500),
    total_centimes BIGINT NOT NULL CHECK (total_centimes BETWEEN 1 AND 500000000),
    saisi_par_user_id UUID NOT NULL,
    saisi_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_bordereau_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_bordereau_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_bordereau_sortie_idx
    ON ent_sortie_bordereau (etablissement_id, sortie_id, jour_collecte);

CREATE TABLE IF NOT EXISTS ent_sortie_bordereau_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    bordereau_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    numero_cheque TEXT NOT NULL CHECK (length(btrim(numero_cheque)) BETWEEN 1 AND 30),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 10000000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_bordereau_ligne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_bordereau_ligne_bordereau_fk FOREIGN KEY (etablissement_id, bordereau_id)
        REFERENCES ent_sortie_bordereau (etablissement_id, id),
    CONSTRAINT ent_sortie_bordereau_ligne_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_bordereau_ligne_idx
    ON ent_sortie_bordereau_ligne (etablissement_id, bordereau_id);

-- Remise au secrétariat : une seule par bordereau ; l'écart et le retard sont posés par la base.
CREATE TABLE IF NOT EXISTS ent_sortie_bordereau_remise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    bordereau_id UUID NOT NULL,
    nb_recus INTEGER NOT NULL CHECK (nb_recus BETWEEN 0 AND 500),
    total_recu_centimes BIGINT NOT NULL CHECK (total_recu_centimes BETWEEN 0 AND 500000000),
    jour_remise DATE NOT NULL,
    ecart BOOLEAN NOT NULL DEFAULT false,
    en_retard BOOLEAN NOT NULL DEFAULT false,
    note TEXT CHECK (note IS NULL OR length(btrim(note)) BETWEEN 1 AND 500),
    recu_par_user_id UUID NOT NULL,
    remis_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_bordereau_remise_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_bordereau_remise_uq UNIQUE (bordereau_id),
    -- Un écart se documente (qui compte quoi) : note obligatoire.
    CONSTRAINT ent_sortie_bordereau_remise_ecart_chk CHECK (NOT ecart OR note IS NOT NULL),
    CONSTRAINT ent_sortie_bordereau_remise_fk FOREIGN KEY (etablissement_id, bordereau_id)
        REFERENCES ent_sortie_bordereau (etablissement_id, id)
);

-- Le lien « une ligne de bordereau a produit ce versement » (index d'unicité posé en 650).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_versement_bordereau_ligne_fk') THEN
        ALTER TABLE ent_sortie_versement ADD CONSTRAINT ent_sortie_versement_bordereau_ligne_fk
            FOREIGN KEY (etablissement_id, bordereau_ligne_id)
            REFERENCES ent_sortie_bordereau_ligne (etablissement_id, id);
    END IF;
END
$$;

-- Acomptes dus aux fournisseurs (VOY-270).
CREATE TABLE IF NOT EXISTS ent_sortie_acompte_fournisseur (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    prestataire_id UUID NOT NULL,
    commande_id UUID,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 100000000000),
    echeance DATE NOT NULL,
    -- Alerte envoyée à la comptabilité : posée UNE fois.
    alerte_le DATE,
    paye_le DATE,
    paye_centimes BIGINT CHECK (paye_centimes IS NULL OR paye_centimes BETWEEN 1 AND 100000000000),
    paye_reference TEXT CHECK (paye_reference IS NULL OR length(btrim(paye_reference)) BETWEEN 1 AND 60),
    paye_par_user_id UUID,
    annule_le DATE,
    annule_motif TEXT CHECK (annule_motif IS NULL OR length(btrim(annule_motif)) BETWEEN 1 AND 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID NOT NULL,
    CONSTRAINT ent_sortie_acompte_fournisseur_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_acompte_fournisseur_paye_chk CHECK (
        (paye_le IS NULL) = (paye_centimes IS NULL AND paye_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_acompte_fournisseur_annule_chk CHECK ((annule_le IS NULL) = (annule_motif IS NULL)),
    CONSTRAINT ent_sortie_acompte_fournisseur_exclusif_chk CHECK (paye_le IS NULL OR annule_le IS NULL),
    CONSTRAINT ent_sortie_acompte_fournisseur_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_acompte_fournisseur_prestataire_fk FOREIGN KEY (etablissement_id, prestataire_id)
        REFERENCES ent_prestataire (etablissement_id, id),
    CONSTRAINT ent_sortie_acompte_fournisseur_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_commande (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_acompte_fournisseur_echeance_idx
    ON ent_sortie_acompte_fournisseur (etablissement_id, echeance)
    WHERE paye_le IS NULL AND annule_le IS NULL;

-- Exports vers la comptabilité (VOY-268) : un fichier CSV par export, empreinte gardée, journalisé.
CREATE TABLE IF NOT EXISTS ent_sortie_argent_export (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    nb_lignes INTEGER NOT NULL CHECK (nb_lignes BETWEEN 0 AND 1000000),
    cle_objet TEXT NOT NULL CHECK (length(cle_objet) BETWEEN 1 AND 300),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    cree_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_argent_export_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_export_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);

-- Ouverture de la liste nominative des paiements au seul responsable du séjour (VOY-223, S-161 du 24), par la
-- direction, avec motif ; refermée sans suppression. Une seule ouverture en cours par sortie.
CREATE TABLE IF NOT EXISTS ent_sortie_nominatif_ouverture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    -- Le responsable désigné : le porteur de la sortie à l'ouverture.
    personne_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 500),
    ouvert_par_user_id UUID NOT NULL,
    ouvert_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    ferme_le TIMESTAMPTZ,
    ferme_par_user_id UUID,
    CONSTRAINT ent_sortie_nominatif_ouverture_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_nominatif_ouverture_ferme_chk CHECK ((ferme_le IS NULL) = (ferme_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_nominatif_ouverture_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_nominatif_ouverture_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_nominatif_ouverture_en_cours_uq
    ON ent_sortie_nominatif_ouverture (sortie_id) WHERE ferme_le IS NULL;

-- Chaque accès nominatif est journalisé (qui, quand, quoi), en ajout seul.
CREATE TABLE IF NOT EXISTS ent_sortie_argent_acces (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    user_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (length(role) BETWEEN 1 AND 40),
    portee TEXT NOT NULL CHECK (portee IN ('liste_paiements', 'export', 'avoirs', 'bordereaux')),
    nb_lignes INTEGER NOT NULL DEFAULT 0 CHECK (nb_lignes >= 0),
    a TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_argent_acces_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_argent_acces_idx
    ON ent_sortie_argent_acces (etablissement_id, sortie_id, a);

-- ───────────────────────── Garde-fous de la base ─────────────────────────

DROP TRIGGER IF EXISTS ent_sortie_bordereau_immuable ON ent_sortie_bordereau;
CREATE TRIGGER ent_sortie_bordereau_immuable BEFORE UPDATE OR DELETE ON ent_sortie_bordereau
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_bordereau_ligne_immuable ON ent_sortie_bordereau_ligne;
CREATE TRIGGER ent_sortie_bordereau_ligne_immuable BEFORE UPDATE OR DELETE ON ent_sortie_bordereau_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_bordereau_remise_immuable ON ent_sortie_bordereau_remise;
CREATE TRIGGER ent_sortie_bordereau_remise_immuable BEFORE UPDATE OR DELETE ON ent_sortie_bordereau_remise
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_argent_export_immuable ON ent_sortie_argent_export;
CREATE TRIGGER ent_sortie_argent_export_immuable BEFORE UPDATE OR DELETE ON ent_sortie_argent_export
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_argent_acces_immuable ON ent_sortie_argent_acces;
CREATE TRIGGER ent_sortie_argent_acces_immuable BEFORE UPDATE OR DELETE ON ent_sortie_argent_acces
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_bordereau_vidage ON ent_sortie_bordereau;
CREATE TRIGGER ent_sortie_bordereau_vidage BEFORE TRUNCATE ON ent_sortie_bordereau
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_bordereau_ligne_vidage ON ent_sortie_bordereau_ligne;
CREATE TRIGGER ent_sortie_bordereau_ligne_vidage BEFORE TRUNCATE ON ent_sortie_bordereau_ligne
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_bordereau_remise_vidage ON ent_sortie_bordereau_remise;
CREATE TRIGGER ent_sortie_bordereau_remise_vidage BEFORE TRUNCATE ON ent_sortie_bordereau_remise
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_argent_acces_vidage ON ent_sortie_argent_acces;
CREATE TRIGGER ent_sortie_argent_acces_vidage BEFORE TRUNCATE ON ent_sortie_argent_acces
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Bordereau : le nombre et le total déclarés doivent être EXACTEMENT ceux des lignes, au COMMIT (les lignes sont
-- insérées après l'en-tête, dans la même transaction). Un bordereau qui ne s'additionne pas n'existe pas.
CREATE OR REPLACE FUNCTION ent_sortie_bordereau_somme() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    n INTEGER;
    s BIGINT;
BEGIN
    SELECT count(*), COALESCE(sum(montant_centimes), 0) INTO n, s
      FROM ent_sortie_bordereau_ligne
     WHERE etablissement_id = NEW.etablissement_id AND bordereau_id = NEW.id;
    IF n <> NEW.nb_cheques OR s <> NEW.total_centimes THEN
        RAISE EXCEPTION 'le bordereau ne correspond pas à ses lignes (nombre ou total)' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_bordereau_somme_tg ON ent_sortie_bordereau;
CREATE CONSTRAINT TRIGGER ent_sortie_bordereau_somme_tg AFTER INSERT ON ent_sortie_bordereau
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ent_sortie_bordereau_somme();

-- Remise : l'écart (nombre ou total différents du bordereau) et le retard (remise un autre jour que la collecte)
-- sont CALCULÉS ici, quelles que soient les valeurs fournies.
CREATE OR REPLACE FUNCTION ent_sortie_bordereau_remise_calcul() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    b ent_sortie_bordereau%ROWTYPE;
BEGIN
    SELECT * INTO b FROM ent_sortie_bordereau
     WHERE etablissement_id = NEW.etablissement_id AND id = NEW.bordereau_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'bordereau introuvable' USING ERRCODE = '23503';
    END IF;
    NEW.ecart := (NEW.nb_recus <> b.nb_cheques OR NEW.total_recu_centimes <> b.total_centimes);
    NEW.en_retard := (NEW.jour_remise > b.jour_collecte);
    IF NEW.jour_remise < b.jour_collecte THEN
        RAISE EXCEPTION 'la remise précède la collecte' USING ERRCODE = '23514';
    END IF;
    IF NEW.ecart AND NEW.note IS NULL THEN
        RAISE EXCEPTION 'un écart de remise se documente' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_bordereau_remise_calcul_tg ON ent_sortie_bordereau_remise;
CREATE TRIGGER ent_sortie_bordereau_remise_calcul_tg BEFORE INSERT ON ent_sortie_bordereau_remise
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_bordereau_remise_calcul();

-- Acompte fournisseur : jamais supprimé ; seuls l'alerte (une fois), le paiement (une fois) et l'annulation motivée
-- (une fois, impossible après paiement) s'écrivent ; montant, échéance et destinataire ne bougent pas.
CREATE OR REPLACE FUNCTION ent_sortie_acompte_fournisseur_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'un acompte fournisseur ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.sortie_id <> OLD.sortie_id OR NEW.prestataire_id <> OLD.prestataire_id
       OR NEW.commande_id IS DISTINCT FROM OLD.commande_id OR NEW.libelle <> OLD.libelle
       OR NEW.montant_centimes <> OLD.montant_centimes OR NEW.echeance <> OLD.echeance
       OR NEW.cree_le <> OLD.cree_le OR NEW.cree_par_user_id <> OLD.cree_par_user_id
       OR (OLD.alerte_le IS NOT NULL AND NEW.alerte_le IS DISTINCT FROM OLD.alerte_le)
       OR (OLD.paye_le IS NOT NULL AND (NEW.paye_le IS DISTINCT FROM OLD.paye_le
                                        OR NEW.paye_centimes IS DISTINCT FROM OLD.paye_centimes
                                        OR NEW.paye_reference IS DISTINCT FROM OLD.paye_reference
                                        OR NEW.paye_par_user_id IS DISTINCT FROM OLD.paye_par_user_id))
       OR (OLD.annule_le IS NOT NULL AND (NEW.annule_le IS DISTINCT FROM OLD.annule_le
                                          OR NEW.annule_motif IS DISTINCT FROM OLD.annule_motif)) THEN
        RAISE EXCEPTION 'un acompte fournisseur ne change que par son alerte, son paiement ou son annulation'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_acompte_fournisseur_garde_tg ON ent_sortie_acompte_fournisseur;
CREATE TRIGGER ent_sortie_acompte_fournisseur_garde_tg BEFORE UPDATE OR DELETE ON ent_sortie_acompte_fournisseur
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_acompte_fournisseur_garde();
DROP TRIGGER IF EXISTS ent_sortie_acompte_fournisseur_vidage ON ent_sortie_acompte_fournisseur;
CREATE TRIGGER ent_sortie_acompte_fournisseur_vidage BEFORE TRUNCATE ON ent_sortie_acompte_fournisseur
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Ouverture nominative : seule la fermeture s'écrit (une fois) ; jamais supprimée.
CREATE OR REPLACE FUNCTION ent_sortie_nominatif_ouverture_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' OR OLD.ferme_le IS NOT NULL
       OR NEW.sortie_id <> OLD.sortie_id OR NEW.personne_id <> OLD.personne_id OR NEW.motif <> OLD.motif
       OR NEW.ouvert_par_user_id <> OLD.ouvert_par_user_id OR NEW.ouvert_le <> OLD.ouvert_le THEN
        RAISE EXCEPTION 'une ouverture nominative ne fait que se refermer' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_nominatif_ouverture_garde_tg ON ent_sortie_nominatif_ouverture;
CREATE TRIGGER ent_sortie_nominatif_ouverture_garde_tg BEFORE UPDATE OR DELETE ON ent_sortie_nominatif_ouverture
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_nominatif_ouverture_garde();
DROP TRIGGER IF EXISTS ent_sortie_nominatif_ouverture_vidage ON ent_sortie_nominatif_ouverture;
CREATE TRIGGER ent_sortie_nominatif_ouverture_vidage BEFORE TRUNCATE ON ent_sortie_nominatif_ouverture
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_bordereau',
    'ent_sortie_bordereau_ligne',
    'ent_sortie_bordereau_remise',
    'ent_sortie_acompte_fournisseur',
    'ent_sortie_argent_export',
    'ent_sortie_nominatif_ouverture',
    'ent_sortie_argent_acces'
]);
