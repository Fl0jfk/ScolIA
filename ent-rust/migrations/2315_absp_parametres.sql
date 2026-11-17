-- 2315 — Socle 15 (absences des personnels), lot A11f : seuil de la mention « répond rarement aux sollicitations »,
-- réglable par établissement (`docs/socles/15_ABSENCES_A4BIS.md` §6 ; décision de Florian du 2026-11-16).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036). Aucune nouvelle table : la colonne rejoint
-- `ent_absp_param` (une ligne par établissement, RLS déjà posée par la migration 2301, enregistrée au registre des
-- données). Pas de ligne, ou colonne par défaut : 3 demandes restées sans réponse (comportement d'origine, constante
-- `SEUIL_SANS_REPONSE_MIN` de `reponse.rs`). Le seuil ne sert qu'à l'organisateur des remplacements : la mention
-- n'est jamais lue d'un professeur (R-AP2).

ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS seuil_sans_reponse_min SMALLINT NOT NULL DEFAULT 3
    CHECK (seuil_sans_reponse_min BETWEEN 1 AND 20);
-- Date du dernier réglage du seuil (distincte de `modifie_le`, partagée avec les autres réglages, ex. l'EDT de référence).
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS seuil_modifie_le TIMESTAMPTZ;

-- Masquage des petits effectifs de la remontée RCD (S-1512, à confirmer avec le DPO) : sous 3 agents, la catégorie de
-- motif est remplacée par `non_detaille` et l'effectif n'est pas conservé (NULL = « < 3 »).
ALTER TABLE ent_absp_rcd_remontee_ligne DROP CONSTRAINT IF EXISTS ent_absp_rcd_remontee_ligne_categorie_nationale_check;
ALTER TABLE ent_absp_rcd_remontee_ligne ADD CONSTRAINT ent_absp_rcd_remontee_ligne_categorie_nationale_check
    CHECK (categorie_nationale IN ('raison_sante', 'evenement_familial', 'formation', 'institutionnel',
        'convenance_personnelle', 'en_attente_de_motif', 'non_detaille'));
ALTER TABLE ent_absp_rcd_remontee_ligne ADD COLUMN IF NOT EXISTS effectif SMALLINT
    CHECK (effectif IS NULL OR effectif >= 3);
