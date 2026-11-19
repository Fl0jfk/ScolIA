-- Socle 21, lot V3 « grille et saisie » — 471 : (1) correctifs F7 et F8 de la reprise M2 (`eva_reprendre_existant`
-- remplacée ; 467 n'est pas modifiée, règle 25) ; (2) retrait de publication, « en cours de correction », rappel
-- de publication ; (3) journaux de publication et d'import (ajout seul). Additive et rejouable.
--
-- F7 : deux reprises simultanées d'un même établissement se suivent (verrou consultatif) et un doublon n'est plus
--      compté « refusé erreur_sql » mais « inchangé » (rapport juste).
-- F8 : une note déjà reprise puis corrigée par l'ancien écrivain est recopiée (une ligne d'historique par correction,
--      y compris A -> B -> A) ; si la nouvelle grille a écrit entre-temps, le conflit est listé et rien n'est écrasé.

CREATE OR REPLACE FUNCTION eva_reprendre_existant(p_etab uuid, p_origine text DEFAULT 'manuel') RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    v_exec uuid := gen_random_uuid();
    -- compteurs : lues, creees, inchangees, refusees, ignorees, ecarts
    c_dev int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_not int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_nsd int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_cor int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_app int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_per int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_reg int[] := ARRAY[0, 0, 0, 0, 0, 0];
    c_coe int[] := ARRAY[0, 0, 0, 0, 0, 0];
    r RECORD;
    y RECORD;
    cf RECORD;
    d RECORD;
    n RECORD;
    a RECORD;
    np RECORD;
    cl RECORD;
    k RECORD;
    v_cours uuid;
    v_annee uuid;
    v_classe uuid;
    v_groupe uuid;
    v_bareme numeric;
    v_coef numeric;
    v_valeur numeric;
    v_statut text;
    v_nb_corr int;
    v_version int;
    v_prec_statut text;
    v_prec_valeur numeric;
    v_pub timestamptz;
    v_pub_par uuid;
    v_etat_cible text;
    v_rang_cible int;
    v_rang_actuel int;
    v_date_arret date;
    v_libelle text;
    v_deja int;
