-- Socle 21, lot V1 (suite) — 463 : états de période par classe (machine d'états en base), transitions
-- en ajout seul, rattrapages autorisés, instantanés d'arrêt immuables, et les ports « période par
-- élève » (R-V29, conception §9.4). Additif, rejouable. Aucune colonne JSON.
--
-- `eva_periode_etat` succède à `ent_note_periode` (022) : ce dernier reste la référence des lectures
-- historiques tant que M3 (V2–V8) n'a pas basculé les écrivains (vues de compatibilité : 469).

-- ── États de période, par classe ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS eva_periode_etat (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    classe_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'ouverte'
        CHECK (etat IN ('ouverte', 'notes_arretees', 'appreciations_closes', 'conseil_tenu', 'close')),
    -- Date d'arrêt EFFECTIVE de la classe (CE-41 : conseils étalés) ; NULL tant qu'on n'a pas arrêté :
    -- la date de la période (`ent_periode.date_arret_notes`) fait alors foi.
    date_arret_effective DATE,
    arretee_le TIMESTAMPTZ,
    -- Motif de la dernière transition, `geste` (une personne) | `auto` (date d'arrêt) | `reprise`.
    origine TEXT NOT NULL DEFAULT 'geste' CHECK (origine IN ('geste', 'auto', 'reprise')),
    modifie_par UUID,
    motif TEXT,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, periode_id, classe_id),
    CONSTRAINT eva_periode_etat_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_periode_etat_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES ent_classe (id, etablissement_id),
    CONSTRAINT eva_periode_etat_arret_chk CHECK (etat = 'ouverte' OR date_arret_effective IS NOT NULL OR origine = 'reprise')
);

CREATE TABLE IF NOT EXISTS eva_periode_transition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    classe_id UUID NOT NULL,
    etat_avant TEXT NOT NULL,
    etat_apres TEXT NOT NULL,
    -- Recul (réouverture) : motif obligatoire ; réouverture d'une période close : motif renforcé.
    reouverture BOOLEAN NOT NULL,
    motif TEXT,
    origine TEXT NOT NULL CHECK (origine IN ('geste', 'auto', 'reprise')),
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_transition_etat_fk FOREIGN KEY (etablissement_id, periode_id, classe_id)
        REFERENCES eva_periode_etat (etablissement_id, periode_id, classe_id),
    CONSTRAINT eva_transition_motif_chk CHECK (NOT reouverture OR (motif IS NOT NULL AND length(btrim(motif)) >= 3)),
    CONSTRAINT eva_transition_geste_chk CHECK (origine <> 'geste' OR par IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS eva_transition_idx ON eva_periode_transition (etablissement_id, periode_id, classe_id, le);

-- Rang d'un état dans la chaîne d'avancement.
CREATE OR REPLACE FUNCTION eva_etat_rang(p_etat text) RETURNS integer LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE p_etat
        WHEN 'ouverte' THEN 0 WHEN 'notes_arretees' THEN 1 WHEN 'appreciations_closes' THEN 2
        WHEN 'conseil_tenu' THEN 3 WHEN 'close' THEN 4 END
$$;

-- La machine d'états. Avancer : d'un cran seulement, jamais de saut. Reculer (réouverture, ★ direction) :
-- vers `ouverte` ou `notes_arretees`, motif d'au moins 3 caractères, 10 si la période était close.
-- Aucune autre transition n'existe : ni la route historique, ni un administratif, ni une session de code
-- ne peut rouvrir sans laisser de trace (défaut D2).
CREATE OR REPLACE FUNCTION ent_trg_eva_periode_etat() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    v_avant integer;
    v_apres integer;
    v_recul boolean;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_periode_etat : suppression physique interdite (R-V6)' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.etat <> 'ouverte' THEN
            RAISE EXCEPTION 'eva_periode_etat : une période naît ouverte (transitions contrôlées)' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.periode_id IS DISTINCT FROM OLD.periode_id
       OR NEW.classe_id IS DISTINCT FROM OLD.classe_id THEN
        RAISE EXCEPTION 'eva_periode_etat : la clé ne change pas' USING ERRCODE = '23514';
    END IF;
    NEW.modifie_le := now();
    IF NEW.etat = OLD.etat THEN
        -- La date d'arrêt effective n'est modifiable que tant que la classe n'est pas arrêtée.
        IF NEW.date_arret_effective IS DISTINCT FROM OLD.date_arret_effective AND OLD.etat <> 'ouverte' THEN
            RAISE EXCEPTION 'eva_periode_etat : date d''arrêt figée après l''arrêt (rouvrir d''abord)' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    v_avant := eva_etat_rang(OLD.etat);
    v_apres := eva_etat_rang(NEW.etat);
    v_recul := v_apres < v_avant;
    IF NOT v_recul AND v_apres <> v_avant + 1 THEN
        RAISE EXCEPTION 'eva_periode_etat : transition % -> % interdite (un cran à la fois)', OLD.etat, NEW.etat
            USING ERRCODE = '23514';
    END IF;
    IF v_recul THEN
        IF NEW.etat NOT IN ('ouverte', 'notes_arretees') THEN
            RAISE EXCEPTION 'eva_periode_etat : une réouverture revient à ouverte ou notes_arretees' USING ERRCODE = '23514';
        END IF;
        IF NEW.origine = 'auto' THEN
            RAISE EXCEPTION 'eva_periode_etat : aucune réouverture automatique' USING ERRCODE = '23514';
        END IF;
        IF NEW.motif IS NULL OR length(btrim(NEW.motif)) < 3 THEN
            RAISE EXCEPTION 'eva_periode_etat : motif obligatoire pour rouvrir une période (D2)' USING ERRCODE = '23514';
        END IF;
        IF OLD.etat = 'close' AND length(btrim(NEW.motif)) < 10 THEN
            RAISE EXCEPTION 'eva_periode_etat : motif renforcé (10 caractères) pour rouvrir une période close' USING ERRCODE = '23514';
        END IF;
        IF NEW.etat = 'ouverte' THEN
            NEW.arretee_le := NULL;
            -- F6 : la classe n'est plus arrêtée, la date d'arrêt effective n'a plus de sens ; la date de la
            -- période fait de nouveau foi, et un nouvel arrêt posera la sienne.
            NEW.date_arret_effective := NULL;
        END IF;
    ELSE
        IF OLD.etat = 'ouverte' THEN
            NEW.arretee_le := COALESCE(NEW.arretee_le, now());
            IF NEW.date_arret_effective IS NULL THEN
                NEW.date_arret_effective := ent_jour_etablissement(NEW.etablissement_id);
            END IF;
        END IF;
    END IF;
    IF NEW.origine = 'geste' AND NEW.modifie_par IS NULL THEN
        RAISE EXCEPTION 'eva_periode_etat : un changement d''état par un geste porte son auteur' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_periode_etat_apres() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.etat IS DISTINCT FROM OLD.etat THEN
        INSERT INTO eva_periode_transition (
            id, etablissement_id, periode_id, classe_id, etat_avant, etat_apres, reouverture, motif, origine, par)
        VALUES (
            gen_random_uuid(), NEW.etablissement_id, NEW.periode_id, NEW.classe_id, OLD.etat, NEW.etat,
            eva_etat_rang(NEW.etat) < eva_etat_rang(OLD.etat), NEW.motif, NEW.origine, NEW.modifie_par);
        -- Premier arrêt de la période : les règles de l'année sont figées (R-V21). Une année sans jeu de règles
        -- (défauts recommandés) reçoit sa ligne par défaut, puis la fige : un jeu posé plus tard naît figé.
        IF OLD.etat = 'ouverte' THEN
            INSERT INTO eva_reglage (id, etablissement_id, annee_id, fige_le)
            SELECT gen_random_uuid(), p.etablissement_id, p.annee_id, now()
              FROM ent_periode p
             WHERE p.id = NEW.periode_id AND p.etablissement_id = NEW.etablissement_id
            ON CONFLICT (etablissement_id, annee_id) DO UPDATE
                SET fige_le = COALESCE(eva_reglage.fige_le, EXCLUDED.fige_le);
        END IF;
    END IF;
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS eva_periode_etat_garde ON eva_periode_etat;
CREATE TRIGGER eva_periode_etat_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_periode_etat
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_periode_etat();
DROP TRIGGER IF EXISTS eva_periode_etat_transition ON eva_periode_etat;
CREATE TRIGGER eva_periode_etat_transition AFTER UPDATE ON eva_periode_etat
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_periode_etat_apres();
DROP TRIGGER IF EXISTS eva_transition_ajout_seul ON eva_periode_transition;
CREATE TRIGGER eva_transition_ajout_seul BEFORE UPDATE OR DELETE ON eva_periode_transition
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_transition_vidage ON eva_periode_transition;
CREATE TRIGGER eva_transition_vidage BEFORE TRUNCATE ON eva_periode_transition
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_periode_etat_vidage ON eva_periode_etat;
CREATE TRIGGER eva_periode_etat_vidage BEFORE TRUNCATE ON eva_periode_etat
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Une ligne `ouverte` pour (période, classe), créée au besoin (rejouable).
CREATE OR REPLACE FUNCTION eva_etat_assurer(p_etab uuid, p_periode uuid, p_classe uuid) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO eva_periode_etat (etablissement_id, periode_id, classe_id)
    VALUES (p_etab, p_periode, p_classe)
    ON CONFLICT (etablissement_id, periode_id, classe_id) DO NOTHING
$$;

-- ── Rattrapage autorisé après l'arrêt (R-V10) ─────────────────────────────────────
-- JOURNALISÉ, en ajout seul : chaque autorisation est une ligne numérotée (version) ; la dernière version fait
-- foi, et la révoquer revient à poser une date de fin passée. L'autorisation n'est valable que jusqu'à son
-- `jusqu_au` inclus, jour de l'établissement.
CREATE TABLE IF NOT EXISTS eva_rattrapage_autorise (
    id UUID PRIMARY KEY,
    evaluation_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version INTEGER NOT NULL CHECK (version >= 1),
    jusqu_au DATE NOT NULL,
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) >= 3),
    par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_rattrapage_uq UNIQUE (evaluation_id, eleve_id, version),
    CONSTRAINT eva_rattrapage_eval_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_rattrapage_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id)
);

