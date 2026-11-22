-- Socle 21, lot V3e + V5 « parcours » (additive, rejouable ; 461–476 versées : aucune n'est modifiée, règle 25).
-- * V3e-1 : le motif d'une écriture reste au journal (`eva_resultat_historique.motif`) même hors correction après l'arrêt.
-- * V3e-5 : UNE fonction pour « la période d'un jour » (`ent_periode_choisir`), lue par `eva_periode_eleve_jour`, par
--   `ent_periode_de_note_pour_classe` et par `eva_periode_dans` ; départage documenté et total des périodes qui se
--   recouvrent (la périodisation historique n'est pas protégée par la contrainte d'exclusion de 335, qui ne vise que
--   l'origine `service` : y imposer l'exclusion romprait des données existantes).
-- * V3e-4 : prolongation d'échéance TRACÉE d'un rattrapage (échéance et motif au journal des convocations).
-- * V5 : résultats externes (CE-02, CE-06, CE-07) : date, décision de compter (tracée), publication, retrait ; garde en base.

-- ── V3e-1 ─────
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
    -- V3e (item 1) : le motif fourni avec l'écriture est CONSERVÉ au journal même hors correction (zéro de remplacement
    -- attribué par le professeur en période ouverte, note d'un rattrapage après l'échéance). `motif_correction` est une
    -- entrée remise à NULL à chaque écriture : un motif ancien ne sert jamais à l'écriture suivante.
    NEW.dernier_motif_correction := v_motif;
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

-- ── V3e-5 : UNE fonction pour « la période d'un jour » ───────────────────────────────────────────────────────────
-- Parmi les périodes d'une périodisation qui touchent [p_du, p_au] : le plus grand recouvrement, puis la plus précise
-- (la plus courte), puis la plus ancienne (début, fin, identifiant). Règle TOTALE : deux appels rendent toujours la
-- même période, quel que soit le plan d'exécution. Pour un jour, tous les recouvrements sont égaux : la plus précise.
CREATE OR REPLACE FUNCTION ent_periode_choisir(p_etab uuid, p_periodisation uuid, p_du date, p_au date, p_annee uuid DEFAULT NULL)
RETURNS uuid LANGUAGE sql STABLE AS $$
    SELECT p.id
      FROM ent_periode p
     WHERE p.etablissement_id = p_etab AND p.periodisation_id = p_periodisation
       AND (p_annee IS NULL OR p.annee_id = p_annee)
       AND p.date_debut <= p_au AND p.date_fin >= p_du
     ORDER BY (least(p.date_fin, p_au) - greatest(p.date_debut, p_du)) DESC,
              (p.date_fin - p.date_debut), p.date_debut, p.date_fin, p.id
     LIMIT 1
$$;

CREATE OR REPLACE FUNCTION eva_periode_eleve_jour(p_etab uuid, p_eleve uuid, p_jour date)
RETURNS TABLE (classe_id uuid, periode_id uuid, etat text, date_arret_effective date)
LANGUAGE sql STABLE AS $$
    SELECT a.classe_id, p.id,
           COALESCE(e.etat, 'ouverte'),
           COALESCE(e.date_arret_effective, p.date_arret_notes)
      FROM ent_affectation a
      JOIN LATERAL (SELECT ent_periodisation_pour_classe(a.etablissement_id, a.classe_id) AS pz) z ON true
      JOIN LATERAL (SELECT ent_periode_choisir(a.etablissement_id, z.pz, p_jour, p_jour) AS pid) c ON c.pid IS NOT NULL
      JOIN ent_periode p ON p.etablissement_id = a.etablissement_id AND p.id = c.pid
      LEFT JOIN eva_periode_etat e
        ON e.etablissement_id = p.etablissement_id AND e.periode_id = p.id AND e.classe_id = a.classe_id
     WHERE a.etablissement_id = p_etab AND a.eleve_id = p_eleve AND a.classe_id IS NOT NULL
       AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
     ORDER BY a.date_debut DESC, a.id
     LIMIT 1
$$;

CREATE OR REPLACE FUNCTION ent_periode_de_note_pour_classe(p_etab uuid, p_note_periode uuid, p_classe uuid)
RETURNS uuid LANGUAGE sql STABLE AS $$
    SELECT ent_periode_choisir(np.etablissement_id, ent_periodisation_pour_classe(np.etablissement_id, c.id),
                               np.date_debut, np.date_fin, c.annee_id)
      FROM ent_note_periode np
      JOIN ent_classe c ON c.etablissement_id = np.etablissement_id AND c.id = p_classe
     WHERE np.etablissement_id = p_etab AND np.id = p_note_periode
$$;

