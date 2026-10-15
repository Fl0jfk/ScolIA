-- Continuité N-1 / N / N+1 sur la fiche (rejet year-jump « Fichier » Charlemagne).
-- Montée rapide = projection N+1 sans valeur juridique de passage (redoublement encore possible).

CREATE TABLE IF NOT EXISTS ent_reinscription_campagne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_cible TEXT NOT NULL,
    ouverte BOOLEAN NOT NULL DEFAULT false,
    ouverte_le TIMESTAMPTZ,
    fermee_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_reinscription_campagne_annee_uq UNIQUE (etablissement_id, annee_cible)
);

CREATE INDEX IF NOT EXISTS ent_reinscription_campagne_etab_idx
    ON ent_reinscription_campagne (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_eleve_scolarite_annee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    niveau TEXT,
    mef TEXT,
    classe TEXT,
    regime TEXT,
    statut TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'saisie',
    -- false pour montée rapide / réinscription anticipée (pas un conseil de classe).
    passage_juridique BOOLEAN NOT NULL DEFAULT false,
    redoublement_possible BOOLEAN NOT NULL DEFAULT true,
    date_montee DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_scolarite_annee_uq
        UNIQUE (etablissement_id, eleve_id, annee_scolaire),
    CONSTRAINT ent_eleve_scolarite_statut_chk
        CHECK (statut IN ('historique', 'courante', 'projetee', 'reinscription')),
    CONSTRAINT ent_eleve_scolarite_source_chk
        CHECK (source IN ('import', 'saisie', 'montee_rapide', 'reinscription_parent', 'sync_courante'))
);

CREATE INDEX IF NOT EXISTS ent_eleve_scolarite_annee_eleve_idx
    ON ent_eleve_scolarite_annee (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_eleve_scolarite_annee_annee_idx
    ON ent_eleve_scolarite_annee (etablissement_id, annee_scolaire);

-- Options rattachées à une année (surtout propositions N+1).
CREATE TABLE IF NOT EXISTS ent_eleve_option_annee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    nature TEXT NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'proposee',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_option_annee_uq
        UNIQUE (etablissement_id, eleve_id, annee_scolaire, nature, code),
    CONSTRAINT ent_eleve_option_annee_nature_chk
        CHECK (nature IN ('academique', 'interne')),
    CONSTRAINT ent_eleve_option_annee_statut_chk
        CHECK (statut IN ('proposee', 'retenue', 'retiree'))
);

CREATE INDEX IF NOT EXISTS ent_eleve_option_annee_eleve_idx
    ON ent_eleve_option_annee (etablissement_id, eleve_id, annee_scolaire);
