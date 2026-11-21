-- Socle 2, lot S2-3 : campagnes d'admission et de réinscription (INS-01, INS-06).
--
-- Une campagne est paramétrée par l'établissement : nature, année cible, dates,
-- niveaux ouverts (capacité facultative), pièces demandées. Une seule campagne
-- ouverte par (établissement, nature, année cible).
-- L'ancienne table `ent_reinscription_campagne` (018) reste lue par l'ancienne
-- interface : elle n'est ni modifiée ni reprise (écart signalé dans SOCLE2_DOMAINES).
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_inscription_campagne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature IN ('admission', 'reinscription')),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 160),
    annee_cible TEXT NOT NULL CHECK (annee_cible ~ '^[0-9]{4}-[0-9]{4}$'),
    date_ouverture DATE,
    date_cloture DATE,
    statut TEXT NOT NULL DEFAULT 'brouillon'
        CHECK (statut IN ('brouillon', 'ouverte', 'fermee')),
    message_famille TEXT CHECK (message_famille IS NULL OR length(message_famille) <= 2000),
    created_by_user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_campagne_dates_chk
        CHECK (date_cloture IS NULL OR date_ouverture IS NULL OR date_cloture >= date_ouverture)
);

CREATE INDEX IF NOT EXISTS ent_inscription_campagne_etab_idx
    ON ent_inscription_campagne (etablissement_id, nature, annee_cible);

CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_campagne_ouverte_uq
    ON ent_inscription_campagne (etablissement_id, nature, annee_cible)
    WHERE statut = 'ouverte';

CREATE TABLE IF NOT EXISTS ent_inscription_campagne_niveau (
    campagne_id UUID NOT NULL REFERENCES ent_inscription_campagne (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (length(btrim(code)) BETWEEN 1 AND 40),
    capacite INTEGER CHECK (capacite IS NULL OR capacite >= 0),
    PRIMARY KEY (campagne_id, code)
);

CREATE TABLE IF NOT EXISTS ent_inscription_campagne_piece (
    campagne_id UUID NOT NULL REFERENCES ent_inscription_campagne (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[a-z0-9_]{1,40}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 160),
    obligatoire BOOLEAN NOT NULL DEFAULT true,
    ordre SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (campagne_id, code)
);
