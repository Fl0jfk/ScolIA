-- 204 — Relances graduées avec confirmation humaine (S2-9, FIN-13).
--
-- Le système **propose** (jamais avant l'échéance) ; une personne habilitée
-- **confirme** l'envoi ou annule. Une relance par (facture, palier).

CREATE TABLE IF NOT EXISTS ent_fin_palier_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    rang SMALLINT NOT NULL,
    jours_retard SMALLINT NOT NULL,
    libelle TEXT NOT NULL,
    frais_centimes BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT ent_fin_palier_relance_uq UNIQUE (etablissement_id, rang),
    CONSTRAINT ent_fin_palier_relance_rang_chk CHECK (rang BETWEEN 1 AND 5),
    CONSTRAINT ent_fin_palier_relance_jours_chk CHECK (jours_retard BETWEEN 1 AND 365),
    CONSTRAINT ent_fin_palier_relance_frais_chk CHECK (frais_centimes BETWEEN 0 AND 100000)
);

CREATE TABLE IF NOT EXISTS ent_fin_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id),
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    palier SMALLINT NOT NULL,
    libelle TEXT NOT NULL,
    montant_du_centimes BIGINT NOT NULL,
    jours_retard INTEGER NOT NULL,
    frais_centimes BIGINT NOT NULL DEFAULT 0,
    statut TEXT NOT NULL DEFAULT 'proposee',
    proposee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    decide_par UUID REFERENCES ent_user (id),
    decide_le TIMESTAMPTZ,
    facture_frais_id UUID REFERENCES ent_fac_facture (id),
    CONSTRAINT ent_fin_relance_uq UNIQUE (etablissement_id, facture_id, palier),
    CONSTRAINT ent_fin_relance_statut_chk CHECK (statut IN ('proposee', 'envoyee', 'annulee')),
    CONSTRAINT ent_fin_relance_montant_chk CHECK (montant_du_centimes > 0)
);

CREATE INDEX IF NOT EXISTS ent_fin_relance_statut_idx
    ON ent_fin_relance (etablissement_id, statut, proposee_le DESC);
