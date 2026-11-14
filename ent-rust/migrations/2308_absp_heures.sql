-- 2308 — Socle 15 (absences des personnels), lot A7 : heures constatées et états mensuels
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §14, §26 ; R-AP28, R-AP29 ; ABP-200 à 219).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée (E13).
--
-- 1. `ent_absp_heure` : l'HEURE CONSTATÉE. Elle naît d'un FAIT (R-AP28) : une séance remplacée assurée (appel fait
--    au socle 12, ou attestation du remplaçant) ; une saisie sans séance n'est permise que pour les natures qui n'en
--    ont pas (mission Pacte, HSE hors séance, vacation, service non fait). Jamais d'heure de remplacement (`rcd`)
--    sans séance. Le cycle : constatée → contestée → validée → figée → exportée ; figée ou exportée, elle ne change
--    plus : une correction devient une ligne de RÉGULARISATION sur l'état suivant (D-027 E), jamais une réécriture.
-- 2. `ent_absp_heure_fait` : faits sur une heure, en ajout seul (contestation, validation, correction,
--    régularisation, attestation).
-- 3. `ent_absp_etat` : l'état mensuel (figé, signé par le chef d'établissement, exporté) par employeur.
-- 4. `ent_absp_etat_ligne` : lignes de l'état, en ajout seul ; durée SIGNÉE : une régularisation est une ligne
--    négative ou positive rattachée à la ligne d'origine.
-- Écart au §26 : l'état « prepare » n'existe pas (l'état naît figé) ; le cycle de l'état est fige → signe → exporte.

-- 3. État mensuel (créé avant les heures : elles y renvoient) -------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_etat (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Premier jour du mois (jour de l'établissement).
    mois DATE NOT NULL CHECK (EXTRACT(DAY FROM mois) = 1),
    employeur TEXT NOT NULL CHECK (employeur IN ('etat', 'ogec')),
    etat TEXT NOT NULL DEFAULT 'fige' CHECK (etat IN ('fige', 'signe', 'exporte')),
    fige_par_user_id UUID NOT NULL,
    fige_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    signe_par_user_id UUID,
    signe_le TIMESTAMPTZ,
    exporte_par_user_id UUID,
    exporte_le TIMESTAMPTZ,
    -- Document généré et déposé au socle 7, trace d'export du socle 9 : références souples (ces socles ne sont pas
    -- dans ce lot).
    document_id UUID,
    export_trace_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_etat_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_etat_mois_employeur_uq UNIQUE (etablissement_id, mois, employeur),
    CONSTRAINT ent_absp_etat_signe_chk CHECK (etat = 'fige' OR (signe_par_user_id IS NOT NULL AND signe_le IS NOT NULL)),
    CONSTRAINT ent_absp_etat_exporte_chk CHECK ((etat = 'exporte') = (exporte_le IS NOT NULL))
);

CREATE OR REPLACE FUNCTION ent_trg_absp_etat_garde() RETURNS trigger AS $$
DECLARE
    rang_ancien INT;
    rang_nouveau INT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_etat : un état mensuel ne se supprime pas';
    END IF;
    rang_ancien := CASE OLD.etat WHEN 'fige' THEN 0 WHEN 'signe' THEN 1 ELSE 2 END;
    rang_nouveau := CASE NEW.etat WHEN 'fige' THEN 0 WHEN 'signe' THEN 1 ELSE 2 END;
    IF rang_nouveau < rang_ancien THEN
        RAISE EXCEPTION 'ent_absp_etat : l''état mensuel ne recule pas';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.mois <> OLD.mois
       OR NEW.employeur <> OLD.employeur OR NEW.fige_par_user_id <> OLD.fige_par_user_id
       OR NEW.fige_le <> OLD.fige_le THEN
        RAISE EXCEPTION 'ent_absp_etat : identité et instantané figés';
    END IF;
    IF OLD.etat <> 'fige' AND (NEW.signe_par_user_id IS DISTINCT FROM OLD.signe_par_user_id
                               OR NEW.signe_le IS DISTINCT FROM OLD.signe_le) THEN
        RAISE EXCEPTION 'ent_absp_etat : la signature ne change plus';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_etat_garde ON ent_absp_etat;
CREATE TRIGGER ent_absp_etat_garde BEFORE UPDATE OR DELETE ON ent_absp_etat
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_etat_garde();

-- 1. Heure constatée ------------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_heure (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Agent connu de l'établissement, avec ou sans compte (ABP-218 : remplaçant sans compte).
    agent_id UUID NOT NULL,
    -- Séance du socle 10 (référence souple : le socle 10 peut recalculer ou supprimer une séance) ; impact du 15.
    seance_id UUID,
    impact_id UUID,
    -- Rattrapage (lot A8) : référence souple, sans clé étrangère tant que la table n'existe pas.
    rattrapage_id UUID,
    nature TEXT NOT NULL CHECK (nature IN (
        'rcd', 'hse', 'mission_pacte', 'rattrapage_du', 'service_non_fait', 'vacation')),
    -- Zéro : heure annulée par une correction (rien ne s'efface).
    duree_minutes INTEGER NOT NULL CHECK (duree_minutes >= 0 AND duree_minutes <= 1440),
    -- Jour de l'établissement du fait (jour de la séance, ou jour saisi).
    le DATE NOT NULL,
    -- Premier jour du mois du fait : colonne calculée (immuable, sans conversion d'horodatage).
    mois DATE GENERATED ALWAYS AS (le - (EXTRACT(DAY FROM le)::integer - 1)) STORED,
    -- Classe de l'employeur au jour du fait (figée) : détermine l'état mensuel et la paie.
    employeur TEXT NOT NULL DEFAULT 'etat' CHECK (employeur IN ('etat', 'ogec')),
    etat TEXT NOT NULL DEFAULT 'constatee' CHECK (etat IN ('constatee', 'contestee', 'validee', 'figee', 'exportee')),
    preuve TEXT NOT NULL CHECK (preuve IN ('appel', 'attestation', 'saisie')),
    etat_id UUID,
    saisie_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_heure_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_heure_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_heure_impact_fk FOREIGN KEY (etablissement_id, impact_id)
        REFERENCES ent_absp_impact (etablissement_id, id),
    CONSTRAINT ent_absp_heure_etat_fk FOREIGN KEY (etablissement_id, etat_id)
        REFERENCES ent_absp_etat (etablissement_id, id),
    -- R-AP28 en base : une heure tirée d'une séance a pour preuve l'appel ou l'attestation ; une saisie n'a pas de
    -- séance et ne peut pas être une heure de remplacement.
    CONSTRAINT ent_absp_heure_fait_chk CHECK ((preuve = 'saisie') = (seance_id IS NULL)),
    CONSTRAINT ent_absp_heure_saisie_chk CHECK (
        preuve <> 'saisie' OR (nature IN ('hse', 'mission_pacte', 'vacation', 'service_non_fait') AND saisie_par_user_id IS NOT NULL)),
    CONSTRAINT ent_absp_heure_etat_fige_chk CHECK ((etat IN ('figee', 'exportee')) = (etat_id IS NOT NULL))
);
-- Une heure par séance, agent et nature : rejouer l'appel ou l'attestation ne crée pas de doublon.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_heure_seance_uq
    ON ent_absp_heure (etablissement_id, seance_id, agent_id, nature) WHERE seance_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_absp_heure_agent_idx ON ent_absp_heure (etablissement_id, agent_id, le);
CREATE INDEX IF NOT EXISTS ent_absp_heure_mois_idx ON ent_absp_heure (etablissement_id, mois, employeur, etat);

CREATE OR REPLACE FUNCTION ent_trg_absp_heure_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_heure : une heure constatée ne se supprime pas';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.agent_id <> OLD.agent_id
       OR NEW.seance_id IS DISTINCT FROM OLD.seance_id OR NEW.nature <> OLD.nature OR NEW.le <> OLD.le
       OR NEW.preuve <> OLD.preuve OR NEW.employeur <> OLD.employeur THEN
        RAISE EXCEPTION 'ent_absp_heure : identité de l''heure immuable';
    END IF;
    -- Figée ou exportée : plus aucune réécriture (régularisation sur l'état suivant). Seule transition permise :
    -- figée → exportée, quand l'état est exporté.
    IF OLD.etat IN ('figee', 'exportee') THEN
        IF NOT (OLD.etat = 'figee' AND NEW.etat = 'exportee' AND NEW.duree_minutes = OLD.duree_minutes
                AND NEW.etat_id IS NOT DISTINCT FROM OLD.etat_id) THEN
            RAISE EXCEPTION 'ent_absp_heure : une heure figée ne se réécrit pas (régularisation sur l''état suivant)';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_heure_garde ON ent_absp_heure;
CREATE TRIGGER ent_absp_heure_garde BEFORE UPDATE OR DELETE ON ent_absp_heure
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_heure_garde();

-- 2. Faits sur une heure (ajout seul) -------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_heure_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    heure_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'constat', 'attestation', 'contestation', 'validation', 'correction', 'regularisation')),
    -- NULL pour un constat automatique (appel fait).
    auteur_user_id UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    raison_code TEXT CHECK (raison_code IS NULL OR raison_code IN (
        'duree_erronee', 'seance_non_assuree', 'mauvais_agent', 'doublon', 'oubli', 'autre')),
    note VARCHAR(500),
    duree_avant INTEGER,
    duree_apres INTEGER,
    etat_avant TEXT,
    etat_apres TEXT,
    CONSTRAINT ent_absp_heure_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_heure_fait_heure_fk FOREIGN KEY (etablissement_id, heure_id)
        REFERENCES ent_absp_heure (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_heure_fait_heure_idx ON ent_absp_heure_fait (etablissement_id, heure_id, le);

DROP TRIGGER IF EXISTS ent_absp_heure_fait_ajout_seul ON ent_absp_heure_fait;
CREATE TRIGGER ent_absp_heure_fait_ajout_seul BEFORE UPDATE OR DELETE ON ent_absp_heure_fait
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rcd_ajout_seul();

-- 4. Lignes de l'état mensuel (ajout seul) --------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_etat_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    etat_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN (
        'rcd', 'hse', 'mission_pacte', 'rattrapage_du', 'service_non_fait', 'vacation')),
    -- Signée : une régularisation à la baisse est négative.
    duree_minutes INTEGER NOT NULL CHECK (duree_minutes <> 0),
    heure_id UUID,
    -- Régularisation : la ligne de l'état exporté qu'elle corrige, et le fait qui la motive.
    regularisation_de_id UUID,
    heure_fait_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_etat_ligne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_etat_ligne_etat_fk FOREIGN KEY (etablissement_id, etat_id)
        REFERENCES ent_absp_etat (etablissement_id, id),
    CONSTRAINT ent_absp_etat_ligne_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_etat_ligne_heure_fk FOREIGN KEY (etablissement_id, heure_id)
        REFERENCES ent_absp_heure (etablissement_id, id),
    CONSTRAINT ent_absp_etat_ligne_regul_fk FOREIGN KEY (etablissement_id, regularisation_de_id)
        REFERENCES ent_absp_etat_ligne (etablissement_id, id),
    CONSTRAINT ent_absp_etat_ligne_fait_fk FOREIGN KEY (etablissement_id, heure_fait_id)
        REFERENCES ent_absp_heure_fait (etablissement_id, id),
    CONSTRAINT ent_absp_etat_ligne_regul_chk CHECK ((regularisation_de_id IS NULL) = (heure_fait_id IS NULL))
);
-- Une heure n'entre qu'une fois, comme ligne d'origine, dans un état ; un fait de régularisation n'est reporté
-- qu'une fois.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_etat_ligne_heure_uq
    ON ent_absp_etat_ligne (etablissement_id, heure_id) WHERE regularisation_de_id IS NULL AND heure_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_etat_ligne_fait_uq
    ON ent_absp_etat_ligne (etablissement_id, heure_fait_id) WHERE heure_fait_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_absp_etat_ligne_etat_idx ON ent_absp_etat_ligne (etablissement_id, etat_id);

DROP TRIGGER IF EXISTS ent_absp_etat_ligne_ajout_seul ON ent_absp_etat_ligne;
CREATE TRIGGER ent_absp_etat_ligne_ajout_seul BEFORE UPDATE OR DELETE ON ent_absp_etat_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rcd_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_absp_etat', 'ent_absp_heure', 'ent_absp_heure_fait', 'ent_absp_etat_ligne']);
