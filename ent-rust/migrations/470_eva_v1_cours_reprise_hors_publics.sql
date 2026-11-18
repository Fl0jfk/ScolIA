-- Socle 21, lot V1 (correctifs de revue, F9) — 470 : les cours « de reprise » (créés par la reprise M2 des
-- évaluations quand aucun cours réel ne portait le devoir : `source = 'reprise'`, `a_verifier = true`) ne sont
-- pas de vrais cours : ils n'ont ni professeur ni séance et n'apparaissent dans AUCUN emploi du temps (une
-- séance ne naît que d'un plan qui nomme un cours). Ils ne doivent pas non plus figurer dans les PUBLICS :
-- `ent_public_cours_jour` les écarte tant que la gestion ne les a pas vérifiés (`a_verifier`), sauf quand
-- l'appelant désigne EXPLICITEMENT le cours (`p_cours`), ce que fait le contrôle des résultats d'une évaluation
-- reprise. Le reste de la définition est celle de 355, inchangée.
CREATE OR REPLACE FUNCTION ent_public_cours_jour(
    p_etab UUID, p_jour DATE,
    p_cours UUID DEFAULT NULL, p_prof UUID DEFAULT NULL, p_eleve UUID DEFAULT NULL,
    p_saisie BOOLEAN DEFAULT false)
RETURNS TABLE (cours_id UUID, eleve_id UUID) LANGUAGE sql STABLE AS $$
    SELECT DISTINCT x.cours_id, x.eleve_id
      FROM (
        SELECT c.id AS cours_id, a.eleve_id
          FROM ent_cours c
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          JOIN ent_affectation a
            ON a.etablissement_id = c.etablissement_id AND a.annee_id = c.annee_id
           AND ((c.classe_id IS NOT NULL AND a.classe_id = c.classe_id)
             OR (c.groupe_id IS NOT NULL AND a.groupe_id = c.groupe_id))
         WHERE c.etablissement_id = p_etab
           AND (p_cours IS NULL OR c.id = p_cours)
           AND (p_prof IS NULL OR ent_prof_sur_cours(p_etab, c.id, p_prof, p_jour, p_saisie))
           AND (p_eleve IS NULL OR a.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
           AND (p_cours IS NOT NULL OR NOT (c.source = 'reprise' AND c.a_verifier))
           AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
        UNION
        SELECT c.id, m.eleve_id
          FROM ent_cours_membre m
          JOIN ent_cours c ON c.id = m.cours_id AND c.etablissement_id = m.etablissement_id
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
         WHERE m.etablissement_id = p_etab AND m.nature = 'inclusion' AND m.annulee_le IS NULL
           AND (p_cours IS NULL OR c.id = p_cours)
           AND (p_prof IS NULL OR ent_prof_sur_cours(p_etab, c.id, p_prof, p_jour, p_saisie))
           AND (p_eleve IS NULL OR m.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
           AND (p_cours IS NOT NULL OR NOT (c.source = 'reprise' AND c.a_verifier))
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
