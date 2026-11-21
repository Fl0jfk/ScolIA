-- Socle 6 (personnels), lot P4, migration 355 : missions, périmètre des droits, bascule des suppléants
-- (`docs/socles/06_PERSONNELS.md` §8.6, §9, §10.5 ; D-017, D-022 ; décisions P3 appliquées par défaut, D-031).
--
-- 1. `ent_mission_type` : catalogue (fourni par ScolIA quand `etablissement_id` est NULL, sinon propre à
--    l'établissement) ; `ent_mission` : une personne, un type, des dates ; `ent_mission_perimetre` : la
--    ou les cibles de la mission (classe, niveau, site, groupe, élèves, agents, discipline).
-- 2. `ent_attribution_cible` : cibles DATÉES d'une attribution de rôle, lues par la portée `mon_perimetre`.
--    La fin d'une mission termine ses cibles par déclencheur (« fin de la mission = fin de l'attribution »).
-- 3. `ent_public_cours_jour` : le professeur d'un cours est désormais celui qui a une INTERVENTION active
--    (bascule du périmètre, écarts 1 à 3 de §24) ; un suppléant voit les élèves de son service ; un service
--    clos garde sa lecture 7 jours (délai de grâce) ; un service prévu ou annulé n'ouvre aucun droit.
--    Variante `p_saisie` : seules les saisies EFFECTIVES (le titulaire n'a plus la saisie pendant une suppléance).
-- 4. `ent_repli_texte_actif` : le repli « classe texte » de l'emploi du temps ne vit que tant qu'un créneau
--    n'est pas rattaché à un service (indicateur « créneaux sans service », lot P3).
--
-- Additive et rejouable. RLS : politique standard posée en ligne (la fonction `ent_rls_poser` n'existe
-- qu'à partir de 580) ; les tables sont aussi inscrites au registre `registre_donnees.rs`.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- 1. Types de missions -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_mission_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID
        CHECK (etablissement_id IS NULL OR etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    description TEXT CHECK (description IS NULL OR length(description) <= 500),
    perimetres_admis TEXT[] NOT NULL CHECK (
        cardinality(perimetres_admis) BETWEEN 1 AND 8
        AND perimetres_admis <@ ARRAY['etablissement', 'site', 'niveau', 'classe', 'groupe', 'eleves', 'agents', 'discipline']::TEXT[]),
    -- Nombre maximal de missions de ce type, simultanées, sur une même cible (PP : 2 permis). NULL = sans limite.
    cardinalite_max SMALLINT CHECK (cardinalite_max IS NULL OR cardinalite_max BETWEEN 1 AND 20),
    annuelle BOOLEAN NOT NULL DEFAULT false,
    -- Rôle appliqué au périmètre pendant la mission : id d'un rôle de l'établissement, OU code d'un
    -- modèle fourni (créé à la première utilisation dans l'établissement). Jamais les deux.
    modele_role_id UUID,
    modele_code TEXT CHECK (modele_code IS NULL OR modele_code IN ('professeur_principal', 'cpe_referent')),
    visible_familles BOOLEAN NOT NULL DEFAULT false,
    visible_eleves BOOLEAN NOT NULL DEFAULT false,
    regle_eligibilite TEXT NOT NULL DEFAULT 'aucune'
        CHECK (regle_eligibilite IN ('enseigne_dans_perimetre', 'fonction', 'aucune')),
    effet_regle TEXT NOT NULL DEFAULT 'avertir' CHECK (effet_regle IN ('avertir', 'bloquer')),
    -- Codes des fonctions qui rendent éligible (règle « fonction »).
    fonctions_eligibles TEXT[],
    -- 35 (accompagnement AESH), 39 (tuteur de stage) : seul ce socle attribue le type.
    proprietaire_socle SMALLINT CHECK (proprietaire_socle IS NULL OR proprietaire_socle BETWEEN 1 AND 99),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_mission_type_un_modele CHECK (modele_role_id IS NULL OR modele_code IS NULL),
    CONSTRAINT ent_mission_type_modele_etab CHECK (modele_role_id IS NULL OR etablissement_id IS NOT NULL),
    CONSTRAINT ent_mission_type_fourni_sans_role CHECK (etablissement_id IS NOT NULL OR modele_role_id IS NULL)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_mission_type_code_uq
    ON ent_mission_type (COALESCE(etablissement_id, '00000000-0000-0000-0000-000000000000'::uuid), code);
CREATE UNIQUE INDEX IF NOT EXISTS ent_mission_type_id_etab_uq
    ON ent_mission_type (id, etablissement_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_mission_type_role_fk') THEN
        ALTER TABLE ent_mission_type ADD CONSTRAINT ent_mission_type_role_fk
            FOREIGN KEY (modele_role_id) REFERENCES ent_role (id);
    END IF;
END $$;

-- Types fournis (libellés modifiables par copie, désactivables par établissement : pas de lecture
-- seule globale à contourner). `ON CONFLICT DO NOTHING` : rejouable.
INSERT INTO ent_mission_type (id, etablissement_id, code, libelle, perimetres_admis, cardinalite_max, annuelle,
                              modele_code, visible_familles, visible_eleves, regle_eligibilite, effet_regle,
                              fonctions_eligibles, proprietaire_socle)
VALUES
 (gen_random_uuid(), NULL, 'professeur_principal', 'Professeur principal', ARRAY['classe'], 2, true,
  'professeur_principal', true, true, 'enseigne_dans_perimetre', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'professeur_referent', 'Professeur référent', ARRAY['groupe', 'etablissement'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'cpe_referent', 'CPE référent', ARRAY['niveau', 'classe'], NULL, true,
  'cpe_referent', true, false, 'fonction', 'avertir', ARRAY['cpe'], NULL),
 (gen_random_uuid(), NULL, 'coordinateur_discipline', 'Coordinateur de discipline', ARRAY['discipline'], 2, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'coordinateur_niveau', 'Coordinateur de niveau ou de cycle', ARRAY['niveau'], 2, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_decrochage', 'Référent décrochage', ARRAY['etablissement', 'niveau'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_harcelement', 'Référent harcèlement', ARRAY['etablissement', 'site'], NULL, false,
  NULL, true, true, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_numerique', 'Référent numérique', ARRAY['etablissement', 'site'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_culture', 'Référent culture', ARRAY['etablissement'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_laicite', 'Référent laïcité et citoyenneté', ARRAY['etablissement'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_egalite', 'Référent égalité', ARRAY['etablissement'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_handicap', 'Référent handicap et inclusion', ARRAY['etablissement'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_orientation', 'Référent orientation', ARRAY['etablissement', 'niveau'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'referent_vie_lyceenne', 'Référent vie lycéenne', ARRAY['etablissement'], NULL, false,
  NULL, false, true, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'tuteur', 'Tuteur d''un agent', ARRAY['agents'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'responsable_laboratoire', 'Responsable de laboratoire ou de salle spécialisée', ARRAY['etablissement', 'site'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'responsable_association_sportive', 'Responsable de l''association sportive', ARRAY['etablissement'], 1, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'chef_de_travaux', 'Chef de travaux (DDFPT)', ARRAY['niveau', 'etablissement'], 1, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'surveillance_internat', 'Surveillance d''internat', ARRAY['site'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, NULL),
 (gen_random_uuid(), NULL, 'accompagnement_aesh', 'Accompagnement d''élèves (AESH)', ARRAY['eleves'], NULL, true,
  NULL, false, false, 'aucune', 'avertir', NULL, 35),
 (gen_random_uuid(), NULL, 'tuteur_de_stage', 'Tuteur de stage', ARRAY['eleves'], NULL, false,
  NULL, false, false, 'aucune', 'avertir', NULL, 39)
ON CONFLICT DO NOTHING;

-- 2. Missions -----------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_mission (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type_id UUID NOT NULL REFERENCES ent_mission_type (id),
    agent_id UUID NOT NULL,
    annee_id UUID,
    du DATE NOT NULL,
    au DATE,
    origine TEXT NOT NULL DEFAULT 'saisie'
        CHECK (origine IN ('saisie', 'delegation', 'import', 'reconduction', 'transfert', 'socle_tiers')),
    origine_ref UUID,
    -- Lettre de mission (socle 7), facultative : référence sans clé (la table appartient à un autre socle).
    lettre_document_id UUID,
    annule_le TIMESTAMPTZ,
    annule_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_mission_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_mission_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_mission_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_mission_agent_idx ON ent_mission (etablissement_id, agent_id, du);
CREATE INDEX IF NOT EXISTS ent_mission_type_idx ON ent_mission (etablissement_id, type_id) WHERE annule_le IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_mission_annee_fk') THEN
        ALTER TABLE ent_mission ADD CONSTRAINT ent_mission_annee_fk FOREIGN KEY (annee_id, etablissement_id)
            REFERENCES ent_annee_scolaire (id, etablissement_id);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS ent_mission_perimetre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    mission_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN
        ('etablissement', 'site', 'niveau', 'classe', 'groupe', 'eleves', 'agents', 'discipline')),
    cible_id UUID,
    discipline_code TEXT CHECK (discipline_code IS NULL OR discipline_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_mission_perimetre_mission_fk FOREIGN KEY (etablissement_id, mission_id)
        REFERENCES ent_mission (etablissement_id, id),
    CONSTRAINT ent_mission_perimetre_forme_chk CHECK (
        (nature = 'etablissement' AND cible_id IS NULL AND discipline_code IS NULL)
        OR (nature = 'discipline' AND cible_id IS NULL AND discipline_code IS NOT NULL)
        OR (nature NOT IN ('etablissement', 'discipline') AND cible_id IS NOT NULL AND discipline_code IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_mission_perimetre_mission_idx ON ent_mission_perimetre (etablissement_id, mission_id);
CREATE INDEX IF NOT EXISTS ent_mission_perimetre_cible_idx
    ON ent_mission_perimetre (etablissement_id, nature, cible_id) WHERE cible_id IS NOT NULL;

-- Cibles datées d'une attribution de rôle (portée `mon_perimetre`). `mission_id` NULL = attribution manuelle.
CREATE TABLE IF NOT EXISTS ent_attribution_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    attribution_id UUID NOT NULL REFERENCES ent_role_attribution (id) ON DELETE CASCADE,
    mission_id UUID,
    nature TEXT NOT NULL CHECK (nature IN ('site', 'niveau', 'classe', 'groupe', 'eleves', 'agents')),
    cible_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE,
    annule_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_attribution_cible_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_attribution_cible_mission_fk FOREIGN KEY (etablissement_id, mission_id)
        REFERENCES ent_mission (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_attribution_cible_attr_idx ON ent_attribution_cible (attribution_id) WHERE annule_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_attribution_cible_mission_idx ON ent_attribution_cible (etablissement_id, mission_id);

-- La source `mission` s'ajoute aux sources d'attribution (351).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conname = 'ent_role_attribution_source_chk'
                  AND pg_get_constraintdef(oid) NOT LIKE '%mission%') THEN
        ALTER TABLE ent_role_attribution DROP CONSTRAINT ent_role_attribution_source_chk;
        ALTER TABLE ent_role_attribution ADD CONSTRAINT ent_role_attribution_source_chk
            CHECK (source IN ('manuelle', 'modification', 'systeme', 'mission'));
    END IF;
END $$;

-- 3. Gardes ------------------------------------------------------------------------------------------
-- Cible d'un périmètre : elle existe, dans CE établissement. Un périmètre ne se modifie ni ne s'efface :
-- on termine la mission et on en pose une autre (historique).
CREATE OR REPLACE FUNCTION ent_trg_mission_perimetre() RETURNS trigger AS $$
DECLARE
    ok BOOLEAN := true;
    m_type UUID;
    m_au DATE;
    m_du DATE;
    m_annule TIMESTAMPTZ;
    t_card SMALLINT;
    t_admis TEXT[];
    t_etab UUID;
    n INTEGER;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'mission_perimetre_immuable' USING ERRCODE = '23514';
    END IF;
    SELECT m.type_id, m.du, m.au, m.annule_le INTO m_type, m_du, m_au, m_annule
      FROM ent_mission m WHERE m.etablissement_id = NEW.etablissement_id AND m.id = NEW.mission_id;
    SELECT t.cardinalite_max, t.perimetres_admis, t.etablissement_id INTO t_card, t_admis, t_etab
      FROM ent_mission_type t WHERE t.id = m_type;
    IF t_etab IS NOT NULL AND t_etab <> NEW.etablissement_id THEN
        RAISE EXCEPTION 'mission_type_autre_etablissement' USING ERRCODE = '23514';
    END IF;
    IF NOT (NEW.nature = ANY (t_admis)) THEN
        RAISE EXCEPTION 'mission_perimetre_non_admis' USING ERRCODE = '23514';
    END IF;
    IF NEW.cible_id IS NOT NULL THEN
        ok := CASE NEW.nature
            WHEN 'classe' THEN EXISTS (SELECT 1 FROM ent_classe x WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.cible_id)
            WHEN 'groupe' THEN EXISTS (SELECT 1 FROM ent_groupe x WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.cible_id)
            WHEN 'site' THEN EXISTS (SELECT 1 FROM ent_site x WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.cible_id)
            WHEN 'niveau' THEN EXISTS (SELECT 1 FROM ent_niveau x WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.cible_id)
            WHEN 'eleves' THEN EXISTS (SELECT 1 FROM ent_eleve x WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.cible_id)
            WHEN 'agents' THEN EXISTS (SELECT 1 FROM ent_agent x WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.cible_id)
            ELSE false END;
        IF NOT ok THEN
            RAISE EXCEPTION 'mission_perimetre_cible_inconnue' USING ERRCODE = '23503';
        END IF;
    END IF;
    -- Cardinalité : pas plus de `cardinalite_max` missions simultanées du même type sur la même cible.
    IF t_card IS NOT NULL AND m_annule IS NULL THEN
        PERFORM pg_advisory_xact_lock(hashtextextended(
            m_type::text || ':' || NEW.nature || ':' || coalesce(NEW.cible_id::text, NEW.discipline_code, ''), 0));
        SELECT count(*) INTO n
          FROM ent_mission_perimetre p
          JOIN ent_mission o ON o.etablissement_id = p.etablissement_id AND o.id = p.mission_id
         WHERE p.etablissement_id = NEW.etablissement_id AND p.nature = NEW.nature
           AND p.cible_id IS NOT DISTINCT FROM NEW.cible_id
           AND p.discipline_code IS NOT DISTINCT FROM NEW.discipline_code
           AND o.type_id = m_type AND o.annule_le IS NULL AND o.id <> NEW.mission_id
           AND o.du <= coalesce(m_au, 'infinity'::date) AND coalesce(o.au, 'infinity'::date) >= m_du;
        IF n >= t_card THEN
            RAISE EXCEPTION 'mission_cardinalite' USING ERRCODE = '23505';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_mission_perimetre_garde ON ent_mission_perimetre;
CREATE TRIGGER ent_mission_perimetre_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_mission_perimetre
    FOR EACH ROW EXECUTE FUNCTION ent_trg_mission_perimetre();

-- Une mission ne change ni de type, ni de personne, ni de début : on la termine, on en pose une autre.
-- Une mission ne se supprime pas. Sa fin (ou son annulation) termine ses cibles d'attribution.
CREATE OR REPLACE FUNCTION ent_trg_mission_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'mission_non_supprimable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NOT EXISTS (SELECT 1 FROM ent_mission_type t
                        WHERE t.id = NEW.type_id AND t.actif
                          AND (t.etablissement_id IS NULL OR t.etablissement_id = NEW.etablissement_id)) THEN
            RAISE EXCEPTION 'mission_type_inconnu' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.type_id <> OLD.type_id OR NEW.agent_id <> OLD.agent_id OR NEW.du <> OLD.du
       OR NEW.etablissement_id <> OLD.etablissement_id THEN
        RAISE EXCEPTION 'mission_immuable' USING ERRCODE = '23514';
    END IF;
    IF OLD.annule_le IS NOT NULL THEN
        RAISE EXCEPTION 'mission_annulee_immuable' USING ERRCODE = '23514';
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_mission_garde ON ent_mission;
CREATE TRIGGER ent_mission_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_mission
    FOR EACH ROW EXECUTE FUNCTION ent_trg_mission_garde();

CREATE OR REPLACE FUNCTION ent_trg_mission_fin() RETURNS trigger AS $$
BEGIN
    UPDATE ent_attribution_cible c
       SET au = NEW.au, annule_le = NEW.annule_le
     WHERE c.etablissement_id = NEW.etablissement_id AND c.mission_id = NEW.id;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_mission_fin ON ent_mission;
CREATE TRIGGER ent_mission_fin AFTER UPDATE OF au, annule_le ON ent_mission
    FOR EACH ROW EXECUTE FUNCTION ent_trg_mission_fin();

-- 4. Le périmètre d'enseignement lit les INTERVENTIONS ----------------------------------------------------
-- Avant : `s.prof_user_id = p_prof` (projection du titulaire). Après : une intervention active à la date
-- (titulaire, co-enseignant, suppléant, intervenant, assistant), service ouvert ou clos, hors annulation.
--  * délai de grâce de 7 jours en LECTURE après la fin d'une intervention (§9.4), jamais en saisie ;
--  * `p_saisie` : seules les saisies effectives ouvrent le droit (le titulaire n'a plus la saisie pendant
--    une suppléance ; assistants et intervenants ne saisissent jamais) ;
--  * le stagiaire observateur n'ouvre aucun droit sur les élèves ;
--  * repli : un service sans aucune intervention (compte sans agent) garde son `prof_user_id`.
DROP FUNCTION IF EXISTS ent_public_cours_jour(UUID, DATE, UUID, UUID, UUID);

CREATE OR REPLACE FUNCTION ent_prof_sur_cours(
    p_etab UUID, p_cours UUID, p_user UUID, p_jour DATE, p_saisie BOOLEAN)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ent_service_enseignement s
         WHERE s.etablissement_id = p_etab AND s.cours_id = p_cours
           AND s.etat IN ('ouvert', 'clos')
           AND (
                CASE WHEN p_saisie THEN
                    EXISTS (
                        SELECT 1 FROM ent_service_intervenants_a(p_etab, s.id, p_jour) x
                          JOIN ent_agent ag ON ag.etablissement_id = p_etab AND ag.id = x.agent_id
                          JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                         WHERE pe.user_id = p_user AND x.saisie_effective)
                ELSE
                    EXISTS (
                        SELECT 1 FROM ent_service_intervenant i
                          JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
                          JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                         WHERE i.etablissement_id = p_etab AND i.service_id = s.id AND pe.user_id = p_user
                           AND i.annule_le IS NULL
                           AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant', 'intervenant', 'assistant')
                           AND i.du <= p_jour AND (i.au IS NULL OR i.au + 7 >= p_jour))
                END
                OR (s.prof_user_id = p_user
                    AND NOT EXISTS (SELECT 1 FROM ent_service_intervenant z
                                     WHERE z.etablissement_id = s.etablissement_id AND z.service_id = s.id))))
$$;

CREATE OR REPLACE FUNCTION ent_public_cours_jour(
    p_etab UUID, p_jour DATE,
    p_cours UUID DEFAULT NULL, p_prof UUID DEFAULT NULL, p_eleve UUID DEFAULT NULL,
    p_saisie BOOLEAN DEFAULT false)
RETURNS TABLE (cours_id UUID, eleve_id UUID) LANGUAGE sql STABLE AS $$
    SELECT DISTINCT x.cours_id, x.eleve_id
      FROM (
        SELECT c.id AS cours_id, a.eleve_id
          FROM ent_cours c
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          JOIN ent_affectation a
            ON a.etablissement_id = c.etablissement_id AND a.annee_id = c.annee_id
           AND ((c.classe_id IS NOT NULL AND a.classe_id = c.classe_id)
             OR (c.groupe_id IS NOT NULL AND a.groupe_id = c.groupe_id))
         WHERE c.etablissement_id = p_etab
           AND (p_cours IS NULL OR c.id = p_cours)
           AND (p_prof IS NULL OR ent_prof_sur_cours(p_etab, c.id, p_prof, p_jour, p_saisie))
           AND (p_eleve IS NULL OR a.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
           AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
        UNION
        SELECT c.id, m.eleve_id
          FROM ent_cours_membre m
          JOIN ent_cours c ON c.id = m.cours_id AND c.etablissement_id = m.etablissement_id
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
         WHERE m.etablissement_id = p_etab AND m.nature = 'inclusion' AND m.annulee_le IS NULL
           AND (p_cours IS NULL OR c.id = p_cours)
           AND (p_prof IS NULL OR ent_prof_sur_cours(p_etab, c.id, p_prof, p_jour, p_saisie))
           AND (p_eleve IS NULL OR m.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
           AND m.du <= p_jour AND (m.au IS NULL OR m.au >= p_jour)
      ) x
      JOIN ent_eleve e ON e.id = x.eleve_id AND e.etablissement_id = p_etab
     WHERE (e.date_sortie IS NULL OR e.date_sortie > p_jour)
       AND NOT EXISTS (
            SELECT 1 FROM ent_cours_membre ex
             WHERE ex.etablissement_id = p_etab AND ex.cours_id = x.cours_id AND ex.eleve_id = x.eleve_id
               AND ex.nature = 'exclusion' AND ex.annulee_le IS NULL
               AND ex.du <= p_jour AND (ex.au IS NULL OR ex.au >= p_jour))
$$;

-- Élèves qu'un professeur voit en LECTURE seulement (intervention sans saisie effective) : à retirer
-- des droits de saisie (`notes.saisir`, `cahier.ecrire`…) quand la portée est « mes élèves par l'enseignement ».
CREATE OR REPLACE FUNCTION ent_eleves_lecture_seule(p_etab UUID, p_user UUID, p_jour DATE)
RETURNS TABLE (eleve_id UUID) LANGUAGE sql STABLE AS $$
    SELECT l.eleve_id
      FROM ent_public_cours_jour(p_etab, p_jour, NULL, p_user, NULL, false) l
     GROUP BY l.eleve_id
    HAVING NOT EXISTS (
        SELECT 1 FROM ent_public_cours_jour(p_etab, p_jour, NULL, p_user, l.eleve_id, true) s)
$$;

-- 5. Repli « classe texte » ----------------------------------------------------------------------------------
-- Actif tant qu'au moins un créneau de l'établissement n'est rattaché à aucun service.
CREATE OR REPLACE FUNCTION ent_repli_texte_actif(p_etab UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT EXISTS (SELECT 1 FROM ent_edt_creneau c WHERE c.etablissement_id = p_etab AND c.service_id IS NULL)
$$;

-- 6. RLS (politique des migrations 100 et 215) ------------------------------------------------------------
ALTER TABLE ent_mission_type ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_mission_type FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_mission_type;
CREATE POLICY ent_rls_etablissement ON ent_mission_type FOR ALL
    USING (etablissement_id IS NULL
           OR etablissement_id = (SELECT ent_rls_etablissement())
           OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_mission', 'ent_mission_perimetre', 'ent_attribution_cible'] LOOP
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
END $$;
