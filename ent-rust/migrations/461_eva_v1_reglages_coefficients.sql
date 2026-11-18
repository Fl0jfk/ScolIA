-- Socle 21, lot V1 « modèle et reprise » (plage 461–479 de docs/socles/MIGRATIONS.md ; 460 = V0).
-- 461 : règles de calcul de l'établissement (paramétrables, historisées), coefficients de matière
-- et de sous-matière, natures d'évaluation. Additif, rejouable. Aucune colonne JSON, aucun BYTEA,
-- aucun tableau (D-036). Chaque table : `etablissement_id NOT NULL`, clés étrangères composites,
-- RLS posée par `ent_rls_poser` (580).
--
-- Les règles de calcul ne sont jamais codées en dur : chaque point où Charlemagne et ScolIA
-- pourraient diverger (étape et mode d'arrondi, pondération des barèmes, notes bonus, non rendu,
-- sous-matières, moyenne générale, moyenne annuelle, médiane, seuil de statistiques) est une
-- colonne de `eva_reglage` (docs/socles/21_REGLES_CALCUL_CHARLEMAGNE.md).

-- ── Cibles des clés composites ────────────────────────────────────────────────────
CREATE UNIQUE INDEX IF NOT EXISTS ent_periode_id_etab_eva_uq ON ent_periode (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_mef_id_etab_eva_uq ON ent_mef (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_niveau_id_etab_eva_uq ON ent_niveau (id, etablissement_id);

-- ── Règles de calcul (un jeu par établissement et par année) ──────────────────────
CREATE TABLE IF NOT EXISTS eva_reglage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- Échelle des moyennes et des notes ramenées (20 : usage courant ; 10, 100 possibles).
    echelle NUMERIC(7, 3) NOT NULL DEFAULT 20 CHECK (echelle > 0 AND echelle <= 1000),
    -- Pas de saisie des notes (0,01 : centième ; 0,5 : demi-point).
    pas_saisie NUMERIC(4, 3) NOT NULL DEFAULT 0.01 CHECK (pas_saisie > 0 AND pas_saisie <= 1),
    -- Arrondi d'AFFICHAGE (et d'export par défaut).
    arrondi_precision TEXT NOT NULL DEFAULT 'centieme'
        CHECK (arrondi_precision IN ('millieme', 'centieme', 'dixieme', 'quart', 'demi_point', 'unite')),
    arrondi_mode TEXT NOT NULL DEFAULT 'demi_haut'
        CHECK (arrondi_mode IN ('demi_haut', 'demi_bas', 'demi_pair', 'superieur', 'inferieur')),
    -- Arrondis d'ÉTAPE (avant l'affichage) : NULL = calcul exact (recommandation).
    arrondi_note_ramenee_precision TEXT
        CHECK (arrondi_note_ramenee_precision IN ('millieme', 'centieme', 'dixieme', 'quart', 'demi_point', 'unite')),
    arrondi_note_ramenee_mode TEXT NOT NULL DEFAULT 'demi_haut'
        CHECK (arrondi_note_ramenee_mode IN ('demi_haut', 'demi_bas', 'demi_pair', 'superieur', 'inferieur')),
    arrondi_sous_matiere_precision TEXT
        CHECK (arrondi_sous_matiere_precision IN ('millieme', 'centieme', 'dixieme', 'quart', 'demi_point', 'unite')),
    arrondi_sous_matiere_mode TEXT NOT NULL DEFAULT 'demi_haut'
        CHECK (arrondi_sous_matiere_mode IN ('demi_haut', 'demi_bas', 'demi_pair', 'superieur', 'inferieur')),
    arrondi_matiere_precision TEXT
        CHECK (arrondi_matiere_precision IN ('millieme', 'centieme', 'dixieme', 'quart', 'demi_point', 'unite')),
    arrondi_matiere_mode TEXT NOT NULL DEFAULT 'demi_haut'
        CHECK (arrondi_matiere_mode IN ('demi_haut', 'demi_bas', 'demi_pair', 'superieur', 'inferieur')),
    arrondi_periode_precision TEXT
        CHECK (arrondi_periode_precision IN ('millieme', 'centieme', 'dixieme', 'quart', 'demi_point', 'unite')),
    arrondi_periode_mode TEXT NOT NULL DEFAULT 'demi_haut'
        CHECK (arrondi_periode_mode IN ('demi_haut', 'demi_bas', 'demi_pair', 'superieur', 'inferieur')),
    -- Notes de barèmes différents : ramenées une à une, ou points cumulés.
    ponderation TEXT NOT NULL DEFAULT 'notes_ramenees'
        CHECK (ponderation IN ('notes_ramenees', 'points_cumules')),
    -- Un travail non rendu : exclu (défaut) ou compté 0. Une ABSENCE n'est jamais un zéro (R-V5,
    -- contrainte 460 sur `ent_eva_regle`) : aucun réglage ici ne le permet.
    non_rendu TEXT NOT NULL DEFAULT 'exclu' CHECK (non_rendu IN ('exclu', 'zero')),
    -- Notes bonus : ignorées (défaut), comme une note normale, ou points au-dessus d'un seuil.
    bonus TEXT NOT NULL DEFAULT 'ignore' CHECK (bonus IN ('ignore', 'comme_normale', 'points_au_dessus')),
    bonus_seuil NUMERIC(7, 3) CHECK (bonus_seuil IS NULL OR bonus_seuil >= 0),
    sous_matieres TEXT NOT NULL DEFAULT 'moyenne_des_moyennes'
        CHECK (sous_matieres IN ('moyenne_des_moyennes', 'toutes_les_notes')),
    generale TEXT NOT NULL DEFAULT 'ponderee' CHECK (generale IN ('ponderee', 'non_ponderee')),
    methode_annuelle TEXT NOT NULL DEFAULT 'moyenne_periodes'
        CHECK (methode_annuelle IN ('moyenne_periodes', 'toutes_evaluations')),
    plafonner_echelle BOOLEAN NOT NULL DEFAULT true,
    mediane_paire TEXT NOT NULL DEFAULT 'moyenne_des_deux'
        CHECK (mediane_paire IN ('moyenne_des_deux', 'inferieure', 'superieure')),
    -- Statistiques montrées aux familles : seuil d'effectif (R-V12) et affichage.
    seuil_effectif_stats SMALLINT NOT NULL DEFAULT 5 CHECK (seuil_effectif_stats BETWEEN 1 AND 100),
    stats_familles BOOLEAN NOT NULL DEFAULT true,
    mediane_familles BOOLEAN NOT NULL DEFAULT false,
    delai_publication_max_jours SMALLINT CHECK (delai_publication_max_jours IS NULL OR delai_publication_max_jours BETWEEN 1 AND 365),
    arret_auto BOOLEAN NOT NULL DEFAULT true,
    seuils_alertes TEXT NOT NULL DEFAULT 'normal' CHECK (seuils_alertes IN ('souple', 'normal', 'strict')),
    -- Posé au premier arrêt de période : échelle et méthode annuelle ne changent plus (R-V21).
    fige_le TIMESTAMPTZ,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    modifie_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_reglage_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT eva_reglage_uq UNIQUE (etablissement_id, annee_id),
    CONSTRAINT eva_reglage_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT eva_reglage_bonus_chk CHECK ((bonus = 'points_au_dessus') = (bonus_seuil IS NOT NULL))
);

