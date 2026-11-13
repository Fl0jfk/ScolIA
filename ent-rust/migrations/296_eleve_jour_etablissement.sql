-- Jour d'établissement dans les fonctions SQL du socle élève (suite de 295).
-- Les fonctions de 280 à 286 prenaient `current_date`, soit le jour du serveur (UTC, sqlx impose
-- TimeZone=UTC) : entre minuit et 1 h (2 h l'été) à Paris, le jour SQL est la veille du jour métier
-- (domain_eleve::horloge::jour_paris). Chaque fonction ci-dessous est redéfinie à l'identique (CREATE OR
-- REPLACE, signatures inchangées) avec le jour de l'ÉTABLISSEMENT concerné (migration 265 :
-- ent_aujourdhui_etab, fuseau de ent_etablissement_parametre, Europe/Paris par défaut), jamais un jour global.
-- La fonction de 340 est redéfinie par 342 (elle ne peut pas l'être ici : 340 s'applique après 296).
-- Idempotent.

-- 280 : version initiale (effet_du plafonné au jour de l'établissement de l'élève)
CREATE OR REPLACE FUNCTION ent_eleve_identite_initiale(e ent_eleve, p_nature text)
 RETURNS uuid
 LANGUAGE plpgsql
AS $$
DECLARE
    vid UUID := gen_random_uuid();
    sx TEXT := ent_eleve_sexe_normalise(e.sexe);
    nom_v TEXT := left(coalesce(nullif(btrim(e.nom), ''), '(à renseigner)'), 120);
    prenom_v TEXT := left(coalesce(nullif(btrim(e.prenom), ''), '(à renseigner)'), 120);
    acteur UUID := nullif(current_setting('ent.acteur_id', true), '')::uuid;
    src TEXT := coalesce(nullif(current_setting('ent.identite_source', true), ''), 'saisie');
BEGIN
    IF p_nature = 'reprise' THEN src := 'reprise'; acteur := NULL; END IF;
    IF e.sexe IS NOT NULL AND btrim(e.sexe) <> '' AND sx IS NULL THEN
        INSERT INTO ent_eleve_identite_a_traiter (etablissement_id, eleve_id, champ, cause, valeur_origine)
        VALUES (e.etablissement_id, e.id, 'sexe', 'valeur_inconnue', left(e.sexe, 40))
        ON CONFLICT DO NOTHING;
    END IF;
    IF nom_v IS DISTINCT FROM e.nom THEN
        INSERT INTO ent_eleve_identite_a_traiter (etablissement_id, eleve_id, champ, cause, valeur_origine)
        VALUES (e.etablissement_id, e.id, 'nom', 'valeur_invalide', left(e.nom, 200))
        ON CONFLICT DO NOTHING;
    END IF;
    IF prenom_v IS DISTINCT FROM e.prenom THEN
        INSERT INTO ent_eleve_identite_a_traiter (etablissement_id, eleve_id, champ, cause, valeur_origine)
        VALUES (e.etablissement_id, e.id, 'prenom', 'valeur_invalide', left(e.prenom, 200))
        ON CONFLICT DO NOTHING;
    END IF;
    INSERT INTO ent_eleve_identite (
        id, etablissement_id, eleve_id, nom_naissance, prenoms, rang_prenom_usuel, sexe,
        date_naissance, nature, effet_du, source, enregistre_le, enregistre_par
    ) VALUES (
        vid, e.etablissement_id, e.id, nom_v, ARRAY[prenom_v], 1, sx, e.date_naissance,
        p_nature,
        least(coalesce(e.date_entree, e.created_at::date), ent_aujourdhui_etab(e.etablissement_id, now())),
        src, now(), acteur
    );
    RETURN vid;
END $$;

