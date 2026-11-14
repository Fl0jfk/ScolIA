-- 2307 — Socle 15 (absences des personnels), lot A6 : remplacement de courte durée (RCD)
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §12, §26 ; R-01, R-02, R-03, R-04 ; ABP-220 à 231).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée (E13).
--
-- 1. `ent_absp_rcd_plan` : le plan annuel, VERSIONNÉ (une ligne par version ; une seule version vivante par année ;
--    le contenu d'une version présentée ou transmise ne change plus, on en crée une nouvelle).
-- 2. `ent_absp_rcd_compte_rendu` : comptes rendus (au moins deux par an), en ajout seul, produits depuis les remontées.
-- 3. `ent_absp_rcd_engagement` : l'engagement d'un volontaire (volume, période, statut Pacte / HSE / autre, R-04).
-- 4. `ent_absp_rcd_remontee` et `_ligne` : la remontée hebdomadaire. AUCUN identifiant de personne, aucune nature
--    fine : seulement discipline, niveau, catégorie nationale du motif, mode de continuité (R-02, §12.3).
-- 5. Le journal du domaine reçoit les objets du plan, des engagements et des remontées (et ceux des heures, A7).

-- 0. Journal : nouveaux objets (la contrainte d'origine est nommée par PostgreSQL) ----------------------------
ALTER TABLE ent_absp_journal DROP CONSTRAINT IF EXISTS ent_absp_journal_objet_check;
ALTER TABLE ent_absp_journal ADD CONSTRAINT ent_absp_journal_objet_check CHECK (objet IN (
    'absence', 'circuit_regle', 'param', 'nature_etab',
    'rcd_plan', 'rcd_engagement', 'rcd_remontee', 'heure', 'etat', 'element_paie', 'dsn'));

-- 1. Plan annuel versionné -----------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_rcd_plan (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    version INTEGER NOT NULL CHECK (version >= 1),
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'presente', 'transmis', 'archive')),
    -- Contenu (figé dès que la version n'est plus un brouillon).
    objectifs VARCHAR(1000),
    -- Délai de prévenance d'un volontaire (S-1507, R-AP36) ; 0 = pas de délai.
    delai_prevenance_heures SMALLINT NOT NULL DEFAULT 24 CHECK (delai_prevenance_heures BETWEEN 0 AND 168),
    continuite_numerique BOOLEAN NOT NULL DEFAULT false,
    referent_agent_id UUID,
    -- Faits de procédure (R-01, R-03) : avis du conseil pédagogique, présentation au CA (public) ou à l'instance
    -- de l'établissement (privé, Q-AP7), transmission au recteur.
    avis_cp_le DATE,
    presente_le DATE,
    instance TEXT CHECK (instance IS NULL OR instance IN ('ca', 'instance_etablissement')),
    transmis_le DATE,
    -- Document généré depuis la trame et déposé au socle 7 (référence souple : le socle 7 n'est pas dans ce lot).
    document_id UUID,
    campagne_lancee_le TIMESTAMPTZ,
    cree_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rcd_plan_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rcd_plan_version_uq UNIQUE (etablissement_id, annee_id, version),
    CONSTRAINT ent_absp_rcd_plan_referent_fk FOREIGN KEY (etablissement_id, referent_agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_rcd_plan_presente_chk CHECK (
        etat NOT IN ('presente', 'transmis') OR (avis_cp_le IS NOT NULL AND presente_le IS NOT NULL AND instance IS NOT NULL)),
    CONSTRAINT ent_absp_rcd_plan_transmis_chk CHECK (etat <> 'transmis' OR transmis_le IS NOT NULL)
);
-- Une seule version vivante (non archivée) par année.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_rcd_plan_vivante_uq
    ON ent_absp_rcd_plan (etablissement_id, annee_id) WHERE etat <> 'archive';

CREATE OR REPLACE FUNCTION ent_trg_absp_rcd_plan_garde() RETURNS trigger AS $$
DECLARE
    rang_ancien INT;
    rang_nouveau INT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_rcd_plan : une version du plan ne se supprime pas';
    END IF;
    IF OLD.etat = 'archive' THEN
        RAISE EXCEPTION 'ent_absp_rcd_plan : une version archivée ne change plus';
    END IF;
    rang_ancien := CASE OLD.etat WHEN 'brouillon' THEN 0 WHEN 'presente' THEN 1 WHEN 'transmis' THEN 2 END;
    rang_nouveau := CASE NEW.etat WHEN 'brouillon' THEN 0 WHEN 'presente' THEN 1 WHEN 'transmis' THEN 2 ELSE 3 END;
    IF rang_nouveau < rang_ancien THEN
        RAISE EXCEPTION 'ent_absp_rcd_plan : l''état d''une version ne recule pas';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.annee_id <> OLD.annee_id
       OR NEW.version <> OLD.version THEN
        RAISE EXCEPTION 'ent_absp_rcd_plan : identité de la version immuable';
    END IF;
    -- Le contenu ne change que tant que la version est un brouillon.
    IF OLD.etat <> 'brouillon' AND (
        NEW.objectifs IS DISTINCT FROM OLD.objectifs
        OR NEW.delai_prevenance_heures <> OLD.delai_prevenance_heures
        OR NEW.continuite_numerique <> OLD.continuite_numerique
        OR NEW.referent_agent_id IS DISTINCT FROM OLD.referent_agent_id) THEN
        RAISE EXCEPTION 'ent_absp_rcd_plan : le contenu d''une version présentée ou transmise ne change plus (nouvelle version)';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_rcd_plan_garde ON ent_absp_rcd_plan;
CREATE TRIGGER ent_absp_rcd_plan_garde BEFORE UPDATE OR DELETE ON ent_absp_rcd_plan
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rcd_plan_garde();

-- 2. Comptes rendus (ajout seul) ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_rcd_compte_rendu (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    plan_id UUID NOT NULL,
    le DATE NOT NULL,
    periode_du DATE NOT NULL,
    periode_au DATE NOT NULL,
    -- Indicateurs agrégés issus des remontées de la période : jamais nominatifs (R-AP38).
    heures_prevues NUMERIC(9,1) NOT NULL CHECK (heures_prevues >= 0),
    heures_absence NUMERIC(9,1) NOT NULL CHECK (heures_absence >= 0),
    heures_remplacees NUMERIC(9,1) NOT NULL CHECK (heures_remplacees >= 0),
    remontees INTEGER NOT NULL CHECK (remontees >= 0),
    document_id UUID,
    cree_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rcd_compte_rendu_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rcd_compte_rendu_periode_chk CHECK (periode_au >= periode_du),
    CONSTRAINT ent_absp_rcd_compte_rendu_plan_fk FOREIGN KEY (etablissement_id, plan_id)
        REFERENCES ent_absp_rcd_plan (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_rcd_compte_rendu_plan_idx ON ent_absp_rcd_compte_rendu (etablissement_id, plan_id, le);

CREATE OR REPLACE FUNCTION ent_trg_absp_rcd_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_rcd_compte_rendu_ajout_seul ON ent_absp_rcd_compte_rendu;
CREATE TRIGGER ent_absp_rcd_compte_rendu_ajout_seul BEFORE UPDATE OR DELETE ON ent_absp_rcd_compte_rendu
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rcd_ajout_seul();

-- 3. Engagement des volontaires --------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_rcd_engagement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    volume_heures NUMERIC(5,1) NOT NULL CHECK (volume_heures > 0 AND volume_heures <= 999),
    statut TEXT NOT NULL CHECK (statut IN ('pacte', 'hse', 'autre')),
    du DATE NOT NULL,
    au DATE NOT NULL,
    etat TEXT NOT NULL DEFAULT 'actif' CHECK (etat IN ('actif', 'termine')),
    cree_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rcd_engagement_periode_chk CHECK (au >= du),
    CONSTRAINT ent_absp_rcd_engagement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rcd_engagement_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
-- Un seul engagement actif par agent et par année (une modification est une modification, pas un doublon).
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_rcd_engagement_actif_uq
    ON ent_absp_rcd_engagement (etablissement_id, agent_id, annee_id) WHERE etat = 'actif';
CREATE INDEX IF NOT EXISTS ent_absp_rcd_engagement_annee_idx ON ent_absp_rcd_engagement (etablissement_id, annee_id, etat);

-- 4. Remontée hebdomadaire (sans nom) --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_rcd_remontee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Lundi de la semaine remontée.
    semaine DATE NOT NULL CHECK (EXTRACT(ISODOW FROM semaine) = 1),
    etat TEXT NOT NULL DEFAULT 'preparee' CHECK (etat IN ('preparee', 'controlee', 'transmise', 'rejetee')),
    preparee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    controlee_par_user_id UUID,
    controlee_le TIMESTAMPTZ,
    transmise_par_user_id UUID,
    transmise_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rcd_remontee_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rcd_remontee_semaine_uq UNIQUE (etablissement_id, semaine),
    CONSTRAINT ent_absp_rcd_remontee_controle_chk CHECK (
        etat NOT IN ('controlee', 'transmise') OR (controlee_par_user_id IS NOT NULL AND controlee_le IS NOT NULL)),
    CONSTRAINT ent_absp_rcd_remontee_transmise_chk CHECK (
        (etat = 'transmise') = (transmise_le IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS ent_absp_rcd_remontee_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    remontee_id UUID NOT NULL,
    discipline_code VARCHAR(40) NOT NULL,
    niveau_code VARCHAR(20) NOT NULL,
    categorie_nationale TEXT NOT NULL CHECK (categorie_nationale IN (
        'raison_sante', 'evenement_familial', 'formation', 'institutionnel',
        'convenance_personnelle', 'en_attente_de_motif')),
    mode_continuite TEXT NOT NULL CHECK (mode_continuite IN (
        'remplacement', 'autre_cours', 'deplacement', 'continuite', 'annulation', 'suppleance', 'aucune')),
    heures_prevues NUMERIC(7,1) NOT NULL CHECK (heures_prevues >= 0),
    heures_absence NUMERIC(7,1) NOT NULL CHECK (heures_absence >= 0),
    heures_remplacees NUMERIC(7,1) NOT NULL CHECK (heures_remplacees >= 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_rcd_remontee_ligne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_rcd_remontee_ligne_cle_uq
        UNIQUE (etablissement_id, remontee_id, discipline_code, niveau_code, categorie_nationale, mode_continuite),
    CONSTRAINT ent_absp_rcd_remontee_ligne_remontee_fk FOREIGN KEY (etablissement_id, remontee_id)
        REFERENCES ent_absp_rcd_remontee (etablissement_id, id)
);

ALTER TABLE ent_absp_rcd_remontee_ligne ADD COLUMN IF NOT EXISTS cree_le TIMESTAMPTZ NOT NULL DEFAULT now();

-- Une remontée transmise ne change plus (ni ses lignes), et ne se supprime pas.
CREATE OR REPLACE FUNCTION ent_trg_absp_rcd_remontee_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_rcd_remontee : une remontée ne se supprime pas';
    END IF;
    IF OLD.etat = 'transmise' THEN
        RAISE EXCEPTION 'ent_absp_rcd_remontee : une remontée transmise ne change plus';
    END IF;
    IF NEW.semaine <> OLD.semaine THEN
        RAISE EXCEPTION 'ent_absp_rcd_remontee : la semaine ne change pas';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_rcd_remontee_garde ON ent_absp_rcd_remontee;
CREATE TRIGGER ent_absp_rcd_remontee_garde BEFORE UPDATE OR DELETE ON ent_absp_rcd_remontee
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rcd_remontee_garde();

CREATE OR REPLACE FUNCTION ent_trg_absp_rcd_remontee_ligne_garde() RETURNS trigger AS $$
DECLARE
    rid UUID;
    eid UUID;
    etat_parent TEXT;
BEGIN
    IF TG_OP = 'DELETE' THEN rid := OLD.remontee_id; eid := OLD.etablissement_id;
    ELSE rid := NEW.remontee_id; eid := NEW.etablissement_id; END IF;
    SELECT etat INTO etat_parent FROM ent_absp_rcd_remontee WHERE etablissement_id = eid AND id = rid;
    IF etat_parent = 'transmise' THEN
        RAISE EXCEPTION 'ent_absp_rcd_remontee_ligne : les lignes d''une remontée transmise ne changent plus';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_rcd_remontee_ligne_garde ON ent_absp_rcd_remontee_ligne;
CREATE TRIGGER ent_absp_rcd_remontee_ligne_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_absp_rcd_remontee_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_rcd_remontee_ligne_garde();

SELECT ent_rls_poser(ARRAY[
    'ent_absp_rcd_plan', 'ent_absp_rcd_compte_rendu', 'ent_absp_rcd_engagement',
    'ent_absp_rcd_remontee', 'ent_absp_rcd_remontee_ligne']);
