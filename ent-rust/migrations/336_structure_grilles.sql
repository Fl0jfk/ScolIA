-- Socle 5, lot S7 (`docs/socles/05_STRUCTURE.md` §9.3, STR-34 et STR-35) : grille horaire VERSIONNÉE.
--
-- Une grille est une VERSION (`ent_grille`) : créneaux de la journée (cours, récréations, pause
-- méridienne, étude), coupure des demi-journées, valable à partir d'une date. Les versions d'une
-- même grille partagent leur `code` et ne se chevauchent pas dans le temps. La grille s'applique à
-- des CIBLES (`ent_grille_cible`) : un site, un niveau, un site et un niveau, des jours de la
-- semaine, ou tout l'établissement. Pour (date, site, niveau, jour), la cible la plus précise gagne
-- (`ent_grille_pour`, une seule implémentation, lue par le port Rust `structure_grilles::structure`).
--
-- « Sans casser l'existant » :
--   * l'ancienne table plate `ent_grille_horaire` (110, lue par la génération des séances et par
--     les anciennes routes) n'est PAS modifiée ; elle est REPRISE dans le nouveau modèle (une
--     version « REPRISE » valable depuis 2000-01-01 par site), les lignes invalides sont laissées
--     de côté ; le service projette la version en vigueur vers elle (cf. documentation du lot) ;
--   * une version déjà commencée (valable_du antérieure à aujourd'hui) est GELÉE par la base :
--     ses créneaux, ses cibles, sa coupure et sa date de début ne changent plus ; on versionne
--     (nouvelle version à une date) au lieu de modifier le passé ; seule sa date de fin peut être
--     posée (clôture par la version suivante).
--
-- Additif et rejouable.

CREATE EXTENSION IF NOT EXISTS btree_gist;
CREATE UNIQUE INDEX IF NOT EXISTS ent_site_id_etab_uq ON ent_site (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_niveau_id_etab_uq ON ent_niveau (id, etablissement_id);

-- ───────────────────────── Versions de grille ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_grille (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (char_length(btrim(code)) BETWEEN 1 AND 40),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 120),
    valable_du DATE NOT NULL,
    valable_au DATE,
    -- Heure murale à partir de laquelle on est « l'après-midi » (décompte en demi-journées).
    coupure_demi_journee TIME NOT NULL DEFAULT '12:30',
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 300),
    origine TEXT NOT NULL DEFAULT 'service' CHECK (origine IN ('service', 'reprise')),
    archivee_le TIMESTAMPTZ,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_grille_dates_chk CHECK (valable_au IS NULL OR valable_au >= valable_du),
    CONSTRAINT ent_grille_id_etab_uq UNIQUE (id, etablissement_id),
    -- Les versions d'une même grille ne se chevauchent pas.
    CONSTRAINT ent_grille_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, code WITH =, daterange(valable_du, valable_au, '[]') WITH &&)
        WHERE (archivee_le IS NULL)
);
CREATE INDEX IF NOT EXISTS ent_grille_etab_idx ON ent_grille (etablissement_id, valable_du);

CREATE TABLE IF NOT EXISTS ent_grille_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    grille_id UUID NOT NULL,
    -- Les deux nuls = tout l'établissement.
    site_id UUID,
    niveau_id UUID,
    -- Jours ISO (1 = lundi … 7 = dimanche) ; NULL = tous les jours.
    jours SMALLINT[] CHECK (jours IS NULL
        OR (cardinality(jours) BETWEEN 1 AND 7 AND jours <@ ARRAY[1,2,3,4,5,6,7]::SMALLINT[])),
    CONSTRAINT ent_grille_cible_grille_fk FOREIGN KEY (grille_id, etablissement_id)
        REFERENCES ent_grille (id, etablissement_id) ON DELETE CASCADE,
    CONSTRAINT ent_grille_cible_site_fk FOREIGN KEY (site_id, etablissement_id)
        REFERENCES ent_site (id, etablissement_id),
    CONSTRAINT ent_grille_cible_niveau_fk FOREIGN KEY (niveau_id, etablissement_id)
        REFERENCES ent_niveau (id, etablissement_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_grille_cible_uq ON ent_grille_cible (
    grille_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(niveau_id, '00000000-0000-0000-0000-000000000000'::uuid));

CREATE TABLE IF NOT EXISTS ent_grille_creneau (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    grille_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (char_length(btrim(code)) BETWEEN 1 AND 20),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 60),
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    nature TEXT NOT NULL DEFAULT 'cours'
        CHECK (nature IN ('cours', 'recreation', 'pause_meridienne', 'etude')),
    ordre SMALLINT NOT NULL CHECK (ordre >= 1),
    CONSTRAINT ent_grille_creneau_heures_chk CHECK (heure_fin > heure_debut),
    CONSTRAINT ent_grille_creneau_grille_fk FOREIGN KEY (grille_id, etablissement_id)
        REFERENCES ent_grille (id, etablissement_id) ON DELETE CASCADE,
    CONSTRAINT ent_grille_creneau_ordre_uq UNIQUE (grille_id, ordre),
    CONSTRAINT ent_grille_creneau_code_uq UNIQUE (grille_id, code),
    -- Deux créneaux d'une grille ne se chevauchent pas (granularité : la minute).
    CONSTRAINT ent_grille_creneau_chevauchement EXCLUDE USING gist (
        grille_id WITH =,
        int4range((EXTRACT(HOUR FROM heure_debut) * 60 + EXTRACT(MINUTE FROM heure_debut))::int,
                  (EXTRACT(HOUR FROM heure_fin) * 60 + EXTRACT(MINUTE FROM heure_fin))::int) WITH &&)
);

