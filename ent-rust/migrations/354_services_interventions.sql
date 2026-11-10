-- Socle 6 (personnels), lot P3, migration 354 : services d'enseignement et interventions
-- (`docs/socles/06_PERSONNELS.md` §8.5, §10.4 PRS-40 à PRS-49 et PRS-54, §17 n° 354 ; Q-P28, R-P3 ;
-- D-022). Voir aussi `05_STRUCTURE.md` §9.7 (cours, migration 341).
--
-- Avant : un service d'enseignement était UNE ligne « professeur × cible × matière » (`prof_user_id`
-- NOT NULL). Impossible de dire « co-enseignement », « suppléant du 3 au 14 mars », « poste vacant »,
-- ni de corriger une erreur sans écraser l'historique ; le professeur ne pouvait être qu'un compte.
--
-- Après (l'objet « cours » est celui du socle 5, Q-P28 ; le service reste la ligne qui dit « qui
-- l'assure », avec ses heures, sa modalité, sa période et son état) :
--  * `ent_service_enseignement` enrichie : code, modalité, heures, période, état, source, service
--    remplacé ; `prof_user_id` devient NULLABLE (service vacant) et reste en PROJECTION du titulaire ;
--  * `ent_service_intervenant` : les personnes (agents) présentes sur un service, datées, par nature
--    (titulaire, co-enseignant, suppléant, assistant, intervenant, observateur). Jamais supprimée :
--    clôturée (`au`) ou annulée (`annule_le`), l'auteur réel de chaque saisie n'est jamais perdu ;
--  * fonctions de calcul : intervenants à une date, intervenants « en saisie » (une saisie par jour
--    sauf co-enseignement : le suppléant retire la saisie au titulaire PENDANT la période, par calcul,
--    sans écriture), services vacants ;
--  * reprise : chaque service existant reçoit une intervention `titulaire` (agent du compte).
--
-- Additive et rejouable (exécutée à chaque démarrage). RLS sur la table créée. Les écrivains
-- historiques (`INSERT INTO ent_service_enseignement (... prof_user_id ...)`) ne changent pas : un
-- déclencheur pose l'intervention du titulaire.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- 1. Le service : colonnes ajoutées --------------------------------------------------------------
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS code TEXT;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS modalite TEXT NOT NULL DEFAULT 'cours';
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS heures_hebdo NUMERIC(4, 2);
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS heures_annuelles NUMERIC(6, 2);
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS du DATE;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS au DATE;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'ouvert';
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS source TEXT;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS code_national TEXT;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS remplace_service_id UUID;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS etat_le TIMESTAMPTZ;
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS etat_par_user_id UUID;

-- Les services existants viennent de la reprise ; les suivants sont des saisies par défaut.
UPDATE ent_service_enseignement SET source = 'reprise' WHERE source IS NULL;
ALTER TABLE ent_service_enseignement ALTER COLUMN source SET DEFAULT 'saisie';
ALTER TABLE ent_service_enseignement ALTER COLUMN source SET NOT NULL;

-- Service vacant : plus de titulaire obligatoire (la colonne reste la projection du titulaire).
ALTER TABLE ent_service_enseignement ALTER COLUMN prof_user_id DROP NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_modalite_chk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_modalite_chk CHECK (modalite IN
            ('cours', 'tp', 'co_intervention', 'accompagnement', 'option', 'dispositif', 'autre'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_etat_chk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_etat_chk CHECK (etat IN
            ('prevu', 'ouvert', 'clos', 'annule'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_source_chk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_source_chk CHECK (source IN
            ('saisie', 'import_sts', 'import_edt', 'reconduction', 'reprise'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_heures_chk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_heures_chk CHECK (
            (heures_hebdo IS NULL OR (heures_hebdo >= 0 AND heures_hebdo <= 40))
            AND (heures_annuelles IS NULL OR (heures_annuelles >= 0 AND heures_annuelles <= 2000)));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_periode_chk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_periode_chk CHECK (au IS NULL OR du IS NULL OR au >= du);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_code_chk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_code_chk CHECK (
            code IS NULL OR char_length(btrim(code)) BETWEEN 1 AND 40);
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS ent_service_id_etab_uq ON ent_service_enseignement (id, etablissement_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_remplace_fk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_remplace_fk
            FOREIGN KEY (remplace_service_id, etablissement_id)
            REFERENCES ent_service_enseignement (id, etablissement_id);
    END IF;
END $$;

-- Unicité : l'ancien index (092) ne connaissait ni la modalité ni le code ni l'annulation. Même nom
-- (092 le recréerait sinon à chaque démarrage), nouvelle définition : un service annulé ne bloque
-- plus la recréation, deux modalités (cours / TP) de la même matière coexistent, un service vacant
-- (sans titulaire) compte comme un titulaire absent. Toute donnée valide hier l'est aujourd'hui.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'ent_service_unique_idx'
                  AND indexdef NOT LIKE '%modalite%') THEN
        DROP INDEX ent_service_unique_idx;
    END IF;
END $$;
CREATE UNIQUE INDEX IF NOT EXISTS ent_service_unique_idx
    ON ent_service_enseignement (
        etablissement_id, annee_id,
        (coalesce(prof_user_id, '00000000-0000-0000-0000-000000000000'::uuid)),
        (coalesce(classe_id, groupe_id)),
        (coalesce(matiere_id, '00000000-0000-0000-0000-000000000000'::uuid)),
        matiere_libelle, modalite, (coalesce(code, ''))
    ) WHERE etat <> 'annule';

-- Un service ne revient jamais en arrière : prévu → ouvert → clos ; prévu / ouvert → annulé.
CREATE OR REPLACE FUNCTION ent_trg_service_etat() RETURNS trigger AS $$
BEGIN
    IF NEW.etat IS DISTINCT FROM OLD.etat THEN
        IF NOT ((OLD.etat = 'prevu' AND NEW.etat IN ('ouvert', 'annule'))
             OR (OLD.etat = 'ouvert' AND NEW.etat IN ('clos', 'annule'))) THEN
            RAISE EXCEPTION 'service_etat_irreversible' USING ERRCODE = '23514';
        END IF;
        NEW.etat_le := now();
    END IF;
    IF OLD.etat IN ('clos', 'annule') AND NEW.etat = OLD.etat
       AND (NEW.annee_id, NEW.classe_id, NEW.groupe_id, NEW.matiere_id, NEW.modalite)
           IS DISTINCT FROM (OLD.annee_id, OLD.classe_id, OLD.groupe_id, OLD.matiere_id, OLD.modalite) THEN
        RAISE EXCEPTION 'service_clos_immuable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_etat ON ent_service_enseignement;
CREATE TRIGGER ent_service_etat BEFORE UPDATE OF etat, annee_id, classe_id, groupe_id, matiere_id, modalite
    ON ent_service_enseignement FOR EACH ROW EXECUTE FUNCTION ent_trg_service_etat();

-- 2. Les interventions ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_service_intervenant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    service_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN
        ('titulaire', 'co_enseignant', 'suppleant', 'assistant', 'intervenant', 'stagiaire_observateur')),
    du DATE NOT NULL,
    au DATE,
    -- Part des heures du service (co-enseignement).
    heures_hebdo NUMERIC(4, 2) CHECK (heures_hebdo IS NULL OR (heures_hebdo >= 0 AND heures_hebdo <= 40)),
    -- Saisit appel, notes, cahier, appréciations à la date. Seules trois natures peuvent saisir.
    saisie BOOLEAN NOT NULL DEFAULT false,
    -- Restriction facultative des droits de saisie : {appel, notes, cahier, appreciations}.
    droits TEXT[] CHECK (droits IS NULL OR droits <@ ARRAY['appel', 'notes', 'cahier', 'appreciations']::TEXT[]),
    origine TEXT NOT NULL DEFAULT 'saisie'
        CHECK (origine IN ('saisie', 'absence', 'vacance', 'import', 'reconduction', 'reprise')),
    -- Absence du socle 15, lot d'import : référence sans clé (les tables appartiennent à d'autres socles).
    origine_ref UUID,
    -- Une correction annule la ligne (jamais de suppression) et peut pointer sa remplaçante.
    remplace_intervention_id UUID,
    annule_le TIMESTAMPTZ,
    annule_par_user_id UUID,
    annule_motif TEXT CHECK (annule_motif IS NULL OR annule_motif IN ('erreur_saisie', 'doublon', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_interv_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_interv_saisie_chk CHECK (
        NOT saisie OR nature IN ('titulaire', 'co_enseignant', 'suppleant')),
    CONSTRAINT ent_interv_annulation_chk CHECK ((annule_le IS NULL) = (annule_motif IS NULL)),
    CONSTRAINT ent_interv_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_interv_service_fk FOREIGN KEY (service_id, etablissement_id)
        REFERENCES ent_service_enseignement (id, etablissement_id),
    CONSTRAINT ent_interv_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_interv_remplace_fk FOREIGN KEY (etablissement_id, remplace_intervention_id)
        REFERENCES ent_service_intervenant (etablissement_id, id),
    -- Un même agent n'a pas deux interventions actives qui se chevauchent sur le même service.
    CONSTRAINT ent_interv_agent_sans_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, service_id WITH =, agent_id WITH =,
        daterange(du, au, '[]') WITH &&
    ) WHERE (annule_le IS NULL),
    -- Un seul titulaire, un seul suppléant à la fois (une saisie par jour, hors co-enseignement).
    CONSTRAINT ent_interv_un_titulaire EXCLUDE USING gist (
        etablissement_id WITH =, service_id WITH =, daterange(du, au, '[]') WITH &&
    ) WHERE (nature = 'titulaire' AND annule_le IS NULL),
    CONSTRAINT ent_interv_un_suppleant EXCLUDE USING gist (
        etablissement_id WITH =, service_id WITH =, daterange(du, au, '[]') WITH &&
    ) WHERE (nature = 'suppleant' AND annule_le IS NULL)
);

CREATE INDEX IF NOT EXISTS ent_interv_service_idx ON ent_service_intervenant (etablissement_id, service_id, du);
CREATE INDEX IF NOT EXISTS ent_interv_agent_idx ON ent_service_intervenant (etablissement_id, agent_id, du)
    WHERE annule_le IS NULL;

-- Compte (user) du titulaire à une date : le titulaire en cours ; à défaut le dernier titulaire passé
-- (les lecteurs historiques gardent un professeur pour les services finis) ; un titulaire futur seul
-- n'est pas encore le professeur du service. NULL si l'agent n'a pas de compte, ou si le service est
-- seulement PRÉVU (répartition de l'année suivante : aucun effet sur les droits) ou ANNULÉ.
CREATE OR REPLACE FUNCTION ent_service_titulaire_user(p_etab UUID, p_service UUID, p_jour DATE)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT p.user_id
      FROM ent_service_intervenant i
      JOIN ent_agent a ON a.etablissement_id = i.etablissement_id AND a.id = i.agent_id
      JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
      JOIN ent_service_enseignement s ON s.etablissement_id = i.etablissement_id AND s.id = i.service_id
     WHERE i.etablissement_id = p_etab AND i.service_id = p_service
       AND i.nature = 'titulaire' AND i.annule_le IS NULL AND i.du <= p_jour
       AND s.etat IN ('ouvert', 'clos')
     ORDER BY (i.au IS NULL OR i.au >= p_jour) DESC, i.du DESC
     LIMIT 1
$$;

-- Garde-fous de l'intervention.
CREATE OR REPLACE FUNCTION ent_trg_service_intervenant() RETURNS trigger AS $$
DECLARE
    s ent_service_enseignement%ROWTYPE;
    a_fusion UUID;
    a_close TIMESTAMPTZ;
    reprise BOOLEAN := coalesce(current_setting('ent.service_reprise', true), '') = 'on';
    fusion BOOLEAN := coalesce(current_setting('ent.agent_fusion', true), '') = 'on';
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'intervention_non_supprimable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        -- Seules la clôture (`au`), la part d'heures et l'annulation se modifient : corriger une
        -- erreur = annuler la ligne et en créer une autre. Exception : la fusion de deux agents
        -- (lot P1, PRS-13) repointe `agent_id` — et rien d'autre.
        IF (NEW.etablissement_id, NEW.service_id, CASE WHEN fusion THEN OLD.agent_id ELSE NEW.agent_id END, NEW.nature, NEW.du, NEW.saisie, NEW.droits,
            NEW.origine, NEW.origine_ref, NEW.cree_le, NEW.cree_par_user_id, NEW.remplace_intervention_id)
           IS DISTINCT FROM
           (OLD.etablissement_id, OLD.service_id, OLD.agent_id, OLD.nature, OLD.du, OLD.saisie, OLD.droits,
            OLD.origine, OLD.origine_ref, OLD.cree_le, OLD.cree_par_user_id, OLD.remplace_intervention_id)
           OR (OLD.annule_le IS NOT NULL AND (NEW.annule_le, NEW.annule_motif, NEW.annule_par_user_id, NEW.au)
               IS DISTINCT FROM (OLD.annule_le, OLD.annule_motif, OLD.annule_par_user_id, OLD.au)) THEN
            RAISE EXCEPTION 'intervention_immuable' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    SELECT * INTO s FROM ent_service_enseignement WHERE id = NEW.service_id AND etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'intervention_incoherente: service' USING ERRCODE = '23514';
    END IF;
    IF s.etat IN ('clos', 'annule') AND NOT reprise THEN
        RAISE EXCEPTION 'intervention_service_clos' USING ERRCODE = '23514';
    END IF;
    SELECT fusionne_dans_agent_id INTO a_fusion FROM ent_agent
     WHERE id = NEW.agent_id AND etablissement_id = NEW.etablissement_id;
    IF a_fusion IS NOT NULL AND NOT reprise THEN
        RAISE EXCEPTION 'intervention_agent_fusionne' USING ERRCODE = '23514';
    END IF;
    SELECT y.cloturee_le INTO a_close FROM ent_annee_scolaire y
     WHERE y.id = s.annee_id AND y.etablissement_id = s.etablissement_id;
    IF a_close IS NOT NULL AND NOT reprise THEN
        RAISE EXCEPTION 'intervention_annee_cloturee' USING ERRCODE = '23514';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(NEW.service_id::text, 0));
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_intervenant_garde ON ent_service_intervenant;
CREATE TRIGGER ent_service_intervenant_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_service_intervenant
    FOR EACH ROW EXECUTE FUNCTION ent_trg_service_intervenant();

-- 3. Projection `prof_user_id` ↔ titulaire -------------------------------------------------------
-- Toute écriture d'une intervention `titulaire` recalcule la projection du service.
CREATE OR REPLACE FUNCTION ent_trg_service_intervenant_projection() RETURNS trigger AS $$
DECLARE v_etab UUID; v_service UUID; v_user UUID; v_courant UUID;
BEGIN
    v_etab := NEW.etablissement_id;
    v_service := NEW.service_id;
    IF NEW.nature <> 'titulaire' THEN
        IF TG_OP = 'INSERT' THEN
            RETURN NULL;
        END IF;
        IF OLD.nature <> 'titulaire' THEN
            RETURN NULL;
        END IF;
    END IF;
    v_user := ent_service_titulaire_user(v_etab, v_service, ent_aujourdhui_etab(v_etab));
    SELECT prof_user_id INTO v_courant FROM ent_service_enseignement WHERE id = v_service AND etablissement_id = v_etab;
    IF v_user IS DISTINCT FROM v_courant THEN
        PERFORM set_config('ent.service_projection', 'on', true);
        UPDATE ent_service_enseignement SET prof_user_id = v_user WHERE id = v_service AND etablissement_id = v_etab;
        PERFORM set_config('ent.service_projection', 'off', true);
    END IF;
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_intervenant_projection ON ent_service_intervenant;
CREATE TRIGGER ent_service_intervenant_projection AFTER INSERT OR UPDATE ON ent_service_intervenant
    FOR EACH ROW EXECUTE FUNCTION ent_trg_service_intervenant_projection();

-- Un service qui s'ouvre (prévu → ouvert) ou s'annule recalcule sa projection : un service prévu ne
-- donne aucun droit aux lecteurs historiques, un service annulé n'en donne plus.
CREATE OR REPLACE FUNCTION ent_trg_service_etat_projection() RETURNS trigger AS $$
DECLARE v_user UUID;
BEGIN
    v_user := ent_service_titulaire_user(NEW.etablissement_id, NEW.id, ent_aujourdhui_etab(NEW.etablissement_id));
    IF v_user IS DISTINCT FROM NEW.prof_user_id THEN
        PERFORM set_config('ent.service_projection', 'on', true);
        UPDATE ent_service_enseignement SET prof_user_id = v_user WHERE id = NEW.id AND etablissement_id = NEW.etablissement_id;
        PERFORM set_config('ent.service_projection', 'off', true);
    END IF;
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_etat_projection ON ent_service_enseignement;
CREATE TRIGGER ent_service_etat_projection AFTER UPDATE OF etat ON ent_service_enseignement
    FOR EACH ROW WHEN (NEW.etat IS DISTINCT FROM OLD.etat) EXECUTE FUNCTION ent_trg_service_etat_projection();

-- `prof_user_id` ne se modifie plus à la main : on change le titulaire (intervention).
CREATE OR REPLACE FUNCTION ent_trg_service_prof_garde() RETURNS trigger AS $$
BEGIN
    IF NEW.prof_user_id IS DISTINCT FROM OLD.prof_user_id
       AND coalesce(current_setting('ent.service_projection', true), '') <> 'on'
       AND coalesce(current_setting('ent.service_reprise', true), '') <> 'on' THEN
        RAISE EXCEPTION 'service_prof_projection: changer le titulaire par une intervention' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_prof_garde ON ent_service_enseignement;
CREATE TRIGGER ent_service_prof_garde BEFORE UPDATE OF prof_user_id ON ent_service_enseignement
    FOR EACH ROW EXECUTE FUNCTION ent_trg_service_prof_garde();

-- Un service créé par un écrivain historique (avec `prof_user_id`) reçoit son titulaire.
CREATE OR REPLACE FUNCTION ent_trg_service_titulaire_initial() RETURNS trigger AS $$
DECLARE v_agent UUID; v_debut DATE;
    reprise BOOLEAN := coalesce(current_setting('ent.service_reprise', true), '') = 'on';
BEGIN
    IF NEW.prof_user_id IS NULL THEN
        RETURN NULL;
    END IF;
    IF EXISTS (SELECT 1 FROM ent_service_intervenant i
                WHERE i.etablissement_id = NEW.etablissement_id AND i.service_id = NEW.id) THEN
        RETURN NULL;
    END IF;
    SELECT a.id INTO v_agent
      FROM ent_agent a
      JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
     WHERE a.etablissement_id = NEW.etablissement_id AND p.user_id = NEW.prof_user_id
       AND a.fusionne_dans_agent_id IS NULL;
    IF v_agent IS NULL THEN
        RETURN NULL; -- compte sans agent : service conservé tel quel (alerte de qualité côté lecture)
    END IF;
    SELECT coalesce(NEW.du, y.date_debut) INTO v_debut
      FROM ent_annee_scolaire y WHERE y.id = NEW.annee_id AND y.etablissement_id = NEW.etablissement_id;
    INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie, origine)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, v_agent, 'titulaire', v_debut, true,
            CASE WHEN reprise OR NEW.source = 'reprise' THEN 'reprise'
                 WHEN NEW.source = 'reconduction' THEN 'reconduction' ELSE 'saisie' END);
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_titulaire_initial ON ent_service_enseignement;
CREATE TRIGGER ent_service_titulaire_initial AFTER INSERT ON ent_service_enseignement
    FOR EACH ROW EXECUTE FUNCTION ent_trg_service_titulaire_initial();

-- Rafraîchit la projection d'un établissement (un titulaire qui prend ses fonctions ou qui part
-- change `prof_user_id` sans qu'aucune écriture n'ait lieu ce jour-là).
CREATE OR REPLACE FUNCTION ent_service_projection_rafraichir(p_etab UUID) RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE n INTEGER;
BEGIN
    PERFORM set_config('ent.service_projection', 'on', true);
    WITH calc AS (
        SELECT s.id, ent_service_titulaire_user(s.etablissement_id, s.id, ent_aujourdhui_etab(s.etablissement_id)) AS u
          FROM ent_service_enseignement s
         WHERE s.etablissement_id = p_etab
           AND EXISTS (SELECT 1 FROM ent_service_intervenant i
                        WHERE i.etablissement_id = s.etablissement_id AND i.service_id = s.id)
    ), maj AS (
        UPDATE ent_service_enseignement s SET prof_user_id = calc.u
          FROM calc WHERE s.id = calc.id AND s.etablissement_id = p_etab
           AND s.prof_user_id IS DISTINCT FROM calc.u
        RETURNING 1
    )
    SELECT count(*) INTO n FROM maj;
    PERFORM set_config('ent.service_projection', 'off', true);
    RETURN n;
END
$$;

-- 4. Reprise : un titulaire par service existant (rejouable : ne fait rien la deuxième fois) -----
DO $$
BEGIN
    PERFORM set_config('ent.service_reprise', 'on', true);
    INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie, origine)
    SELECT gen_random_uuid(), s.etablissement_id, s.id, a.id, 'titulaire', coalesce(s.du, y.date_debut), true, 'reprise'
      FROM ent_service_enseignement s
      JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
      JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.user_id = s.prof_user_id
      JOIN ent_agent a ON a.etablissement_id = p.etablissement_id AND a.personne_id = p.id
                      AND a.fusionne_dans_agent_id IS NULL
     WHERE s.prof_user_id IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM ent_service_intervenant i
                        WHERE i.etablissement_id = s.etablissement_id AND i.service_id = s.id)
    ON CONFLICT DO NOTHING;
    PERFORM set_config('ent.service_reprise', 'off', true);
