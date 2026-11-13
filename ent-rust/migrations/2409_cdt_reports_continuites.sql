-- Socle 20, lot C1 — migration 2409 : propositions de report (quand la séance change) et continuités pédagogiques.
-- Réf. : 20_CAHIER_DE_TEXTES.md §8.4, §12, R-C3, R-C4. (2407 : exercices, 2408 : progressions — lots C9 et C10, non écrites ici.)
-- Les séances référencées ne sont jamais supprimées (RESTRICT) : un report se propose, il ne perd rien et ne déplace rien en silence.

CREATE TABLE IF NOT EXISTS ent_cdt_report (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    origine_seance_id UUID NOT NULL,
    -- Événement de séance du socle 10 à l'origine du changement (annulation, déplacement, remplacement…).
    evenement_id UUID,
    objet TEXT NOT NULL CHECK (objet IN ('entree', 'travail')),
    objet_id UUID NOT NULL,
    vers_seance_id UUID,
    etat TEXT NOT NULL DEFAULT 'proposee' CHECK (etat IN ('proposee', 'appliquee', 'refusee', 'appliquee_auto')),
    traite_par_user_id UUID,
    traite_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_report_traite_chk CHECK ((etat = 'proposee') = (traite_le IS NULL)),
    CONSTRAINT ent_cdt_report_origine_fk FOREIGN KEY (etablissement_id, origine_seance_id)
        REFERENCES ent_edt_seance (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_report_vers_fk FOREIGN KEY (etablissement_id, vers_seance_id)
        REFERENCES ent_edt_seance (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_report_ouvert_idx ON ent_cdt_report (etablissement_id, etat, cree_le) WHERE etat = 'proposee';

-- Continuité d'un élève absent longtemps (hospitalisation, exclusion temporaire…) : SANS motif (minimisation).
CREATE TABLE IF NOT EXISTS ent_cdt_continuite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE,
    ouverte_par_user_id UUID,
    close_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_continuite_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_continuite_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_cdt_continuite_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_cdt_continuite_cours (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    continuite_id UUID NOT NULL,
    cours_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (continuite_id, cours_id),
    CONSTRAINT ent_cdt_continuite_cours_continuite_fk FOREIGN KEY (etablissement_id, continuite_id)
        REFERENCES ent_cdt_continuite (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_continuite_cours_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_report', 'ent_cdt_continuite', 'ent_cdt_continuite_cours']);
