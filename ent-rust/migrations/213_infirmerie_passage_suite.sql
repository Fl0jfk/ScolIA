-- 213 — Passages à l'infirmerie (SAN-02/03/04) : heures, soins, issue, information
-- de la famille. Additif et idempotent.
--
-- * `soins` (chiffré) complète `note_soins` (chiffré aussi, voir champs_chiffres.rs).
-- * `issue` : retour en classe, repos, famille appelée, renvoi au domicile, secours.
-- * information de la famille : état + horodatage ; la notification ne contient
--   aucun détail médical (ni motif, ni soins).
-- * renvoi au domicile : qui est venu chercher l'élève (responsable ou contact
--   habilité à récupérer), jamais une saisie libre non vérifiée.

ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS heure_arrivee TIME;
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS heure_sortie TIME;
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS soins TEXT NOT NULL DEFAULT '';
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS issue TEXT NOT NULL DEFAULT 'non_renseignee';
ALTER TABLE ent_infirmerie_passage DROP CONSTRAINT IF EXISTS ent_infirmerie_passage_issue_chk;
ALTER TABLE ent_infirmerie_passage ADD CONSTRAINT ent_infirmerie_passage_issue_chk
    CHECK (issue IN ('non_renseignee', 'retour_classe', 'repos_infirmerie', 'famille_appelee',
                     'renvoi_domicile', 'secours_appeles'));
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS famille_information TEXT NOT NULL DEFAULT 'non';
ALTER TABLE ent_infirmerie_passage DROP CONSTRAINT IF EXISTS ent_infirmerie_passage_info_chk;
ALTER TABLE ent_infirmerie_passage ADD CONSTRAINT ent_infirmerie_passage_info_chk
    CHECK (famille_information IN ('non', 'a_informer', 'informee'));
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS famille_informee_at TIMESTAMPTZ;
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS famille_informee_par UUID;
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS recupere_par_nom TEXT;
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS recupere_par_personne_id UUID;
ALTER TABLE ent_infirmerie_passage ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
