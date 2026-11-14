-- 2246 — Socle 12, lot AS4 (décompte versionné). Réf. : 12_ASSIDUITE.md §13.1, §13.2, §22.6, R-AS15 à R-AS17.
-- Le décompte en demi-journées est calculé à partir des ÉPISODES (AS3), selon la règle d'assiduité en
-- vigueur LE JOUR concerné (jamais rétroactive), le calendrier (vacances, fériés, fermetures, demi-journées
-- sans cours : socle 5) et le fuseau de l'établissement. Il est matérialisé (recalculable, idempotent), puis
-- figé à l'arrêt du mois ; les corrections postérieures sont listées à part (rectifications).
-- Additive et rejouable ; aucune colonne JSON (D-036) ; aucun BYTEA ; RLS par `ent_rls_poser`.

-- ── Demi-journées matérialisées : recalculables, avec leur explication codée ─────────────────────
CREATE TABLE IF NOT EXISTS ent_asd_demi_journee (
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    jour DATE NOT NULL,
    periode TEXT NOT NULL CHECK (periode IN ('matin', 'apres_midi')),
    minutes_manquees INTEGER NOT NULL CHECK (minutes_manquees >= 0),
    minutes_non_justifiees INTEGER NOT NULL CHECK (minutes_non_justifiees >= 0),
    seances_manquees INTEGER NOT NULL CHECK (seances_manquees >= 0),
    seances_non_justifiees INTEGER NOT NULL CHECK (seances_non_justifiees >= 0),
    -- Minutes de cours attendues dans la demi-journée (définition « toute la demi-journée »), sinon NULL.
    minutes_attendues INTEGER CHECK (minutes_attendues IS NULL OR minutes_attendues >= 0),
    -- Règle appliquée ce jour-là, identifiée par sa date de début (R-AS15) ; elle est lue à la date du
    -- jour, jamais à celle du calcul.
    regle_valide_du DATE NOT NULL,
    regle_id UUID,
    definition TEXT NOT NULL CHECK (definition IN ('une_seance', 'minutes', 'entiere')),
    seuil_minutes INTEGER NOT NULL CHECK (seuil_minutes BETWEEN 1 AND 600),
    coupure TIME NOT NULL,
    -- Origine de la coupure : grille du socle 5 (STR-35) ou, à défaut, repli sur la règle.
    coupure_source TEXT NOT NULL CHECK (coupure_source IN ('grille', 'regle')),
    -- Origine du calendrier : calendrier du socle 5, ou repli (aucune année scolaire définie : lundi au
    -- vendredi, matin et après-midi).
    calendrier_source TEXT NOT NULL CHECK (calendrier_source IN ('calendrier', 'repli')),
    compte BOOLEAN NOT NULL,
    non_justifiee BOOLEAN NOT NULL,
    -- Pourquoi la demi-journée compte ou ne compte pas (liste fermée, pas de texte libre).
    raison TEXT NOT NULL CHECK (raison IN (
        'compte', 'sous_le_seuil', 'demi_journee_incomplete', 'sans_cours', 'hors_annee', 'hors_scolarisation')),
    calcule_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, eleve_id, jour, periode),
    CONSTRAINT ent_asd_demi_journee_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_demi_journee_coherence_chk CHECK (
        (NOT non_justifiee OR compte)
        AND (raison = 'compte' OR NOT compte)
        AND (raison <> 'compte' OR compte))
);
CREATE INDEX IF NOT EXISTS ent_asd_demi_journee_jour_idx ON ent_asd_demi_journee (etablissement_id, jour);

-- ── Arrêt du mois : une ligne par établissement et par mois (le fait d'avoir arrêté) ─────────────
CREATE TABLE IF NOT EXISTS ent_asd_mois_cloture (
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    mois DATE NOT NULL CHECK (EXTRACT(DAY FROM mois) = 1),
    -- NULL : arrêt automatique (système, le 1er du mois suivant) ; sinon la personne qui l'a demandé.
    arrete_par UUID,
    eleves INTEGER NOT NULL CHECK (eleves >= 0),
    arrete_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, mois)
);

