-- Socle 21, lot V4 — 475 : absences, dispenses, aménagements, rattrapages, évaluation de remplacement.
--
-- * Port 12 (absence à la séance de l'évaluation) : LECTURE de l'assiduité (`ent_asd_absence_seance`, séance de l'EDT du
--   cours ce jour-là), jamais d'écriture : l'appel propose, le professeur confirme.
-- * Port 34 (dispenses, inaptitudes) : EN SIMULATION tant que le socle 34 n'existe pas : `eva_dispense_port`, SANS motif
--   (R-V16). Le socle 34 remplacera la table ; la fonction `eva_dispense_a_la_date` reste le seul point de lecture.
-- * Port 35 (aménagements) : LECTURE de `ent_amenagement` (socle 2, catalogue fermé de codes, jamais de diagnostic).
--   Indisponibilité d'un AESH (socle 6) : en simulation (`eva_aesh_indisponibilite_port`).
-- * Rattrapages et évaluation de remplacement : `eva_rattrapage` (une ligne par convocation, reconvocation = nouvelle ligne
--   liée), `eva_rattrapage_historique` (ajout seul, alimenté par déclencheur). Le 0 n'est attribuable qu'à une convocation
--   de NATURE remplacement, en état « absent non justifié » (R-01, CE-29), par un acte tracé (colonnes `zero_attribue_*`).
-- * `ent_trg_eva_resultat` : pose la marque interne `amenage` (CE-35) ; garde de 474 (auteur obligatoire) conservée.
-- 461–474 intactes (règle 25). Additive et rejouable.

-- ── Port 34 (simulation) : dispenses et inaptitudes datées, sans motif ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS eva_dispense_port (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    -- NULL : tous les enseignements.
    matiere_id UUID,
    -- `dispense` : dispense d'enseignement (PPS, décision du recteur : évaluation neutralisée, CE-36) ;
    -- `inaptitude_totale` : EPS (CE-32, CE-33) ; `inaptitude_partielle` : le professeur adapte et note (CE-34), rien n'est pré-rempli.
    nature TEXT NOT NULL CHECK (nature IN ('dispense', 'inaptitude_totale', 'inaptitude_partielle')),
    du DATE NOT NULL,
    au DATE,
    saisi_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retiree_le TIMESTAMPTZ,
    retiree_par UUID,
    CONSTRAINT eva_dispense_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT eva_dispense_retrait_chk CHECK ((retiree_le IS NULL) = (retiree_par IS NULL)),
    CONSTRAINT eva_dispense_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_dispense_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_dispense_eleve_idx ON eva_dispense_port (etablissement_id, eleve_id, du);

-- Une dispense ne se réécrit pas : seul son retrait (une fois) change la ligne.
CREATE OR REPLACE FUNCTION ent_trg_eva_dispense_port() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_dispense_port : suppression physique interdite, retirer la dispense' USING ERRCODE = '23514';
    END IF;
    IF OLD.retiree_le IS NOT NULL THEN
        RAISE EXCEPTION 'eva_dispense_port : dispense déjà retirée' USING ERRCODE = '23514';
    END IF;
    IF NEW.eleve_id IS DISTINCT FROM OLD.eleve_id OR NEW.matiere_id IS DISTINCT FROM OLD.matiere_id
       OR NEW.nature IS DISTINCT FROM OLD.nature OR NEW.du IS DISTINCT FROM OLD.du OR NEW.au IS DISTINCT FROM OLD.au
       OR NEW.saisi_par IS DISTINCT FROM OLD.saisi_par THEN
        RAISE EXCEPTION 'eva_dispense_port : seul le retrait modifie une dispense' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_dispense_garde ON eva_dispense_port;
CREATE TRIGGER eva_dispense_garde BEFORE UPDATE OR DELETE ON eva_dispense_port
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_dispense_port();

-- ── Port 35 (simulation) : indisponibilité d'un AESH (socle 6) ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS eva_aesh_indisponibilite_port (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    aesh_user_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE NOT NULL,
    saisi_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_aesh_indispo_dates_chk CHECK (au >= du)
);
CREATE INDEX IF NOT EXISTS eva_aesh_indispo_idx ON eva_aesh_indisponibilite_port (etablissement_id, aesh_user_id, du);

-- ── Rattrapages et évaluation de remplacement ────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS eva_rattrapage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evaluation_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    -- `rattrapage` : l'évaluation manquée est repassée ; `remplacement` : l'évaluation est celle de remplacement de fin
    -- d'année (R-01) : seule nature qui peut mener à la note zéro.
    nature TEXT NOT NULL CHECK (nature IN ('rattrapage', 'remplacement')),
    -- Convocation collective (EVA-259) : plusieurs élèves, une séance, un identifiant.
    seance_id UUID NOT NULL,
    -- Reconvocation après une absence justifiée (CE-28) : la convocation précédente.
    precedent_id UUID,
    prevu_le DATE NOT NULL,
    heure TIME,
    lieu TEXT CHECK (lieu IS NULL OR char_length(btrim(lieu)) BETWEEN 1 AND 80),
    modalite TEXT NOT NULL DEFAULT 'ecrit' CHECK (modalite IN ('ecrit', 'oral', 'numerique', 'autre')),
    echeance DATE NOT NULL,
    etat TEXT NOT NULL DEFAULT 'convoque'
        CHECK (etat IN ('convoque', 'passe', 'absent_justifie', 'absent_non_justifie', 'annule')),
    motif_annulation TEXT CHECK (motif_annulation IS NULL OR char_length(btrim(motif_annulation)) BETWEEN 3 AND 500),
    zero_attribue_le TIMESTAMPTZ,
    zero_attribue_par UUID,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par UUID NOT NULL,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_rattrapage_dates_chk CHECK (echeance >= prevu_le),
    CONSTRAINT eva_rattrapage_annulation_chk CHECK ((etat = 'annule') = (motif_annulation IS NOT NULL)),
    -- R-01 / CE-29 : le zéro n'existe que pour une convocation de remplacement, restée « absent non justifié ».
    CONSTRAINT eva_rattrapage_zero_chk CHECK (
        (zero_attribue_le IS NULL AND zero_attribue_par IS NULL)
        OR (zero_attribue_le IS NOT NULL AND zero_attribue_par IS NOT NULL
            AND nature = 'remplacement' AND etat = 'absent_non_justifie')),
    CONSTRAINT eva_rattrapage_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT eva_rattrapage_eval_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_rattrapage_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_rattrapage_precedent_fk FOREIGN KEY (precedent_id, etablissement_id)
        REFERENCES eva_rattrapage (id, etablissement_id)
);
-- Une seule convocation EN COURS par évaluation et par élève.
CREATE UNIQUE INDEX IF NOT EXISTS eva_rattrapage_actif_uq
    ON eva_rattrapage (etablissement_id, evaluation_id, eleve_id) WHERE etat = 'convoque';
