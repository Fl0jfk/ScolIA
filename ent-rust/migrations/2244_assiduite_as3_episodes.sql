-- 2244 — Socle 12, lot AS3 (épisodes d'absence). Réf. : 12_ASSIDUITE.md §8.3, §8.4, §22.4, R-AS3, R-AS5, R-AS8.
-- L'ÉPISODE est l'unité du motif, de la justification, de la qualification et de l'avis : une séance
-- n'appartient qu'à un épisode ; deux séances sont contiguës si aucune séance où l'élève était présent
-- ne les sépare. Les anciennes tables (`ent_absence`, `ent_absence_seance`) restent écrites par l'appel
-- (projection de transition, retrait prévu au lot AS16) : chaque absence ancienne est reprise en
-- épisode, et toute décision de l'ancien circuit (accepter, refuser, annuler vers « présent ») est
-- projetée sur ses épisodes par déclencheur. Additive, rejouable (E8), sans colonne JSON (D-036),
-- sans BYTEA ; RLS par `ent_rls_poser` (580).

CREATE TABLE IF NOT EXISTS ent_asd_absence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    jour_debut DATE NOT NULL,
    jour_fin DATE NOT NULL,
    etat TEXT NOT NULL DEFAULT 'a_qualifier' CHECK (etat IN ('ouvert', 'a_qualifier', 'qualifie', 'retire')),
    qualification TEXT NOT NULL DEFAULT 'en_attente'
        CHECK (qualification IN ('en_attente', 'reputee_legitime', 'excuse_valable', 'non_valable', 'sans_motif')),
    nature_code TEXT REFERENCES ref_asd_nature_motif (code),
    motif_id UUID,
    prevenue BOOLEAN NOT NULL DEFAULT false,
    source TEXT NOT NULL DEFAULT 'appel'
        CHECK (source IN ('appel', 'declaration', 'accueil', 'famille', 'vie_scolaire', 'reprise')),
    -- Libellé famille : liste FERMÉE de codes (D-023), jamais la note interne (R-AS22).
    libelle_famille_code TEXT CHECK (libelle_famille_code IS NULL OR libelle_famille_code IN (
        'absence_justifiee', 'absence_excusee', 'motif_non_retenu', 'motif_non_retenu_delai',
        'motif_non_retenu_piece', 'motif_non_communique')),
    -- Absence ancienne dont cet épisode est la lecture (un jour = une absence ancienne, DA-05 ; un
    -- jour peut compter plusieurs épisodes si l'élève a été présent entre deux séances). Sans clé
    -- étrangère : l'ancienne table disparaît à AS16.
    absence_legacy_id UUID,
    qualifie_par UUID,
    qualifie_le TIMESTAMPTZ,
    -- Contrôle de version (conflits de qualification concurrente).
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_absence_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_absence_jours_chk CHECK (jour_fin >= jour_debut),
    CONSTRAINT ent_asd_absence_etat_chk CHECK (
        (etat IN ('ouvert', 'a_qualifier') AND qualification = 'en_attente')
        OR (etat = 'qualifie' AND qualification <> 'en_attente')
        OR etat = 'retire'),
    CONSTRAINT ent_asd_absence_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_absence_motif_fk FOREIGN KEY (etablissement_id, motif_id)
        REFERENCES ent_asd_motif (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_absence_eleve_idx ON ent_asd_absence (etablissement_id, eleve_id, jour_debut, jour_fin);
CREATE INDEX IF NOT EXISTS ent_asd_absence_file_idx ON ent_asd_absence (etablissement_id, jour_debut)
    WHERE etat IN ('ouvert', 'a_qualifier');
CREATE INDEX IF NOT EXISTS ent_asd_absence_legacy_idx ON ent_asd_absence (etablissement_id, absence_legacy_id)
    WHERE absence_legacy_id IS NOT NULL;

-- R-AS5 : seules les cinq natures de L131-8 peuvent être « réputées légitimes » (garde SQL de dernier
-- recours ; le service refuse avant avec un message clair).
CREATE OR REPLACE FUNCTION ent_asd_absence_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.qualification = 'reputee_legitime' AND NOT EXISTS (
        SELECT 1 FROM ref_asd_nature_motif n WHERE n.code = NEW.nature_code AND n.legitime_l131_8) THEN
        RAISE EXCEPTION 'seules les natures de L131-8 peuvent être réputées légitimes' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_absence_garde_trg ON ent_asd_absence;
CREATE TRIGGER ent_asd_absence_garde_trg BEFORE INSERT OR UPDATE ON ent_asd_absence
    FOR EACH ROW EXECUTE FUNCTION ent_asd_absence_garde();

-- Une séance (ou une demi-journée historique) d'un épisode, horaires FIGÉS au constat.
CREATE TABLE IF NOT EXISTS ent_asd_absence_seance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    jour DATE NOT NULL,
    kind TEXT NOT NULL DEFAULT 'seance' CHECK (kind IN ('seance', 'demi_journee')),
    periode TEXT CHECK (periode IS NULL OR periode IN ('matin', 'apres_midi')),
    seance_ref UUID,
    seance_source TEXT NOT NULL DEFAULT 'edt_creneau',
    heure_debut TIME,
    heure_fin TIME,
    appel_id UUID,
    -- Règle d'assiduité en vigueur au constat, identifiée par sa date (R-AS15) : stable même si la
    -- version du jour est remplacée.
    regle_valide_du DATE,
    prevenue BOOLEAN NOT NULL DEFAULT false,
    source TEXT NOT NULL DEFAULT 'appel' CHECK (source IN ('appel', 'accueil', 'famille', 'vie_scolaire', 'reprise')),
    -- Constat défait (correction, séance annulée) : la ligne reste, elle ne compte plus (R-AS8, R-AS12).
    retiree_le TIMESTAMPTZ,
    retiree_cause TEXT CHECK (retiree_cause IS NULL OR retiree_cause IN ('correction', 'neutralisation', 'ancienne_absence_supprimee', 'scission')),
    retiree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_absence_seance_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_absence_seance_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_absence_seance_forme_chk CHECK (
        (kind = 'seance' AND heure_debut IS NOT NULL AND heure_fin IS NOT NULL AND heure_fin > heure_debut AND periode IS NULL)
        OR (kind = 'demi_journee' AND periode IS NOT NULL)),
    CONSTRAINT ent_asd_absence_seance_retiree_chk CHECK ((retiree_le IS NULL) = (retiree_cause IS NULL))
);
-- Une séance ⇒ un épisode (R-AS3) : une seule ligne ACTIVE par séance et par élève.
CREATE UNIQUE INDEX IF NOT EXISTS ent_asd_absence_seance_actif_uq
    ON ent_asd_absence_seance (etablissement_id, eleve_id, jour, seance_ref)
    WHERE kind = 'seance' AND retiree_le IS NULL AND seance_ref IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_asd_absence_seance_absence_idx ON ent_asd_absence_seance (etablissement_id, absence_id);
CREATE INDEX IF NOT EXISTS ent_asd_absence_seance_appel_idx ON ent_asd_absence_seance (etablissement_id, appel_id, eleve_id)
    WHERE appel_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_asd_absence_seance_jour_idx ON ent_asd_absence_seance (etablissement_id, eleve_id, jour);

-- Historique des états d'un épisode : ajout seul (fin de DA-10). La note interne y est conservée
-- (courte) ; elle n'est jamais lue de la famille ni du professeur (R-AS22, NJ-35).
CREATE TABLE IF NOT EXISTS ent_asd_absence_etat (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    etat TEXT NOT NULL CHECK (etat IN ('ouvert', 'a_qualifier', 'qualifie', 'retire')),
    qualification TEXT NOT NULL CHECK (qualification IN ('en_attente', 'reputee_legitime', 'excuse_valable', 'non_valable', 'sans_motif')),
    origine TEXT NOT NULL CHECK (origine IN (
        'appel', 'consolidation', 'qualification', 'requalification', 'masse', 'correction', 'neutralisation',
        'scission', 'fusion', 'projection', 'reprise', 'justificatif')),
    auteur_id UUID,
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code ~ '^[a-z0-9_]{2,40}$'),
    note_interne TEXT CHECK (note_interne IS NULL OR length(btrim(note_interne)) BETWEEN 1 AND 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_absence_etat_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_absence_etat_idx ON ent_asd_absence_etat (etablissement_id, absence_id, cree_le);
DROP TRIGGER IF EXISTS ent_asd_absence_etat_ajout_seul ON ent_asd_absence_etat;
CREATE TRIGGER ent_asd_absence_etat_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_absence_etat
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

-- Rapprochement épisode ↔ déclarations (situations) qui couvrent ses séances.
CREATE TABLE IF NOT EXISTS ent_asd_absence_declaration (
    absence_id UUID NOT NULL,
    situation_id UUID NOT NULL,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (absence_id, situation_id),
    CONSTRAINT ent_asd_absence_declaration_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_absence_declaration_situation_fk FOREIGN KEY (etablissement_id, situation_id)
        REFERENCES ent_asd_situation (etablissement_id, id)
);

-- ── Reprise des absences anciennes (004, 010, 011, 130) : une absence ancienne = un épisode ───────
-- Identifiants déterministes : la reprise est idempotente. Le statut ancien devient l'état :
-- a_traiter → à qualifier ; justifiee → excuse valable ; refusee → non valable ; annulee_present → retiré.
-- Les séances sont reprises à l'identique (même identifiant, mêmes horaires, mêmes demi-journées) : le
-- décompte des demi-journées ne change donc pas (critère de fin d'AS3). Une absence ancienne qui
-- couvrait plusieurs séances non contiguës reste un seul épisode (la contiguïté fine ne se rejoue pas
-- sur l'historique : le CPE peut scinder, AS-127).
INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification,
        nature_code, motif_id, prevenue, source, absence_legacy_id, qualifie_par, qualifie_le, cree_le, modifie_le)
SELECT md5('asd_absence:' || a.id::text)::uuid, a.etablissement_id, a.eleve_id, a.debut, a.fin,
       CASE a.statut WHEN 'a_traiter' THEN 'a_qualifier' WHEN 'annulee_present' THEN 'retire' ELSE 'qualifie' END,
       CASE a.statut WHEN 'justifiee' THEN 'excuse_valable' WHEN 'refusee' THEN 'non_valable' ELSE 'en_attente' END,
       m.nature_code, m.id,
       COALESCE((SELECT bool_or(s.prevenue) FROM ent_absence_seance s
                  WHERE s.etablissement_id = a.etablissement_id AND s.absence_id = a.id), false),
       'reprise', a.id,
       CASE WHEN a.statut IN ('justifiee', 'refusee') THEN a.decision_par_user_id END,
       CASE WHEN a.statut IN ('justifiee', 'refusee') THEN a.decision_at END,
       a.created_at, now()
  FROM ent_absence a
  JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
  LEFT JOIN ent_asd_motif m ON m.etablissement_id = a.etablissement_id AND m.code = a.motif AND a.motif <> 'appel'
ON CONFLICT DO NOTHING;

INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, periode, seance_ref,
        seance_source, heure_debut, heure_fin, appel_id, regle_valide_du, prevenue, source, cree_le)
SELECT s.id, s.etablissement_id, md5('asd_absence:' || s.absence_id::text)::uuid, s.eleve_id, s.jour, s.kind,
       s.periode, s.seance_ref, s.seance_source, s.heure_debut, s.heure_fin, s.appel_id,
       (SELECT r.valide_du FROM ent_asd_regle r
         WHERE r.etablissement_id = s.etablissement_id AND r.valide_du <= s.jour
         ORDER BY r.valide_du DESC LIMIT 1),
       s.prevenue, s.source, s.created_at
  FROM ent_absence_seance s
  JOIN ent_asd_absence ea ON ea.etablissement_id = s.etablissement_id AND ea.id = md5('asd_absence:' || s.absence_id::text)::uuid
ON CONFLICT DO NOTHING;

INSERT INTO ent_asd_absence_etat (id, etablissement_id, absence_id, etat, qualification, origine, auteur_id, cree_le)
SELECT md5('asd_absence_etat:reprise:' || ea.id::text)::uuid, ea.etablissement_id, ea.id, ea.etat, ea.qualification,
       'reprise', ea.qualifie_par, ea.cree_le
  FROM ent_asd_absence ea
 WHERE ea.source = 'reprise'
ON CONFLICT DO NOTHING;

-- ── Projection de l'ancien circuit vers les épisodes ──────────────────────────────────────────
-- L'ancien circuit (accepter, refuser, annuler vers « présent », justifier) écrit `ent_absence.statut`.
-- Tant qu'il existe, sa décision s'applique aux épisodes encore en attente de la même absence. Le
-- nouveau circuit (qualification) pose `ent.asd_projection = 'episode'` dans sa transaction avant de
-- réécrire l'ancien statut : le déclencheur ne repart pas dans l'autre sens.
CREATE OR REPLACE FUNCTION ent_asd_projeter_statut() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF COALESCE(current_setting('ent.asd_projection', true), '') = 'episode' THEN
        RETURN NULL;
    END IF;
    IF NEW.statut = 'annulee_present' THEN
        WITH maj AS (
            UPDATE ent_asd_absence SET etat = 'retire', version = version + 1, modifie_le = now()
             WHERE etablissement_id = NEW.etablissement_id AND absence_legacy_id = NEW.id AND etat <> 'retire'
            RETURNING id, etablissement_id, qualification)
        INSERT INTO ent_asd_absence_etat (id, etablissement_id, absence_id, etat, qualification, origine, auteur_id, motif_code)
        SELECT gen_random_uuid(), etablissement_id, id, 'retire', qualification, 'projection',
               NEW.decision_par_user_id, 'ancien_circuit' FROM maj;
        UPDATE ent_asd_absence_seance SET retiree_le = now(), retiree_cause = 'correction', retiree_par = NEW.decision_par_user_id
         WHERE etablissement_id = NEW.etablissement_id AND retiree_le IS NULL
           AND absence_id IN (SELECT id FROM ent_asd_absence WHERE etablissement_id = NEW.etablissement_id AND absence_legacy_id = NEW.id);
    ELSIF NEW.statut IN ('justifiee', 'refusee') THEN
        WITH maj AS (
            UPDATE ent_asd_absence
               SET etat = 'qualifie',
                   qualification = CASE NEW.statut WHEN 'justifiee' THEN 'excuse_valable' ELSE 'non_valable' END,
                   qualifie_par = NEW.decision_par_user_id, qualifie_le = now(),
                   version = version + 1, modifie_le = now()
             WHERE etablissement_id = NEW.etablissement_id AND absence_legacy_id = NEW.id
               AND etat IN ('ouvert', 'a_qualifier')
            RETURNING id, etablissement_id, qualification)
        INSERT INTO ent_asd_absence_etat (id, etablissement_id, absence_id, etat, qualification, origine, auteur_id, motif_code)
        SELECT gen_random_uuid(), etablissement_id, id, 'qualifie', qualification, 'projection',
               NEW.decision_par_user_id, 'ancien_circuit' FROM maj;
    END IF;
    RETURN NULL;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_projeter_statut_trg ON ent_absence;
CREATE TRIGGER ent_asd_projeter_statut_trg AFTER UPDATE OF statut ON ent_absence
    FOR EACH ROW WHEN (OLD.statut IS DISTINCT FROM NEW.statut)
    EXECUTE FUNCTION ent_asd_projeter_statut();

-- Suppression physique d'une absence ancienne (reprise, purge technique) : ses épisodes sont retirés,
-- jamais supprimés (R-AS8) ; leurs séances ne comptent plus.
CREATE OR REPLACE FUNCTION ent_asd_projeter_suppression() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH maj AS (
        UPDATE ent_asd_absence SET etat = 'retire', version = version + 1, modifie_le = now()
         WHERE etablissement_id = OLD.etablissement_id AND absence_legacy_id = OLD.id AND etat <> 'retire'
        RETURNING id, etablissement_id, qualification)
    INSERT INTO ent_asd_absence_etat (id, etablissement_id, absence_id, etat, qualification, origine, motif_code)
    SELECT gen_random_uuid(), etablissement_id, id, 'retire', qualification, 'projection', 'ancienne_absence_supprimee' FROM maj;
    UPDATE ent_asd_absence_seance SET retiree_le = now(), retiree_cause = 'ancienne_absence_supprimee'
     WHERE etablissement_id = OLD.etablissement_id AND retiree_le IS NULL
       AND absence_id IN (SELECT id FROM ent_asd_absence WHERE etablissement_id = OLD.etablissement_id AND absence_legacy_id = OLD.id);
    RETURN NULL;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_projeter_suppression_trg ON ent_absence;
CREATE TRIGGER ent_asd_projeter_suppression_trg AFTER DELETE ON ent_absence
    FOR EACH ROW EXECUTE FUNCTION ent_asd_projeter_suppression();

SELECT ent_rls_poser(ARRAY['ent_asd_absence', 'ent_asd_absence_seance', 'ent_asd_absence_etat', 'ent_asd_absence_declaration']);