-- Historique des règles (ajout seul) : une version par changement, pour expliquer un écart avec
-- un autre logiciel « à la date où le calcul a été fait » (R-V23).
CREATE TABLE IF NOT EXISTS eva_reglage_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    reglage_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    echelle NUMERIC(7, 3) NOT NULL,
    pas_saisie NUMERIC(4, 3) NOT NULL,
    arrondi_precision TEXT NOT NULL,
    arrondi_mode TEXT NOT NULL,
    arrondi_note_ramenee_precision TEXT,
    arrondi_sous_matiere_precision TEXT,
    arrondi_matiere_precision TEXT,
    arrondi_periode_precision TEXT,
    ponderation TEXT NOT NULL,
    non_rendu TEXT NOT NULL,
    bonus TEXT NOT NULL,
    sous_matieres TEXT NOT NULL,
    generale TEXT NOT NULL,
    methode_annuelle TEXT NOT NULL,
    mediane_paire TEXT NOT NULL,
    seuil_effectif_stats SMALLINT NOT NULL,
    modifie_par UUID,
    pris_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_reglage_histo_fk FOREIGN KEY (reglage_id, etablissement_id)
        REFERENCES eva_reglage (id, etablissement_id) ON DELETE CASCADE,
    CONSTRAINT eva_reglage_histo_uq UNIQUE (reglage_id, version)
);

