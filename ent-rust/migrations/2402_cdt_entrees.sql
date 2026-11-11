-- Socle 20, lot C1 — migration 2402 : entrées du cahier (une par séance datée et par cours), versions en ajout
-- seul, notions, contributions. Réf. : 20_CAHIER_DE_TEXTES.md §24.1, R-C1, R-C3, R-C5, R-C24.
-- Succède à `ent_seance_contenu` (150), conservée jusqu'à la bascule des écritures (C2) puis retirée (2419).

-- Cibles des clés composites (index uniques manquants, additifs).
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_seance_etab_id_cdt_uq ON ent_edt_seance (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_objet_etab_id_cdt_uq ON ent_objet (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_note_id_etab_cdt_uq ON ent_note (id, etablissement_id);

CREATE TABLE IF NOT EXISTS ent_cdt_entree (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID NOT NULL,
    seance_id UUID NOT NULL,
    annee_id UUID NOT NULL,
    titre TEXT CHECK (titre IS NULL OR length(btrim(titre)) BETWEEN 1 AND 200),
    -- Balisage maison : aucun HTML stocké ni rendu brut (R-C22).
    contenu TEXT NOT NULL CHECK (length(btrim(contenu)) > 0 AND length(contenu) <= 20000),
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'programmee', 'publiee', 'retiree')),
    publie_a TIMESTAMPTZ,
    -- Plage calme (R-C5) : rien n'est visible des élèves et des familles avant `visible_a`.
    visible_a TIMESTAMPTZ,
    retire_motif TEXT CHECK (retire_motif IS NULL OR length(btrim(retire_motif)) BETWEEN 3 AND 500),
    retire_par_user_id UUID,
    auteur_user_id UUID REFERENCES ent_user (id),
    auteur_agent_id UUID,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    renseignee_le TIMESTAMPTZ,
    progression_etape_id UUID,
    -- Reprise de `ent_seance_contenu` : identifiant d'origine, unique (reprise rejouable).
    ancien_id UUID UNIQUE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_entree_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_entree_seance_cours_uq UNIQUE (etablissement_id, cours_id, seance_id),
    CONSTRAINT ent_cdt_entree_retrait_chk CHECK ((retire_motif IS NULL) = (retire_par_user_id IS NULL)),
    CONSTRAINT ent_cdt_entree_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT,
    -- R-C3 : une séance qui porte une entrée n'est jamais supprimée.
    CONSTRAINT ent_cdt_entree_seance_fk FOREIGN KEY (etablissement_id, seance_id)
        REFERENCES ent_edt_seance (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_entree_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_entree_agent_fk FOREIGN KEY (etablissement_id, auteur_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_entree_cours_idx ON ent_cdt_entree (etablissement_id, cours_id, seance_id);
CREATE INDEX IF NOT EXISTS ent_cdt_entree_visible_idx ON ent_cdt_entree (etablissement_id, visible_a) WHERE etat <> 'retiree';

DROP TRIGGER IF EXISTS ent_cdt_entree_jamais_supprime ON ent_cdt_entree;
CREATE TRIGGER ent_cdt_entree_jamais_supprime BEFORE DELETE ON ent_cdt_entree
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_jamais_supprime();

-- Versions : ajout seul (valeur juridique : on relit ce qui a été écrit, R-C24).
CREATE TABLE IF NOT EXISTS ent_cdt_entree_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entree_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    titre TEXT,
    contenu TEXT NOT NULL,
    etat TEXT NOT NULL CHECK (etat IN ('brouillon', 'programmee', 'publiee', 'retiree')),
    auteur_user_id UUID,
    auteur_agent_id UUID,
    motif TEXT,
    -- Reprise de `ent_cdt_historique` : ligne d'origine, unique (reprise rejouable).
    ancien_historique_id UUID UNIQUE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_entree_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_entree_version_uq UNIQUE (entree_id, version),
    CONSTRAINT ent_cdt_entree_version_fk FOREIGN KEY (etablissement_id, entree_id)
        REFERENCES ent_cdt_entree (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_entree_version_ajout_seul ON ent_cdt_entree_version;
CREATE TRIGGER ent_cdt_entree_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_entree_version
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_cdt_entree_notion (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entree_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre >= 1),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (entree_id, ordre),
    CONSTRAINT ent_cdt_entree_notion_fk FOREIGN KEY (etablissement_id, entree_id)
        REFERENCES ent_cdt_entree (etablissement_id, id) ON DELETE RESTRICT
);

-- Contribution signée d'un intervenant (assistant, co-enseignant, intervenant extérieur) à une séance.
CREATE TABLE IF NOT EXISTS ent_cdt_contribution (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entree_id UUID NOT NULL,
    auteur_agent_id UUID,
    libelle_externe TEXT CHECK (libelle_externe IS NULL OR length(btrim(libelle_externe)) BETWEEN 1 AND 120),
    contenu TEXT NOT NULL CHECK (length(btrim(contenu)) > 0 AND length(contenu) <= 8000),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    retiree_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_contribution_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_contribution_auteur_chk CHECK (auteur_agent_id IS NOT NULL OR libelle_externe IS NOT NULL),
    CONSTRAINT ent_cdt_contribution_entree_fk FOREIGN KEY (etablissement_id, entree_id)
        REFERENCES ent_cdt_entree (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_contribution_agent_fk FOREIGN KEY (etablissement_id, auteur_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_cdt_contribution_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    contribution_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    contenu TEXT NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_contribution_version_uq UNIQUE (contribution_id, version),
    CONSTRAINT ent_cdt_contribution_version_fk FOREIGN KEY (etablissement_id, contribution_id)
        REFERENCES ent_cdt_contribution (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_contribution_version_ajout_seul ON ent_cdt_contribution_version;
CREATE TRIGGER ent_cdt_contribution_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_contribution_version
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_cdt_entree', 'ent_cdt_entree_version', 'ent_cdt_entree_notion',
                           'ent_cdt_contribution', 'ent_cdt_contribution_version']);
