-- Socle 21, lot V3b — 472 : M2-M3 du plan de migration (§22). Les LECTEURS passent sur `eva_*` : les anciennes tables
-- `ent_devoir` et `ent_note` sont RENOMMÉES `ent_devoir_archive` et `ent_note_archive` (aucune perte : leurs données sont
-- déjà reprises dans `eva_*` par la reprise M2, rejouée ici une dernière fois), et des VUES en lecture seule portent
-- désormais leur nom, alimentées par `eva_*` (source unique ; ni écriture miroir, ni double source).
--
-- * `ent_devoir` (vue) : évaluations OUVERTES en mode note, forme exacte de l'ancienne table ; `publie_le` = publication.
-- * `ent_note` (vue) : résultats des évaluations ouvertes, forme exacte de l'ancienne table + colonne `publiee` (l'évaluation
--   est-elle publiée ?). Les lecteurs qui répondent à une famille ou à un élève DOIVENT filtrer sur `publiee` (R-V7) ; les
--   lecteurs du personnel (conseil, bulletin) passent par le moteur unique de V2, pas par cette vue.
-- * `ent_note_periode` RESTE une table : c'est le registre « période de notes » (création par POST /note-periodes) et la cible
--   des clés étrangères des bulletins, conseils, appréciations et du journal de statut. Sa colonne `statut` n'est plus une
--   source : elle est GELÉE (déclencheur) et le statut se lit dans `ent_note_periode_v` (468), dérivé de `eva_periode_etat`.
-- * Les clés étrangères du cahier de textes vers `ent_note` sont retirées (on ne référence pas une vue) ; le cahier contrôle
--   déjà l'existence de la note à l'écriture (C-D8).
-- * `eva_reprendre_existant` et `eva_reprendre_tout` sont remplacées (471 et 467 intactes, règle 25) : elles lisent les
--   archives ; correctif M2 de la revue V3 (la recopie des corrections est isolée dans un sous-bloc : une correction ancienne
--   invalide est listée « refusée » sans annuler la reprise) ; F8 : contexte d'établissement obligatoire, EXECUTE retiré à PUBLIC.

-- ── 1. Archives : renommage sans perte (idempotent) ─────────────────────────────────────────────────────────────────────
DO $$
BEGIN
    IF to_regclass('ent_devoir_archive') IS NULL AND (SELECT c.relkind FROM pg_class c WHERE c.oid = to_regclass('ent_devoir')) = 'r' THEN
        ALTER TABLE ent_devoir RENAME TO ent_devoir_archive;
    END IF;
    IF to_regclass('ent_note_archive') IS NULL AND (SELECT c.relkind FROM pg_class c WHERE c.oid = to_regclass('ent_note')) = 'r' THEN
        ALTER TABLE ent_note RENAME TO ent_note_archive;
    END IF;
END
$$;

-- Les clés étrangères des AUTRES tables (cahier de textes) vers les archives sont retirées : leurs notes vivent désormais
-- dans `eva_resultat`. Restent les clés internes aux archives (note → devoir, correction → note).
DO $$
DECLARE
    c RECORD;
BEGIN
    FOR c IN
        SELECT k.conrelid::regclass::text AS tbl, k.conname
          FROM pg_constraint k
         WHERE k.contype = 'f'
           AND k.confrelid IN ('ent_note_archive'::regclass, 'ent_devoir_archive'::regclass)
           AND k.conrelid NOT IN ('ent_note_archive'::regclass, 'ent_devoir_archive'::regclass, 'ent_note_correction'::regclass)
    LOOP
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', c.tbl, c.conname);
    END LOOP;
END
$$;

-- ── 2. `ent_note_periode.statut` n'est plus une source (le statut se lit dans `eva_periode_etat`) ───────────────────────
CREATE OR REPLACE FUNCTION ent_trg_note_periode_statut_fige() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.statut IS DISTINCT FROM OLD.statut THEN
        RAISE EXCEPTION 'ent_note_periode.statut est figé : le statut d''une période se change par la machine d''états (eva_periode_etat), jamais ici'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_note_periode_statut_fige ON ent_note_periode;
CREATE TRIGGER ent_note_periode_statut_fige BEFORE UPDATE ON ent_note_periode
    FOR EACH ROW EXECUTE FUNCTION ent_trg_note_periode_statut_fige();

