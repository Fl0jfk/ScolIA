-- Socle 5, lot S1 : l'objet unique « cours » et sa règle de visibilité unique.
--
-- Décision de conception (PERSONAS_05-06 §0 A, Q-S20) : le « cours » (ce qui est enseigné à quel
-- public) est la ligne existante de `ent_service_enseignement` (public = classe OU groupe,
-- matière, professeur). On ne crée pas de seconde table ; la séparation « cours » /
-- « intervention » viendra avec le socle 6 sans changer cette règle.
--
-- 1. Membres individuels datés d'un cours (R-S5) : élève d'ULIS inclus dans un cours d'une autre
--    classe, décloisonnement, élève accueilli (inclusion) ; échange, scolarité partagée (exclusion).
-- 2. `ent_public_cours_jour` : LA fonction « qui est dans le public du cours à la date d ».
--    public = (classe ou groupe à d) ∪ inclusions actives − exclusions actives, hors élèves
--    sortis. `perimetre.rs`, `eleves_autorises` et « mes enseignements » s'y appuient.

CREATE TABLE IF NOT EXISTS ent_cours_membre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    cours_id UUID NOT NULL REFERENCES ent_service_enseignement (id),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    nature TEXT NOT NULL CHECK (nature IN ('inclusion', 'exclusion')),
    motif_code TEXT NOT NULL CHECK (motif_code IN
        ('ulis', 'upe2a', 'decloisonnement', 'avance', 'parcours_amenage', 'scolarite_partagee',
         'echange', 'accueil_temporaire', 'dispense_cours', 'autre')),
    du DATE NOT NULL,
    au DATE,
    cree_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    annulee_le TIMESTAMPTZ,
    annulee_par UUID,
    CONSTRAINT ent_cours_membre_dates_chk CHECK (au IS NULL OR au >= du)
);
CREATE INDEX IF NOT EXISTS ent_cours_membre_cours_idx ON ent_cours_membre (etablissement_id, cours_id);
CREATE INDEX IF NOT EXISTS ent_cours_membre_eleve_idx ON ent_cours_membre (etablissement_id, eleve_id);

-- Cohérence : même établissement (cours, élève) ; pas deux périodes actives qui se chevauchent
-- pour (cours, élève, nature) ; la période reste dans l'année du cours.
CREATE OR REPLACE FUNCTION ent_trg_cours_membre() RETURNS trigger AS $$
DECLARE a_debut DATE; a_fin DATE; a_close TIMESTAMPTZ;
BEGIN
    -- Un membre ne se modifie pas : seule l'annulation (annulee_le NULL -> date) est permise.
    IF TG_OP = 'UPDATE' THEN
        IF (NEW.etablissement_id, NEW.cours_id, NEW.eleve_id, NEW.nature, NEW.motif_code, NEW.du, NEW.au)
               IS DISTINCT FROM (OLD.etablissement_id, OLD.cours_id, OLD.eleve_id, OLD.nature, OLD.motif_code, OLD.du, OLD.au)
           OR (OLD.annulee_le IS NOT NULL AND NEW.annulee_le IS DISTINCT FROM OLD.annulee_le)
           OR (OLD.annulee_par IS NOT NULL AND NEW.annulee_par IS DISTINCT FROM OLD.annulee_par) THEN
            RAISE EXCEPTION 'cours_membre_immuable' USING ERRCODE = '23514';
        END IF;
    END IF;
    SELECT y.date_debut, y.date_fin, y.cloturee_le INTO a_debut, a_fin, a_close
      FROM ent_service_enseignement s
      JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
     WHERE s.id = NEW.cours_id AND s.etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'cours_membre_incoherent: cours' USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = NEW.eleve_id AND e.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'cours_membre_incoherent: eleve' USING ERRCODE = '23514';
    END IF;
    IF a_close IS NOT NULL THEN
        RAISE EXCEPTION 'cours_membre_annee_cloturee' USING ERRCODE = '23514';
    END IF;
    IF NEW.du < a_debut OR NEW.du > a_fin OR (NEW.au IS NOT NULL AND NEW.au > a_fin) THEN
        RAISE EXCEPTION 'cours_membre_incoherent: hors annee' USING ERRCODE = '23514';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(NEW.cours_id::text, 0));
    IF NEW.annulee_le IS NULL AND EXISTS (
        SELECT 1 FROM ent_cours_membre m
         WHERE m.cours_id = NEW.cours_id AND m.eleve_id = NEW.eleve_id AND m.nature = NEW.nature
           AND m.annulee_le IS NULL AND m.id <> NEW.id
           AND m.du <= coalesce(NEW.au, 'infinity'::date) AND coalesce(m.au, 'infinity'::date) >= NEW.du) THEN
        RAISE EXCEPTION 'cours_membre_chevauchement' USING ERRCODE = '23505';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_cours_membre_coherence ON ent_cours_membre;
