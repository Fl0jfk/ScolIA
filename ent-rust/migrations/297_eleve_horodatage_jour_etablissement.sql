-- Jour d'établissement dans les CONVERSIONS D'HORODATAGE en date (suite de 295 et 296).
-- `X::date` donne le jour UTC (sqlx impose TimeZone=UTC) : entre minuit et 1 h (2 h l'été) à Paris, un
-- horodatage de création ou de modification tombe la veille du jour métier. Chaque fonction ci-dessous est
-- redéfinie à l'identique (CREATE OR REPLACE, signatures inchangées) en convertissant l'horodatage au jour de
-- l'ÉTABLISSEMENT de la ligne : ent_aujourdhui_etab(<établissement>, <horodatage>) (migration 265, fuseau de
-- ent_etablissement_parametre, Europe/Paris par défaut).
-- La fonction de 340/342 (enseignement repris d'une option d'année) est redéfinie par 343 : elle est créée
-- après 297. Idempotent.

-- 280 / 296 : version initiale (effet_du : création convertie au jour de l'établissement, plafonnée à son jour)
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
        least(coalesce(e.date_entree, ent_aujourdhui_etab(e.etablissement_id, e.created_at)), ent_aujourdhui_etab(e.etablissement_id, now())),
        src, now(), acteur
    );
    RETURN vid;
END $$;