-- ───────────────────────── Gel des versions commencées ─────────────────────────
CREATE OR REPLACE FUNCTION ent_trg_grille_gel() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.valable_du < ent_aujourdhui_etab(OLD.etablissement_id) THEN
            RAISE EXCEPTION 'grille_gelee: une version commencée ne se supprime pas' USING ERRCODE = '42501';
        END IF;
        RETURN OLD;
    END IF;
    IF OLD.valable_du < ent_aujourdhui_etab(OLD.etablissement_id)
       AND (NEW.code IS DISTINCT FROM OLD.code OR NEW.libelle IS DISTINCT FROM OLD.libelle
            OR NEW.valable_du IS DISTINCT FROM OLD.valable_du
            OR NEW.coupure_demi_journee IS DISTINCT FROM OLD.coupure_demi_journee
            OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id) THEN
        RAISE EXCEPTION 'grille_gelee: une version commencée ne se modifie plus, créez une nouvelle version'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_grille_gel ON ent_grille;
CREATE TRIGGER ent_grille_gel BEFORE UPDATE OR DELETE ON ent_grille
    FOR EACH ROW EXECUTE FUNCTION ent_trg_grille_gel();

-- Créneaux et cibles d'une version commencée : gelés aussi. Contrôle sur la grille parente
-- (verrouillée pour éviter une course avec sa clôture).
CREATE OR REPLACE FUNCTION ent_trg_grille_fils_gel() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_grille uuid; v_etab uuid; v_du date;
BEGIN
    v_grille := CASE WHEN TG_OP = 'DELETE' THEN OLD.grille_id ELSE NEW.grille_id END;
    v_etab := CASE WHEN TG_OP = 'DELETE' THEN OLD.etablissement_id ELSE NEW.etablissement_id END;
    SELECT g.valable_du INTO v_du FROM ent_grille g WHERE g.id = v_grille AND g.etablissement_id = v_etab;
    -- Grille absente : suppression en cascade de la grille elle-même (déjà contrôlée).
    IF v_du IS NOT NULL AND v_du < ent_aujourdhui_etab(v_etab)
       AND COALESCE(current_setting('ent.grille_reprise', true), '') <> 'on' THEN
        RAISE EXCEPTION 'grille_gelee: une version commencée ne se modifie plus, créez une nouvelle version'
            USING ERRCODE = '42501';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$;
DROP TRIGGER IF EXISTS ent_grille_creneau_gel ON ent_grille_creneau;
CREATE TRIGGER ent_grille_creneau_gel BEFORE INSERT OR UPDATE OR DELETE ON ent_grille_creneau
    FOR EACH ROW EXECUTE FUNCTION ent_trg_grille_fils_gel();
DROP TRIGGER IF EXISTS ent_grille_cible_gel ON ent_grille_cible;
CREATE TRIGGER ent_grille_cible_gel BEFORE INSERT OR UPDATE OR DELETE ON ent_grille_cible
    FOR EACH ROW EXECUTE FUNCTION ent_trg_grille_fils_gel();

