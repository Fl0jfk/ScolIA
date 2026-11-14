-- Socle N1 (« Demandes et flux de travail »), lot N1-3 — migration 3046 : HSE (heures supplémentaires effectives).
-- Réf. : docs/socles/N1_DEMANDES.md §17, §21.5, §22.9, §36 (R-DM50), §38 et D-061 (HSE comptées EN HEURES, jamais en euros ;
-- circuit demande du professeur → validation de la direction, rien d'autre ; attestation du service fait = réglage
-- facultatif, désactivé par défaut) ; NJ-91 (adresse codée en dur, signature-image, acceptation sans contrôle,
-- déposant sans trace).
--
-- Contenu : ent_dem_hse_reglage, ent_dem_hse (le volet d'une demande `rh.hse`), ent_dem_hse_public, ent_dem_hse_decision
-- (historique des décisions et avenants, ajout seul), ent_dem_hse_seance, ent_dem_hse_enveloppe et _mvt (en quarts
-- d'heure, mouvements en ajout seul), ent_dem_hse_export (lots d'export de secours).
-- Aucun montant : ni colonne d'euros ni de taux (R-DM44). Aucune adresse électronique (NJ-91). Aucune donnée
-- semi-structurée, aucun tableau, aucun contenu binaire (D-036). Additive et rejouable (E8). Clés étrangères composites
-- (etablissement_id, …) et ON DELETE RESTRICT (NJ-89).

-- Le journal des réglages du guichet accueille les réglages HSE et reprographie (tout changement est tracé).
ALTER TABLE ent_dem_reglage_journal DROP CONSTRAINT IF EXISTS ent_dem_reglage_journal_sujet_check;
ALTER TABLE ent_dem_reglage_journal ADD CONSTRAINT ent_dem_reglage_journal_sujet_check CHECK (sujet IN (
    'repli_rh', 'nature', 'file', 'membres', 'routage', 'reponse_type', 'palier', 'hse', 'reprographie'));

CREATE UNIQUE INDEX IF NOT EXISTS ent_agent_etab_id_uq ON ent_agent (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_classe_etab_id_uq ON ent_classe (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_groupe_etab_id_uq ON ent_groupe (etablissement_id, id);

-- Garde commune aux tables de N1-3 qui ne se suppriment jamais (la purge ne touche pas aux heures).
CREATE OR REPLACE FUNCTION ent_dem_n13_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : suppression interdite (NJ-89) ; rien ne s''efface, on corrige par un nouveau fait', TG_TABLE_NAME
        USING ERRCODE = '23001';
END
$$;

CREATE OR REPLACE FUNCTION ent_dem_n13_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : ajout seul, ni modification ni suppression (R-DM28)', TG_TABLE_NAME
        USING ERRCODE = '23001';
END
$$;

-- ───────────────────────── Réglage de l'établissement (D-061) ─────────────────────────
-- `attestation_service_fait` : désactivée par défaut. Désactivée, la décision de la direction vaut confiance et les
-- heures sont transmises à la validation. Activée, les heures ne sont transmises qu'à l'attestation (R-DM50).
CREATE TABLE IF NOT EXISTS ent_dem_hse_reglage (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    attestation_service_fait BOOLEAN NOT NULL DEFAULT false,
    -- Part des attestations sur déclaration sur l'honneur contrôlées par échantillon (SN1-15, défaut 10 %).
    echantillon_pct SMALLINT NOT NULL DEFAULT 10 CHECK (echantillon_pct BETWEEN 0 AND 100),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par_user_id UUID
);

-- ───────────────────────── Le volet HSE d'une demande `rh.hse` ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_hse (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    annee_id UUID NOT NULL,
    site_id UUID,
    -- Le bénéficiaire est un AGENT (socle 6) : il est lu de la fiche, jamais saisi comme texte (NJ-91).
    beneficiaire_agent_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (motif IN (
        'soutien', 'atelier', 'accompagnement_sortie', 'stage_reussite', 'preparation_concours', 'tutorat', 'autre')),
    -- Type d'heure de l'export (l'application académique en déduit le tarif ; ScolIA ne calcule aucun montant).
    type_heure TEXT NOT NULL CHECK (type_heure IN ('hse_enseignement', 'hse_accompagnement', 'hse_autre')),
    volume_demande_quarts INT NOT NULL CHECK (volume_demande_quarts BETWEEN 1 AND 1600),
    volume_accorde_quarts INT CHECK (volume_accorde_quarts IS NULL OR volume_accorde_quarts BETWEEN 0 AND 1600),
    periode_du DATE NOT NULL,
    periode_au DATE NOT NULL,
    lieu_libelle TEXT CHECK (lieu_libelle IS NULL OR length(lieu_libelle) <= 120),
    -- R-DM43 : une demande déposée au nom d'un collègue est confirmée (ou retirée) par lui avant décision.
    confirmation_requise BOOLEAN NOT NULL DEFAULT false,
    confirmee_le TIMESTAMPTZ,
    decision TEXT CHECK (decision IS NULL OR decision IN ('accordee', 'accordee_partiellement', 'refusee')),
    decide_par_personne_id UUID,
    decide_le TIMESTAMPTZ,
    avenants SMALLINT NOT NULL DEFAULT 0 CHECK (avenants >= 0),
    -- Un avenant en attente : volume et période demandés, nouvelle décision à rendre (§17.4 point 8).
    avenant_volume_quarts INT CHECK (avenant_volume_quarts IS NULL OR avenant_volume_quarts BETWEEN 1 AND 1600),
    avenant_periode_au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, demande_id),
    CONSTRAINT ent_dem_hse_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_annee_cfk FOREIGN KEY (etablissement_id, annee_id)
        REFERENCES ent_annee_scolaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_agent_cfk FOREIGN KEY (etablissement_id, beneficiaire_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_decideur_cfk FOREIGN KEY (etablissement_id, decide_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_periode_chk CHECK (periode_au >= periode_du),
    CONSTRAINT ent_dem_hse_decision_chk CHECK (
        (decision IS NULL) = (decide_le IS NULL) AND (decision IS NULL) = (decide_par_personne_id IS NULL)),
    -- Accordé ≤ demandé (un avenant relève le plafond par une nouvelle décision, jamais par un UPDATE du demandeur).
    CONSTRAINT ent_dem_hse_accorde_chk CHECK (
        volume_accorde_quarts IS NULL OR decision IS NOT NULL),
    CONSTRAINT ent_dem_hse_confirmation_chk CHECK (confirmee_le IS NULL OR confirmation_requise)
);
CREATE INDEX IF NOT EXISTS ent_dem_hse_agent_idx ON ent_dem_hse (etablissement_id, beneficiaire_agent_id, annee_id);
CREATE INDEX IF NOT EXISTS ent_dem_hse_site_idx ON ent_dem_hse (etablissement_id, annee_id, site_id);

CREATE OR REPLACE FUNCTION ent_dem_hse_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_hse : suppression interdite (NJ-89)' USING ERRCODE = '23001';
    END IF;
    IF NEW.etablissement_id <> OLD.etablissement_id OR NEW.demande_id <> OLD.demande_id
       OR NEW.beneficiaire_agent_id <> OLD.beneficiaire_agent_id OR NEW.annee_id <> OLD.annee_id
       OR NEW.motif <> OLD.motif OR NEW.type_heure <> OLD.type_heure OR NEW.cree_le <> OLD.cree_le
       OR NEW.volume_demande_quarts <> OLD.volume_demande_quarts THEN
        RAISE EXCEPTION 'ent_dem_hse : bénéficiaire, année et motif figés (NJ-91)' USING ERRCODE = '23001';
    END IF;
    -- NJ-91 : le déposant ne peut pas se faire accorder ses propres heures par un UPDATE : la décision exige un
    -- décideur différent du bénéficiaire (R-DM40) — contrôle aussi tenu par la base.
    IF NEW.decision IS DISTINCT FROM OLD.decision AND NEW.decide_par_personne_id IS NOT NULL THEN
        IF EXISTS (SELECT 1 FROM ent_agent a
                    WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.beneficiaire_agent_id
                      AND a.personne_id = NEW.decide_par_personne_id) THEN
            RAISE EXCEPTION 'ent_dem_hse : personne ne décide sa propre demande (R-DM40)' USING ERRCODE = '23001';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_dem_hse_garde_trg ON ent_dem_hse;
CREATE TRIGGER ent_dem_hse_garde_trg BEFORE UPDATE OR DELETE ON ent_dem_hse
    FOR EACH ROW EXECUTE FUNCTION ent_dem_hse_garde();

-- Public visé : une classe ou un groupe par ligne (jamais un élève nommé : minimisation).
CREATE TABLE IF NOT EXISTS ent_dem_hse_public (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    classe_id UUID,
    groupe_id UUID,
    CONSTRAINT ent_dem_hse_public_hse_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_hse (etablissement_id, demande_id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_public_classe_cfk FOREIGN KEY (etablissement_id, classe_id)
        REFERENCES ent_classe (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_public_groupe_cfk FOREIGN KEY (etablissement_id, groupe_id)
        REFERENCES ent_groupe (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_public_un_chk CHECK (num_nonnulls(classe_id, groupe_id) = 1)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_hse_public_classe_uq
    ON ent_dem_hse_public (etablissement_id, demande_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_hse_public_groupe_uq
    ON ent_dem_hse_public (etablissement_id, demande_id, groupe_id) WHERE groupe_id IS NOT NULL;

-- ───────────────────────── Décisions et avenants (ajout seul) ─────────────────────────
-- Chaque décision est conservée : le décideur, le jour, le volume, le motif. L'accord précédent n'est jamais réécrit.
CREATE TABLE IF NOT EXISTS ent_dem_hse_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang >= 0),
    decision TEXT NOT NULL CHECK (decision IN ('accordee', 'accordee_partiellement', 'refusee')),
    volume_quarts INT NOT NULL CHECK (volume_quarts BETWEEN 0 AND 1600),
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code ~ '^[a-z0-9_]{1,60}$'),
    avenant BOOLEAN NOT NULL DEFAULT false,
    -- Dépassement de l'enveloppe : avertissement, jamais un blocage (R-04 du 15), mais confirmé et motivé (conf. 2).
    depassement_enveloppe BOOLEAN NOT NULL DEFAULT false,
    decide_par_personne_id UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_hse_decision_rang_uq UNIQUE (etablissement_id, demande_id, rang),
    CONSTRAINT ent_dem_hse_decision_hse_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_hse (etablissement_id, demande_id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_decision_par_cfk FOREIGN KEY (etablissement_id, decide_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_decision_depassement_chk CHECK (NOT depassement_enveloppe OR motif_code IS NOT NULL)
);
DROP TRIGGER IF EXISTS ent_dem_hse_decision_ajout_seul_trg ON ent_dem_hse_decision;
CREATE TRIGGER ent_dem_hse_decision_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_dem_hse_decision
    FOR EACH ROW EXECUTE FUNCTION ent_dem_n13_ajout_seul();

-- ───────────────────────── Lots d'export de secours (DEM-282) ─────────────────────────
-- Tant que le socle 15 (heures constatées) n'est pas versé, N1 garde les heures et les exporte chaque mois, en heures
-- par type d'heure, pour l'application académique. Un lot trace qui a exporté quoi et quand.
CREATE TABLE IF NOT EXISTS ent_dem_hse_export (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    mois DATE NOT NULL CHECK (EXTRACT(day FROM mois) = 1),
    nb_seances INT NOT NULL CHECK (nb_seances >= 0),
    total_quarts INT NOT NULL CHECK (total_quarts >= 0),
    exporte_par_personne_id UUID NOT NULL,
    exporte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_hse_export_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_hse_export_par_cfk FOREIGN KEY (etablissement_id, exporte_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_dem_hse_export_ajout_seul_trg ON ent_dem_hse_export;
CREATE TRIGGER ent_dem_hse_export_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_dem_hse_export
    FOR EACH ROW EXECUTE FUNCTION ent_dem_n13_ajout_seul();

-- ───────────────────────── Séances (faites ou prévues) ─────────────────────────
-- Une ligne par séance. La quantité transmise est `retenu_quarts` (≤ durée) : fixée par la décision. Une séance sans
-- détail demandé est portée par une séance « forfait » à la fin de la période.
CREATE TABLE IF NOT EXISTS ent_dem_hse_seance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    jour DATE NOT NULL,
    debut TIME,
    duree_quarts INT NOT NULL CHECK (duree_quarts BETWEEN 1 AND 1600),
    retenu_quarts INT CHECK (retenu_quarts IS NULL OR retenu_quarts BETWEEN 0 AND 1600),
    forfait BOOLEAN NOT NULL DEFAULT false,
    etat TEXT NOT NULL DEFAULT 'prevue' CHECK (etat IN ('prevue', 'faite', 'contestee', 'attestee', 'annulee')),
    lieu_libelle TEXT CHECK (lieu_libelle IS NULL OR length(lieu_libelle) <= 120),
    declaree_le TIMESTAMPTZ,
    declaree_par_personne_id UUID,
    -- Attestation (réglage facultatif, R-DM50) : sur un indice (appel, tiers) ou une déclaration sur l'honneur,
    -- celle-ci contrôlée par échantillon.
    fondement TEXT CHECK (fondement IS NULL OR fondement IN ('appel', 'tiers', 'declaration_honneur')),
    echantillon BOOLEAN NOT NULL DEFAULT false,
    attestee_par_personne_id UUID,
    attestee_le TIMESTAMPTZ,
    contestee_motif TEXT CHECK (contestee_motif IS NULL OR length(contestee_motif) <= 300),
    -- Transmission des heures : `aucune` (pas encore), `port_15` (socle 15), `export_secours` (état mensuel CSV),
    -- `hors_scolia` (reprise : déjà transmise au rectorat, jamais recomptée, CDM-50).
    transmission TEXT NOT NULL DEFAULT 'aucune' CHECK (transmission IN ('aucune', 'port_15', 'export_secours', 'hors_scolia')),
    transmise_le TIMESTAMPTZ,
    heure_constatee_id UUID,
    export_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_hse_seance_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_hse_seance_hse_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_hse (etablissement_id, demande_id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_seance_export_cfk FOREIGN KEY (etablissement_id, export_id)
        REFERENCES ent_dem_hse_export (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_seance_retenu_chk CHECK (retenu_quarts IS NULL OR retenu_quarts <= duree_quarts),
    CONSTRAINT ent_dem_hse_seance_transmise_chk CHECK ((transmission = 'aucune') = (transmise_le IS NULL)),
    CONSTRAINT ent_dem_hse_seance_export_chk CHECK (export_id IS NULL OR transmission = 'export_secours'),
    CONSTRAINT ent_dem_hse_seance_attestee_chk CHECK (
        (etat = 'attestee') = (attestee_le IS NOT NULL) AND (attestee_le IS NULL) = (attestee_par_personne_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_dem_hse_seance_demande_idx ON ent_dem_hse_seance (etablissement_id, demande_id, jour);
CREATE INDEX IF NOT EXISTS ent_dem_hse_seance_export_idx
    ON ent_dem_hse_seance (etablissement_id, jour) WHERE transmission = 'export_secours';

-- R-DM42 : une heure n'est transmise qu'une fois. La transmission ne se défait pas ; l'export d'un lot se pose une fois.
CREATE OR REPLACE FUNCTION ent_dem_hse_seance_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_hse_seance : suppression interdite (NJ-89) ; une séance s''annule' USING ERRCODE = '23001';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.demande_id <> OLD.demande_id
       OR NEW.cree_le <> OLD.cree_le THEN
        RAISE EXCEPTION 'ent_dem_hse_seance : identité figée' USING ERRCODE = '23001';
    END IF;
    IF OLD.transmission <> 'aucune' THEN
        IF NEW.transmission IS DISTINCT FROM OLD.transmission OR NEW.transmise_le IS DISTINCT FROM OLD.transmise_le
           OR NEW.jour <> OLD.jour OR NEW.retenu_quarts IS DISTINCT FROM OLD.retenu_quarts
           OR NEW.duree_quarts <> OLD.duree_quarts
           OR (OLD.export_id IS NOT NULL AND NEW.export_id IS DISTINCT FROM OLD.export_id) THEN
            RAISE EXCEPTION 'ent_dem_hse_seance : heure déjà transmise, plus modifiable (R-DM42) ; régularisation par le socle 15'
                USING ERRCODE = '23001';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_dem_hse_seance_garde_trg ON ent_dem_hse_seance;
CREATE TRIGGER ent_dem_hse_seance_garde_trg BEFORE UPDATE OR DELETE ON ent_dem_hse_seance
    FOR EACH ROW EXECUTE FUNCTION ent_dem_hse_seance_garde();

-- ───────────────────────── Enveloppe annuelle, EN QUARTS D'HEURE (D-061) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_hse_enveloppe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- NULL = enveloppe de l'établissement entier (aucune enveloppe saisie = aucune alerte).
    site_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_hse_enveloppe_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_hse_enveloppe_annee_cfk FOREIGN KEY (etablissement_id, annee_id)
        REFERENCES ent_annee_scolaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_enveloppe_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_hse_enveloppe_site_uq
    ON ent_dem_hse_enveloppe (etablissement_id, annee_id, site_id) WHERE site_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_hse_enveloppe_etab_uq
    ON ent_dem_hse_enveloppe (etablissement_id, annee_id) WHERE site_id IS NULL;
DROP TRIGGER IF EXISTS ent_dem_hse_enveloppe_sans_suppression_trg ON ent_dem_hse_enveloppe;
CREATE TRIGGER ent_dem_hse_enveloppe_sans_suppression_trg BEFORE DELETE ON ent_dem_hse_enveloppe
    FOR EACH ROW EXECUTE FUNCTION ent_dem_n13_sans_suppression();

CREATE TABLE IF NOT EXISTS ent_dem_hse_enveloppe_mvt (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    enveloppe_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('dotation', 'abondement', 'retrait', 'prevision')),
    quarts INT NOT NULL CHECK (quarts BETWEEN 1 AND 400000),
    note TEXT CHECK (note IS NULL OR length(note) <= 300),
    par_personne_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_hse_enveloppe_mvt_env_cfk FOREIGN KEY (etablissement_id, enveloppe_id)
        REFERENCES ent_dem_hse_enveloppe (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_hse_enveloppe_mvt_par_cfk FOREIGN KEY (etablissement_id, par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_dem_hse_enveloppe_mvt_idx ON ent_dem_hse_enveloppe_mvt (etablissement_id, enveloppe_id, le);
DROP TRIGGER IF EXISTS ent_dem_hse_enveloppe_mvt_ajout_seul_trg ON ent_dem_hse_enveloppe_mvt;
CREATE TRIGGER ent_dem_hse_enveloppe_mvt_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_dem_hse_enveloppe_mvt
    FOR EACH ROW EXECUTE FUNCTION ent_dem_n13_ajout_seul();

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_hse_reglage',
    'ent_dem_hse',
    'ent_dem_hse_public',
    'ent_dem_hse_decision',
    'ent_dem_hse_export',
    'ent_dem_hse_seance',
    'ent_dem_hse_enveloppe',
    'ent_dem_hse_enveloppe_mvt'
]);
