-- Facturation familles P1 (tranche) : facture foyer + lignes (montants en centimes).

CREATE TABLE IF NOT EXISTS ent_fac_facture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    numero TEXT NOT NULL,
    libelle TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'brouillon',
    total_centimes BIGINT NOT NULL DEFAULT 0,
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    emise_at TIMESTAMPTZ,
    CONSTRAINT ent_fac_facture_numero_uq UNIQUE (etablissement_id, numero),
    CONSTRAINT ent_fac_facture_statut_chk CHECK (
        statut IN ('brouillon', 'emise', 'soldee')
    )
);

CREATE INDEX IF NOT EXISTS ent_fac_facture_foyer_idx
    ON ent_fac_facture (etablissement_id, foyer_id);

CREATE INDEX IF NOT EXISTS ent_fac_facture_etab_idx
    ON ent_fac_facture (etablissement_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ent_fac_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id) ON DELETE CASCADE,
    eleve_id UUID REFERENCES ent_eleve (id),
    libelle TEXT NOT NULL,
    quantite INT NOT NULL DEFAULT 1,
    montant_unitaire_centimes BIGINT NOT NULL,
    CONSTRAINT ent_fac_ligne_qte_chk CHECK (quantite > 0)
);

CREATE INDEX IF NOT EXISTS ent_fac_ligne_facture_idx
    ON ent_fac_ligne (etablissement_id, facture_id);
