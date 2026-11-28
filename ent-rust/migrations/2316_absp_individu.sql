-- 2316 — Transverse « la deuxième personne fondée sur la personne physique », lot 3 : absences des personnels
-- (socle 15, R-AP10). Conception : docs/socles/TRANSVERSE_INDIVIDU.md §5. Plage du socle 15 (2316 vérifiée libre sur
-- toutes les branches). Règle 38 : en fin de tableau, après 279 (fonctions de l'individu). 2300 et 2302 sont versées et
-- ne changent pas (règle 25) : leurs fonctions sont recréées à l'identique, sauf la comparaison convertie, avec
-- `SET search_path` et des relations qualifiées `public.`. Les déclencheurs existants appellent ces fonctions par leur nom :
-- rien d'autre à recréer.
--
-- Personne ne valide ni ne refuse sa propre absence : ni depuis un autre de ses comptes, ni comme valideur d'un circuit
-- par une autre de ses fiches d'agent. L'individu ne fait que restreindre (principe 1) : l'annulation de sa propre absence
-- reste jugée sur le compte (dans l'application).

-- 1. Projection de transition `ent_absence_rh` (2300).
CREATE OR REPLACE FUNCTION ent_trg_absence_rh_valideur() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF NEW.valide_par_user_id IS NOT NULL
       AND public.ent_meme_individu(NEW.etablissement_id, NEW.valide_par_user_id, NEW.user_id)
       AND (TG_OP = 'INSERT' OR NEW.valide_par_user_id IS DISTINCT FROM OLD.valide_par_user_id) THEN
        RAISE EXCEPTION 'Personne ne valide sa propre absence (R-AP10)' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

-- 2. Valideur d'un circuit (2302) : pas une fiche de l'individu absent.
CREATE OR REPLACE FUNCTION ent_trg_absp_circuit_valideur() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    absent UUID;
    absent_personne UUID;
    valideur_personne UUID;
BEGIN
    IF NEW.role = 'valideur' THEN
        SELECT a.agent_id, ag.personne_id INTO absent, absent_personne
          FROM public.ent_absp_absence a
          LEFT JOIN public.ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
         WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.absence_id;
        SELECT ag.personne_id INTO valideur_personne
          FROM public.ent_agent ag
         WHERE ag.etablissement_id = NEW.etablissement_id AND ag.id = NEW.agent_id;
        IF absent = NEW.agent_id
           OR (absent_personne IS NOT NULL AND valideur_personne IS NOT NULL
               AND valideur_personne = ANY (public.ent_individu_personnes(NEW.etablissement_id,
                                                                          ARRAY[absent_personne]))) THEN
            RAISE EXCEPTION 'Le valideur d''une absence ne peut pas être l''absent (R-AP10)' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

-- 3. Décisions (2302) : ajout seul, et l'auteur d'une validation, d'une prise en compte ou d'un refus n'est pas
--    l'individu absent (par son agent, par l'un de ses comptes, ou par une fiche reliée à la sienne).
CREATE OR REPLACE FUNCTION ent_trg_absp_decision() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    absent UUID;
    absent_personne UUID;
    auteur_personne UUID;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
    END IF;
    IF NEW.type IN ('validation', 'prise_en_compte', 'refus') THEN
        SELECT a.agent_id, ag.personne_id INTO absent, absent_personne
          FROM public.ent_absp_absence a
          LEFT JOIN public.ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
         WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.absence_id;
        IF NEW.auteur_agent_id IS NOT NULL THEN
            SELECT ag.personne_id INTO auteur_personne
              FROM public.ent_agent ag
             WHERE ag.etablissement_id = NEW.etablissement_id AND ag.id = NEW.auteur_agent_id;
        END IF;
        IF (NEW.auteur_agent_id IS NOT NULL AND NEW.auteur_agent_id = absent)
           OR (absent_personne IS NOT NULL
               AND absent_personne = ANY (public.ent_individu_personnes_du_compte(NEW.etablissement_id,
                                                                                  NEW.auteur_user_id)))
           OR (absent_personne IS NOT NULL AND auteur_personne IS NOT NULL
               AND auteur_personne = ANY (public.ent_individu_personnes(NEW.etablissement_id,
                                                                        ARRAY[absent_personne]))) THEN
            RAISE EXCEPTION 'Personne ne valide ni ne refuse sa propre absence (R-AP10)' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
