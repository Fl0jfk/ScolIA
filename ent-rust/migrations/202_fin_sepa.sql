-- 202 — Prélèvements par échéance, pré-notification, rejets SEPA (S2-9, FIN-11/12).

ALTER TABLE ent_fac_sepa_prelevement
    ADD COLUMN IF NOT EXISTS echeance_id UUID REFERENCES ent_fac_echeance (id);
ALTER TABLE ent_fac_sepa_prelevement
    ADD COLUMN IF NOT EXISTS prenotification_id UUID;
-- Rang de présentation de l'échéance (1 = première, 2 = re-présentation après rejet).
ALTER TABLE ent_fac_sepa_prelevement
    ADD COLUMN IF NOT EXISTS presentation SMALLINT NOT NULL DEFAULT 1;

-- Une échéance n'a au plus qu'un prélèvement en cours ou exécuté.
CREATE UNIQUE INDEX IF NOT EXISTS ent_fac_sepa_prelevement_echeance_uq
    ON ent_fac_sepa_prelevement (etablissement_id, echeance_id)
    WHERE echeance_id IS NOT NULL AND statut IN ('prevu', 'exporte', 'execute');

-- Pré-notification d'un foyer pour une date d'exécution (montant total annoncé).
CREATE TABLE IF NOT EXISTS ent_fin_prenotification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    mandat_id UUID NOT NULL REFERENCES ent_fac_sepa_mandat (id),
    date_execution DATE NOT NULL,
    montant_centimes BIGINT NOT NULL,
    nb_prelevements INTEGER NOT NULL,
    envoyee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID REFERENCES ent_user (id),
    CONSTRAINT ent_fin_prenotification_uq UNIQUE (etablissement_id, foyer_id, date_execution),
    CONSTRAINT ent_fin_prenotification_montant_chk CHECK (montant_centimes > 0)
);

-- Rejet traité (FIN-12) : motif normalisé, frais, suite donnée.
CREATE TABLE IF NOT EXISTS ent_fin_rejet (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    prelevement_id UUID NOT NULL REFERENCES ent_fac_sepa_prelevement (id),
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id),
    echeance_id UUID REFERENCES ent_fac_echeance (id),
    code_motif TEXT NOT NULL,
    libelle_motif TEXT NOT NULL,
    montant_centimes BIGINT NOT NULL,
    frais_centimes BIGINT NOT NULL DEFAULT 0,
    facture_frais_id UUID REFERENCES ent_fac_facture (id),
    -- 'representation' : l'échéance sera reprise au prochain lot ;
    -- 'manuel' : le foyer repasse au paiement manuel.
    suite TEXT NOT NULL,
    traite_par UUID REFERENCES ent_user (id),
    traite_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_rejet_prelevement_uq UNIQUE (etablissement_id, prelevement_id),
    CONSTRAINT ent_fin_rejet_suite_chk CHECK (suite IN ('representation', 'manuel')),
    CONSTRAINT ent_fin_rejet_frais_chk CHECK (frais_centimes >= 0)
);

CREATE INDEX IF NOT EXISTS ent_fin_rejet_foyer_idx
    ON ent_fin_rejet (etablissement_id, foyer_id, traite_le DESC);