-- ── Décompte figé d'un élève pour un mois arrêté (ajout seul) ────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_asd_mois_arrete (
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    mois DATE NOT NULL CHECK (EXTRACT(DAY FROM mois) = 1),
    demi_journees INTEGER NOT NULL CHECK (demi_journees >= 0),
    non_justifiees INTEGER NOT NULL CHECK (non_justifiees >= 0 AND non_justifiees <= demi_journees),
    retards INTEGER NOT NULL DEFAULT 0 CHECK (retards >= 0),
    -- Règle en vigueur le dernier jour du mois (celle dont le seuil de signalement a servi).
    regle_valide_du DATE NOT NULL,
    regle_id UUID,
    seuil_signalement INTEGER NOT NULL CHECK (seuil_signalement BETWEEN 1 AND 62),
    arrete_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, eleve_id, mois),
    CONSTRAINT ent_asd_mois_arrete_cloture_fk FOREIGN KEY (etablissement_id, mois)
        REFERENCES ent_asd_mois_cloture (etablissement_id, mois),
    CONSTRAINT ent_asd_mois_arrete_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);

-- ── Rectifications postérieures à l'arrêt (R-AS16) : ce qui a changé depuis, listé à part ────────
CREATE TABLE IF NOT EXISTS ent_asd_rectification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    mois DATE NOT NULL CHECK (EXTRACT(DAY FROM mois) = 1),
    avant_demi_journees INTEGER NOT NULL CHECK (avant_demi_journees >= 0),
    avant_non_justifiees INTEGER NOT NULL CHECK (avant_non_justifiees >= 0),
    apres_demi_journees INTEGER NOT NULL CHECK (apres_demi_journees >= 0),
    apres_non_justifiees INTEGER NOT NULL CHECK (apres_non_justifiees >= 0),
    -- Cause connue du recalcul (liste fermée) ; `recalcul` si aucune cause n'a été donnée.
    cause TEXT NOT NULL DEFAULT 'recalcul' CHECK (cause IN (
        'recalcul', 'requalification', 'correction_appel', 'neutralisation', 'calendrier')),
    cause_ref_id UUID,
    -- Le franchissement du seuil de signalement a changé : la direction est alertée (§13.6).
    touche_signalement BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    vue_par UUID,
    vue_le TIMESTAMPTZ,
    CONSTRAINT ent_asd_rectification_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_rectification_cloture_fk FOREIGN KEY (etablissement_id, mois)
        REFERENCES ent_asd_mois_cloture (etablissement_id, mois),
    CONSTRAINT ent_asd_rectification_vue_chk CHECK ((vue_par IS NULL) = (vue_le IS NULL)),
    CONSTRAINT ent_asd_rectification_difference_chk CHECK (
        avant_demi_journees <> apres_demi_journees OR avant_non_justifiees <> apres_non_justifiees)
);
CREATE INDEX IF NOT EXISTS ent_asd_rectification_mois_idx ON ent_asd_rectification (etablissement_id, mois, cree_le);

-- Le mois arrêté est immuable (ajout seul) ; une rectification ne change plus, hors « vue par ».
CREATE OR REPLACE FUNCTION ent_asd_rectification_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_asd_rectification est en ajout seul (suppression interdite)';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id OR NEW.mois IS DISTINCT FROM OLD.mois
       OR NEW.avant_demi_journees IS DISTINCT FROM OLD.avant_demi_journees
       OR NEW.avant_non_justifiees IS DISTINCT FROM OLD.avant_non_justifiees
       OR NEW.apres_demi_journees IS DISTINCT FROM OLD.apres_demi_journees
       OR NEW.apres_non_justifiees IS DISTINCT FROM OLD.apres_non_justifiees
       OR NEW.cause IS DISTINCT FROM OLD.cause OR NEW.cause_ref_id IS DISTINCT FROM OLD.cause_ref_id
       OR NEW.touche_signalement IS DISTINCT FROM OLD.touche_signalement
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le
       OR (OLD.vue_par IS NOT NULL AND (NEW.vue_par IS DISTINCT FROM OLD.vue_par OR NEW.vue_le IS DISTINCT FROM OLD.vue_le)) THEN
        RAISE EXCEPTION 'ent_asd_rectification : seule la mention « vue par » peut être posée, une fois';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_rectification_garde_trg ON ent_asd_rectification;
CREATE TRIGGER ent_asd_rectification_garde_trg BEFORE UPDATE OR DELETE ON ent_asd_rectification
    FOR EACH ROW EXECUTE FUNCTION ent_asd_rectification_garde();