BEGIN
    IF ent_rls_etablissement() IS NOT NULL AND ent_rls_etablissement() <> p_etab THEN
        RAISE EXCEPTION 'eva_reprendre_existant : établissement hors du contexte' USING ERRCODE = '42501';
    END IF;
    -- F7 : deux reprises simultanées du même établissement se suivent (la seconde voit les lignes de la première
    -- et les compte « inchangées », jamais « refusées erreur_sql »).
    PERFORM pg_advisory_xact_lock(hashtextextended('eva_reprise:' || p_etab::text, 0));
    ALTER TABLE eva_resultat DISABLE TRIGGER eva_resultat_garde;
    ALTER TABLE eva_resultat DISABLE TRIGGER eva_resultat_histo;
    ALTER TABLE eva_coefficient_matiere DISABLE TRIGGER eva_coef_matiere_garde;
    INSERT INTO eva_reprise_execution (id, etablissement_id, origine) VALUES (v_exec, p_etab, p_origine);

    -- ── Règles de calcul : une ligne par année (l'ancienne table `ent_eva_regle` n'en a pas) ──
    -- `absent_compte_zero` n'est jamais repris : une absence n'est jamais un zéro (R-V5).
    FOR r IN SELECT * FROM ent_eva_regle WHERE etablissement_id = p_etab LOOP
        FOR y IN SELECT ya.id FROM ent_annee_scolaire ya WHERE ya.etablissement_id = p_etab ORDER BY ya.date_debut LOOP
            c_reg[1] := c_reg[1] + 1;
            IF EXISTS (SELECT 1 FROM eva_reglage g WHERE g.etablissement_id = p_etab AND g.annee_id = y.id) THEN
                c_reg[3] := c_reg[3] + 1;
                CONTINUE;
            END IF;
            BEGIN
                INSERT INTO eva_reglage (id, etablissement_id, annee_id, arrondi_precision, modifie_par)
                VALUES (gen_random_uuid(), p_etab, y.id,
                        CASE WHEN r.arrondi IN ('centieme', 'dixieme', 'demi_point') THEN r.arrondi ELSE 'centieme' END,
                        r.updated_by_user_id);
                c_reg[2] := c_reg[2] + 1;
            EXCEPTION WHEN unique_violation THEN
                -- F7 : la ligne vient d'être créée par une reprise simultanée.
                c_reg[3] := c_reg[3] + 1;
            WHEN OTHERS THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'reglage', y.id, 'refusee', 'erreur_sql', SQLSTATE);
                c_reg[4] := c_reg[4] + 1;
            END;
        END LOOP;
    END LOOP;

    -- ── Coefficients de matière (par classe), valables dès le début de l'année de la classe ──
    FOR cf IN
        SELECT cm.id, cm.classe_id, cm.matiere_id, cm.coef, cm.updated_by_user_id, kl.annee_id, ya.date_debut
          FROM ent_eva_coef_matiere cm
          JOIN ent_classe kl ON kl.id = cm.classe_id AND kl.etablissement_id = cm.etablissement_id
          JOIN ent_annee_scolaire ya ON ya.id = kl.annee_id AND ya.etablissement_id = kl.etablissement_id
         WHERE cm.etablissement_id = p_etab
         ORDER BY cm.id
    LOOP
        c_coe[1] := c_coe[1] + 1;
        IF EXISTS (SELECT 1 FROM eva_coefficient_matiere x WHERE x.etablissement_id = p_etab
                    AND x.annee_id = cf.annee_id AND x.classe_id = cf.classe_id AND x.matiere_id = cf.matiere_id) THEN
            c_coe[3] := c_coe[3] + 1;
            CONTINUE;
        END IF;
        BEGIN
            v_coef := eva_dec3(cf.coef);
            IF v_coef IS NULL OR v_coef < 0 OR v_coef > 100 THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'coefficient', cf.id, 'refusee', 'coefficient_invalide');
                c_coe[4] := c_coe[4] + 1;
                CONTINUE;
            END IF;
            INSERT INTO eva_coefficient_matiere (id, etablissement_id, annee_id, matiere_id, classe_id, coef, valable_du, cree_par)
            VALUES (cf.id, p_etab, cf.annee_id, cf.matiere_id, cf.classe_id, v_coef, cf.date_debut, cf.updated_by_user_id);
            c_coe[2] := c_coe[2] + 1;
        EXCEPTION WHEN unique_violation THEN
            -- F7 : la ligne vient d'être créée par une reprise simultanée.
            c_coe[3] := c_coe[3] + 1;
        WHEN OTHERS THEN
            PERFORM eva_reprise_noter(v_exec, p_etab, 'coefficient', cf.id, 'refusee', 'erreur_sql', SQLSTATE);
            c_coe[4] := c_coe[4] + 1;
        END;
    END LOOP;

    -- ── Évaluations ───────────────────────────────────────────────────────────────
    FOR d IN
        SELECT dv.*, nper.periode_id AS periode_ref
          FROM ent_devoir dv
          LEFT JOIN ent_note_periode nper ON nper.id = dv.periode_id AND nper.etablissement_id = dv.etablissement_id
         WHERE dv.etablissement_id = p_etab
         ORDER BY dv.created_at, dv.id
    LOOP
        c_dev[1] := c_dev[1] + 1;
        IF EXISTS (SELECT 1 FROM eva_evaluation WHERE id = d.id AND etablissement_id = p_etab) THEN
            c_dev[3] := c_dev[3] + 1;
            CONTINUE;
        END IF;
        BEGIN
            v_bareme := eva_dec3(d.sur);
            v_coef := eva_dec3(d.coef);
            IF v_bareme IS NULL OR v_bareme <= 0 OR v_bareme > 1000 THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'devoir', d.id, 'refusee', 'bareme_invalide');
                c_dev[4] := c_dev[4] + 1;
                CONTINUE;
            END IF;
            IF v_coef IS NULL OR v_coef < 0 OR v_coef > 100 THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'devoir', d.id, 'refusee', 'coefficient_invalide');
                c_dev[4] := c_dev[4] + 1;
                CONTINUE;
            END IF;
            -- 1. le cours par le service d'enseignement (socle 5 : chaque service porte un cours).
            v_cours := NULL;
            IF d.service_id IS NOT NULL THEN
                SELECT s.cours_id INTO v_cours FROM ent_service_enseignement s
                 WHERE s.id = d.service_id AND s.etablissement_id = p_etab;
            END IF;
            -- 2. sinon par le public (classe ou groupe) et la matière, valable à la date du devoir.
            IF v_cours IS NULL THEN
                SELECT c.id INTO v_cours FROM ent_cours c
                 WHERE c.etablissement_id = p_etab AND c.matiere_id = d.matiere_id
                   AND ((d.groupe_id IS NOT NULL AND c.groupe_id = d.groupe_id)
                        OR (d.groupe_id IS NULL AND d.classe_id IS NOT NULL AND c.classe_id = d.classe_id))
                   AND c.ouvert_le <= d.date_devoir AND (c.ferme_le IS NULL OR c.ferme_le >= d.date_devoir)
                 ORDER BY c.ouvert_le DESC, c.id LIMIT 1;
            END IF;
            -- 3. sinon n'importe quel cours de ce public et de cette matière (ouvert depuis).
            IF v_cours IS NULL THEN
                SELECT c.id INTO v_cours FROM ent_cours c
                 WHERE c.etablissement_id = p_etab AND c.matiere_id = d.matiere_id
                   AND ((d.groupe_id IS NOT NULL AND c.groupe_id = d.groupe_id)
                        OR (d.groupe_id IS NULL AND d.classe_id IS NOT NULL AND c.classe_id = d.classe_id))
                 ORDER BY c.ouvert_le DESC, c.id LIMIT 1;
            END IF;
            -- 4. sinon un cours « reprise », à vérifier par la gestion (jamais inventé sans public).
            IF v_cours IS NULL THEN
                IF d.groupe_id IS NULL AND d.classe_id IS NULL THEN
                    PERFORM eva_reprise_noter(v_exec, p_etab, 'devoir', d.id, 'refusee', 'public_introuvable',
                                              'ni service, ni classe, ni groupe');
                    c_dev[4] := c_dev[4] + 1;
                    CONTINUE;
                END IF;
                v_classe := CASE WHEN d.groupe_id IS NULL THEN d.classe_id END;
                v_groupe := d.groupe_id;
                SELECT COALESCE((SELECT g.annee_id FROM ent_groupe g WHERE g.id = v_groupe AND g.etablissement_id = p_etab),
                                (SELECT kc.annee_id FROM ent_classe kc WHERE kc.id = v_classe AND kc.etablissement_id = p_etab))
                  INTO v_annee;
                IF v_annee IS NULL THEN
                    PERFORM eva_reprise_noter(v_exec, p_etab, 'devoir', d.id, 'refusee', 'annee_introuvable');
                    c_dev[4] := c_dev[4] + 1;
                    CONTINUE;
                END IF;
                SELECT m.libelle INTO v_libelle FROM ent_matiere m WHERE m.id = d.matiere_id AND m.etablissement_id = p_etab;
                v_cours := gen_random_uuid();
                INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, matiere_id, classe_id, groupe_id,
                                       ouvert_le, source, a_verifier)
                SELECT v_cours, p_etab, v_annee, COALESCE(NULLIF(btrim(v_libelle), ''), 'Reprise des notes'),
                       d.matiere_id, v_classe, v_groupe, LEAST(d.date_devoir, an.date_debut), 'reprise', true
                  FROM ent_annee_scolaire an WHERE an.id = v_annee AND an.etablissement_id = p_etab;
                PERFORM eva_reprise_noter(v_exec, p_etab, 'devoir', d.id, 'ecart', 'cours_reprise_cree',
                                          'cours créé pour la reprise, à vérifier par la gestion');
                c_dev[6] := c_dev[6] + 1;
            END IF;
            SELECT c.annee_id INTO v_annee FROM ent_cours c WHERE c.id = v_cours AND c.etablissement_id = p_etab;
            -- Publication : un devoir historique sans publication explicite est visible dès sa création.
            IF d.publication_explicite THEN
                v_pub := d.publie_le;
                v_pub_par := COALESCE(d.publie_par_user_id, d.created_by_user_id);
            ELSE
                v_pub := d.created_at;
                v_pub_par := d.created_by_user_id;
            END IF;
            INSERT INTO eva_evaluation (
                id, etablissement_id, annee_id, cours_id, periode_id, date_eval, titre, mode, bareme, coef,
                comptage, etat, publication, publiee_le, publiee_par, origine_devoir_id, cree_par, cree_le)
            VALUES (
                d.id, p_etab, v_annee, v_cours, d.periode_ref, d.date_devoir,
                COALESCE(NULLIF(left(btrim(d.titre), 200), ''), 'Évaluation'), 'note', v_bareme, v_coef,
                'normal', 'ouverte', 'explicite', v_pub, v_pub_par, d.id, d.created_by_user_id, d.created_at);
            c_dev[2] := c_dev[2] + 1;
        EXCEPTION WHEN unique_violation THEN
            -- F7 : la ligne vient d'être créée par une reprise simultanée.
            c_dev[3] := c_dev[3] + 1;
        WHEN OTHERS THEN
            -- Le détail ne contient que le code d'erreur SQL, jamais une valeur de donnée.
            PERFORM eva_reprise_noter(v_exec, p_etab, 'devoir', d.id, 'refusee', 'erreur_sql', SQLSTATE);
            c_dev[4] := c_dev[4] + 1;
        END;
    END LOOP;

    -- ── Notes sans devoir : voie historique fermée en V0, non reprises, listées ─────────
    FOR n IN SELECT id FROM ent_note WHERE etablissement_id = p_etab AND devoir_id IS NULL ORDER BY created_at, id LOOP
        c_nsd[1] := c_nsd[1] + 1;
        c_nsd[5] := c_nsd[5] + 1;
        PERFORM eva_reprise_noter(v_exec, p_etab, 'note_sans_devoir', n.id, 'ignoree', 'note_sans_devoir',
                                  'à rattacher à une évaluation par la direction');
    END LOOP;

    -- ── Résultats (notes) ─────────────────────────────────────────────────────────────
    FOR n IN
        SELECT nt.* FROM ent_note nt
         WHERE nt.etablissement_id = p_etab AND nt.devoir_id IS NOT NULL
         ORDER BY nt.created_at, nt.id
    LOOP
        c_not[1] := c_not[1] + 1;
        IF EXISTS (SELECT 1 FROM eva_resultat WHERE id = n.id AND etablissement_id = p_etab) THEN
            c_not[3] := c_not[3] + 1;
            -- F8 : l'ancien écrivain a CORRIGÉ la note depuis la reprise (nouvelles lignes d'`ent_note_correction`) :
            -- chaque correction est recopiée dans l'ordre, une ligne d'historique par correction (donc A -> B -> A
            -- laisse deux lignes). Rien n'est recopié si un autre écrivain (la grille V3) a écrit entre-temps :
            -- le conflit est listé, jamais tranché en silence.
            SELECT count(*) INTO v_nb_corr FROM ent_note_correction c
             WHERE c.etablissement_id = p_etab AND c.note_id = n.id;
            SELECT greatest(count(*) - 1, 0) INTO v_deja FROM eva_resultat_historique h
             WHERE h.etablissement_id = p_etab AND h.resultat_id = n.id AND h.source = 'reprise';
            IF v_nb_corr > v_deja THEN
                IF EXISTS (SELECT 1 FROM eva_resultat_historique h
                            WHERE h.etablissement_id = p_etab AND h.resultat_id = n.id AND h.source <> 'reprise') THEN
                    PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'ecart', 'deux_ecrivains',
                                              'corrigée dans l''ancienne grille et dans la nouvelle : à arbitrer');
                    c_not[6] := c_not[6] + 1;
                ELSE
                    FOR k IN SELECT * FROM ent_note_correction c
                              WHERE c.etablissement_id = p_etab AND c.note_id = n.id
                              ORDER BY c.corrige_le, c.id OFFSET v_deja
                    LOOP
                        SELECT rs.statut, rs.valeur, rs.version INTO v_prec_statut, v_prec_valeur, v_version
                          FROM eva_resultat rs WHERE rs.id = n.id AND rs.etablissement_id = p_etab;
                        UPDATE eva_resultat
                           SET statut = k.nouveau_statut,
                               valeur = CASE WHEN k.nouveau_statut = 'note' THEN eva_dec3(k.nouvelle_valeur) END,
                               version = v_version + 1, saisi_par = k.corrige_par_user_id, saisi_le = k.corrige_le
                         WHERE id = n.id AND etablissement_id = p_etab;
                        INSERT INTO eva_resultat_historique (id, etablissement_id, resultat_id, evaluation_id, eleve_id,
                            version, statut_avant, valeur_avant, statut_apres, valeur_apres, correction, motif, source,
                            auteur, le)
                        VALUES (gen_random_uuid(), p_etab, n.id, n.devoir_id, n.eleve_id, v_version + 1, v_prec_statut,
                                v_prec_valeur, k.nouveau_statut,
                                CASE WHEN k.nouveau_statut = 'note' THEN eva_dec3(k.nouvelle_valeur) END,
                                true, k.motif, 'reprise', k.corrige_par_user_id, k.corrige_le);
                        c_cor[1] := c_cor[1] + 1;
                        c_cor[2] := c_cor[2] + 1;
                    END LOOP;
                    PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'ecart', 'correction_recopiee',
                                              'correction(s) de l''ancienne grille recopiée(s) dans l''historique');
                    c_not[6] := c_not[6] + 1;
                END IF;
            END IF;
            -- Dérive : l'ancien écrivain a modifié la note depuis la reprise sans correction journalisée (listée).
            IF EXISTS (SELECT 1 FROM eva_resultat rs WHERE rs.id = n.id AND rs.etablissement_id = p_etab
                        AND (rs.statut IS DISTINCT FROM n.statut
                             OR rs.valeur IS DISTINCT FROM (CASE WHEN n.statut = 'note' THEN eva_dec3(n.valeur) END))) THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'ecart', 'derive_depuis_la_reprise');
                c_not[6] := c_not[6] + 1;
            END IF;
            CONTINUE;
        END IF;
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM eva_evaluation WHERE id = n.devoir_id AND etablissement_id = p_etab) THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'refusee', 'evaluation_non_reprise');
                c_not[4] := c_not[4] + 1;
                CONTINUE;
            END IF;
            v_statut := n.statut;
            IF v_statut NOT IN ('note', 'absent', 'a_rattraper', 'dispense', 'non_note') THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'refusee', 'statut_inconnu');
                c_not[4] := c_not[4] + 1;
                CONTINUE;
            END IF;
            v_valeur := CASE WHEN v_statut = 'note' THEN eva_dec3(n.valeur) END;
            IF v_statut = 'note' AND (v_valeur IS NULL OR v_valeur < 0 OR v_valeur > 9999) THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'refusee', 'valeur_invalide');
                c_not[4] := c_not[4] + 1;
                CONTINUE;
            END IF;
            SELECT count(*) INTO v_nb_corr FROM ent_note_correction c
             WHERE c.etablissement_id = p_etab AND c.note_id = n.id;
            v_version := v_nb_corr + 1;
            INSERT INTO eva_resultat (
                id, etablissement_id, evaluation_id, eleve_id, statut, valeur, source, version,
                saisi_par, saisi_le, cree_le)
            VALUES (
                n.id, p_etab, n.devoir_id, n.eleve_id, v_statut, v_valeur, 'reprise', v_version,
                COALESCE(n.updated_by_user_id, n.auteur_user_id), COALESCE(n.updated_at, n.created_at), n.created_at);
            -- Historique : version 1 = valeur d'origine ; une version par correction (qui, quand, motif).
            v_prec_statut := NULL;
            v_prec_valeur := NULL;
            v_version := 1;
            IF v_nb_corr = 0 THEN
                INSERT INTO eva_resultat_historique (id, etablissement_id, resultat_id, evaluation_id, eleve_id,
                    version, statut_avant, valeur_avant, statut_apres, valeur_apres, correction, motif, source, auteur, le)
                VALUES (gen_random_uuid(), p_etab, n.id, n.devoir_id, n.eleve_id, 1, NULL, NULL, v_statut, v_valeur,
                        false, NULL, 'reprise', n.auteur_user_id, n.created_at);
            ELSE
                FOR k IN SELECT * FROM ent_note_correction c
                          WHERE c.etablissement_id = p_etab AND c.note_id = n.id
                          ORDER BY c.corrige_le, c.id
                LOOP
                    IF v_version = 1 THEN
                        INSERT INTO eva_resultat_historique (id, etablissement_id, resultat_id, evaluation_id, eleve_id,
                            version, statut_avant, valeur_avant, statut_apres, valeur_apres, correction, motif, source, auteur, le)
                        VALUES (gen_random_uuid(), p_etab, n.id, n.devoir_id, n.eleve_id, 1, NULL, NULL, k.ancien_statut,
                                CASE WHEN k.ancien_statut = 'note' THEN eva_dec3(k.ancienne_valeur) END,
                                false, NULL, 'reprise', n.auteur_user_id, n.created_at);
                        v_version := 2;
                    END IF;
                    INSERT INTO eva_resultat_historique (id, etablissement_id, resultat_id, evaluation_id, eleve_id,
                        version, statut_avant, valeur_avant, statut_apres, valeur_apres, correction, motif, source, auteur, le)
                    VALUES (gen_random_uuid(), p_etab, n.id, n.devoir_id, n.eleve_id, v_version, k.ancien_statut,
                            CASE WHEN k.ancien_statut = 'note' THEN eva_dec3(k.ancienne_valeur) END,
                            k.nouveau_statut, CASE WHEN k.nouveau_statut = 'note' THEN eva_dec3(k.nouvelle_valeur) END,
                            true, k.motif, 'reprise', k.corrige_par_user_id, k.corrige_le);
                    v_version := v_version + 1;
                    c_cor[1] := c_cor[1] + 1;
                    c_cor[2] := c_cor[2] + 1;
                END LOOP;
            END IF;
            c_not[2] := c_not[2] + 1;
            -- Écarts : arrondi au millième, note au-delà du barème de l'évaluation.
            IF v_statut = 'note' AND abs(n.valeur::numeric - v_valeur) > 0.0000001 THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'ecart', 'arrondi_millieme');
                c_not[6] := c_not[6] + 1;
            ELSIF v_statut = 'note' AND v_valeur > (SELECT e.bareme FROM eva_evaluation e
                                                     WHERE e.id = n.devoir_id AND e.etablissement_id = p_etab) THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'ecart', 'hors_bareme',
                                          'ignorée par le moteur comme par l''ancien');
                c_not[6] := c_not[6] + 1;
            END IF;
        EXCEPTION WHEN unique_violation THEN
            -- F7 : la ligne vient d'être créée par une reprise simultanée.
            c_not[3] := c_not[3] + 1;
        WHEN OTHERS THEN
            PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'refusee', 'erreur_sql', SQLSTATE);
            c_not[4] := c_not[4] + 1;
        END;
    END LOOP;

    -- ── Appréciations ─────────────────────────────────────────────────────────────────
    FOR a IN
        SELECT ap.*, nper.periode_id AS periode_ref, nper.date_debut AS np_debut, nper.date_fin AS np_fin
          FROM ent_note_appreciation ap
          LEFT JOIN ent_note_periode nper ON nper.id = ap.periode_id AND nper.etablissement_id = ap.etablissement_id
         WHERE ap.etablissement_id = p_etab
         ORDER BY ap.created_at, ap.id
    LOOP
        c_app[1] := c_app[1] + 1;
        IF EXISTS (SELECT 1 FROM eva_appreciation WHERE etablissement_id = p_etab
                    AND (id = a.id OR origine_appreciation_id = a.id)) THEN
            c_app[3] := c_app[3] + 1;
            CONTINUE;
        END IF;
        BEGIN
            IF a.periode_ref IS NULL THEN
                PERFORM eva_reprise_noter(v_exec, p_etab, 'appreciation', a.id, 'refusee', 'periode_non_rattachee');
                c_app[4] := c_app[4] + 1;
                CONTINUE;
            END IF;
            v_cours := NULL;
            IF a.matiere_id IS NOT NULL THEN
                -- Le cours de la matière suivi par l'élève à la fin (sinon au début) de la période.
                SELECT c.id INTO v_cours
                  FROM ent_public_cours_jour(p_etab, a.np_fin, NULL, NULL, a.eleve_id) pu
                  JOIN ent_cours c ON c.id = pu.cours_id AND c.etablissement_id = p_etab
                 WHERE c.matiere_id = a.matiere_id
                 ORDER BY c.ouvert_le DESC, c.id LIMIT 1;
                IF v_cours IS NULL THEN
                    SELECT c.id INTO v_cours
                      FROM ent_public_cours_jour(p_etab, a.np_debut, NULL, NULL, a.eleve_id) pu
                      JOIN ent_cours c ON c.id = pu.cours_id AND c.etablissement_id = p_etab
                     WHERE c.matiere_id = a.matiere_id
                     ORDER BY c.ouvert_le DESC, c.id LIMIT 1;
                END IF;
                -- Les cours « de reprise » ne sont pas dans les publics (470) : on les cherche ici directement.
                IF v_cours IS NULL THEN
                    SELECT c.id INTO v_cours
                      FROM ent_cours c
                      JOIN ent_affectation af ON af.etablissement_id = c.etablissement_id AND af.annee_id = c.annee_id
                       AND ((c.classe_id IS NOT NULL AND af.classe_id = c.classe_id)
                         OR (c.groupe_id IS NOT NULL AND af.groupe_id = c.groupe_id))
                     WHERE c.etablissement_id = p_etab AND c.matiere_id = a.matiere_id
                       AND c.source = 'reprise' AND c.a_verifier AND af.eleve_id = a.eleve_id
                       AND af.date_debut <= a.np_fin AND (af.date_fin IS NULL OR af.date_fin >= a.np_debut)
                     ORDER BY c.ouvert_le DESC, c.id LIMIT 1;
                END IF;
                IF v_cours IS NULL THEN
                    PERFORM eva_reprise_noter(v_exec, p_etab, 'appreciation', a.id, 'refusee', 'cours_introuvable',
                                              'aucun cours de cette matière pour l''élève sur la période');
                    c_app[4] := c_app[4] + 1;
                    CONTINUE;
                END IF;
            END IF;
            INSERT INTO eva_appreciation (id, etablissement_id, periode_id, eleve_id, type, cours_id, texte, etat,
                                          version, auteur, origine_appreciation_id, cree_le, maj_le)
            VALUES (a.id, p_etab, a.periode_ref, a.eleve_id,
                    CASE WHEN a.matiere_id IS NULL THEN 'generale' ELSE 'cours' END, v_cours,
                    left(a.texte, 2000), 'validee', 1, a.auteur_user_id, a.id, a.created_at, a.updated_at);
            c_app[2] := c_app[2] + 1;
        EXCEPTION WHEN unique_violation THEN
            -- F7 : la ligne vient d'être créée par une reprise simultanée.
            c_app[3] := c_app[3] + 1;
        WHEN OTHERS THEN
            PERFORM eva_reprise_noter(v_exec, p_etab, 'appreciation', a.id, 'refusee', 'erreur_sql', SQLSTATE);
            c_app[4] := c_app[4] + 1;
        END;
    END LOOP;

    -- ── États de période : un par classe de la périodisation de la période ─────────────
    FOR np IN
        SELECT q.*, p.periodisation_id, p.annee_id AS p_annee, p.date_arret_notes, p.date_fin AS p_fin
          FROM ent_note_periode q
          LEFT JOIN ent_periode p ON p.id = q.periode_id AND p.etablissement_id = q.etablissement_id
         WHERE q.etablissement_id = p_etab
         ORDER BY q.created_at, q.id
    LOOP
        IF np.periode_id IS NULL OR np.periodisation_id IS NULL THEN
            c_per[1] := c_per[1] + 1;
            c_per[4] := c_per[4] + 1;
            PERFORM eva_reprise_noter(v_exec, p_etab, 'periode', np.id, 'refusee', 'periode_non_rattachee');
            CONTINUE;
        END IF;
        v_etat_cible := CASE np.statut WHEN 'fermee' THEN 'appreciations_closes'
                                       WHEN 'partielle_appreciations' THEN 'notes_arretees' ELSE 'ouverte' END;
        v_rang_cible := eva_etat_rang(v_etat_cible);
        v_date_arret := COALESCE(np.date_arret_notes, np.p_fin);
        FOR cl IN
            SELECT c.id FROM ent_classe c
             WHERE c.etablissement_id = p_etab AND c.annee_id = np.p_annee
               AND ent_periodisation_pour_classe(p_etab, c.id) = np.periodisation_id
        LOOP
            c_per[1] := c_per[1] + 1;
            BEGIN
                IF EXISTS (SELECT 1 FROM eva_periode_etat e WHERE e.etablissement_id = p_etab
                            AND e.periode_id = np.periode_id AND e.classe_id = cl.id) THEN
                    SELECT eva_etat_rang(e.etat) INTO v_rang_actuel FROM eva_periode_etat e
                     WHERE e.etablissement_id = p_etab AND e.periode_id = np.periode_id AND e.classe_id = cl.id;
                    IF v_rang_actuel < v_rang_cible THEN
                        -- Rattrapage d'avance (l'ancien écrivain a arrêté depuis la reprise précédente).
                        IF v_rang_actuel = 0 THEN
                            UPDATE eva_periode_etat SET etat = 'notes_arretees', origine = 'reprise',
                                   motif = 'reprise M2 : ' || np.statut, date_arret_effective = v_date_arret
                             WHERE etablissement_id = p_etab AND periode_id = np.periode_id AND classe_id = cl.id;
                            v_rang_actuel := 1;
                        END IF;
                        IF v_rang_actuel = 1 AND v_rang_cible >= 2 THEN
                            UPDATE eva_periode_etat SET etat = 'appreciations_closes', origine = 'reprise',
                                   motif = 'reprise M2 : ' || np.statut
                             WHERE etablissement_id = p_etab AND periode_id = np.periode_id AND classe_id = cl.id;
                        END IF;
                        c_per[2] := c_per[2] + 1;
                    ELSE
                        c_per[3] := c_per[3] + 1;
                        IF v_rang_actuel > v_rang_cible THEN
                            PERFORM eva_reprise_noter(v_exec, p_etab, 'periode', np.id, 'ecart', 'plus_avancee_que_l_ancienne',
                                                      'l''état ScolIA est plus avancé que l''ancien statut');
                            c_per[6] := c_per[6] + 1;
                        END IF;
                    END IF;
                ELSE
                    INSERT INTO eva_periode_etat (etablissement_id, periode_id, classe_id, origine, motif)
                    VALUES (p_etab, np.periode_id, cl.id, 'reprise', 'reprise M2');
                    IF v_rang_cible >= 1 THEN
                        UPDATE eva_periode_etat SET etat = 'notes_arretees', origine = 'reprise',
                               motif = 'reprise M2 : ' || np.statut, date_arret_effective = v_date_arret
                         WHERE etablissement_id = p_etab AND periode_id = np.periode_id AND classe_id = cl.id;
                    END IF;
                    IF v_rang_cible >= 2 THEN
                        UPDATE eva_periode_etat SET etat = 'appreciations_closes', origine = 'reprise',
                               motif = 'reprise M2 : ' || np.statut
                         WHERE etablissement_id = p_etab AND periode_id = np.periode_id AND classe_id = cl.id;
                    END IF;
                    c_per[2] := c_per[2] + 1;
                END IF;
            EXCEPTION WHEN unique_violation THEN
                -- F7 : la ligne vient d'être créée par une reprise simultanée.
                c_per[3] := c_per[3] + 1;
            WHEN OTHERS THEN
                c_per[4] := c_per[4] + 1;
                PERFORM eva_reprise_noter(v_exec, p_etab, 'periode', np.id, 'refusee', 'erreur_sql', SQLSTATE);
            END;
        END LOOP;
    END LOOP;

    INSERT INTO eva_reprise_compte (execution_id, etablissement_id, objet, lues, creees, inchangees, refusees, ignorees, ecarts)
    VALUES (v_exec, p_etab, 'reglage', c_reg[1], c_reg[2], c_reg[3], c_reg[4], c_reg[5], c_reg[6]),
           (v_exec, p_etab, 'coefficient', c_coe[1], c_coe[2], c_coe[3], c_coe[4], c_coe[5], c_coe[6]),
           (v_exec, p_etab, 'devoir', c_dev[1], c_dev[2], c_dev[3], c_dev[4], c_dev[5], c_dev[6]),
           (v_exec, p_etab, 'note', c_not[1], c_not[2], c_not[3], c_not[4], c_not[5], c_not[6]),
           (v_exec, p_etab, 'note_sans_devoir', c_nsd[1], c_nsd[2], c_nsd[3], c_nsd[4], c_nsd[5], c_nsd[6]),
           (v_exec, p_etab, 'correction', c_cor[1], c_cor[2], c_cor[3], c_cor[4], c_cor[5], c_cor[6]),
           (v_exec, p_etab, 'appreciation', c_app[1], c_app[2], c_app[3], c_app[4], c_app[5], c_app[6]),
           (v_exec, p_etab, 'periode', c_per[1], c_per[2], c_per[3], c_per[4], c_per[5], c_per[6]);
    UPDATE eva_reprise_execution SET terminee_le = now() WHERE id = v_exec AND etablissement_id = p_etab;
    ALTER TABLE eva_resultat ENABLE TRIGGER eva_resultat_garde;
    ALTER TABLE eva_resultat ENABLE TRIGGER eva_resultat_histo;
    ALTER TABLE eva_coefficient_matiere ENABLE TRIGGER eva_coef_matiere_garde;
    RETURN v_exec;
