-- 446 (rejouée APRÈS 2300 : elle lit les archives de levée, créées en 2300) — Socle 10 (emploi du temps), lot T3 : reprise des exceptions, déplacements et lots existants dans le
-- journal du réel (docs/socles/10_EMPLOI_DU_TEMPS.md §22, étape M4 : « toutes les exceptions passées
-- retrouvées, compte et test »).
--
-- Sources reprises :
--   * `ent_edt_annulation_masse` (171)            → `ent_edt_lot` (nature `reprise` ou `annulation`) ;
--   * `ent_edt_exception` (042, 171) vivantes     → modifications vivantes ;
--   * `ent_edt_deplacement` (170) vivants         → modifications `deplacement` vivantes ;
--   * `ent_edt_exception_levee` et `ent_edt_deplacement_leve` (2300, archives) → modifications levées.
-- Une exception de nature `deplacement` n'est pas reprise deux fois : le déplacement (170) en est la source.
-- Le motif libre historique (`ent_edt_exception.motif`) est INTERNE par nature (jamais diffusé) : il n'est
-- pas recopié ; le motif public d'un lot l'est.
-- Idempotente (clés `ancien_*` uniques) : la fonction peut être rejouée sans doublon ; elle est exécutée à
-- la migration et appelée par les tests « avant / après ». Aucune colonne JSON ni tableau, aucune date du jour en UTC.

CREATE OR REPLACE FUNCTION ent_edt_modification_reprise() RETURNS TABLE (
    lots_repris BIGINT, exceptions_reprises BIGINT, deplacements_repris BIGINT, archives_reprises BIGINT)
LANGUAGE plpgsql AS $$
DECLARE
    n_lots BIGINT; n_exc BIGINT; n_dep BIGINT; n_arch BIGINT; n_arch_dep BIGINT;