CREATE INDEX IF NOT EXISTS eva_rattrapage_eval_idx ON eva_rattrapage (etablissement_id, evaluation_id);
CREATE INDEX IF NOT EXISTS eva_rattrapage_eleve_idx ON eva_rattrapage (etablissement_id, eleve_id, prevu_le);

CREATE TABLE IF NOT EXISTS eva_rattrapage_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    rattrapage_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    version INTEGER NOT NULL,
    etat_avant TEXT,
    etat_apres TEXT NOT NULL,
    prevu_le_avant DATE,
    prevu_le_apres DATE NOT NULL,
    par UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_rattrapage_histo_fk FOREIGN KEY (rattrapage_id, etablissement_id)
        REFERENCES eva_rattrapage (id, etablissement_id),
    CONSTRAINT eva_rattrapage_histo_uq UNIQUE (rattrapage_id, version)
);

-- Machine d'états d'une convocation : convoque → passe | absent_justifie | absent_non_justifie | annule ;
-- convoque → convoque (déplacement : date, heure, lieu, modalité, échéance). Les états finaux ne bougent plus (une
-- reconvocation est une NOUVELLE ligne), sauf la pose, une fois, de `zero_attribue_*` sur « absent non justifié ».
CREATE OR REPLACE FUNCTION ent_trg_eva_rattrapage() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_rattrapage : suppression physique interdite, annuler la convocation' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.etat <> 'convoque' THEN
            RAISE EXCEPTION 'eva_rattrapage : une convocation naît « convoque »' USING ERRCODE = '23514';
        END IF;
        NEW.version := 1;
        RETURN NEW;
    END IF;
    IF NEW.evaluation_id IS DISTINCT FROM OLD.evaluation_id OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.nature IS DISTINCT FROM OLD.nature OR NEW.seance_id IS DISTINCT FROM OLD.seance_id
       OR NEW.precedent_id IS DISTINCT FROM OLD.precedent_id OR NEW.cree_par IS DISTINCT FROM OLD.cree_par
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'eva_rattrapage : évaluation, élève, nature et origine ne changent pas' USING ERRCODE = '23514';
    END IF;
    IF OLD.etat <> 'convoque' THEN
        IF OLD.etat = 'absent_non_justifie' AND OLD.zero_attribue_le IS NULL AND NEW.zero_attribue_le IS NOT NULL
           AND NEW.etat = OLD.etat AND NEW.prevu_le = OLD.prevu_le AND NEW.echeance = OLD.echeance THEN
            NEW.version := OLD.version + 1;
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'eva_rattrapage : une convocation terminée ne change plus (reconvoquer crée une nouvelle ligne)'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.zero_attribue_le IS NOT NULL THEN
        RAISE EXCEPTION 'eva_rattrapage : le zéro ne s''attribue qu''après une absence non justifiée constatée' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat IS NOT DISTINCT FROM OLD.etat AND NEW.prevu_le IS NOT DISTINCT FROM OLD.prevu_le
       AND NEW.heure IS NOT DISTINCT FROM OLD.heure AND NEW.lieu IS NOT DISTINCT FROM OLD.lieu
       AND NEW.modalite IS NOT DISTINCT FROM OLD.modalite AND NEW.echeance IS NOT DISTINCT FROM OLD.echeance THEN
        RETURN OLD;
    END IF;
    NEW.version := OLD.version + 1;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_rattrapage_garde ON eva_rattrapage;
