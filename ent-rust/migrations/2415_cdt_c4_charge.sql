-- Socle 20, lot C4 — migration 2415 : charge de travail. Réf. : 20_CAHIER_DE_TEXTES.md §11, R-C16, R-C32.
-- Aucune table, aucune colonne JSON (D-036) : RLS et registre inchangés. Les repères (position souple / normal / strict) sont
-- déjà portés par `ent_cdt_reglage.repere_charge` ; les alertes ignorées par `ent_cdt_alerte_charge` (2410).
-- Seuls des index servent le calcul groupé par élève (une requête par nature de donnée) et le comptage des dérogations.
CREATE INDEX IF NOT EXISTS ent_cdt_travail_charge_idx ON ent_cdt_travail (etablissement_id, cours_id, echeance_le)
    WHERE etat = 'publie';
CREATE INDEX IF NOT EXISTS ent_cdt_alerte_charge_cree_idx ON ent_cdt_alerte_charge (etablissement_id, cree_le) WHERE ignoree;