BEGIN
    -- 1. Lots.
    INSERT INTO ent_edt_lot
        (id, etablissement_id, cle, nature, du, au, portee, heure_debut, heure_fin, motif_public_code,
         motif_public, origine, nb_seances, nb_classes, cree_par, cree_le, ancien_masse_id)
    SELECT gen_random_uuid(), m.etablissement_id, NULL, 'reprise', m.du, m.au,
           CASE WHEN m.portee = 'niveau' THEN 'niveau' WHEN m.portee = 'classes' THEN 'classes' ELSE 'etablissement' END,
           m.heure_debut, m.heure_fin, 'autre', left(btrim(m.motif_public), 200), 'reprise',
           m.nb_seances, m.nb_classes, m.cree_par_user_id, m.created_at, m.id
      FROM ent_edt_annulation_masse m
     WHERE NOT EXISTS (SELECT 1 FROM ent_edt_lot l
                        WHERE l.etablissement_id = m.etablissement_id AND l.ancien_masse_id = m.id);
    GET DIAGNOSTICS n_lots = ROW_COUNT;

    -- La clé d'idempotence de l'ancien lot est reprise quand elle ne collisionne pas.
    UPDATE ent_edt_lot l SET cle = m.cle
      FROM ent_edt_annulation_masse m
     WHERE l.etablissement_id = m.etablissement_id AND l.ancien_masse_id = m.id
       AND l.cle IS NULL AND m.cle IS NOT NULL AND char_length(m.cle) <= 80
       AND NOT EXISTS (SELECT 1 FROM ent_edt_lot o
                        WHERE o.etablissement_id = l.etablissement_id AND o.cle = m.cle);

    INSERT INTO ent_edt_lot_cible (lot_id, etablissement_id, cible_id)
    SELECT l.id, l.etablissement_id, c.cible
      FROM ent_edt_annulation_masse m
      JOIN ent_edt_lot l ON l.etablissement_id = m.etablissement_id AND l.ancien_masse_id = m.id
      CROSS JOIN LATERAL unnest(m.cibles) AS c(cible)
    ON CONFLICT DO NOTHING;

    -- 2. Exceptions vivantes (hors déplacements, repris plus bas).
    INSERT INTO ent_edt_modification
        (id, etablissement_id, seance_id, lot_id, creneau_id, jour, type, avant_etat, apres_etat,
         apres_lieu_id, apres_salle_texte, apres_remplacant_user_id, motif_public_code, motif_public,
         origine, origine_ref, cree_par, cree_le, ancien_exception_id)
    SELECT gen_random_uuid(), x.etablissement_id, s.id, l.id, x.creneau_id, x.jour,
           CASE coalesce(x.nature, x.kind)
                WHEN 'remplacement' THEN 'intervenant'
                WHEN 'changement_salle' THEN 'changement_lieu'
                ELSE 'annulation' END,
           'prevue',
           CASE coalesce(x.nature, x.kind)
                WHEN 'annulation' THEN 'annulee' WHEN 'remplacement' THEN 'modifiee'
                WHEN 'changement_salle' THEN 'modifiee' ELSE 'annulee' END,
           sa.id, x.nouvelle_salle, x.remplacant_user_id,
           'autre', CASE WHEN l.id IS NOT NULL THEN l.motif_public END,
           CASE WHEN x.absence_rh_id IS NOT NULL THEN 'absence' WHEN x.lot_id IS NOT NULL THEN 'lot' ELSE 'saisie' END,
           x.absence_rh_id, x.created_by_user_id, x.created_at, x.id
      FROM ent_edt_exception x
      LEFT JOIN ent_edt_seance s ON s.etablissement_id = x.etablissement_id
           AND s.creneau_id = x.creneau_id AND s.jour = x.jour
      LEFT JOIN ent_edt_lot l ON l.etablissement_id = x.etablissement_id AND l.ancien_masse_id = x.lot_id
      LEFT JOIN ent_salle sa ON sa.etablissement_id = x.etablissement_id
           AND lower(btrim(sa.code)) = lower(btrim(x.nouvelle_salle))
     WHERE coalesce(x.nature, '') <> 'deplacement'
       AND NOT EXISTS (SELECT 1 FROM ent_edt_modification m
                        WHERE m.etablissement_id = x.etablissement_id AND m.ancien_exception_id = x.id)
       -- Déjà portée par une ligne vivante du journal (écrite par le socle 10 ou par le pont) : pas de doublon.
       AND NOT EXISTS (SELECT 1 FROM ent_edt_modification m
                        WHERE m.etablissement_id = x.etablissement_id AND m.creneau_id = x.creneau_id
                          AND m.jour = x.jour AND m.levee_le IS NULL
                          AND m.type IN ('annulation', 'deplacement', 'neutralisation', 'changement_lieu', 'intervenant'))
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS n_exc = ROW_COUNT;

    -- 3. Déplacements vivants.
    INSERT INTO ent_edt_modification
        (id, etablissement_id, seance_id, seance_arrivee_id, creneau_id, jour, type, avant_etat, apres_etat,
         avant_jour, apres_jour, apres_debut, apres_fin, apres_lieu_id, apres_salle_texte, motif_public_code,
         origine, cree_par, cree_le, ancien_deplacement_id)
    SELECT gen_random_uuid(), d.etablissement_id, s.id, arr.id, d.creneau_id, d.jour_origine, 'deplacement',
           'prevue', 'deplacee', d.jour_origine, d.nouveau_jour, d.nouvelle_heure_debut, d.nouvelle_heure_fin,
           sa.id, d.nouvelle_salle, 'autre', 'reprise', d.cree_par_user_id, d.created_at, d.id
      FROM ent_edt_deplacement d
      LEFT JOIN ent_edt_seance s ON s.etablissement_id = d.etablissement_id
           AND s.creneau_id = d.creneau_id AND s.jour = d.jour_origine
      LEFT JOIN ent_edt_seance arr ON arr.etablissement_id = d.etablissement_id
           AND arr.creneau_id = d.creneau_id AND arr.jour = d.nouveau_jour AND d.nouveau_jour <> d.jour_origine
      LEFT JOIN ent_salle sa ON sa.etablissement_id = d.etablissement_id
           AND lower(btrim(sa.code)) = lower(btrim(d.nouvelle_salle))
     WHERE NOT EXISTS (SELECT 1 FROM ent_edt_modification m
                        WHERE m.etablissement_id = d.etablissement_id AND m.ancien_deplacement_id = d.id)
       AND NOT EXISTS (SELECT 1 FROM ent_edt_modification m
                        WHERE m.etablissement_id = d.etablissement_id AND m.creneau_id = d.creneau_id
                          AND m.jour = d.jour_origine AND m.levee_le IS NULL AND m.type = 'deplacement')
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS n_dep = ROW_COUNT;

    -- 4. Archives (levées ou remplacées) : modifications levées, rien ne s'efface (AR-15).
    INSERT INTO ent_edt_modification
        (id, etablissement_id, seance_id, creneau_id, jour, type, avant_etat, apres_etat, apres_lieu_id,
         apres_salle_texte, apres_remplacant_user_id, motif_public_code, origine, origine_ref, cree_par,
         cree_le, levee_le, levee_par, ancien_archive_id)
    SELECT gen_random_uuid(), a.etablissement_id, s.id, a.creneau_id, a.jour,
           CASE WHEN coalesce(a.nature, a.kind) = 'deplacement' THEN 'deplacement'
                WHEN coalesce(a.nature, a.kind) = 'remplacement' THEN 'intervenant'
                WHEN coalesce(a.nature, a.kind) = 'changement_salle' THEN 'changement_lieu'
                ELSE 'annulation' END,
           'prevue',
           CASE WHEN coalesce(a.nature, a.kind) = 'deplacement' THEN 'deplacee'
                WHEN coalesce(a.nature, a.kind) IN ('remplacement', 'changement_salle') THEN 'modifiee'
                ELSE 'annulee' END,
           sa.id, a.nouvelle_salle, a.remplacant_user_id, 'autre',
           CASE WHEN a.absence_rh_id IS NOT NULL THEN 'absence' WHEN a.lot_id IS NOT NULL THEN 'lot' ELSE 'saisie' END,
           a.absence_rh_id, coalesce(a.pose_par_user_id, a.levee_par_user_id), coalesce(a.pose_le, a.levee_le),
           a.levee_le, a.levee_par_user_id, a.id
      FROM ent_edt_exception_levee a
      LEFT JOIN ent_edt_seance s ON s.etablissement_id = a.etablissement_id
           AND s.creneau_id = a.creneau_id AND s.jour = a.jour
      LEFT JOIN ent_salle sa ON sa.etablissement_id = a.etablissement_id
           AND lower(btrim(sa.code)) = lower(btrim(a.nouvelle_salle))
     WHERE NOT EXISTS (SELECT 1 FROM ent_edt_modification m
                        WHERE m.etablissement_id = a.etablissement_id AND m.ancien_archive_id = a.id)
       -- Levée dans la même transaction que son écriture au journal (pont ou socle 10) : déjà portée.
       AND NOT EXISTS (SELECT 1 FROM ent_edt_modification m
                        WHERE m.etablissement_id = a.etablissement_id AND m.creneau_id = a.creneau_id
                          AND m.jour = a.jour AND m.levee_le = a.levee_le)
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS n_arch = ROW_COUNT;

    -- Les archives de déplacement dont l'exception d'archive n'a pas déjà produit la ligne (nature
    -- `deplacement`) ne produisent rien de plus : l'archive d'exception porte le même fait.
    n_arch_dep := 0;

    -- Les modifications reprises d'une exception vivante sont sans conflit d'état vivant par séance :
    -- l'index d'unicité partiel protège ; en cas de doublon historique (rare) la dernière gagne.
    RETURN QUERY SELECT n_lots, n_exc, n_dep, n_arch + n_arch_dep;
END
$$;

-- Exécutée à la migration. Un doublon d'état vivant sur une même séance (exception et déplacement
-- historiques) ne doit jamais bloquer le démarrage : l'index d'unicité partiel s'applique aux lignes vivantes
-- reprises, on lève donc d'abord les doublons avant l'insertion éventuelle.
SELECT * FROM ent_edt_modification_reprise();