CREATE TRIGGER eva_rattrapage_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_rattrapage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_rattrapage();

-- Historique en ajout seul, alimenté par la base (aucun écrivain ne peut l'oublier).
CREATE OR REPLACE FUNCTION ent_trg_eva_rattrapage_historique() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO eva_rattrapage_historique
        (id, etablissement_id, rattrapage_id, eleve_id, version, etat_avant, etat_apres, prevu_le_avant, prevu_le_apres, par)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.eleve_id, NEW.version,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.etat END, NEW.etat,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.prevu_le END, NEW.prevu_le, NEW.modifie_par);
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_rattrapage_historique_trg ON eva_rattrapage;
DROP TRIGGER IF EXISTS eva_rattrapage_historique_ins ON eva_rattrapage;
DROP TRIGGER IF EXISTS eva_rattrapage_historique_upd ON eva_rattrapage;
CREATE TRIGGER eva_rattrapage_historique_ins AFTER INSERT ON eva_rattrapage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_rattrapage_historique();
CREATE TRIGGER eva_rattrapage_historique_upd AFTER UPDATE ON eva_rattrapage
    FOR EACH ROW WHEN (OLD.version IS DISTINCT FROM NEW.version)
    EXECUTE FUNCTION ent_trg_eva_rattrapage_historique();