-- ── 3. Reprise : lit les archives (CREATE OR REPLACE ; 467 et 471 intactes) ─────────────────────────────────────────────
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
    v_k_ok int;
    v_code text;
BEGIN
    -- F8 (revue V3) : le contexte d'établissement est OBLIGATOIRE et doit être celui demandé. Avant, une session sans
    -- contexte (NULL) passait et pouvait reprendre n'importe quel établissement.
    IF ent_rls_etablissement() IS DISTINCT FROM p_etab THEN
        RAISE EXCEPTION 'eva_reprendre_existant : contexte d''établissement absent ou différent' USING ERRCODE = '42501';
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
          FROM ent_devoir_archive dv
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
    FOR n IN SELECT id FROM ent_note_archive WHERE etablissement_id = p_etab AND devoir_id IS NULL ORDER BY created_at, id LOOP
        c_nsd[1] := c_nsd[1] + 1;
        c_nsd[5] := c_nsd[5] + 1;
        PERFORM eva_reprise_noter(v_exec, p_etab, 'note_sans_devoir', n.id, 'ignoree', 'note_sans_devoir',
                                  'à rattacher à une évaluation par la direction');
    END LOOP;

    -- ── Résultats (notes) ─────────────────────────────────────────────────────────────
    FOR n IN
        SELECT nt.* FROM ent_note_archive nt
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
                    -- M2 (revue V3) : la recopie des corrections est ISOLÉE dans un sous-bloc. Une correction ancienne
                    -- invalide (statut inconnu, valeur hors bornes, contrainte refusée) annule SEULE la recopie de cette
                    -- note (rien n'est écrit pour elle), est listée « refusée » et n'interrompt ni les autres notes ni la
                    -- reprise. Mêmes contrôles qu'à l'insertion d'une note (statut dans la liste, valeur bornée).
                    BEGIN
                    v_k_ok := 0;
                    FOR k IN SELECT * FROM ent_note_correction c
                              WHERE c.etablissement_id = p_etab AND c.note_id = n.id
                              ORDER BY c.corrige_le, c.id OFFSET v_deja
                    LOOP
                        IF k.nouveau_statut NOT IN ('note', 'absent', 'a_rattraper', 'dispense', 'non_note') THEN
                            RAISE EXCEPTION 'correction_statut_inconnu' USING ERRCODE = 'P0001';
                        END IF;
                        IF k.nouveau_statut = 'note' AND (eva_dec3(k.nouvelle_valeur) IS NULL
                           OR eva_dec3(k.nouvelle_valeur) < 0 OR eva_dec3(k.nouvelle_valeur) > 9999) THEN
                            RAISE EXCEPTION 'correction_valeur_invalide' USING ERRCODE = 'P0001';
                        END IF;
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
                        v_k_ok := v_k_ok + 1;
                    END LOOP;
                    PERFORM eva_reprise_noter(v_exec, p_etab, 'note', n.id, 'ecart', 'correction_recopiee',
                                              'correction(s) de l''ancienne grille recopiée(s) dans l''historique');
                    c_not[6] := c_not[6] + 1;
                    c_cor[1] := c_cor[1] + v_k_ok;
                    c_cor[2] := c_cor[2] + v_k_ok;
                    EXCEPTION WHEN OTHERS THEN
                        -- Le détail ne contient que le code : jamais une valeur de donnée.
                        v_code := CASE WHEN SQLERRM IN ('correction_statut_inconnu', 'correction_valeur_invalide')
                                       THEN SQLERRM ELSE 'erreur_sql' END;
                        PERFORM eva_reprise_noter(v_exec, p_etab, 'correction', n.id, 'refusee', v_code,
                                                  CASE WHEN v_code = 'erreur_sql' THEN SQLSTATE END);
                        c_cor[1] := c_cor[1] + 1;
                        c_cor[4] := c_cor[4] + 1;
                    END;
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

-- Reprise de tous les établissements : un contexte d'établissement est posé pour chacun (F8 : plus de reprise sans contexte).
CREATE OR REPLACE FUNCTION eva_reprendre_tout() RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    e uuid;
    n integer := 0;
BEGIN
    FOR e IN
        SELECT etablissement_id FROM ent_devoir_archive
        UNION SELECT etablissement_id FROM ent_eva_regle
        UNION SELECT etablissement_id FROM ent_eva_coef_matiere
        UNION SELECT etablissement_id FROM ent_note_archive
        UNION SELECT etablissement_id FROM ent_note_periode
        UNION SELECT etablissement_id FROM ent_note_appreciation
    LOOP
        IF NOT EXISTS (SELECT 1 FROM eva_reprise_execution x WHERE x.etablissement_id = e) THEN
            PERFORM set_config('app.etablissement_id', e::text, true);
            PERFORM eva_reprendre_existant(e, 'migration');
            n := n + 1;
        END IF;
    END LOOP;
    PERFORM set_config('app.etablissement_id', '', true);
    RETURN n;
END
$$;

-- F8 : ni l'un ni l'autre ne s'appellent depuis un rôle quelconque : le propriétaire (le rôle de l'application et des
-- migrations) les appelle ; aucun autre rôle sans GRANT explicite.
REVOKE ALL ON FUNCTION eva_reprendre_existant(uuid, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION eva_reprendre_tout() FROM PUBLIC;

-- Dernière reprise de l'existant : les données écrites par les anciens écrivains depuis la reprise (ou la première
-- exécution) rejoignent `eva_*` AVANT que les anciennes tables ne deviennent des archives lues par personne. Insertion
-- seule, idempotente ; une ligne refusée est listée dans le rapport, jamais perdue en silence (elle reste dans l'archive).
DO $$
DECLARE
    e uuid;
BEGIN
    FOR e IN
        SELECT etablissement_id FROM ent_devoir_archive
        UNION SELECT etablissement_id FROM ent_note_archive
    LOOP
        PERFORM set_config('app.etablissement_id', e::text, true);
        PERFORM eva_reprendre_existant(e, 'migration');
    END LOOP;
    PERFORM set_config('app.etablissement_id', '', true);
END
$$;

-- ── 3 bis. Période du référentiel d'une période de notes, POUR UNE CLASSE ──────────────────────────────────────────────
-- Une période de notes (`ent_note_periode`, plage de dates) correspond, dans la périodisation PROPRE à chaque classe
-- (R-V29, CE-65 : trimestres pour les unes, semestres pour d'autres), à la période du référentiel qui la recouvre le plus ;
-- on ne se fie pas au seul lien `ent_note_periode.periode_id` (posé par la reprise, parfois absent ou d'une autre
-- périodisation). Dans une périodisation les périodes ne se chevauchent pas : une seule candidate, la plus recouvrante ;
-- à recouvrement égal (données de labo qui se chevauchent), la plus ancienne, comme `eva_periode_eleve_jour`.
CREATE OR REPLACE FUNCTION ent_periode_de_note_pour_classe(p_etab uuid, p_note_periode uuid, p_classe uuid)
RETURNS uuid LANGUAGE sql STABLE AS $$
    SELECT p.id
      FROM ent_note_periode np
      JOIN ent_classe c ON c.etablissement_id = np.etablissement_id AND c.id = p_classe
      JOIN ent_periode p ON p.etablissement_id = np.etablissement_id AND p.annee_id = c.annee_id
                        AND p.periodisation_id = ent_periodisation_pour_classe(np.etablissement_id, c.id)
     WHERE np.etablissement_id = p_etab AND np.id = p_note_periode
       AND p.date_debut <= np.date_fin AND p.date_fin >= np.date_debut
     ORDER BY (least(p.date_fin, np.date_fin) - greatest(p.date_debut, np.date_debut)) DESC, p.date_debut, p.id
     LIMIT 1
$$;

-- Statut d'une période de notes : le MOINS avancé des classes de l'année qu'elle recouvre (une classe sans état compte
-- `ouverte`, une classe dont la périodisation n'a aucune période recouvrante est ignorée) ; sans classe, l'ancien statut
-- (gelé) fait foi. Remplace la vue de 468 (même forme, mêmes colonnes dans le même ordre).
DROP VIEW IF EXISTS ent_note_periode_v;
CREATE VIEW ent_note_periode_v WITH (security_invoker = true) AS
SELECT np.id, np.etablissement_id, np.code, np.libelle, np.date_debut, np.date_fin,
       COALESCE(CASE ag.rang_min WHEN 0 THEN 'ouverte' WHEN 1 THEN 'partielle_appreciations' ELSE 'fermee' END,
                np.statut) AS statut,
       np.created_at, np.periode_id,
       COALESCE(ag.rang_min >= 2, np.statut = 'fermee') AS appreciations_closes
  FROM ent_note_periode np
  LEFT JOIN LATERAL (
        SELECT min(COALESCE(eva_etat_rang(e.etat), 0)) AS rang_min
          FROM ent_classe c
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
                                   AND y.date_debut <= np.date_fin AND y.date_fin >= np.date_debut
          JOIN LATERAL (SELECT ent_periode_de_note_pour_classe(np.etablissement_id, np.id, c.id) AS pid) pe ON pe.pid IS NOT NULL
          LEFT JOIN eva_periode_etat e
            ON e.etablissement_id = c.etablissement_id AND e.periode_id = pe.pid AND e.classe_id = c.id
         WHERE c.etablissement_id = np.etablissement_id) ag ON true;

-- ── 4. Vues de compatibilité (lecture seule, `security_invoker` : la RLS de l'appelant s'applique) ──────────────────────
-- Les vues `_v` de 468 pour `ent_devoir` et `ent_note` sont remplacées par les vues de même nom que les anciennes tables.
DROP VIEW IF EXISTS ent_devoir_v;
DROP VIEW IF EXISTS ent_note_v;
DROP VIEW IF EXISTS ent_devoir;
DROP VIEW IF EXISTS ent_note;

CREATE VIEW ent_devoir WITH (security_invoker = true) AS
SELECT ev.id, ev.etablissement_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ev.etablissement_id AND np.date_debut <= ev.date_eval AND np.date_fin >= ev.date_eval
         ORDER BY (np.periode_id = ev.periode_id) DESC, np.date_debut, np.created_at, np.id LIMIT 1) AS periode_id,
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
CREATE VIEW ent_note WITH (security_invoker = true) AS
SELECT r.id, r.etablissement_id, r.eleve_id, co.matiere_id,
       COALESCE(r.valeur, 0)::double precision AS valeur,
       ev.bareme::double precision AS sur,
       ev.date_eval AS date_note,
       r.saisi_par AS auteur_user_id,
       ev.titre AS libelle,
       r.cree_le AS created_at,
       r.evaluation_id AS devoir_id,
       (SELECT np.id FROM ent_note_periode np
         WHERE np.etablissement_id = ev.etablissement_id AND np.date_debut <= ev.date_eval AND np.date_fin >= ev.date_eval
         ORDER BY (np.periode_id = ev.periode_id) DESC, np.date_debut, np.created_at, np.id LIMIT 1) AS periode_id,
       CASE WHEN r.statut IN ('non_rendu', 'annule') THEN 'non_note' ELSE r.statut END AS statut,
       r.saisi_le AS updated_at,
       r.saisi_par AS updated_by_user_id,
       (ev.publiee_le IS NOT NULL) AS publiee
  FROM eva_resultat r
  JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
  JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
 WHERE ev.mode = 'note' AND ev.etat = 'ouverte';


-- ── 5. Rattachement des classes : `ent_devoir` n'est plus une table (112 remplacée, intacte) ──────────────────────────────
CREATE OR REPLACE FUNCTION ent_rattacher_classes(p_etab uuid DEFAULT NULL)
RETURNS TABLE (o_table text, o_rattachees bigint) AS $$
DECLARE
    n bigint;
BEGIN
    UPDATE ent_eleve t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, current_date)
     WHERE t.classe_id IS NULL AND btrim(coalesce(t.classe, '')) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_eleve'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_eleve_periode t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.valeur, t.date_debut)
     WHERE t.kind = 'classe' AND t.classe_id IS NULL AND btrim(t.valeur) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_eleve_periode'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_eleve_scolarite_annee t
       SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, ent_jour_de_annee_label(t.annee_scolaire))
     WHERE t.classe_id IS NULL AND btrim(coalesce(t.classe, '')) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_eleve_scolarite_annee'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_appel t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.jour)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_appel'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_edt_creneau t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, current_date)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_edt_creneau'; o_rattachees := n; RETURN NEXT;

    -- `ent_devoir` est désormais une vue dérivée de `eva_evaluation` (V3b) : sa classe vient du cours, il ne figure plus au rapport.

    UPDATE ent_cahier_texte t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.pour_le)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_cahier_texte'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_conseil_classe t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.jour)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_conseil_classe'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_annonce t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.cible_classe, t.created_at::date)
     WHERE t.classe_id IS NULL AND btrim(coalesce(t.cible_classe, '')) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_annonce'; o_rattachees := n; RETURN NEXT;
END
$$ LANGUAGE plpgsql;
