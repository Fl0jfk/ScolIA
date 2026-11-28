-- Socle 21, lot V7 « famille et élève » (`docs/socles/21_EVALUATIONS.md` §10, §17.6, §17.15 ; EVA-159 à EVA-168,
-- EVA-269 à EVA-272 ; livraison : `docs/socles/21_EVALUATIONS_V7_LIVRAISON.md`). Plage d'extension 701–719 du socle 21 ;
-- posée en FIN du tableau de `migrate.rs` (règle 38). Additive et rejouable ; aucune migration versée n'est modifiée
-- (règle 25 : 466 reste telle quelle).
--
-- La table des demandes de révision (`eva_revision_demande`) existe depuis 466 (V1), avec sa RLS, mais sans service.
-- Ce lot y ajoute ce qu'il faut pour la tenir honnêtement :
--   * le rôle du demandeur (parent ou élève), figé à la demande ;
--   * l'auteur de la réponse (un professeur du cours ou la direction) et l'instant du retrait ;
--   * une seule demande OUVERTE par résultat et par demandeur (pas de relance en rafale) ;
--   * un garde : une demande ne se supprime pas, son texte, sa cible et son auteur ne changent jamais, et elle ne
--     quitte l'état « ouverte » qu'une fois (réponse ou retrait), jamais pour y revenir.
-- Aucune colonne JSON (D-036). Fonction avec `search_path` fixé, relations qualifiées `public.` (aucune lue ici).

ALTER TABLE public.eva_revision_demande ADD COLUMN IF NOT EXISTS demandeur_role TEXT;
ALTER TABLE public.eva_revision_demande ADD COLUMN IF NOT EXISTS repondue_par UUID;
ALTER TABLE public.eva_revision_demande ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'eva_revision_demandeur_role_chk') THEN
        ALTER TABLE public.eva_revision_demande ADD CONSTRAINT eva_revision_demandeur_role_chk
            CHECK (demandeur_role IS NULL OR demandeur_role IN ('parent', 'eleve'));
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS eva_revision_ouverte_uq
    ON public.eva_revision_demande (etablissement_id, resultat_id, demandeur) WHERE etat = 'ouverte';
CREATE INDEX IF NOT EXISTS eva_revision_resultat_idx
    ON public.eva_revision_demande (etablissement_id, resultat_id);
CREATE INDEX IF NOT EXISTS eva_revision_demandeur_idx
    ON public.eva_revision_demande (etablissement_id, demandeur, etat);

CREATE OR REPLACE FUNCTION eva_revision_demande_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_revision_demande : une demande de révision ne se supprime pas' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.resultat_id IS DISTINCT FROM OLD.resultat_id OR NEW.demandeur IS DISTINCT FROM OLD.demandeur
       OR NEW.demandeur_role IS DISTINCT FROM OLD.demandeur_role OR NEW.message IS DISTINCT FROM OLD.message
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'eva_revision_demande : une demande ne se réécrit pas' USING ERRCODE = '23514';
    END IF;
    IF OLD.etat <> 'ouverte' THEN
        RAISE EXCEPTION 'eva_revision_demande : demande déjà close' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'repondue' AND (NEW.repondue_par IS NULL OR NEW.repondue_le IS NULL OR NEW.retiree_le IS NOT NULL) THEN
        RAISE EXCEPTION 'eva_revision_demande : une réponse porte son auteur et sa date' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'retiree' AND (NEW.retiree_le IS NULL OR NEW.reponse IS NOT NULL OR NEW.repondue_par IS NOT NULL) THEN
        RAISE EXCEPTION 'eva_revision_demande : un retrait porte sa date et aucune réponse' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'ouverte' AND (NEW.reponse IS NOT NULL OR NEW.repondue_par IS NOT NULL OR NEW.repondue_le IS NOT NULL
                                 OR NEW.retiree_le IS NOT NULL) THEN
        RAISE EXCEPTION 'eva_revision_demande : une demande ouverte ne porte ni réponse ni retrait' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_revision_demande_garde_trg ON public.eva_revision_demande;
CREATE TRIGGER eva_revision_demande_garde_trg BEFORE UPDATE OR DELETE ON public.eva_revision_demande
    FOR EACH ROW EXECUTE FUNCTION eva_revision_demande_garde();
