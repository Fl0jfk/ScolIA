-- 643 — Socle 42, lot V0b : budget en centimes versionné, prix annoncé figé, rappels de
-- l'organisateur, documents émis (PDF). Réf. : 42_SORTIES_VOYAGES.md §12.1, §19.9, R-SV15 à R-SV17,
-- NX-16, NX-51, NX-53 à NX-57, NX-59.
--
-- * Montants en CENTIMES entiers (BIGINT), jamais de flottant (R-SV16).
-- * Une version validée est FIGÉE (R-SV17) : ni elle ni ses lignes ne se modifient ; seule la
--   bascule « validée » → « remplacée » est permise, quand la version n+1 est validée.
-- * Le prix annoncé aux familles est en AJOUT SEUL : chaque changement est un fait daté (NX-55).
-- * Aucun JSON (D-036) ; aucune identité d'élève dans ces tables.

CREATE TABLE IF NOT EXISTS ent_sortie_budget_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero > 0),
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'validee', 'remplacee')),
    -- Compteur d'écritures : une décision porte sur la version LUE (verrou optimiste).
    rev INTEGER NOT NULL DEFAULT 1 CHECK (rev > 0),
    nb_payants INTEGER NOT NULL DEFAULT 0 CHECK (nb_payants BETWEEN 0 AND 2000),
    -- Marge de sécurité en pourcentage entier (préréglages 0 / 5 / 7 / 10, NX-53).
    marge_pct INTEGER NOT NULL DEFAULT 0 CHECK (marge_pct BETWEEN 0 AND 30),
    marge_motif TEXT CHECK (marge_motif IS NULL OR length(marge_motif) <= 1000),
    motif_revision TEXT CHECK (motif_revision IS NULL OR length(motif_revision) <= 1000),
    -- Résultat du calcul, scellé à la validation ; NULL tant que la version est un brouillon.
    part_familles_centimes BIGINT CHECK (part_familles_centimes IS NULL OR part_familles_centimes >= 0),
    total_depenses_centimes BIGINT CHECK (total_depenses_centimes IS NULL OR total_depenses_centimes >= 0),
    total_recettes_centimes BIGINT CHECK (total_recettes_centimes IS NULL OR total_recettes_centimes >= 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    valide_le TIMESTAMPTZ,
    valide_par_user_id UUID,
    CONSTRAINT ent_sortie_budget_version_uq UNIQUE (sortie_id, numero),
    CONSTRAINT ent_sortie_budget_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_budget_version_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_budget_version_scellee_chk CHECK (
        etat = 'brouillon'
        OR (part_familles_centimes IS NOT NULL AND total_depenses_centimes IS NOT NULL
            AND total_recettes_centimes IS NOT NULL AND valide_le IS NOT NULL))
);
-- Un seul brouillon et une seule version validée à la fois par sortie.
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_budget_version_brouillon_uq
    ON ent_sortie_budget_version (sortie_id) WHERE etat = 'brouillon';
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_budget_version_validee_uq
    ON ent_sortie_budget_version (sortie_id) WHERE etat = 'validee';

