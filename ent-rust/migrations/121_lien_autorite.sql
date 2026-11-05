-- Socle 2, lot S2-1 (socle 1 B7) : autorité légale, garde, dates et suspension
-- des liens de responsabilité ; vue unique des liens ACTIFS.
--
-- L'autorité parentale est distincte du lien familial et des droits fins
-- (`peut_voir_notes`, `peut_voir_vs`…, conservés). Un lien suspendu (décision de
-- justice) ou hors de ses dates n'ouvre plus AUCUN accès : tous les contrôles
-- famille lisent `ent_eleve_lien_actif` (point unique, voir `responsabilite.rs`).

-- Les liens existants gardent leur comportement : autorité présumée (valeur
-- posée une seule fois à l'ajout de la colonne), aucune date, non suspendus.
ALTER TABLE ent_eleve_lien
    ADD COLUMN IF NOT EXISTS autorite_parentale BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE ent_eleve_lien
    ADD COLUMN IF NOT EXISTS garde TEXT
        CHECK (garde IS NULL OR garde IN ('exclusive', 'alternee', 'aucune'));
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS date_debut DATE;
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS date_fin DATE;
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS suspendu_le TIMESTAMPTZ;
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS suspendu_par UUID;
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS suspension_motif TEXT;
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS suspension_reference TEXT;
ALTER TABLE ent_eleve_lien ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE ent_eleve_lien DROP CONSTRAINT IF EXISTS ent_eleve_lien_dates_chk;
ALTER TABLE ent_eleve_lien
    ADD CONSTRAINT ent_eleve_lien_dates_chk
    CHECK (date_debut IS NULL OR date_fin IS NULL OR date_fin >= date_debut);

-- Une suspension est toujours motivée (décision, référence).
ALTER TABLE ent_eleve_lien DROP CONSTRAINT IF EXISTS ent_eleve_lien_suspension_chk;
ALTER TABLE ent_eleve_lien
    ADD CONSTRAINT ent_eleve_lien_suspension_chk
    CHECK (
        suspendu_le IS NULL
        OR (suspension_motif IS NOT NULL AND length(btrim(suspension_motif)) BETWEEN 3 AND 500)
    );

-- Liens ACTIFS aujourd'hui (UTC, comme `today_utc()`) : non suspendus, dans
-- leurs dates, personne non absorbée par une fusion. Recréée à chaque passage :
-- la liste des colonnes suit la table.
DROP VIEW IF EXISTS ent_eleve_lien_actif;
CREATE VIEW ent_eleve_lien_actif AS
SELECT l.*
FROM ent_eleve_lien l
JOIN ent_personne p
  ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
WHERE l.suspendu_le IS NULL
  AND (l.date_debut IS NULL OR l.date_debut <= (now() AT TIME ZONE 'UTC')::date)
  AND (l.date_fin IS NULL OR l.date_fin >= (now() AT TIME ZONE 'UTC')::date)
  AND p.fusionnee_dans IS NULL;