CREATE OR REPLACE FUNCTION ent_trg_eva_reglage() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        -- Échelle et méthode annuelle figées dès le premier arrêt de période (R-V21).
        IF OLD.fige_le IS NOT NULL
           AND (NEW.echelle IS DISTINCT FROM OLD.echelle
                OR NEW.methode_annuelle IS DISTINCT FROM OLD.methode_annuelle) THEN
            RAISE EXCEPTION 'règles figées pour l''année : échelle et méthode annuelle ne changent plus (R-V21)'
                USING ERRCODE = '23514';
        END IF;
        IF OLD.fige_le IS NOT NULL AND NEW.fige_le IS NULL THEN
            RAISE EXCEPTION 'règles figées : le gel ne se retire pas' USING ERRCODE = '23514';
        END IF;
        NEW.version := OLD.version + 1;
        NEW.modifie_le := now();
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_reglage_historique() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO eva_reglage_historique (
        id, etablissement_id, reglage_id, version, echelle, pas_saisie, arrondi_precision, arrondi_mode,
        arrondi_note_ramenee_precision, arrondi_sous_matiere_precision, arrondi_matiere_precision,
        arrondi_periode_precision, ponderation, non_rendu, bonus, sous_matieres, generale,
        methode_annuelle, mediane_paire, seuil_effectif_stats, modifie_par)
    VALUES (
        gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.version, NEW.echelle, NEW.pas_saisie,
        NEW.arrondi_precision, NEW.arrondi_mode, NEW.arrondi_note_ramenee_precision,
        NEW.arrondi_sous_matiere_precision, NEW.arrondi_matiere_precision, NEW.arrondi_periode_precision,
        NEW.ponderation, NEW.non_rendu, NEW.bonus, NEW.sous_matieres, NEW.generale,
        NEW.methode_annuelle, NEW.mediane_paire, NEW.seuil_effectif_stats, NEW.modifie_par);
    RETURN NULL;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_ajout_seul() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : ajout seul (modification et suppression interdites)', TG_TABLE_NAME
        USING ERRCODE = '23514';
END
$$;

