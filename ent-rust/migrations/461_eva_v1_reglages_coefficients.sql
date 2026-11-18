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
    -- Plancher DUR de 3 (en dessous, la moyenne de la classe révèle les notes des autres) ; 5 par défaut.
    seuil_effectif_stats SMALLINT NOT NULL DEFAULT 5 CHECK (seuil_effectif_stats BETWEEN 3 AND 100),
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
    arrondi_note_ramenee_mode TEXT NOT NULL,
    arrondi_sous_matiere_precision TEXT,
    arrondi_sous_matiere_mode TEXT NOT NULL,
    arrondi_matiere_precision TEXT,
    arrondi_matiere_mode TEXT NOT NULL,
    arrondi_periode_precision TEXT,
    arrondi_periode_mode TEXT NOT NULL,
    ponderation TEXT NOT NULL,
    non_rendu TEXT NOT NULL,
    bonus TEXT NOT NULL,
    bonus_seuil NUMERIC(7, 3),
    sous_matieres TEXT NOT NULL,
    generale TEXT NOT NULL,
    methode_annuelle TEXT NOT NULL,
    plafonner_echelle BOOLEAN NOT NULL,
    mediane_paire TEXT NOT NULL,
    seuil_effectif_stats SMALLINT NOT NULL,
    stats_familles BOOLEAN NOT NULL,
    mediane_familles BOOLEAN NOT NULL,
    figee BOOLEAN NOT NULL DEFAULT false,
    modifie_par UUID,
    pris_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_reglage_histo_fk FOREIGN KEY (reglage_id, etablissement_id)
        REFERENCES eva_reglage (id, etablissement_id) ON DELETE CASCADE,
    CONSTRAINT eva_reglage_histo_uq UNIQUE (reglage_id, version)
);

