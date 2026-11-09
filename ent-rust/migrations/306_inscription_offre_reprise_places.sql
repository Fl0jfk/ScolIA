-- Lot I2 (socle 3, §8.2, §16 M3-M4) — reprise des niveaux de campagne en offres, vœux des
-- dossiers existants, et compteur de places.
--
-- Idempotent. Ce que la reprise ne sait pas rapprocher est rangé dans
-- `ent_inscription_reprise_rapport` (jamais d'échec, jamais de perte).
--
-- **Compteur de places** (vue `ent_inscription_offre_occupation`) : calculé à chaque lecture à
-- partir des faits, jamais saisi. Un dossier ne compte qu'**une fois** : par son vœu retenu.
--   * `proposees`     = dossiers à l'état `admis` (en attente de la réponse de la famille) ;
--   * `confirmees`    = `confirme` (admission) ou `confirmee`, `en_attente_passage` (réinscription) ;
--   * `validees`      = `validee` ;
--   * `liste_attente` = `liste_attente` (ne consomme pas de place) ;
--   * `reserve_reinscription` = réinscrits dont le vœu retenu porte sur l'offre **jumelle** (même
--     niveau, parcours et régime) d'une campagne de réinscription de la même année scolaire
--     (`confirmee`, `en_attente_passage`, `validee`, et `a_confirmer` si le mode est `tacite`) ;
--   * `disponibles`   = capacité − réserve − proposées − confirmées − validées (négatif = dépassement
--     assumé, §7.1) ; NULL si la capacité n'est pas limitée.
-- Un dossier bloqué pour désaccord garde la place qu'il avait (état d'avant le blocage).

-- 1. Les niveaux de campagne deviennent des offres sans parcours.
INSERT INTO ent_inscription_offre
    (id, etablissement_id, campagne_id, niveau_code, niveau_id, parcours_code, libelle_public, capacite)
SELECT gen_random_uuid(), n.etablissement_id, n.campagne_id, n.code,
       (SELECT nv.id FROM ent_niveau nv
         WHERE nv.etablissement_id = n.etablissement_id AND upper(nv.code) = upper(btrim(n.code))),
       '', COALESCE((SELECT nv.libelle FROM ent_niveau nv
                      WHERE nv.etablissement_id = n.etablissement_id AND upper(nv.code) = upper(btrim(n.code))),
                    n.code),
       n.capacite
  FROM ent_inscription_campagne_niveau n
 WHERE NOT EXISTS (
        SELECT 1 FROM ent_inscription_offre o
         WHERE o.campagne_id = n.campagne_id AND upper(o.niveau_code) = upper(n.code)
           AND o.parcours_code = '' AND o.regime IS NULL AND o.archivee_le IS NULL);

INSERT INTO ent_inscription_reprise_rapport (etablissement_id, objet, objet_id, cause)
SELECT o.etablissement_id, 'offre', o.id, 'niveau_absent_du_referentiel'
  FROM ent_inscription_offre o
 WHERE o.niveau_id IS NULL
ON CONFLICT DO NOTHING;

-- 2. Vœu n°1 de chaque dossier qui nomme un niveau proposé par sa campagne (offre sans parcours).
INSERT INTO ent_inscription_voeu (id, etablissement_id, dossier_id, rang, offre_id, statut)
SELECT gen_random_uuid(), d.etablissement_id, d.id, 1, o.id,
       CASE WHEN d.etat IN ('liste_attente', 'admis', 'confirme', 'confirmee', 'en_attente_passage', 'validee')
            THEN 'retenu' ELSE 'demande' END
  FROM ent_inscription_dossier d
  JOIN ent_inscription_offre o
    ON o.etablissement_id = d.etablissement_id AND o.campagne_id = d.campagne_id
   AND upper(o.niveau_code) = upper(btrim(d.niveau_demande))
   AND o.parcours_code = '' AND o.regime IS NULL AND o.archivee_le IS NULL
 WHERE d.niveau_demande IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM ent_inscription_voeu v WHERE v.dossier_id = d.id);

