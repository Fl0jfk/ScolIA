-- Socle 5, lot S0 : garde-fous d'état et de cohérence de la structure (défauts n° 5, 16, 17
-- de docs/socles/05_STRUCTURE.md §2.2). Additif et idempotent ; aucune donnée réécrite.

-- 1. Une année clôturée ne se rouvre pas (n° 5) : `cloturee_le` ne repasse jamais à NULL.
--    (Une réouverture bornée et journalisée sera une commande du lot S8.)
CREATE OR REPLACE FUNCTION ent_trg_annee_garde() RETURNS trigger AS $$
BEGIN
    IF OLD.cloturee_le IS NOT NULL AND NEW.cloturee_le IS NULL THEN
        RAISE EXCEPTION 'annee_cloturee_non_rouvrable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_annee_garde ON ent_annee_scolaire;
CREATE TRIGGER ent_annee_garde BEFORE UPDATE ON ent_annee_scolaire
    FOR EACH ROW EXECUTE FUNCTION ent_trg_annee_garde();

-- 2. Une affectation n'est plus jamais perdue (n° 17) : toute suppression physique
--    (correction d'une affectation posée à tort) laisse une trace.
CREATE TABLE IF NOT EXISTS ent_affectation_supprimee (
    id UUID NOT NULL,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    classe_id UUID,
    groupe_id UUID,
    date_debut DATE NOT NULL,
    date_fin DATE,
    source TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    supprimee_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_affectation_supprimee_eleve_idx
    ON ent_affectation_supprimee (etablissement_id, eleve_id);

CREATE OR REPLACE FUNCTION ent_trg_affectation_archive() RETURNS trigger AS $$
BEGIN
    INSERT INTO ent_affectation_supprimee
        (id, etablissement_id, annee_id, eleve_id, classe_id, groupe_id, date_debut, date_fin, source, created_at)
    VALUES (OLD.id, OLD.etablissement_id, OLD.annee_id, OLD.eleve_id, OLD.classe_id, OLD.groupe_id,
            OLD.date_debut, OLD.date_fin, OLD.source, OLD.created_at);
    RETURN OLD;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_affectation_archive ON ent_affectation;
CREATE TRIGGER ent_affectation_archive BEFORE DELETE ON ent_affectation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_affectation_archive();

-- 3. Cohérence établissement / année des clés (n° 16) : classe, groupe et élève d'une
--    affectation ou d'un service appartiennent au même établissement, classe et groupe à la
--    même année. (Les clés étrangères composées viendront au lot S13 ; ce garde-fou refuse
--    déjà toute nouvelle ligne incohérente. Lignes existantes non modifiées.)
CREATE OR REPLACE FUNCTION ent_trg_structure_coherence() RETURNS trigger AS $$
BEGIN
    IF NEW.classe_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ent_classe c
         WHERE c.id = NEW.classe_id AND c.etablissement_id = NEW.etablissement_id AND c.annee_id = NEW.annee_id) THEN
        RAISE EXCEPTION 'structure_incoherente: classe' USING ERRCODE = '23514';
    END IF;
    IF NEW.groupe_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ent_groupe g
         WHERE g.id = NEW.groupe_id AND g.etablissement_id = NEW.etablissement_id AND g.annee_id = NEW.annee_id) THEN
        RAISE EXCEPTION 'structure_incoherente: groupe' USING ERRCODE = '23514';
    END IF;
    IF TG_TABLE_NAME = 'ent_affectation' THEN
        IF NOT EXISTS (
            SELECT 1 FROM ent_eleve e WHERE e.id = NEW.eleve_id AND e.etablissement_id = NEW.etablissement_id) THEN
            RAISE EXCEPTION 'structure_incoherente: eleve' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_affectation_coherence ON ent_affectation;
CREATE TRIGGER ent_affectation_coherence BEFORE INSERT OR UPDATE OF etablissement_id, annee_id, classe_id, groupe_id, eleve_id
    ON ent_affectation FOR EACH ROW EXECUTE FUNCTION ent_trg_structure_coherence();

DROP TRIGGER IF EXISTS ent_service_coherence ON ent_service_enseignement;
CREATE TRIGGER ent_service_coherence BEFORE INSERT OR UPDATE OF etablissement_id, annee_id, classe_id, groupe_id
    ON ent_service_enseignement FOR EACH ROW EXECUTE FUNCTION ent_trg_structure_coherence();

-- Isolation par établissement de la table d'archive (mode permissif, comme les autres).
ALTER TABLE ent_affectation_supprimee ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_affectation_supprimee FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_affectation_supprimee;
CREATE POLICY ent_rls_etablissement ON ent_affectation_supprimee FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
