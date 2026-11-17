-- Socle 28, lot R3 (MS4 d) — migration 2567 : rapprochement des référentiels et des personnes, file « à décider »,
-- décisions tracées, synonymes validés.
-- Réf. : docs/socles/28_MISE_EN_SERVICE.md §15–§16 (rep_decision, rep_synonyme), T0 §6, docs/reprise/R3_RAPPROCHEMENT.md.
--
-- * `rep_execution_compte.rapprochees` : fiches de l'export reliées à une fiche EXISTANTE de ScolIA (lien à enregistrer,
--   aucune fiche créée ni modifiée). L'invariant du rapport (CHECK) en tient compte.
-- * `rep_proposition*` : la file « à décider » d'une exécution : une ligne par fiche de l'export, ses candidats ScolIA, leurs
--   raisons (codes) et leur confiance. Résumés MINIMISÉS (ce qui sert à distinguer deux personnes : nom, prénom, naissance,
--   classe, fin d'INE, adresse masquée) ; ils sont mis à NULL à la purge de la zone de reprise (MSE-179).
-- * `rep_decision` : trace des décisions humaines (qui, quand, quoi, quelles raisons étaient affichées) ; en ajout seul : seule
--   l'annulation (réversibilité tant que la reprise n'est pas appliquée) peut y être posée, une fois.
-- * `rep_synonyme` : équivalences de libellés (classes, sites) validées par une personne, rejouées aux exécutions suivantes.
-- Aucune colonne JSON (D-036), aucune clé étrangère vers le métier (la cible dépend de `cible_objet`). RLS d'établissement.
-- `rep_echantillon` et `rep_bascule*` : lot « rapports et bascule » (MS4 e), pas ici.

ALTER TABLE rep_execution_compte ADD COLUMN IF NOT EXISTS rapprochees BIGINT NOT NULL DEFAULT 0 CHECK (rapprochees >= 0);
ALTER TABLE rep_execution_compte DROP CONSTRAINT IF EXISTS rep_execution_compte_invariant_chk;
ALTER TABLE rep_execution_compte ADD CONSTRAINT rep_execution_compte_invariant_chk CHECK (
    lues = creees + mises_a_jour + inchangees + archivees + ignorees + refusees + a_decider + rapprochees);

SELECT ent_d036_contrainte('rep_correspondance', 'rep_correspondance_etab_id_uq', 'UNIQUE (etablissement_id, id)');

-- Motifs d'une ligne « à décider » : ceux de R2 et ceux du rapprochement (la file humaine porte le détail).
ALTER TABLE rep_execution_a_decider DROP CONSTRAINT IF EXISTS rep_execution_a_decider_motif_check;
ALTER TABLE rep_execution_a_decider ADD CONSTRAINT rep_execution_a_decider_motif_check CHECK (motif IN (
    'rapprochement_a_decider', 'modifie_depuis_reprise', 'aucun_candidat', 'rapprochement_incertain',
    'rapprochement_ambigu', 'rapprochement_contradictoire', 'rapprochement_hors_r3'));

