-- Socle 20 (cahier de textes), lot C1 — migration 2401 : réglages, natures de travail et gardes communes.
-- Réf. : docs/socles/20_CAHIER_DE_TEXTES.md §24.1 ; exigences E1–E14 (PRA_SAUVEGARDES §18.4) ; D-036 (aucune colonne
-- JSON ici : tout est typé). Additif et rejouable (rejoué à chaque démarrage) ; aucune table existante modifiée.
--
-- Conventions communes aux migrations 2401 à 2411 :
--  * `etablissement_id UUID NOT NULL`, clé primaire UUID applicative, `UNIQUE (etablissement_id, id)`,
--    clés étrangères COMPOSITES `(etablissement_id, x_id)` ou `(x_id, etablissement_id)` selon l'index de la cible,
--    `ON DELETE RESTRICT` partout (aucune cascade), horodatage `cree_le`, RLS posée en fin de fichier (ent_rls_poser) ;
--  * les tables « ajout seul » refusent UPDATE et DELETE (`ent_cdt_ajout_seul`) ;
--  * une entrée, un travail, une remise ne se suppriment jamais (`ent_cdt_jamais_supprime`, R-C3).

CREATE OR REPLACE FUNCTION ent_cdt_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% : ajout seul (aucun UPDATE ni DELETE)', TG_TABLE_NAME USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION ent_cdt_jamais_supprime() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% : jamais supprimé (retirer ou clore, R-C3)', TG_TABLE_NAME USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;

-- Réglages par établissement et année (S20-xx) : peu de réglages, de bons défauts (D-023).
CREATE TABLE IF NOT EXISTS ent_cdt_reglage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    publication_defaut TEXT NOT NULL DEFAULT 'debut_seance'
        CHECK (publication_defaut IN ('debut_seance', 'fin_seance', 'immediate')),
    repere_renseignement TEXT NOT NULL DEFAULT 'soir' CHECK (repere_renseignement IN ('soir', '48h', 'semaine')),
    repere_charge TEXT NOT NULL DEFAULT 'normal' CHECK (repere_charge IN ('souple', 'normal', 'strict')),
    lecture_annee_close_jusqu_au DATE,
    copies_lisibles_responsables BOOLEAN NOT NULL DEFAULT false,
    cloture_ramassage_jours SMALLINT NOT NULL DEFAULT 14 CHECK (cloture_ramassage_jours BETWEEN 1 AND 90),
    rappel_veille_heure TIME NOT NULL DEFAULT '17:00',
    echeance_plage_calme_lycee BOOLEAN NOT NULL DEFAULT false,
    taille_max_mo SMALLINT CHECK (taille_max_mo IS NULL OR taille_max_mo BETWEEN 1 AND 100),
    version INTEGER NOT NULL DEFAULT 1,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_user_id UUID,
    CONSTRAINT ent_cdt_reglage_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_reglage_annee_uq UNIQUE (etablissement_id, annee_id),
    CONSTRAINT ent_cdt_reglage_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT
);

-- Formats de rendu proposés par défaut (extension : pdf, docx, jpg…).
CREATE TABLE IF NOT EXISTS ent_cdt_reglage_format (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    reglage_id UUID NOT NULL,
    format_code TEXT NOT NULL CHECK (format_code ~ '^[a-z0-9]{1,10}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (reglage_id, format_code),
    CONSTRAINT ent_cdt_reglage_format_fk FOREIGN KEY (etablissement_id, reglage_id)
        REFERENCES ent_cdt_reglage (etablissement_id, id) ON DELETE RESTRICT
);

-- Natures de travail (leçon, exercices, devoir maison…) : par établissement, archivables, jamais supprimées.
CREATE TABLE IF NOT EXISTS ent_cdt_nature (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z0-9_]{1,30}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 80),
    duree_defaut_min SMALLINT CHECK (duree_defaut_min IS NULL OR duree_defaut_min BETWEEN 1 AND 600),
    ordre SMALLINT NOT NULL DEFAULT 0,
    archivee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_nature_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_nature_code_uq UNIQUE (etablissement_id, code)
);

-- Natures par défaut d'un établissement (appelée par la reprise et à la première saisie).
CREATE OR REPLACE FUNCTION ent_cdt_assurer_natures(p_etab UUID) RETURNS VOID
LANGUAGE sql AS $$
    INSERT INTO ent_cdt_nature (id, etablissement_id, code, libelle, duree_defaut_min, ordre)
    SELECT gen_random_uuid(), p_etab, v.code, v.libelle, v.duree, v.ordre
      FROM (VALUES
        ('travail',   'Travail',            30, 1),
        ('exercices', 'Exercices',          30, 2),
        ('lecon',     'Leçon à apprendre',  20, 3),
        ('revision',  'Révision',           30, 4),
        ('dm',        'Devoir maison',      60, 5),
        ('recherche', 'Recherche',          45, 6),
        ('lecture',   'Lecture',            20, 7),
        ('projet',    'Projet',             60, 8)) AS v(code, libelle, duree, ordre)
    ON CONFLICT (etablissement_id, code) DO NOTHING
$$;

SELECT ent_rls_poser(ARRAY['ent_cdt_reglage', 'ent_cdt_reglage_format', 'ent_cdt_nature']);
