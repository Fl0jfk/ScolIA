-- Socle 2, lot S2-3 : coordonnées bancaires à la validation de l'inscription
-- (D-014, INS-14). Jamais demandées à la préinscription.
--
-- Un mandat SEPA ne se transfère pas : chaque établissement (créancier distinct)
-- fait signer un NOUVEAU mandat. La demande porte l'IBAN (repris masqué d'un
-- autre établissement à l'initiative du parent, ou saisi), chiffré comme
-- `ent_fac_sepa_mandat` (D3 : `champs_chiffres.rs`). À la signature, le mandat
-- actif est créé par le domaine facturation (`ent_fac_sepa_mandat`).
-- Aucune référence à l'établissement d'origine n'est stockée ici.
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_mandat_sepa_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    dossier_id UUID NOT NULL REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    iban TEXT NOT NULL,
    bic TEXT,
    titulaire TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('reprise', 'saisie')),
    statut TEXT NOT NULL DEFAULT 'a_signer' CHECK (statut IN ('a_signer', 'signe', 'annule')),
    mandat_id UUID REFERENCES ent_fac_sepa_mandat (id),
    demande_par_user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    signe_le TIMESTAMPTZ,
    CONSTRAINT ent_mandat_sepa_demande_signe_chk
        CHECK (statut <> 'signe' OR (mandat_id IS NOT NULL AND signe_le IS NOT NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_mandat_sepa_demande_active_uq
    ON ent_mandat_sepa_demande (dossier_id) WHERE statut IN ('a_signer', 'signe');
