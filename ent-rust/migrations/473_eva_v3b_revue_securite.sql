-- Socle 21, lot V3b — 473 : correctifs de la revue de sécurité de V3 qui touchent la base.
--
-- * F2 : la base vérifie qu'une écriture sur un résultat après l'arrêt de la période vient d'un porteur de
--   `notes.corriger` (attestation posée par l'application dans la transaction), pas seulement d'un motif non vide
--   (`ent_trg_eva_resultat` remplacée ; 464 et 471 intactes, règle 25).
-- * F4 : le délai de 24 h du retrait par le professeur court depuis la PREMIÈRE publication (`premiere_publiee_le`,
--   posée une fois et jamais modifiée ; retirer puis republier ne le relance pas).
-- Additive et rejouable.

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
            IF nullif(current_setting('app.eva_correcteur', true), '') IS DISTINCT FROM NEW.saisi_par::text THEN
                RAISE EXCEPTION 'eva_resultat : une écriture après l''arrêt exige un porteur de notes.corriger (attestation absente)'
                    USING ERRCODE = '23514';
            END IF;
            v_corr := true;
        END IF;
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

-- ── F4 : première publication d'une évaluation ───────────────────────────────────────────────────────────────────────
ALTER TABLE eva_evaluation ADD COLUMN IF NOT EXISTS premiere_publiee_le TIMESTAMPTZ;

-- Reprise de l'existant : la première publication connue (journal des publications, ajout seul), à défaut la publication
-- en cours. La garde de l'évaluation est suspendue le temps de la reprise (aucune version, aucun historique créé).
DO $$
BEGIN
    ALTER TABLE eva_evaluation DISABLE TRIGGER eva_evaluation_garde;
    UPDATE eva_evaluation e
       SET premiere_publiee_le = COALESCE(
               (SELECT min(j.le) FROM eva_evaluation_publication_journal j
                 WHERE j.etablissement_id = e.etablissement_id AND j.evaluation_id = e.id
                   AND j.action IN ('publiee', 'publiee_auto_programmee', 'publiee_auto_delai', 'publiee_auto_arret')),
               e.publiee_le)
     WHERE e.premiere_publiee_le IS NULL AND (e.publiee_le IS NOT NULL OR e.retiree_le IS NOT NULL);
    ALTER TABLE eva_evaluation ENABLE TRIGGER eva_evaluation_garde;
END
$$;

-- La première publication se pose une fois (à la première publication) et ne change plus : retirer puis republier ne la
-- déplace pas.
CREATE OR REPLACE FUNCTION ent_trg_eva_evaluation_premiere_publication() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.premiere_publiee_le IS NOT NULL THEN
        NEW.premiere_publiee_le := OLD.premiere_publiee_le;
    ELSIF NEW.publiee_le IS NOT NULL AND NEW.premiere_publiee_le IS NULL THEN
        NEW.premiere_publiee_le := NEW.publiee_le;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_evaluation_premiere_publication ON eva_evaluation;
CREATE TRIGGER eva_evaluation_premiere_publication BEFORE INSERT OR UPDATE ON eva_evaluation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_evaluation_premiere_publication();
