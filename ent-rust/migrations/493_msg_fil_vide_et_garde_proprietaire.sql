-- Messagerie, suites E1 (passe 3) : `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md`, « Suites (passe 3) ».
-- Plage 482–499 du socle 16 : 493 est prise ici ; 482–492 sont versées et ne changent pas (règle 25). Rejouable.
-- Placée APRÈS 690 dans `migrate.rs` (règle 38). Aucune table nouvelle.
--
-- F-6 : un fil dont le **seul** message a été annulé restait listé à la famille avec son sujet. La branche « fil sans
-- aucun message » de `ent_msg_fil_lisible` (485) n'est plus ouverte qu'au **créateur du fil** (le premier message en
-- deux temps d'un parent : le fil naît avant son message) et au côté établissement. Même corps que 485 : seule cette
-- branche change ; fonction `LANGUAGE sql` qui retourne un ensemble, inlinée (famille « qualifiées » de 690) : aucune
-- clause `SET` (elle ferait perdre l'inlining), toutes les relations qualifiées `public.`.
--
-- Jeton de garde d'annulation (492) : `ent_msg_annulation_ouvrir` inscrivait un jeton pour tout message du
-- périmètre, appelable par le rôle applicatif, qui pouvait aussi écrire directement dans la table de garde.
--   1. la fonction reçoit l'**auteur** et refuse tout message dont il n'est pas l'auteur, ou déjà remis ;
--   2. elle devient `SECURITY DEFINER` (propriétaire = rôle de migration, `search_path` fixé) ;
--   3. `INSERT`, `UPDATE`, `DELETE` et `TRUNCATE` sur la table de garde sont retirés à tout rôle autre que le
--      propriétaire (le rôle applicatif ne lit plus que : le déclencheur d'ajout seul lit la table avec ses droits).
-- Limite : tant que l'application tourne avec le rôle propriétaire (`docs/RLS_ISOLATION.md` §1), rien de cela ne la lie ;
-- cela vaut avec un rôle applicatif non propriétaire (`ent_app`).

CREATE OR REPLACE FUNCTION ent_msg_fil_lisible(p_etab UUID, p_user UUID)
RETURNS TABLE (fil_id UUID)
LANGUAGE sql STABLE AS $$
    WITH moi AS (
        SELECT pe.id FROM public.ent_personne pe
         WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user AND pe.fusionnee_dans IS NULL
        UNION
        SELECT s.id FROM public.ent_personne s
          JOIN public.ent_personne pe ON pe.id = s.fusionnee_dans AND pe.etablissement_id = s.etablissement_id
         WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user
    ), mon_eleve AS (
        SELECT c.eleve_id FROM public.ent_eleve_compte c
         WHERE c.etablissement_id = p_etab AND c.user_id = p_user
    ), mes_boites AS (
        SELECT b.id FROM public.ent_msg_boite b
         WHERE b.etablissement_id = p_etab AND b.actif
           AND EXISTS (SELECT 1 FROM public.ent_msg_boite_membres(p_etab, b.id) bm WHERE bm.user_id = p_user)
    )
    SELECT f.id
      FROM public.ent_msg_fil f
     WHERE f.etablissement_id = p_etab
       AND EXISTS (
            SELECT 1 FROM public.ent_msg_participant p
             WHERE p.etablissement_id = f.etablissement_id AND p.fil_id = f.id
               AND p.au IS NULL AND p.suspendu_le IS NULL
               AND (p.personne_id IN (SELECT id FROM moi)
                    OR p.eleve_id IN (SELECT eleve_id FROM mon_eleve)
                    OR p.boite_id IN (SELECT id FROM mes_boites))
               AND (p.cote <> 'famille'
                    OR EXISTS (SELECT 1 FROM public.ent_eleve_lien_communicant l
                                WHERE l.etablissement_id = f.etablissement_id AND l.user_id = p_user
                                  AND (l.eleve_id = f.eleve_id
                                       OR l.eleve_id IN (SELECT fe.eleve_id FROM public.ent_msg_fil_eleve fe
                                                          WHERE fe.etablissement_id = f.etablissement_id
                                                            AND fe.fil_id = f.id))
                                  AND (NOT f.vie_scolaire OR (l.peut_voir_vs AND l.autorite_parentale))
                                  AND (f.foyer_id IS NULL
                                       OR (l.foyer_id = f.foyer_id AND l.est_payeur))))
               AND (p.cote = 'etablissement'
                    -- F-6 : un fil sans aucun message n'est lisible que de son créateur (côté famille).
                    OR (f.created_by_user_id = p_user
                        AND NOT EXISTS (SELECT 1 FROM public.ent_msg m
                                         WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id))
                    OR EXISTS (SELECT 1 FROM public.ent_msg m
                                WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id
                                  AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now()
                                       OR m.auteur_user_id = p_user))))
$$;

-- La version à deux arguments (492) disparaît : sinon le rôle applicatif l'appellerait toujours.
DROP FUNCTION IF EXISTS ent_msg_annulation_ouvrir(UUID, UUID);

CREATE OR REPLACE FUNCTION ent_msg_annulation_ouvrir(p_etab UUID, p_msg UUID, p_auteur UUID) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    -- Jetons des transactions terminées (toute transaction d'identifiant inférieur à `xmin` l'est) : plus rien ne les
    -- lit ; la purge borne la taille de la table sans tâche de fond. Faite avec les droits du propriétaire.
    DELETE FROM public.ent_msg_annulation_garde g
     WHERE g.xid < pg_catalog.pg_snapshot_xmin(pg_catalog.pg_current_snapshot());
    -- Le jeton ne se demande que pour l'établissement du contexte courant (aucun jeton « ailleurs », même sans RLS).
    IF public.ent_rls_etablissement() IS NOT NULL AND public.ent_rls_etablissement() <> p_etab THEN
        RAISE EXCEPTION 'ent_msg : annulation refusée (autre établissement)'
            USING ERRCODE = 'check_violation';
    END IF;
    -- Seul l'auteur annule, et seulement un message dont la remise est à venir.
    IF NOT EXISTS (SELECT 1 FROM public.ent_msg m
                    WHERE m.etablissement_id = p_etab AND m.id = p_msg
                      AND m.auteur_user_id = p_auteur
                      AND m.remise_prevue_le > pg_catalog.now()) THEN
        RAISE EXCEPTION 'ent_msg : annulation refusée (message remis, inconnu ou d''un autre auteur)'
            USING ERRCODE = 'check_violation';
    END IF;
    INSERT INTO public.ent_msg_annulation_garde (etablissement_id, message_id, xid)
    VALUES (p_etab, p_msg, pg_catalog.pg_current_xact_id())
    ON CONFLICT DO NOTHING;
END
$$;

-- Écriture directe interdite à tout rôle autre que le propriétaire (PUBLIC compris, et le rôle applicatif quel que soit
-- son nom, y compris ceux des privilèges par défaut du script de déploiement). Rejouable.
DO $$
DECLARE
    g RECORD;
BEGIN
    FOR g IN
        SELECT DISTINCT a.grantee
          FROM pg_catalog.pg_class c
          CROSS JOIN LATERAL pg_catalog.aclexplode(c.relacl) a
         WHERE c.oid = 'public.ent_msg_annulation_garde'::pg_catalog.regclass
           AND a.grantee <> c.relowner
           AND a.privilege_type IN ('INSERT', 'UPDATE', 'DELETE', 'TRUNCATE')
    LOOP
        EXECUTE pg_catalog.format(
            'REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON public.ent_msg_annulation_garde FROM %s',
            CASE WHEN g.grantee = 0 THEN 'PUBLIC' ELSE pg_catalog.quote_ident(pg_catalog.pg_get_userbyid(g.grantee)) END);
    END LOOP;
END
$$;