DROP TRIGGER IF EXISTS ent_asd_mois_arrete_ajout_seul ON ent_asd_mois_arrete;
CREATE TRIGGER ent_asd_mois_arrete_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_mois_arrete
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();
DROP TRIGGER IF EXISTS ent_asd_mois_cloture_ajout_seul ON ent_asd_mois_cloture;
CREATE TRIGGER ent_asd_mois_cloture_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_mois_cloture
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

-- ── Épisodes des absences de l'ancien circuit sans ligne de séance (DA-14) ───────────────────────
-- Une absence créée par l'ancienne route (famille, accueil : `ent_absence` à la journée, sans séance) n'a
-- aucun épisode. Cette fonction lui donne un épisode et ses demi-journées entières (lundi au vendredi,
-- matin et après-midi ; le calendrier du socle 5 retire ensuite vacances et fériés au calcul). Identifiants
-- déterministes comme la reprise de 2244 : appelée à chaque calcul, elle est idempotente. Plafond de 400
-- jours par absence (bornes d'une saisie ancienne).
CREATE OR REPLACE FUNCTION ent_asd_adopter_absences(p_etab uuid) RETURNS integer LANGUAGE plpgsql AS $$
DECLARE
    n integer;
BEGIN
    WITH orphelines AS (
        SELECT a.* FROM ent_absence a
         WHERE a.etablissement_id = p_etab
           AND NOT EXISTS (SELECT 1 FROM ent_asd_absence ea
                            WHERE ea.etablissement_id = a.etablissement_id AND ea.absence_legacy_id = a.id)
           AND NOT EXISTS (SELECT 1 FROM ent_absence_seance s
                            WHERE s.etablissement_id = a.etablissement_id AND s.absence_id = a.id)
    ), episodes AS (
        INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification,
                nature_code, motif_id, prevenue, source, absence_legacy_id, qualifie_par, qualifie_le, cree_le, modifie_le)
        SELECT md5('asd_absence:' || a.id::text)::uuid, a.etablissement_id, a.eleve_id, a.debut, a.fin,
               CASE a.statut WHEN 'a_traiter' THEN 'a_qualifier' WHEN 'annulee_present' THEN 'retire' ELSE 'qualifie' END,
               CASE a.statut WHEN 'justifiee' THEN 'excuse_valable' WHEN 'refusee' THEN 'non_valable' ELSE 'en_attente' END,
               m.nature_code, m.id, false, 'reprise', a.id,
               CASE WHEN a.statut IN ('justifiee', 'refusee') THEN a.decision_par_user_id END,
               CASE WHEN a.statut IN ('justifiee', 'refusee') THEN a.decision_at END,
               a.created_at, now()
          FROM orphelines a
          LEFT JOIN ent_asd_motif m ON m.etablissement_id = a.etablissement_id AND m.code = a.motif AND a.motif <> 'appel'
        ON CONFLICT DO NOTHING
        RETURNING id, etablissement_id, absence_legacy_id, etat
    ), seances AS (
        INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, periode,
                seance_source, regle_valide_du, source, retiree_le, retiree_cause)
        SELECT md5('asd_seance_dj:' || a.id::text || ':' || d::date::text || ':' || p.periode)::uuid,
               a.etablissement_id, md5('asd_absence:' || a.id::text)::uuid, a.eleve_id, d::date, 'demi_journee', p.periode,
               'ancien_circuit',
               (SELECT r.valide_du FROM ent_asd_regle r
                 WHERE r.etablissement_id = a.etablissement_id AND r.valide_du <= d::date
                 ORDER BY r.valide_du DESC LIMIT 1),
               'reprise',
               CASE WHEN a.statut = 'annulee_present' THEN now() END,
               CASE WHEN a.statut = 'annulee_present' THEN 'correction' END
          FROM orphelines a
          CROSS JOIN LATERAL generate_series(a.debut::timestamp, LEAST(a.fin, a.debut + 400)::timestamp, interval '1 day') d
          CROSS JOIN (VALUES ('matin'), ('apres_midi')) AS p(periode)
         WHERE EXTRACT(ISODOW FROM d) < 6
        ON CONFLICT DO NOTHING
        RETURNING 1
    )
    SELECT count(*) INTO n FROM episodes;
    RETURN n;
END
$$;

SELECT ent_rls_poser(ARRAY['ent_asd_demi_journee', 'ent_asd_mois_cloture', 'ent_asd_mois_arrete', 'ent_asd_rectification']);
