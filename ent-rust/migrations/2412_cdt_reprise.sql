-- Socle 20, lot C1 — migration 2412 : REPRISE de l'existant (étape M2 du §25) dans les tables 2401–2411, quarantaine, vues de
-- compatibilité. Réf. : 20_CAHIER_DE_TEXTES.md §25. Rejouable (rejouée à chaque démarrage) : chaque ligne reprise garde son
-- identifiant d'origine (`ancien_id`, unique) ; rien n'est jamais repris deux fois, rien n'est modifié ni supprimé dans les
-- tables d'origine (150), qui restent les tables d'écriture jusqu'à la bascule (C2).
--
-- Ce qui est repris :
--   ent_seance_contenu → ent_cdt_entree (cours déduit de la séance : service puis créneau ; sinon QUARANTAINE) + versions ;
--   ent_travail → ent_cdt_travail (cours de la séance, sinon cours unique de la cible dans la matière ; sinon QUARANTAINE) ;
--     type 'controle' → nature « révision » + ancien_type = 'controle' ;
--   ent_travail_remise → ent_cdt_remise + dépôt n° 1 (empreinte = SHA-256 de la liste des empreintes des fichiers) + retour ;
--   ent_cdt_piece → ent_cdt_piece_jointe (par l'objet du registre ; sinon QUARANTAINE) ;
--   ent_cdt_historique → versions (instantané relu une fois, puis gardé figé) ;
--   ent_travail_eleve (fait) → ent_cdt_coche. Le « vu » n'est pas repris (R-C42) ; `vu_le` reste lu par les avis jusqu'à C3.
-- Les visas restent dans `ent_cdt_visa` (élargie en 2410) : aucun visa n'est copié.

-- Quarantaine : ce que la reprise n'a pas pu rattacher ; visible de la direction, décidé avant la bascule des écritures (M3).
CREATE TABLE IF NOT EXISTS ent_cdt_quarantaine (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (objet IN ('seance_contenu', 'travail', 'piece')),
    ancien_id UUID NOT NULL,
    raison TEXT NOT NULL CHECK (raison IN ('cours_introuvable', 'objet_absent')),
    detail TEXT NOT NULL CHECK (length(detail) <= 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    traitee_le TIMESTAMPTZ,
    traitee_par_user_id UUID,
    CONSTRAINT ent_cdt_quarantaine_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_quarantaine_ancien_uq UNIQUE (etablissement_id, objet, ancien_id),
    CONSTRAINT ent_cdt_quarantaine_traitee_chk CHECK ((traitee_le IS NULL) = (traitee_par_user_id IS NULL))
);
SELECT ent_rls_poser(ARRAY['ent_cdt_quarantaine']);

-- Cours d'un travail de l'ancien modèle : celui de sa séance ; à défaut, le cours OUVERT à la date de la cible (classe ou
-- groupe) dont le libellé ou la matière est celle du travail, à condition qu'il soit unique.
CREATE OR REPLACE FUNCTION ent_cdt_cours_de_travail(
    p_etab UUID, p_seance UUID, p_classe UUID, p_groupe UUID, p_matiere TEXT, p_jour DATE)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT COALESCE(
        ent_cdt_cours_de_seance(p_etab, p_seance),
        (SELECT (array_agg(c.id))[1]
           FROM ent_cours c
           LEFT JOIN ent_matiere m ON m.etablissement_id = c.etablissement_id AND m.id = c.matiere_id
          WHERE c.etablissement_id = p_etab
            AND ((p_classe IS NOT NULL AND c.classe_id = p_classe) OR (p_groupe IS NOT NULL AND c.groupe_id = p_groupe))
            AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
            AND (lower(btrim(c.libelle)) = lower(btrim(p_matiere)) OR lower(btrim(m.libelle)) = lower(btrim(p_matiere)))
         HAVING count(*) = 1))
$$;

-- Agent du compte (un compte sans agent : l'auteur reste connu par `auteur_user_id`).
CREATE OR REPLACE FUNCTION ent_cdt_agent_du_compte(p_etab UUID, p_user UUID) RETURNS UUID
LANGUAGE sql STABLE AS $$
    SELECT a.id FROM ent_agent a
      JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
     WHERE a.etablissement_id = p_etab AND p.user_id = p_user
     ORDER BY a.id LIMIT 1
$$;

-- Compteurs de reprise, en LECTURE SEULE : chaque ancien objet est repris ou en quarantaine, jamais perdu.
CREATE OR REPLACE FUNCTION ent_cdt_reprise_compteurs(p_etab UUID DEFAULT NULL)
RETURNS TABLE (objet TEXT, anciens BIGINT, repris BIGINT, quarantaine BIGINT)
LANGUAGE sql STABLE AS $$
    SELECT 'seance_contenu'::text,
           (SELECT count(*) FROM ent_seance_contenu c WHERE p_etab IS NULL OR c.etablissement_id = p_etab),
           (SELECT count(*) FROM ent_cdt_entree e WHERE e.ancien_id IS NOT NULL AND (p_etab IS NULL OR e.etablissement_id = p_etab)),
           (SELECT count(*) FROM ent_cdt_quarantaine q WHERE q.objet = 'seance_contenu' AND (p_etab IS NULL OR q.etablissement_id = p_etab))
    UNION ALL
    SELECT 'travail',
           (SELECT count(*) FROM ent_travail t WHERE p_etab IS NULL OR t.etablissement_id = p_etab),
           (SELECT count(*) FROM ent_cdt_travail n WHERE n.ancien_id IS NOT NULL AND (p_etab IS NULL OR n.etablissement_id = p_etab)),
           (SELECT count(*) FROM ent_cdt_quarantaine q WHERE q.objet = 'travail' AND (p_etab IS NULL OR q.etablissement_id = p_etab))
    UNION ALL
    SELECT 'remise',
           (SELECT count(*) FROM ent_travail_remise r WHERE p_etab IS NULL OR r.etablissement_id = p_etab),
           (SELECT count(*) FROM ent_cdt_remise x WHERE x.ancien_id IS NOT NULL AND (p_etab IS NULL OR x.etablissement_id = p_etab)),
           (SELECT count(*) FROM ent_travail_remise r WHERE (p_etab IS NULL OR r.etablissement_id = p_etab)
               AND NOT EXISTS (SELECT 1 FROM ent_cdt_remise x WHERE x.ancien_id = r.id))
    UNION ALL
    SELECT 'piece',
           (SELECT count(*) FROM ent_cdt_piece p WHERE p_etab IS NULL OR p.etablissement_id = p_etab),
           (SELECT count(*) FROM ent_cdt_piece_jointe j WHERE j.ancien_id IS NOT NULL AND (p_etab IS NULL OR j.etablissement_id = p_etab)),
           (SELECT count(*) FROM ent_cdt_piece p WHERE (p_etab IS NULL OR p.etablissement_id = p_etab)
               AND NOT EXISTS (SELECT 1 FROM ent_cdt_piece_jointe j WHERE j.ancien_id = p.id))
$$;

-- La reprise. `p_etab` NULL : tous les établissements. Renvoie les compteurs avant / après (objet, anciens, repris, quarantaine).
CREATE OR REPLACE FUNCTION ent_cdt_reprise(p_etab UUID DEFAULT NULL)
RETURNS TABLE (objet TEXT, anciens BIGINT, repris BIGINT, quarantaine BIGINT)
LANGUAGE plpgsql AS $$
#variable_conflict use_column
BEGIN
    PERFORM ent_cdt_assurer_natures(x.etablissement_id)
       FROM (SELECT DISTINCT etablissement_id FROM ent_travail WHERE p_etab IS NULL OR etablissement_id = p_etab) x;

    -- 1. Contenus de séance -------------------------------------------------------------------------------------------
    INSERT INTO ent_cdt_quarantaine (id, etablissement_id, objet, ancien_id, raison, detail)
    SELECT gen_random_uuid(), c.etablissement_id, 'seance_contenu', c.id, 'cours_introuvable',
           'séance sans service ni créneau rattaché à un cours'
      FROM ent_seance_contenu c
     WHERE (p_etab IS NULL OR c.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.ancien_id = c.id)
       AND ent_cdt_cours_de_seance(c.etablissement_id, c.seance_id) IS NULL
    ON CONFLICT (etablissement_id, objet, ancien_id) DO NOTHING;

    INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, contenu, etat, publie_a, visible_a,
                                auteur_user_id, auteur_agent_id, version, renseignee_le, ancien_id, cree_le, modifie_le)
    SELECT gen_random_uuid(), c.etablissement_id, ent_cdt_cours_de_seance(c.etablissement_id, c.seance_id), c.seance_id,
           s.annee_id,
           c.contenu || CASE WHEN btrim(c.ressources) <> '' THEN E'\n\nRessources : ' || c.ressources ELSE '' END,
           'publiee', c.created_at, c.created_at, c.auteur_user_id,
           ent_cdt_agent_du_compte(c.etablissement_id, c.auteur_user_id),
           c.version, c.updated_at, c.id, c.created_at, c.updated_at
      FROM ent_seance_contenu c
      JOIN ent_edt_seance s ON s.etablissement_id = c.etablissement_id AND s.id = c.seance_id
     WHERE (p_etab IS NULL OR c.etablissement_id = p_etab)
       AND ent_cdt_cours_de_seance(c.etablissement_id, c.seance_id) IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.ancien_id = c.id)
    ON CONFLICT DO NOTHING;

    -- 2. Travaux ---------------------------------------------------------------------------------------------------------
    INSERT INTO ent_cdt_quarantaine (id, etablissement_id, objet, ancien_id, raison, detail)
    SELECT gen_random_uuid(), t.etablissement_id, 'travail', t.id, 'cours_introuvable',
           'aucun cours (ou plusieurs) pour la cible et la matière « ' || left(t.matiere_libelle, 120) || ' »'
      FROM ent_travail t
     WHERE (p_etab IS NULL OR t.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_travail n WHERE n.ancien_id = t.id)
       AND ent_cdt_cours_de_travail(t.etablissement_id, t.seance_id, t.classe_id, t.groupe_id, t.matiere_libelle, t.donne_le) IS NULL
    ON CONFLICT (etablissement_id, objet, ancien_id) DO NOTHING;

    INSERT INTO ent_cdt_travail (id, etablissement_id, cours_id, annee_id, seance_donnee_id, donne_le, echeance_mode,
                                 echeance_le, nature_id, titre, consigne, duree_estimee_min, rendu, retard_accepte, public,
                                 etat, publie_a, visible_a, retire_motif, retire_par_user_id, auteur_user_id, auteur_agent_id,
                                 version, ancien_id, ancien_type, cree_le, modifie_le)
    SELECT gen_random_uuid(), t.etablissement_id, cr.cours_id, co.annee_id, t.seance_id, t.donne_le, 'date_heure',
           COALESCE(t.date_limite, ent_cdt_fin_de_journee(t.etablissement_id, t.pour_le)),
           (SELECT na.id FROM ent_cdt_nature na WHERE na.etablissement_id = t.etablissement_id
               AND na.code = CASE t.type WHEN 'a_rendre' THEN 'dm' WHEN 'controle' THEN 'revision' ELSE 'travail' END),
           t.titre, t.consigne, t.duree_estimee_min,
           CASE WHEN t.rendu_en_ligne THEN 'en_ligne' ELSE 'aucun' END, t.acceptation_retard, 'cours',
           CASE WHEN t.supprime_le IS NULL THEN 'publie' ELSE 'retire' END, t.created_at, t.created_at,
           CASE WHEN t.supprime_le IS NULL THEN NULL ELSE COALESCE(t.retire_motif, 'supprimé par son auteur') END,
           CASE WHEN t.supprime_le IS NULL THEN NULL ELSE COALESCE(t.retire_par_user_id, t.auteur_user_id) END,
           t.auteur_user_id, ent_cdt_agent_du_compte(t.etablissement_id, t.auteur_user_id),
           t.version, t.id, CASE WHEN t.type = 'controle' THEN 'controle' END, t.created_at, t.updated_at
      FROM ent_travail t
      CROSS JOIN LATERAL (
            SELECT ent_cdt_cours_de_travail(t.etablissement_id, t.seance_id, t.classe_id, t.groupe_id, t.matiere_libelle, t.donne_le) AS cours_id) cr
      JOIN ent_cours co ON co.etablissement_id = t.etablissement_id AND co.id = cr.cours_id
     WHERE (p_etab IS NULL OR t.etablissement_id = p_etab)
       AND cr.cours_id IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_travail n WHERE n.ancien_id = t.id)
    ON CONFLICT DO NOTHING;

    INSERT INTO ent_cdt_travail_format (etablissement_id, travail_id, format_code)
    SELECT n.etablissement_id, n.id, f
      FROM ent_travail t
      JOIN ent_cdt_travail n ON n.ancien_id = t.id
      CROSS JOIN LATERAL unnest(t.formats_acceptes) AS f
     WHERE (p_etab IS NULL OR t.etablissement_id = p_etab) AND f ~ '^[a-z0-9]{1,10}$'
    ON CONFLICT DO NOTHING;

    -- 3. Versions : historique (instantané relu une fois, gardé figé) + état courant -----------------------------------------
    INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, titre, contenu, etat, auteur_user_id, motif,
                                        ancien_historique_id, cree_le)
    SELECT gen_random_uuid(), h.etablissement_id, e.id, h.version, NULL, COALESCE(h.instantane->>'contenu', ''), 'publiee',
           NULLIF(h.instantane->>'auteur_user_id', '')::uuid, h.action, h.id, h.created_at
      FROM ent_cdt_historique h
      JOIN ent_cdt_entree e ON e.etablissement_id = h.etablissement_id AND e.ancien_id = h.objet_id
     WHERE h.objet = 'seance_contenu' AND (p_etab IS NULL OR h.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_entree_version v WHERE v.ancien_historique_id = h.id)
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, titre, contenu, etat, auteur_user_id,
                                        auteur_agent_id, motif, cree_le)
    SELECT gen_random_uuid(), e.etablissement_id, e.id, e.version, e.titre, e.contenu, e.etat, e.auteur_user_id,
           e.auteur_agent_id, 'reprise', e.modifie_le
      FROM ent_cdt_entree e
     WHERE e.ancien_id IS NOT NULL AND (p_etab IS NULL OR e.etablissement_id = p_etab)
    ON CONFLICT DO NOTHING;

    INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne, duree_estimee_min,
                                         echeance_le, etat, auteur_user_id, motif, ancien_historique_id, cree_le)
    SELECT gen_random_uuid(), h.etablissement_id, n.id, h.version, COALESCE(h.instantane->>'titre', n.titre),
           COALESCE(h.instantane->>'consigne', n.consigne), NULLIF(h.instantane->>'duree_estimee_min', '')::smallint,
           COALESCE(NULLIF(h.instantane->>'date_limite', '')::timestamptz,
                    ent_cdt_fin_de_journee(h.etablissement_id, NULLIF(h.instantane->>'pour_le', '')::date), n.echeance_le),
           'publie', NULLIF(h.instantane->>'auteur_user_id', '')::uuid, h.action, h.id, h.created_at
      FROM ent_cdt_historique h
      JOIN ent_cdt_travail n ON n.etablissement_id = h.etablissement_id AND n.ancien_id = h.objet_id
     WHERE h.objet = 'travail' AND (p_etab IS NULL OR h.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_travail_version v WHERE v.ancien_historique_id = h.id)
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne, duree_estimee_min,
                                         echeance_le, etat, auteur_user_id, auteur_agent_id, motif, cree_le)
    SELECT gen_random_uuid(), n.etablissement_id, n.id, n.version, n.titre, n.consigne, n.duree_estimee_min, n.echeance_le,
           n.etat, n.auteur_user_id, n.auteur_agent_id, 'reprise', n.modifie_le
      FROM ent_cdt_travail n
     WHERE n.ancien_id IS NOT NULL AND (p_etab IS NULL OR n.etablissement_id = p_etab)
    ON CONFLICT DO NOTHING;

    -- 4. Remises, dépôt n° 1, retours --------------------------------------------------------------------------------------
    INSERT INTO ent_cdt_remise (id, etablissement_id, travail_id, eleve_id, etat, en_retard, version, ancien_id, cree_le, modifie_le)
    SELECT gen_random_uuid(), r.etablissement_id, n.id, r.eleve_id, 'deposee', r.en_retard, 1, r.id, r.soumise_le, r.derniere_le
      FROM ent_travail_remise r
      JOIN ent_cdt_travail n ON n.etablissement_id = r.etablissement_id AND n.ancien_id = r.travail_id
     WHERE (p_etab IS NULL OR r.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_remise x WHERE x.ancien_id = r.id)
    ON CONFLICT DO NOTHING;

    INSERT INTO ent_cdt_depot (id, etablissement_id, remise_id, numero, recu_le, par_user_id, empreinte, nb_fichiers,
                               commentaire_eleve, cree_le)
    SELECT gen_random_uuid(), x.etablissement_id, x.id, 1, r.soumise_le,
           (SELECT ec.user_id FROM ent_eleve_compte ec WHERE ec.etablissement_id = r.etablissement_id AND ec.eleve_id = r.eleve_id LIMIT 1),
           encode(sha256(convert_to(COALESCE((
               SELECT string_agg(o.sha256, ',' ORDER BY p.created_at, p.id)
                 FROM ent_cdt_piece p
                 JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.cle_objet = p.storage_key
                WHERE p.etablissement_id = r.etablissement_id AND p.remise_id = r.id AND p.genre = 'copie'), ''), 'UTF8')), 'hex'),
           (SELECT count(*) FROM ent_cdt_piece p WHERE p.etablissement_id = r.etablissement_id AND p.remise_id = r.id
               AND p.genre = 'copie' AND p.retiree_le IS NULL)::smallint,
           NULLIF(r.commentaire_eleve, ''), r.soumise_le
      FROM ent_cdt_remise x
      JOIN ent_travail_remise r ON r.etablissement_id = x.etablissement_id AND r.id = x.ancien_id
     WHERE x.ancien_id IS NOT NULL AND (p_etab IS NULL OR x.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_depot d WHERE d.remise_id = x.id)
    ON CONFLICT DO NOTHING;
    UPDATE ent_cdt_remise x SET dernier_depot_id = d.id
      FROM ent_cdt_depot d
     WHERE d.remise_id = x.id AND d.numero = 1 AND x.dernier_depot_id IS NULL AND x.ancien_id IS NOT NULL
       AND (p_etab IS NULL OR x.etablissement_id = p_etab);

    INSERT INTO ent_cdt_retour (id, etablissement_id, remise_id, etat, commentaire, note_id, montrer_note, rendu_le,
                                auteur_user_id, version, ancien_id, cree_le, modifie_le)
    SELECT gen_random_uuid(), x.etablissement_id, x.id, 'rendu', r.retour_commentaire, r.note_id, r.note_id IS NOT NULL,
           COALESCE(r.retour_le, r.derniere_le), r.retour_par_user_id, 1, r.id, COALESCE(r.retour_le, r.derniere_le),
           COALESCE(r.retour_le, r.derniere_le)
      FROM ent_cdt_remise x
      JOIN ent_travail_remise r ON r.etablissement_id = x.etablissement_id AND r.id = x.ancien_id
     WHERE r.statut = 'corrige' AND r.retour_commentaire IS NOT NULL AND (p_etab IS NULL OR x.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_retour y WHERE y.remise_id = x.id)
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_cdt_retour_version (id, etablissement_id, retour_id, version, commentaire, note_id, rendu_le, auteur_user_id, cree_le)
    SELECT gen_random_uuid(), y.etablissement_id, y.id, y.version, y.commentaire, y.note_id, y.rendu_le, y.auteur_user_id, y.rendu_le
      FROM ent_cdt_retour y
     WHERE y.ancien_id IS NOT NULL AND (p_etab IS NULL OR y.etablissement_id = p_etab)
    ON CONFLICT DO NOTHING;

    -- 5. Pièces ------------------------------------------------------------------------------------------------------------
    INSERT INTO ent_cdt_quarantaine (id, etablissement_id, objet, ancien_id, raison, detail)
    SELECT gen_random_uuid(), p.etablissement_id, 'piece', p.id, 'objet_absent',
           'pièce sans objet au registre (empreinte à calculer par le socle 7)'
      FROM ent_cdt_piece p
     WHERE (p_etab IS NULL OR p.etablissement_id = p_etab)
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_piece_jointe j WHERE j.ancien_id = p.id)
       AND NOT EXISTS (SELECT 1 FROM ent_objet o WHERE o.etablissement_id = p.etablissement_id AND o.cle_objet = p.storage_key)
       AND (EXISTS (SELECT 1 FROM ent_cdt_travail n WHERE n.ancien_id = p.travail_id)
            OR EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.ancien_id = p.seance_contenu_id)
            OR EXISTS (SELECT 1 FROM ent_cdt_remise x WHERE x.ancien_id = p.remise_id))
    ON CONFLICT (etablissement_id, objet, ancien_id) DO NOTHING;

    INSERT INTO ent_cdt_piece_jointe (id, etablissement_id, objet_id, porteur, porteur_id, nom, ordre, retiree_le,
                                      retiree_par_user_id, ancien_id, cree_le)
    SELECT gen_random_uuid(), p.etablissement_id, o.id, c.porteur, c.porteur_id, p.nom,
           (row_number() OVER (PARTITION BY c.porteur, c.porteur_id ORDER BY p.created_at, p.id))::smallint,
           p.retiree_le, CASE WHEN p.retiree_le IS NULL THEN NULL ELSE COALESCE(p.retiree_par, p.created_by_user_id) END,
           p.id, p.created_at
      FROM ent_cdt_piece p
      JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.cle_objet = p.storage_key
      CROSS JOIN LATERAL (
            SELECT CASE p.genre WHEN 'ressource' THEN 'travail' WHEN 'seance' THEN 'entree'
                                WHEN 'copie' THEN 'depot' ELSE 'retour' END AS porteur,
                   CASE p.genre
                       WHEN 'ressource' THEN (SELECT n.id FROM ent_cdt_travail n WHERE n.etablissement_id = p.etablissement_id AND n.ancien_id = p.travail_id)
                       WHEN 'seance' THEN (SELECT e.id FROM ent_cdt_entree e WHERE e.etablissement_id = p.etablissement_id AND e.ancien_id = p.seance_contenu_id)
                       WHEN 'copie' THEN (SELECT d.id FROM ent_cdt_depot d JOIN ent_cdt_remise x ON x.etablissement_id = d.etablissement_id AND x.id = d.remise_id
                                           WHERE x.etablissement_id = p.etablissement_id AND x.ancien_id = p.remise_id AND d.numero = 1)
                       ELSE (SELECT y.id FROM ent_cdt_retour y JOIN ent_cdt_remise x ON x.etablissement_id = y.etablissement_id AND x.id = y.remise_id
                              WHERE x.etablissement_id = p.etablissement_id AND x.ancien_id = p.remise_id)
                   END AS porteur_id) c
     WHERE (p_etab IS NULL OR p.etablissement_id = p_etab) AND c.porteur_id IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM ent_cdt_piece_jointe j WHERE j.ancien_id = p.id)
    ON CONFLICT DO NOTHING;

    -- 6. Coches « fait » ----------------------------------------------------------------------------------------------------
    INSERT INTO ent_cdt_coche (etablissement_id, travail_id, eleve_id, fait_le)
    SELECT te.etablissement_id, n.id, te.eleve_id, te.fait_le
      FROM ent_travail_eleve te
      JOIN ent_cdt_travail n ON n.etablissement_id = te.etablissement_id AND n.ancien_id = te.travail_id
     WHERE te.fait AND te.fait_le IS NOT NULL AND (p_etab IS NULL OR te.etablissement_id = p_etab)
    ON CONFLICT DO NOTHING;

    -- Nettoyage : une ligne de quarantaine dont l'objet a fini par être repris n'a plus lieu d'être.
    DELETE FROM ent_cdt_quarantaine q
     WHERE q.traitee_le IS NULL AND (p_etab IS NULL OR q.etablissement_id = p_etab)
       AND ((q.objet = 'seance_contenu' AND EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.ancien_id = q.ancien_id))
         OR (q.objet = 'travail' AND EXISTS (SELECT 1 FROM ent_cdt_travail n WHERE n.ancien_id = q.ancien_id))
         OR (q.objet = 'piece' AND EXISTS (SELECT 1 FROM ent_cdt_piece_jointe j WHERE j.ancien_id = q.ancien_id)));

    RETURN QUERY SELECT * FROM ent_cdt_reprise_compteurs(p_etab);
