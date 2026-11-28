-- Socle 21, lot V3f « barème détaillé, conversion des notes, duplication » (complément de V3, plage d'extension 700–719).
--
-- Les tables du barème détaillé existent depuis V1 (462 : `eva_bareme_item`, 464 : `eva_resultat_item`) mais aucun code ne
-- les écrivait. Ce lot les met en service (EVA-262 à 264, CE-39, CE-83, EVA-125 multiple) et les garde EN BASE :
--   * `eva_evaluation.detail_visible_eleve` : le professeur montre (ou non) le détail par exercice à l'élève et à sa famille,
--     avec la note publiée (EVA-264) ; faux par défaut ;
--   * `eva_bareme_item` : un exercice annulé (énoncé faux, CE-83) le reste, avec sa date et son auteur ; un exercice qui
--     porte des points ne change plus (ni libellé, ni maximum, ni rang) et ne se supprime pas ;
--   * `eva_resultat_item` : les points d'un exercice ne dépassent jamais son maximum, l'exercice et la note sont de la même
--     évaluation, aucun point ne s'écrit sur un exercice annulé ; `resultat_version` = version de la note que ce détail
--     compose (un détail dont la version diffère de celle de la note est PÉRIMÉ : la note a été changée à la main dans la
--     grille, le détail n'est plus montré) ; aucune suppression physique.
-- Additive et rejouable ; aucune migration versée n'est modifiée (règle 25) ; posée en FIN de tableau (règle 38) ; chaque
-- fonction porte `SET search_path = pg_catalog, public, pg_temp` et ne lit que des relations `public.`.

ALTER TABLE public.eva_evaluation ADD COLUMN IF NOT EXISTS detail_visible_eleve BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE public.eva_bareme_item ADD COLUMN IF NOT EXISTS annule_le TIMESTAMPTZ;
ALTER TABLE public.eva_bareme_item ADD COLUMN IF NOT EXISTS annule_par UUID;
ALTER TABLE public.eva_bareme_item DROP CONSTRAINT IF EXISTS eva_bareme_item_annule_chk;
ALTER TABLE public.eva_bareme_item ADD CONSTRAINT eva_bareme_item_annule_chk
    CHECK (NOT annule OR annule_le IS NOT NULL);

ALTER TABLE public.eva_resultat_item ADD COLUMN IF NOT EXISTS resultat_version INTEGER;
ALTER TABLE public.eva_resultat_item ADD COLUMN IF NOT EXISTS saisi_par UUID;
ALTER TABLE public.eva_resultat_item ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE public.eva_resultat_item DROP CONSTRAINT IF EXISTS eva_resultat_item_version_chk;
ALTER TABLE public.eva_resultat_item ADD CONSTRAINT eva_resultat_item_version_chk
    CHECK (resultat_version IS NULL OR resultat_version >= 1);
CREATE INDEX IF NOT EXISTS eva_resultat_item_item_idx ON public.eva_resultat_item (etablissement_id, item_id);

CREATE OR REPLACE FUNCTION ent_trg_eva_bareme_item() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    v_points boolean;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.annule THEN
            RAISE EXCEPTION 'eva_bareme_item : un exercice naît non annulé' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    v_points := EXISTS (SELECT 1 FROM public.eva_resultat_item ri
                         WHERE ri.etablissement_id = OLD.etablissement_id AND ri.item_id = OLD.id);
    IF TG_OP = 'DELETE' THEN
        IF OLD.annule OR v_points THEN
            RAISE EXCEPTION 'eva_bareme_item : un exercice annulé ou qui porte des points ne se supprime pas'
                USING ERRCODE = '23514';
        END IF;
        RETURN OLD;
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.evaluation_id IS DISTINCT FROM OLD.evaluation_id THEN
        RAISE EXCEPTION 'eva_bareme_item : l''évaluation d''un exercice ne change pas' USING ERRCODE = '23514';
    END IF;
    IF OLD.annule AND NOT NEW.annule THEN
        RAISE EXCEPTION 'eva_bareme_item : un exercice annulé le reste (CE-83)' USING ERRCODE = '23514';
    END IF;
    IF OLD.annule AND (NEW.annule_le IS DISTINCT FROM OLD.annule_le OR NEW.annule_par IS DISTINCT FROM OLD.annule_par) THEN
        RAISE EXCEPTION 'eva_bareme_item : la trace de l''annulation ne change pas' USING ERRCODE = '23514';
    END IF;
    IF (v_points OR OLD.annule)
       AND (NEW.libelle IS DISTINCT FROM OLD.libelle OR NEW.maximum IS DISTINCT FROM OLD.maximum
            OR NEW.ordre IS DISTINCT FROM OLD.ordre) THEN
        RAISE EXCEPTION 'eva_bareme_item : un exercice qui porte des points ne change plus (annuler l''exercice)'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS eva_bareme_item_garde ON public.eva_bareme_item;
CREATE TRIGGER eva_bareme_item_garde BEFORE INSERT OR UPDATE OR DELETE ON public.eva_bareme_item
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_bareme_item();
DROP TRIGGER IF EXISTS eva_bareme_item_vidage ON public.eva_bareme_item;
CREATE TRIGGER eva_bareme_item_vidage BEFORE TRUNCATE ON public.eva_bareme_item
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

CREATE OR REPLACE FUNCTION ent_trg_eva_resultat_item() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    v_item RECORD;
    v_eval_resultat uuid;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_resultat_item : suppression physique interdite (le détail se remplace)' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' AND (NEW.resultat_id IS DISTINCT FROM OLD.resultat_id OR NEW.item_id IS DISTINCT FROM OLD.item_id
                             OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id) THEN
        RAISE EXCEPTION 'eva_resultat_item : la note et l''exercice d''un détail ne changent pas' USING ERRCODE = '23514';
    END IF;
    SELECT i.evaluation_id, i.maximum, i.annule INTO v_item
      FROM public.eva_bareme_item i
     WHERE i.etablissement_id = NEW.etablissement_id AND i.id = NEW.item_id;
    SELECT r.evaluation_id INTO v_eval_resultat
      FROM public.eva_resultat r
     WHERE r.etablissement_id = NEW.etablissement_id AND r.id = NEW.resultat_id;
    IF v_item.evaluation_id IS NULL OR v_eval_resultat IS NULL OR v_item.evaluation_id <> v_eval_resultat THEN
        RAISE EXCEPTION 'eva_resultat_item : l''exercice et la note doivent être de la même évaluation' USING ERRCODE = '23514';
    END IF;
    IF v_item.annule AND (TG_OP = 'INSERT' OR NEW.valeur IS DISTINCT FROM OLD.valeur) THEN
        RAISE EXCEPTION 'eva_resultat_item : aucun point ne s''écrit sur un exercice annulé' USING ERRCODE = '23514';
    END IF;
    IF NEW.valeur > v_item.maximum THEN
        RAISE EXCEPTION 'eva_resultat_item : % points pour un exercice sur %', NEW.valeur, v_item.maximum
            USING ERRCODE = '23514';
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS eva_resultat_item_garde ON public.eva_resultat_item;
CREATE TRIGGER eva_resultat_item_garde BEFORE INSERT OR UPDATE OR DELETE ON public.eva_resultat_item
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_resultat_item();
DROP TRIGGER IF EXISTS eva_resultat_item_vidage ON public.eva_resultat_item;
CREATE TRIGGER eva_resultat_item_vidage BEFORE TRUNCATE ON public.eva_resultat_item
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