-- 282 : reprise des numéros (année de création au jour de l'établissement)
CREATE OR REPLACE FUNCTION ent_eleve_numero_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; r RECORD;
BEGIN
    FOR r IN SELECT etablissement_id, id, created_at FROM ent_eleve WHERE numero IS NULL
             ORDER BY etablissement_id, created_at, id
    LOOP
        UPDATE ent_eleve SET numero = ent_eleve_numero_attribuer(r.etablissement_id, ent_aujourdhui_etab(r.etablissement_id, r.created_at))
        WHERE etablissement_id = r.etablissement_id AND id = r.id AND numero IS NULL;
        n := n + 1;
    END LOOP;
    RETURN n;
END $$;

-- 283 : pont de l'ancien chemin d'écriture d'ent_eleve vers la scolarisation
CREATE OR REPLACE FUNCTION ent_eleve_scolarite_pont() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    jour DATE := ent_jour_etablissement(NEW.etablissement_id);
    derniere ent_eleve_scolarisation;
    code_sortie TEXT;
BEGIN
    IF coalesce(current_setting('ent.scolarite_projection', true), '') = 'on' THEN
        RETURN NULL;
    END IF;
    SELECT * INTO derniere FROM ent_eleve_scolarisation s
    WHERE s.etablissement_id = NEW.etablissement_id AND s.eleve_id = NEW.id
    ORDER BY s.date_entree DESC LIMIT 1;
    IF TG_OP = 'INSERT' THEN
        IF NEW.status = 'preinscrit' AND NEW.date_entree IS NULL THEN
            PERFORM ent_eleve_scolarite_projeter(NEW.etablissement_id, NEW.id);
            RETURN NULL;
        END IF;
        INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree,
            date_sortie, motif_sortie, motif_sortie_libre, source, decide_par)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id,
            coalesce(NEW.date_entree, ent_aujourdhui_etab(NEW.etablissement_id, NEW.created_at), jour),
            'premiere_inscription',
            CASE WHEN NEW.status = 'sorti' OR NEW.date_sortie IS NOT NULL
                 THEN greatest(coalesce(NEW.date_sortie, jour), coalesce(NEW.date_entree, ent_aujourdhui_etab(NEW.etablissement_id, NEW.created_at), jour)) END,
            CASE WHEN NEW.status = 'sorti' OR NEW.date_sortie IS NOT NULL THEN 'autre' END,
            CASE WHEN NEW.status = 'sorti' OR NEW.date_sortie IS NOT NULL THEN left(NEW.motif_sortie, 500) END,
            'heritage', nullif(current_setting('ent.acteur_id', true), '')::uuid);
        RETURN NULL;
    END IF;
    -- UPDATE
    IF derniere.id IS NULL THEN
        IF NEW.status <> 'preinscrit' OR NEW.date_entree IS NOT NULL THEN
            INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree, source)
            VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id,
                    coalesce(NEW.date_entree, jour), 'premiere_inscription', 'heritage');
        END IF;
        RETURN NULL;
    END IF;
    IF NEW.date_entree IS NOT NULL AND NEW.date_entree IS DISTINCT FROM OLD.date_entree
       AND NEW.date_entree IS DISTINCT FROM derniere.date_entree THEN
        UPDATE ent_eleve_scolarisation SET date_entree = NEW.date_entree
        WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
    END IF;
    -- Un décès ne se défait pas par l'ancien chemin (R-E12) : réouverture refusée.
    IF derniere.motif_sortie = 'deces'
       AND ((NEW.date_sortie IS DISTINCT FROM OLD.date_sortie AND NEW.date_sortie IS NULL)
            OR (NEW.status IS DISTINCT FROM OLD.status AND NEW.status <> 'sorti' AND OLD.status = 'sorti')) THEN
        RAISE EXCEPTION 'scolarisation : un décès ne se défait pas par l''ancien chemin' USING ERRCODE = '42501';
    END IF;
    IF NEW.date_sortie IS DISTINCT FROM OLD.date_sortie THEN
        IF NEW.date_sortie IS NULL THEN
            -- Sortie annulée par l'ancien chemin : la scolarisation est rouverte.
            UPDATE ent_eleve_scolarisation SET date_sortie = NULL, motif_sortie = NULL, motif_sortie_libre = NULL,
                destination_uai = NULL, destination_libelle = NULL, destination_type = NULL
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        ELSE
            code_sortie := CASE WHEN NEW.motif_sortie IN ('fin_de_cycle', 'changement_etablissement', 'demenagement',
                'exclusion_definitive', 'abandon', 'fin_obligation', 'instruction_famille', 'etranger', 'deces')
                THEN NEW.motif_sortie ELSE 'autre' END;
            UPDATE ent_eleve_scolarisation SET
                date_sortie = greatest(NEW.date_sortie, date_entree),
                motif_sortie = coalesce(code_sortie, 'autre'),
                motif_sortie_libre = CASE WHEN code_sortie = 'autre' THEN left(NEW.motif_sortie, 500) END
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        END IF;
    ELSIF NEW.motif_sortie IS DISTINCT FROM OLD.motif_sortie AND derniere.date_sortie IS NOT NULL THEN
        code_sortie := CASE WHEN NEW.motif_sortie IN ('fin_de_cycle', 'changement_etablissement', 'demenagement',
            'exclusion_definitive', 'abandon', 'fin_obligation', 'instruction_famille', 'etranger', 'deces')
            THEN NEW.motif_sortie ELSE 'autre' END;
        UPDATE ent_eleve_scolarisation SET motif_sortie = coalesce(code_sortie, 'autre'),
            motif_sortie_libre = CASE WHEN code_sortie = 'autre' THEN left(NEW.motif_sortie, 500) END
        WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
    END IF;
    IF NEW.status IS DISTINCT FROM OLD.status THEN
        IF NEW.status = 'sorti' AND derniere.date_sortie IS NULL AND NEW.date_sortie IS NOT DISTINCT FROM OLD.date_sortie THEN
            -- Bascule de statut vers « sorti » sans date : sortie du jour.
            UPDATE ent_eleve_scolarisation SET date_sortie = greatest(jour, date_entree),
                motif_sortie = coalesce(motif_sortie, 'autre')
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        ELSIF NEW.status <> 'sorti' AND OLD.status = 'sorti' AND derniere.date_sortie IS NOT NULL
              AND NEW.date_sortie IS NOT DISTINCT FROM OLD.date_sortie THEN
            -- Réactivation par l'ancien chemin : la dernière scolarisation est rouverte.
            UPDATE ent_eleve_scolarisation SET date_sortie = NULL, motif_sortie = NULL, motif_sortie_libre = NULL,
                destination_uai = NULL, destination_libelle = NULL, destination_type = NULL
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        END IF;
    END IF;
    PERFORM ent_eleve_scolarite_projeter(NEW.etablissement_id, NEW.id);
    RETURN NULL;
