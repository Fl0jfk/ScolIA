-- Socle 21, lot V1 (suite) — 462 : évaluations, évaluations communes, restrictions de public,
-- barème détaillé. Additif, rejouable ; clés étrangères composites (établissement, …).
--
-- `eva_evaluation` succède à `ent_devoir` (022, 180). Une évaluation vise UN COURS (socle 5), à
-- une DATE ; sa PÉRIODE se résout PAR ÉLÈVE (R-V29, conception §9.4) : `periode_id` n'est qu'une
-- période INDICATIVE (tri de l'écran du professeur), aucun contrôle ne la lit.

CREATE TABLE IF NOT EXISTS eva_evaluation_commune (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    date_eval DATE NOT NULL,
    bareme NUMERIC(7, 3) NOT NULL CHECK (bareme > 0 AND bareme <= 1000),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_commune_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT eva_commune_id_etab_uq UNIQUE (id, etablissement_id)
);

CREATE TABLE IF NOT EXISTS eva_evaluation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    cours_id UUID NOT NULL,
    -- Période INDICATIVE (classe majoritaire) : tri d'écran seulement (R-V29 / §9.4 point 4).
    periode_id UUID,
    date_eval DATE NOT NULL,
    titre TEXT NOT NULL CHECK (char_length(btrim(titre)) BETWEEN 1 AND 200),
    nature_id UUID,
    mode TEXT NOT NULL DEFAULT 'note' CHECK (mode IN ('note', 'positionnement')),
    -- Barème : obligatoire pour une évaluation notée, absent pour un positionnement (Q-EV13).
    bareme NUMERIC(7, 3) CHECK (bareme IS NULL OR (bareme > 0 AND bareme <= 1000)),
    coef NUMERIC(6, 3) NOT NULL DEFAULT 1 CHECK (coef >= 0 AND coef <= 100),
    comptage TEXT NOT NULL DEFAULT 'normal'
        CHECK (comptage IN ('normal', 'si_ameliore', 'ne_compte_pas', 'bonus')),
    hors_bareme_autorise BOOLEAN NOT NULL DEFAULT false,
    -- Évaluation de remplacement (R-01) : seule habilitée à porter un 0 après absence non justifiée.
    remplacement BOOLEAN NOT NULL DEFAULT false,
    commune_id UUID,
    -- Liens vers les socles 10 (séance) et 20 (travail) : posés par leurs lots (pas de clé ici,
    -- les tables n'appartiennent pas à ce socle).
    seance_id UUID,
    travail_id UUID,
    -- Sous-matière visée (NULL : la matière du cours elle-même).
    matiere_id UUID,
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('brouillon', 'ouverte', 'annulee')),
    motif_annulation TEXT,
    publication TEXT NOT NULL DEFAULT 'explicite' CHECK (publication IN ('explicite', 'programmee', 'a_l_arret')),
    publication_prevue_le TIMESTAMPTZ,
    publiee_le TIMESTAMPTZ,
    publiee_par UUID,
    annoncee BOOLEAN NOT NULL DEFAULT false,
    -- Reprise de l'existant (M2) : le devoir d'origine ; unique, sans clé (l'ancienne table disparaît en V13).
    origine_devoir_id UUID,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    -- Dernier auteur d'une modification des champs qui entrent dans le calcul, et son motif quand
    -- l'évaluation portait déjà des résultats ou que la période était arrêtée (M4, journal ci-dessous).
    modifie_par UUID,
    motif_correction TEXT CHECK (motif_correction IS NULL OR char_length(btrim(motif_correction)) >= 3),
    CONSTRAINT eva_eval_bareme_mode_chk CHECK ((mode = 'note') = (bareme IS NOT NULL)),
    CONSTRAINT eva_eval_annulation_chk CHECK ((etat = 'annulee') = (motif_annulation IS NOT NULL AND length(btrim(motif_annulation)) >= 3)),
    CONSTRAINT eva_eval_publication_chk CHECK (publication <> 'programmee' OR publication_prevue_le IS NOT NULL),
    CONSTRAINT eva_eval_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT eva_eval_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id),
    CONSTRAINT eva_eval_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_eval_nature_fk FOREIGN KEY (nature_id, etablissement_id)
        REFERENCES eva_nature (id, etablissement_id),
    CONSTRAINT eva_eval_commune_fk FOREIGN KEY (commune_id, etablissement_id)
        REFERENCES eva_evaluation_commune (id, etablissement_id),
    CONSTRAINT eva_eval_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_eval_id_etab_uq UNIQUE (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_eval_cours_idx ON eva_evaluation (etablissement_id, cours_id, date_eval);
CREATE INDEX IF NOT EXISTS eva_eval_periode_idx ON eva_evaluation (etablissement_id, periode_id) WHERE periode_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS eva_eval_origine_uq ON eva_evaluation (etablissement_id, origine_devoir_id)
    WHERE origine_devoir_id IS NOT NULL;

-- Journal des modifications d'une évaluation qui changent un calcul (ajout seul, M4).
CREATE TABLE IF NOT EXISTS eva_evaluation_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evaluation_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    bareme_avant NUMERIC(7, 3),
    bareme_apres NUMERIC(7, 3),
    coef_avant NUMERIC(6, 3) NOT NULL,
    coef_apres NUMERIC(6, 3) NOT NULL,
    comptage_avant TEXT NOT NULL,
    comptage_apres TEXT NOT NULL,
    mode_avant TEXT NOT NULL,
    mode_apres TEXT NOT NULL,
    hors_bareme_avant BOOLEAN NOT NULL,
    hors_bareme_apres BOOLEAN NOT NULL,
    date_avant DATE NOT NULL,
    date_apres DATE NOT NULL,
    -- Présent quand l'évaluation portait des résultats ou que la période était arrêtée.
    correction BOOLEAN NOT NULL,
    motif TEXT,
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_eval_histo_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_eval_histo_motif_chk CHECK (NOT correction OR (motif IS NOT NULL AND length(btrim(motif)) >= 3)),
    CONSTRAINT eva_eval_histo_uq UNIQUE (evaluation_id, version)
);

-- Une évaluation qui porte des résultats ne se supprime jamais (D7) : elle s'annule (état `annulee`).
CREATE OR REPLACE FUNCTION ent_trg_eva_evaluation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    v_cours RECORD;
    v_calcul boolean;
    v_figee boolean;
    v_motif text;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_evaluation : suppression physique interdite, annuler l''évaluation (R-V6)'
            USING ERRCODE = '23514';
    END IF;
    SELECT c.matiere_id INTO v_cours FROM ent_cours c WHERE c.id = NEW.cours_id AND c.etablissement_id = NEW.etablissement_id;
    -- F11 : la sous-matière visée est une sous-matière de la matière du cours.
    IF NEW.matiere_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM ent_matiere m
            WHERE m.id = NEW.matiere_id AND m.etablissement_id = NEW.etablissement_id
              AND m.parent_id IS NOT NULL AND m.parent_id = v_cours.matiere_id) THEN
        RAISE EXCEPTION 'eva_evaluation : la matière visée doit être une sous-matière de la matière du cours'
            USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        RETURN NEW;
    END IF;
    -- Le cours et l'année d'une évaluation ne changent pas ; déplacer = annuler et recréer (CE-51).
    IF NEW.cours_id IS DISTINCT FROM OLD.cours_id OR NEW.annee_id IS DISTINCT FROM OLD.annee_id THEN
        RAISE EXCEPTION 'eva_evaluation : le cours et l''année ne changent pas' USING ERRCODE = '23514';
    END IF;
    -- Une évaluation annulée reste annulée.
    IF OLD.etat = 'annulee' AND NEW.etat <> 'annulee' THEN
        RAISE EXCEPTION 'eva_evaluation : une évaluation annulée ne se rouvre pas' USING ERRCODE = '23514';
    END IF;
    -- Une évaluation publiée ne repasse pas en brouillon.
    IF OLD.publiee_le IS NOT NULL AND NEW.etat = 'brouillon' THEN
        RAISE EXCEPTION 'eva_evaluation : une évaluation publiée ne repasse pas en brouillon' USING ERRCODE = '23514';
    END IF;
    -- M4 : barème, coefficient, comptage, mode, hors barème et date entrent dans un calcul. Dès qu'un résultat
    -- existe ou qu'une période de son public est arrêtée, ils ne changent plus que par une correction motivée
    -- et journalisée.
    v_calcul := NEW.bareme IS DISTINCT FROM OLD.bareme OR NEW.coef IS DISTINCT FROM OLD.coef
             OR NEW.comptage IS DISTINCT FROM OLD.comptage OR NEW.mode IS DISTINCT FROM OLD.mode
             OR NEW.hors_bareme_autorise IS DISTINCT FROM OLD.hors_bareme_autorise
             OR NEW.date_eval IS DISTINCT FROM OLD.date_eval;
    v_motif := nullif(btrim(coalesce(NEW.motif_correction, '')), '');
    IF v_calcul THEN
        v_figee := EXISTS (SELECT 1 FROM eva_resultat r
                            WHERE r.etablissement_id = OLD.etablissement_id AND r.evaluation_id = OLD.id)
                   OR EXISTS (SELECT 1
                                FROM ent_public_cours_jour(OLD.etablissement_id, OLD.date_eval, OLD.cours_id) pu
                                CROSS JOIN LATERAL eva_periode_eleve_jour(OLD.etablissement_id, pu.eleve_id, OLD.date_eval) pe
                               WHERE pe.etat <> 'ouverte');
        IF v_figee AND (v_motif IS NULL OR length(v_motif) < 3) THEN
            RAISE EXCEPTION 'eva_evaluation : des résultats existent ou la période est arrêtée, correction motivée requise (M4)'
                USING ERRCODE = '23514';
        END IF;
        INSERT INTO eva_evaluation_historique (
            id, etablissement_id, evaluation_id, version, bareme_avant, bareme_apres, coef_avant, coef_apres,
            comptage_avant, comptage_apres, mode_avant, mode_apres, hors_bareme_avant, hors_bareme_apres,
            date_avant, date_apres, correction, motif, par)
        VALUES (gen_random_uuid(), OLD.etablissement_id, OLD.id, OLD.version + 1, OLD.bareme, NEW.bareme,
                OLD.coef, NEW.coef, OLD.comptage, NEW.comptage, OLD.mode, NEW.mode, OLD.hors_bareme_autorise,
                NEW.hors_bareme_autorise, OLD.date_eval, NEW.date_eval, v_figee, CASE WHEN v_figee THEN v_motif END,
                NEW.modifie_par);
    END IF;
    -- Le motif ne survit pas à l'écriture : une correction suivante doit porter le sien.
    NEW.motif_correction := NULL;
    NEW.version := OLD.version + 1;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_evaluation_garde ON eva_evaluation;
