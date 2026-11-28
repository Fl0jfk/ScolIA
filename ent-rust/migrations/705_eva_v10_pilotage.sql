-- Socle 21, lot V10 « pilotage et statistiques » (`docs/socles/21_EVALUATIONS.md` §17.12, §14.6 point 10, R-V44 ;
-- EVA-153, EVA-156, EVA-212 à EVA-218, EVA-279 ; livraison : `docs/socles/21_EVALUATIONS_V10_LIVRAISON.md`). Plage
-- d'extension 700–719 du socle 21 (701 à 704 tenues par les PR ouvertes de V14, V6, V7 et V8) ; posée en FIN du tableau
-- de `migrate.rs` (règle 38). Additive et rejouable ; aucune migration versée n'est modifiée (règle 25).
--
-- L'évaluation commune existe depuis 462 (V1 : libellé, date, barème ; `eva_evaluation.commune_id`), sans aucun
-- service. Ce lot y ajoute :
--   * la marque « examen blanc » (ouvre la comparaison avec le contrôle continu, EVA-216) et l'auteur de la dernière
--     modification ;
--   * l'index de lecture des évaluations d'une commune ;
--   * le journal des consultations de statistiques (R-V44 : « consultations journalisées »), en ajout seul.
-- Aucune colonne JSON (D-036). Aucune fonction nouvelle.

-- 1. Évaluation commune : examen blanc --------------------------------------------------------------------------------
ALTER TABLE eva_evaluation_commune ADD COLUMN IF NOT EXISTS examen_blanc BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE eva_evaluation_commune ADD COLUMN IF NOT EXISTS modifie_par UUID;
ALTER TABLE eva_evaluation_commune ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ;

-- Lecture des évaluations d'une commune (comparaison, détachement).
CREATE INDEX IF NOT EXISTS eva_eval_commune_idx ON eva_evaluation (etablissement_id, commune_id)
    WHERE commune_id IS NOT NULL;

-- Une évaluation par cours dans une même commune (CE-46 : « une évaluation par cours concerné »). Les évaluations
-- annulées ne comptent pas : un cours dont l'évaluation a été annulée peut en rattacher une nouvelle.
CREATE UNIQUE INDEX IF NOT EXISTS eva_eval_commune_cours_uq ON eva_evaluation (etablissement_id, commune_id, cours_id)
    WHERE commune_id IS NOT NULL AND etat <> 'annulee';

-- 2. Journal des consultations de statistiques (R-V44) ----------------------------------------------------------------
-- Une ligne par lecture d'une vue statistique (statistiques de période, répartition par nature, harmonisation,
-- comparaison d'une évaluation commune, écarts à l'examen blanc). Ajout seul : ni réécriture, ni suppression, ni
-- troncature. Aucune valeur calculée n'y est gardée : seulement qui a regardé quoi, et quand.
CREATE TABLE IF NOT EXISTS eva_consultation_pilotage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    acteur_id UUID NOT NULL,
    vue TEXT NOT NULL CHECK (vue IN ('statistiques', 'natures', 'harmonisation', 'commune', 'ecarts_examen')),
    periode_id UUID,
    commune_id UUID,
    niveau_id UUID,
    axe TEXT CHECK (axe IS NULL OR axe IN ('matiere', 'classe', 'niveau', 'professeur', 'cours')),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_consultation_pilotage_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES public.ent_periode (id, etablissement_id),
    CONSTRAINT eva_consultation_pilotage_commune_fk FOREIGN KEY (commune_id, etablissement_id)
        REFERENCES public.eva_evaluation_commune (id, etablissement_id),
    CONSTRAINT eva_consultation_pilotage_cible_chk CHECK (
        (vue IN ('commune', 'ecarts_examen')) = (commune_id IS NOT NULL)
        AND (vue IN ('statistiques', 'natures', 'harmonisation')) = (periode_id IS NOT NULL)
        AND (vue = 'harmonisation') = (niveau_id IS NOT NULL)
        AND (vue = 'statistiques') = (axe IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS eva_consultation_pilotage_le_idx ON eva_consultation_pilotage (etablissement_id, le);
DROP TRIGGER IF EXISTS eva_consultation_pilotage_ajout_seul ON eva_consultation_pilotage;
CREATE TRIGGER eva_consultation_pilotage_ajout_seul BEFORE UPDATE OR DELETE ON eva_consultation_pilotage
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_consultation_pilotage_vidage ON eva_consultation_pilotage;
CREATE TRIGGER eva_consultation_pilotage_vidage BEFORE TRUNCATE ON eva_consultation_pilotage
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

SELECT ent_rls_poser(ARRAY['eva_consultation_pilotage']);