DROP TRIGGER IF EXISTS eva_rattrapage_histo_ajout_seul ON eva_rattrapage_historique;
CREATE TRIGGER eva_rattrapage_histo_ajout_seul BEFORE UPDATE OR DELETE ON eva_rattrapage_historique
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_rattrapage_histo_vidage ON eva_rattrapage_historique;
CREATE TRIGGER eva_rattrapage_histo_vidage BEFORE TRUNCATE ON eva_rattrapage_historique
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- ── Ports en lecture ─────────────────────────────────────────────────────────────────────────────────────────────────
-- Port 12 : l'élève était-il absent à la séance de ce cours ce jour-là, et l'absence est-elle justifiée ? NULL : pas
-- d'absence constatée (ou pas de séance du cours ce jour : une évaluation hors séance se saisit à la main).
CREATE OR REPLACE FUNCTION eva_absence_a_la_seance(p_etab uuid, p_eleve uuid, p_cours uuid, p_jour date)
RETURNS text LANGUAGE sql STABLE AS $$
    SELECT CASE WHEN a.etat = 'qualifie' AND a.qualification IN ('reputee_legitime', 'excuse_valable') THEN 'justifiee'
                WHEN a.etat = 'qualifie' THEN 'non_justifiee'
                ELSE 'en_attente' END
      FROM ent_asd_absence_seance sa
      JOIN ent_asd_absence a ON a.etablissement_id = sa.etablissement_id AND a.id = sa.absence_id AND a.etat <> 'retire'
     WHERE sa.etablissement_id = p_etab AND sa.eleve_id = p_eleve AND sa.jour = p_jour
       AND sa.kind = 'seance' AND sa.retiree_le IS NULL
       AND EXISTS (SELECT 1 FROM ent_edt_seance s
                    WHERE s.etablissement_id = p_etab AND s.cours_id = p_cours AND s.jour = p_jour
                      AND s.creneau_id = sa.seance_ref
                      AND s.etat NOT IN ('annulee', 'deplacee', 'neutralisee', 'non_assuree', 'retiree'))
     ORDER BY sa.heure_debut, sa.id
     LIMIT 1
$$;

-- Port 34 : dispense ou inaptitude en vigueur ce jour-là pour cet enseignement (la plus forte d'abord). SANS motif.
CREATE OR REPLACE FUNCTION eva_dispense_a_la_date(p_etab uuid, p_eleve uuid, p_matiere uuid, p_jour date)
RETURNS text LANGUAGE sql STABLE AS $$
    SELECT d.nature
      FROM eva_dispense_port d
     WHERE d.etablissement_id = p_etab AND d.eleve_id = p_eleve AND d.retiree_le IS NULL
       AND d.du <= p_jour AND (d.au IS NULL OR d.au >= p_jour)
       AND (d.matiere_id IS NULL OR d.matiere_id = p_matiere)
     ORDER BY CASE d.nature WHEN 'dispense' THEN 0 WHEN 'inaptitude_totale' THEN 1 ELSE 2 END, d.id
     LIMIT 1
$$;

-- Port 35 : un aménagement d'évaluation (portée examen ou les deux) est-il en vigueur ce jour-là ?
CREATE OR REPLACE FUNCTION eva_amenage_a_la_date(p_etab uuid, p_eleve uuid, p_jour date)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT EXISTS (SELECT 1 FROM ent_amenagement a
                    WHERE a.etablissement_id = p_etab AND a.eleve_id = p_eleve AND a.actif AND a.retire_at IS NULL
                      AND a.portee IN ('examen', 'les_deux')
                      AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour))
$$;