END
$$;

-- 5. Calculs de date ---------------------------------------------------------------------------
-- Interventions actives à une date, avec la saisie EFFECTIVE : un suppléant actif retire la saisie au
-- titulaire (calcul, pas écriture) ; les co-enseignants gardent la leur.
CREATE OR REPLACE FUNCTION ent_service_intervenants_a(p_etab UUID, p_service UUID, p_jour DATE)
RETURNS TABLE (intervention_id UUID, agent_id UUID, nature TEXT, saisie_effective BOOLEAN, droits TEXT[])
LANGUAGE sql STABLE AS $$
    SELECT i.id, i.agent_id, i.nature,
           (i.saisie AND NOT (i.nature = 'titulaire' AND EXISTS (
                SELECT 1 FROM ent_service_intervenant sp
                 WHERE sp.etablissement_id = i.etablissement_id AND sp.service_id = i.service_id
                   AND sp.nature = 'suppleant' AND sp.annule_le IS NULL AND sp.saisie
                   AND sp.du <= p_jour AND (sp.au IS NULL OR sp.au >= p_jour)))),
           i.droits
      FROM ent_service_intervenant i
     WHERE i.etablissement_id = p_etab AND i.service_id = p_service AND i.annule_le IS NULL
       AND i.du <= p_jour AND (i.au IS NULL OR i.au >= p_jour)
$$;

-- Services OUVERTS à la date, dans leur période, sans aucune intervention en saisie.
CREATE OR REPLACE FUNCTION ent_services_vacants(p_etab UUID, p_jour DATE)
RETURNS TABLE (service_id UUID) LANGUAGE sql STABLE AS $$
    SELECT s.id
      FROM ent_service_enseignement s
      JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
     WHERE s.etablissement_id = p_etab AND s.etat = 'ouvert'
       AND y.date_debut <= p_jour AND y.date_fin >= p_jour
       AND coalesce(s.du, y.date_debut) <= p_jour AND coalesce(s.au, y.date_fin) >= p_jour
       AND NOT EXISTS (SELECT 1 FROM ent_service_intervenants_a(p_etab, s.id, p_jour) x WHERE x.saisie_effective)
$$;

-- 6. RLS (politique des migrations 100 et 215) ---------------------------------------------------
ALTER TABLE ent_service_intervenant ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_service_intervenant FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_service_intervenant;
CREATE POLICY ent_rls_etablissement ON ent_service_intervenant FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