CREATE TABLE IF NOT EXISTS ent_sortie_budget_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    sens TEXT NOT NULL CHECK (sens IN ('depense', 'recette')),
    poste TEXT NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 0 AND 100000000000),
    -- Source de la ligne (§12.1). En V0b seule la saisie motivée existe ; les devis et commandes
    -- alimenteront les lignes au lot V0c.
    source TEXT NOT NULL DEFAULT 'saisie_motivee'
        CHECK (source IN ('devis_signe', 'devis_recu', 'commande', 'saisie_motivee')),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 1000),
    -- Affectation d'une recette : « encadrement » = elle couvre l'encadrement et le surcoût handicap
    -- (hors part des familles, R-SV15) ; « general » = elle réduit la base de la part des familles.
    -- Un don affecté déclare toujours son affectation.
    affectation TEXT CHECK (affectation IS NULL OR affectation IN ('encadrement', 'general')),
    rang INTEGER NOT NULL DEFAULT 0,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_budget_ligne_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_sortie_budget_version (etablissement_id, id),
    CONSTRAINT ent_sortie_budget_ligne_poste_chk CHECK (
        (sens = 'depense' AND poste IN (
            'transport', 'hebergement', 'restauration', 'activites', 'assurance', 'divers',
            'encadrement', 'surcout_handicap'))
        OR (sens = 'recette' AND poste IN (
            'etablissement', 'apel', 'collectivite', 'subvention_europeenne', 'don_affecte',
            'ventes_eleves', 'aides_individuelles'))),
    CONSTRAINT ent_sortie_budget_ligne_motif_chk CHECK (
        source <> 'saisie_motivee' OR length(btrim(coalesce(motif, ''))) > 0),
    CONSTRAINT ent_sortie_budget_ligne_affectation_chk CHECK (
        (sens = 'recette' OR affectation IS NULL)
        AND (poste <> 'don_affecte' OR affectation IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_sortie_budget_ligne_version_idx
    ON ent_sortie_budget_ligne (etablissement_id, version_id, rang);

-- Figement (R-SV17) : une version qui n'est plus un brouillon, et ses lignes, ne bougent plus.
CREATE OR REPLACE FUNCTION ent_sortie_budget_fige() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_etat TEXT;
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_budget_version' THEN
        IF TG_OP = 'DELETE' THEN
            IF OLD.etat <> 'brouillon' THEN
                RAISE EXCEPTION 'une version de budget validée ne se supprime pas' USING ERRCODE = '42501';
            END IF;
            RETURN OLD;
        END IF;
        IF OLD.etat <> 'brouillon' THEN
            -- Seule bascule permise : validée -> remplacée, tout le reste inchangé.
            IF NOT (OLD.etat = 'validee' AND NEW.etat = 'remplacee'
                    AND NEW.numero = OLD.numero AND NEW.sortie_id = OLD.sortie_id
                    AND NEW.nb_payants = OLD.nb_payants AND NEW.marge_pct = OLD.marge_pct
                    AND NEW.part_familles_centimes IS NOT DISTINCT FROM OLD.part_familles_centimes
                    AND NEW.total_depenses_centimes IS NOT DISTINCT FROM OLD.total_depenses_centimes
                    AND NEW.total_recettes_centimes IS NOT DISTINCT FROM OLD.total_recettes_centimes) THEN
                RAISE EXCEPTION 'une version de budget validée est figée' USING ERRCODE = '42501';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    -- Lignes : la version doit être un brouillon.
    SELECT etat INTO v_etat FROM ent_sortie_budget_version
     WHERE etablissement_id = COALESCE(NEW.etablissement_id, OLD.etablissement_id)
       AND id = COALESCE(NEW.version_id, OLD.version_id);
    IF v_etat IS DISTINCT FROM 'brouillon' THEN
        RAISE EXCEPTION 'les lignes d''une version de budget validée sont figées' USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_budget_version_fige ON ent_sortie_budget_version;
CREATE TRIGGER ent_sortie_budget_version_fige
    BEFORE UPDATE OR DELETE ON ent_sortie_budget_version
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_budget_fige();
DROP TRIGGER IF EXISTS ent_sortie_budget_ligne_fige ON ent_sortie_budget_ligne;
CREATE TRIGGER ent_sortie_budget_ligne_fige
    BEFORE INSERT OR UPDATE OR DELETE ON ent_sortie_budget_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_budget_fige();

-- Prix annoncé aux familles (NX-55, R-SV17) : en AJOUT SEUL. Le prix courant est la ligne de rang le
-- plus élevé. Il est figé à l'ouverture ; une hausse exige une décision motivée de la direction.
CREATE TABLE IF NOT EXISTS ent_sortie_prix_fige (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    rang INTEGER NOT NULL CHECK (rang > 0),
    version_id UUID NOT NULL,
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes >= 0),
    cause TEXT NOT NULL CHECK (cause IN ('ouverture', 'baisse', 'hausse_decidee')),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 1000),
    fige_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    fige_par_user_id UUID,
    CONSTRAINT ent_sortie_prix_fige_uq UNIQUE (sortie_id, rang),
    CONSTRAINT ent_sortie_prix_fige_hausse_chk CHECK (
        cause <> 'hausse_decidee' OR length(btrim(coalesce(motif, ''))) > 0),
    CONSTRAINT ent_sortie_prix_fige_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_prix_fige_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_sortie_budget_version (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_sortie_prix_fige_immuable ON ent_sortie_prix_fige;
CREATE TRIGGER ent_sortie_prix_fige_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_prix_fige
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Rappels de l'organisateur (NX-16) : un rappel par sortie et par code, créé une seule fois
-- (idempotent). La notification part par l'outbox (`voyages.rappel_organisateur`, socle 8).
CREATE TABLE IF NOT EXISTS ent_sortie_rappel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN (
        'reunion_parents', 'dossier_bloque', 'effectif', 'liste_car', 'communication_parents')),
    -- Jour (Europe/Paris) où le rappel a été constaté ; J-n = jours avant le départ à ce moment.
    jour DATE NOT NULL,
    jours_avant INTEGER NOT NULL CHECK (jours_avant >= 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_rappel_uq UNIQUE (sortie_id, code),
    CONSTRAINT ent_sortie_rappel_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_sortie_rappel_immuable ON ent_sortie_rappel;
CREATE TRIGGER ent_sortie_rappel_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_rappel
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Documents émis par le serveur (PDF de la fiche budget, de la circulaire). Le fichier est dans le
-- magasin d'objets (clé opaque, empreinte au registre `ent_objet`) ; jamais d'URL publique (R-SV28).
-- En ajout seul : un nouveau document « remplace » l'ancien (le plus récent par type fait foi).
CREATE TABLE IF NOT EXISTS ent_sortie_document (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('budget', 'circulaire')),
    version_id UUID,
    cle_objet TEXT NOT NULL CHECK (length(cle_objet) BETWEEN 1 AND 300),
    nom TEXT NOT NULL CHECK (length(nom) BETWEEN 1 AND 200),
    taille INTEGER NOT NULL CHECK (taille > 0),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    emis_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    emis_par_user_id UUID,
    CONSTRAINT ent_sortie_document_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_document_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_sortie_budget_version (etablissement_id, id),
    CONSTRAINT ent_sortie_document_budget_chk CHECK (type <> 'budget' OR version_id IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_sortie_document_sortie_idx
    ON ent_sortie_document (etablissement_id, sortie_id, type, emis_le);
DROP TRIGGER IF EXISTS ent_sortie_document_immuable ON ent_sortie_document;
CREATE TRIGGER ent_sortie_document_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_document
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_budget_version',
    'ent_sortie_budget_ligne',
    'ent_sortie_prix_fige',
    'ent_sortie_rappel',
    'ent_sortie_document'
]);