END
$$;


-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- Garde des résultats : (1) une ligne reprise est modifiable par la grille (sa source devient celle de
-- l'écrivain, une fois) ; (2) un public restreint est respecté en base. Reprend 464 à ces deux ajouts près.
-- ═════════════════════════════════════════════════════════════════════════════════════════════
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

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- V3 « grille et saisie » : retrait de publication, délai maximal, journaux
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- Retrait de publication (EVA-145) : la famille voit « résultats retirés pour vérification », jamais une note
-- qui disparaît sans mot. `publiee_le` redevient NULL (la vue `eva_resultat_publie` ne montre plus rien) et
-- `retiree_le` garde la trace ; une nouvelle publication remet `retiree_le` à NULL.
ALTER TABLE eva_evaluation ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
ALTER TABLE eva_evaluation ADD COLUMN IF NOT EXISTS retiree_par UUID;
-- « En cours de correction » (§10.1, S-111) : la publication automatique au délai maximal l'épargne.
ALTER TABLE eva_evaluation ADD COLUMN IF NOT EXISTS en_cours_de_correction BOOLEAN NOT NULL DEFAULT false;
-- Rappel envoyé au professeur avant la publication automatique (un seul).
ALTER TABLE eva_evaluation ADD COLUMN IF NOT EXISTS rappel_publication_le TIMESTAMPTZ;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'eva_eval_retrait_chk') THEN
        ALTER TABLE eva_evaluation ADD CONSTRAINT eva_eval_retrait_chk
            CHECK ((retiree_le IS NULL) = (retiree_par IS NULL) AND (retiree_le IS NULL OR publiee_le IS NULL));
    END IF;