-- 280 : resynchronisation quotidienne (sans argument : jour de l'établissement de chaque élève ; un jour explicite reste possible)
CREATE OR REPLACE FUNCTION ent_eleve_identite_resynchroniser(p_jour date DEFAULT NULL)
 RETURNS bigint
 LANGUAGE plpgsql
AS $$
DECLARE n BIGINT := 0; r RECORD; v ent_eleve_identite;
BEGIN
    FOR r IN
        SELECT e.etablissement_id, e.id AS eleve_id,
               ent_eleve_identite_courante(e.etablissement_id, e.id, coalesce(p_jour, ent_aujourdhui_etab(e.etablissement_id, now()))) AS courante
        FROM ent_eleve e
        WHERE e.identite_version_id IS DISTINCT FROM
              ent_eleve_identite_courante(e.etablissement_id, e.id, coalesce(p_jour, ent_aujourdhui_etab(e.etablissement_id, now())))
    LOOP
        CONTINUE WHEN r.courante IS NULL;
        SELECT * INTO v FROM ent_eleve_identite WHERE etablissement_id = r.etablissement_id AND id = r.courante;
        PERFORM ent_eleve_identite_projeter(v);
        n := n + 1;
    END LOOP;
    RETURN n;
END $$;

-- 281 : retrait d'un identifiant
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_retirer(p_etab uuid, p_eleve uuid, p_systeme text, p_motif text)
 RETURNS void
 LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE ent_eleve_identifiant SET statut = 'revoque', valide_au = greatest(valide_du, ent_aujourdhui_etab(p_etab, now())), motif = p_motif
    WHERE etablissement_id = p_etab AND eleve_id = p_eleve AND systeme = p_systeme
      AND statut <> 'revoque' AND valide_au IS NULL;
    PERFORM ent_eleve_identifiant_projeter(p_etab, p_eleve, p_systeme);
END $$;

-- 281 : pont des écritures héritées (INE, badge)
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_pont()
 RETURNS trigger
 LANGUAGE plpgsql
AS $$
DECLARE
    acteur UUID := nullif(current_setting('ent.acteur_id', true), '')::uuid;
    v TEXT;
BEGIN
    IF coalesce(current_setting('ent.identifiant_projection', true), '') = 'on' THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'INSERT' OR NEW.ine IS DISTINCT FROM OLD.ine THEN
        v := nullif(btrim(coalesce(NEW.ine, '')), '');
        IF v IS NULL THEN
            IF TG_OP = 'UPDATE' THEN
                PERFORM ent_eleve_identifiant_retirer(NEW.etablissement_id, NEW.id, 'INE', 'retiré par la saisie héritée');
            END IF;
        ELSIF char_length(v) <= 64 THEN
            PERFORM ent_eleve_identifiant_enregistrer(
                NEW.etablissement_id, NEW.id, 'INE', upper(v), 'saisie', 'declare', ent_aujourdhui_etab(NEW.etablissement_id, now()), acteur, NULL, false);
        END IF;
    END IF;
    IF TG_OP = 'INSERT' OR NEW.numero_badge IS DISTINCT FROM OLD.numero_badge THEN
        v := nullif(btrim(coalesce(NEW.numero_badge, '')), '');
        IF v IS NULL THEN
            IF TG_OP = 'UPDATE' THEN
                PERFORM ent_eleve_identifiant_retirer(NEW.etablissement_id, NEW.id, 'BADGE', 'retiré par la saisie héritée');
            END IF;
        ELSIF char_length(v) <= 64 THEN
            PERFORM ent_eleve_identifiant_enregistrer(
                NEW.etablissement_id, NEW.id, 'BADGE', v, 'badge', 'confirme', ent_aujourdhui_etab(NEW.etablissement_id, now()), acteur, NULL, false);
        END IF;
    END IF;
    RETURN NULL;
END $$;

-- 281 : reprise de l'existant (chaque identifiant repris prend le jour de son établissement)
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_reprendre()
 RETURNS bigint
 LANGUAGE plpgsql
AS $$
DECLARE n BIGINT := 0; k BIGINT;
BEGIN
    WITH src AS (
        SELECT e.etablissement_id, e.id AS eleve_id, upper(btrim(e.ine)) AS valeur,
               count(*) OVER (PARTITION BY e.etablissement_id, lower(btrim(e.ine))) AS nb
        FROM ent_eleve e
        WHERE nullif(btrim(coalesce(e.ine, '')), '') IS NOT NULL AND char_length(btrim(e.ine)) <= 64
          AND NOT EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                          WHERE i.etablissement_id = e.etablissement_id AND i.eleve_id = e.id AND i.systeme = 'INE')
    )
    INSERT INTO ent_eleve_identifiant (id, etablissement_id, eleve_id, systeme, valeur, statut, provenance,
                                       valide_du, motif, enregistre_le)
    SELECT gen_random_uuid(), etablissement_id, eleve_id, 'INE', valeur,
           CASE WHEN nb > 1 THEN 'conteste' ELSE 'a_verifier' END, 'reprise_logiciel', ent_aujourdhui_etab(etablissement_id, now()),
           CASE WHEN nb > 1 THEN 'INE porté par plusieurs élèves à la reprise' END, now()
    FROM src;
    GET DIAGNOSTICS k = ROW_COUNT; n := n + k;
    WITH src AS (
        SELECT e.etablissement_id, e.id AS eleve_id, btrim(e.numero_badge) AS valeur,
               count(*) OVER (PARTITION BY e.etablissement_id, lower(btrim(e.numero_badge))) AS nb
        FROM ent_eleve e
        WHERE nullif(btrim(coalesce(e.numero_badge, '')), '') IS NOT NULL AND char_length(btrim(e.numero_badge)) <= 64
          AND NOT EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                          WHERE i.etablissement_id = e.etablissement_id AND i.eleve_id = e.id AND i.systeme = 'BADGE')
    )
    INSERT INTO ent_eleve_identifiant (id, etablissement_id, eleve_id, systeme, valeur, statut, provenance,
                                       valide_du, confirme_le, confirme_par_source, motif, enregistre_le)
    SELECT gen_random_uuid(), etablissement_id, eleve_id, 'BADGE', valeur,
           CASE WHEN nb > 1 THEN 'conteste' ELSE 'confirme' END, 'badge', ent_aujourdhui_etab(etablissement_id, now()),
           CASE WHEN nb = 1 THEN now() END, CASE WHEN nb = 1 THEN 'badge' END,
           CASE WHEN nb > 1 THEN 'badge porté par plusieurs élèves à la reprise' END, now()
    FROM src;
    GET DIAGNOSTICS k = ROW_COUNT; n := n + k;
    RETURN n;
