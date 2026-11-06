-- Socle 2 (l'élève), lot E0 / étape M1 : clés étrangères composées
-- (etablissement_id, eleve_id) -> ent_eleve (etablissement_id, id).
--
-- Une donnée ne peut plus être rattachée à l'élève d'un AUTRE établissement, ni à
-- un élève inexistant. Additif et sans perte : les contraintes sont ajoutées
-- NOT VALID (appliquées aux nouvelles écritures), les lignes existantes qui ne
-- les respectent pas sont LISTÉES dans `ent_eleve_orphelin` (jamais supprimées,
-- jamais bloquantes), puis la validation est tentée table par table : elle
-- n'aboutit que s'il n'y a plus aucun orphelin. `ent_eleve_valider_cles()` permet
-- de la relancer après traitement. Numéros 240-249 réservés au socle 2 (élève).

-- Cible des clés composées.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_etab_id_uniq') THEN
        ALTER TABLE ent_eleve
            ADD CONSTRAINT ent_eleve_etab_id_uniq UNIQUE (etablissement_id, id);
    END IF;
END $$;

-- Rapport des orphelins (maintenance : aucune politique d'établissement, aucun
-- accès applicatif ; contient des copies de lignes, à traiter puis marquer).
CREATE TABLE IF NOT EXISTS ent_eleve_orphelin (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_nom    TEXT NOT NULL,
    ligne_id     UUID,
    etablissement_id UUID,
    eleve_id     UUID,
    cause        TEXT NOT NULL CHECK (cause IN ('eleve_absent', 'autre_etablissement')),
    ligne        JSONB NOT NULL,
    detecte_le   TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le    TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS ent_eleve_orphelin_table_idx ON ent_eleve_orphelin (table_nom) WHERE traite_le IS NULL;

-- Tables qui portent (etablissement_id, eleve_id) sans clé étrangère vers l'élève.
CREATE OR REPLACE FUNCTION ent_eleve_tables_a_cle() RETURNS TEXT[] LANGUAGE sql IMMUTABLE AS $$
    SELECT ARRAY[
        'ent_eleve_periode', 'ent_eleve_scolarite_annee', 'ent_eleve_option',
        'ent_eleve_option_annee', 'ent_eleve_compte', 'ent_eleve_lien',
        'ent_eleve_document', 'ent_eleve_dispositif', 'ent_eleve_grille_repas',
        'ent_eleve_bourse', 'ent_eleve_dispense_repas', 'ent_metier_event'
    ]
$$;

-- Ajoute la clé composée (NOT VALID) à une table, si absente.
CREATE OR REPLACE FUNCTION ent_eleve_ajouter_cle(t TEXT) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE c TEXT := t || '_eleve_etab_fkey';
BEGIN
    IF to_regclass(t) IS NULL THEN RETURN; END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = c AND conrelid = to_regclass(t)) THEN
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (etablissement_id, eleve_id)
             REFERENCES ent_eleve (etablissement_id, id) NOT VALID', t, c);
    END IF;
END $$;

-- Recense les orphelins d'une table (remplace le rapport non traité de cette table).
CREATE OR REPLACE FUNCTION ent_eleve_rapporter_orphelins(t TEXT) RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE n BIGINT;
BEGIN
    IF to_regclass(t) IS NULL THEN RETURN 0; END IF;
    DELETE FROM ent_eleve_orphelin WHERE table_nom = t AND traite_le IS NULL;
    EXECUTE format(
        'INSERT INTO ent_eleve_orphelin (table_nom, ligne_id, etablissement_id, eleve_id, cause, ligne)
         SELECT %L, NULLIF(to_jsonb(x) ->> ''id'', '''')::uuid, x.etablissement_id, x.eleve_id,
                CASE WHEN EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = x.eleve_id)
                     THEN ''autre_etablissement'' ELSE ''eleve_absent'' END,
                to_jsonb(x)
         FROM %I x
         WHERE x.eleve_id IS NOT NULL
           AND NOT EXISTS (SELECT 1 FROM ent_eleve e
                           WHERE e.id = x.eleve_id AND e.etablissement_id = x.etablissement_id)', t, t);
    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END $$;

-- Rapporte puis tente de valider toutes les clés ; renvoie (table, orphelins, validée).
CREATE OR REPLACE FUNCTION ent_eleve_valider_cles()
RETURNS TABLE (table_nom TEXT, orphelins BIGINT, validee BOOLEAN) LANGUAGE plpgsql AS $$
DECLARE t TEXT; n BIGINT; c TEXT;
BEGIN
    FOREACH t IN ARRAY ent_eleve_tables_a_cle() LOOP
        CONTINUE WHEN to_regclass(t) IS NULL;
        PERFORM ent_eleve_ajouter_cle(t);
        n := ent_eleve_rapporter_orphelins(t);
        c := t || '_eleve_etab_fkey';
        IF n = 0 THEN
            EXECUTE format('ALTER TABLE %I VALIDATE CONSTRAINT %I', t, c);
            table_nom := t; orphelins := 0; validee := true;
        ELSE
            RAISE NOTICE 'clé % : % orphelin(s) listé(s) dans ent_eleve_orphelin, validation différée', c, n;
            table_nom := t; orphelins := n; validee := false;
        END IF;
        RETURN NEXT;
    END LOOP;
END $$;

SELECT * FROM ent_eleve_valider_cles();
