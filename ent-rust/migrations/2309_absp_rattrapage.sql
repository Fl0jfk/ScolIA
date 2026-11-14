-- 2309 — Socle 15 (absences des personnels), lot A8 : rattrapages dus
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §15, §26 ; R-AP26, R-AP27, R-AP28 ; S-1514, S-1515 ; ABP-190 à 199).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée (E13).
--
-- 1. `ent_absp_rattrapage` : le RATTRAPAGE DÛ, une ligne par absence « à rattraper » (traitement `rattrapage`). Heures dues
--    prises dans l'impact, échéance (défaut : fin de l'année scolaire, S-1514). Cycle : du → planifié (un créneau
--    confirmé) → soldé ; la direction peut prolonger, requalifier en retenue ou abandonner avec motif. SOLDÉ exige, EN BASE,
--    un créneau assuré : jamais une déclaration seule (R-AP28).
-- 2. `ent_absp_rattrapage_creneau` : les créneaux proposés par l'agent, confirmés par la direction, puis assurés.
--    La séance ajoutée du socle 10 est une RÉFÉRENCE SOUPLE (`seance_ref`), sans clé étrangère : le 15 n'écrit jamais dans
--    les tables du 10 (R-AP26). Jamais deux créneaux confirmés ou assurés qui se chevauchent pour un même agent
--    (contrainte d'exclusion). Un créneau assuré ne change plus ; rien ne se supprime.
-- Ni la nature de l'absence ni un texte libre n'apparaissent dans ces tables (niveau « organisation », R-AP2).

-- 0. Journal : objet « rattrapage » ----------------------------------------------------------------------------
ALTER TABLE ent_absp_journal DROP CONSTRAINT IF EXISTS ent_absp_journal_objet_check;
ALTER TABLE ent_absp_journal ADD CONSTRAINT ent_absp_journal_objet_check CHECK (objet IN (
    'absence', 'circuit_regle', 'param', 'nature_etab',
    'rcd_plan', 'rcd_engagement', 'rcd_remontee', 'heure', 'etat', 'element_paie', 'dsn', 'rattrapage'));

-- 1. Rattrapage dû -----------------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_rattrapage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    heures_dues NUMERIC(5,1) NOT NULL CHECK (heures_dues > 0),
    -- Jour de l'établissement.
    echeance DATE NOT NULL,
    etat TEXT NOT NULL DEFAULT 'du' CHECK (etat IN ('du', 'planifie', 'solde', 'prolonge', 'retenue', 'abandonne')),
    -- Cours rendu aux mêmes élèves (enseignant) ou heures de service (salarié).
    forme TEXT NOT NULL DEFAULT 'cours' CHECK (forme IN ('cours', 'service')),
    -- Relance automatique des créneaux (R-AP27) et alerte d'échéance : chacune une seule fois, jour de l'établissement.
    relance_le DATE,
    alerte_echeance_le DATE,
    -- Décision de la direction (prolonger, requalifier, abandonner) : motif CODÉ, jamais un texte libre.
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'accord_agent', 'absence_prolongee', 'calendrier', 'impossible', 'decision_direction', 'autre')),
    decide_par_user_id UUID,
    decide_le TIMESTAMPTZ,
    solde_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rattrapage_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rattrapage_absence_uq UNIQUE (etablissement_id, absence_id),
    CONSTRAINT ent_absp_rattrapage_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_rattrapage_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_rattrapage_solde_chk CHECK ((etat = 'solde') = (solde_le IS NOT NULL)),
    CONSTRAINT ent_absp_rattrapage_decision_chk CHECK (
        etat NOT IN ('retenue', 'abandonne', 'prolonge') OR (decide_par_user_id IS NOT NULL AND decide_le IS NOT NULL)),
    CONSTRAINT ent_absp_rattrapage_abandon_chk CHECK (etat <> 'abandonne' OR motif_code IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_absp_rattrapage_agent_idx ON ent_absp_rattrapage (etablissement_id, agent_id, etat);
CREATE INDEX IF NOT EXISTS ent_absp_rattrapage_echeance_idx
    ON ent_absp_rattrapage (etablissement_id, echeance) WHERE etat IN ('du', 'planifie', 'prolonge');

-- 2. Créneaux ----------------------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_rattrapage_creneau (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    rattrapage_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    plage TSTZRANGE GENERATED ALWAYS AS (tstzrange(debut, fin, '[)')) STORED,
    etat TEXT NOT NULL DEFAULT 'propose' CHECK (etat IN ('propose', 'confirme', 'assure', 'refuse', 'annule')),
    propose_par_user_id UUID NOT NULL,
    propose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirme_par_user_id UUID,
    confirme_le TIMESTAMPTZ,
    -- Séance `ajoutee` du socle 10 (référence souple, R-AP26) ; NULL pour des heures de service sans séance.
    seance_ref UUID,
    assure_preuve TEXT CHECK (assure_preuve IS NULL OR assure_preuve IN ('appel', 'attestation')),
    assure_le TIMESTAMPTZ,
    assure_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rattrapage_creneau_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rattrapage_creneau_dates_chk CHECK (fin > debut AND fin - debut <= interval '12 hours'),
    CONSTRAINT ent_absp_rattrapage_creneau_rattrapage_fk FOREIGN KEY (etablissement_id, rattrapage_id)
        REFERENCES ent_absp_rattrapage (etablissement_id, id),
    CONSTRAINT ent_absp_rattrapage_creneau_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    -- Confirmé ou assuré : la direction l'a confirmé. Assuré : un FAIT (appel fait, ou attestation), jamais une déclaration.
    CONSTRAINT ent_absp_rattrapage_creneau_confirme_chk CHECK (
        etat NOT IN ('confirme', 'assure') OR (confirme_par_user_id IS NOT NULL AND confirme_le IS NOT NULL)),
    CONSTRAINT ent_absp_rattrapage_creneau_assure_chk CHECK (
        (etat = 'assure') = (assure_preuve IS NOT NULL AND assure_le IS NOT NULL)),
    CONSTRAINT ent_absp_rattrapage_creneau_appel_chk CHECK (assure_preuve IS DISTINCT FROM 'appel' OR seance_ref IS NOT NULL),
    -- Jamais deux créneaux retenus qui se chevauchent pour un même agent, quel que soit le chemin.
    CONSTRAINT ent_absp_rattrapage_creneau_sans_chevauchement
        EXCLUDE USING gist (etablissement_id WITH =, agent_id WITH =, plage WITH &&)
        WHERE (etat IN ('confirme', 'assure'))
);
-- Une séance du 10 ne sert qu'un créneau retenu.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_rattrapage_creneau_seance_uq
    ON ent_absp_rattrapage_creneau (etablissement_id, seance_ref)
    WHERE seance_ref IS NOT NULL AND etat IN ('confirme', 'assure');
CREATE INDEX IF NOT EXISTS ent_absp_rattrapage_creneau_idx
    ON ent_absp_rattrapage_creneau (etablissement_id, rattrapage_id, etat);

-- Gardes : rien ne se supprime ; l'identité est immuable ; un état terminal ne bouge plus ; SOLDÉ exige un créneau assuré.
CREATE OR REPLACE FUNCTION ent_trg_absp_rattrapage_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_rattrapage : un rattrapage ne se supprime pas';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.absence_id <> OLD.absence_id
       OR NEW.agent_id <> OLD.agent_id THEN
        RAISE EXCEPTION 'ent_absp_rattrapage : identité du rattrapage immuable';
    END IF;
    IF OLD.etat IN ('solde', 'retenue', 'abandonne') THEN
        RAISE EXCEPTION 'ent_absp_rattrapage : un rattrapage clos ne change plus';
    END IF;
    IF NEW.etat = 'solde' AND NOT EXISTS (
        SELECT 1 FROM ent_absp_rattrapage_creneau c
         WHERE c.etablissement_id = NEW.etablissement_id AND c.rattrapage_id = NEW.id AND c.etat = 'assure') THEN
        RAISE EXCEPTION 'ent_absp_rattrapage : un rattrapage ne se solde que par un créneau assuré';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_rattrapage_garde ON ent_absp_rattrapage;
CREATE TRIGGER ent_absp_rattrapage_garde BEFORE UPDATE OR DELETE ON ent_absp_rattrapage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rattrapage_garde();

CREATE OR REPLACE FUNCTION ent_trg_absp_rattrapage_creneau_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_rattrapage_creneau : un créneau ne se supprime pas';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.rattrapage_id <> OLD.rattrapage_id
       OR NEW.agent_id <> OLD.agent_id OR NEW.debut <> OLD.debut OR NEW.fin <> OLD.fin
       OR NEW.propose_par_user_id <> OLD.propose_par_user_id THEN
        RAISE EXCEPTION 'ent_absp_rattrapage_creneau : identité du créneau immuable';
    END IF;
    IF OLD.etat IN ('assure', 'refuse', 'annule') THEN
        RAISE EXCEPTION 'ent_absp_rattrapage_creneau : un créneau assuré, refusé ou annulé ne change plus';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_rattrapage_creneau_garde ON ent_absp_rattrapage_creneau;
CREATE TRIGGER ent_absp_rattrapage_creneau_garde BEFORE UPDATE OR DELETE ON ent_absp_rattrapage_creneau
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rattrapage_creneau_garde();

-- Le lien souple posé par l'A7 (`ent_absp_heure.rattrapage_id`) devient une clé étrangère maintenant que la table existe.
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_absp_heure_rattrapage_fk') THEN
        ALTER TABLE ent_absp_heure ADD CONSTRAINT ent_absp_heure_rattrapage_fk
            FOREIGN KEY (etablissement_id, rattrapage_id) REFERENCES ent_absp_rattrapage (etablissement_id, id);
    END IF;
END $$;

SELECT ent_rls_poser(ARRAY['ent_absp_rattrapage', 'ent_absp_rattrapage_creneau']);