-- ───────────────────────── File « à décider » ─────────────────────────
CREATE TABLE IF NOT EXISTS rep_proposition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    execution_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (length(objet) BETWEEN 1 AND 120),
    source_id TEXT NOT NULL CHECK (length(source_id) BETWEEN 1 AND 400),
    source_ref TEXT NOT NULL CHECK (length(source_ref) BETWEEN 1 AND 220),
    cible_objet TEXT NOT NULL CHECK (length(cible_objet) BETWEEN 1 AND 60),
    motif TEXT NOT NULL CHECK (motif IN ('aucun_candidat', 'rapprochement_incertain', 'rapprochement_ambigu', 'rapprochement_contradictoire')),
    meilleure_confiance TEXT CHECK (meilleure_confiance IS NULL OR meilleure_confiance IN ('faible', 'moyenne', 'forte', 'certaine')),
    empreinte_source TEXT CHECK (empreinte_source IS NULL OR length(empreinte_source) = 64),
    -- Résumé minimisé de la fiche de l'export (NULL après purge).
    src_nom TEXT CHECK (src_nom IS NULL OR length(src_nom) <= 200),
    src_prenom TEXT CHECK (src_prenom IS NULL OR length(src_prenom) <= 200),
    src_naissance DATE,
    src_classe TEXT CHECK (src_classe IS NULL OR length(src_classe) <= 120),
    src_ine_fin TEXT CHECK (src_ine_fin IS NULL OR length(src_ine_fin) <= 12),
    src_contact TEXT CHECK (src_contact IS NULL OR length(src_contact) <= 120),
    src_detail TEXT CHECK (src_detail IS NULL OR length(src_detail) <= 200),
    resume_purge_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rep_proposition_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT rep_proposition_ligne_uq UNIQUE (etablissement_id, execution_id, objet, source_id),
    CONSTRAINT rep_proposition_exec_fk FOREIGN KEY (etablissement_id, execution_id)
        REFERENCES rep_execution (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS rep_proposition_exec_idx ON rep_proposition (etablissement_id, execution_id, objet);

CREATE TABLE IF NOT EXISTS rep_proposition_candidat (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    proposition_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang BETWEEN 1 AND 5),
    cible_id UUID NOT NULL,
    confiance TEXT NOT NULL CHECK (confiance IN ('faible', 'moyenne', 'forte', 'certaine')),
    cand_nom TEXT CHECK (cand_nom IS NULL OR length(cand_nom) <= 200),
    cand_prenom TEXT CHECK (cand_prenom IS NULL OR length(cand_prenom) <= 200),
    cand_naissance DATE,
    cand_classe TEXT CHECK (cand_classe IS NULL OR length(cand_classe) <= 120),
    cand_ine_fin TEXT CHECK (cand_ine_fin IS NULL OR length(cand_ine_fin) <= 12),
    cand_contact TEXT CHECK (cand_contact IS NULL OR length(cand_contact) <= 120),
    cand_detail TEXT CHECK (cand_detail IS NULL OR length(cand_detail) <= 200),
    PRIMARY KEY (proposition_id, rang),
    CONSTRAINT rep_proposition_candidat_cible_uq UNIQUE (proposition_id, cible_id),
    CONSTRAINT rep_proposition_candidat_prop_fk FOREIGN KEY (etablissement_id, proposition_id)
        REFERENCES rep_proposition (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS rep_proposition_candidat_etab_idx ON rep_proposition_candidat (etablissement_id, proposition_id);

CREATE TABLE IF NOT EXISTS rep_proposition_raison (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    proposition_id UUID NOT NULL,
    rang SMALLINT NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    polarite TEXT NOT NULL CHECK (polarite IN ('pour', 'contre', 'info')),
    PRIMARY KEY (proposition_id, rang, code),
    CONSTRAINT rep_proposition_raison_cand_fk FOREIGN KEY (proposition_id, rang)
        REFERENCES rep_proposition_candidat (proposition_id, rang) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS rep_proposition_raison_etab_idx ON rep_proposition_raison (etablissement_id, proposition_id);

-- ───────────────────────── Décisions humaines (ajout seul) ─────────────────────────
CREATE TABLE IF NOT EXISTS rep_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    correspondance_id UUID NOT NULL,
    execution_id UUID,
    decision TEXT NOT NULL CHECK (decision IN ('rattacher', 'creer', 'ignorer')),
    cible_id UUID,
    par_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    confiance_vue TEXT CHECK (confiance_vue IS NULL OR confiance_vue IN ('faible', 'moyenne', 'forte', 'certaine')),
    en_lot BOOLEAN NOT NULL DEFAULT false,
    annulee_le TIMESTAMPTZ,
    annulee_par UUID,
    annulee_motif TEXT CHECK (annulee_motif IS NULL OR length(btrim(annulee_motif)) BETWEEN 1 AND 500),
    CONSTRAINT rep_decision_cible_chk CHECK ((decision = 'rattacher') = (cible_id IS NOT NULL)),
    CONSTRAINT rep_decision_annulation_chk CHECK ((annulee_le IS NULL) = (annulee_par IS NULL)),
    CONSTRAINT rep_decision_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT rep_decision_corr_fk FOREIGN KEY (etablissement_id, correspondance_id)
        REFERENCES rep_correspondance (etablissement_id, id)
);
-- Une seule décision en vigueur par fiche de l'export ; les annulées restent comme historique.
CREATE UNIQUE INDEX IF NOT EXISTS rep_decision_active_uq
    ON rep_decision (etablissement_id, correspondance_id) WHERE annulee_le IS NULL;
CREATE INDEX IF NOT EXISTS rep_decision_corr_idx ON rep_decision (etablissement_id, correspondance_id, le DESC);

CREATE TABLE IF NOT EXISTS rep_decision_raison (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    decision_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    polarite TEXT NOT NULL CHECK (polarite IN ('pour', 'contre', 'info')),
    PRIMARY KEY (decision_id, code),
    CONSTRAINT rep_decision_raison_dec_fk FOREIGN KEY (etablissement_id, decision_id)
        REFERENCES rep_decision (etablissement_id, id)
);

CREATE OR REPLACE FUNCTION rep_decision_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rep_decision est en ajout seul (trace des décisions)' USING ERRCODE = '42501';
    END IF;
    -- Seule l'annulation peut être posée, une fois : rien d'autre ne change.
    IF OLD.annulee_le IS NOT NULL
       OR (NEW.id, NEW.etablissement_id, NEW.correspondance_id, NEW.execution_id, NEW.decision, NEW.cible_id,
           NEW.par_user_id, NEW.le, NEW.motif, NEW.confiance_vue, NEW.en_lot)
          IS DISTINCT FROM
          (OLD.id, OLD.etablissement_id, OLD.correspondance_id, OLD.execution_id, OLD.decision, OLD.cible_id,
           OLD.par_user_id, OLD.le, OLD.motif, OLD.confiance_vue, OLD.en_lot) THEN
        RAISE EXCEPTION 'rep_decision : seule l''annulation (une fois) peut être posée' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rep_decision_garde_trg ON rep_decision;
CREATE TRIGGER rep_decision_garde_trg BEFORE UPDATE OR DELETE ON rep_decision
    FOR EACH ROW EXECUTE FUNCTION rep_decision_garde();

-- ───────────────────────── Synonymes validés ─────────────────────────
CREATE TABLE IF NOT EXISTS rep_synonyme (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    source TEXT NOT NULL DEFAULT 'next' CHECK (source ~ '^[a-z][a-z_]{0,29}$'),
    domaine TEXT NOT NULL CHECK (domaine IN ('classe', 'site', 'matiere')),
    valeur_cle TEXT NOT NULL CHECK (length(valeur_cle) BETWEEN 1 AND 200),
    valeur_cible_id UUID NOT NULL,
    propose_par TEXT NOT NULL CHECK (propose_par IN ('sys', 'ia', 'personne')),
    valide_par UUID,
    valide_le TIMESTAMPTZ,
    decision_id UUID,
    annule_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rep_synonyme_validation_chk CHECK ((valide_par IS NULL) = (valide_le IS NULL)),
    CONSTRAINT rep_synonyme_uq UNIQUE (etablissement_id, source, domaine, valeur_cle)
);
CREATE INDEX IF NOT EXISTS rep_synonyme_etab_idx ON rep_synonyme (etablissement_id, domaine);

SELECT ent_rls_poser(ARRAY[
    'rep_proposition', 'rep_proposition_candidat', 'rep_proposition_raison',
    'rep_decision', 'rep_decision_raison', 'rep_synonyme']);
