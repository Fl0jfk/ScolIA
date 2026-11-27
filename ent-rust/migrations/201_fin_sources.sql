-- 201 — Facturation par sources idempotentes et avoirs (S2-9, FIN-02/04/06).
--
-- Une **source** (inscription, changement de régime, sortie, absence prolongée,
-- consommation…) produit **au plus une ligne** par foyer payeur : clé
-- `source_cle` unique par établissement. Le rejeu d'un événement ne double rien.
-- Un **avoir** est une pièce `type_piece = 'avoir'` (montants négatifs) qui
-- référence la facture d'origine.

ALTER TABLE ent_fac_facture
    ADD COLUMN IF NOT EXISTS type_piece TEXT NOT NULL DEFAULT 'facture';
ALTER TABLE ent_fac_facture DROP CONSTRAINT IF EXISTS ent_fac_facture_type_piece_chk;
ALTER TABLE ent_fac_facture
    ADD CONSTRAINT ent_fac_facture_type_piece_chk CHECK (type_piece IN ('facture', 'avoir'));
ALTER TABLE ent_fac_facture
    ADD COLUMN IF NOT EXISTS facture_origine_id UUID REFERENCES ent_fac_facture (id);
-- Origine : 'annuelle' (échéancier de l'année), 'evenement' (fait métier),
-- 'consommation', 'frais', 'manuelle'. NULL pour les factures antérieures.
ALTER TABLE ent_fac_facture
    ADD COLUMN IF NOT EXISTS origine TEXT;

CREATE INDEX IF NOT EXISTS ent_fac_facture_origine_idx
    ON ent_fac_facture (etablissement_id, foyer_id, annee_scolaire, origine, statut);
CREATE INDEX IF NOT EXISTS ent_fac_facture_avoir_idx
    ON ent_fac_facture (etablissement_id, facture_origine_id) WHERE facture_origine_id IS NOT NULL;

ALTER TABLE ent_fac_ligne ADD COLUMN IF NOT EXISTS source_cle TEXT;
ALTER TABLE ent_fac_ligne ADD COLUMN IF NOT EXISTS article_id UUID REFERENCES ent_fin_article (id);
ALTER TABLE ent_fac_ligne ADD COLUMN IF NOT EXISTS categorie TEXT;
ALTER TABLE ent_fac_ligne ADD COLUMN IF NOT EXISTS periode_debut DATE;
ALTER TABLE ent_fac_ligne ADD COLUMN IF NOT EXISTS periode_fin DATE;
-- Ligne d'avoir : ligne de facture qu'elle annule (en tout ou partie).
ALTER TABLE ent_fac_ligne ADD COLUMN IF NOT EXISTS ligne_origine_id UUID REFERENCES ent_fac_ligne (id);

CREATE UNIQUE INDEX IF NOT EXISTS ent_fac_ligne_source_uq
    ON ent_fac_ligne (etablissement_id, source_cle) WHERE source_cle IS NOT NULL;

-- Mode de paiement d'un foyer payeur : prélèvement (mandat actif) ou manuel
-- (retour au paiement manuel après rejets, ou choix).
CREATE TABLE IF NOT EXISTS ent_fin_compte_payeur (
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id) ON DELETE CASCADE,
    mode_paiement TEXT NOT NULL DEFAULT 'prelevement',
    motif TEXT,
    depuis TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID REFERENCES ent_user (id),
    PRIMARY KEY (etablissement_id, foyer_id),
    CONSTRAINT ent_fin_compte_payeur_mode_chk CHECK (mode_paiement IN ('prelevement', 'manuel'))
);