CREATE OR REPLACE FUNCTION ent_trg_eva_rattrapage_version() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.version := COALESCE((SELECT max(r.version) FROM eva_rattrapage_autorise r
                              WHERE r.etablissement_id = NEW.etablissement_id
                                AND r.evaluation_id = NEW.evaluation_id AND r.eleve_id = NEW.eleve_id), 0) + 1;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_rattrapage_version ON eva_rattrapage_autorise;
CREATE TRIGGER eva_rattrapage_version BEFORE INSERT ON eva_rattrapage_autorise
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_rattrapage_version();
DROP TRIGGER IF EXISTS eva_rattrapage_ajout_seul ON eva_rattrapage_autorise;
CREATE TRIGGER eva_rattrapage_ajout_seul BEFORE UPDATE OR DELETE ON eva_rattrapage_autorise
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_rattrapage_vidage ON eva_rattrapage_autorise;
CREATE TRIGGER eva_rattrapage_vidage BEFORE TRUNCATE ON eva_rattrapage_autorise
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Le rattrapage est-il en vigueur (dernière version, date de fin non dépassée au jour de l'établissement) ?
CREATE OR REPLACE FUNCTION eva_rattrapage_en_vigueur(p_etab uuid, p_evaluation uuid, p_eleve uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT COALESCE((SELECT r.jusqu_au >= ent_jour_etablissement(p_etab)
                       FROM eva_rattrapage_autorise r
                      WHERE r.etablissement_id = p_etab AND r.evaluation_id = p_evaluation AND r.eleve_id = p_eleve
                      ORDER BY r.version DESC LIMIT 1), false)
$$;

-- ── Instantané d'arrêt (immuable) : ce que le conseil examine et ce que le bulletin imprime ──
-- Normalisé (D-036, §21.2) : une ligne par élève et par moyenne, jamais un bloc JSON. Les moyennes sont
-- gardées EXACTES (fraction) en plus de leur valeur affichée, pour que l'empreinte soit stable.
CREATE TABLE IF NOT EXISTS eva_instantane (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    classe_id UUID NOT NULL,
    periode_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    -- Raison de la nouvelle version (rattrapage autorisé, correction motivée…) ; NULL pour la version 1.
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) >= 3),
    pris_par UUID,
    pris_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_instantane_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES ent_classe (id, etablissement_id),
    CONSTRAINT eva_instantane_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_instantane_uq UNIQUE (etablissement_id, classe_id, periode_id, version),
    CONSTRAINT eva_instantane_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT eva_instantane_motif_chk CHECK (version = 1 OR motif IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS eva_instantane_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    instantane_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN
        ('moyenne_cours', 'moyenne_matiere', 'moyenne_generale', 'moyenne_annuelle', 'statistique_classe')),
    cours_id UUID,
    matiere_id UUID,
    -- Valeur exacte (fraction réduite) et valeur affichée ; NULL = aucune moyenne.
    valeur_num NUMERIC(40, 0),
    valeur_den NUMERIC(40, 0) CHECK (valeur_den IS NULL OR valeur_den > 0),
    valeur_affichee NUMERIC(9, 3),
    statut TEXT CHECK (statut IN ('en_attente', 'dispense', 'externe')),
    nb_comptees SMALLINT CHECK (nb_comptees IS NULL OR nb_comptees >= 0),
    nb_non_rendues SMALLINT CHECK (nb_non_rendues IS NULL OR nb_non_rendues >= 0),
    coef NUMERIC(6, 3),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_instantane_ligne_fk FOREIGN KEY (instantane_id, etablissement_id)
        REFERENCES eva_instantane (id, etablissement_id),
    CONSTRAINT eva_instantane_ligne_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_instantane_ligne_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_instantane_ligne_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id),
    CONSTRAINT eva_instantane_ligne_valeur_chk CHECK ((valeur_num IS NULL) = (valeur_den IS NULL))
);
CREATE INDEX IF NOT EXISTS eva_instantane_ligne_idx ON eva_instantane_ligne (etablissement_id, instantane_id, eleve_id);

