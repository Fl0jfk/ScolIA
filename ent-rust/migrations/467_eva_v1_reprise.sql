-- Socle 21, lot V1 (suite) — 467 : reprise M2 des évaluations existantes vers `eva_*`, idempotente,
-- avec rapport. Additif, rejouable. Aucune colonne JSON.
--
--   ent_devoir            → eva_evaluation   (même identifiant ; cours trouvé par service, sinon par
--                                             public + matière, sinon cours « reprise » à vérifier)
--   ent_note              → eva_resultat     (même identifiant ; DOUBLE PRECISION → NUMERIC(7,3),
--                                             écart d'arrondi listé)
--   ent_note_correction   → eva_resultat_historique (qui, quand, avant, après, motif : conservés)
--   ent_note_appreciation → eva_appreciation (même identifiant ; cours retrouvé par le public)
--   ent_note_periode      → eva_periode_etat (un état par classe de la périodisation ; transitions
--                                             inscrites avec l'origine `reprise`)
--
-- Règles : (1) INSERTION SEULE : une ligne déjà reprise n'est jamais modifiée (l'ancien écrivain peut
-- continuer d'écrire jusqu'à M3 ; une dérive est LISTÉE, pas corrigée en silence) ; (2) rien n'est
-- supprimé dans les anciennes tables ; (3) chaque exécution écrit son rapport : lues = créées +
-- inchangées + refusées + ignorées, en contrainte ; (4) une note sans devoir (voie historique fermée
-- en V0) n'est pas reprise : elle est listée.

CREATE TABLE IF NOT EXISTS eva_reprise_execution (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    origine TEXT NOT NULL CHECK (origine IN ('migration', 'manuel')),
    lancee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    terminee_le TIMESTAMPTZ,
    CONSTRAINT eva_reprise_execution_id_etab_uq UNIQUE (id, etablissement_id)
);

CREATE TABLE IF NOT EXISTS eva_reprise_compte (
    execution_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (objet IN
        ('reglage', 'coefficient', 'devoir', 'note', 'note_sans_devoir', 'correction', 'appreciation', 'periode')),
    lues INTEGER NOT NULL DEFAULT 0 CHECK (lues >= 0),
    creees INTEGER NOT NULL DEFAULT 0 CHECK (creees >= 0),
    inchangees INTEGER NOT NULL DEFAULT 0 CHECK (inchangees >= 0),
    refusees INTEGER NOT NULL DEFAULT 0 CHECK (refusees >= 0),
    ignorees INTEGER NOT NULL DEFAULT 0 CHECK (ignorees >= 0),
    -- Lignes reprises mais signalées (arrondi, hors barème, dérive) : sous-ensemble, non additif.
    ecarts INTEGER NOT NULL DEFAULT 0 CHECK (ecarts >= 0),
    saisi_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (execution_id, objet),
    CONSTRAINT eva_reprise_compte_exec_fk FOREIGN KEY (execution_id, etablissement_id)
        REFERENCES eva_reprise_execution (id, etablissement_id),
    -- L'invariant du rapport : rien ne se perd, rien ne se compte deux fois.
    CONSTRAINT eva_reprise_compte_invariant_chk CHECK (lues = creees + inchangees + refusees + ignorees)
);

CREATE TABLE IF NOT EXISTS eva_reprise_ligne (
    id UUID PRIMARY KEY,
    execution_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL,
    source_id UUID NOT NULL,
    issue TEXT NOT NULL CHECK (issue IN ('refusee', 'ignoree', 'ecart')),
    code TEXT NOT NULL CHECK (code ~ '^[a-z0-9_]{1,60}$'),
    -- Pas de nom, de note ni de texte d'appréciation : des identifiants et des codes (D-036, R-V17).
    detail TEXT CHECK (detail IS NULL OR char_length(detail) <= 200),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_reprise_ligne_exec_fk FOREIGN KEY (execution_id, etablissement_id)
        REFERENCES eva_reprise_execution (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_reprise_ligne_idx ON eva_reprise_ligne (etablissement_id, execution_id, objet);

DROP TRIGGER IF EXISTS eva_reprise_execution_pas_de_suppression ON eva_reprise_execution;
CREATE TRIGGER eva_reprise_execution_pas_de_suppression BEFORE DELETE ON eva_reprise_execution
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_pas_de_suppression();
DROP TRIGGER IF EXISTS eva_reprise_ligne_vidage ON eva_reprise_ligne;
CREATE TRIGGER eva_reprise_ligne_vidage BEFORE TRUNCATE ON eva_reprise_ligne
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_reprise_ligne_ajout_seul ON eva_reprise_ligne;
CREATE TRIGGER eva_reprise_ligne_ajout_seul BEFORE UPDATE OR DELETE ON eva_reprise_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Écrit une ligne de rapport.
CREATE OR REPLACE FUNCTION eva_reprise_noter(p_exec uuid, p_etab uuid, p_objet text, p_source uuid,
                                             p_issue text, p_code text, p_detail text DEFAULT NULL)
RETURNS void LANGUAGE sql AS $$
    INSERT INTO eva_reprise_ligne (id, execution_id, etablissement_id, objet, source_id, issue, code, detail)
    VALUES (gen_random_uuid(), p_exec, p_etab, p_objet, p_source, p_issue, p_code, left(p_detail, 200))
$$;

-- Arrondit un DOUBLE PRECISION à trois décimales, par le chemin décimal exact (jamais de dérive
-- binaire : 14.499999999999998 → 14.5). Rend NULL si la valeur n'est pas finie.
CREATE OR REPLACE FUNCTION eva_dec3(x double precision) RETURNS numeric LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE WHEN x IS NULL OR x = 'Infinity'::float8 OR x = '-Infinity'::float8 OR x = 'NaN'::float8
                THEN NULL ELSE round(x::numeric, 3) END
$$;

-- La reprise d'un établissement. Rend l'identifiant de l'exécution.
-- SECURITY DEFINER (propriétaire des tables) : c'est la SEULE porte d'écriture de `source = 'reprise'` et du
-- journal reconstitué. Elle désactive, le temps de sa transaction, les gardes du résultat et du coefficient
-- (comme `ent_fin_purger_etablissement`, 522) : aucune variable de session ne joue ce rôle. Le contexte
-- d'établissement posé par l'appelant reste obligatoire (la RLS s'applique au propriétaire : FORCE) et
-- l'appel hors de ce contexte est refusé.
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
BEGIN
    IF ent_rls_etablissement() IS NOT NULL AND ent_rls_etablissement() <> p_etab THEN
        RAISE EXCEPTION 'eva_reprendre_existant : établissement hors du contexte' USING ERRCODE = '42501';
    END IF;
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
            EXCEPTION WHEN OTHERS THEN
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
        EXCEPTION WHEN OTHERS THEN
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
        EXCEPTION WHEN OTHERS THEN
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
            -- Dérive : l'ancien écrivain a modifié la note depuis la reprise (listée, jamais corrigée seule).
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
        EXCEPTION WHEN OTHERS THEN
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
        EXCEPTION WHEN OTHERS THEN
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
            EXCEPTION WHEN OTHERS THEN
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

-- (Les ordres ENABLE ci-dessus sont dans la même transaction : une erreur annule aussi la désactivation.)
-- Au démarrage : seuls les établissements qui n'ont encore AUCUNE exécution sont repris (les migrations
-- sont rejouées à chaque démarrage : le rapport ne se répète pas). Les reprises suivantes sont des
-- demandes explicites (`eva_reprendre_existant(établissement, 'manuel')`).
CREATE OR REPLACE FUNCTION eva_reprendre_tout() RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    e uuid;
    n integer := 0;
BEGIN
    FOR e IN
        SELECT etablissement_id FROM ent_devoir
        UNION SELECT etablissement_id FROM ent_eva_regle
        UNION SELECT etablissement_id FROM ent_eva_coef_matiere
        UNION SELECT etablissement_id FROM ent_note
        UNION SELECT etablissement_id FROM ent_note_periode
        UNION SELECT etablissement_id FROM ent_note_appreciation
    LOOP
        IF NOT EXISTS (SELECT 1 FROM eva_reprise_execution x WHERE x.etablissement_id = e) THEN
            PERFORM eva_reprendre_existant(e, 'migration');
            n := n + 1;
        END IF;
    END LOOP;
    RETURN n;
END
$$;

SELECT ent_rls_poser(ARRAY['eva_reprise_execution', 'eva_reprise_compte', 'eva_reprise_ligne']);