END $$;

-- 282 : numéro d'élève (année du numéro prise au jour de l'établissement)
CREATE OR REPLACE FUNCTION ent_eleve_numero_avant_insert()
 RETURNS trigger
 LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.numero IS NULL AND NOT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = NEW.id) THEN
        NEW.numero := ent_eleve_numero_attribuer(NEW.etablissement_id, ent_aujourdhui_etab(NEW.etablissement_id, now()));
    END IF;
    RETURN NEW;
END $$;

-- 284 : inscription reprise de l'ancienne table (année future ou non)
CREATE OR REPLACE FUNCTION ent_eleve_inscription_depuis_legacy(l ent_eleve_scolarite_annee)
 RETURNS uuid
 LANGUAGE plpgsql
AS $$
DECLARE
    v_annee UUID; v_an ent_annee_scolaire; v_niveau UUID; v_mef UUID; v_scol UUID; v_exist UUID;
    v_etat TEXT; v_source TEXT; v_y INT; v_id UUID;
BEGIN
    PERFORM set_config('ent.inscription_pont', 'on', true);
    IF l.annee_scolaire ~ '^[0-9]{4}-[0-9]{4}$'
       AND substr(l.annee_scolaire, 6, 4)::int = substr(l.annee_scolaire, 1, 4)::int + 1 THEN
        SELECT a.id INTO v_annee FROM ent_annee_scolaire a
        WHERE a.etablissement_id = l.etablissement_id AND a.libelle = l.annee_scolaire;
        IF v_annee IS NULL THEN
            v_y := substr(l.annee_scolaire, 1, 4)::int;
            IF make_date(v_y, 9, 1) > ent_aujourdhui_etab(l.etablissement_id, now()) THEN
                INSERT INTO ent_eleve_inscription_a_traiter (etablissement_id, eleve_id, legacy_id, cause)
                VALUES (l.etablissement_id, l.eleve_id, l.id, 'annee_future') ON CONFLICT DO NOTHING;
                PERFORM set_config('ent.inscription_pont', 'off', true);
                RETURN NULL;
            END IF;
            v_annee := ent_assurer_annee(l.etablissement_id, make_date(v_y, 9, 15));
        END IF;
    END IF;
    IF v_annee IS NULL THEN
        INSERT INTO ent_eleve_inscription_a_traiter (etablissement_id, eleve_id, legacy_id, cause)
        VALUES (l.etablissement_id, l.eleve_id, l.id, 'annee_illisible') ON CONFLICT DO NOTHING;
        PERFORM set_config('ent.inscription_pont', 'off', true);
        RETURN NULL;
    END IF;
    SELECT * INTO v_an FROM ent_annee_scolaire a WHERE a.id = v_annee;
    SELECT s.id INTO v_scol FROM ent_eleve_scolarisation s
    WHERE s.etablissement_id = l.etablissement_id AND s.eleve_id = l.eleve_id
    ORDER BY (s.date_entree < v_an.date_fin AND (s.date_sortie IS NULL OR s.date_sortie > v_an.date_debut)) DESC,
             s.date_entree DESC
    LIMIT 1;
    IF v_scol IS NULL THEN
        INSERT INTO ent_eleve_inscription_a_traiter (etablissement_id, eleve_id, legacy_id, cause)
        VALUES (l.etablissement_id, l.eleve_id, l.id, 'sans_scolarisation') ON CONFLICT DO NOTHING;
        PERFORM set_config('ent.inscription_pont', 'off', true);
        RETURN NULL;
    END IF;
    SELECT n.id INTO v_niveau FROM ent_niveau n
    WHERE n.etablissement_id = l.etablissement_id AND upper(n.code) = upper(btrim(coalesce(l.niveau, '')));
    SELECT m.id INTO v_mef FROM ent_mef m
    WHERE m.etablissement_id = l.etablissement_id AND m.code_mef = upper(btrim(coalesce(l.mef, '')));
    IF (l.niveau IS NOT NULL AND btrim(l.niveau) <> '' AND v_niveau IS NULL) THEN
        INSERT INTO ent_eleve_inscription_a_traiter (etablissement_id, eleve_id, legacy_id, cause)
        VALUES (l.etablissement_id, l.eleve_id, l.id, 'niveau_inconnu') ON CONFLICT DO NOTHING;
    END IF;
    IF (l.mef IS NOT NULL AND btrim(l.mef) <> '' AND v_mef IS NULL) THEN
        INSERT INTO ent_eleve_inscription_a_traiter (etablissement_id, eleve_id, legacy_id, cause)
        VALUES (l.etablissement_id, l.eleve_id, l.id, 'mef_inconnu') ON CONFLICT DO NOTHING;
    END IF;
    v_etat := CASE l.statut WHEN 'historique' THEN 'terminee' WHEN 'courante' THEN 'en_cours'
                            WHEN 'projetee' THEN 'prevue' ELSE 'confirmee' END;
    v_source := CASE l.source WHEN 'montee_rapide' THEN 'montee' WHEN 'reinscription_parent' THEN 'reinscription'
                              WHEN 'import' THEN 'import' ELSE 'saisie' END;
    SELECT i.id INTO v_exist FROM ent_eleve_inscription i
    WHERE i.eleve_id = l.eleve_id AND i.annee_id = v_annee AND i.etat <> 'annulee';
    IF v_exist IS NOT NULL THEN
        UPDATE ent_eleve_inscription SET legacy_id = coalesce(legacy_id, l.id), niveau_id = coalesce(v_niveau, niveau_id),
            mef_id = coalesce(v_mef, mef_id), etat = v_etat, modifie_le = now()
        WHERE id = v_exist;
        v_id := v_exist;
    ELSE
        INSERT INTO ent_eleve_inscription (id, etablissement_id, eleve_id, scolarisation_id, annee_id, niveau_id, mef_id,
            etat, source, legacy_id, date_debut, cree_le)
        VALUES (gen_random_uuid(), l.etablissement_id, l.eleve_id, v_scol, v_annee, v_niveau, v_mef,
            v_etat, v_source, l.id, v_an.date_debut, l.created_at)
        ON CONFLICT (legacy_id) WHERE legacy_id IS NOT NULL DO UPDATE SET
            niveau_id = coalesce(EXCLUDED.niveau_id, ent_eleve_inscription.niveau_id),
            mef_id = coalesce(EXCLUDED.mef_id, ent_eleve_inscription.mef_id),
            etat = EXCLUDED.etat, modifie_le = now()
        RETURNING id INTO v_id;
    END IF;
    PERFORM set_config('ent.inscription_pont', 'off', true);
    RETURN v_id;