DROP TRIGGER IF EXISTS eva_instantane_immuable ON eva_instantane;
CREATE TRIGGER eva_instantane_immuable BEFORE UPDATE OR DELETE ON eva_instantane
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_instantane_ligne_immuable ON eva_instantane_ligne;
CREATE TRIGGER eva_instantane_ligne_immuable BEFORE UPDATE OR DELETE ON eva_instantane_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_instantane_vidage ON eva_instantane;
CREATE TRIGGER eva_instantane_vidage BEFORE TRUNCATE ON eva_instantane
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_instantane_ligne_vidage ON eva_instantane_ligne;
CREATE TRIGGER eva_instantane_ligne_vidage BEFORE TRUNCATE ON eva_instantane_ligne
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- ── Ports « période par élève » (R-V29) ───────────────────────────────────────────
-- Classe de rattachement d'un élève à une date (affectation de classe en cours), puis période de la
-- PÉRIODISATION de cette classe à cette date, puis état de cette classe pour cette période. Un élève
-- sans affectation de classe à la date, ou une date hors de toute période, ne rend aucune ligne.
CREATE OR REPLACE FUNCTION eva_periode_eleve_jour(p_etab uuid, p_eleve uuid, p_jour date)
RETURNS TABLE (classe_id uuid, periode_id uuid, etat text, date_arret_effective date)
LANGUAGE sql STABLE AS $$
    SELECT a.classe_id, p.id,
           COALESCE(e.etat, 'ouverte'),
           COALESCE(e.date_arret_effective, p.date_arret_notes)
      FROM ent_affectation a
      JOIN LATERAL (SELECT ent_periodisation_pour_classe(a.etablissement_id, a.classe_id) AS pz) z ON true
      JOIN ent_periode p
        ON p.etablissement_id = a.etablissement_id AND p.periodisation_id = z.pz
       AND p.date_debut <= p_jour AND p.date_fin >= p_jour
      LEFT JOIN eva_periode_etat e
        ON e.etablissement_id = p.etablissement_id AND e.periode_id = p.id AND e.classe_id = a.classe_id
     WHERE a.etablissement_id = p_etab AND a.eleve_id = p_eleve AND a.classe_id IS NOT NULL
       AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
     ORDER BY a.date_debut DESC
     LIMIT 1
