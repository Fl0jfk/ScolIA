-- 650 — Socle 42, lot V2a (1/2) : l'argent des sorties en MODE SUIVI (R-SV44, D-034). ScolIA décide la part
-- des familles, la répartit entre les payeurs, pointe les versements et propose des avoirs ; il n'émet AUCUNE
-- facture, AUCUN avis de prélèvement et AUCUN avoir comptable : la comptabilité reste dans Charlemagne.
-- Réf. : 42_SORTIES_VOYAGES.md §12.1, §12.5, §32.5, VOY-267, VOY-273, R-SV15, R-SV18, CV-71, CV-81.
--
-- Exigences communes : `etablissement_id` NOT NULL et clés étrangères COMPOSITES partout, UUID fournis par
-- l'application, aucun BYTEA, aucun JSON (D-036), montants en CENTIMES entiers avec CHECK de bornes, migration
-- rejouable (aucune modification des migrations 640 à 649), RLS posée en fin de fichier.
--
-- * Les tables d'argent sont en AJOUT SEUL (déclencheurs UPDATE / DELETE / TRUNCATE refusés) : une erreur de
--   pointage s'annule par une écriture INVERSE motivée, jamais par une modification.
-- * La part d'un payeur est un instantané daté (prix annoncé × répartition du socle 24/4) : si le prix baisse, une
--   nouvelle ligne est ajoutée, l'ancienne reste. L'échéancier n'est pas stocké : il se CALCULE depuis la part
--   courante et le dernier plan accordé, donc il ne peut pas être périmé.
-- * Aucun montant ni aucun nom ne part dans l'outbox (les événements ne portent que des identifiants et des codes).

-- Réglages de l'établissement (aucune ligne semée : le code applique les valeurs par défaut).
CREATE TABLE IF NOT EXISTS ent_sortie_argent_reglage (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Un organisateur peut-il collecter des chèques (VOY-266) ? Dans le PUBLIC, jamais (régisseur seul) : le
    -- régime vient du socle 24 ; ce réglage ne peut que restreindre le privé.
    collecte_organisateur BOOLEAN NOT NULL DEFAULT true,
    -- Chèques-vacances : seulement si l'établissement est agréé ANCV (§12.3) ; désactivé par défaut.
    cheques_vacances BOOLEAN NOT NULL DEFAULT false,
    -- Alerte à la comptabilité avant l'échéance d'un acompte fournisseur (VOY-270).
    alerte_acompte_jours INTEGER NOT NULL DEFAULT 14 CHECK (alerte_acompte_jours BETWEEN 1 AND 90),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_user_id UUID
);

-- Part d'un payeur pour un participant, à un prix annoncé donné.
CREATE TABLE IF NOT EXISTS ent_sortie_part (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    compte_id UUID NOT NULL,
    -- Rang du prix annoncé figé (`ent_sortie_prix_fige.rang`) dont la part découle (R-SV15, R-SV17).
    prix_rang INTEGER NOT NULL CHECK (prix_rang > 0),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 0 AND 100000000000),
    -- Part du payeur dans la répartition (points de base : 10 000 = 100 %).
    part_bps INTEGER NOT NULL CHECK (part_bps BETWEEN 1 AND 10000),
    jour_calcul DATE NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_part_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_part_uq UNIQUE (participant_id, compte_id, prix_rang),
    CONSTRAINT ent_sortie_part_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_part_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_part_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_part_sortie_idx
    ON ent_sortie_part (etablissement_id, sortie_id, participant_id, prix_rang);

