-- Socle 20, lot C1 — migration 2406 : retours (commentaire, grille), versions en ajout seul, signalements de copie,
-- consentements « montrer comme exemple ». Réf. : 20_CAHIER_DE_TEXTES.md §24.1, §10.3, §10.4, R-C13, R-C20.
-- La note n'est JAMAIS saisie ici : `resultat_id` pointe le résultat du socle 21 (sans clé : table d'un autre socle) ;
-- `note_id` reprend le lien historique vers `ent_note` (S2-6).

CREATE TABLE IF NOT EXISTS ent_cdt_retour (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    remise_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'rendu')),
    commentaire TEXT NOT NULL DEFAULT '' CHECK (length(commentaire) <= 4000),
    resultat_id UUID,
    note_id UUID,
    montrer_note BOOLEAN NOT NULL DEFAULT false,
    rendu_le TIMESTAMPTZ,
    auteur_user_id UUID REFERENCES ent_user (id),
    auteur_agent_id UUID,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    ancien_id UUID UNIQUE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_retour_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_retour_remise_uq UNIQUE (etablissement_id, remise_id),
    CONSTRAINT ent_cdt_retour_rendu_chk CHECK ((etat = 'rendu') = (rendu_le IS NOT NULL)),
    CONSTRAINT ent_cdt_retour_remise_fk FOREIGN KEY (etablissement_id, remise_id)
        REFERENCES ent_cdt_remise (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_retour_note_fk FOREIGN KEY (note_id, etablissement_id)
        REFERENCES ent_note (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_retour_agent_fk FOREIGN KEY (etablissement_id, auteur_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_cdt_retour_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    retour_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    commentaire TEXT NOT NULL,
    note_id UUID,
    rendu_le TIMESTAMPTZ,
    auteur_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_retour_version_uq UNIQUE (retour_id, version),
    CONSTRAINT ent_cdt_retour_version_fk FOREIGN KEY (etablissement_id, retour_id)
        REFERENCES ent_cdt_retour (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_retour_version_ajout_seul ON ent_cdt_retour_version;
CREATE TRIGGER ent_cdt_retour_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_retour_version
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Grille facultative : critères du professeur ou compétences du socle 22 (sans clé : autre socle).
CREATE TABLE IF NOT EXISTS ent_cdt_retour_critere (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    retour_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre >= 1),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    niveau TEXT CHECK (niveau IS NULL OR niveau IN ('non_atteint', 'partiel', 'atteint', 'depasse')),
    competence_item_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (retour_id, ordre),
    CONSTRAINT ent_cdt_retour_critere_fk FOREIGN KEY (etablissement_id, retour_id)
        REFERENCES ent_cdt_retour (etablissement_id, id) ON DELETE RESTRICT
);

-- Contenu préoccupant dans une copie : signalement à la CPE, à la direction ou à la protection de l'enfance.
CREATE TABLE IF NOT EXISTS ent_cdt_signalement_copie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    remise_id UUID NOT NULL,
    vers TEXT NOT NULL CHECK (vers IN ('cpe', 'direction', 'protection')),
    dossier_ref UUID,
    par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_signalement_copie_remise_fk FOREIGN KEY (etablissement_id, remise_id)
        REFERENCES ent_cdt_remise (etablissement_id, id) ON DELETE RESTRICT
);

-- « Montrer cette copie comme exemple » : consentement explicite de l'élève ET, pour un mineur, de ses responsables.
CREATE TABLE IF NOT EXISTS ent_cdt_consentement_exemple (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    remise_id UUID NOT NULL,
    demande_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    eleve_reponse TEXT CHECK (eleve_reponse IS NULL OR eleve_reponse IN ('accepte', 'refuse')),
    responsable_reponse TEXT CHECK (responsable_reponse IS NULL OR responsable_reponse IN ('accepte', 'refuse')),
    anonymise BOOLEAN NOT NULL DEFAULT true,
    etat TEXT NOT NULL DEFAULT 'demande' CHECK (etat IN ('demande', 'accorde', 'refuse', 'retire')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_consentement_exemple_remise_fk FOREIGN KEY (etablissement_id, remise_id)
        REFERENCES ent_cdt_remise (etablissement_id, id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_retour', 'ent_cdt_retour_version', 'ent_cdt_retour_critere',
                           'ent_cdt_signalement_copie', 'ent_cdt_consentement_exemple']);
