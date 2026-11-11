-- Socle 20, lot C1 — migration 2403 : travaux (donné lors d'une séance, attendu pour une autre), versions, formats,
-- public restreint, variantes, échéances individuelles, demandes de délai, groupes de remise, notes d'accompagnement.
-- Réf. : 20_CAHIER_DE_TEXTES.md §24.1, §9, §10, R-C1, R-C8, R-C24, R-C25. Succède à `ent_travail` (150).
-- Aucun motif d'aménagement stocké en clair : seulement un code interne (R-C8).

CREATE TABLE IF NOT EXISTS ent_cdt_travail (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID NOT NULL,
    annee_id UUID NOT NULL,
    seance_donnee_id UUID,
    donne_le DATE NOT NULL,
    echeance_mode TEXT NOT NULL CHECK (echeance_mode IN ('seance', 'date_heure')),
    echeance_seance_id UUID,
    -- Heure locale de l'établissement convertie en instant (jamais l'heure UTC saisie à la main).
    echeance_le TIMESTAMPTZ NOT NULL,
    nature_id UUID NOT NULL,
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    consigne TEXT NOT NULL CHECK (length(btrim(consigne)) > 0 AND length(consigne) <= 20000),
    duree_estimee_min SMALLINT CHECK (duree_estimee_min IS NULL OR duree_estimee_min BETWEEN 1 AND 600),
    rendu TEXT NOT NULL DEFAULT 'aucun' CHECK (rendu IN ('aucun', 'en_ligne', 'en_classe')),
    retard_accepte BOOLEAN NOT NULL DEFAULT true,
    cloture_ramassage_le TIMESTAMPTZ,
    remise_par_groupe BOOLEAN NOT NULL DEFAULT false,
    inscription_groupes_jusqu_au TIMESTAMPTZ,
    taille_groupe SMALLINT CHECK (taille_groupe IS NULL OR taille_groupe BETWEEN 2 AND 12),
    public TEXT NOT NULL DEFAULT 'cours' CHECK (public IN ('cours', 'restreint')),
    -- Évaluation annoncée du socle 21 : sans clé (table d'un autre socle), contrôlée par le code.
    evaluation_id UUID,
    ia_autorisee TEXT NOT NULL DEFAULT 'non' CHECK (ia_autorisee IN ('non', 'recherche', 'relecture', 'libre')),
    -- Même travail donné à plusieurs cours.
    relie_id UUID,
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'programme', 'publie', 'retire')),
    publie_a TIMESTAMPTZ,
    visible_a TIMESTAMPTZ,
    retire_motif TEXT CHECK (retire_motif IS NULL OR length(btrim(retire_motif)) BETWEEN 3 AND 500),
    retire_par_user_id UUID,
    auteur_user_id UUID REFERENCES ent_user (id),
    auteur_agent_id UUID,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    -- Reprise de `ent_travail` : identifiant d'origine, unique (reprise rejouable) ; 'controle' repris en nature
    -- « révision » + proposition au professeur de créer l'évaluation annoncée (socle 21).
    ancien_id UUID UNIQUE,
    ancien_type TEXT CHECK (ancien_type IS NULL OR ancien_type = 'controle'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_travail_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_travail_echeance_chk CHECK ((echeance_mode = 'seance') = (echeance_seance_id IS NOT NULL)),
    CONSTRAINT ent_cdt_travail_retrait_chk CHECK ((retire_motif IS NULL) = (retire_par_user_id IS NULL)),
    CONSTRAINT ent_cdt_travail_groupe_chk CHECK (remise_par_groupe OR (taille_groupe IS NULL AND inscription_groupes_jusqu_au IS NULL)),
    CONSTRAINT ent_cdt_travail_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_travail_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT,
    -- R-C3 : une séance qui porte un travail (donné ou attendu) n'est jamais supprimée.
    CONSTRAINT ent_cdt_travail_seance_donnee_fk FOREIGN KEY (etablissement_id, seance_donnee_id)
        REFERENCES ent_edt_seance (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_travail_seance_echeance_fk FOREIGN KEY (etablissement_id, echeance_seance_id)
        REFERENCES ent_edt_seance (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_travail_nature_fk FOREIGN KEY (etablissement_id, nature_id)
        REFERENCES ent_cdt_nature (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_travail_relie_fk FOREIGN KEY (etablissement_id, relie_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_travail_agent_fk FOREIGN KEY (etablissement_id, auteur_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_travail_echeance_idx ON ent_cdt_travail (etablissement_id, cours_id, echeance_le);
CREATE INDEX IF NOT EXISTS ent_cdt_travail_visible_idx ON ent_cdt_travail (etablissement_id, visible_a) WHERE etat <> 'retire';

DROP TRIGGER IF EXISTS ent_cdt_travail_jamais_supprime ON ent_cdt_travail;
CREATE TRIGGER ent_cdt_travail_jamais_supprime BEFORE DELETE ON ent_cdt_travail
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_jamais_supprime();

CREATE TABLE IF NOT EXISTS ent_cdt_travail_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    titre TEXT NOT NULL,
    consigne TEXT NOT NULL,
    duree_estimee_min SMALLINT,
    echeance_le TIMESTAMPTZ,
    etat TEXT NOT NULL CHECK (etat IN ('brouillon', 'programme', 'publie', 'retire')),
    auteur_user_id UUID,
    auteur_agent_id UUID,
    motif TEXT,
    ancien_historique_id UUID UNIQUE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_travail_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_travail_version_uq UNIQUE (travail_id, version),
    CONSTRAINT ent_cdt_travail_version_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_travail_version_ajout_seul ON ent_cdt_travail_version;
CREATE TRIGGER ent_cdt_travail_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_travail_version
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_cdt_travail_format (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    format_code TEXT NOT NULL CHECK (format_code ~ '^[a-z0-9]{1,10}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (travail_id, format_code),
    CONSTRAINT ent_cdt_travail_format_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT
);

-- Public restreint (R-C25 : seulement des élèves du cours à la date, contrôlé par le code).
CREATE TABLE IF NOT EXISTS ent_cdt_travail_cible (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (travail_id, eleve_id),
    CONSTRAINT ent_cdt_travail_cible_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_travail_cible_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

-- Variante de consigne (aménagement appliqué sans jamais l'étiqueter : libellé INTERNE, R-C8).
CREATE TABLE IF NOT EXISTS ent_cdt_variante (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    libelle_interne TEXT NOT NULL CHECK (length(btrim(libelle_interne)) BETWEEN 1 AND 80),
    consigne TEXT NOT NULL CHECK (length(btrim(consigne)) > 0 AND length(consigne) <= 20000),
    duree_estimee_min SMALLINT CHECK (duree_estimee_min IS NULL OR duree_estimee_min BETWEEN 1 AND 600),
    echeance_le TIMESTAMPTZ,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_variante_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_variante_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_cdt_variante_eleve (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    variante_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    attribuee_par_user_id UUID,
    attribuee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retiree_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (variante_id, eleve_id),
    CONSTRAINT ent_cdt_variante_eleve_variante_fk FOREIGN KEY (etablissement_id, variante_id)
        REFERENCES ent_cdt_variante (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_variante_eleve_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

-- Prolongation individuelle : ajout seul, la ligne la plus récente fait foi ; motif CODÉ, jamais affiché tel quel.
CREATE TABLE IF NOT EXISTS ent_cdt_echeance_eleve (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    echeance_le TIMESTAMPTZ NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN ('amenagement', 'absence', 'technique', 'panne', 'autre')),
    par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_echeance_eleve_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_echeance_eleve_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_echeance_eleve_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_echeance_eleve_idx ON ent_cdt_echeance_eleve (etablissement_id, travail_id, eleve_id, cree_le DESC);
DROP TRIGGER IF EXISTS ent_cdt_echeance_eleve_ajout_seul ON ent_cdt_echeance_eleve;
CREATE TRIGGER ent_cdt_echeance_eleve_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_echeance_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_cdt_delai_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    demandeur_personne_id UUID,
    message TEXT CHECK (message IS NULL OR length(message) <= 1000),
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('ouverte', 'acceptee', 'refusee')),
    reponse TEXT CHECK (reponse IS NULL OR length(reponse) <= 1000),
    decide_par_user_id UUID,
    decide_le TIMESTAMPTZ,
    echeance_eleve_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_delai_demande_decision_chk CHECK ((etat = 'ouverte') = (decide_le IS NULL)),
    CONSTRAINT ent_cdt_delai_demande_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_delai_demande_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_delai_demande_personne_fk FOREIGN KEY (etablissement_id, demandeur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_delai_demande_echeance_fk FOREIGN KEY (etablissement_id, echeance_eleve_id)
        REFERENCES ent_cdt_echeance_eleve (etablissement_id, id) ON DELETE RESTRICT
);

-- Groupes de remise (un seul groupe actif par élève et par travail : déclencheur ci-dessous).
CREATE TABLE IF NOT EXISTS ent_cdt_groupe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 80),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_groupe_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_groupe_travail_id_uq UNIQUE (etablissement_id, travail_id, id),
    CONSTRAINT ent_cdt_groupe_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_cdt_groupe_membre (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    groupe_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    depuis TIMESTAMPTZ NOT NULL DEFAULT now(),
    jusqu_au TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (groupe_id, eleve_id, depuis),
    CONSTRAINT ent_cdt_groupe_membre_dates_chk CHECK (jusqu_au IS NULL OR jusqu_au >= depuis),
    CONSTRAINT ent_cdt_groupe_membre_groupe_fk FOREIGN KEY (etablissement_id, groupe_id)
        REFERENCES ent_cdt_groupe (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_groupe_membre_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

CREATE OR REPLACE FUNCTION ent_cdt_groupe_membre_unique() RETURNS trigger AS $$
BEGIN
    IF NEW.jusqu_au IS NULL AND EXISTS (
        SELECT 1 FROM ent_cdt_groupe_membre m
          JOIN ent_cdt_groupe g ON g.etablissement_id = m.etablissement_id AND g.id = m.groupe_id
          JOIN ent_cdt_groupe g2 ON g2.etablissement_id = g.etablissement_id AND g2.travail_id = g.travail_id
                                AND g2.id = NEW.groupe_id
         WHERE m.etablissement_id = NEW.etablissement_id AND m.eleve_id = NEW.eleve_id
           AND m.jusqu_au IS NULL
           AND NOT (m.groupe_id = NEW.groupe_id AND m.depuis = NEW.depuis)) THEN
        RAISE EXCEPTION 'ent_cdt_groupe_membre : élève déjà dans un groupe actif de ce travail' USING ERRCODE = '23505';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_groupe_membre_unique_trg ON ent_cdt_groupe_membre;
CREATE TRIGGER ent_cdt_groupe_membre_unique_trg BEFORE INSERT OR UPDATE ON ent_cdt_groupe_membre
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_groupe_membre_unique();

-- Note d'accompagnement d'une AESH (sur une entrée OU un travail, pour un élève accompagné).
CREATE TABLE IF NOT EXISTS ent_cdt_note_accompagnement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entree_id UUID,
    travail_id UUID,
    eleve_id UUID NOT NULL,
    agent_aesh_id UUID NOT NULL,
    texte TEXT NOT NULL CHECK (length(btrim(texte)) > 0 AND length(texte) <= 2000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_note_accompagnement_cible_chk CHECK ((entree_id IS NULL) <> (travail_id IS NULL)),
    CONSTRAINT ent_cdt_note_accompagnement_entree_fk FOREIGN KEY (etablissement_id, entree_id)
        REFERENCES ent_cdt_entree (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_note_accompagnement_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_note_accompagnement_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_note_accompagnement_agent_fk FOREIGN KEY (etablissement_id, agent_aesh_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_travail', 'ent_cdt_travail_version', 'ent_cdt_travail_format',
                           'ent_cdt_travail_cible', 'ent_cdt_variante', 'ent_cdt_variante_eleve',
                           'ent_cdt_echeance_eleve', 'ent_cdt_delai_demande', 'ent_cdt_groupe',
                           'ent_cdt_groupe_membre', 'ent_cdt_note_accompagnement']);
