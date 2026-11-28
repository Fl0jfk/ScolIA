-- Socle 21, lot V11 « alertes précoces », partie déterministe (`docs/socles/21_EVALUATIONS.md` §13.3, §13.8, §14.2,
-- R-V20, R-V40, S-118 ; EVA-109, EVA-154, EVA-155, EVA-209, EVA-276, EVA-277 ; livraison :
-- `docs/socles/21_EVALUATIONS_V11_LIVRAISON.md`). Plage d'extension 700–719 du socle 21 (701 à 705 tenues par les PR
-- ouvertes de V14, V6, V7, V8 et V10) ; posée en FIN du tableau de `migrate.rs` (règle 38). Additive et rejouable ;
-- aucune migration versée n'est modifiée (règle 25).
--
-- Aucun modèle de langage : les alertes sont des RÈGLES écrites par des personnes (baisse de la moyenne générale,
-- matières sous le seuil, absences et non-rendus aux évaluations), lues avec la position de `eva_reglage.seuils_alertes`
-- (461, S-118 : souple, normal, strict). Rien n'est calculé ni gardé en base : une alerte se recalcule à chaque lecture,
-- elle n'a aucune conséquence automatique (RGPD art. 22, R-V20) et n'est jamais visible de la famille ni de l'élève.
--
-- Ce lot ajoute deux journaux en ajout seul :
--   * la SUITE HUMAINE donnée à une situation (vue, entretien, famille informée, cellule de veille, sans suite), posée
--     par une personne et datée : c'est la seule trace écrite d'une alerte, et elle est le fait d'une personne ;
--   * le journal des CONSULTATIONS des alertes (profilage au sens du RGPD, §13.8) : qui a regardé quelle vue, quand.
-- Aucune colonne JSON (D-036). Aucune fonction nouvelle.

-- 1. Suite humaine d'une alerte -------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS eva_alerte_suite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    periode_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('vue', 'entretien_prevu', 'entretien_fait', 'famille_informee',
                                           'cellule_de_veille', 'sans_suite')),
    -- Note interne, courte, jamais montrée à la famille ; aucune donnée de santé ni motif d'absence (minimisation).
    commentaire TEXT CHECK (commentaire IS NULL OR char_length(btrim(commentaire)) BETWEEN 1 AND 500),
    auteur_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_alerte_suite_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES public.ent_eleve (id, etablissement_id),
    CONSTRAINT eva_alerte_suite_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES public.ent_periode (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_alerte_suite_eleve_idx ON eva_alerte_suite (etablissement_id, eleve_id, periode_id, le);
DROP TRIGGER IF EXISTS eva_alerte_suite_ajout_seul ON eva_alerte_suite;
CREATE TRIGGER eva_alerte_suite_ajout_seul BEFORE UPDATE OR DELETE ON eva_alerte_suite
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_alerte_suite_vidage ON eva_alerte_suite;
CREATE TRIGGER eva_alerte_suite_vidage BEFORE TRUNCATE ON eva_alerte_suite
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- 2. Journal des consultations des alertes ---------------------------------------------------------------------------
-- Une ligne par lecture d'une vue d'alertes (classe, cours, changements d'un élève, suites d'un élève). Aucune valeur
-- calculée n'y est gardée : seulement qui a regardé quoi, et quand.
CREATE TABLE IF NOT EXISTS eva_alerte_consultation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    acteur_id UUID NOT NULL,
    vue TEXT NOT NULL CHECK (vue IN ('classe', 'cours', 'changements', 'suites')),
    periode_id UUID,
    classe_id UUID,
    cours_id UUID,
    eleve_id UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_alerte_consultation_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES public.ent_periode (id, etablissement_id),
    CONSTRAINT eva_alerte_consultation_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES public.ent_eleve (id, etablissement_id),
    CONSTRAINT eva_alerte_consultation_cible_chk CHECK (
        (vue = 'classe') = (classe_id IS NOT NULL)
        AND (vue = 'cours') = (cours_id IS NOT NULL)
        AND (vue IN ('changements', 'suites')) = (eleve_id IS NOT NULL)
        AND (vue <> 'suites') = (periode_id IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS eva_alerte_consultation_le_idx ON eva_alerte_consultation (etablissement_id, le);
DROP TRIGGER IF EXISTS eva_alerte_consultation_ajout_seul ON eva_alerte_consultation;
CREATE TRIGGER eva_alerte_consultation_ajout_seul BEFORE UPDATE OR DELETE ON eva_alerte_consultation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_alerte_consultation_vidage ON eva_alerte_consultation;
CREATE TRIGGER eva_alerte_consultation_vidage BEFORE TRUNCATE ON eva_alerte_consultation
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

SELECT ent_rls_poser(ARRAY['eva_alerte_suite', 'eva_alerte_consultation']);
