-- 210 — Santé (socle 2, lot S2-11) : PAI complet (SAN-05). Additif et idempotent.
--
-- * `version` : incrémentée à chaque mise à jour du PAI ; les signatures portent la
--   version signée, donc toute modification invalide les signatures précédentes.
-- * révision annuelle : `date_revision` (échéance), calculée à la création.
-- * trousse d'urgence : présence, lieu de rangement (non médical), péremption.
-- * `ent_pai_signature` : une ligne par partie et par version (famille, médecin,
--   chef d'établissement, infirmier, collectivité). Aucun texte médical.
-- Données de santé (Q7 HDS en attente du DPO) : les textes du PAI sont chiffrés
-- (socle 1, D3) ; voir `champs_chiffres.rs`.

ALTER TABLE ent_pai ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE ent_pai ADD COLUMN IF NOT EXISTS date_revision DATE;
ALTER TABLE ent_pai ADD COLUMN IF NOT EXISTS trousse_urgence BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_pai ADD COLUMN IF NOT EXISTS trousse_lieu TEXT NOT NULL DEFAULT '';
ALTER TABLE ent_pai ADD COLUMN IF NOT EXISTS trousse_peremption DATE;

CREATE TABLE IF NOT EXISTS ent_pai_signature (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    pai_id UUID NOT NULL REFERENCES ent_pai (id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    partie TEXT NOT NULL
        CHECK (partie IN ('famille', 'medecin', 'chef_etablissement', 'infirmier', 'collectivite')),
    signataire_nom TEXT NOT NULL CHECK (length(btrim(signataire_nom)) BETWEEN 1 AND 120),
    -- Compte du signataire (famille, infirmier, direction) ou du membre du personnel
    -- qui consigne la signature d'une partie sans compte (médecin, collectivité).
    signataire_user_id UUID REFERENCES ent_user (id),
    signe_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_pai_signature_uq UNIQUE (pai_id, version, partie)
);

CREATE INDEX IF NOT EXISTS ent_pai_signature_pai_idx
    ON ent_pai_signature (etablissement_id, pai_id, version);