-- V5 (règle 6, « une note appartient à l'élève et à la date ») : le jour p_jour relève-t-il de la période p_periode DANS
-- SA PÉRIODISATION ? C'est la période du BULLETIN qui décide, pas la classe qu'avait l'élève ce jour-là : un élève qui
-- passe d'une classe à trimestres à une classe à semestres garde ses notes (ni perdues, ni comptées deux fois, car les
-- périodes d'une périodisation ne comptent un jour qu'une fois, par `ent_periode_choisir`).
CREATE OR REPLACE FUNCTION eva_periode_dans(p_etab uuid, p_periode uuid, p_jour date)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT COALESCE((SELECT ent_periode_choisir(p.etablissement_id, p.periodisation_id, p_jour, p_jour, p.annee_id) = p.id
                       FROM ent_periode p
                      WHERE p.etablissement_id = p_etab AND p.id = p_periode), false)
$$;

-- ── V3e-4 : prolongation d'échéance tracée ───────────────────────────────────────────────────────────────────────
ALTER TABLE eva_rattrapage ADD COLUMN IF NOT EXISTS dernier_motif_prolongation TEXT
    CHECK (dernier_motif_prolongation IS NULL OR char_length(btrim(dernier_motif_prolongation)) BETWEEN 3 AND 500);
ALTER TABLE eva_rattrapage_historique ADD COLUMN IF NOT EXISTS echeance_avant DATE;
ALTER TABLE eva_rattrapage_historique ADD COLUMN IF NOT EXISTS echeance_apres DATE;
ALTER TABLE eva_rattrapage_historique ADD COLUMN IF NOT EXISTS motif TEXT
    CHECK (motif IS NULL OR char_length(btrim(motif)) BETWEEN 3 AND 500);

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
    -- V3e (item 4) : le motif d'une prolongation ne vaut que pour l'écriture qui déplace l'échéance (jamais périmé).
    IF NEW.echeance IS NOT DISTINCT FROM OLD.echeance THEN
        NEW.dernier_motif_prolongation := NULL;
    END IF;
    NEW.version := OLD.version + 1;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_rattrapage_historique() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO eva_rattrapage_historique
        (id, etablissement_id, rattrapage_id, eleve_id, version, etat_avant, etat_apres, prevu_le_avant, prevu_le_apres, par,
         echeance_avant, echeance_apres, motif)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.eleve_id, NEW.version,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.etat END, NEW.etat,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.prevu_le END, NEW.prevu_le, NEW.modifie_par,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.echeance END, NEW.echeance, NEW.dernier_motif_prolongation);
    RETURN NEW;
END
$$;

-- ── V5 : résultats externes (CE-02, CE-06, CE-07) ────────────────────────────────────────────────────────────────
-- Un résultat d'un autre établissement, du CNED ou de l'étranger ne compte PAS par défaut. Le geste de le compter dans
-- UN cours (décision tracée, motif) et celui de le publier à la famille sont distincts, une fois chacun ; un retrait est
-- motivé. Rien ne se modifie ni ne se supprime : corriger = retirer puis redéclarer.
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS date_resultat DATE;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS libelle TEXT
    CHECK (libelle IS NULL OR char_length(btrim(libelle)) BETWEEN 1 AND 160);
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS compte_decide_par UUID;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS compte_decide_le TIMESTAMPTZ;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS compte_motif TEXT
    CHECK (compte_motif IS NULL OR char_length(btrim(compte_motif)) BETWEEN 3 AND 500);
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS publiee_le TIMESTAMPTZ;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS publiee_par UUID;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS retiree_par UUID;
ALTER TABLE eva_resultat_externe ADD COLUMN IF NOT EXISTS motif_retrait TEXT
    CHECK (motif_retrait IS NULL OR char_length(btrim(motif_retrait)) BETWEEN 3 AND 500);
ALTER TABLE eva_resultat_externe DROP CONSTRAINT IF EXISTS eva_resultat_externe_decision_chk;
ALTER TABLE eva_resultat_externe ADD CONSTRAINT eva_resultat_externe_decision_chk CHECK (
    (compte_dans_cours_id IS NULL) = (compte_decide_par IS NULL)
    AND (compte_dans_cours_id IS NULL) = (compte_decide_le IS NULL)
    AND (compte_dans_cours_id IS NULL) = (compte_motif IS NULL)
    AND (publiee_le IS NULL) = (publiee_par IS NULL)
    AND (retiree_le IS NULL) = (retiree_par IS NULL)
    AND (retiree_le IS NULL) = (motif_retrait IS NULL)
    AND (publiee_le IS NULL OR compte_dans_cours_id IS NOT NULL)
    AND (compte_dans_cours_id IS NULL OR date_resultat IS NOT NULL OR periode_id IS NOT NULL));

