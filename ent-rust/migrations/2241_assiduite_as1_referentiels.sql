-- 2241 — Socle 12, lot AS1 (référentiels et droits). Réf. : 12_ASSIDUITE.md §22.1, §23, §24.
-- Natures de motif (L131-8), profils académiques, motifs d'établissement (versionnés), règle
-- d'assiduité DATÉE et immuable (R-AS15, DA-11), activités appelées. Reprise idempotente des motifs
-- (003, 013) et de la règle (130). Additive et rejouable (E8) ; aucune colonne JSON (D-036) ;
-- aucun BYTEA ; RLS par `ent_rls_poser` (580).

-- ── Référentiels globaux (aucune donnée d'établissement : E4, E12) ───────────────────────────
CREATE TABLE IF NOT EXISTS ref_asd_nature_motif (
    code TEXT PRIMARY KEY CHECK (code ~ '^[a-z0-9_]{2,40}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    legitime_l131_8 BOOLEAN NOT NULL DEFAULT false,
    medical BOOLEAN NOT NULL DEFAULT false,
    ordre SMALLINT NOT NULL DEFAULT 100,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Les cinq motifs réputés légitimes (L131-8 du code de l'éducation) puis les natures de classement.
INSERT INTO ref_asd_nature_motif (code, libelle, legitime_l131_8, medical, ordre) VALUES
    ('maladie_enfant', 'Maladie de l''enfant', true, true, 10),
    ('maladie_contagieuse_famille', 'Maladie transmissible ou contagieuse d''un membre de la famille', true, true, 20),
    ('reunion_solennelle_famille', 'Réunion solennelle de famille', true, false, 30),
    ('difficulte_communications', 'Empêchement résultant de la difficulté accidentelle des communications', true, false, 40),
    ('absence_temporaire_responsables', 'Absence temporaire des personnes responsables lorsque l''enfant les suit', true, false, 50),
    ('rendez_vous_medical', 'Rendez-vous médical ou paramédical', false, true, 110),
    ('transport', 'Problème de transport', false, false, 120),
    ('examen_concours', 'Examen, concours ou épreuve extérieure', false, false, 130),
    ('convocation_officielle', 'Convocation officielle (justice, administration, JDC)', false, false, 140),
    ('fete_religieuse', 'Fête religieuse', false, false, 150),
    ('competition_sportive', 'Compétition ou entraînement sportif', false, false, 160),
    ('raison_familiale', 'Raison familiale', false, false, 170),
    ('autre', 'Autre motif', false, false, 900)
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS ref_asd_profil_academique (
    id UUID PRIMARY KEY,
    code_academie TEXT NOT NULL CHECK (code_academie ~ '^[A-Z0-9_]{2,20}$'),
    version INTEGER NOT NULL CHECK (version >= 1),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    mois_type TEXT NOT NULL CHECK (mois_type IN ('civil', 'glissant')),
    outil TEXT NOT NULL DEFAULT 'manuel' CHECK (outil IN ('manuel', 'export', 'api')),
    age_limite_penal SMALLINT NOT NULL DEFAULT 16 CHECK (age_limite_penal BETWEEN 6 AND 18),
    valide_du DATE NOT NULL,
    valide_au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ref_asd_profil_dates_chk CHECK (valide_au IS NULL OR valide_au >= valide_du),
    CONSTRAINT ref_asd_profil_uq UNIQUE (code_academie, version)
);

CREATE TABLE IF NOT EXISTS ref_asd_profil_niveau (
    profil_id UUID NOT NULL REFERENCES ref_asd_profil_academique (id),
    niveau SMALLINT NOT NULL CHECK (niveau BETWEEN 1 AND 6),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    seuil_demi_journees SMALLINT NOT NULL CHECK (seuil_demi_journees BETWEEN 1 AND 100),
    delai_jours SMALLINT CHECK (delai_jours IS NULL OR delai_jours BETWEEN 0 AND 365),
    piece_exigee_code TEXT,
    destinataire_code TEXT NOT NULL DEFAULT 'vie_scolaire'
        CHECK (destinataire_code IN ('vie_scolaire', 'direction', 'dasen')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (profil_id, niveau)
);

-- Profil de départ « seuil légal » (R131-7) : aucune particularité d'académie n'est inventée ;
-- les profils réels sont à fournir par l'établissement (point Florian, §27.2).
INSERT INTO ref_asd_profil_academique (id, code_academie, version, libelle, mois_type, outil, age_limite_penal, valide_du)
VALUES ('00000000-0000-4000-8000-00000000a501', 'LEGAL', 1, 'Seuil légal (R131-7), sans particularité d''académie', 'civil', 'manuel', 16, DATE '2000-01-01')
ON CONFLICT (code_academie, version) DO NOTHING;
INSERT INTO ref_asd_profil_niveau (profil_id, niveau, libelle, seuil_demi_journees, delai_jours, destinataire_code)
VALUES ('00000000-0000-4000-8000-00000000a501', 1, 'Quatre demi-journées dans le mois sans motif légitime ni excuse valable', 4, NULL, 'direction')
ON CONFLICT (profil_id, niveau) DO NOTHING;

-- ── Motifs d'établissement (remplace ent_absence_motif_etab) ────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_asd_motif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z0-9_]{2,40}$'),
    nature_code TEXT NOT NULL REFERENCES ref_asd_nature_motif (code),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    libelle_famille TEXT CHECK (libelle_famille IS NULL OR length(btrim(libelle_famille)) BETWEEN 1 AND 200),
    visible_famille BOOLEAN NOT NULL DEFAULT true,
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_motif_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_motif_code_uq UNIQUE (etablissement_id, code)
);

-- Historique des libellés (DA-10, DA-20) : ajout seul.
CREATE TABLE IF NOT EXISTS ent_asd_motif_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    motif_id UUID NOT NULL,
    libelle TEXT NOT NULL,
    libelle_famille TEXT,
    visible_famille BOOLEAN NOT NULL,
    actif BOOLEAN NOT NULL,
    auteur_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_motif_version_fk FOREIGN KEY (etablissement_id, motif_id)
        REFERENCES ent_asd_motif (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_motif_version_motif_idx ON ent_asd_motif_version (etablissement_id, motif_id, cree_le);

-- ── Règle d'assiduité datée (R-AS15) ────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_asd_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    valide_du DATE NOT NULL,
    definition_demi_journee TEXT NOT NULL DEFAULT 'minutes'
        CHECK (definition_demi_journee IN ('une_seance', 'minutes', 'entiere')),
    seuil_minutes INTEGER NOT NULL DEFAULT 30 CHECK (seuil_minutes BETWEEN 1 AND 600),
    -- Transitoire : la coupure viendra du calendrier du socle 5 (AS4) ; reprise de 130.
    heure_coupure TIME NOT NULL DEFAULT '12:00',
    type_mois TEXT NOT NULL DEFAULT 'civil' CHECK (type_mois IN ('civil', 'glissant')),
    profil_id UUID REFERENCES ref_asd_profil_academique (id),
    seuil_signalement_demi_journees INTEGER NOT NULL DEFAULT 4 CHECK (seuil_signalement_demi_journees BETWEEN 1 AND 62),
    delai_consolidation_min INTEGER NOT NULL DEFAULT 10 CHECK (delai_consolidation_min BETWEEN 0 AND 240),
    delai_verification_min INTEGER NOT NULL DEFAULT 15 CHECK (delai_verification_min BETWEEN 0 AND 240),
    rythme_relance TEXT NOT NULL DEFAULT 'standard' CHECK (rythme_relance IN ('court', 'standard', 'long')),
    seuil_retard_absence_pct INTEGER NOT NULL DEFAULT 50 CHECK (seuil_retard_absence_pct BETWEEN 1 AND 100),
    seuil_absences_repetees INTEGER NOT NULL DEFAULT 3 CHECK (seuil_absences_repetees BETWEEN 1 AND 100),
    seuil_retards INTEGER NOT NULL DEFAULT 5 CHECK (seuil_retards BETWEEN 1 AND 100),
    decide_par UUID,
    -- Posé à l'arrêt d'un mois qui l'a utilisée (AS4) : la règle devient définitivement figée.
    figee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_regle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_regle_date_uq UNIQUE (etablissement_id, valide_du)
);

-- Une règle figée par un mois arrêté (figee_le posé par AS4) ne change plus jamais : ni modification,
-- ni suppression, ni défigeage (R-AS15, DA-11). Avant cela, la version du jour peut être remplacée.
CREATE OR REPLACE FUNCTION ent_asd_regle_immuable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.figee_le IS NOT NULL THEN
        RAISE EXCEPTION 'ent_asd_regle : règle figée par un mois arrêté, créer une nouvelle version (R-AS15)';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_regle_immuable_trg ON ent_asd_regle;
CREATE TRIGGER ent_asd_regle_immuable_trg BEFORE UPDATE OR DELETE ON ent_asd_regle
    FOR EACH ROW EXECUTE FUNCTION ent_asd_regle_immuable();

CREATE OR REPLACE FUNCTION ent_asd_ajout_seul() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_motif_version_ajout_seul ON ent_asd_motif_version;
CREATE TRIGGER ent_asd_motif_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_motif_version
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

-- ── Activités appelées (RA-02) ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_asd_activite_appelee (
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type_activite TEXT NOT NULL CHECK (type_activite IN ('permanence', 'etude', 'cdi', 'demi_pension', 'accompagnement')),
    active BOOLEAN NOT NULL DEFAULT true,
    valide_du DATE NOT NULL DEFAULT DATE '2000-01-01',
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, type_activite)
);

-- ── Reprise idempotente ─────────────────────────────────────────────────────────────────────
-- Règle : première version « minutes » reprenant le comportement passé de 130 (on garde le calcul
-- des mois écoulés) ; un établissement sans ligne reçoit les valeurs par défaut du code.
INSERT INTO ent_asd_regle (id, etablissement_id, valide_du, definition_demi_journee, seuil_minutes,
                           heure_coupure, seuil_signalement_demi_journees, delai_verification_min, profil_id)
SELECT md5('asd_regle_v1:' || e.id::text)::uuid, e.id, DATE '2000-01-01', 'minutes',
       coalesce(r.seuil_minutes, 30), coalesce(r.heure_coupure, TIME '12:00'),
       coalesce(r.seuil_signalement, 4), coalesce(r.delai_verification_minutes, 15),
       '00000000-0000-4000-8000-00000000a501'::uuid
  FROM ent_etablissement e
  LEFT JOIN ent_vs_regle_assiduite r ON r.etablissement_id = e.id
ON CONFLICT DO NOTHING;
-- Établissements connus seulement par leur ligne de règle (base sans ent_etablissement).
INSERT INTO ent_asd_regle (id, etablissement_id, valide_du, definition_demi_journee, seuil_minutes,
                           heure_coupure, seuil_signalement_demi_journees, delai_verification_min, profil_id)
SELECT md5('asd_regle_v1:' || r.etablissement_id::text)::uuid, r.etablissement_id, DATE '2000-01-01', 'minutes',
       r.seuil_minutes, r.heure_coupure, r.seuil_signalement, r.delai_verification_minutes,
       '00000000-0000-4000-8000-00000000a501'::uuid
  FROM ent_vs_regle_assiduite r
ON CONFLICT DO NOTHING;

-- Motifs : correspondance des codes globaux (le motif technique « appel » n'est pas repris : DA-20).
INSERT INTO ent_asd_motif (id, etablissement_id, code, nature_code, libelle)
SELECT md5('asd_motif:' || e.id::text || ':' || m.code)::uuid, e.id, m.code,
       CASE m.code WHEN 'maladie' THEN 'maladie_enfant' WHEN 'familial' THEN 'raison_familiale' ELSE 'autre' END,
       m.libelle
  FROM ent_etablissement e CROSS JOIN ent_absence_motif m
 WHERE m.code <> 'appel'
ON CONFLICT DO NOTHING;
INSERT INTO ent_asd_motif (id, etablissement_id, code, nature_code, libelle)
SELECT md5('asd_motif:' || m.etablissement_id::text || ':' || m.code)::uuid, m.etablissement_id, m.code, 'autre', m.libelle
  FROM ent_absence_motif_etab m
 WHERE m.code <> 'appel'
ON CONFLICT DO NOTHING;
INSERT INTO ent_asd_motif_version (id, etablissement_id, motif_id, libelle, libelle_famille, visible_famille, actif)
SELECT md5('asd_motif_v1:' || m.id::text)::uuid, m.etablissement_id, m.id, m.libelle, m.libelle_famille, m.visible_famille, m.actif
  FROM ent_asd_motif m
ON CONFLICT DO NOTHING;

-- Activités appelées : permanence et étude actives par défaut (obligation R131-5 pour toute activité).
INSERT INTO ent_asd_activite_appelee (etablissement_id, type_activite, active)
SELECT e.id, t.t, true FROM ent_etablissement e CROSS JOIN (VALUES ('permanence'), ('etude')) AS t(t)
ON CONFLICT DO NOTHING;

SELECT ent_rls_poser(ARRAY[
    'ent_asd_motif', 'ent_asd_motif_version', 'ent_asd_regle', 'ent_asd_activite_appelee'
]);
