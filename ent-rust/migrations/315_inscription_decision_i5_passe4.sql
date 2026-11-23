-- Lot I5, passe 4 (socle 3, `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`, section « Passe 4 ») — **315**.
--
-- Additif et rejouable. 313 et 314 sont versées et **ne changent pas** (règle 25) : tout ce qui les corrige est ici.
--
-- Nouveau modèle des verrous (décision du pilote, passe 4) :
-- * le **rang d'attente n'est plus stocké** : il se calcule en lecture (`ent_inscription_rang_attente`), à partir de
--   `ordre_attente` (jamais renuméroté). La colonne `rang_attente` reste en **lecture seule** pour la reprise : un
--   déclencheur refuse toute modification ;
-- * **un verrou consultatif unique par offre**, pris en premier : sa clé est dérivée de l'identifiant de l'offre par
--   `ent_inscription_offre_cle` (source unique, utilisée par le code et par les tests) ;
-- * un vœu **créé par une décision** porte son origine : il ne sort de l'export qu'une fois la décision publiée.
--
-- Toute fonction ci-dessous porte `SET search_path = pg_catalog, public, pg_temp` (chantier transverse T1).

-- ───────────────────────────── Verrou consultatif par offre ─────────────────────────────
CREATE OR REPLACE FUNCTION ent_inscription_offre_cle(p_offre uuid) RETURNS bigint
LANGUAGE sql IMMUTABLE PARALLEL SAFE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT hashtextextended('ent_inscription_offre:' || p_offre::text, 0)
$$;

-- ───────────────────────────── Rang calculé en lecture ─────────────────────────────
-- **Seule** définition du rang d'attente. `p_famille = false` : le rang vu par le personnel, parmi les dossiers
-- réellement en liste. `p_famille = true` : le rang vu par la famille, parmi les dossiers dont l'entrée ou la sortie est
-- **publiée** (`ent_inscription_en_liste_visible`). L'ordre est celui de l'entrée sur la liste (`ordre_attente`, repli
-- sur l'ancien `rang_attente` repris), puis l'identifiant : un départ ne décale jamais personne dans la table.
CREATE OR REPLACE FUNCTION ent_inscription_rang_attente(p_etab uuid, p_dossier uuid, p_famille boolean) RETURNS bigint
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT r.pos
      FROM (SELECT x.id,
                   row_number() OVER (ORDER BY COALESCE(x.ordre_attente, x.rang_attente::bigint) NULLS LAST, x.id) AS pos
              FROM ent_inscription_dossier x
             WHERE x.etablissement_id = p_etab
               AND COALESCE(x.offre_attente_id,
                            (SELECT v.offre_id FROM ent_inscription_voeu v
                              WHERE v.etablissement_id = x.etablissement_id AND v.id = x.voeu_retenu_id))
                   = (SELECT COALESCE(m.offre_attente_id, v2.offre_id)
                        FROM ent_inscription_dossier m
                        LEFT JOIN ent_inscription_voeu v2
                               ON v2.etablissement_id = m.etablissement_id AND v2.id = m.voeu_retenu_id
                       WHERE m.etablissement_id = p_etab AND m.id = p_dossier)
               AND CASE WHEN p_famille THEN ent_inscription_en_liste_visible(x.etablissement_id, x.id, x.etat)
                        ELSE x.etat = 'liste_attente' END) r
     WHERE r.id = p_dossier
$$;

CREATE INDEX IF NOT EXISTS ent_inscription_dossier_attente_idx
    ON ent_inscription_dossier (etablissement_id, offre_attente_id, ordre_attente, id)
    WHERE ordre_attente IS NOT NULL;

-- ───────────────────────────── `rang_attente` : lecture seule ─────────────────────────────
CREATE OR REPLACE FUNCTION ent_inscription_rang_lecture_seule() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.rang_attente IS DISTINCT FROM OLD.rang_attente AND NOT ent_inscription_purge_voulue() THEN
        RAISE EXCEPTION 'rang_attente est en lecture seule : le rang se calcule (ent_inscription_rang_attente)'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_inscription_rang_lecture_seule_trg ON ent_inscription_dossier;
CREATE TRIGGER ent_inscription_rang_lecture_seule_trg
    BEFORE UPDATE OF rang_attente ON ent_inscription_dossier
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_rang_lecture_seule();

COMMENT ON COLUMN ent_inscription_dossier.rang_attente IS
    'Reprise seulement (lecture seule depuis 315) : le rang se calcule par ent_inscription_rang_attente.';

-- ───────────────────────────── Origine d'un vœu ─────────────────────────────
ALTER TABLE ent_inscription_voeu ADD COLUMN IF NOT EXISTS origine TEXT NOT NULL DEFAULT 'famille';
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_voeu_origine_chk') THEN
        ALTER TABLE ent_inscription_voeu
            ADD CONSTRAINT ent_inscription_voeu_origine_chk CHECK (origine IN ('famille', 'decision'));
    END IF;
END $$;
