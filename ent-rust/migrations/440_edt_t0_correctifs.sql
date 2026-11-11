-- 440 — Socle 10 (emploi du temps), lot T0 : correctifs urgents (docs/socles/10_EMPLOI_DU_TEMPS.md §2.2, §23.1).
--
-- D1 (grave) : la régénération des séances supprimait physiquement toute séance « plus attendue »
-- (fermeture ajoutée, jour férié, créneau déplacé), et le contenu du cahier de textes partait avec elle
-- (`ON DELETE CASCADE`) ; le travail à faire perdait sa séance (`ON DELETE SET NULL`).
-- Correction : une séance qui porte quelque chose n'est plus jamais supprimée, elle est **retirée**
-- (état `retiree`, date et motif) ; les clés étrangères deviennent `RESTRICT` (défense en profondeur :
-- un `DELETE` oublié dans le code échoue au lieu de détruire le cahier).
-- D4 (RLS permissive des tables EDT et `ent_salle`) : déjà posée par 587 sur `ent_edt_*`, `ent_salle`,
-- `ent_delegation` et par 581 sur `ent_absence_rh` ; vérifiée par un test (tests_edt_t0).
--
-- Additive et rejouable (E8) ; aucune donnée n'est modifiée ni supprimée.

ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS retiree_motif TEXT;

DO $$
BEGIN
    -- Nouvelle liste d'états : `retiree` s'ajoute (les états du socle 10 complet arrivent en T2).
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_statut_t0_chk') THEN
        ALTER TABLE ent_edt_seance DROP CONSTRAINT IF EXISTS ent_edt_seance_statut_chk;
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_statut_t0_chk
            CHECK (statut IN ('prevue', 'annulee', 'remplacee', 'retiree'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_retiree_chk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_retiree_chk
            CHECK ((statut = 'retiree') = (retiree_le IS NOT NULL));
    END IF;
    -- Contenu du cahier : plus de suppression en cascade.
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conrelid = 'ent_seance_contenu'::regclass AND contype = 'f'
                  AND confrelid = 'ent_edt_seance'::regclass AND confdeltype <> 'r') THEN
        ALTER TABLE ent_seance_contenu DROP CONSTRAINT ent_seance_contenu_seance_id_fkey;
        ALTER TABLE ent_seance_contenu ADD CONSTRAINT ent_seance_contenu_seance_id_fkey
            FOREIGN KEY (seance_id) REFERENCES ent_edt_seance (id) ON DELETE RESTRICT;
    END IF;
    -- Travail à faire : la séance ne disparaît plus sous lui.
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conrelid = 'ent_travail'::regclass AND contype = 'f'
                  AND confrelid = 'ent_edt_seance'::regclass AND confdeltype <> 'r') THEN
        ALTER TABLE ent_travail DROP CONSTRAINT ent_travail_seance_id_fkey;
        ALTER TABLE ent_travail ADD CONSTRAINT ent_travail_seance_id_fkey
            FOREIGN KEY (seance_id) REFERENCES ent_edt_seance (id) ON DELETE RESTRICT;
    END IF;
END
$$;

-- Les lectures de séances actives écartent les retirées : index partiel d'appui.
CREATE INDEX IF NOT EXISTS ent_edt_seance_actives_idx
    ON ent_edt_seance (etablissement_id, jour) WHERE statut <> 'retiree';
