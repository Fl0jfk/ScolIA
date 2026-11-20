-- Socle 21, lot V3c — 474 : revue de sécurité de V3b (constats M1, M2, faibles). 461–473 intactes (règle 25).
--
-- * M1 : `ent_trg_eva_resultat` exige un AUTEUR (`saisi_par` non NULL) et une attestation `app.eva_correcteur` non vide :
--   avant, un `saisi_par` NULL et une attestation vide se « ressemblaient » (`NULL IS DISTINCT FROM NULL` est faux) et
--   une écriture après l'arrêt passait sans porteur de `notes.corriger`.
-- * M2 : la vue de compatibilité `ent_note` ne rend plus 0 pour un absent, un dispensé, un annulé (`valeur` NULL hors
--   statut `note` : « absent, dispensé, annulé, non rendu ne valent jamais 0 », règle 2).
-- * Faibles : `NULLS LAST` sur le tri des périodes des deux vues (un `periode_id` indicatif NULL ne passait plus devant).
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

-- ── M2 + NULLS LAST : vues de compatibilité (corps de 472, deux écarts) ─────────────────────────────────────────────
CREATE OR REPLACE VIEW ent_devoir WITH (security_invoker = true) AS
SELECT ev.id, ev.etablissement_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ev.etablissement_id AND np.date_debut <= ev.date_eval AND np.date_fin >= ev.date_eval
         ORDER BY (np.periode_id = ev.periode_id) DESC NULLS LAST, np.date_debut, np.created_at, np.id LIMIT 1) AS periode_id,
       co.matiere_id,
       COALESCE(cl.code, g.code, '') AS classe,
       ev.titre,
       ev.date_eval AS date_devoir,
       ev.coef::double precision AS coef,
       ev.bareme::double precision AS sur,
       ev.cree_par AS created_by_user_id,
       ev.cree_le AS created_at,
       co.classe_id,
       (SELECT s.id FROM ent_service_enseignement s
         WHERE s.etablissement_id = ev.etablissement_id AND s.cours_id = ev.cours_id
         ORDER BY s.created_at, s.id LIMIT 1) AS service_id,
       co.groupe_id,
       true AS publication_explicite,
       ev.publiee_le AS publie_le,
       ev.publiee_par AS publie_par_user_id
  FROM eva_evaluation ev
  JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
  LEFT JOIN ent_classe cl ON cl.id = co.classe_id AND cl.etablissement_id = co.etablissement_id
  LEFT JOIN ent_groupe g ON g.id = co.groupe_id AND g.etablissement_id = co.etablissement_id
 WHERE ev.mode = 'note' AND ev.etat = 'ouverte';

-- La période de notes d'une évaluation est trouvée par sa DATE (les périodes de notes sont des plages de dates ; la période
-- `eva_evaluation.periode_id` n'est qu'indicative).
-- Écarts de forme assumés (comme 468) : `libelle` = titre de l'évaluation (ce que l'ancien écrivain posait) ; `valeur` rend 0 pour un statut non numérique ; `non_rendu` et `annule` se lisent
-- `non_note` ; `publication_explicite` vaut toujours vrai et `publie_le` porte la publication (retrait : NULL).
-- AJOUT : `publiee` (l'évaluation est publiée ?). Les notes NON publiées sont dans la vue : aucun lecteur qui répond à une
-- famille ou à un élève ne doit les montrer sans filtrer sur `publiee` (test de fuite, revue V3b).
CREATE OR REPLACE VIEW ent_note WITH (security_invoker = true) AS
SELECT r.id, r.etablissement_id, r.eleve_id, co.matiere_id,
       (CASE WHEN r.statut = 'note' THEN r.valeur END)::double precision AS valeur,
       ev.bareme::double precision AS sur,
       ev.date_eval AS date_note,
       r.saisi_par AS auteur_user_id,
       ev.titre AS libelle,
       r.cree_le AS created_at,
       r.evaluation_id AS devoir_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ev.etablissement_id AND np.date_debut <= ev.date_eval AND np.date_fin >= ev.date_eval
         ORDER BY (np.periode_id = ev.periode_id) DESC NULLS LAST, np.date_debut, np.created_at, np.id LIMIT 1) AS periode_id,
       CASE WHEN r.statut IN ('non_rendu', 'annule') THEN 'non_note' ELSE r.statut END AS statut,
       r.saisi_le AS updated_at,
       r.saisi_par AS updated_by_user_id,
       (ev.publiee_le IS NOT NULL) AS publiee
  FROM eva_resultat r
  JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
  JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
 WHERE ev.mode = 'note' AND ev.etat = 'ouverte';