END
$$;

SELECT count(*) FROM ent_cdt_reprise(NULL);

-- Vues de compatibilité : l'ancienne forme (colonnes de 150) lue dans les nouvelles tables, pour les lecteurs non encore
-- basculés. Les tables d'origine portant encore les noms `ent_travail` et `ent_seance_contenu` jusqu'à leur retrait (2419),
-- les vues portent le suffixe `_compat` ; le retrait les renomme. Elles s'exécutent avec les droits de l'appelant (RLS appliquée).
CREATE OR REPLACE VIEW ent_seance_contenu_compat WITH (security_invoker = true) AS
SELECT COALESCE(e.ancien_id, e.id) AS id, e.etablissement_id, e.seance_id,
       COALESCE(e.auteur_user_id, '00000000-0000-0000-0000-000000000000'::uuid) AS auteur_user_id,
       e.contenu, ''::text AS ressources, e.version, e.cree_le AS created_at, e.modifie_le AS updated_at
  FROM ent_cdt_entree e
 WHERE e.etat = 'publiee';

CREATE OR REPLACE VIEW ent_travail_compat WITH (security_invoker = true) AS
SELECT COALESCE(t.ancien_id, t.id) AS id, t.etablissement_id, t.seance_donnee_id AS seance_id,
       c.classe_id, c.groupe_id, c.libelle AS matiere_libelle,
       COALESCE(t.auteur_user_id, '00000000-0000-0000-0000-000000000000'::uuid) AS auteur_user_id,
       CASE WHEN t.ancien_type = 'controle' THEN 'controle' WHEN t.rendu = 'en_ligne' THEN 'a_rendre' ELSE 'a_faire' END AS type,
       t.titre, t.consigne, t.donne_le, ent_aujourdhui_etab(t.etablissement_id, t.echeance_le) AS pour_le,
       t.duree_estimee_min::integer AS duree_estimee_min, t.rendu = 'en_ligne' AS rendu_en_ligne,
       CASE WHEN t.rendu = 'en_ligne' THEN t.echeance_le END AS date_limite, t.retard_accepte AS acceptation_retard,
       t.version, CASE WHEN t.etat = 'retire' THEN t.modifie_le END AS supprime_le,
       t.cree_le AS created_at, t.modifie_le AS updated_at
  FROM ent_cdt_travail t
  JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id
 WHERE t.etat IN ('publie', 'retire');