CREATE OR REPLACE FUNCTION ent_trg_eva_reglage() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        -- Un jeu de règles posé APRÈS un premier arrêt de période naît figé (R-V21).
        IF NEW.fige_le IS NULL AND EXISTS (
               SELECT 1 FROM eva_periode_etat e
                 JOIN ent_periode p ON p.id = e.periode_id AND p.etablissement_id = e.etablissement_id
                WHERE e.etablissement_id = NEW.etablissement_id AND p.annee_id = NEW.annee_id
                  AND e.etat <> 'ouverte') THEN
            NEW.fige_le := now();
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' THEN
        -- Dès le premier arrêt de période, TOUTES les règles qui entrent dans un calcul sont figées : un
        -- recalcul d'une période arrêtée utilise donc les règles en vigueur à la date d'arrêt (R-V21, R-V23).
        -- Restent modifiables : seuil et affichage des statistiques aux familles, délai de publication,
        -- arrêt automatique, seuils d'alerte.
        IF OLD.fige_le IS NOT NULL
           AND (NEW.echelle IS DISTINCT FROM OLD.echelle
                OR NEW.pas_saisie IS DISTINCT FROM OLD.pas_saisie
                OR NEW.arrondi_precision IS DISTINCT FROM OLD.arrondi_precision
                OR NEW.arrondi_mode IS DISTINCT FROM OLD.arrondi_mode
                OR NEW.arrondi_note_ramenee_precision IS DISTINCT FROM OLD.arrondi_note_ramenee_precision
                OR NEW.arrondi_note_ramenee_mode IS DISTINCT FROM OLD.arrondi_note_ramenee_mode
                OR NEW.arrondi_sous_matiere_precision IS DISTINCT FROM OLD.arrondi_sous_matiere_precision
                OR NEW.arrondi_sous_matiere_mode IS DISTINCT FROM OLD.arrondi_sous_matiere_mode
                OR NEW.arrondi_matiere_precision IS DISTINCT FROM OLD.arrondi_matiere_precision
                OR NEW.arrondi_matiere_mode IS DISTINCT FROM OLD.arrondi_matiere_mode
                OR NEW.arrondi_periode_precision IS DISTINCT FROM OLD.arrondi_periode_precision
                OR NEW.arrondi_periode_mode IS DISTINCT FROM OLD.arrondi_periode_mode
                OR NEW.ponderation IS DISTINCT FROM OLD.ponderation
                OR NEW.non_rendu IS DISTINCT FROM OLD.non_rendu
                OR NEW.bonus IS DISTINCT FROM OLD.bonus
                OR NEW.bonus_seuil IS DISTINCT FROM OLD.bonus_seuil
                OR NEW.sous_matieres IS DISTINCT FROM OLD.sous_matieres
                OR NEW.generale IS DISTINCT FROM OLD.generale
                OR NEW.methode_annuelle IS DISTINCT FROM OLD.methode_annuelle
                OR NEW.plafonner_echelle IS DISTINCT FROM OLD.plafonner_echelle
                OR NEW.mediane_paire IS DISTINCT FROM OLD.mediane_paire) THEN
            RAISE EXCEPTION 'règles figées pour l''année : les règles de calcul ne changent plus après le premier arrêt (R-V21)'
                USING ERRCODE = '23514';
        END IF;
        IF OLD.fige_le IS NOT NULL AND NEW.fige_le IS DISTINCT FROM OLD.fige_le THEN
            RAISE EXCEPTION 'règles figées : le gel ne se retire ni ne se modifie' USING ERRCODE = '23514';
        END IF;
        IF NEW.annee_id IS DISTINCT FROM OLD.annee_id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id THEN
            RAISE EXCEPTION 'eva_reglage : l''année ne change pas' USING ERRCODE = '23514';
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
        arrondi_note_ramenee_precision, arrondi_note_ramenee_mode, arrondi_sous_matiere_precision,
        arrondi_sous_matiere_mode, arrondi_matiere_precision, arrondi_matiere_mode,
        arrondi_periode_precision, arrondi_periode_mode, ponderation, non_rendu, bonus, bonus_seuil,
        sous_matieres, generale, methode_annuelle, plafonner_echelle, mediane_paire, seuil_effectif_stats,
        stats_familles, mediane_familles, figee, modifie_par)
    VALUES (
        gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.version, NEW.echelle, NEW.pas_saisie,
        NEW.arrondi_precision, NEW.arrondi_mode, NEW.arrondi_note_ramenee_precision, NEW.arrondi_note_ramenee_mode,
        NEW.arrondi_sous_matiere_precision, NEW.arrondi_sous_matiere_mode, NEW.arrondi_matiere_precision,
        NEW.arrondi_matiere_mode, NEW.arrondi_periode_precision, NEW.arrondi_periode_mode,
        NEW.ponderation, NEW.non_rendu, NEW.bonus, NEW.bonus_seuil, NEW.sous_matieres, NEW.generale,
        NEW.methode_annuelle, NEW.plafonner_echelle, NEW.mediane_paire, NEW.seuil_effectif_stats,
        NEW.stats_familles, NEW.mediane_familles, NEW.fige_le IS NOT NULL, NEW.modifie_par);
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
-- TRUNCATE : même convention que 521 et 650 (déclencheur d'instruction, aucune dérogation par réglage de
-- session). Un TRUNCATE ... CASCADE sur une table parente est refusé dès qu'il touche une table protégée.
DROP TRIGGER IF EXISTS eva_reglage_vidage ON eva_reglage;
CREATE TRIGGER eva_reglage_vidage BEFORE TRUNCATE ON eva_reglage
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_reglage_histo_vidage ON eva_reglage_historique;
CREATE TRIGGER eva_reglage_histo_vidage BEFORE TRUNCATE ON eva_reglage_historique
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

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

-- Coefficient d'une sous-matière dans sa matière parente (défaut 1 : aucune ligne). DATÉ comme celui de
-- la matière : un changement est une nouvelle ligne, jamais une modification (R-V23).
CREATE TABLE IF NOT EXISTS eva_coefficient_sous_matiere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    matiere_id UUID NOT NULL,
    coef NUMERIC(6, 3) NOT NULL CHECK (coef >= 0 AND coef <= 100),
    valable_du DATE NOT NULL,
    valable_au DATE,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_coef_sous_matiere_dates_chk CHECK (valable_au IS NULL OR valable_au >= valable_du),
    CONSTRAINT eva_coef_sous_matiere_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT eva_coef_sous_matiere_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_coef_sous_matiere_sans_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, annee_id WITH =, matiere_id WITH =,
        daterange(valable_du, valable_au, '[]') WITH &&)
);

