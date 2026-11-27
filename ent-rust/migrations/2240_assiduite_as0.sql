-- 2240 — Socle 12, lot AS0 (correctifs urgents) : traçabilité de l'appel, regroupement par
-- journée, anti-doublon des déclarations. Réf. : 12_ASSIDUITE.md §23-24 (DA-05, DA-06, DA-22).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036).

ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS valide_par UUID;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS valide_le TIMESTAMPTZ;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS modifie_par UUID;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ;

-- Regroupement des absences d'un élève par jour (clôture d'appel).
CREATE INDEX IF NOT EXISTS ent_absence_a_traiter_jour_idx
    ON ent_absence (etablissement_id, eleve_id, debut, fin)
    WHERE statut = 'a_traiter';

-- Anti-doublon (DA-22) : les doublons actifs existants sont annulés (le plus ancien est gardé),
-- puis une déclaration active identique est refusée.
UPDATE ent_absence_declaration d
   SET statut = 'annulee', annulee_at = now()
  FROM (
        SELECT id, row_number() OVER (
                   PARTITION BY etablissement_id, eleve_id, source, debut_jour,
                                COALESCE(debut_heure, '00:00'::time), fin_jour,
                                COALESCE(fin_heure, '23:59'::time), motif
                   ORDER BY created_at, id) AS rang
          FROM ent_absence_declaration
         WHERE statut = 'active'
       ) x
 WHERE d.id = x.id AND x.rang > 1;

CREATE UNIQUE INDEX IF NOT EXISTS ent_absence_declaration_unique_active_idx
    ON ent_absence_declaration (etablissement_id, eleve_id, source, debut_jour,
                                COALESCE(debut_heure, '00:00'::time), fin_jour,
                                COALESCE(fin_heure, '23:59'::time), motif)
    WHERE statut = 'active';
