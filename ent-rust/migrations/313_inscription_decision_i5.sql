-- Lot I5 (socle 3, `docs/socles/03_INSCRIPTIONS.md` §7.1, §8.7, §11.4, §11.5) — décisions, avis, commissions,
-- liste d'attente, propositions de place et conditions de confirmation.
--
-- Additif et rejouable. Plage 313–329 du socle 3 (310–312 : lots I3 et I4).
--
-- * `ent_inscription_decision` : devient **en ajout seul** (une décision revue = une nouvelle ligne qui
--   remplace la précédente, jamais une écriture sur l'ancienne). Seule la publication à la famille complète une
--   ligne, une fois (`publiee_le`, `publiee_par`) ; la date programmée se change tant que la décision n'est pas
--   publiée. Les décisions reprises avant ce lot sont réputées publiées (les familles les lisaient déjà).
-- * `ent_inscription_avis` : avis internes (entretien, commission, pédagogie), en ajout seul.
-- * `ent_inscription_deport` : conflit d'intérêts déclaré par un membre du personnel (INS-63).
-- * `ent_inscription_commission` et `_dossier` : séance, dossiers examinés, procès-verbal.
-- * `ent_inscription_proposition_place` : place proposée à une famille (admission directe ou depuis la liste
--   d'attente) ; sa réponse, son délai.
-- * `ent_inscription_condition_param` (par campagne) et `ent_inscription_condition` (par dossier) : ce qui reste
--   à remplir pour confirmer.
-- Aucune décision n'est prise par une règle : le système **désigne** (liste d'attente) ou **calcule**
-- (conditions) ; une personne décide, envoie, publie.

-- ───────────────────────────── Décisions ─────────────────────────────
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_name = 'ent_inscription_decision' AND column_name = 'publiee_le') THEN
        ALTER TABLE ent_inscription_decision
            ADD COLUMN publiee_le TIMESTAMPTZ,
            ADD COLUMN publiee_par UUID,
            ADD COLUMN publication_prevue_le TIMESTAMPTZ,
            ADD COLUMN alternative BOOLEAN NOT NULL DEFAULT false,
            ADD COLUMN origine TEXT NOT NULL DEFAULT 'direction';
        -- Reprise : ce qui était décidé avant ce lot était déjà lu par les familles.
        UPDATE ent_inscription_decision SET publiee_le = decide_le, origine = 'reprise';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_decision_origine_chk') THEN
        ALTER TABLE ent_inscription_decision
            ADD CONSTRAINT ent_inscription_decision_origine_chk
            CHECK (origine IN ('direction', 'commission', 'liste_attente', 'reprise'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS ent_inscription_decision_a_publier_idx
    ON ent_inscription_decision (etablissement_id, publication_prevue_le) WHERE publiee_le IS NULL;

-- Ajout seul : rien ne s'écrit sur une décision, sauf la publication (une fois) et sa date programmée tant que
-- la décision n'est pas publiée. Une suppression directe est refusée ; seule la purge du dossier (cascade) passe.
CREATE OR REPLACE FUNCTION ent_inscription_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF pg_trigger_depth() < 2 THEN
            RAISE EXCEPTION 'historique en ajout seul : suppression refusée (%)', TG_TABLE_NAME
                USING ERRCODE = 'check_violation';
        END IF;
        RETURN OLD;
    END IF;
    IF TG_TABLE_NAME = 'ent_inscription_decision' THEN
        IF (to_jsonb(NEW) - 'publiee_le' - 'publiee_par' - 'publication_prevue_le')
           IS DISTINCT FROM (to_jsonb(OLD) - 'publiee_le' - 'publiee_par' - 'publication_prevue_le') THEN
            RAISE EXCEPTION 'une décision ne se modifie pas : une nouvelle décision la remplace'
                USING ERRCODE = 'check_violation';
        END IF;
        IF OLD.publiee_le IS NOT NULL AND (NEW.publiee_le IS DISTINCT FROM OLD.publiee_le
                                           OR NEW.publiee_par IS DISTINCT FROM OLD.publiee_par
                                           OR NEW.publication_prevue_le IS DISTINCT FROM OLD.publication_prevue_le) THEN
            RAISE EXCEPTION 'une décision publiée ne se modifie plus'
                USING ERRCODE = 'check_violation';
        END IF;
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'historique en ajout seul : modification refusée (%)', TG_TABLE_NAME
        USING ERRCODE = 'check_violation';
END $$;

DROP TRIGGER IF EXISTS ent_inscription_decision_ajout_seul_trg ON ent_inscription_decision;
CREATE TRIGGER ent_inscription_decision_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_inscription_decision
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_ajout_seul();

-- Le journal du dossier connaît les faits du lot (décision publiée, avis, déport, condition, liste d'attente,
-- réponse de la famille).
ALTER TABLE ent_inscription_suivi_dossier DROP CONSTRAINT IF EXISTS ent_inscription_suivi_dossier_kind_check;
ALTER TABLE ent_inscription_suivi_dossier DROP CONSTRAINT IF EXISTS ent_inscription_suivi_dossier_kind_i5_chk;
ALTER TABLE ent_inscription_suivi_dossier
    ADD CONSTRAINT ent_inscription_suivi_dossier_kind_i5_chk
    CHECK (kind IN ('statut', 'commentaire', 'piece', 'relance', 'decision', 'avis', 'deport', 'condition',
                    'liste_attente', 'reponse'));

-- ───────────────────────────── Avis internes (INS-31) ─────────────────────────────
CREATE TABLE IF NOT EXISTS ent_inscription_avis (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    auteur_id UUID NOT NULL,
    contexte TEXT NOT NULL CHECK (contexte IN ('entretien', 'commission', 'pedagogique')),
    sens TEXT NOT NULL CHECK (sens IN ('favorable', 'reserve', 'defavorable', 'sans_avis')),
    texte TEXT NOT NULL CHECK (char_length(texte) BETWEEN 3 AND 2000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_avis_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_avis_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_avis_dossier_idx
    ON ent_inscription_avis (etablissement_id, dossier_id, cree_le);
DROP TRIGGER IF EXISTS ent_inscription_avis_ajout_seul_trg ON ent_inscription_avis;
CREATE TRIGGER ent_inscription_avis_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_inscription_avis
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_ajout_seul();

-- ───────────────────────────── Déport (INS-63) ─────────────────────────────
CREATE TABLE IF NOT EXISTS ent_inscription_deport (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    membre_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (char_length(motif) BETWEEN 3 AND 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_deport_uq UNIQUE (dossier_id, membre_id),
    CONSTRAINT ent_inscription_deport_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE
);
DROP TRIGGER IF EXISTS ent_inscription_deport_ajout_seul_trg ON ent_inscription_deport;
CREATE TRIGGER ent_inscription_deport_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_inscription_deport
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_ajout_seul();

-- ───────────────────────────── Commissions (INS-32) ─────────────────────────────
CREATE TABLE IF NOT EXISTS ent_inscription_commission (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL,
    intitule TEXT NOT NULL CHECK (char_length(btrim(intitule)) BETWEEN 3 AND 120),
    seance_le TIMESTAMPTZ NOT NULL,
    statut TEXT NOT NULL DEFAULT 'preparee' CHECK (statut IN ('preparee', 'tenue', 'cloturee')),
    membres UUID[] NOT NULL DEFAULT '{}',
    pv_texte TEXT CHECK (pv_texte IS NULL OR char_length(pv_texte) <= 20000),
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cloturee_le TIMESTAMPTZ,
    CONSTRAINT ent_inscription_commission_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_commission_campagne_fk
        FOREIGN KEY (etablissement_id, campagne_id)
        REFERENCES ent_inscription_campagne (etablissement_id, id)
);
CREATE TABLE IF NOT EXISTS ent_inscription_commission_dossier (
    commission_id UUID NOT NULL,
    dossier_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    ordre INTEGER NOT NULL DEFAULT 0,
    examine_le TIMESTAMPTZ,
    PRIMARY KEY (commission_id, dossier_id),
    CONSTRAINT ent_inscription_commission_dossier_commission_fk
        FOREIGN KEY (etablissement_id, commission_id)
        REFERENCES ent_inscription_commission (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_inscription_commission_dossier_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_commission_dossier_dossier_idx
    ON ent_inscription_commission_dossier (etablissement_id, dossier_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_decision_commission_fk') THEN
        ALTER TABLE ent_inscription_decision
            ADD CONSTRAINT ent_inscription_decision_commission_fk
            FOREIGN KEY (etablissement_id, commission_id)
            REFERENCES ent_inscription_commission (etablissement_id, id);
    END IF;
END $$;

-- ───────────────────────────── Propositions de place (INS-34, INS-35) ─────────────────────────────
-- `designee` : le système a désigné le suivant de la liste ; rien n'est parti. `envoyee` : une personne a
-- confirmé (la décision « admis » est prise et publiée). `acceptee`, `declinee`, `expiree` (constatée par une
-- personne), `ecartee` (une personne passe son tour, motif), `annulee` (la place n'est plus à proposer).
CREATE TABLE IF NOT EXISTS ent_inscription_proposition_place (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    offre_id UUID,
    origine TEXT NOT NULL CHECK (origine IN ('decision', 'liste_attente')),
    statut TEXT NOT NULL DEFAULT 'designee'
        CHECK (statut IN ('designee', 'envoyee', 'acceptee', 'declinee', 'expiree', 'ecartee', 'annulee')),
    rang_au_moment INTEGER,
    designee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    envoyee_le TIMESTAMPTZ,
    envoyee_par UUID,
    repondre_avant DATE,
    repondu_le TIMESTAMPTZ,
    decision_id UUID,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 500),
    CONSTRAINT ent_inscription_proposition_place_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_proposition_place_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_inscription_proposition_place_offre_fk
        FOREIGN KEY (etablissement_id, offre_id)
        REFERENCES ent_inscription_offre (etablissement_id, id),
    CONSTRAINT ent_inscription_proposition_place_decision_fk
        FOREIGN KEY (etablissement_id, decision_id)
        REFERENCES ent_inscription_decision (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_proposition_place_dossier_idx
    ON ent_inscription_proposition_place (etablissement_id, dossier_id, designee_le);
CREATE INDEX IF NOT EXISTS ent_inscription_proposition_place_offre_idx
    ON ent_inscription_proposition_place (etablissement_id, offre_id, statut);
-- Une seule proposition ouverte (désignée ou envoyée) par dossier.
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_proposition_place_ouverte_uq
    ON ent_inscription_proposition_place (etablissement_id, dossier_id) WHERE statut IN ('designee', 'envoyee');

-- ───────────────────────────── Conditions de confirmation (§7.1, §8.7) ─────────────────────────────
-- Par défaut une campagne n'exige aucune condition de ce tableau : le comportement d'avant ce lot est conservé
-- (D-069 : paramètre de l'établissement). L'accord des responsables (I4) et les pièces bloquantes (I3) restent
-- exigés par leurs propres règles et sont **calculés**, jamais saisis.
CREATE TABLE IF NOT EXISTS ent_inscription_condition_param (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN ('reponse_famille', 'contrat_signe', 'frais_payes', 'acompte_paye',
                                       'situation_financiere_examinee', 'passage')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (campagne_id, code),
    CONSTRAINT ent_inscription_condition_param_campagne_fk
        FOREIGN KEY (etablissement_id, campagne_id)
        REFERENCES ent_inscription_campagne (etablissement_id, id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS ent_inscription_condition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN ('reponse_famille', 'contrat_signe', 'frais_payes', 'acompte_paye',
                                       'situation_financiere_examinee', 'passage', 'reserve')),
    libelle TEXT NOT NULL CHECK (char_length(libelle) BETWEEN 1 AND 500),
    remplie BOOLEAN NOT NULL DEFAULT false,
    remplie_le TIMESTAMPTZ,
    remplie_par UUID,
    source TEXT CHECK (source IS NULL OR source IN ('famille', 'personnel', 'evenement', 'levee')),
    source_ref UUID,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_condition_uq UNIQUE (dossier_id, code),
    CONSTRAINT ent_inscription_condition_coherence_chk
        CHECK ((remplie AND remplie_le IS NOT NULL AND source IS NOT NULL) OR (NOT remplie)),
    CONSTRAINT ent_inscription_condition_levee_chk
        CHECK (source IS DISTINCT FROM 'levee' OR (motif IS NOT NULL AND remplie_par IS NOT NULL)),
    CONSTRAINT ent_inscription_condition_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE
);

SELECT ent_rls_poser(ARRAY[
    'ent_inscription_avis',
    'ent_inscription_deport',
    'ent_inscription_commission',
    'ent_inscription_commission_dossier',
    'ent_inscription_proposition_place',
    'ent_inscription_condition_param',
    'ent_inscription_condition'
]);