END $$;

-- 286 : option d'année vers affectation
CREATE OR REPLACE FUNCTION ent_eleve_option_annee_vers_affectation(o ent_eleve_option_annee)
 RETURNS uuid
 LANGUAGE plpgsql
AS $$
DECLARE v_an ent_annee_scolaire; v_groupe UUID; v_aff UUID;
BEGIN
    SELECT affectation_id INTO v_aff FROM ent_eleve_option_groupe WHERE source = 'option_annee' AND option_id = o.id;
    IF o.statut = 'proposee' THEN RETURN NULL; END IF;
    IF o.statut = 'retiree' THEN
        IF v_aff IS NOT NULL THEN
            UPDATE ent_affectation SET date_fin = least(coalesce(date_fin, ent_aujourdhui_etab(o.etablissement_id, now())), greatest(date_debut, ent_aujourdhui_etab(o.etablissement_id, now())))
            WHERE id = v_aff AND etablissement_id = o.etablissement_id AND (date_fin IS NULL OR date_fin > ent_aujourdhui_etab(o.etablissement_id, now()));
        END IF;
        RETURN v_aff;
    END IF;
    IF v_aff IS NOT NULL THEN RETURN v_aff; END IF;
    SELECT a.* INTO v_an FROM ent_annee_scolaire a
    WHERE a.etablissement_id = o.etablissement_id AND a.libelle = o.annee_scolaire;
    IF v_an.id IS NOT NULL AND v_an.date_debut > ent_aujourdhui_etab(o.etablissement_id, now()) THEN
        RETURN NULL;
    END IF;
    IF v_an.id IS NULL OR btrim(coalesce(o.code, '')) = '' THEN
        INSERT INTO ent_eleve_option_a_traiter (etablissement_id, eleve_id, source, option_id, cause)
        VALUES (o.etablissement_id, o.eleve_id, 'option_annee', o.id,
                CASE WHEN v_an.id IS NULL THEN 'annee_inconnue' ELSE 'code_vide' END) ON CONFLICT DO NOTHING;
        RETURN NULL;
    END IF;
    v_groupe := ent_eleve_option_groupe_assurer(o.etablissement_id, v_an.id, o.code, o.libelle);
    v_aff := gen_random_uuid();
    INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, date_fin, source)
    VALUES (v_aff, o.etablissement_id, v_an.id, o.eleve_id, v_groupe, v_an.date_debut, v_an.date_fin, 'saisie');
    INSERT INTO ent_eleve_option_groupe (source, option_id, etablissement_id, eleve_id, groupe_id, affectation_id)
    VALUES ('option_annee', o.id, o.etablissement_id, o.eleve_id, v_groupe, v_aff);
    RETURN v_aff;
END $$;

-- 286 : comparaison des options
CREATE OR REPLACE FUNCTION ent_eleve_option_comparaison(p_etab uuid)
 RETURNS TABLE(heritees bigint, reprises bigint, a_traiter bigint, en_attente bigint)
 LANGUAGE sql
 STABLE
AS $$
    SELECT
        (SELECT count(*) FROM ent_eleve_option WHERE etablissement_id = p_etab)
          + (SELECT count(*) FROM ent_eleve_option_annee WHERE etablissement_id = p_etab AND statut = 'retenue'),
        (SELECT count(*) FROM ent_eleve_option_groupe WHERE etablissement_id = p_etab),
        (SELECT count(*) FROM ent_eleve_option_a_traiter WHERE etablissement_id = p_etab AND traite_le IS NULL),
        (SELECT count(*) FROM ent_eleve_option_annee o JOIN ent_annee_scolaire a
            ON a.etablissement_id = o.etablissement_id AND a.libelle = o.annee_scolaire
          WHERE o.etablissement_id = p_etab AND o.statut = 'retenue' AND a.date_debut > ent_aujourdhui_etab(p_etab, now()))
$$;
