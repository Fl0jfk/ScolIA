-- 522 — Finances, lot F2 : comptes débiteurs et payeurs (socle 24, §9, MIG-4).
--
-- Réf. : docs/socles/24_FINANCES.md §9 (comptes, désignations datées, organismes, élève majeur),
-- §23 (modèle), R-F4, R-F5, R-F26, R-F30, R-F31, R-F33, R-F40, FAC-210, FAC-217, FAC-218, FAC-253 à
-- FAC-257, FD-15 ; PERSONAS_24_25.md (bloquants A et C).
--
-- * **Compte débiteur** : le « débiteur » d'une pièce n'est plus seulement un lieu de vie. Le compte
--   garde le `foyer_id` (compatibilité de toutes les tables `ent_fac_*`, Q-A1 du socle 4) et porte
--   l'identité du payeur : une personne, un organisme, ou un foyer historique à vérifier. Un compte
--   par foyer et par établissement (D-013, R-F5) ; la consolidation entre établissements reste
--   désactivée (R-F33 : `entite_facturation_id` posée, aucune consolidation).
-- * **Désignation de payeur datée** (`ent_fin_designation`) : adaptateur du port `payeurs` du socle 4
--   (`ent_eleve_payeur` n'existe pas encore) ; le jour où le socle 4 livre sa table, seule la lecture
--   du port change. Parts en points de base (10 000 = 100 %), jamais de flottant ; une période par
--   ligne, sans chevauchement pour un même (élève, catégorie, compte) ; une désignation exige
--   l'acceptation du payeur désigné ou une pièce (R-F30, R-F31).
-- * **Aucune pièce émise n'est réécrite** : la reprise (MIG-4) renseigne seulement `compte_id`, colonne
--   de rattachement non figée par les déclencheurs de la 520 (elle n'existait pas) ; voir plus bas.
-- * Aucun JSON métier (D-036), aucun `ON DELETE CASCADE` (RESTRICT partout), RLS ENABLE + FORCE.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- ── Entité de facturation (R-F33, R-F40) ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_entite_facturation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 160),
    -- Jeu de règles : aucune règle du privé n'est codée en dur (R-F40).
    regime TEXT NOT NULL DEFAULT 'prive_sous_contrat'
        CHECK (regime IN ('prive_sous_contrat', 'public_eple')),
    siren TEXT CHECK (siren IS NULL OR siren ~ '^[0-9]{9}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Une entité par établissement tant que la consolidation (Q13 de SOCLES.md) n'est pas décidée.
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_entite_etab_uq ON ent_fin_entite_facturation (etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_entite_etab_id_uq ON ent_fin_entite_facturation (etablissement_id, id);

-- ── Compte débiteur (§9.1) ──────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_compte (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entite_facturation_id UUID NOT NULL,
    type_debiteur TEXT NOT NULL CHECK (type_debiteur IN ('personne', 'organisme', 'foyer')),
    personne_id UUID,
    -- Réservé au socle 4 (`ent_organisme`, lot payeurs) : sans clé étrangère tant que la table n'existe pas.
    organisme_id UUID,
    foyer_id UUID NOT NULL,
    libelle_facturation TEXT NOT NULL CHECK (length(btrim(libelle_facturation)) BETWEEN 1 AND 200),
    siret TEXT CHECK (siret IS NULL OR siret ~ '^[0-9]{14}$'),
    reference_client TEXT CHECK (reference_client IS NULL OR length(reference_client) <= 80),
    organisme_public BOOLEAN NOT NULL DEFAULT false,
    chorus_pro BOOLEAN NOT NULL DEFAULT false,
    etat TEXT NOT NULL DEFAULT 'actif' CHECK (etat IN ('actif', 'archive')),
    a_verifier BOOLEAN NOT NULL DEFAULT false,
    a_verifier_motif TEXT CHECK (a_verifier_motif IS NULL OR length(a_verifier_motif) <= 200),
    origine TEXT NOT NULL
        CHECK (origine IN ('reprise', 'designation', 'organisme', 'majeur', 'manuel')),
    cree_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_compte_type_chk CHECK (
        (type_debiteur = 'personne' AND personne_id IS NOT NULL AND organisme_id IS NULL)
     OR (type_debiteur = 'organisme' AND personne_id IS NULL)
     OR (type_debiteur = 'foyer' AND personne_id IS NULL AND organisme_id IS NULL)),
    CONSTRAINT ent_fin_compte_a_verifier_chk CHECK (NOT a_verifier OR a_verifier_motif IS NOT NULL),
    CONSTRAINT ent_fin_compte_entite_fk FOREIGN KEY (etablissement_id, entite_facturation_id)
        REFERENCES ent_fin_entite_facturation (etablissement_id, id),
    CONSTRAINT ent_fin_compte_foyer_fk FOREIGN KEY (etablissement_id, foyer_id)
        REFERENCES ent_foyer (etablissement_id, id),
    CONSTRAINT ent_fin_compte_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_compte_etab_id_uq ON ent_fin_compte (etablissement_id, id);
-- Un compte par foyer (le foyer reste le débiteur des pièces) et par établissement (R-F5).
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_compte_foyer_uq ON ent_fin_compte (etablissement_id, foyer_id);
-- Une personne peut être payeur dans deux foyers (familles recomposées) : index de recherche, pas d'unicité.
CREATE INDEX IF NOT EXISTS ent_fin_compte_personne_idx
    ON ent_fin_compte (etablissement_id, personne_id) WHERE personne_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_compte_organisme_uq
    ON ent_fin_compte (etablissement_id, organisme_id) WHERE organisme_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_compte_siret_uq
    ON ent_fin_compte (etablissement_id, siret) WHERE siret IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_fin_compte_verifier_idx
    ON ent_fin_compte (etablissement_id) WHERE a_verifier;

-- Rattachement des pièces : `compte_id` (obligatoire pour toute nouvelle pièce, contrainte en fin de fichier).
ALTER TABLE ent_fac_facture ADD COLUMN IF NOT EXISTS compte_id UUID;
CREATE INDEX IF NOT EXISTS ent_fac_facture_compte_idx ON ent_fac_facture (etablissement_id, compte_id);

-- ── Désignation de payeur datée (adaptateur du port `payeurs`, socle 4 §8.9) ────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_designation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Les lignes d'une même proposition de répartition (somme des parts = 100 %) ; elles deviennent
    -- opposables ensemble, jamais l'une sans l'autre.
    groupe_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    categorie TEXT NOT NULL CHECK (categorie IN
        ('toutes', 'scolarite', 'restauration', 'internat', 'activite', 'frais', 'voyages', 'fournitures')),
    compte_id UUID NOT NULL,
    -- Part en points de base (10 000 = 100 %) ; la somme des parts d'un rang vaut 10 000 à toute date.
    part_bps INTEGER NOT NULL CHECK (part_bps BETWEEN 1 AND 10000),
    -- 1 = payeur principal ; 2 = payeur suivant, qui reçoit la part qu'un plafond d'organisme ne couvre pas.
    rang SMALLINT NOT NULL DEFAULT 1 CHECK (rang IN (1, 2)),
    du DATE NOT NULL,
    au DATE,
    source TEXT NOT NULL CHECK (source IN ('dossier', 'finances', 'reprise', 'majeur')),
    -- en_attente : proposée, pas encore opposable (R-F30) ; effective : lue par la facturation ;
    -- refusee / annulee : conservées pour l'historique.
    statut TEXT NOT NULL DEFAULT 'effective'
        CHECK (statut IN ('en_attente', 'effective', 'refusee', 'annulee')),
    acceptation TEXT NOT NULL DEFAULT 'non_requise'
        CHECK (acceptation IN ('non_requise', 'en_attente', 'acceptee', 'refusee')),
    acceptee_le TIMESTAMPTZ,
    acceptee_par UUID,
    piece_nature TEXT CHECK (piece_nature IS NULL
        OR piece_nature IN ('jugement', 'convention_homologuee', 'accord_des_deux')),
    piece_reference TEXT CHECK (piece_reference IS NULL OR length(piece_reference) <= 120),
    piece_document_id UUID,
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 300),
    decide_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_designation_periode_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_fin_designation_piece_chk CHECK (
        (piece_nature IS NULL) = (piece_reference IS NULL)),
    -- R-F30 : une désignation effective repose sur l'acceptation du payeur désigné, sur une pièce,
    -- ou sur une origine qui porte déjà son consentement (dossier du socle 4, reprise).
    CONSTRAINT ent_fin_designation_opposable_chk CHECK (
        statut <> 'effective'
        OR source IN ('dossier', 'reprise')
        OR acceptation IN ('acceptee', 'non_requise') AND (acceptation = 'acceptee' OR piece_nature IS NOT NULL)),
    CONSTRAINT ent_fin_designation_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_fin_designation_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    -- Jamais deux périodes opposables qui se chevauchent pour un même (élève, catégorie, compte).
    -- Les propositions en attente ne comptent pas : elles ferment l'ancienne répartition à leur activation.
    CONSTRAINT ent_fin_designation_pas_de_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, eleve_id WITH =, categorie WITH =, compte_id WITH =,
        daterange(du, au, '[]') WITH &&
    ) WHERE (statut = 'effective')
);
CREATE INDEX IF NOT EXISTS ent_fin_designation_groupe_idx
    ON ent_fin_designation (etablissement_id, groupe_id);
CREATE INDEX IF NOT EXISTS ent_fin_designation_eleve_idx
    ON ent_fin_designation (etablissement_id, eleve_id, categorie, du);
CREATE INDEX IF NOT EXISTS ent_fin_designation_compte_idx
    ON ent_fin_designation (etablissement_id, compte_id);

-- ── Prise en charge d'un organisme (FAC-255 à FAC-257, CF-82, CF-83) ────────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_prise_en_charge (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    compte_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    reference TEXT NOT NULL CHECK (length(btrim(reference)) BETWEEN 1 AND 80),
    du DATE NOT NULL,
    au DATE NOT NULL,
    categories TEXT[] NOT NULL CHECK (cardinality(categories) BETWEEN 1 AND 8),
    -- Plafond en montant (centimes entiers) ; NULL = sans plafond. Jamais de flottant.
    plafond_centimes BIGINT CHECK (plafond_centimes IS NULL OR (plafond_centimes > 0 AND plafond_centimes <= 1000000000)),
    statut TEXT NOT NULL DEFAULT 'active' CHECK (statut IN ('active', 'echue', 'annulee')),
    cree_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_pec_periode_chk CHECK (au >= du),
    CONSTRAINT ent_fin_pec_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    CONSTRAINT ent_fin_pec_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_fin_pec_compte_idx ON ent_fin_prise_en_charge (etablissement_id, compte_id, eleve_id);

-- ── Tâches du secrétariat et de la comptabilité (FAC-210, FAC-218, FAC-257) ─────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_tache (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature IN (
        'payeur_manquant', 'repartition_incomplete', 'demande_repartition', 'designation_a_accepter',
        'prise_en_charge_a_renouveler', 'part_non_couverte', 'compte_a_verifier', 'ligne_sans_periode',
        'guichet_quota')),
    eleve_id UUID,
    compte_id UUID,
    categorie TEXT,
    designation_id UUID,
    prise_en_charge_id UUID,
    -- « Léo : restauration sans payeur depuis le 01/02 » : la date depuis laquelle la situation dure.
    depuis DATE NOT NULL,
    detail TEXT CHECK (detail IS NULL OR length(detail) <= 300),
    statut TEXT NOT NULL DEFAULT 'ouverte' CHECK (statut IN ('ouverte', 'resolue', 'abandonnee')),
    cree_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolue_le TIMESTAMPTZ,
    resolue_par UUID,
    CONSTRAINT ent_fin_tache_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_fin_tache_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id)
);
-- Idempotence : une seule tâche ouverte par situation (rejeu d'un événement, génération répétée).
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_tache_ouverte_uq ON ent_fin_tache (
    etablissement_id, nature,
    COALESCE(eleve_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(compte_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(designation_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(prise_en_charge_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(categorie, '')
) WHERE statut = 'ouverte';
CREATE INDEX IF NOT EXISTS ent_fin_tache_statut_idx ON ent_fin_tache (etablissement_id, statut, created_at DESC);

-- ── Conséquences d'un changement de payeur (§9.2, FAC-176, FAC-208) ─────────────────────────────
-- Deux sens, une ligne par ligne de pièce émise touchée : `credit` = avoir proposé au payeur sortant
-- (arrondi en faveur de la famille : au centime supérieur) ; `debit` = ce qui revient au payeur entrant
-- (arrondi à l'inférieur). Rien n'est émis : la comptabilité valide, puis émet (N3).
CREATE TABLE IF NOT EXISTS ent_fin_consequence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    designation_groupe_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    categorie TEXT NOT NULL,
    date_effet DATE NOT NULL,
    sens TEXT NOT NULL CHECK (sens IN ('credit', 'debit')),
    compte_id UUID NOT NULL,
    ligne_id UUID NOT NULL,
    facture_id UUID NOT NULL,
    -- Mois (comme le reste de l'engine : mois commencé dû à celui qui l'a commencé) : mois de la ligne,
    -- et mois qui changent de payeur.
    mois_total INTEGER NOT NULL CHECK (mois_total > 0),
    mois_apres INTEGER NOT NULL CHECK (mois_apres >= 0),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes >= 0),
    statut TEXT NOT NULL DEFAULT 'proposee' CHECK (statut IN ('proposee', 'appliquee', 'refusee')),
    decide_par UUID,
    decide_le TIMESTAMPTZ,
    motif_refus TEXT CHECK (motif_refus IS NULL OR length(motif_refus) <= 300),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_consequence_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_fin_consequence_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_consequence_uq ON ent_fin_consequence
    (etablissement_id, designation_groupe_id, ligne_id, sens, compte_id);
CREATE INDEX IF NOT EXISTS ent_fin_consequence_eleve_idx
    ON ent_fin_consequence (etablissement_id, eleve_id, statut);

-- ── Rapport de reprise (MIG-4) ──────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_reprise_rapport (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    comptes_crees INTEGER NOT NULL,
    pieces_rattachees INTEGER NOT NULL,
    foyers_a_verifier INTEGER NOT NULL,
    designations_creees INTEGER NOT NULL,
    lance_par UUID
);
CREATE INDEX IF NOT EXISTS ent_fin_reprise_rapport_idx ON ent_fin_reprise_rapport (etablissement_id, le DESC);

-- ── Rattachement d'une pièce à son compte : seule colonne autorisée à changer après l'émission ─
-- (une seule fois, de NULL vers un compte : reprise MIG-4). La fonction remplace celle de la 520 ;
-- tout le reste de la pièce émise demeure immuable.
CREATE OR REPLACE FUNCTION ent_fac_facture_protege() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.statut <> 'brouillon' THEN
            RAISE EXCEPTION 'ent_fac_facture %: une pièce émise ne se supprime pas', OLD.id
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN OLD;
    END IF;
    IF OLD.statut = 'brouillon' THEN
        RETURN NEW;
    END IF;
    IF NEW.statut = 'brouillon' THEN
        RAISE EXCEPTION 'ent_fac_facture %: retour au brouillon interdit', OLD.id
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF OLD.compte_id IS NOT NULL AND NEW.compte_id IS DISTINCT FROM OLD.compte_id THEN
        RAISE EXCEPTION 'ent_fac_facture %: le compte d''une pièce émise ne change pas', OLD.id
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF (to_jsonb(NEW) - 'statut' - 'soldee_at' - 'compte_id')
       IS DISTINCT FROM (to_jsonb(OLD) - 'statut' - 'soldee_at' - 'compte_id') THEN
        RAISE EXCEPTION 'ent_fac_facture %: une pièce émise est immuable (montants, numéro, débiteur, dates)', OLD.id
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END
$$;

-- ── Entité par défaut et compte d'un foyer (une seule implémentation : reprise, service, déclencheur) ─
CREATE OR REPLACE FUNCTION ent_fin_entite_par_defaut(p_etab UUID) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE
    v_entite UUID;
BEGIN
    SELECT id INTO v_entite FROM ent_fin_entite_facturation WHERE etablissement_id = p_etab;
    IF v_entite IS NULL THEN
        v_entite := gen_random_uuid();
        INSERT INTO ent_fin_entite_facturation (id, etablissement_id, libelle)
        VALUES (v_entite, p_etab, COALESCE(
            (SELECT left(NULLIF(btrim(e.nom), ''), 160) FROM ent_etablissement e WHERE e.id = p_etab),
            'Établissement'));
    END IF;
    RETURN v_entite;
END
$$;

-- Compte du foyer (le crée au besoin). Payeur identifié = exactement une personne `est_payeur` dans le
-- foyer ; sinon compte « foyer », SIGNALÉ à vérifier (aucun payeur, ou plusieurs : CF-09), jamais éclaté.
CREATE OR REPLACE FUNCTION ent_fin_compte_du_foyer(
    p_etab UUID, p_foyer UUID, p_par UUID DEFAULT NULL, p_origine TEXT DEFAULT 'reprise')
RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE
    v_compte UUID;
    v_personnes UUID[];
    v_nb INTEGER;
BEGIN
    SELECT id INTO v_compte FROM ent_fin_compte
     WHERE etablissement_id = p_etab AND foyer_id = p_foyer;
    IF v_compte IS NOT NULL THEN
        RETURN v_compte;
    END IF;
    SELECT array_agg(DISTINCT l.personne_id) INTO v_personnes
      FROM ent_eleve_lien l
     WHERE l.etablissement_id = p_etab AND l.foyer_id = p_foyer
       AND l.est_payeur AND l.personne_id IS NOT NULL;
    v_nb := COALESCE(cardinality(v_personnes), 0);
    v_compte := gen_random_uuid();
    INSERT INTO ent_fin_compte (
        id, etablissement_id, entite_facturation_id, type_debiteur, personne_id, foyer_id,
        libelle_facturation, etat, a_verifier, a_verifier_motif, origine, cree_par)
    SELECT v_compte, p_etab, ent_fin_entite_par_defaut(p_etab),
           CASE WHEN v_nb = 1 THEN 'personne' ELSE 'foyer' END,
           CASE WHEN v_nb = 1 THEN v_personnes[1] END,
           p_foyer,
           left(COALESCE(
                CASE WHEN v_nb = 1
                     THEN (SELECT btrim(COALESCE(p.prenom || ' ', '') || p.nom)
                             FROM ent_personne p WHERE p.etablissement_id = p_etab AND p.id = v_personnes[1])
                END,
                NULLIF(btrim(fo.libelle), ''), 'Foyer'), 200),
           'actif',
           v_nb <> 1,
           CASE WHEN v_nb = 0 THEN 'aucun_payeur_identifie'
                WHEN v_nb > 1 THEN 'plusieurs_payeurs_dans_le_foyer' END,
           p_origine, p_par
      FROM ent_foyer fo WHERE fo.etablissement_id = p_etab AND fo.id = p_foyer;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'ent_fin_compte_du_foyer : foyer % inconnu de l''établissement', p_foyer
            USING ERRCODE = 'foreign_key_violation';
    END IF;
    RETURN v_compte;
END
$$;

-- Toute pièce porte son compte (R-F5) : posé à l'insertion par la base, quel que soit le chemin
-- (service, adaptateur historique, reprise) ; un compte fourni doit être celui du foyer de la pièce.
CREATE OR REPLACE FUNCTION ent_fac_facture_pose_compte() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_foyer UUID;
BEGIN
    IF NEW.compte_id IS NULL THEN
        NEW.compte_id := ent_fin_compte_du_foyer(NEW.etablissement_id, NEW.foyer_id, NEW.created_by_user_id, 'reprise');
    ELSE
        SELECT foyer_id INTO v_foyer FROM ent_fin_compte
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.compte_id;
        IF v_foyer IS DISTINCT FROM NEW.foyer_id THEN
            RAISE EXCEPTION 'ent_fac_facture : le compte % n''est pas celui du foyer de la pièce', NEW.compte_id
                USING ERRCODE = 'check_violation';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_fac_facture_pose_compte_trg ON ent_fac_facture;
CREATE TRIGGER ent_fac_facture_pose_compte_trg
    BEFORE INSERT ON ent_fac_facture
    FOR EACH ROW EXECUTE FUNCTION ent_fac_facture_pose_compte();

-- ── MIG-4 : reprise des comptes depuis les foyers qui ont des pièces ────────────────────────────
-- Idempotente (rejouable à tout moment) : chaque foyer porteur de pièces reçoit UN compte ; chaque
-- pièce sans `compte_id` y est rattachée ; les désignations « toutes catégories » des liens payeurs
-- actuels sont reprises à la date de création du lien. Aucun montant, aucun numéro, aucune date de
-- pièce n'est modifié (seule la colonne de rattachement `compte_id`, créée ci-dessus).
CREATE OR REPLACE FUNCTION ent_fin_reprendre_comptes(p_etab UUID, p_par UUID DEFAULT NULL)
RETURNS TABLE (comptes_crees INTEGER, pieces_rattachees INTEGER, foyers_a_verifier INTEGER,
               designations_creees INTEGER)
LANGUAGE plpgsql AS $$
DECLARE
    v_avant INTEGER;
    v_apres INTEGER;
    v_pieces INTEGER := 0;
    v_verifier INTEGER;
    v_desig INTEGER := 0;
    r RECORD;
BEGIN
    SELECT count(*) INTO v_avant FROM ent_fin_compte WHERE etablissement_id = p_etab;
    FOR r IN
        SELECT DISTINCT f.foyer_id FROM ent_fac_facture f
         WHERE f.etablissement_id = p_etab AND f.compte_id IS NULL
    LOOP
        PERFORM ent_fin_compte_du_foyer(p_etab, r.foyer_id, p_par, 'reprise');
    END LOOP;
    -- Comptes aussi pour les foyers payeurs d'élèves (désignations reprises ci-dessous).
    FOR r IN
        SELECT DISTINCT l.foyer_id FROM ent_eleve_lien l
         WHERE l.etablissement_id = p_etab AND l.est_payeur AND l.foyer_id IS NOT NULL
           AND l.suspendu_le IS NULL
    LOOP
        PERFORM ent_fin_compte_du_foyer(p_etab, r.foyer_id, p_par, 'reprise');
    END LOOP;
    SELECT count(*) INTO v_apres FROM ent_fin_compte WHERE etablissement_id = p_etab;

    UPDATE ent_fac_facture f SET compte_id = c.id
      FROM ent_fin_compte c
     WHERE f.etablissement_id = p_etab AND f.compte_id IS NULL
       AND c.etablissement_id = p_etab AND c.foyer_id = f.foyer_id;
    GET DIAGNOSTICS v_pieces = ROW_COUNT;

    WITH liens AS (
        SELECT l.eleve_id, c.id AS compte_id,
               COALESCE(SUM(NULLIF(l.part_paiement, 0)), 0)::int AS part_pct,
               min(l.created_at)::date AS depuis
          FROM ent_eleve_lien l
          JOIN ent_fin_compte c ON c.etablissement_id = l.etablissement_id AND c.foyer_id = l.foyer_id
         WHERE l.etablissement_id = p_etab AND l.est_payeur AND l.foyer_id IS NOT NULL
           AND l.suspendu_le IS NULL
           AND NOT EXISTS (SELECT 1 FROM ent_fin_designation d
                            WHERE d.etablissement_id = p_etab AND d.eleve_id = l.eleve_id)
         GROUP BY l.eleve_id, c.id
    ), totaux AS (
        SELECT eleve_id, SUM(part_pct) AS total, count(*) AS nb_comptes, gen_random_uuid() AS groupe_id
          FROM liens GROUP BY eleve_id
    ), ins AS (
        INSERT INTO ent_fin_designation (
            id, etablissement_id, groupe_id, eleve_id, categorie, compte_id, part_bps, rang, du, source,
            statut, acceptation, motif)
        SELECT gen_random_uuid(), p_etab, t.groupe_id, li.eleve_id, 'toutes', li.compte_id,
               CASE WHEN t.total = 0 THEN (10000 / t.nb_comptes)
                    ELSE (li.part_pct * 10000 / t.total) END,
               1, li.depuis, 'reprise', 'effective', 'non_requise',
               'reprise des liens payeurs existants'
          FROM liens li JOIN totaux t ON t.eleve_id = li.eleve_id
         WHERE t.total = 100 OR t.total = 0
        RETURNING 1
    )
    SELECT count(*)::int INTO v_desig FROM ins;

    SELECT count(*) INTO v_verifier FROM ent_fin_compte
     WHERE etablissement_id = p_etab AND a_verifier AND origine = 'reprise';
    IF v_apres > v_avant OR v_pieces > 0 OR v_desig > 0 THEN
        INSERT INTO ent_fin_reprise_rapport (
            id, etablissement_id, comptes_crees, pieces_rattachees, foyers_a_verifier,
            designations_creees, lance_par)
        VALUES (gen_random_uuid(), p_etab, v_apres - v_avant, v_pieces, v_verifier, v_desig, p_par);
    END IF;
    RETURN QUERY SELECT v_apres - v_avant, v_pieces, v_verifier, v_desig;
END
$$;

-- Reprise des données existantes, établissement par établissement (aucune base ScolIA n'est déployée,
-- mais les bases de labo et de recette portent des pièces).
DO $$
DECLARE
    e UUID;
BEGIN
    FOR e IN SELECT DISTINCT etablissement_id FROM ent_fac_facture LOOP
        PERFORM ent_fin_reprendre_comptes(e, NULL);
    END LOOP;
END
$$;

ALTER TABLE ent_fac_facture DROP CONSTRAINT IF EXISTS ent_fac_facture_compte_fk;
ALTER TABLE ent_fac_facture ADD CONSTRAINT ent_fac_facture_compte_fk
    FOREIGN KEY (etablissement_id, compte_id) REFERENCES ent_fin_compte (etablissement_id, id);
ALTER TABLE ent_fac_facture DROP CONSTRAINT IF EXISTS ent_fac_facture_compte_requis_chk;
ALTER TABLE ent_fac_facture ADD CONSTRAINT ent_fac_facture_compte_requis_chk
    CHECK (compte_id IS NOT NULL) NOT VALID;

-- ── Déclencheurs : l'historique des comptes et des désignations ne s'efface pas ─────────────────
CREATE OR REPLACE FUNCTION ent_trg_fin_f2_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
        RETURN NEW;
    END IF;
    RAISE EXCEPTION '% : ni suppression ni vidage (historique financier)', TG_TABLE_NAME
        USING ERRCODE = 'restrict_violation';
END
$$;
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_fin_compte', 'ent_fin_designation', 'ent_fin_prise_en_charge',
                             'ent_fin_consequence', 'ent_fin_tache', 'ent_fin_reprise_rapport']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_suppression', t);
        EXECUTE format(
            'CREATE TRIGGER %I BEFORE DELETE ON %I FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_f2_sans_suppression()',
            t || '_sans_suppression', t);
    END LOOP;
END
$$;

-- ── RLS d'établissement (même politique que les migrations 100, 206 et 521) ─────────────────────
DO $$
DECLARE
    t TEXT;
    tables TEXT[] := ARRAY['ent_fin_entite_facturation', 'ent_fin_compte', 'ent_fin_designation',
                           'ent_fin_prise_en_charge', 'ent_fin_tache', 'ent_fin_consequence',
                           'ent_fin_reprise_rapport'];
BEGIN
    FOREACH t IN ARRAY tables LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
            'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
            'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
            t);
    END LOOP;
END
$$;

-- Revue de sécurité (E3) : quota quotidien d'actions au guichet par agent, paramétrable par l'établissement
-- (30 par défaut). Au-delà : refus, alerte à la direction (tâche `guichet_quota`), journal.
ALTER TABLE ent_fin_parametres
    ADD COLUMN IF NOT EXISTS guichet_quota_jour INTEGER NOT NULL DEFAULT 30
        CHECK (guichet_quota_jour BETWEEN 1 AND 1000);