CREATE TRIGGER eva_evaluation_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_evaluation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_evaluation();
DROP TRIGGER IF EXISTS eva_evaluation_histo_ajout_seul ON eva_evaluation_historique;
CREATE TRIGGER eva_evaluation_histo_ajout_seul BEFORE UPDATE OR DELETE ON eva_evaluation_historique
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_evaluation_vidage ON eva_evaluation;
CREATE TRIGGER eva_evaluation_vidage BEFORE TRUNCATE ON eva_evaluation
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_evaluation_histo_vidage ON eva_evaluation_historique;
CREATE TRIGGER eva_evaluation_histo_vidage BEFORE TRUNCATE ON eva_evaluation_historique
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Restriction de public (demi-groupe, volontaires) : seuls ces élèves sont concernés.
CREATE TABLE IF NOT EXISTS eva_evaluation_restriction (
    evaluation_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    motif_code TEXT NOT NULL CHECK (motif_code IN ('demi_groupe', 'volontaire', 'amenagement', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (evaluation_id, eleve_id),
    CONSTRAINT eva_restriction_eval_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_restriction_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id)
);

-- Barème détaillé (exercices, critères) : la note est la somme des points (§8.9).
CREATE TABLE IF NOT EXISTS eva_bareme_item (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evaluation_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre >= 1),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    maximum NUMERIC(7, 3) NOT NULL CHECK (maximum > 0 AND maximum <= 1000),
    -- Exercice annulé (énoncé faux) : le barème se réduit de son maximum (CE-83).
    annule BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_bareme_item_eval_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_bareme_item_uq UNIQUE (evaluation_id, ordre),
    CONSTRAINT eva_bareme_item_id_etab_uq UNIQUE (id, etablissement_id)
);

SELECT ent_rls_poser(ARRAY[
    'eva_evaluation_commune', 'eva_evaluation', 'eva_evaluation_historique', 'eva_evaluation_restriction',
    'eva_bareme_item'
]);
