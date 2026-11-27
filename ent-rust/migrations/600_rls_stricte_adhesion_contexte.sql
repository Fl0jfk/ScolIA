-- 600 — RLS stricte : accès transverse du déclencheur d'adhésion (docs/securite/RLS_STRICTE.md §4).
--
-- `ent_membership` (adhésion d'un compte à un établissement) est lue avant tout choix d'établissement : elle
-- n'est donc pas sous RLS. Mais ses déclencheurs écrivent ou lisent des tables **sous RLS** de
-- L'ÉTABLISSEMENT DE LA LIGNE (fiche personne, agent, engagement, fonction ; décompte des administrateurs).
-- Sans contexte, la RLS stricte les refuserait (aucune ligne visible, écriture refusée) : l'ouverture
-- d'une adhésion échouerait.
--
-- Mécanisme minimal et borné : deux déclencheurs encadrent les existants. Le premier pose
-- `app.etablissement_id` = établissement de la ligne (valable pour la seule transaction, `set_config(..., true)`),
-- le second rend la valeur précédente. Pas de contournement : aucune politique n'est modifiée, aucune
-- fonction n'est `SECURITY DEFINER`, le contexte posé n'ouvre QUE l'établissement de la ligne en cours.
-- Idempotent (rejoué à chaque démarrage).

CREATE OR REPLACE FUNCTION ent_membership_ctx_pose() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    cible UUID := COALESCE(NEW.etablissement_id, OLD.etablissement_id);
BEGIN
    -- Pile d'un niveau : la valeur de l'appelant (ou vide) est conservée jusqu'à `ctx_rend`.
    PERFORM set_config('app.rls_membership_prec', COALESCE(current_setting('app.etablissement_id', true), ''), true);
    PERFORM set_config('app.etablissement_id', cible::text, true);
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$;

CREATE OR REPLACE FUNCTION ent_membership_ctx_rend() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM set_config('app.etablissement_id', COALESCE(current_setting('app.rls_membership_prec', true), ''), true);
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$;

-- Ordre d'exécution = ordre alphabétique des noms : `…_a_…` avant les déclencheurs existants
-- (`ent_membership_agent_sync_trg`, `ent_membership_dernier_admin`), `…_zz_…` après.
DROP TRIGGER IF EXISTS ent_membership_a_ctx_pose_avant ON ent_membership;
CREATE TRIGGER ent_membership_a_ctx_pose_avant BEFORE DELETE OR UPDATE ON ent_membership
    FOR EACH ROW EXECUTE FUNCTION ent_membership_ctx_pose();
DROP TRIGGER IF EXISTS ent_membership_zz_ctx_rend_avant ON ent_membership;
CREATE TRIGGER ent_membership_zz_ctx_rend_avant BEFORE DELETE OR UPDATE ON ent_membership
    FOR EACH ROW EXECUTE FUNCTION ent_membership_ctx_rend();
DROP TRIGGER IF EXISTS ent_membership_a_ctx_pose_apres ON ent_membership;
CREATE TRIGGER ent_membership_a_ctx_pose_apres AFTER INSERT OR UPDATE OF role ON ent_membership
    FOR EACH ROW EXECUTE FUNCTION ent_membership_ctx_pose();
DROP TRIGGER IF EXISTS ent_membership_zz_ctx_rend_apres ON ent_membership;
CREATE TRIGGER ent_membership_zz_ctx_rend_apres AFTER INSERT OR UPDATE OF role ON ent_membership
    FOR EACH ROW EXECUTE FUNCTION ent_membership_ctx_rend();