$$;

-- Même chose pour une évaluation : la date est celle de l'évaluation. Ne lit JAMAIS
-- `eva_evaluation.periode_id` (période indicative).
CREATE OR REPLACE FUNCTION eva_periode_eleve(p_etab uuid, p_evaluation uuid, p_eleve uuid)
RETURNS TABLE (classe_id uuid, periode_id uuid, etat text, date_arret_effective date)
LANGUAGE sql STABLE AS $$
    SELECT pe.classe_id, pe.periode_id, pe.etat, pe.date_arret_effective
      FROM eva_evaluation ev
      CROSS JOIN LATERAL eva_periode_eleve_jour(ev.etablissement_id, p_eleve, ev.date_eval) pe
     WHERE ev.etablissement_id = p_etab AND ev.id = p_evaluation
$$;

-- Version ensembliste pour la grille : tout le public de l'évaluation, son état et sa période,
-- cellule par cellule. `concerne` : l'élève est dans le public du cours à la date (socle 5) ET dans la
-- restriction éventuelle ; sinon `non_concerne` (jamais saisi). `hors_periode_ouverte` : l'état de sa
-- classe ne permet plus la saisie (sauf rattrapage autorisé).
CREATE OR REPLACE FUNCTION eva_grille_eleves(p_etab uuid, p_evaluation uuid)
RETURNS TABLE (eleve_id uuid, classe_id uuid, periode_id uuid, etat text, date_arret_effective date,
               saisie_ouverte boolean, rattrapage_autorise boolean)
