-- Messagerie, suites E1 (passe 4) : `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md`, « Suites, passe 4 ».
-- Plage 482–499 du socle 16 : 494 est prise ici ; 482–493 sont versées et ne changent pas (règle 25). Rejouable.
-- Placée à la FIN de `migrate.rs` (règle 38). Aucune table, aucune fonction créée ou recréée (donc aucun `search_path`
-- à poser) : seuls des droits d'exécution changent.
--
-- M1 : `ent_msg_annulation_ouvrir(etab, message, auteur)` (493) est appelable par `PUBLIC` (droit par défaut d'une
-- fonction) et `p_auteur` est affirmé par l'appelant : tout compte de la base pouvait demander un jeton d'annulation.
-- Le droit d'exécution est retiré à `PUBLIC` et donné aux seuls rôles de l'application, c'est-à-dire ceux qui écrivent
-- déjà dans `ent_msg` (en cible : `ent_app`, `docs/RLS_ISOLATION.md` §1). Le propriétaire garde son droit.
-- Limite documentée : la base ne connaît pas l'utilisateur de l'application ; un rôle applicatif compromis qui
-- connaît l'auteur d'un message **planifié** peut encore ouvrir son jeton (il peut déjà l'annuler par l'API).
-- Si le rôle applicatif est créé après cette migration : `GRANT EXECUTE ON FUNCTION
-- ent_msg_annulation_ouvrir(uuid, uuid, uuid) TO ent_app;` (script de déploiement).

REVOKE EXECUTE ON FUNCTION public.ent_msg_annulation_ouvrir(UUID, UUID, UUID) FROM PUBLIC;

DO $$
DECLARE
    g RECORD;
BEGIN
    FOR g IN
        SELECT DISTINCT a.grantee
          FROM pg_catalog.pg_class c
          CROSS JOIN LATERAL pg_catalog.aclexplode(c.relacl) a
         WHERE c.oid = 'public.ent_msg'::pg_catalog.regclass
           AND a.grantee <> 0
           AND a.grantee <> c.relowner
           AND a.privilege_type = 'INSERT'
    LOOP
        EXECUTE pg_catalog.format(
            'GRANT EXECUTE ON FUNCTION public.ent_msg_annulation_ouvrir(UUID, UUID, UUID) TO %I',
            pg_catalog.pg_get_userbyid(g.grantee));
    END LOOP;
END
$$;
