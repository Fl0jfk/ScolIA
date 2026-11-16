-- 2251 — Socle 12, lot AS6. Réf. : 12_ASSIDUITE.md §9.5 (appel non fait), AS-151, CA-41.
-- La relance d'un appel non fait (table 132) peut maintenant être AUTOMATIQUE : posée par le système à
-- `début + délai`, UNE SEULE FOIS par séance et par jour. Rien n'est réécrit : on ajoute l'origine, on rend
-- l'envoyeur facultatif pour une relance automatique, et la base interdit une seconde relance automatique.
-- Additive, rejouable (E8) ; aucune migration existante modifiée ; aucune colonne JSON.

ALTER TABLE ent_appel_relance ADD COLUMN IF NOT EXISTS origine TEXT NOT NULL DEFAULT 'manuelle';
ALTER TABLE ent_appel_relance DROP CONSTRAINT IF EXISTS ent_appel_relance_origine_chk;
ALTER TABLE ent_appel_relance ADD CONSTRAINT ent_appel_relance_origine_chk CHECK (origine IN ('manuelle', 'automatique'));
ALTER TABLE ent_appel_relance ALTER COLUMN envoyee_par_user_id DROP NOT NULL;
ALTER TABLE ent_appel_relance DROP CONSTRAINT IF EXISTS ent_appel_relance_envoyeur_chk;
ALTER TABLE ent_appel_relance ADD CONSTRAINT ent_appel_relance_envoyeur_chk CHECK (
    (origine = 'automatique' AND envoyee_par_user_id IS NULL)
    OR (origine = 'manuelle' AND envoyee_par_user_id IS NOT NULL));

-- Une seule relance automatique par séance et par jour : garanti par la base, pas seulement par le code.
CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_relance_auto_uq
    ON ent_appel_relance (etablissement_id, creneau_id, jour) WHERE origine = 'automatique';
