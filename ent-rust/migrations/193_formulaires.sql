-- Socle 2, lot S2-8 : formulaires, autorisations et consentements versionnés
-- (D24, FRM-01→04). Un consentement est un formulaire de type « consentement »
-- portant un `objet` stable (ex. droit_image) ; ses réponses sont des
-- décisions DATÉES et immuables (historique conservé, jamais d'écrasement).

CREATE TABLE IF NOT EXISTS ent_formulaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    type TEXT NOT NULL,
    -- Identifiant stable de l'objet recueilli (consentement / autorisation).
    objet TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    titre TEXT NOT NULL,
    texte TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'brouillon',
    audience TEXT NOT NULL DEFAULT 'etablissement',
    cible_ids UUID[] NOT NULL DEFAULT '{}',
    cree_par UUID NOT NULL,
    publie_le TIMESTAMPTZ,
    archive_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_formulaire_type_chk CHECK (type IN ('consentement', 'autorisation', 'sondage')),
    CONSTRAINT ent_formulaire_statut_chk CHECK (statut IN ('brouillon', 'publie', 'archive')),
    CONSTRAINT ent_formulaire_audience_chk CHECK (audience IN ('etablissement', 'niveau', 'classe', 'groupe')),
    CONSTRAINT ent_formulaire_objet_chk CHECK (
        (type = 'sondage' AND objet IS NULL)
        OR (type <> 'sondage' AND objet IS NOT NULL AND objet ~ '^[a-z][a-z0-9_]{1,39}$')
    ),
    CONSTRAINT ent_formulaire_titre_chk CHECK (length(btrim(titre)) > 0 AND length(titre) <= 200),
    CONSTRAINT ent_formulaire_texte_chk CHECK (length(btrim(texte)) > 0 AND length(texte) <= 10000),
    CONSTRAINT ent_formulaire_version_chk CHECK (version >= 1)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_formulaire_objet_version_uq
    ON ent_formulaire (etablissement_id, objet, version) WHERE objet IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_formulaire_etab_idx
    ON ent_formulaire (etablissement_id, statut, created_at DESC);

CREATE TABLE IF NOT EXISTS ent_formulaire_reponse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    formulaire_id UUID NOT NULL REFERENCES ent_formulaire (id),
    -- Recopiés de la version signée : l'historique reste lisible après archivage.
    objet TEXT,
    version INTEGER NOT NULL,
    eleve_id UUID NOT NULL,
    signataire_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- Qualité du signataire au moment de la décision : responsable ou élève majeur.
    qualite TEXT NOT NULL,
    decision TEXT NOT NULL,
    commentaire TEXT,
    -- Empreinte du texte signé : preuve de ce qui a été accepté ou refusé.
    empreinte TEXT NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_formulaire_reponse_decision_chk CHECK (decision IN ('accepte', 'refuse', 'retire')),
    CONSTRAINT ent_formulaire_reponse_qualite_chk CHECK (qualite IN ('responsable', 'eleve_majeur')),
    CONSTRAINT ent_formulaire_reponse_comm_chk CHECK (commentaire IS NULL OR length(commentaire) <= 1000)
);

CREATE INDEX IF NOT EXISTS ent_formulaire_reponse_eleve_idx
    ON ent_formulaire_reponse (etablissement_id, eleve_id, objet, decide_le DESC);
CREATE INDEX IF NOT EXISTS ent_formulaire_reponse_formulaire_idx
    ON ent_formulaire_reponse (etablissement_id, formulaire_id);