END $$;

-- 283 : reprise des scolarisations existantes
CREATE OR REPLACE FUNCTION ent_eleve_scolarisation_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; e RECORD; entree DATE; sortie DATE; code TEXT; jour DATE;
BEGIN
    FOR e IN SELECT * FROM ent_eleve x
             WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_scolarisation s
                               WHERE s.etablissement_id = x.etablissement_id AND s.eleve_id = x.id)
               AND NOT (x.status = 'preinscrit' AND x.date_entree IS NULL AND x.date_sortie IS NULL)
             ORDER BY x.etablissement_id, x.created_at, x.id
    LOOP
        jour := ent_jour_etablissement(e.etablissement_id);
        entree := coalesce(e.date_entree, ent_aujourdhui_etab(e.etablissement_id, e.created_at));
        sortie := e.date_sortie;
        IF sortie IS NULL AND e.status = 'sorti' THEN
            sortie := greatest(ent_aujourdhui_etab(e.etablissement_id, e.updated_at), entree);
            INSERT INTO ent_eleve_scolarite_ecart (etablissement_id, eleve_id, cause, statut_herite, statut_derive)
            VALUES (e.etablissement_id, e.id, 'sortie_sans_date', e.status, 'sorti') ON CONFLICT DO NOTHING;
        END IF;
        IF sortie IS NOT NULL AND sortie < entree THEN
            INSERT INTO ent_eleve_scolarite_ecart (etablissement_id, eleve_id, cause, statut_herite, statut_derive)
            VALUES (e.etablissement_id, e.id, 'sortie_avant_entree', e.status, NULL) ON CONFLICT DO NOTHING;
            sortie := entree;
        END IF;
        code := CASE WHEN e.motif_sortie IN ('fin_de_cycle', 'changement_etablissement', 'demenagement',
            'exclusion_definitive', 'abandon', 'fin_obligation', 'instruction_famille', 'etranger', 'deces')
            THEN e.motif_sortie ELSE 'autre' END;
        INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree,
            date_sortie, motif_sortie, motif_sortie_libre, source, cree_le)
        VALUES (gen_random_uuid(), e.etablissement_id, e.id, entree, 'reprise', sortie,
            CASE WHEN sortie IS NOT NULL THEN code END,
            CASE WHEN sortie IS NOT NULL AND code = 'autre' THEN left(e.motif_sortie, 500) END,
            'reprise', now());
        n := n + 1;
    END LOOP;
    -- Écarts entre statut hérité et statut dérivé (après la reprise, au jour de l'établissement).
    INSERT INTO ent_eleve_scolarite_ecart (etablissement_id, eleve_id, cause, statut_herite, statut_derive)
    SELECT x.etablissement_id, x.id, 'statut_herite_different', x.status,
           ent_eleve_statut_a(x.etablissement_id, x.id, ent_jour_etablissement(x.etablissement_id))
    FROM ent_eleve x
    WHERE x.statut IS NULL
      AND x.status IS DISTINCT FROM CASE ent_eleve_statut_a(x.etablissement_id, x.id, ent_jour_etablissement(x.etablissement_id))
              WHEN 'scolarise' THEN 'actif' WHEN 'sorti' THEN 'sorti' ELSE 'preinscrit' END
    ON CONFLICT DO NOTHING;
    -- Projections de tous les élèves repris (statut dérivé mis en cache).
    PERFORM ent_eleve_scolarite_projeter(x.etablissement_id, x.id)
    FROM ent_eleve x WHERE x.statut IS NULL;
    RETURN n;
END $$;
