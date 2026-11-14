-- Socle 2, lot S2-3 : dossiers d'admission / de réinscription (INS-02 → INS-05,
-- INS-07, INS-08, INS-13, INS-15 ; D-014).
--
-- Le dossier est la COPIE PROPRE à l'établissement de ce que la famille a saisi
-- (D-014) : rien d'autre que le formulaire envoyé. L'élève (`eleve_id`) n'existe
-- qu'à la validation (admission) ou dès la préparation (réinscription).
-- Responsables : rang 1 = déposant, rang 2 = second parent (e-mail non vérifié
-- au dépôt, invité à la validation). Parents séparés : e-mail obligatoire.
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_inscription_dossier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL REFERENCES ent_inscription_campagne (id),
    nature TEXT NOT NULL CHECK (nature IN ('admission', 'reinscription')),
    statut TEXT NOT NULL DEFAULT 'brouillon' CHECK (statut IN (
        'brouillon', 'deposee', 'en_instruction', 'incomplet', 'liste_attente',
        'admis', 'refuse', 'validee', 'desistement')),
    eleve_id UUID REFERENCES ent_eleve (id),
    eleve_nom TEXT NOT NULL CHECK (length(btrim(eleve_nom)) BETWEEN 1 AND 120),
    eleve_prenom TEXT NOT NULL CHECK (length(btrim(eleve_prenom)) BETWEEN 1 AND 120),
    eleve_date_naissance DATE,
    eleve_sexe TEXT CHECK (eleve_sexe IS NULL OR eleve_sexe IN ('F', 'M')),
    eleve_ine TEXT CHECK (eleve_ine IS NULL OR length(eleve_ine) <= 20),
    niveau_demande TEXT CHECK (niveau_demande IS NULL OR length(niveau_demande) <= 40),
    regime_demande TEXT CHECK (regime_demande IS NULL
        OR regime_demande IN ('externe', 'demi_pension', 'interne')),
    etablissement_origine TEXT CHECK (etablissement_origine IS NULL OR length(etablissement_origine) <= 200),
    classe_cible_id UUID REFERENCES ent_classe (id),
    -- E-mail VÉRIFIÉ du déposant (code ou passkey) ; compte s'il existe déjà.
    deposant_email TEXT CHECK (deposant_email IS NULL OR length(deposant_email) <= 254),
    deposant_user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    -- « Utiliser mes informations ? » accepté : seule condition de la reprise d'IBAN.
    reutilise_identite BOOLEAN NOT NULL DEFAULT false,
    decision_motif TEXT CHECK (decision_motif IS NULL OR length(decision_motif) <= 1000),
    decide_par UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    decide_le TIMESTAMPTZ,
    valide_par UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    valide_le TIMESTAMPTZ,
    depose_le TIMESTAMPTZ,
    derniere_relance_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_dossier_valide_chk
        CHECK (statut <> 'validee' OR (eleve_id IS NOT NULL AND valide_le IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS ent_inscription_dossier_campagne_idx
    ON ent_inscription_dossier (etablissement_id, campagne_id, statut);
CREATE INDEX IF NOT EXISTS ent_inscription_dossier_deposant_idx
    ON ent_inscription_dossier (etablissement_id, deposant_email);
-- Réinscription : un seul dossier par élève et par campagne.
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_dossier_eleve_uq
    ON ent_inscription_dossier (campagne_id, eleve_id) WHERE eleve_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS ent_inscription_responsable (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    dossier_id UUID NOT NULL REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    rang SMALLINT NOT NULL CHECK (rang IN (1, 2)),
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 120),
    prenom TEXT CHECK (prenom IS NULL OR length(prenom) <= 120),
    civilite TEXT CHECK (civilite IS NULL OR civilite IN ('mme', 'm', 'autre')),
    email TEXT CHECK (email IS NULL OR length(email) <= 254),
    telephone TEXT CHECK (telephone IS NULL OR length(telephone) <= 40),
    adresse TEXT CHECK (adresse IS NULL OR length(adresse) <= 500),
    lien TEXT NOT NULL DEFAULT 'autre' CHECK (lien IN ('mere', 'pere', 'tuteur', 'autre')),
    meme_foyer BOOLEAN NOT NULL DEFAULT true,
    autorite_parentale BOOLEAN NOT NULL DEFAULT true,
    -- Personne créée (ou retrouvée) dans l'établissement à la validation.
    personne_id UUID REFERENCES ent_personne (id) ON DELETE SET NULL,
    CONSTRAINT ent_inscription_responsable_rang_uq UNIQUE (dossier_id, rang),
    -- D-014 : parents séparés → un e-mail par parent obligatoire.
    CONSTRAINT ent_inscription_responsable_separes_chk
        CHECK (meme_foyer OR email IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS ent_inscription_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    dossier_id UUID NOT NULL REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    obligatoire BOOLEAN NOT NULL DEFAULT true,
    statut TEXT NOT NULL DEFAULT 'attendue'
        CHECK (statut IN ('attendue', 'deposee', 'acceptee', 'refusee')),
    nom_fichier TEXT CHECK (nom_fichier IS NULL OR length(nom_fichier) <= 200),
    motif_refus TEXT CHECK (motif_refus IS NULL OR length(motif_refus) <= 500),
    maj_par UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    maj_le TIMESTAMPTZ,
    CONSTRAINT ent_inscription_piece_uq UNIQUE (dossier_id, code),
    CONSTRAINT ent_inscription_piece_refus_chk CHECK (statut <> 'refusee' OR motif_refus IS NOT NULL)
);

-- Suivi du dossier : changements d'étape, commentaires, pièces, relances.
-- `visible_famille` faux = note interne de l'administration.
CREATE TABLE IF NOT EXISTS ent_inscription_suivi_dossier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    dossier_id UUID NOT NULL REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('statut', 'commentaire', 'piece', 'relance')),
    statut_avant TEXT,
    statut_apres TEXT,
    texte TEXT CHECK (texte IS NULL OR length(texte) <= 2000),
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    auteur_user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_inscription_suivi_dossier_idx
    ON ent_inscription_suivi_dossier (etablissement_id, dossier_id, created_at);
