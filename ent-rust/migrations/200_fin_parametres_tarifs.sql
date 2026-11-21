-- 200 — Finances reliées (socle 2, lot S2-9) : paramètres, articles, grilles
-- tarifaires, réductions, barèmes de bourse, calendrier d'échéances.
-- Additif et idempotent. Montants en centimes entiers (BIGINT), taux en points
-- de base (1 % = 100 bps). Année scolaire au format « AAAA-AAAA » (comme
-- ent_fac_facture.annee_scolaire et ent_inscription_campagne.annee_cible).

-- Paramètres de facturation de l'établissement (une ligne par établissement).
CREATE TABLE IF NOT EXISTS ent_fin_parametres (
    etablissement_id UUID PRIMARY KEY,
    -- Mois facturables de l'année scolaire : de `mois_debut` sur `nb_mois` mois.
    mois_debut SMALLINT NOT NULL DEFAULT 9,
    nb_mois SMALLINT NOT NULL DEFAULT 10,
    -- Jour du mois des échéances calculées quand aucun calendrier n'est posé.
    jour_echeance SMALLINT NOT NULL DEFAULT 5,
    -- SEPA : délai minimal de pré-notification (jours calendaires) avant exécution.
    jours_prenotification SMALLINT NOT NULL DEFAULT 14,
    -- Frais refacturés à chaque rejet de prélèvement (0 = aucun).
    frais_rejet_centimes BIGINT NOT NULL DEFAULT 0,
    -- Nombre de rejets d'un foyer sur l'année avant retour au paiement manuel.
    rejets_avant_manuel SMALLINT NOT NULL DEFAULT 2,
    -- Absence prolongée (remise d'ordre) : seuil en jours calendaires d'une absence justifiée.
    seuil_absence_jours SMALLINT NOT NULL DEFAULT 15,
    -- Créancier SEPA (compte de l'établissement, pas une donnée personnelle).
    crediteur_nom TEXT,
    crediteur_iban TEXT,
    crediteur_bic TEXT,
    crediteur_ics TEXT,
    -- Comptabilité : comptes et journaux de l'export.
    compte_clients TEXT NOT NULL DEFAULT '411000',
    compte_banque TEXT NOT NULL DEFAULT '512000',
    journal_ventes TEXT NOT NULL DEFAULT 'VT',
    journal_banque TEXT NOT NULL DEFAULT 'BQ',
    updated_by UUID REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_parametres_mois_chk CHECK (mois_debut BETWEEN 1 AND 12 AND nb_mois BETWEEN 1 AND 12),
    CONSTRAINT ent_fin_parametres_jour_chk CHECK (jour_echeance BETWEEN 1 AND 28),
    CONSTRAINT ent_fin_parametres_prenotif_chk CHECK (jours_prenotification BETWEEN 1 AND 60),
    CONSTRAINT ent_fin_parametres_frais_chk CHECK (frais_rejet_centimes BETWEEN 0 AND 100000),
    CONSTRAINT ent_fin_parametres_rejets_chk CHECK (rejets_avant_manuel BETWEEN 1 AND 10),
    CONSTRAINT ent_fin_parametres_seuil_chk CHECK (seuil_absence_jours BETWEEN 1 AND 365)
);

-- Article facturable (FIN-01) : ce qui est vendu, avec sa catégorie et son compte de produit.
CREATE TABLE IF NOT EXISTS ent_fin_article (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    categorie TEXT NOT NULL,
    compte_produit TEXT NOT NULL DEFAULT '706000',
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_article_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_fin_article_categorie_chk CHECK (
        categorie IN ('scolarite', 'restauration', 'internat', 'activite', 'frais', 'bourse', 'reduction')
    )
);

-- Grille tarifaire annuelle (FIN-01) : un montant annuel par article, filtré par
-- niveau, régime et tranche de quotient familial (bornes en centimes, [min, max[).
CREATE TABLE IF NOT EXISTS ent_fin_tarif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    article_id UUID NOT NULL REFERENCES ent_fin_article (id) ON DELETE RESTRICT,
    niveau TEXT,
    regime TEXT,
    qf_min_centimes BIGINT,
    qf_max_centimes BIGINT,
    montant_annuel_centimes BIGINT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_tarif_annee_chk CHECK (annee_scolaire ~ '^[0-9]{4}-[0-9]{4}$'),
    CONSTRAINT ent_fin_tarif_regime_chk CHECK (regime IS NULL OR regime IN ('externe', 'demi_pension', 'interne')),
    CONSTRAINT ent_fin_tarif_montant_chk CHECK (montant_annuel_centimes BETWEEN 0 AND 100000000),
    CONSTRAINT ent_fin_tarif_qf_chk CHECK (
        (qf_min_centimes IS NULL OR qf_min_centimes >= 0)
        AND (qf_max_centimes IS NULL OR qf_min_centimes IS NULL OR qf_max_centimes > qf_min_centimes)
    )
);

CREATE INDEX IF NOT EXISTS ent_fin_tarif_annee_idx
    ON ent_fin_tarif (etablissement_id, annee_scolaire) WHERE actif;

-- Réductions (FIN-07) : fratrie (à partir du N-ième enfant d'un même foyer payeur),
-- personnel ou autre (affectées à un élève), en taux sur des catégories d'articles.
CREATE TABLE IF NOT EXISTS ent_fin_reduction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    nature TEXT NOT NULL,
    rang_min SMALLINT,
    taux_bps INTEGER NOT NULL,
    categories TEXT[] NOT NULL DEFAULT ARRAY['scolarite']::TEXT[],
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_reduction_code_uq UNIQUE (etablissement_id, annee_scolaire, code),
    CONSTRAINT ent_fin_reduction_nature_chk CHECK (nature IN ('fratrie', 'personnel', 'autre')),
    CONSTRAINT ent_fin_reduction_taux_chk CHECK (taux_bps BETWEEN 1 AND 10000),
    CONSTRAINT ent_fin_reduction_rang_chk CHECK (
        (nature = 'fratrie' AND rang_min BETWEEN 2 AND 20) OR (nature <> 'fratrie' AND rang_min IS NULL)
    )
);

-- Réduction « personnel » / « autre » accordée à un élève.
CREATE TABLE IF NOT EXISTS ent_fin_reduction_eleve (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    reduction_id UUID NOT NULL REFERENCES ent_fin_reduction (id) ON DELETE CASCADE,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_reduction_eleve_uq UNIQUE (etablissement_id, reduction_id, eleve_id)
);

-- Barème des bourses (FIN-07) : montant annuel déduit par (type, échelon) pour
-- une bourse ACCEPTÉE avec procuration (versée à l'établissement).
CREATE TABLE IF NOT EXISTS ent_fin_bourse_bareme (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    bourse_type_id UUID NOT NULL REFERENCES ent_bourse_type (id) ON DELETE RESTRICT,
    echelon TEXT NOT NULL,
    montant_annuel_centimes BIGINT NOT NULL,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_bourse_bareme_uq UNIQUE (etablissement_id, annee_scolaire, bourse_type_id, echelon),
    CONSTRAINT ent_fin_bourse_bareme_montant_chk CHECK (montant_annuel_centimes BETWEEN 1 AND 100000000)
);

-- Calendrier d'échéances de l'année (FIN-05) : dates des prélèvements / paiements.
CREATE TABLE IF NOT EXISTS ent_fin_echeancier_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    rang SMALLINT NOT NULL,
    date_echeance DATE NOT NULL,
    CONSTRAINT ent_fin_echeancier_type_uq UNIQUE (etablissement_id, annee_scolaire, rang),
    CONSTRAINT ent_fin_echeancier_type_rang_chk CHECK (rang BETWEEN 1 AND 24)
);

-- Compteurs de numérotation continue (factures, avoirs, lettrages, exports).
CREATE TABLE IF NOT EXISTS ent_fin_compteur (
    etablissement_id UUID NOT NULL,
    cle TEXT NOT NULL,
    dernier BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (etablissement_id, cle)
);
