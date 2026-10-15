-- Vanne paiement acompte réinscription N+1.
-- Campagne : montant d'acompte requis (0 = pas de vanne).
-- Facture : nature + année pour cibler les acomptes encaissés.

ALTER TABLE ent_reinscription_campagne
    ADD COLUMN IF NOT EXISTS acompte_centimes BIGINT NOT NULL DEFAULT 0;

ALTER TABLE ent_fac_facture
    ADD COLUMN IF NOT EXISTS nature TEXT;

ALTER TABLE ent_fac_facture
    ADD COLUMN IF NOT EXISTS annee_scolaire TEXT;

CREATE INDEX IF NOT EXISTS ent_fac_facture_nature_annee_idx
    ON ent_fac_facture (etablissement_id, foyer_id, nature, annee_scolaire)
    WHERE nature IS NOT NULL;