CREATE OR REPLACE FUNCTION ent_trg_eva_resultat_externe() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_resultat_externe : suppression physique interdite, retirer le résultat avec un motif'
            USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.compte_dans_cours_id IS NOT NULL OR NEW.publiee_le IS NOT NULL OR NEW.retiree_le IS NOT NULL THEN
            RAISE EXCEPTION 'eva_resultat_externe : un résultat externe naît non compté, non publié et non retiré'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id OR NEW.matiere_id IS DISTINCT FROM OLD.matiere_id
       OR NEW.periode_id IS DISTINCT FROM OLD.periode_id OR NEW.annee_libelle IS DISTINCT FROM OLD.annee_libelle
       OR NEW.origine IS DISTINCT FROM OLD.origine OR NEW.valeur IS DISTINCT FROM OLD.valeur
       OR NEW.bareme IS DISTINCT FROM OLD.bareme OR NEW.coef IS DISTINCT FROM OLD.coef
       OR NEW.document_id IS DISTINCT FROM OLD.document_id OR NEW.saisi_par IS DISTINCT FROM OLD.saisi_par
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le OR NEW.date_resultat IS DISTINCT FROM OLD.date_resultat
       OR NEW.libelle IS DISTINCT FROM OLD.libelle THEN
        RAISE EXCEPTION 'eva_resultat_externe : un résultat déclaré ne se modifie pas (le retirer puis le redéclarer)'
            USING ERRCODE = '23514';
    END IF;
    IF OLD.retiree_le IS NOT NULL THEN
        RAISE EXCEPTION 'eva_resultat_externe : résultat déjà retiré' USING ERRCODE = '23514';
    END IF;
    -- Une décision de compter ne se rejoue pas : retirer, puis redéclarer.
    IF OLD.compte_dans_cours_id IS NOT NULL AND (NEW.compte_dans_cours_id IS DISTINCT FROM OLD.compte_dans_cours_id
        OR NEW.compte_decide_par IS DISTINCT FROM OLD.compte_decide_par OR NEW.compte_decide_le IS DISTINCT FROM OLD.compte_decide_le
        OR NEW.compte_motif IS DISTINCT FROM OLD.compte_motif) THEN
        RAISE EXCEPTION 'eva_resultat_externe : la décision de compter ne se modifie pas (retirer, puis redéclarer)'
            USING ERRCODE = '23514';
    END IF;
    IF OLD.publiee_le IS NOT NULL AND (NEW.publiee_le IS DISTINCT FROM OLD.publiee_le OR NEW.publiee_par IS DISTINCT FROM OLD.publiee_par) THEN
        RAISE EXCEPTION 'eva_resultat_externe : la publication ne se modifie pas' USING ERRCODE = '23514';
    END IF;
    IF NEW.compte_dans_cours_id IS NOT NULL AND OLD.compte_dans_cours_id IS NULL THEN
        -- Le cours doit enseigner la matière du résultat (aucune moyenne de maths alimentée par une note de français).
        IF NOT EXISTS (SELECT 1 FROM ent_cours co
                        WHERE co.etablissement_id = NEW.etablissement_id AND co.id = NEW.compte_dans_cours_id
                          AND co.matiere_id = NEW.matiere_id) THEN
            RAISE EXCEPTION 'eva_resultat_externe : le cours choisi n''enseigne pas la matière du résultat' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_resultat_externe_garde ON eva_resultat_externe;
CREATE TRIGGER eva_resultat_externe_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_resultat_externe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_resultat_externe();
DROP TRIGGER IF EXISTS eva_resultat_externe_vidage ON eva_resultat_externe;
CREATE TRIGGER eva_resultat_externe_vidage BEFORE TRUNCATE ON eva_resultat_externe
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Enseignement suivi hors de l'établissement et moyenne externe (R-V38) : la déclaration est figée (corriger = nouvelle
-- déclaration), sauf la clôture (`au`) et le référent ; une moyenne externe se corrige, avec son journal en ajout seul.
CREATE OR REPLACE FUNCTION ent_trg_eva_enseignement_externe() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_enseignement_externe : suppression physique interdite' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id OR NEW.matiere_id IS DISTINCT FROM OLD.matiere_id
       OR NEW.organisme IS DISTINCT FROM OLD.organisme OR NEW.du IS DISTINCT FROM OLD.du
       OR NEW.compte_moyenne_generale IS DISTINCT FROM OLD.compte_moyenne_generale
       OR NEW.declare_par IS DISTINCT FROM OLD.declare_par OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'eva_enseignement_externe : seules la clôture (au), le libellé et le référent évoluent'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_enseignement_externe_garde ON eva_enseignement_externe;
CREATE TRIGGER eva_enseignement_externe_garde BEFORE UPDATE OR DELETE ON eva_enseignement_externe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_enseignement_externe();