-- ───────────────────────── Résolution ─────────────────────────
-- Grille en vigueur pour (date, site, niveau, jour ISO). Spécificité : site + niveau, niveau,
-- site, établissement ; à égalité, la cible qui précise des jours l'emporte, puis la version la
-- plus récente. NULL si aucune grille ne s'applique.
CREATE OR REPLACE FUNCTION ent_grille_pour(p_etab uuid, p_date date, p_site uuid, p_niveau uuid)
RETURNS uuid LANGUAGE sql STABLE AS $$
    SELECT g.id
      FROM ent_grille g
      JOIN ent_grille_cible c ON c.grille_id = g.id AND c.etablissement_id = g.etablissement_id
     WHERE g.etablissement_id = p_etab AND g.archivee_le IS NULL
       AND g.valable_du <= p_date AND (g.valable_au IS NULL OR g.valable_au >= p_date)
       AND (c.site_id IS NULL OR c.site_id = p_site)
       AND (c.niveau_id IS NULL OR c.niveau_id = p_niveau)
       AND (c.jours IS NULL OR EXTRACT(ISODOW FROM p_date)::smallint = ANY (c.jours))
     ORDER BY CASE WHEN c.site_id IS NOT NULL AND c.niveau_id IS NOT NULL THEN 0
                   WHEN c.niveau_id IS NOT NULL THEN 1
                   WHEN c.site_id IS NOT NULL THEN 2 ELSE 3 END,
              (c.jours IS NULL), g.valable_du DESC, g.id
     LIMIT 1
$$;

-- ───────────────────────── Reprise de l'existant ─────────────────────────
-- Une version « REPRISE » par site (ou pour l'établissement) à partir de `ent_grille_horaire`.
-- Une ligne invalide (chevauchement, doublon) est laissée de côté : elle reste dans l'ancienne
-- table, que le lot ne modifie pas.
DO $$
DECLARE r RECORD; l RECORD; v_g UUID; v_code TEXT; v_coupure TIME; v_ordre INT;
BEGIN
    -- La reprise écrit des versions « commencées » : elle seule lève le gel, le temps du bloc.
    PERFORM set_config('ent.grille_reprise', 'on', true);
    FOR r IN
        SELECT DISTINCT h.etablissement_id, h.site_id FROM ent_grille_horaire h
         WHERE NOT EXISTS (
             SELECT 1 FROM ent_grille g
              WHERE g.etablissement_id = h.etablissement_id AND g.origine = 'reprise'
                AND EXISTS (SELECT 1 FROM ent_grille_cible c
                             WHERE c.grille_id = g.id
                               AND c.site_id IS NOT DISTINCT FROM h.site_id AND c.niveau_id IS NULL))
    LOOP
        v_g := gen_random_uuid();
        v_code := 'REPRISE-' || left(replace(COALESCE(r.site_id::text, 'ETAB'), '-', ''), 12);
        SELECT COALESCE(
                 (SELECT max(heure_fin) FROM ent_grille_horaire
                   WHERE etablissement_id = r.etablissement_id AND site_id IS NOT DISTINCT FROM r.site_id
                     AND demi_journee = 'matin'),
                 (SELECT min(heure_debut) FROM ent_grille_horaire
                   WHERE etablissement_id = r.etablissement_id AND site_id IS NOT DISTINCT FROM r.site_id
                     AND demi_journee = 'apres_midi'),
                 TIME '12:30') INTO v_coupure;
        INSERT INTO ent_grille (id, etablissement_id, code, libelle, valable_du, coupure_demi_journee, origine, motif)
        VALUES (v_g, r.etablissement_id, v_code, 'Grille horaire (reprise)', DATE '2000-01-01',
                v_coupure, 'reprise', 'Reprise de la grille historique');
        INSERT INTO ent_grille_cible (id, etablissement_id, grille_id, site_id)
        VALUES (gen_random_uuid(), r.etablissement_id, v_g, r.site_id);
        v_ordre := 0;
        FOR l IN
            SELECT * FROM ent_grille_horaire h
             WHERE h.etablissement_id = r.etablissement_id AND h.site_id IS NOT DISTINCT FROM r.site_id
             ORDER BY h.heure_debut, h.code
        LOOP
            BEGIN
                INSERT INTO ent_grille_creneau (id, etablissement_id, grille_id, code, libelle,
                                                heure_debut, heure_fin, nature, ordre)
                VALUES (gen_random_uuid(), r.etablissement_id, v_g, left(btrim(l.code), 20),
                        left(btrim(l.libelle), 60), l.heure_debut, l.heure_fin, 'cours', v_ordre + 1);
                v_ordre := v_ordre + 1;
            EXCEPTION WHEN exclusion_violation OR unique_violation OR check_violation THEN
                NULL;
            END;
        END LOOP;
    END LOOP;
    PERFORM set_config('ent.grille_reprise', 'off', true);
END
$$;

-- Journal de structure : l'objet `grille` est autorisé par le contrôle de format posé en 335 puis
-- par la liste fermée reposée en 338 (voir le commentaire de 335 : migrations rejouées).

-- ───────────────────────── RLS ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_grille', 'ent_grille_cible', 'ent_grille_creneau'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL
                USING (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
                WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))', t);
    END LOOP;
END
$$;