INSERT INTO ent_inscription_reprise_rapport (etablissement_id, objet, objet_id, cause)
SELECT d.etablissement_id, 'dossier', d.id, 'niveau_sans_offre'
  FROM ent_inscription_dossier d
 WHERE d.niveau_demande IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM ent_inscription_voeu v WHERE v.dossier_id = d.id)
   AND EXISTS (SELECT 1 FROM ent_inscription_offre o WHERE o.campagne_id = d.campagne_id)
ON CONFLICT DO NOTHING;

UPDATE ent_inscription_dossier d
   SET voeu_retenu_id = v.id
  FROM ent_inscription_voeu v
 WHERE v.dossier_id = d.id AND v.rang = 1 AND v.statut = 'retenu' AND d.voeu_retenu_id IS NULL;

-- 3. Compteur de places.
CREATE OR REPLACE FUNCTION ent_inscription_etat_effectif(p_etat TEXT, p_avant TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE WHEN p_etat = 'bloque_desaccord' THEN COALESCE(p_avant, p_etat) ELSE p_etat END
$$;

CREATE OR REPLACE VIEW ent_inscription_offre_occupation AS
SELECT o.id AS offre_id,
       o.etablissement_id,
       o.campagne_id,
       o.capacite,
       (SELECT count(*) FROM ent_inscription_dossier d
          JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
         WHERE d.etablissement_id = o.etablissement_id AND v.offre_id = o.id
           AND ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage) = 'admis') AS proposees,
       (SELECT count(*) FROM ent_inscription_dossier d
          JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
         WHERE d.etablissement_id = o.etablissement_id AND v.offre_id = o.id
           AND ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage)
               IN ('confirme', 'confirmee', 'en_attente_passage')) AS confirmees,
       (SELECT count(*) FROM ent_inscription_dossier d
          JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
         WHERE d.etablissement_id = o.etablissement_id AND v.offre_id = o.id
           AND ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage) = 'validee') AS validees,
       (SELECT count(*) FROM ent_inscription_dossier d
          JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
         WHERE d.etablissement_id = o.etablissement_id AND v.offre_id = o.id
           AND ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage) = 'liste_attente') AS liste_attente,
       (SELECT count(DISTINCT d.id) FROM ent_inscription_dossier d
          JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.dossier_id = d.id
         WHERE d.etablissement_id = o.etablissement_id AND v.offre_id = o.id AND v.statut = 'demande'
           AND ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage)
               IN ('deposee', 'en_instruction', 'attente_famille')) AS demandes,
       CASE WHEN o.places_reservees_reinscription AND c.nature <> 'reinscription' THEN
            (SELECT count(*) FROM ent_inscription_dossier d
               JOIN ent_inscription_campagne cd
                 ON cd.etablissement_id = d.etablissement_id AND cd.id = d.campagne_id
               JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
               JOIN ent_inscription_offre o2 ON o2.etablissement_id = v.etablissement_id AND o2.id = v.offre_id
              WHERE d.etablissement_id = o.etablissement_id
                AND d.nature = 'reinscription'
                AND cd.annee_cible = c.annee_cible
                AND cd.site_id IS NOT DISTINCT FROM c.site_id
                AND upper(o2.niveau_code) = upper(o.niveau_code)
                AND o2.parcours_code = o.parcours_code
                AND COALESCE(o2.regime, '') = COALESCE(o.regime, '')
                AND (ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage)
                       IN ('confirmee', 'en_attente_passage', 'validee')
                     OR (ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage) = 'a_confirmer'
                         AND cd.mode_reinscription = 'tacite')))
            ELSE 0 END AS reserve_reinscription
  FROM ent_inscription_offre o
  JOIN ent_inscription_campagne c ON c.etablissement_id = o.etablissement_id AND c.id = o.campagne_id
 WHERE o.archivee_le IS NULL;
