-- Socle 2 (l'élève), lot E4 / étape M5 (2/2) : inscription annuelle.
--
-- `ent_eleve_inscription` succède à `ent_eleve_scolarite_annee` : une inscription par
-- élève et par année scolaire (`annee_id`, plus le texte `annee_scolaire`), rattachée à la
-- scolarisation qui la couvre. L'ancienne table n'est PAS supprimée ni transformée en
-- vue (ses lecteurs et écrivains existants continuent de fonctionner) : un pont
-- bidirectionnel maintient les deux en accord le temps de migrer les lecteurs (M11). Le
-- drapeau transactionnel `ent.inscription_pont` coupe le rebouclage.
--
-- Additif, idempotent, rejoué à chaque démarrage.

CREATE TABLE IF NOT EXISTS ent_eleve_inscription (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL,
    eleve_id          UUID NOT NULL,
    scolarisation_id  UUID NOT NULL REFERENCES ent_eleve_scolarisation (id),
    annee_id          UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    niveau_id         UUID REFERENCES ent_niveau (id),
    mef_id            UUID REFERENCES ent_mef (id),
    statut_apprenant  TEXT NOT NULL DEFAULT 'scolaire',
    etat              TEXT NOT NULL,
    redoublement      BOOLEAN,
    decision_passage  TEXT,
    date_debut        DATE,
    date_fin          DATE,
    source            TEXT NOT NULL DEFAULT 'saisie',
    source_ref        UUID,
    legacy_id         UUID,
    cree_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par          UUID,
    modifie_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_inscription_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_inscription_regles_chk CHECK (
        statut_apprenant IN ('scolaire', 'apprenti', 'autre')
        AND etat IN ('prevue', 'confirmee', 'en_cours', 'terminee', 'annulee')
        AND source IN ('saisie', 'admission', 'reinscription', 'montee', 'import', 'reprise')
        AND (date_fin IS NULL OR date_debut IS NULL OR date_fin >= date_debut)
        AND (decision_passage IS NULL OR char_length(decision_passage) <= 80)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_inscription_annee_uniq
    ON ent_eleve_inscription (eleve_id, annee_id) WHERE etat <> 'annulee';
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_inscription_legacy_uniq
    ON ent_eleve_inscription (legacy_id) WHERE legacy_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_eleve_inscription_eleve_idx
    ON ent_eleve_inscription (etablissement_id, eleve_id, annee_id);

ALTER TABLE ent_eleve_inscription ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_inscription FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_inscription;
CREATE POLICY ent_rls_etablissement ON ent_eleve_inscription FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- Lignes de l'ancienne table qu'on n'a pas pu reprendre (rien n'est supprimé).
CREATE TABLE IF NOT EXISTS ent_eleve_inscription_a_traiter (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL,
    eleve_id         UUID NOT NULL,
    legacy_id        UUID NOT NULL,
    cause            TEXT NOT NULL,
    detecte_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le        TIMESTAMPTZ,
    UNIQUE (legacy_id, cause)
);
ALTER TABLE ent_eleve_inscription_a_traiter ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_inscription_a_traiter FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_inscription_a_traiter;
CREATE POLICY ent_rls_etablissement ON ent_eleve_inscription_a_traiter FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- ───────────────────────── Ancienne table → inscription ─────────────────────────
-- Année : le libellé `AAAA-AAAA` est résolu (ou créé, comme le fait déjà le trigger de
-- classe) ; tout autre texte est rangé « à traiter ». Niveau et MEF : correspondance exacte
-- de code, sinon NULL (la ligne est reprise quand même). La scolarisation rattachée est
-- celle qui couvre l'année, à défaut la plus récente.
CREATE OR REPLACE FUNCTION ent_eleve_inscription_depuis_legacy(l ent_eleve_scolarite_annee) RETURNS UUID
LANGUAGE plpgsql AS $$
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
            -- Une année pas encore commencée n'est jamais créée par la reprise (c'est la préparation
            -- de l'année suivante du référentiel qui la crée) : la ligne attend, rangée « à traiter ».
            IF make_date(v_y, 9, 1) > current_date THEN
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

-- Inscription → ancienne table (les lecteurs hérités continuent de voir les inscriptions
-- écrites par les commandes). Une inscription annulée n'est pas écrite.
CREATE OR REPLACE FUNCTION ent_eleve_inscription_vers_legacy(i ent_eleve_inscription) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE v_label TEXT; v_niveau TEXT; v_mef TEXT; v_statut TEXT; v_source TEXT;
BEGIN
    IF i.etat = 'annulee' THEN RETURN; END IF;
    PERFORM set_config('ent.inscription_pont', 'on', true);
    SELECT a.libelle INTO v_label FROM ent_annee_scolaire a WHERE a.id = i.annee_id;
    SELECT n.code INTO v_niveau FROM ent_niveau n WHERE n.id = i.niveau_id;
    SELECT m.code_mef INTO v_mef FROM ent_mef m WHERE m.id = i.mef_id;
    v_statut := CASE i.etat WHEN 'terminee' THEN 'historique' WHEN 'en_cours' THEN 'courante'
                            WHEN 'prevue' THEN 'projetee' ELSE 'reinscription' END;
    v_source := CASE i.source WHEN 'montee' THEN 'montee_rapide' WHEN 'reinscription' THEN 'reinscription_parent'
                              WHEN 'import' THEN 'import' ELSE 'saisie' END;
    INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, niveau, mef, statut, source)
    VALUES (gen_random_uuid(), i.etablissement_id, i.eleve_id, v_label, v_niveau, v_mef, v_statut, v_source)
    ON CONFLICT (etablissement_id, eleve_id, annee_scolaire) DO UPDATE SET
        niveau = coalesce(EXCLUDED.niveau, ent_eleve_scolarite_annee.niveau),
        mef = coalesce(EXCLUDED.mef, ent_eleve_scolarite_annee.mef),
        statut = EXCLUDED.statut, updated_at = now();
    UPDATE ent_eleve_inscription SET legacy_id = (
        SELECT l.id FROM ent_eleve_scolarite_annee l
        WHERE l.etablissement_id = i.etablissement_id AND l.eleve_id = i.eleve_id AND l.annee_scolaire = v_label)
    WHERE id = i.id AND legacy_id IS NULL;
    PERFORM set_config('ent.inscription_pont', 'off', true);
END $$;

CREATE OR REPLACE FUNCTION ent_eleve_inscription_pont_legacy() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF coalesce(current_setting('ent.inscription_pont', true), '') = 'on' THEN RETURN NULL; END IF;
    IF TG_OP = 'DELETE' THEN
        PERFORM set_config('ent.inscription_pont', 'on', true);
        UPDATE ent_eleve_inscription SET etat = 'annulee', modifie_le = now() WHERE legacy_id = OLD.id AND etat <> 'annulee';
        PERFORM set_config('ent.inscription_pont', 'off', true);
        RETURN NULL;
    END IF;
    PERFORM ent_eleve_inscription_depuis_legacy(NEW);
    RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_inscription_pont_legacy ON ent_eleve_scolarite_annee;
CREATE TRIGGER ent_eleve_inscription_pont_legacy
    AFTER INSERT OR UPDATE OR DELETE ON ent_eleve_scolarite_annee
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_inscription_pont_legacy();

CREATE OR REPLACE FUNCTION ent_eleve_inscription_pont_nouvelle() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF coalesce(current_setting('ent.inscription_pont', true), '') = 'on' THEN RETURN NULL; END IF;
    PERFORM ent_eleve_inscription_vers_legacy(NEW);
    RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_inscription_pont_nouvelle ON ent_eleve_inscription;
CREATE TRIGGER ent_eleve_inscription_pont_nouvelle
    AFTER INSERT OR UPDATE ON ent_eleve_inscription
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_inscription_pont_nouvelle();

-- Reprise (idempotente) de l'ancienne table.
CREATE OR REPLACE FUNCTION ent_eleve_inscription_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; l ent_eleve_scolarite_annee;
BEGIN
    FOR l IN SELECT * FROM ent_eleve_scolarite_annee s
             WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_inscription i WHERE i.legacy_id = s.id)
               AND NOT EXISTS (SELECT 1 FROM ent_eleve_inscription_a_traiter t
                               WHERE t.legacy_id = s.id AND t.cause IN ('annee_illisible', 'sans_scolarisation'))
             ORDER BY s.etablissement_id, s.created_at, s.id
    LOOP
        IF ent_eleve_inscription_depuis_legacy(l) IS NOT NULL THEN n := n + 1; END IF;
    END LOOP;
    RETURN n;
END $$;

SELECT ent_eleve_inscription_reprendre();