-- Des résultats existent-ils pour cette matière (ou sous-matière visée) à partir d'une date ? Un
-- coefficient ne s'applique jamais à des résultats déjà calculés (R-V23, CE-53).
-- (plpgsql : les tables `eva_evaluation` et `eva_resultat` sont créées par les migrations suivantes.)
CREATE OR REPLACE FUNCTION eva_coef_touche_des_resultats(p_etab uuid, p_annee uuid, p_matiere uuid,
                                                         p_sous boolean, p_depuis date)
RETURNS boolean LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1
          FROM eva_evaluation ev
          JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
         WHERE ev.etablissement_id = p_etab AND ev.annee_id = p_annee AND ev.date_eval >= p_depuis
           AND (CASE WHEN p_sous THEN ev.matiere_id = p_matiere ELSE co.matiere_id = p_matiere END)
           AND EXISTS (SELECT 1 FROM eva_resultat r
                        WHERE r.etablissement_id = ev.etablissement_id AND r.evaluation_id = ev.id));
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_coefficient() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    v_sous boolean := (TG_TABLE_NAME = 'eva_coefficient_sous_matiere');
    v_jour date;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION '% : suppression physique interdite, poser une nouvelle ligne datée (R-V23)', TG_TABLE_NAME
            USING ERRCODE = '23514';
    END IF;
    v_jour := ent_jour_etablissement(NEW.etablissement_id);
    IF TG_OP = 'INSERT' THEN
        IF NEW.valable_du < v_jour
           AND eva_coef_touche_des_resultats(NEW.etablissement_id, NEW.annee_id, NEW.matiere_id, v_sous, NEW.valable_du) THEN
            RAISE EXCEPTION '% : un coefficient n''est jamais rétroactif, des résultats existent depuis le % (R-V23)',
                TG_TABLE_NAME, NEW.valable_du USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    -- UPDATE : seule la date de fin peut changer (clore une ligne), jamais le coefficient ni la cible.
    IF to_jsonb(NEW) - 'valable_au' IS DISTINCT FROM to_jsonb(OLD) - 'valable_au' THEN
        RAISE EXCEPTION '% : une ligne datée ne se modifie pas, clore la ligne et en poser une nouvelle (R-V23)', TG_TABLE_NAME
            USING ERRCODE = '23514';
    END IF;
    IF NEW.valable_au IS DISTINCT FROM OLD.valable_au
       AND (NEW.valable_au IS NULL OR NEW.valable_au < v_jour)
       AND eva_coef_touche_des_resultats(NEW.etablissement_id, NEW.annee_id, NEW.matiere_id, v_sous,
                                         COALESCE(NEW.valable_au + 1, OLD.valable_du)) THEN
        RAISE EXCEPTION '% : la fin de validité ne peut pas toucher des résultats existants (R-V23)', TG_TABLE_NAME
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_coef_matiere_garde ON eva_coefficient_matiere;
CREATE TRIGGER eva_coef_matiere_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_coefficient_matiere
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_coefficient();
DROP TRIGGER IF EXISTS eva_coef_sous_matiere_garde ON eva_coefficient_sous_matiere;
CREATE TRIGGER eva_coef_sous_matiere_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_coefficient_sous_matiere
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_coefficient();
DROP TRIGGER IF EXISTS eva_coef_matiere_vidage ON eva_coefficient_matiere;
CREATE TRIGGER eva_coef_matiere_vidage BEFORE TRUNCATE ON eva_coefficient_matiere
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_coef_sous_matiere_vidage ON eva_coefficient_sous_matiere;
CREATE TRIGGER eva_coef_sous_matiere_vidage BEFORE TRUNCATE ON eva_coefficient_sous_matiere
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

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
