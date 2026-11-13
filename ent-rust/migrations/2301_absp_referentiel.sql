-- 2301 — Socle 15 (absences des personnels), lot A1 : référentiel des natures, paramètres et règles de
-- circuit (`docs/socles/15_ABSENCES_PERSONNELS.md` §9, §26, §28.2).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; aucun BYTEA (E5) ; RLS posée.
--
-- La NATURE d'une absence est un fait confidentiel (R-AP2) : `ent_absp_nature` ne contient que le
-- référentiel (aucune donnée de personne) ; le lien nature ↔ absence vit dans `ent_absp_absence`
-- (migration 2302), seule table marquée « santé ».

-- 1. Natures ----------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_nature (
    id UUID PRIMARY KEY,
    -- NULL = nature FOURNIE par ScolIA (référentiel commun, jamais modifiée par un établissement) ;
    -- NOT NULL = nature PROPRE à l'établissement (non sensible, §9.1).
    etablissement_id UUID
        CHECK (etablissement_id IS NULL OR etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    libelle_agent TEXT NOT NULL CHECK (length(btrim(libelle_agent)) BETWEEN 1 AND 160),
    famille TEXT NOT NULL CHECK (famille IN (
        'sante', 'parentalite', 'famille', 'autorisation', 'service', 'representation',
        'greve', 'conges', 'autre')),
    -- R-02 : catégorie nationale remontée (agrégée, sans nom) ; jamais modifiable par l'établissement.
    categorie_nationale TEXT CHECK (categorie_nationale IS NULL OR categorie_nationale IN (
        'raison_sante', 'evenement_familial', 'formation', 'institutionnel',
        'convenance_personnelle', 'en_attente_de_motif')),
    employeur TEXT NOT NULL DEFAULT 'tous' CHECK (employeur IN ('etat', 'ogec', 'tous')),
    -- Une nature sensible n'apparaît jamais en clair hors du cercle d'instruction (R-AP2).
    sensible BOOLEAN NOT NULL,
    -- R-AP12 : une nature non discrétionnaire ne se refuse pas.
    refus_possible BOOLEAN NOT NULL,
    piece_attendue TEXT NOT NULL DEFAULT 'aucune'
        CHECK (piece_attendue IN ('aucune', 'arret', 'justificatif', 'convocation')),
    delai_piece_heures SMALLINT CHECK (delai_piece_heures IS NULL OR delai_piece_heures BETWEEN 1 AND 720),
    -- Traitement par défaut selon l'employeur (§9.3) ; `traitement_impose` : la nature le fixe, le
    -- valideur constate (NX15-19, R-AP12).
    traitement_etat TEXT CHECK (traitement_etat IS NULL OR traitement_etat IN (
        'sans_incidence', 'rattrapage_interne', 'transmission_rectorat', 'service_non_fait')),
    traitement_ogec TEXT CHECK (traitement_ogec IS NULL OR traitement_ogec IN (
        'sans_incidence', 'rattrapage', 'retenue_salaire', 'maintien_salaire', 'conge_paye')),
    traitement_impose BOOLEAN NOT NULL DEFAULT false,
    -- R-AP47 : où se prend la décision, par employeur. `externe` : l'établissement prend en compte et
    -- transmet, l'employeur (rectorat, DSDEN) décide.
    decision_etat TEXT NOT NULL DEFAULT 'interne' CHECK (decision_etat IN ('interne', 'externe')),
    decision_ogec TEXT NOT NULL DEFAULT 'interne' CHECK (decision_ogec IN ('interne', 'externe')),
    texte_libre_permis BOOLEAN NOT NULL DEFAULT false,
    -- Posée par la direction seulement (absence injustifiée).
    direction_seule BOOLEAN NOT NULL DEFAULT false,
    -- Retard (≤ 4 h, même jour, R-AP14).
    duree_max_heures SMALLINT CHECK (duree_max_heures IS NULL OR duree_max_heures BETWEEN 1 AND 24),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Jamais de texte libre sur une nature sensible ; une nature propre n'est jamais sensible.
    CONSTRAINT ent_absp_nature_libre_chk CHECK (NOT (sensible AND texte_libre_permis)),
    CONSTRAINT ent_absp_nature_propre_chk CHECK (etablissement_id IS NULL OR NOT sensible)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_nature_code_uq
    ON ent_absp_nature (COALESCE(etablissement_id, '00000000-0000-0000-0000-000000000000'::uuid), code);

-- Référentiel fourni (version initiale, §9.2). `ON CONFLICT DO NOTHING` : rejouable.
INSERT INTO ent_absp_nature
  (id, etablissement_id, code, libelle_agent, famille, categorie_nationale, employeur, sensible, refus_possible,
   piece_attendue, delai_piece_heures, traitement_etat, traitement_ogec, traitement_impose,
   decision_etat, decision_ogec, texte_libre_permis, direction_seule, duree_max_heures)
VALUES
 (gen_random_uuid(), NULL, 'maladie', 'Arrêt de travail (maladie)', 'sante', 'raison_sante', 'tous', true, false,
  'arret', 48, 'transmission_rectorat', 'maintien_salaire', true, 'externe', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'accident_service', 'Accident du travail / de service', 'sante', 'raison_sante', 'tous', true, false,
  'justificatif', 48, 'transmission_rectorat', 'maintien_salaire', true, 'externe', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'maternite_paternite', 'Congé de maternité, de paternité, d''adoption', 'parentalite', 'evenement_familial', 'tous', true, false,
  'justificatif', NULL, 'transmission_rectorat', 'maintien_salaire', true, 'externe', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'enfant_malade', 'Garde d''enfant malade', 'famille', 'evenement_familial', 'tous', true, false,
  'justificatif', NULL, 'sans_incidence', 'maintien_salaire', false, 'interne', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'evenement_familial', 'Événement familial (décès, mariage, naissance…)', 'famille', 'evenement_familial', 'tous', true, false,
  'justificatif', NULL, 'sans_incidence', 'maintien_salaire', false, 'interne', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'asa_service', 'Autorisation d''absence (nécessités du service)', 'autorisation', 'convenance_personnelle', 'etat', false, true,
  'aucune', NULL, 'sans_incidence', NULL, false, 'interne', 'interne', true, false, NULL),
 (gen_random_uuid(), NULL, 'formation', 'Formation', 'service', 'formation', 'tous', false, true,
  'convocation', NULL, 'sans_incidence', 'sans_incidence', false, 'interne', 'interne', true, false, NULL),
 (gen_random_uuid(), NULL, 'convocation', 'Convocation (examen, jury, rectorat, justice)', 'service', 'institutionnel', 'tous', false, false,
  'convocation', NULL, 'sans_incidence', 'sans_incidence', true, 'interne', 'interne', true, false, NULL),
 (gen_random_uuid(), NULL, 'representation', 'Mandat syndical, représentation du personnel', 'representation', NULL, 'tous', true, false,
  'aucune', NULL, 'sans_incidence', 'sans_incidence', true, 'interne', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'greve', 'Grève', 'greve', NULL, 'tous', true, false,
  'aucune', NULL, 'service_non_fait', 'retenue_salaire', true, 'interne', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'convenance', 'Convenance personnelle (sans solde)', 'autorisation', 'convenance_personnelle', 'tous', false, true,
  'aucune', NULL, 'service_non_fait', 'retenue_salaire', false, 'interne', 'interne', true, false, NULL),
 (gen_random_uuid(), NULL, 'retard', 'Retard (moins de 4 h, même jour)', 'autre', NULL, 'tous', false, true,
  'aucune', NULL, 'sans_incidence', 'sans_incidence', false, 'interne', 'interne', true, false, 4),
 (gen_random_uuid(), NULL, 'conge_paye', 'Congés payés, RTT, récupération', 'conges', NULL, 'ogec', false, true,
  'aucune', NULL, NULL, 'conge_paye', true, 'interne', 'interne', false, false, NULL),
 (gen_random_uuid(), NULL, 'absence_injustifiee', 'Absence sans justification', 'autre', 'en_attente_de_motif', 'tous', false, true,
  'aucune', NULL, 'service_non_fait', 'retenue_salaire', false, 'interne', 'interne', false, true, NULL),
 (gen_random_uuid(), NULL, 'autre_service', 'Indisponibilité de service', 'service', NULL, 'tous', false, true,
  'aucune', NULL, 'sans_incidence', 'sans_incidence', true, 'interne', 'interne', true, true, NULL),
 (gen_random_uuid(), NULL, 'inconnue', 'Je préciserai plus tard', 'autre', 'en_attente_de_motif', 'tous', false, true,
  'aucune', NULL, NULL, NULL, false, 'interne', 'interne', false, false, NULL)
ON CONFLICT DO NOTHING;

-- Activation par établissement d'une nature fournie (une nature fournie est active tant qu'aucune
-- ligne ne la désactive : S-1500).
CREATE TABLE IF NOT EXISTS ent_absp_nature_etab (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_id UUID NOT NULL REFERENCES ent_absp_nature (id),
    actif BOOLEAN NOT NULL,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_user_id UUID,
    PRIMARY KEY (etablissement_id, nature_id)
);

-- 2. Paramètres (une ligne par établissement ; colonnes typées, S-1500 à S-1517) ------------------------
CREATE TABLE IF NOT EXISTS ent_absp_param (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- S-1501 : délai avant reprise d'une absence par la direction (valideur silencieux), jours ouvrés.
    reprise_valideur_jours SMALLINT NOT NULL DEFAULT 3 CHECK (reprise_valideur_jours BETWEEN 1 AND 30),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_user_id UUID
);

-- 3. Règles de circuit (§9.4) ----------------------------------------------------------------------------
-- Valideur / instructeur / informé selon l'employeur (et la famille de nature). À défaut de règle : le
-- chef d'établissement valide, le secrétariat (État) ou la comptabilité (OGEC) instruit.
CREATE TABLE IF NOT EXISTS ent_absp_circuit_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    employeur TEXT NOT NULL CHECK (employeur IN ('etat', 'ogec', 'tous')),
    famille TEXT CHECK (famille IS NULL OR famille IN (
        'sante', 'parentalite', 'famille', 'autorisation', 'service', 'representation',
        'greve', 'conges', 'autre')),
    role TEXT NOT NULL CHECK (role IN ('valideur', 'instructeur', 'informe')),
    agent_id UUID NOT NULL,
    rang SMALLINT NOT NULL DEFAULT 1 CHECK (rang BETWEEN 1 AND 5),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_absp_circuit_regle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_circuit_regle_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_circuit_regle_idx
    ON ent_absp_circuit_regle (etablissement_id, employeur, role) WHERE actif;

SELECT ent_rls_poser(ARRAY['ent_absp_nature_etab', 'ent_absp_param', 'ent_absp_circuit_regle']);
-- `ent_absp_nature` : les lignes fournies (etablissement_id NULL) sont lisibles de tous ; l'écriture est
-- limitée aux lignes de l'établissement courant. Même politique que `ent_mission_type` (355).
ALTER TABLE ent_absp_nature ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_absp_nature FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_absp_nature;
CREATE POLICY ent_rls_etablissement ON ent_absp_nature FOR ALL
    USING (etablissement_id IS NULL
           OR etablissement_id = (SELECT ent_rls_etablissement())
           OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