LANGUAGE sql STABLE AS $$
    SELECT pu.eleve_id, pe.classe_id, pe.periode_id, pe.etat, pe.date_arret_effective,
           -- Sans classe ou sans période à la date : verrouillé (M8), sauf droit de correction (côté service).
           COALESCE(pe.etat = 'ouverte', false) AS saisie_ouverte,
           eva_rattrapage_en_vigueur(p_etab, p_evaluation, pu.eleve_id)
      FROM eva_evaluation ev
      CROSS JOIN LATERAL ent_public_cours_jour(ev.etablissement_id, ev.date_eval, ev.cours_id) pu
      LEFT JOIN LATERAL eva_periode_eleve_jour(ev.etablissement_id, pu.eleve_id, ev.date_eval) pe ON true
     WHERE ev.etablissement_id = p_etab AND ev.id = p_evaluation
       AND (NOT EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                         WHERE rr.etablissement_id = ev.etablissement_id AND rr.evaluation_id = ev.id)
            OR EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                        WHERE rr.etablissement_id = ev.etablissement_id AND rr.evaluation_id = ev.id
                          AND rr.eleve_id = pu.eleve_id))
$$;

-- ── Arrêt automatique à la date d'arrêt (M8, `eva_reglage.arret_auto`) ─────────────
-- Fait passer en `notes_arretees` (origine `auto`) chaque classe encore ouverte dont la date d'arrêt (effective,
-- sinon celle de la période) est atteinte au jour de l'établissement. IDEMPOTENT. Ne ré-arrête jamais une classe
-- que la direction a rouverte : dès qu'une transition existe pour (période, classe), la main est humaine.
-- Rend le nombre de classes arrêtées.
CREATE OR REPLACE FUNCTION eva_arret_auto(p_etab uuid, p_jour date DEFAULT NULL) RETURNS integer
LANGUAGE plpgsql AS $$
DECLARE
    v_jour date := COALESCE(p_jour, ent_jour_etablissement(p_etab));
    r RECORD;
    n integer := 0;
BEGIN
    FOR r IN
        SELECT p.id AS periode_id, c.id AS classe_id,
               COALESCE(e.date_arret_effective, p.date_arret_notes) AS date_arret
          FROM ent_periode p
          JOIN ent_classe c ON c.etablissement_id = p.etablissement_id AND c.annee_id = p.annee_id
           AND ent_periodisation_pour_classe(c.etablissement_id, c.id) = p.periodisation_id
          LEFT JOIN eva_reglage g ON g.etablissement_id = p.etablissement_id AND g.annee_id = p.annee_id
          LEFT JOIN eva_periode_etat e ON e.etablissement_id = p.etablissement_id AND e.periode_id = p.id
           AND e.classe_id = c.id
         WHERE p.etablissement_id = p_etab
           AND COALESCE(g.arret_auto, true)
           AND COALESCE(e.etat, 'ouverte') = 'ouverte'
           AND COALESCE(e.date_arret_effective, p.date_arret_notes) <= v_jour
           AND NOT EXISTS (SELECT 1 FROM eva_periode_transition t
                            WHERE t.etablissement_id = p.etablissement_id AND t.periode_id = p.id
                              AND t.classe_id = c.id)
         ORDER BY p.date_debut, c.id
    LOOP
        PERFORM eva_etat_assurer(p_etab, r.periode_id, r.classe_id);
        UPDATE eva_periode_etat
           SET etat = 'notes_arretees', origine = 'auto', date_arret_effective = r.date_arret,
               motif = 'arrêt automatique à la date d''arrêt'
         WHERE etablissement_id = p_etab AND periode_id = r.periode_id AND classe_id = r.classe_id
           AND etat = 'ouverte';
        n := n + 1;
    END LOOP;
    RETURN n;
END
$$;

SELECT ent_rls_poser(ARRAY[
    'eva_periode_etat', 'eva_periode_transition', 'eva_rattrapage_autorise',
    'eva_instantane', 'eva_instantane_ligne'
]);