CREATE TRIGGER ent_cours_membre_coherence BEFORE INSERT OR UPDATE ON ent_cours_membre
    FOR EACH ROW EXECUTE FUNCTION ent_trg_cours_membre();

-- Un membre n'est jamais supprimé : il est annulé (trace conservée).
CREATE OR REPLACE FUNCTION ent_trg_cours_membre_pas_de_suppression() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'cours_membre_non_supprimable' USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cours_membre_pas_de_suppression ON ent_cours_membre;
CREATE TRIGGER ent_cours_membre_pas_de_suppression BEFORE DELETE ON ent_cours_membre
    FOR EACH ROW EXECUTE FUNCTION ent_trg_cours_membre_pas_de_suppression();

-- LA règle de visibilité. Filtres facultatifs : un cours, un professeur (titulaire du cours),
-- un élève. Sans filtre : tout l'établissement.
CREATE OR REPLACE FUNCTION ent_public_cours_jour(
    p_etab UUID, p_jour DATE,
    p_cours UUID DEFAULT NULL, p_prof UUID DEFAULT NULL, p_eleve UUID DEFAULT NULL)
RETURNS TABLE (cours_id UUID, eleve_id UUID) LANGUAGE sql STABLE AS $$
    SELECT DISTINCT x.cours_id, x.eleve_id
      FROM (
        SELECT s.id AS cours_id, a.eleve_id
          FROM ent_service_enseignement s
          JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
          JOIN ent_affectation a
            ON a.etablissement_id = s.etablissement_id AND a.annee_id = s.annee_id
           AND ((s.classe_id IS NOT NULL AND a.classe_id = s.classe_id)
             OR (s.groupe_id IS NOT NULL AND a.groupe_id = s.groupe_id))
         WHERE s.etablissement_id = p_etab
           AND (p_cours IS NULL OR s.id = p_cours)
           AND (p_prof IS NULL OR s.prof_user_id = p_prof)
           AND (p_eleve IS NULL OR a.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
        UNION
        SELECT s.id, m.eleve_id
          FROM ent_cours_membre m
          JOIN ent_service_enseignement s ON s.id = m.cours_id AND s.etablissement_id = m.etablissement_id
          JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
         WHERE m.etablissement_id = p_etab AND m.nature = 'inclusion' AND m.annulee_le IS NULL
           AND (p_cours IS NULL OR s.id = p_cours)
           AND (p_prof IS NULL OR s.prof_user_id = p_prof)
           AND (p_eleve IS NULL OR m.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND m.du <= p_jour AND (m.au IS NULL OR m.au >= p_jour)
      ) x
      JOIN ent_eleve e ON e.id = x.eleve_id AND e.etablissement_id = p_etab
     WHERE (e.date_sortie IS NULL OR e.date_sortie > p_jour)
       AND NOT EXISTS (
            SELECT 1 FROM ent_cours_membre ex
             WHERE ex.etablissement_id = p_etab AND ex.cours_id = x.cours_id AND ex.eleve_id = x.eleve_id
               AND ex.nature = 'exclusion' AND ex.annulee_le IS NULL
               AND ex.du <= p_jour AND (ex.au IS NULL OR ex.au >= p_jour))
$$;

ALTER TABLE ent_cours_membre ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_cours_membre FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_cours_membre;
CREATE POLICY ent_rls_etablissement ON ent_cours_membre FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