-- ── Résultats : la marque « aménagé » se pose en base (CE-35) ───────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION ent_trg_eva_resultat() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    ev RECORD;
    per RECORD;
    v_corr boolean := false;
    v_motif text := nullif(btrim(coalesce(NEW.motif_correction, '')), '');
    v_arretee boolean;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_resultat : suppression physique interdite, remplacer la valeur ou annuler (D7, R-V6)'
            USING ERRCODE = '23514';
    END IF;
    -- M1 (revue V3b) : tout ce qui change une note est ATTRIBUÉ. Un auteur NULL ne passe jamais (sinon l'attestation
    -- `app.eva_correcteur` vide, lue comme NULL, « égalait » l'auteur NULL : `NULL IS DISTINCT FROM NULL` est faux).
    IF NEW.saisi_par IS NULL THEN
        RAISE EXCEPTION 'eva_resultat : auteur obligatoire (toute écriture d''un résultat est attribuée)' USING ERRCODE = '23514';
    END IF;
    -- `reprise` n'est écrite que par la fonction de reprise (qui désactive ce déclencheur) : jamais ici.
    -- V3 : une ligne reprise (source `reprise`) doit pouvoir être modifiée par la grille : à sa première
    -- écriture ordinaire, sa source devient celle de l'écrivain (`saisie`, `import`…), une fois pour toutes ;
    -- jamais l'inverse, et la source `reprise` n'est posée qu'à l'INSERT de la fonction de reprise.
    IF NEW.source = 'reprise' AND (TG_OP = 'INSERT' OR OLD.source <> 'reprise') THEN
        RAISE EXCEPTION 'eva_resultat : la source reprise est réservée à la reprise de l''existant' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.source IS DISTINCT FROM OLD.source AND OLD.source <> 'reprise' THEN
        RAISE EXCEPTION 'eva_resultat : la source d''un résultat ne change pas' USING ERRCODE = '23514';
    END IF;
    SELECT e.bareme, e.hors_bareme_autorise, e.etat, e.mode, e.date_eval, e.cours_id
      INTO ev
      FROM eva_evaluation e
     WHERE e.id = NEW.evaluation_id AND e.etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'eva_resultat : évaluation introuvable' USING ERRCODE = '23503';
    END IF;
    IF TG_OP = 'UPDATE' AND (NEW.evaluation_id IS DISTINCT FROM OLD.evaluation_id
                             OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id) THEN
        RAISE EXCEPTION 'eva_resultat : l''évaluation et l''élève ne changent pas' USING ERRCODE = '23514';
    END IF;
    IF ev.mode <> 'note' THEN
        RAISE EXCEPTION 'eva_resultat : évaluation de positionnement, pas de résultat chiffré (socle 22)' USING ERRCODE = '23514';
    END IF;
    IF ev.etat = 'annulee' THEN
        RAISE EXCEPTION 'eva_resultat : évaluation annulée' USING ERRCODE = '23514';
    END IF;
    IF NEW.statut = 'note' AND NEW.valeur > ev.bareme AND NOT ev.hors_bareme_autorise THEN
        RAISE EXCEPTION 'eva_resultat : % sur % : vérifiez le barème', NEW.valeur, ev.bareme USING ERRCODE = '23514';
    END IF;
    -- Public : un élève hors du public du cours à la date n'a pas de résultat (non concerné, R-V2).
    IF NOT EXISTS (SELECT 1 FROM ent_public_cours_jour(NEW.etablissement_id, ev.date_eval, ev.cours_id) pu
                    WHERE pu.eleve_id = NEW.eleve_id) THEN
        RAISE EXCEPTION 'eva_resultat : élève non concerné par cette évaluation à sa date' USING ERRCODE = '23514';
    END IF;
    -- Évaluation à public restreint (demi-groupe, volontaires) : seuls les élèves visés ont un résultat.
    IF EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                WHERE rr.etablissement_id = NEW.etablissement_id AND rr.evaluation_id = NEW.evaluation_id)
       AND NOT EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                        WHERE rr.etablissement_id = NEW.etablissement_id AND rr.evaluation_id = NEW.evaluation_id
                          AND rr.eleve_id = NEW.eleve_id) THEN
        RAISE EXCEPTION 'eva_resultat : élève non concerné par cette évaluation (public restreint)' USING ERRCODE = '23514';
    END IF;
    -- Période de L'ÉLÈVE (R-V29) : classe de rattachement à la date de l'évaluation. Un élève sans classe ou
    -- sans période à cette date est VERROUILLÉ (M8) : seule une correction motivée écrit.
    SELECT * INTO per FROM eva_periode_eleve_jour(NEW.etablissement_id, NEW.eleve_id, ev.date_eval);
    v_arretee := NOT FOUND OR per.etat <> 'ouverte';
    IF v_arretee THEN
        -- Rattrapage autorisé (E4) : en vigueur à ce jour, et seulement sur une ligne absente ou restée sans
        -- note (absent, à rattraper, non rendu). Une note déjà saisie ne s'écrase pas par ce biais.
        IF eva_rattrapage_en_vigueur(NEW.etablissement_id, NEW.evaluation_id, NEW.eleve_id)
           AND (TG_OP = 'INSERT' OR OLD.statut IN ('absent', 'a_rattraper', 'non_rendu')) THEN
            v_corr := false;
        ELSE
            -- Après l'arrêt : seulement une correction motivée (la direction), jamais une saisie ordinaire.
            IF v_motif IS NULL OR length(v_motif) < 3 THEN
                RAISE EXCEPTION 'eva_resultat : notes arrêtées pour cet élève, correction motivée requise (R-V10)'
                    USING ERRCODE = '23514';
            END IF;
            -- F2 (revue V3) : un motif non vide ne suffit plus. L'écriture après l'arrêt doit venir d'un porteur de
            -- `notes.corriger` : l'application l'atteste dans la transaction (`app.eva_correcteur` = l'auteur, posée
            -- par `ecrire_resultat_tx` APRÈS contrôle du droit). Tout autre écrivain (code oublié, SQL direct) est
            -- refusé ; la reprise de l'existant désactive ce déclencheur et n'est pas concernée.
            IF nullif(current_setting('app.eva_correcteur', true), '') IS NULL
               OR current_setting('app.eva_correcteur', true) IS DISTINCT FROM NEW.saisi_par::text THEN
                RAISE EXCEPTION 'eva_resultat : une écriture après l''arrêt exige un porteur de notes.corriger (attestation absente)'
                    USING ERRCODE = '23514';
            END IF;
            v_corr := true;
        END IF;
    END IF;
    -- V4 (CE-35, R-19) : marque INTERNE « aménagé » (jamais imprimée, jamais dans la vue des publications) : un aménagement
    -- d'évaluation (port 35, `ent_amenagement` portée examen / les deux) était en vigueur à la date de l'évaluation.
    IF NEW.statut = 'note' AND (TG_OP = 'INSERT' OR OLD.statut IS DISTINCT FROM NEW.statut OR OLD.valeur IS DISTINCT FROM NEW.valeur) THEN
        NEW.amenage := eva_amenage_a_la_date(NEW.etablissement_id, NEW.eleve_id, ev.date_eval);
    ELSIF NEW.statut <> 'note' THEN
        NEW.amenage := false;
    END IF;
    NEW.derniere_ecriture_correction := v_corr;
    NEW.dernier_motif_correction := CASE WHEN v_corr THEN v_motif END;
    NEW.motif_correction := NULL;
    IF TG_OP = 'UPDATE' THEN
        NEW.version := OLD.version + 1;
        NEW.saisi_le := now();
        -- Aucune écriture sans effet : la ligne reste telle quelle, la version aussi (rien n'est journalisé).
        IF NEW.statut IS NOT DISTINCT FROM OLD.statut AND NEW.valeur IS NOT DISTINCT FROM OLD.valeur
           AND NEW.rattrapage_le IS NOT DISTINCT FROM OLD.rattrapage_le
           AND NEW.amenage IS NOT DISTINCT FROM OLD.amenage
           AND NEW.commentaire_eleve IS NOT DISTINCT FROM OLD.commentaire_eleve
           AND NEW.note_privee IS NOT DISTINCT FROM OLD.note_privee THEN
            RETURN OLD;
        END IF;
    END IF;
    RETURN NEW;
END
$$;

SELECT ent_rls_poser(ARRAY['eva_dispense_port', 'eva_aesh_indisponibilite_port', 'eva_rattrapage', 'eva_rattrapage_historique']);
