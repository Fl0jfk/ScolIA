-- Socle 2, lot S2-2 : référentiel scolaire complet (D1/D4).
-- Sites, niveaux / MEF (nomenclatures nationales : champs prêts pour l'import SIECLE / BCN),
-- cycle de vie de l'année, périodes, calendrier, grille horaire, séances datées.
-- ADDITIF et idempotent : aucune colonne existante n'est modifiée ni supprimée.
-- Toutes les tables portent etablissement_id (cloisonnement).

-- ─────────────────────────── Sites (REF-14) ───────────────────────────
CREATE TABLE IF NOT EXISTS ent_site (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    adresse TEXT,
    -- UAI (ex-RNE) du site quand il en a un propre.
    uai TEXT,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_site_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_site_code_chk CHECK (length(btrim(code)) > 0)
);

-- ─────────────────────────── Niveaux (REF-07) ─────────────────────────
-- Code = mnémonique de niveau (6EME, 5EME, 4EME, 3EME, 2NDE, 1ERE, TERM…), stable pour les
-- imports ; `source` dit d'où vient la ligne (nomenclature fournie, saisie, import BCN).
CREATE TABLE IF NOT EXISTS ent_niveau (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    libelle_court TEXT,
    ordre SMALLINT NOT NULL DEFAULT 0,
    cycle SMALLINT,
    source TEXT NOT NULL DEFAULT 'saisie',
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_niveau_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_niveau_code_chk CHECK (length(btrim(code)) > 0),
    CONSTRAINT ent_niveau_cycle_chk CHECK (cycle IS NULL OR cycle BETWEEN 1 AND 4),
    CONSTRAINT ent_niveau_source_chk CHECK (source IN ('nomenclature', 'saisie', 'import_bcn', 'import_siecle'))
);

-- ─────────────────────────── MEF (REF-07) ─────────────────────────────
-- Module élémentaire de formation : code MEF à 11 caractères (nomenclature BCN N_MEF),
-- MEFSTAT11 pour les statistiques, formation/diplôme BCN pour les voies professionnelles.
CREATE TABLE IF NOT EXISTS ent_mef (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code_mef TEXT NOT NULL,
    mefstat11 TEXT,
    niveau_id UUID REFERENCES ent_niveau (id),
    libelle_long TEXT NOT NULL,
    libelle_edition TEXT,
    code_formation_diplome TEXT,
    date_ouverture DATE,
    date_fermeture DATE,
    source TEXT NOT NULL DEFAULT 'saisie',
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_mef_code_uq UNIQUE (etablissement_id, code_mef),
    CONSTRAINT ent_mef_code_chk CHECK (code_mef ~ '^[0-9A-Z]{11}$'),
    CONSTRAINT ent_mef_mefstat_chk CHECK (mefstat11 IS NULL OR mefstat11 ~ '^[0-9A-Z]{11}$'),
    CONSTRAINT ent_mef_dates_chk CHECK (
        date_ouverture IS NULL OR date_fermeture IS NULL OR date_fermeture >= date_ouverture
    ),
    CONSTRAINT ent_mef_source_chk CHECK (source IN ('saisie', 'import_siecle', 'import_bcn'))
);

CREATE INDEX IF NOT EXISTS ent_mef_niveau_idx ON ent_mef (etablissement_id, niveau_id);

-- ───────────────── Année : cycle de vie (REF-01 / 02 / 03) ─────────────────
-- preparation → courante → cloturee. `courante` (bool) reste la colonne historique ;
-- `statut` en est dérivé par le trigger ci-dessous (cloturee_le fait foi pour la clôture).
ALTER TABLE ent_annee_scolaire ADD COLUMN IF NOT EXISTS statut TEXT NOT NULL DEFAULT 'preparation';
ALTER TABLE ent_annee_scolaire ADD COLUMN IF NOT EXISTS cloturee_le TIMESTAMPTZ;

ALTER TABLE ent_annee_scolaire DROP CONSTRAINT IF EXISTS ent_annee_scolaire_statut_chk;
ALTER TABLE ent_annee_scolaire ADD CONSTRAINT ent_annee_scolaire_statut_chk
    CHECK (statut IN ('preparation', 'courante', 'cloturee'));

CREATE OR REPLACE FUNCTION ent_trg_annee_statut() RETURNS trigger AS $$
BEGIN
    IF NEW.cloturee_le IS NOT NULL THEN
        NEW.statut := 'cloturee';
        NEW.courante := false;
    ELSIF NEW.courante THEN
        NEW.statut := 'courante';
    ELSE
        NEW.statut := 'preparation';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_annee_statut ON ent_annee_scolaire;
CREATE TRIGGER ent_annee_statut BEFORE INSERT OR UPDATE ON ent_annee_scolaire
    FOR EACH ROW EXECUTE FUNCTION ent_trg_annee_statut();

UPDATE ent_annee_scolaire SET statut = 'courante' WHERE courante AND statut <> 'courante';

-- Affectations posées en lot (REF-11 : rentrée, montée) : sources supplémentaires.
ALTER TABLE ent_affectation DROP CONSTRAINT IF EXISTS ent_affectation_source_chk;
ALTER TABLE ent_affectation ADD CONSTRAINT ent_affectation_source_chk
    CHECK (source IN ('saisie', 'amorcage', 'sync_classe', 'lot', 'montee'));

-- ───────────────────── Classes : rattachements (REF-08) ─────────────────────
ALTER TABLE ent_classe ADD COLUMN IF NOT EXISTS site_id UUID REFERENCES ent_site (id);
ALTER TABLE ent_classe ADD COLUMN IF NOT EXISTS niveau_id UUID REFERENCES ent_niveau (id);
ALTER TABLE ent_classe ADD COLUMN IF NOT EXISTS mef_id UUID REFERENCES ent_mef (id);
ALTER TABLE ent_classe ADD COLUMN IF NOT EXISTS archivee_le TIMESTAMPTZ;

-- ─────────────────────────── Périodes (REF-04) ───────────────────────────
-- Trimestres / semestres d'une année, globaux (niveau_id nul) ou propres à un niveau.
CREATE TABLE IF NOT EXISTS ent_periode (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    niveau_id UUID REFERENCES ent_niveau (id),
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'periode',
    ordre SMALLINT NOT NULL DEFAULT 1,
    date_debut DATE NOT NULL,
    date_fin DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_periode_dates_chk CHECK (date_fin >= date_debut),
    CONSTRAINT ent_periode_kind_chk CHECK (kind IN ('trimestre', 'semestre', 'periode')),
    CONSTRAINT ent_periode_code_chk CHECK (length(btrim(code)) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_periode_code_uq
    ON ent_periode (
        etablissement_id, annee_id,
        coalesce(niveau_id, '00000000-0000-0000-0000-000000000000'::uuid), code
    );

CREATE INDEX IF NOT EXISTS ent_periode_annee_idx ON ent_periode (etablissement_id, annee_id);

-- Rapatriement de ent_note_periode (verrou de saisie des notes) : la définition des
-- dates vit désormais dans ent_periode ; ent_note_periode garde le statut de verrou.
ALTER TABLE ent_note_periode ADD COLUMN IF NOT EXISTS periode_id UUID REFERENCES ent_periode (id);

-- ───────────────────── Calendrier : jours neutralisés (REF-05) ─────────────────────
-- Fermetures et journées banalisées. Les vacances / fériés historiques restent dans
-- ent_periode_vacances (vacances_repo) ; la lecture unifiée les fusionne.
CREATE TABLE IF NOT EXISTS ent_jour_calendrier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    jour DATE NOT NULL,
    kind TEXT NOT NULL,
    libelle TEXT NOT NULL,
    -- Nul = tout l'établissement.
    site_id UUID REFERENCES ent_site (id),
    created_by_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_jour_calendrier_kind_chk CHECK (kind IN ('ferme', 'banalise', 'ferie', 'vacances'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_jour_calendrier_uq
    ON ent_jour_calendrier (
        etablissement_id, jour, coalesce(site_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );

-- ─────────────────────── Grille horaire (REF-06) ───────────────────────
-- Créneaux types M1…M4 (matin) et S1…S4 (après-midi) ; sert au décompte en demi-journées.
CREATE TABLE IF NOT EXISTS ent_grille_horaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    -- Nul = grille de tout l'établissement.
    site_id UUID REFERENCES ent_site (id),
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    demi_journee TEXT NOT NULL,
    ordre SMALLINT NOT NULL,
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_grille_heures_chk CHECK (heure_fin > heure_debut),
    CONSTRAINT ent_grille_demi_chk CHECK (demi_journee IN ('matin', 'apres_midi')),
    CONSTRAINT ent_grille_code_chk CHECK (length(btrim(code)) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_grille_horaire_uq
    ON ent_grille_horaire (
        etablissement_id, coalesce(site_id, '00000000-0000-0000-0000-000000000000'::uuid), code
    );

-- ───────────── EDT-03 : créneau lié à un service d'enseignement ─────────────
ALTER TABLE ent_edt_creneau
    ADD COLUMN IF NOT EXISTS service_id UUID REFERENCES ent_service_enseignement (id);

-- ───────────────── Séances datées (EDT-06) ─────────────────
-- Occurrence datée d'un créneau : support de l'appel, des absences à la séance (D-020),
-- du cahier de textes et des remplacements. Générée depuis l'EDT, la grille, les semaines
-- A/B, les exceptions et le calendrier ; régénération idempotente.
CREATE TABLE IF NOT EXISTS ent_edt_seance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    creneau_id UUID NOT NULL REFERENCES ent_edt_creneau (id),
    jour DATE NOT NULL,
    classe_id UUID REFERENCES ent_classe (id),
    classe TEXT NOT NULL,
    service_id UUID REFERENCES ent_service_enseignement (id),
    prof_user_id UUID NOT NULL REFERENCES ent_user (id),
    matiere_id UUID,
    matiere_libelle TEXT NOT NULL,
    salle TEXT,
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    grille_code TEXT,
    demi_journee TEXT,
    statut TEXT NOT NULL DEFAULT 'prevue',
    remplacant_user_id UUID REFERENCES ent_user (id),
    source TEXT NOT NULL DEFAULT 'genere',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_seance_uq UNIQUE (etablissement_id, creneau_id, jour),
    CONSTRAINT ent_edt_seance_statut_chk CHECK (statut IN ('prevue', 'annulee', 'remplacee')),
    CONSTRAINT ent_edt_seance_demi_chk CHECK (demi_journee IS NULL OR demi_journee IN ('matin', 'apres_midi')),
    CONSTRAINT ent_edt_seance_heures_chk CHECK (heure_fin > heure_debut)
);

CREATE INDEX IF NOT EXISTS ent_edt_seance_jour_idx ON ent_edt_seance (etablissement_id, jour);
CREATE INDEX IF NOT EXISTS ent_edt_seance_classe_idx ON ent_edt_seance (etablissement_id, classe_id, jour);
CREATE INDEX IF NOT EXISTS ent_edt_seance_prof_idx ON ent_edt_seance (etablissement_id, prof_user_id, jour);