END $$;

-- Journal des publications : ajout seul. Aucune note, aucun nom (R-V17) ; le motif d'un retrait est du texte
-- interne du professeur, jamais envoyé dans un événement.
CREATE TABLE IF NOT EXISTS eva_evaluation_publication_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evaluation_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (action IN
        ('publiee', 'programmee', 'programmation_annulee', 'retiree', 'publiee_auto_programmee',
         'publiee_auto_delai', 'publiee_auto_arret', 'rappel_delai', 'definie_a_l_arret')),
    prevue_le TIMESTAMPTZ,
    motif TEXT CHECK (motif IS NULL OR char_length(btrim(motif)) BETWEEN 3 AND 500),
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_pub_journal_eval_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_pub_journal_retrait_chk CHECK (action <> 'retiree' OR motif IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS eva_pub_journal_eval_idx ON eva_evaluation_publication_journal (etablissement_id, evaluation_id, le);

-- Journal des imports et collages : ajout seul, aucune donnée d'élève (comptes seulement).
CREATE TABLE IF NOT EXISTS eva_import_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('collage', 'fichier')),
    nb_lignes INTEGER NOT NULL CHECK (nb_lignes >= 0),
    nb_cellules INTEGER NOT NULL CHECK (nb_cellules >= 0),
    nb_ecrites INTEGER NOT NULL CHECK (nb_ecrites >= 0),
    nb_inchangees INTEGER NOT NULL CHECK (nb_inchangees >= 0),
    nb_conflits INTEGER NOT NULL CHECK (nb_conflits >= 0),
    nb_refusees INTEGER NOT NULL CHECK (nb_refusees >= 0),
    par UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_import_journal_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_import_journal_cours_idx ON eva_import_journal (etablissement_id, cours_id, le);

DROP TRIGGER IF EXISTS eva_pub_journal_ajout_seul ON eva_evaluation_publication_journal;
CREATE TRIGGER eva_pub_journal_ajout_seul BEFORE UPDATE OR DELETE ON eva_evaluation_publication_journal
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_pub_journal_vidage ON eva_evaluation_publication_journal;
CREATE TRIGGER eva_pub_journal_vidage BEFORE TRUNCATE ON eva_evaluation_publication_journal
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_import_journal_ajout_seul ON eva_import_journal;
CREATE TRIGGER eva_import_journal_ajout_seul BEFORE UPDATE OR DELETE ON eva_import_journal
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_import_journal_vidage ON eva_import_journal;
CREATE TRIGGER eva_import_journal_vidage BEFORE TRUNCATE ON eva_import_journal
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

SELECT ent_rls_poser(ARRAY['eva_evaluation_publication_journal', 'eva_import_journal']);
