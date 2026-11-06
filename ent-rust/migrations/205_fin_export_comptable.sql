-- 205 — Export comptable (S2-9, FIN-16) : une pièce n'est exportée qu'une fois.

CREATE TABLE IF NOT EXISTS ent_fin_export (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    numero TEXT NOT NULL,
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    format TEXT NOT NULL,
    nb_ecritures INTEGER NOT NULL,
    total_debit_centimes BIGINT NOT NULL,
    total_credit_centimes BIGINT NOT NULL,
    -- Empreinte SHA-256 (hexadécimal) du fichier remis : preuve d'intégrité.
    empreinte TEXT NOT NULL,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_export_numero_uq UNIQUE (etablissement_id, numero),
    CONSTRAINT ent_fin_export_dates_chk CHECK (fin >= debut),
    CONSTRAINT ent_fin_export_equilibre_chk CHECK (total_debit_centimes = total_credit_centimes)
);

CREATE TABLE IF NOT EXISTS ent_fin_export_piece (
    etablissement_id UUID NOT NULL,
    export_id UUID NOT NULL REFERENCES ent_fin_export (id) ON DELETE CASCADE,
    piece_type TEXT NOT NULL,
    piece_id UUID NOT NULL,
    PRIMARY KEY (etablissement_id, piece_type, piece_id),
    CONSTRAINT ent_fin_export_piece_type_chk CHECK (piece_type IN ('facture', 'avoir', 'encaissement', 'rejet'))
);