-- Plan de paiement accordé pour un payeur : standard (1 à 3 fois, proposé spontanément) ou LONG (4 à 10 fois,
-- accordé par la comptabilité sur demande, avec motif). Le dernier plan (rang le plus haut) fait foi. L'organisateur
-- ne le voit jamais : il ne voit que « 1 plan en cours ».
CREATE TABLE IF NOT EXISTS ent_sortie_plan (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    compte_id UUID NOT NULL,
    rang INTEGER NOT NULL CHECK (rang > 0),
    nb_echeances INTEGER NOT NULL CHECK (nb_echeances BETWEEN 1 AND 10),
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    accorde_par_user_id UUID NOT NULL,
    accorde_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_plan_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_plan_uq UNIQUE (participant_id, compte_id, rang),
    CONSTRAINT ent_sortie_plan_long_chk CHECK (nb_echeances <= 3 OR motif IS NOT NULL),
    CONSTRAINT ent_sortie_plan_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_plan_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_plan_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id)
);

-- Versements pointés (VOY-267), sans facture. Une ligne « annulation » reprend le même montant et le même moyen
-- qu'un versement qu'elle annule, avec un motif : le journal n'est jamais modifié.
CREATE TABLE IF NOT EXISTS ent_sortie_versement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    compte_id UUID NOT NULL,
    sens TEXT NOT NULL CHECK (sens IN ('versement', 'annulation')),
    moyen TEXT NOT NULL CHECK (moyen IN ('cheque', 'especes', 'virement', 'cheques_vacances')),
    -- 1 centime à 100 000 € par écriture.
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 10000000),
    -- Jour de la valeur (jour de l'établissement) ; jamais dans le futur (contrôlé par le code et par la base).
    date_valeur DATE NOT NULL,
    reference TEXT CHECK (reference IS NULL OR length(btrim(reference)) BETWEEN 1 AND 40),
    annule_id UUID,
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    bordereau_ligne_id UUID,
    saisi_par_user_id UUID NOT NULL,
    saisi_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_versement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_versement_sens_chk CHECK (
        (sens = 'versement' AND annule_id IS NULL)
        OR (sens = 'annulation' AND annule_id IS NOT NULL AND motif IS NOT NULL)),
    CONSTRAINT ent_sortie_versement_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_versement_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_versement_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    CONSTRAINT ent_sortie_versement_annule_fk FOREIGN KEY (etablissement_id, annule_id)
        REFERENCES ent_sortie_versement (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_versement_part_idx
    ON ent_sortie_versement (etablissement_id, sortie_id, participant_id, compte_id);
-- Un versement ne s'annule qu'une fois.
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_versement_annulation_uq
    ON ent_sortie_versement (annule_id) WHERE annule_id IS NOT NULL;
-- Une ligne de bordereau ne produit qu'un versement (le lien est posé en 651).
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_versement_bordereau_uq
    ON ent_sortie_versement (bordereau_ligne_id) WHERE bordereau_ligne_id IS NOT NULL AND sens = 'versement';

-- Frais réels d'un désistement (VOY-273, CV-71) : billet nominatif, pénalité du prestataire… base de l'avoir proposé.
-- Une correction est une annulation motivée, jamais une modification.
CREATE TABLE IF NOT EXISTS ent_sortie_frais_reel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN (
        'billet_nominatif', 'penalite_prestataire', 'hebergement_non_remboursable', 'autre')),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 100000000000),
    annule_id UUID,
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    saisi_par_user_id UUID NOT NULL,
    saisi_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_frais_reel_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_frais_reel_annule_chk CHECK ((annule_id IS NULL) OR motif IS NOT NULL),
    CONSTRAINT ent_sortie_frais_reel_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_frais_reel_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_frais_reel_annule_fk FOREIGN KEY (etablissement_id, annule_id)
        REFERENCES ent_sortie_frais_reel (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_frais_reel_annulation_uq
    ON ent_sortie_frais_reel (annule_id) WHERE annule_id IS NOT NULL;

-- Avoirs PROPOSÉS (§12.5) : un montant, jamais une écriture comptable. Exportés vers la comptabilité, qui décide.
-- Le remboursement va toujours au payeur qui a payé (CF-69) : la ligne porte le compte payeur.
CREATE TABLE IF NOT EXISTS ent_sortie_avoir_propose (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    compte_id UUID NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN (
        'desistement_avant_engagement', 'desistement_apres_engagement', 'motif_legitime',
        'baisse_prix', 'annulation_collective', 'depart_en_preparation')),
    -- Clé d'idempotence : une même cause ne propose qu'un avoir par payeur.
    cle TEXT NOT NULL CHECK (length(cle) BETWEEN 1 AND 120),
    paye_net_centimes BIGINT NOT NULL CHECK (paye_net_centimes BETWEEN 0 AND 100000000000),
    frais_retenus_centimes BIGINT NOT NULL DEFAULT 0 CHECK (frais_retenus_centimes BETWEEN 0 AND 100000000000),
    montant_propose_centimes BIGINT NOT NULL CHECK (montant_propose_centimes BETWEEN 0 AND 100000000000),
    statut TEXT NOT NULL DEFAULT 'propose' CHECK (statut IN ('propose', 'retenu', 'ecarte')),
    montant_retenu_centimes BIGINT CHECK (montant_retenu_centimes IS NULL OR montant_retenu_centimes BETWEEN 0 AND 100000000000),
    decide_par_user_id UUID,
    decide_le TIMESTAMPTZ,
    decision_motif TEXT CHECK (decision_motif IS NULL OR length(btrim(decision_motif)) BETWEEN 1 AND 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_avoir_propose_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_avoir_propose_cle_uq UNIQUE (etablissement_id, cle),
    CONSTRAINT ent_sortie_avoir_propose_borne_chk CHECK (montant_propose_centimes <= paye_net_centimes),
    CONSTRAINT ent_sortie_avoir_propose_decision_chk CHECK (
        (statut = 'propose') = (decide_par_user_id IS NULL AND decide_le IS NULL AND montant_retenu_centimes IS NULL)),
    CONSTRAINT ent_sortie_avoir_propose_retenu_chk CHECK (
        statut <> 'retenu' OR (montant_retenu_centimes IS NOT NULL AND montant_retenu_centimes <= paye_net_centimes)),
    CONSTRAINT ent_sortie_avoir_propose_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_avoir_propose_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_avoir_propose_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_avoir_propose_sortie_idx
    ON ent_sortie_avoir_propose (etablissement_id, sortie_id, statut);

-- ───────────────────────── Garde-fous de la base ─────────────────────────

-- Ajout seul : plus aucune modification ni suppression (une erreur s'annule par une écriture inverse).
DROP TRIGGER IF EXISTS ent_sortie_part_immuable ON ent_sortie_part;
CREATE TRIGGER ent_sortie_part_immuable BEFORE UPDATE OR DELETE ON ent_sortie_part
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_plan_immuable ON ent_sortie_plan;
CREATE TRIGGER ent_sortie_plan_immuable BEFORE UPDATE OR DELETE ON ent_sortie_plan
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_versement_immuable ON ent_sortie_versement;
CREATE TRIGGER ent_sortie_versement_immuable BEFORE UPDATE OR DELETE ON ent_sortie_versement
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_frais_reel_immuable ON ent_sortie_frais_reel;
CREATE TRIGGER ent_sortie_frais_reel_immuable BEFORE UPDATE OR DELETE ON ent_sortie_frais_reel
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_part_vidage ON ent_sortie_part;
CREATE TRIGGER ent_sortie_part_vidage BEFORE TRUNCATE ON ent_sortie_part
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_plan_vidage ON ent_sortie_plan;
CREATE TRIGGER ent_sortie_plan_vidage BEFORE TRUNCATE ON ent_sortie_plan
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_versement_vidage ON ent_sortie_versement;
CREATE TRIGGER ent_sortie_versement_vidage BEFORE TRUNCATE ON ent_sortie_versement
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_frais_reel_vidage ON ent_sortie_frais_reel;
CREATE TRIGGER ent_sortie_frais_reel_vidage BEFORE TRUNCATE ON ent_sortie_frais_reel
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Une annulation reprend EXACTEMENT le versement qu'elle annule (même sortie, participant, payeur, moyen, montant)
-- et ne vise qu'un versement (jamais une autre annulation). Un versement n'a pas de date de valeur future.
CREATE OR REPLACE FUNCTION ent_sortie_versement_controle() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v ent_sortie_versement%ROWTYPE;
BEGIN
    IF NEW.date_valeur > ent_aujourdhui_etab(NEW.etablissement_id) THEN
        RAISE EXCEPTION 'la date de valeur d''un versement ne peut pas être dans le futur' USING ERRCODE = '23514';
    END IF;
    IF NEW.sens = 'annulation' THEN
        SELECT * INTO v FROM ent_sortie_versement
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.annule_id;
        IF NOT FOUND OR v.sens <> 'versement'
           OR v.sortie_id <> NEW.sortie_id OR v.participant_id <> NEW.participant_id
           OR v.compte_id <> NEW.compte_id OR v.moyen <> NEW.moyen
           OR v.montant_centimes <> NEW.montant_centimes THEN
            RAISE EXCEPTION 'une annulation reprend exactement le versement annulé' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_versement_controle_tg ON ent_sortie_versement;
CREATE TRIGGER ent_sortie_versement_controle_tg BEFORE INSERT ON ent_sortie_versement
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_versement_controle();

-- Une annulation de frais vise un frais d'origine du même participant, du même montant.
CREATE OR REPLACE FUNCTION ent_sortie_frais_reel_controle() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    f ent_sortie_frais_reel%ROWTYPE;
BEGIN
    IF NEW.annule_id IS NOT NULL THEN
        SELECT * INTO f FROM ent_sortie_frais_reel
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.annule_id;
        IF NOT FOUND OR f.annule_id IS NOT NULL OR f.sortie_id <> NEW.sortie_id
           OR f.participant_id <> NEW.participant_id OR f.montant_centimes <> NEW.montant_centimes THEN
            RAISE EXCEPTION 'une annulation de frais reprend exactement le frais annulé' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_frais_reel_controle_tg ON ent_sortie_frais_reel;
CREATE TRIGGER ent_sortie_frais_reel_controle_tg BEFORE INSERT ON ent_sortie_frais_reel
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_frais_reel_controle();

-- Avoir proposé : le montant et la cause ne bougent jamais ; seule la décision de la comptabilité (une fois) est écrite.
CREATE OR REPLACE FUNCTION ent_sortie_avoir_propose_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'un avoir proposé ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut <> 'propose' OR NEW.statut = 'propose'
       OR NEW.sortie_id <> OLD.sortie_id OR NEW.participant_id <> OLD.participant_id
       OR NEW.compte_id <> OLD.compte_id OR NEW.motif_code <> OLD.motif_code OR NEW.cle <> OLD.cle
       OR NEW.paye_net_centimes <> OLD.paye_net_centimes
       OR NEW.frais_retenus_centimes <> OLD.frais_retenus_centimes
       OR NEW.montant_propose_centimes <> OLD.montant_propose_centimes THEN
        RAISE EXCEPTION 'un avoir proposé ne change que par la décision de la comptabilité' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_avoir_propose_garde_tg ON ent_sortie_avoir_propose;
CREATE TRIGGER ent_sortie_avoir_propose_garde_tg BEFORE UPDATE OR DELETE ON ent_sortie_avoir_propose
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_avoir_propose_garde();
DROP TRIGGER IF EXISTS ent_sortie_avoir_propose_vidage ON ent_sortie_avoir_propose;
CREATE TRIGGER ent_sortie_avoir_propose_vidage BEFORE TRUNCATE ON ent_sortie_avoir_propose
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_argent_reglage',
    'ent_sortie_part',
    'ent_sortie_plan',
    'ent_sortie_versement',
    'ent_sortie_frais_reel',
    'ent_sortie_avoir_propose'
]);
