-- 2315 — Socle 15 (absences des personnels), lot A11f : seuil de la mention « répond rarement aux sollicitations »,
-- réglable par établissement (`docs/socles/15_ABSENCES_A4BIS.md` §6 ; décision de Florian du 2026-11-16).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036). Aucune nouvelle table : la colonne rejoint
-- `ent_absp_param` (une ligne par établissement, RLS déjà posée par la migration 2301, enregistrée au registre des
-- données). Pas de ligne, ou colonne par défaut : 3 demandes restées sans réponse (comportement d'origine, constante
-- `SEUIL_SANS_REPONSE_MIN` de `reponse.rs`). Le seuil ne sert qu'à l'organisateur des remplacements : la mention
-- n'est jamais lue d'un professeur (R-AP2).

ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS seuil_sans_reponse_min SMALLINT NOT NULL DEFAULT 3
    CHECK (seuil_sans_reponse_min BETWEEN 1 AND 20);
