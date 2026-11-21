-- Socle 5, lot S3 (`docs/socles/05_STRUCTURE.md` §9.3, STR-23 à STR-30, STR-93, R-S2, R-S19).
--
-- Calendrier de l'établissement : référentiel officiel GLOBAL (vacances par zone, tenu par
-- ScolIA, jamais écrit par un établissement), événements du calendrier par année (vacances,
-- fériés, ponts, journées banalisées, rattrapage, examens…) avec cibles (établissement, site,
-- niveau, classe, groupe ou service), types de semaine (A/B) et semaines matérialisées.
--
-- Les JOURS FÉRIÉS sont CALCULÉS par le code (Pâques, Ascension, Pentecôte, régimes locaux) ;
-- aucun jeu de données externe n'est embarqué (point ouvert : licence du jeu de données officiel
-- des vacances à vérifier, voir 05_STRUCTURE.md §24). La table `ref_calendrier_officiel` reste
-- vide tant qu'aucune source vérifiée n'y a été saisie ; la saisie manuelle fonctionne seule.
--
-- Migration additive et rejouable. La fermeture exceptionnelle (effets, levée) est en 334.

-- Clés étrangères composites (id, etablissement_id) pour les cibles.
-- (mêmes index, mêmes noms que 337 et 338 : IF NOT EXISTS les rend inoffensifs quelle que soit
-- l'ordre d'application sur une base déjà migrée).
CREATE UNIQUE INDEX IF NOT EXISTS ent_annee_scolaire_id_etab_uq ON ent_annee_scolaire (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_site_id_etab_uq ON ent_site (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_niveau_id_etab_uq ON ent_niveau (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_groupe_id_etab_uq ON ent_groupe (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_classe_id_etab_uq ON ent_classe (id, etablissement_id);

-- ───────────────────────── Référentiel officiel (global, hors tenant) ─────────────────────────
-- Lecture seule pour les établissements : aucune colonne etablissement_id, donc aucune RLS
-- (comme `ent_etablissement`). Écrit uniquement par la fonction de plateforme (import / saisie
-- de référence), jamais par une route d'établissement.
CREATE TABLE IF NOT EXISTS ref_calendrier_officiel (
    id UUID PRIMARY KEY,
    zone TEXT NOT NULL CHECK (zone = lower(zone) AND length(btrim(zone)) BETWEEN 1 AND 32),
    annee_libelle TEXT NOT NULL CHECK (length(btrim(annee_libelle)) BETWEEN 1 AND 32),
    type TEXT NOT NULL CHECK (type IN ('vacances', 'ferie', 'pont_national')),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    du DATE NOT NULL,
    au DATE NOT NULL,
    source TEXT NOT NULL CHECK (length(btrim(source)) BETWEEN 1 AND 200),
    source_version TEXT NOT NULL CHECK (length(btrim(source_version)) BETWEEN 1 AND 64),
    importe_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (au >= du),
    UNIQUE (zone, annee_libelle, type, libelle)
);

-- ───────────────────────── Événements du calendrier ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_calendrier_evenement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN
        ('vacances', 'ferie', 'pont', 'fermeture', 'fermeture_exceptionnelle', 'journee_banalisee',
         'journee_pedagogique', 'rattrapage', 'examen', 'stage', 'periode_entreprise', 'rentree',
         'pre_rentree', 'evenement')),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 160),
    du DATE NOT NULL,
    au DATE NOT NULL,
    -- Demi-journées : `du_demi = apres_midi` = commence l'après-midi de `du` ;
    -- `au_demi = matin` = finit le matin de `au`.
    du_demi TEXT CHECK (du_demi IS NULL OR du_demi IN ('matin', 'apres_midi')),
    au_demi TEXT CHECK (au_demi IS NULL OR au_demi IN ('matin', 'apres_midi')),
    effet TEXT NOT NULL CHECK (effet IN
        ('sans_cours', 'sans_eleves', 'accueil_seulement', 'cours_modifies', 'informatif')),
    origine TEXT NOT NULL DEFAULT 'saisie' CHECK (origine IN ('officiel', 'calcule', 'saisie', 'import')),
    ref_officiel_id UUID REFERENCES ref_calendrier_officiel (id),
    -- Version de la source officielle adoptée (pour proposer les différences à accepter).
    ref_version TEXT,
    publie_familles BOOLEAN NOT NULL DEFAULT false,
    -- R-S19 : publié aux personnels seulement (conseil pédagogique, formation, pré-rentrée).
    publie_personnels BOOLEAN NOT NULL DEFAULT false,
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    -- Rattrapage (STR-27) : la journée suit l'emploi du temps de ce jour de la semaine.
    jour_reference TEXT CHECK (jour_reference IS NULL OR jour_reference IN
        ('lun', 'mar', 'mer', 'jeu', 'ven', 'sam', 'dim')),
    -- Fermeture exceptionnelle (migration 334 pour les contrôles) : services touchés.
    services_touches TEXT[] NOT NULL DEFAULT '{}',
    conditionnelle BOOLEAN NOT NULL DEFAULT false,
    levee_prevue_le TIMESTAMPTZ,
    internes_presents BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    annule_le TIMESTAMPTZ,
    annule_par UUID,
    annule_motif TEXT CHECK (annule_motif IS NULL OR length(annule_motif) <= 500),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_calendrier_evenement_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT ent_calendrier_evenement_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_calendrier_evenement_dates_chk CHECK (
        au >= du
        AND NOT (du = au AND du_demi = 'apres_midi' AND au_demi = 'matin')),
    CONSTRAINT ent_calendrier_evenement_rattrapage_chk CHECK (
        (type = 'rattrapage') = (jour_reference IS NOT NULL)),
    CONSTRAINT ent_calendrier_evenement_annule_chk CHECK (
        annule_le IS NOT NULL OR (annule_par IS NULL AND annule_motif IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_calendrier_evenement_periode_idx
    ON ent_calendrier_evenement (etablissement_id, annee_id, du, au) WHERE annule_le IS NULL;
-- Un événement officiel ou calculé n'est adopté qu'une fois par année.
CREATE UNIQUE INDEX IF NOT EXISTS ent_calendrier_evenement_officiel_uq
    ON ent_calendrier_evenement (etablissement_id, annee_id, ref_officiel_id)
    WHERE ref_officiel_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_calendrier_evenement_calcule_uq
    ON ent_calendrier_evenement (etablissement_id, annee_id, type, libelle, du)
    WHERE origine = 'calcule';

-- Cibles : aucune ligne = tout l'établissement ; une colonne exactement par ligne.
CREATE TABLE IF NOT EXISTS ent_calendrier_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    site_id UUID,
    niveau_id UUID,
    classe_id UUID,
    groupe_id UUID,
    -- Services (STR-93) : jours d'ouverture et de fermeture hors cours (R-S2).
    service TEXT CHECK (service IS NULL OR service IN
        ('secretariat', 'intendance', 'restauration', 'internat', 'accueil', 'cdi', 'vie_scolaire')),
    CONSTRAINT ent_calendrier_cible_evenement_fk FOREIGN KEY (evenement_id, etablissement_id)
        REFERENCES ent_calendrier_evenement (id, etablissement_id) ON DELETE CASCADE,
    CONSTRAINT ent_calendrier_cible_site_fk FOREIGN KEY (site_id, etablissement_id)
        REFERENCES ent_site (id, etablissement_id),
    CONSTRAINT ent_calendrier_cible_niveau_fk FOREIGN KEY (niveau_id, etablissement_id)
        REFERENCES ent_niveau (id, etablissement_id),
    CONSTRAINT ent_calendrier_cible_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES ent_classe (id, etablissement_id),
    CONSTRAINT ent_calendrier_cible_groupe_fk FOREIGN KEY (groupe_id, etablissement_id)
        REFERENCES ent_groupe (id, etablissement_id),
    CONSTRAINT ent_calendrier_cible_une_seule_chk CHECK (
        (site_id IS NOT NULL)::int + (niveau_id IS NOT NULL)::int + (classe_id IS NOT NULL)::int
        + (groupe_id IS NOT NULL)::int + (service IS NOT NULL)::int = 1)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_calendrier_cible_uq ON ent_calendrier_cible
    (evenement_id, (COALESCE(site_id::text, niveau_id::text, classe_id::text, groupe_id::text, service)));
CREATE INDEX IF NOT EXISTS ent_calendrier_cible_evenement_idx
    ON ent_calendrier_cible (etablissement_id, evenement_id);

-- ───────────────────────── Types de semaine (A/B) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_semaine_type_regle (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    etiquettes TEXT[] NOT NULL CHECK (cardinality(etiquettes) BETWEEN 2 AND 4),
    mode TEXT NOT NULL CHECK (mode IN ('alternance_continue', 'reprise_apres_vacances')),
    -- Lundi de la première semaine portant la première étiquette.
    premiere_semaine DATE NOT NULL CHECK (EXTRACT(ISODOW FROM premiere_semaine) = 1),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (annee_id),
    CONSTRAINT ent_semaine_type_regle_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id)
);

CREATE TABLE IF NOT EXISTS ent_semaine (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    lundi DATE NOT NULL CHECK (EXTRACT(ISODOW FROM lundi) = 1),
    -- NULL = semaine sans étiquette (vacances, avant la première semaine).
    etiquette TEXT CHECK (etiquette IS NULL OR length(btrim(etiquette)) BETWEEN 1 AND 8),
    source TEXT NOT NULL DEFAULT 'calcule' CHECK (source IN ('calcule', 'force')),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 300),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (annee_id, lundi),
    CONSTRAINT ent_semaine_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_semaine_lundi_idx ON ent_semaine (etablissement_id, lundi);

-- ───────────────────────── Journal : nouveaux objets ─────────────────────────
-- NOT VALID : les migrations sont rejouées à chaque démarrage et des lots suivants (335, 338, 341)
-- écrivent d'autres objets ; la liste finale (union de tous les lots) est posée par 341.
ALTER TABLE ent_structure_journal DROP CONSTRAINT IF EXISTS ent_structure_journal_objet_check;
ALTER TABLE ent_structure_journal ADD CONSTRAINT ent_structure_journal_objet_check CHECK (objet IN
    ('profil', 'parametre', 'fuseau', 'site', 'batiment', 'salle', 'lieu', 'disponibilite',
     'indisponibilite', 'import_salles', 'prereglage',
     'matiere', 'offre', 'offre_groupe', 'regle_choix', 'incompatibilite', 'prerequis',
     'enseignements_suivis', 'enseignement_suivi',
     'calendrier_evenement', 'calendrier_adoption', 'semaines', 'semaine', 'fermeture')) NOT VALID;

-- ───────────────────────── RLS ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_calendrier_evenement', 'ent_calendrier_cible',
                             'ent_semaine_type_regle', 'ent_semaine'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL
                USING (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
                WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))', t);
    END LOOP;
END
$$;
