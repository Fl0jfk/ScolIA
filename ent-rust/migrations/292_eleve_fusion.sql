-- Socle 2 (l'élève), lot E6 : fusion de deux fiches d'élèves, annulable (ELV-23, ELV-24, §7.10).
--
-- Le registre `ent_eleve_fusion_ligne` garde, pour CHAQUE ligne déplacée, la table et la clé de
-- la ligne (jamais son contenu : un déplacement ne change que `eleve_id`, la défusion le remet
-- donc « à l'identique ») et l'EMPREINTE de la ligne telle qu'elle est après le déplacement :
-- à l'annulation, une ligne dont l'empreinte a changé a été modifiée depuis la fusion → la
-- défusion est refusée et la liste est montrée (§7.10). Aucune donnée de santé ni d'identité
-- n'est recopiée dans ce registre.
--
-- Une fusion ne supprime rien : l'élève absorbé reste, marqué `fusionne_dans` (290), et ce qui
-- ne peut pas passer (historiques immuables, lignes en conflit d'unicité) reste sur lui.

CREATE TABLE IF NOT EXISTS ent_eleve_fusion (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL,
    absorbe_id        UUID NOT NULL,
    conserve_id       UUID NOT NULL,
    motif_code        TEXT NOT NULL,
    motif             TEXT,
    conflits_resolus  JSONB NOT NULL DEFAULT '[]'::jsonb,
    resume            JSONB NOT NULL DEFAULT '{}'::jsonb,
    doublons_touches  JSONB NOT NULL DEFAULT '[]'::jsonb,
    fusionne_par      UUID,
    fusionne_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    annule_par        UUID,
    annule_le         TIMESTAMPTZ,
    CONSTRAINT ent_eleve_fusion_distincts_chk CHECK (absorbe_id <> conserve_id),
    CONSTRAINT ent_eleve_fusion_motif_code_chk CHECK (motif_code IN
        ('doublon_saisie', 'doublon_import', 'erreur_creation', 'autre')),
    CONSTRAINT ent_eleve_fusion_motif_chk CHECK (motif IS NULL OR char_length(motif) <= 500),
    CONSTRAINT ent_eleve_fusion_autre_chk CHECK (motif_code <> 'autre' OR (motif IS NOT NULL AND btrim(motif) <> '')),
    FOREIGN KEY (etablissement_id, absorbe_id) REFERENCES ent_eleve (etablissement_id, id),
    FOREIGN KEY (etablissement_id, conserve_id) REFERENCES ent_eleve (etablissement_id, id),
    UNIQUE (etablissement_id, id)
);
-- Un élève n'est absorbé qu'une fois à la fois.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_fusion_active_uq
    ON ent_eleve_fusion (etablissement_id, absorbe_id) WHERE annule_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_eleve_fusion_conserve_idx
    ON ent_eleve_fusion (etablissement_id, conserve_id, fusionne_le DESC);

CREATE TABLE IF NOT EXISTS ent_eleve_fusion_ligne (
    id                BIGSERIAL PRIMARY KEY,
    fusion_id         UUID NOT NULL REFERENCES ent_eleve_fusion (id),
    etablissement_id  UUID NOT NULL,
    table_nom         TEXT NOT NULL,
    cle               JSONB NOT NULL,
    empreinte         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ent_eleve_fusion_ligne_fusion_idx
    ON ent_eleve_fusion_ligne (fusion_id, table_nom);

-- Les lignes de la fusion et les lignes de conflit non déplacées sont de l'audit : ajout seul
-- (la défusion ne les efface pas, elle marque la fusion annulée).
CREATE OR REPLACE FUNCTION ent_eleve_fusion_ligne_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'fusion : le registre des lignes déplacées est en ajout seul' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_eleve_fusion_ligne_garde ON ent_eleve_fusion_ligne;
CREATE TRIGGER ent_eleve_fusion_ligne_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_fusion_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_fusion_ligne_garde();

-- Une fusion se pose une fois et s'annule une fois : seules `annule_par` et `annule_le` changent.
CREATE OR REPLACE FUNCTION ent_eleve_fusion_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'fusion : ne se supprime pas (l''annuler)' USING ERRCODE = '42501';
    END IF;
    IF OLD.annule_le IS NULL AND NEW.annule_le IS NOT NULL
       AND (to_jsonb(NEW) - 'annule_le' - 'annule_par') = (to_jsonb(OLD) - 'annule_le' - 'annule_par') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'fusion : immuable (seule l''annulation, une fois, est permise)' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_eleve_fusion_garde ON ent_eleve_fusion;
CREATE TRIGGER ent_eleve_fusion_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_fusion
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_fusion_garde();

DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_fusion', 'ent_eleve_fusion_ligne'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format($p$CREATE POLICY ent_rls_etablissement ON %I FOR ALL
            USING (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
            WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))$p$, t);
    END LOOP;
END $$;
