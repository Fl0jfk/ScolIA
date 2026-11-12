-- Messagerie, lot M1 : reprise des fils existants (MIG-1 à MIG-4, `16_MESSAGERIE.md` §26 et §9.2).
-- Suite de 482 (tables, fonctions de participation et de lecture). Migration rejouable : un fil
-- repris porte `reprise_m1_le` et n'est jamais retraité ; un fil créé après M1 le porte dès
-- l'insertion (posé par le code).
--
-- Règle de reprise : « aucun lecteur ne gagne ni ne perd un fil, sauf le cas MIG-3 documenté »
--   MIG-1 fil ouvert par un parent (`prive_user_id`) : ce parent + le personnel qui y a écrit
--         (la boîte « vie scolaire » du §26 viendra avec M3 ; le pont des rôles réservés la remplace) ;
--   MIG-2 fil de relance (`foyer_id`) : les payeurs actifs du foyer + le personnel qui y a écrit ;
--   MIG-3 fil ouvert par l'établissement sans portée : les responsables qui ont **réellement lu ou
--         écrit**, plus ceux du même foyer ; deux foyers ayant lu → fil **dédoublé** (fils
--         jumeaux, messages du foyer déplacés, messages du personnel copiés avec `copie_de`) ;
--         personne n'ayant lu → un fil par foyer ; rapport pour la direction ;
--   MIG-4 fil ouvert par un élève (audience `eleve`) : l'élève + le personnel qui y a écrit ;
--         **aucun** parent.
--   Fils de vie scolaire (alerte d'absences, carnet) : tous les titulaires avec droit « vie
--         scolaire » ; deux foyers dans un même fil sont **signalés** (rapport), le dédoublement
--         vient avec le carnet (M7, MIG-5), car `ent_carnet_entree.fil_notif_id` en dépend.

ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS reprise_m1_le TIMESTAMPTZ;

-- Dédouble un fil : les comptes `p_users` (un foyer) passent dans un fil jumeau ---------------------
--  * leurs messages sont **déplacés** dans le jumeau (avec leurs pièces et accusés) ;
--  * les messages du personnel sont **copiés** (même contenu, mêmes horodatages, `copie_de`) ;
--  * le personnel participant suit ; la famille du jumeau est ce foyer seul ;
--  * les comptes sortent du fil d'origine (`separation`) s'ils y participaient (MSG-118, MSG-284) ;
--  * un accusé de lecture posé par un autre responsable sur un message déplacé est supprimé :
--    « qui a lu le message de la mère » ne doit pas fuir vers l'autre foyer (R-M4).
-- Mode `separation` (MSG-118, CM-03, CM-04) : un responsable demande son propre fil, ou la direction
-- le sépare. Les parents **ont déjà lu** le passé commun : **tous** les messages sont copiés dans
-- le jumeau (le passé reste lisible des deux), aucun n'est déplacé ; seuls les nouveaux messages
-- ne sont plus communs. Mode `reprise` (défaut) : voir ci-dessus.
CREATE OR REPLACE FUNCTION ent_msg_dedoubler(
    p_fil UUID, p_users UUID[], p_regle TEXT, p_effet TEXT, p_mode TEXT DEFAULT 'reprise')
RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE
    f ent_msg_fil%ROWTYPE;
    g ent_msg_fil%ROWTYPE;
    m ent_msg%ROWTYPE;
    m2 ent_msg%ROWTYPE;
    pc ent_msg_piece%ROWTYPE;
    v_twin UUID := gen_random_uuid();
    v_foyer UUID;
    v_famille BOOLEAN;
BEGIN
    SELECT * INTO f FROM ent_msg_fil WHERE id = p_fil FOR UPDATE;
    IF NOT FOUND OR p_users IS NULL OR cardinality(p_users) = 0 THEN
        RETURN NULL;
    END IF;
    -- Le foyer du groupe, s'il est unique.
    SELECT min(l.foyer_id::text)::uuid INTO v_foyer
      FROM ent_eleve_lien l
     WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
       AND l.user_id = ANY (p_users)
    HAVING count(DISTINCT l.foyer_id) = 1 AND count(l.foyer_id) = count(*);

    g := f;
    g.id := v_twin;
    g.jumeau_de := COALESCE(f.jumeau_de, f.id);
    g.foyer_lecteur_id := v_foyer;
    g.prive_user_id := CASE WHEN v_foyer IS NULL AND cardinality(p_users) = 1
                            THEN p_users[1] ELSE f.prive_user_id END;
    g.reprise_m1_le := now();
    INSERT INTO ent_msg_fil SELECT (g).*;

    INSERT INTO ent_msg_fil_eleve (etablissement_id, fil_id, eleve_id)
    SELECT e.etablissement_id, v_twin, e.eleve_id FROM ent_msg_fil_eleve e WHERE e.fil_id = p_fil
    ON CONFLICT DO NOTHING;

    -- Le personnel suit dans le jumeau ; la famille du jumeau est ce foyer seul.
    INSERT INTO ent_msg_participant
        (id, etablissement_id, fil_id, personne_id, eleve_id, cote, qualite, raison, peut_ecrire, du)
    SELECT gen_random_uuid(), p.etablissement_id, v_twin, p.personne_id, p.eleve_id, p.cote, p.qualite,
           p.raison, p.peut_ecrire, p.du
      FROM ent_msg_participant p
     WHERE p.fil_id = p_fil AND p.au IS NULL AND p.cote = 'etablissement'
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_msg_participant
        (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
    SELECT gen_random_uuid(), f.etablissement_id, v_twin,
           ent_msg_personne_du_compte(f.etablissement_id, u), 'famille', 'destinataire', 'titulaire_foyer'
      FROM unnest(p_users) AS u
    ON CONFLICT DO NOTHING;
    -- Sortie du fil d'origine (si ces responsables y participaient).
    UPDATE ent_msg_participant p
       SET au = now(), motif_sortie = 'separation'
     WHERE p.fil_id = p_fil AND p.au IS NULL AND p.cote = 'famille'
       AND p.personne_id IN (SELECT pe.id FROM ent_personne pe
                              WHERE pe.etablissement_id = f.etablissement_id AND pe.user_id = ANY (p_users));

    FOR m IN SELECT * FROM ent_msg WHERE fil_id = p_fil ORDER BY created_at, id LOOP
        IF p_mode = 'separation' THEN
            m2 := m;
            m2.id := gen_random_uuid();
            m2.fil_id := v_twin;
            m2.copie_de := m.id;
            m2.reponse_a := NULL;
            INSERT INTO ent_msg SELECT (m2).*;
            FOR pc IN SELECT * FROM ent_msg_piece WHERE message_id = m.id LOOP
                pc.id := gen_random_uuid();
                pc.message_id := m2.id;
                pc.fil_id := v_twin;
                INSERT INTO ent_msg_piece SELECT (pc).*;
            END LOOP;
            UPDATE ent_msg_accuse SET message_id = m2.id, fil_id = v_twin
             WHERE message_id = m.id AND lecteur_user_id = ANY (p_users);
        ELSIF m.auteur_user_id = ANY (p_users) THEN
            UPDATE ent_msg SET fil_id = v_twin WHERE id = m.id;
            UPDATE ent_msg_accuse SET fil_id = v_twin WHERE message_id = m.id;
            UPDATE ent_msg_piece SET fil_id = v_twin WHERE message_id = m.id;
            DELETE FROM ent_msg_accuse a
             WHERE a.message_id = m.id AND a.lecteur_user_id <> ALL (p_users)
               AND EXISTS (SELECT 1 FROM ent_membership mb
                            WHERE mb.user_id = a.lecteur_user_id AND mb.etablissement_id = m.etablissement_id
                              AND mb.role IN ('parent', 'eleve'));
        ELSIF NOT EXISTS (SELECT 1 FROM ent_membership mb
                           WHERE mb.user_id = m.auteur_user_id AND mb.etablissement_id = m.etablissement_id
                             AND mb.role IN ('parent', 'eleve'))
              AND EXISTS (SELECT 1 FROM ent_membership mb
                           WHERE mb.user_id = m.auteur_user_id AND mb.etablissement_id = m.etablissement_id) THEN
            m2 := m;
            m2.id := gen_random_uuid();
            m2.fil_id := v_twin;
            m2.copie_de := m.id;
            m2.reponse_a := NULL;
            INSERT INTO ent_msg SELECT (m2).*;
            FOR pc IN SELECT * FROM ent_msg_piece WHERE message_id = m.id LOOP
                pc.id := gen_random_uuid();
                pc.message_id := m2.id;
                pc.fil_id := v_twin;
                INSERT INTO ent_msg_piece SELECT (pc).*;
            END LOOP;
            UPDATE ent_msg_accuse SET message_id = m2.id, fil_id = v_twin
             WHERE message_id = m.id AND lecteur_user_id = ANY (p_users);
        ELSE
            -- Le message d'un autre foyer reste dans le fil d'origine : les accusés des
            -- responsables du jumeau sur ce message n'y restent pas (R-M4).
            DELETE FROM ent_msg_accuse a
             WHERE a.message_id = m.id AND a.lecteur_user_id = ANY (p_users);
        END IF;
    END LOOP;

    UPDATE ent_msg_fil x SET last_message_at = COALESCE(
            (SELECT max(mm.created_at) FROM ent_msg mm WHERE mm.fil_id = x.id), x.created_at)
     WHERE x.id IN (p_fil, v_twin);

    INSERT INTO ent_msg_reprise_rapport (id, etablissement_id, fil_id, fil_jumeau_id, regle, effet)
    VALUES (gen_random_uuid(), f.etablissement_id, p_fil, v_twin, p_regle, p_effet);
    RETURN v_twin;
END
$$;

-- Reprise d'un fil sans participants --------------------------------------------------------------
CREATE OR REPLACE FUNCTION ent_msg_m1_reprise_fil(p_fil UUID) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    f ent_msg_fil%ROWTYPE;
    v_role_ouvreur TEXT;
    v_nature TEXT;
    v_origine TEXT;
    v_regle TEXT;
    v_has_impl BOOLEAN;
    v_rang INTEGER := 0;
    r RECORD;
    w RECORD;
    v_foyers INTEGER;
BEGIN
    SELECT * INTO f FROM ent_msg_fil WHERE id = p_fil FOR UPDATE;
    IF NOT FOUND OR f.reprise_m1_le IS NOT NULL THEN
        RETURN;
    END IF;
    SELECT mb.role INTO v_role_ouvreur FROM ent_membership mb
     WHERE mb.user_id = f.created_by_user_id AND mb.etablissement_id = f.etablissement_id;

    v_nature := CASE
        WHEN f.audience = 'eleve' THEN 'eleve'
        WHEN f.foyer_id IS NOT NULL THEN 'systeme'
        WHEN f.vie_scolaire AND EXISTS (SELECT 1 FROM ent_carnet_entree c
                                         WHERE c.etablissement_id = f.etablissement_id
                                           AND c.fil_notif_id = f.id) THEN 'carnet'
        WHEN f.vie_scolaire THEN 'systeme'
        ELSE 'famille' END;
    v_origine := CASE
        WHEN v_role_ouvreur = 'eleve' THEN 'eleve'
        WHEN v_role_ouvreur = 'parent' THEN 'famille'
        WHEN f.foyer_id IS NOT NULL OR f.vie_scolaire THEN 'systeme'
        ELSE 'personnel' END;
    UPDATE ent_msg_fil SET nature = v_nature, origine = v_origine,
           ouvert_par_personne_id = (SELECT pe.id FROM ent_personne pe
                                      WHERE pe.etablissement_id = f.etablissement_id
                                        AND pe.user_id = f.created_by_user_id AND pe.fusionnee_dans IS NULL)
     WHERE id = f.id;
    f.origine := v_origine;
    INSERT INTO ent_msg_fil_eleve (etablissement_id, fil_id, eleve_id)
    VALUES (f.etablissement_id, f.id, f.eleve_id) ON CONFLICT DO NOTHING;

    -- Le personnel : celui qui a ouvert le fil, et tout personnel qui y a écrit.
    IF v_role_ouvreur IS NOT NULL AND v_role_ouvreur NOT IN ('parent', 'eleve') THEN
        PERFORM ent_msg_ajouter_participant(f.id, f.created_by_user_id, 'auteur', 'auteur');
    END IF;
    FOR w IN SELECT DISTINCT m.auteur_user_id AS u
               FROM ent_msg m
               JOIN ent_membership mb ON mb.user_id = m.auteur_user_id
                                     AND mb.etablissement_id = m.etablissement_id
              WHERE m.fil_id = f.id AND mb.role NOT IN ('parent', 'eleve') LOOP
        PERFORM ent_msg_ajouter_participant(f.id, w.u, 'ajoute', 'a_ecrit');
    END LOOP;

    -- La famille et l'élève.
    IF f.audience = 'eleve' OR f.prive_user_id IS NOT NULL OR f.foyer_id IS NOT NULL
       OR f.foyer_lecteur_id IS NOT NULL OR f.vie_scolaire THEN
        PERFORM ent_msg_peupler_famille(f.id);
        v_regle := CASE WHEN f.audience = 'eleve' THEN 'MIG-4'
                        WHEN f.prive_user_id IS NOT NULL THEN 'MIG-1'
                        WHEN f.foyer_id IS NOT NULL THEN 'MIG-2'
                        WHEN f.vie_scolaire THEN 'VIE_SCOLAIRE'
                        ELSE 'MIG-3' END;
        IF f.vie_scolaire THEN
            SELECT count(DISTINCT COALESCE(l.foyer_id::text, 'u:' || l.user_id::text)) INTO v_foyers
              FROM ent_eleve_lien_communicant l
             WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
               AND l.peut_voir_vs AND l.autorite_parentale;
            IF v_foyers > 1 THEN
                INSERT INTO ent_msg_reprise_rapport (id, etablissement_id, fil_id, regle, effet)
                VALUES (gen_random_uuid(), f.etablissement_id, f.id, 'VIE_SCOLAIRE', 'foyers_dans_un_fil');
            END IF;
        END IF;
    ELSE
        -- MIG-3 : fil de l'établissement sans portée. Les responsables qui ont lu ou écrit,
        -- groupés par foyer (un responsable sans foyer forme son propre groupe).
        SELECT EXISTS (
            SELECT 1 FROM ent_eleve_lien_communicant l
             WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
               AND (EXISTS (SELECT 1 FROM ent_msg_accuse a
                             WHERE a.fil_id = f.id AND a.lecteur_user_id = l.user_id)
                    OR EXISTS (SELECT 1 FROM ent_msg m
                                WHERE m.fil_id = f.id AND m.auteur_user_id = l.user_id))
        ) INTO v_has_impl;
        FOR r IN
            WITH liens AS (
                SELECT l.user_id, COALESCE(l.foyer_id::text, 'u:' || l.user_id::text) AS cle,
                       l.created_at AS lien_le
                  FROM ent_eleve_lien_communicant l
                 WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id),
            impl AS (
                SELECT x.user_id, min(x.t) AS t
                  FROM (SELECT a.lecteur_user_id AS user_id, a.lu_at AS t
                          FROM ent_msg_accuse a WHERE a.fil_id = f.id
                        UNION ALL
                        SELECT m.auteur_user_id, m.created_at FROM ent_msg m WHERE m.fil_id = f.id) x
                 GROUP BY x.user_id)
            SELECT li.cle, array_agg(DISTINCT li.user_id) AS users, min(i.t) AS premier,
                   min(li.lien_le) AS lien_le
              FROM liens li LEFT JOIN impl i ON i.user_id = li.user_id
             GROUP BY li.cle
            HAVING (NOT v_has_impl) OR min(i.t) IS NOT NULL
             ORDER BY min(i.t) NULLS LAST, min(li.lien_le), li.cle
        LOOP
            v_rang := v_rang + 1;
            IF v_rang = 1 THEN
                INSERT INTO ent_msg_participant
                    (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
                SELECT gen_random_uuid(), f.etablissement_id, f.id,
                       ent_msg_personne_du_compte(f.etablissement_id, u), 'famille', 'destinataire',
                       'titulaire_foyer'
                  FROM unnest(r.users) AS u
                ON CONFLICT DO NOTHING;
                -- Les colonnes héritées disent désormais à qui s'adresse le fil.
                UPDATE ent_msg_fil x
                   SET foyer_lecteur_id = (SELECT min(l.foyer_id::text)::uuid FROM ent_eleve_lien l
                                            WHERE l.etablissement_id = f.etablissement_id
                                              AND l.eleve_id = f.eleve_id AND l.user_id = ANY (r.users)
                                           HAVING count(DISTINCT l.foyer_id) = 1
                                              AND count(l.foyer_id) = count(*))
                 WHERE x.id = f.id;
                v_regle := 'MIG-3';
            ELSE
                PERFORM ent_msg_dedoubler(f.id, r.users, 'MIG-3',
                    CASE WHEN v_has_impl THEN 'dedouble_deux_foyers' ELSE 'un_fil_par_foyer' END);
            END IF;
        END LOOP;
    END IF;

    -- Des responsables communicants qui n'ont ni lu ni écrit n'ont reçu ni participation ni jumeau
    -- (MIG-3, un foyer a lu) : à signaler à la direction, jamais en silence.
    IF v_rang > 0 AND v_has_impl AND EXISTS (
        SELECT 1 FROM ent_eleve_lien_communicant l
         WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
           AND NOT EXISTS (SELECT 1 FROM ent_msg_participant p
                             JOIN ent_personne pe ON pe.id = p.personne_id
                            WHERE p.fil_id = f.id AND pe.user_id = l.user_id)
           AND NOT EXISTS (SELECT 1 FROM ent_msg_reprise_rapport x
                            JOIN ent_msg_participant p2 ON p2.fil_id = x.fil_jumeau_id
                            JOIN ent_personne pe2 ON pe2.id = p2.personne_id
                           WHERE x.fil_id = f.id AND pe2.user_id = l.user_id)) THEN
        INSERT INTO ent_msg_reprise_rapport (id, etablissement_id, fil_id, regle, effet)
        VALUES (gen_random_uuid(), f.etablissement_id, f.id, 'MIG-3', 'sans_participant_famille');
    END IF;

    UPDATE ent_msg_fil SET reprise_m1_le = now() WHERE id = f.id;
END
$$;

CREATE OR REPLACE FUNCTION ent_msg_m1_reprise() RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE
    r RECORD;
    n INTEGER := 0;
BEGIN
    FOR r IN SELECT f.id FROM ent_msg_fil f
              WHERE f.reprise_m1_le IS NULL
                AND NOT EXISTS (SELECT 1 FROM ent_msg_participant p WHERE p.fil_id = f.id)
              ORDER BY f.created_at, f.id LOOP
        PERFORM ent_msg_m1_reprise_fil(r.id);
        n := n + 1;
    END LOOP;
    RETURN n;
END
$$;

SELECT ent_msg_m1_reprise();