DROP TRIGGER IF EXISTS eva_reglage_garde ON eva_reglage;
CREATE TRIGGER eva_reglage_garde BEFORE INSERT OR UPDATE ON eva_reglage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_reglage();
DROP TRIGGER IF EXISTS eva_reglage_histo ON eva_reglage;
CREATE TRIGGER eva_reglage_histo AFTER INSERT OR UPDATE ON eva_reglage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_reglage_historique();
-- Un jeu de règles ne se supprime pas (l'historique le référence).
CREATE OR REPLACE FUNCTION ent_trg_eva_pas_de_suppression() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : suppression physique interdite (R-V6)', TG_TABLE_NAME USING ERRCODE = '23514';
END
$$;
DROP TRIGGER IF EXISTS eva_reglage_sans_suppression ON eva_reglage;
CREATE TRIGGER eva_reglage_sans_suppression BEFORE DELETE ON eva_reglage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_pas_de_suppression();
DROP TRIGGER IF EXISTS eva_reglage_histo_ajout_seul ON eva_reglage_historique;
CREATE TRIGGER eva_reglage_histo_ajout_seul BEFORE UPDATE OR DELETE ON eva_reglage_historique
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- ── Coefficients de matière (la cible la plus précise gagne : classe > MEF > niveau > défaut) ──
CREATE TABLE IF NOT EXISTS eva_coefficient_matiere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    matiere_id UUID NOT NULL,
    classe_id UUID,
    mef_id UUID,
    niveau_id UUID,
    defaut BOOLEAN NOT NULL DEFAULT false,
    -- Coefficient 0 : la matière ne pèse rien (équivalent « hors moyenne » pondérée).
    coef NUMERIC(6, 3) NOT NULL CHECK (coef >= 0 AND coef <= 100),
    hors_moyenne_generale BOOLEAN NOT NULL DEFAULT false,
    valable_du DATE NOT NULL,
    valable_au DATE,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_coef_matiere_cible_chk CHECK (
        (classe_id IS NOT NULL)::int + (mef_id IS NOT NULL)::int + (niveau_id IS NOT NULL)::int + defaut::int = 1),
    CONSTRAINT eva_coef_matiere_dates_chk CHECK (valable_au IS NULL OR valable_au >= valable_du),
    CONSTRAINT eva_coef_matiere_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT eva_coef_matiere_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_coef_matiere_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES ent_classe (id, etablissement_id),
    CONSTRAINT eva_coef_matiere_mef_fk FOREIGN KEY (mef_id, etablissement_id)
        REFERENCES ent_mef (id, etablissement_id),
    CONSTRAINT eva_coef_matiere_niveau_fk FOREIGN KEY (niveau_id, etablissement_id)
        REFERENCES ent_niveau (id, etablissement_id),
    -- Pas de chevauchement de validité pour une même matière et une même cible (CE-53 : un
    -- changement est DATÉ, jamais rétroactif).
    CONSTRAINT eva_coef_matiere_sans_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, annee_id WITH =, matiere_id WITH =,
        (coalesce(classe_id, mef_id, niveau_id, '00000000-0000-0000-0000-000000000000'::uuid)) WITH =,
        (CASE WHEN classe_id IS NOT NULL THEN 'c' WHEN mef_id IS NOT NULL THEN 'm'
              WHEN niveau_id IS NOT NULL THEN 'n' ELSE 'd' END) WITH =,
        daterange(valable_du, valable_au, '[]') WITH &&)
);
CREATE INDEX IF NOT EXISTS eva_coef_matiere_idx ON eva_coefficient_matiere (etablissement_id, annee_id, matiere_id);

-- Coefficient d'une sous-matière dans sa matière parente (défaut 1 : aucune ligne).
CREATE TABLE IF NOT EXISTS eva_coefficient_sous_matiere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    matiere_id UUID NOT NULL,
    coef NUMERIC(6, 3) NOT NULL CHECK (coef >= 0 AND coef <= 100),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_coef_sous_matiere_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT eva_coef_sous_matiere_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_coef_sous_matiere_uq UNIQUE (etablissement_id, annee_id, matiere_id)
);

-- Natures d'évaluation (DS, interrogation, DM, oral, TP, examen blanc…) : données de l'établissement.
CREATE TABLE IF NOT EXISTS eva_nature (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z0-9_]{1,40}$'),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 80),
    ordre SMALLINT NOT NULL DEFAULT 0,
    archivee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_nature_uq UNIQUE (etablissement_id, code),
    CONSTRAINT eva_nature_id_etab_uq UNIQUE (id, etablissement_id)
);

SELECT ent_rls_poser(ARRAY[
    'eva_reglage', 'eva_reglage_historique', 'eva_coefficient_matiere',
    'eva_coefficient_sous_matiere', 'eva_nature'
]);