CREATE TABLE IF NOT EXISTS eva_moyenne_externe_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    moyenne_id UUID NOT NULL,
    enseignement_externe_id UUID NOT NULL,
    statut_avant TEXT,
    valeur_avant NUMERIC(7, 3),
    statut_apres TEXT NOT NULL,
    valeur_apres NUMERIC(7, 3),
    par UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_moyenne_externe_histo_fk FOREIGN KEY (moyenne_id) REFERENCES eva_moyenne_externe (id)
);
CREATE INDEX IF NOT EXISTS eva_moyenne_externe_histo_idx ON eva_moyenne_externe_historique (etablissement_id, moyenne_id, le);

CREATE OR REPLACE FUNCTION ent_trg_eva_moyenne_externe() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_moyenne_externe : suppression physique interdite' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
           OR NEW.enseignement_externe_id IS DISTINCT FROM OLD.enseignement_externe_id
           OR NEW.periode_id IS DISTINCT FROM OLD.periode_id OR NEW.bareme IS DISTINCT FROM OLD.bareme
           OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
            RAISE EXCEPTION 'eva_moyenne_externe : enseignement, période et barème ne changent pas' USING ERRCODE = '23514';
        END IF;
        IF NEW.statut IS NOT DISTINCT FROM OLD.statut AND NEW.valeur IS NOT DISTINCT FROM OLD.valeur
           AND NEW.appreciation_transcrite IS NOT DISTINCT FROM OLD.appreciation_transcrite
           AND NEW.document_id IS NOT DISTINCT FROM OLD.document_id THEN
            RETURN OLD;
        END IF;
    END IF;
    RETURN NEW;
END
$$;
-- Le journal s'écrit APRÈS la ligne (la clé étrangère du journal vise la ligne, qui doit exister).
CREATE OR REPLACE FUNCTION ent_trg_eva_moyenne_externe_historique() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO eva_moyenne_externe_historique (id, etablissement_id, moyenne_id, enseignement_externe_id,
                                                statut_avant, valeur_avant, statut_apres, valeur_apres, par)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.enseignement_externe_id,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.statut END, CASE WHEN TG_OP = 'UPDATE' THEN OLD.valeur END,
            NEW.statut, NEW.valeur, NEW.saisi_par);
    RETURN NULL;
END
$$;
DROP TRIGGER IF EXISTS eva_moyenne_externe_garde ON eva_moyenne_externe;
CREATE TRIGGER eva_moyenne_externe_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_moyenne_externe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_moyenne_externe();
DROP TRIGGER IF EXISTS eva_moyenne_externe_histo ON eva_moyenne_externe;
CREATE TRIGGER eva_moyenne_externe_histo AFTER INSERT OR UPDATE ON eva_moyenne_externe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_moyenne_externe_historique();
DROP TRIGGER IF EXISTS eva_moyenne_externe_histo_ajout_seul ON eva_moyenne_externe_historique;
CREATE TRIGGER eva_moyenne_externe_histo_ajout_seul BEFORE UPDATE OR DELETE ON eva_moyenne_externe_historique
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_enseignement_externe_vidage ON eva_enseignement_externe;
CREATE TRIGGER eva_enseignement_externe_vidage BEFORE TRUNCATE ON eva_enseignement_externe
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_moyenne_externe_vidage ON eva_moyenne_externe;
CREATE TRIGGER eva_moyenne_externe_vidage BEFORE TRUNCATE ON eva_moyenne_externe
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_moyenne_externe_histo_vidage ON eva_moyenne_externe_historique;
CREATE TRIGGER eva_moyenne_externe_histo_vidage BEFORE TRUNCATE ON eva_moyenne_externe_historique
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

SELECT ent_rls_poser(ARRAY['eva_moyenne_externe_historique']);

-- ── V5 (D15, CE-09, CE-10) : la classe de rattachement ───────────────────────────────────────────────────────────
-- LA classe d'un élève pour une période : sa dernière affectation de classe qui touche [p_du, p_au] (la plus récente,
-- puis l'identifiant). Une seule implémentation, lue par l'effectif, le bulletin, le relevé et le conseil : un élève
-- passé de la 5A à la 5B le 15 janvier n'a de bulletin du semestre que dans la 5B, et ses notes de la 5A y comptent.
CREATE OR REPLACE FUNCTION eva_classe_rattachement(p_etab uuid, p_eleve uuid, p_du date, p_au date)
RETURNS uuid LANGUAGE sql STABLE AS $$
    SELECT a.classe_id
      FROM ent_affectation a
     WHERE a.etablissement_id = p_etab AND a.eleve_id = p_eleve AND a.classe_id IS NOT NULL
       AND a.date_debut <= p_au AND (a.date_fin IS NULL OR a.date_fin >= p_du)
     ORDER BY a.date_debut DESC, a.id
     LIMIT 1
$$;
