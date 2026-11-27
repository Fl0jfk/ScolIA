-- D-036 (lot 2) — corrections des tables normalisées par 560, 561 et 563 :
--  * E2 : références composites (etablissement_id, id) — une ligne fille ne peut plus pointer une
--    ligne d'un autre établissement (les clés étrangères simples sont retirées) ;
--  * E3 : identifiants générés par l'application (plus de défaut `gen_random_uuid()`) ;
--  * E6 : horodatage `cree_le` sur les tables filles.
-- Rejouable : chaque ajout est gardé par un test d'existence.

-- Retire les clés étrangères dont la colonne locale est exactement `col` (ancienne forme simple).
CREATE OR REPLACE FUNCTION ent_d036_retirer_fk_simple(tbl regclass, col text) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE c record;
BEGIN
    FOR c IN SELECT con.conname FROM pg_constraint con
              WHERE con.conrelid = tbl AND con.contype = 'f' AND array_length(con.conkey, 1) = 1
                AND con.conkey[1] = (SELECT attnum FROM pg_attribute WHERE attrelid = tbl AND attname = col)
    LOOP
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', tbl, c.conname);
    END LOOP;
END
$$;

-- Ajoute une contrainte (unique ou clé étrangère) si son nom est libre. Une clé étrangère est posée
-- NOT VALID puis validée ; si des lignes héritées la violent, elle reste non validée (appliquée aux
-- nouvelles écritures) au lieu de bloquer le démarrage.
CREATE OR REPLACE FUNCTION ent_d036_contrainte(tbl regclass, nom text, definition text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = tbl AND conname = nom) THEN
        RETURN;
    END IF;
    IF definition LIKE 'FOREIGN KEY%' THEN
        EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I %s NOT VALID', tbl, nom, definition);
        BEGIN
            EXECUTE format('ALTER TABLE %s VALIDATE CONSTRAINT %I', tbl, nom);
        EXCEPTION WHEN foreign_key_violation THEN
            RAISE NOTICE 'D-036 : % reste non validée (lignes héritées incohérentes)', nom;
        END;
    ELSE
        EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I %s', tbl, nom, definition);
    END IF;
END
$$;

-- Parents : unicité (établissement, id), cible des références composites.
SELECT ent_d036_contrainte('ent_delegation', 'ent_delegation_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_contrainte('ent_inscription_responsable', 'ent_inscription_responsable_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_contrainte('ent_signalement_assiduite', 'ent_signalement_assiduite_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_contrainte('ent_delegation_droit', 'ent_delegation_droit_etab_ordre_uq', 'UNIQUE (etablissement_id, delegation_id, ordre)');

-- 560 : droits de délégation.
SELECT ent_d036_retirer_fk_simple('ent_delegation_droit', 'delegation_id');
SELECT ent_d036_contrainte('ent_delegation_droit', 'ent_delegation_droit_delegation_fk',
    'FOREIGN KEY (etablissement_id, delegation_id) REFERENCES ent_delegation (etablissement_id, id) ON DELETE CASCADE');
ALTER TABLE ent_delegation_droit_cible DROP CONSTRAINT IF EXISTS ent_delegation_droit_cible_delegation_id_ordre_fkey;
SELECT ent_d036_contrainte('ent_delegation_droit_cible', 'ent_delegation_droit_cible_droit_fk',
    'FOREIGN KEY (etablissement_id, delegation_id, ordre) REFERENCES ent_delegation_droit (etablissement_id, delegation_id, ordre) ON DELETE CASCADE');

-- 561 : résolutions d'inscription (identifiant fourni par l'application ; responsable composite).
ALTER TABLE ent_inscription_resolution ALTER COLUMN id DROP DEFAULT;
SELECT ent_d036_retirer_fk_simple('ent_inscription_resolution', 'responsable_id');
SELECT ent_d036_contrainte('ent_inscription_resolution', 'ent_inscription_resolution_responsable_fk',
    'FOREIGN KEY (etablissement_id, responsable_id) REFERENCES ent_inscription_responsable (etablissement_id, id) ON DELETE SET NULL (responsable_id)');

-- 563 : détail des signalements.
SELECT ent_d036_retirer_fk_simple('ent_signalement_assiduite_detail', 'signalement_id');
SELECT ent_d036_contrainte('ent_signalement_assiduite_detail', 'ent_signalement_assiduite_detail_signalement_fk',
    'FOREIGN KEY (etablissement_id, signalement_id) REFERENCES ent_signalement_assiduite (etablissement_id, id) ON DELETE CASCADE');

-- E6 : horodatage des tables filles.
ALTER TABLE ent_delegation_droit ADD COLUMN IF NOT EXISTS cree_le TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE ent_delegation_droit_cible ADD COLUMN IF NOT EXISTS cree_le TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE ent_signalement_assiduite_detail ADD COLUMN IF NOT EXISTS cree_le TIMESTAMPTZ NOT NULL DEFAULT now();
