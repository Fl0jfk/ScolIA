-- 652 — Socle 42, lot V2a : motif CODÉ du contournement du devis signé (NX-07, §32.5, VOY-168).
-- Réf. : 42_SORTIES_VOYAGES.md §13.2, §32.5. Avant ce lot, le contournement n'avait qu'un texte libre.
--
-- Codes : transport en commun, à pied, véhicule de l'établissement, autre (avec texte). « Transport en commun », « à
-- pied » et « véhicule de l'établissement » ne demandent aucune justification supplémentaire ; « autre » exige un
-- texte. Les contournements déjà enregistrés (texte seul) sont classés « autre ». La colonne texte existante
-- (640) reste la justification libre ; aucune migration versée n'est modifiée.

ALTER TABLE ent_sortie ADD COLUMN IF NOT EXISTS contournement_devis_code TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_contournement_code_chk') THEN
        ALTER TABLE ent_sortie ADD CONSTRAINT ent_sortie_contournement_code_chk CHECK (
            contournement_devis_code IS NULL
            OR (contournement_devis_code IN ('transport_en_commun', 'a_pied', 'vehicule_etablissement', 'autre')
                AND contournement_devis_motif IS NOT NULL
                AND (contournement_devis_code <> 'autre' OR length(btrim(contournement_devis_motif)) > 0)));
    END IF;
END
$$;

-- Reprise de l'existant : un texte seul devient « autre ».
UPDATE ent_sortie SET contournement_devis_code = 'autre'
 WHERE contournement_devis_motif IS NOT NULL AND contournement_devis_code IS NULL;
