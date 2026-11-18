-- Socle 21, lot V1 (suite) — 466 : demandes de correction et de révision, journal des consultations
-- (R-V26, ajout seul), enseignements suivis hors de l'établissement et leurs moyennes externes
-- (R-V38), résultats externes ponctuels (CE-02). Additif, rejouable. Aucune colonne JSON.

CREATE TABLE IF NOT EXISTS eva_correction_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    resultat_id UUID,
    appreciation_id UUID,
    demandeur UUID NOT NULL,
    motif TEXT NOT NULL CHECK (char_length(btrim(motif)) BETWEEN 3 AND 1000),
    etat TEXT NOT NULL DEFAULT 'en_attente' CHECK (etat IN ('en_attente', 'acceptee', 'refusee', 'retiree')),
    decide_par UUID,
    decide_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_correction_demande_cible_chk CHECK ((resultat_id IS NULL) <> (appreciation_id IS NULL)),
    CONSTRAINT eva_correction_demande_decision_chk CHECK ((etat IN ('acceptee', 'refusee')) = (decide_par IS NOT NULL AND decide_le IS NOT NULL)),
    CONSTRAINT eva_correction_demande_resultat_fk FOREIGN KEY (resultat_id, etablissement_id)
        REFERENCES eva_resultat (id, etablissement_id),
    CONSTRAINT eva_correction_demande_appreciation_fk FOREIGN KEY (appreciation_id, etablissement_id)
        REFERENCES eva_appreciation (id, etablissement_id)
);

-- Demande de révision d'une note par une famille ou l'élève (CE-59) : pas un recours juridique.
CREATE TABLE IF NOT EXISTS eva_revision_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    resultat_id UUID NOT NULL,
    demandeur UUID NOT NULL,
    message TEXT NOT NULL CHECK (char_length(btrim(message)) BETWEEN 3 AND 1000),
    reponse TEXT CHECK (reponse IS NULL OR char_length(btrim(reponse)) BETWEEN 1 AND 1000),
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('ouverte', 'repondue', 'retiree')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    repondue_le TIMESTAMPTZ,
    CONSTRAINT eva_revision_reponse_chk CHECK ((etat = 'repondue') = (reponse IS NOT NULL)),
    CONSTRAINT eva_revision_resultat_fk FOREIGN KEY (resultat_id, etablissement_id)
        REFERENCES eva_resultat (id, etablissement_id)
);

-- Journal des consultations (R-V26, R-V46) : qui a ouvert quoi. Ajout seul ; un parent ne voit jamais
-- l'activité de l'autre (la lecture est réservée à la direction, côté service).
CREATE TABLE IF NOT EXISTS eva_consultation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (objet IN ('releve', 'bulletin', 'dossier_conseil', 'instantane', 'resultat')),
    consulte_par UUID NOT NULL,
    role TEXT NOT NULL CHECK (char_length(role) BETWEEN 1 AND 40),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_consultation_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_consultation_idx ON eva_consultation (etablissement_id, eleve_id, le);
DROP TRIGGER IF EXISTS eva_consultation_ajout_seul ON eva_consultation;
CREATE TRIGGER eva_consultation_ajout_seul BEFORE UPDATE OR DELETE ON eva_consultation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- ── Résultats externes ponctuels (notes d'un établissement d'origine, CNED, étranger, reprise) ──
-- Ne comptent PAS par défaut ; un professeur peut décider de les compter dans un cours (geste tracé).
CREATE TABLE IF NOT EXISTS eva_resultat_externe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    matiere_id UUID NOT NULL,
    periode_id UUID,
    annee_libelle TEXT CHECK (annee_libelle IS NULL OR char_length(annee_libelle) <= 20),
    origine TEXT NOT NULL CHECK (origine IN ('etablissement', 'cned', 'etranger', 'reprise', 'autre')),
    valeur NUMERIC(7, 3) NOT NULL CHECK (valeur >= 0),
    bareme NUMERIC(7, 3) NOT NULL CHECK (bareme > 0 AND bareme <= 1000),
    coef NUMERIC(6, 3) NOT NULL DEFAULT 1 CHECK (coef >= 0 AND coef <= 100),
    compte_dans_cours_id UUID,
    -- Pièce justificative : un document du socle 7 (sans clé ici, la table n'est pas de ce socle).
    document_id UUID,
    saisi_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_resultat_externe_borne_chk CHECK (valeur <= bareme),
    CONSTRAINT eva_resultat_externe_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_resultat_externe_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_resultat_externe_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_resultat_externe_cours_fk FOREIGN KEY (compte_dans_cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id)
);

-- ── Enseignement suivi hors de l'établissement et moyenne externe (R-V38, Q-EV18) ───────────────
CREATE TABLE IF NOT EXISTS eva_enseignement_externe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    matiere_id UUID NOT NULL,
    organisme TEXT NOT NULL CHECK (organisme IN ('cned', 'etablissement', 'conservatoire', 'etranger', 'autre')),
    organisme_libelle TEXT CHECK (organisme_libelle IS NULL OR char_length(organisme_libelle) <= 160),
    du DATE NOT NULL,
    au DATE,
    referent TEXT CHECK (referent IS NULL OR char_length(referent) <= 160),
    -- Déclaré par la direction : la moyenne externe vaut moyenne de matière (R-V38).
    compte_moyenne_generale BOOLEAN NOT NULL DEFAULT false,
    declare_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_enseignement_externe_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT eva_enseignement_externe_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_enseignement_externe_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_enseignement_externe_id_etab_uq UNIQUE (id, etablissement_id)
);

CREATE TABLE IF NOT EXISTS eva_moyenne_externe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    enseignement_externe_id UUID NOT NULL,
    periode_id UUID NOT NULL,
    valeur NUMERIC(7, 3) CHECK (valeur IS NULL OR valeur >= 0),
    bareme NUMERIC(7, 3) NOT NULL DEFAULT 20 CHECK (bareme > 0 AND bareme <= 1000),
    appreciation_transcrite TEXT CHECK (appreciation_transcrite IS NULL OR char_length(appreciation_transcrite) <= 600),
    statut TEXT NOT NULL CHECK (statut IN ('moyenne', 'en_attente', 'dispense')),
    document_id UUID,
    saisi_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_moyenne_externe_valeur_chk CHECK ((statut = 'moyenne') = (valeur IS NOT NULL)),
    CONSTRAINT eva_moyenne_externe_borne_chk CHECK (valeur IS NULL OR valeur <= bareme),
    CONSTRAINT eva_moyenne_externe_ens_fk FOREIGN KEY (enseignement_externe_id, etablissement_id)
        REFERENCES eva_enseignement_externe (id, etablissement_id),
    CONSTRAINT eva_moyenne_externe_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_moyenne_externe_uq UNIQUE (enseignement_externe_id, periode_id)
);

SELECT ent_rls_poser(ARRAY[
    'eva_correction_demande', 'eva_revision_demande', 'eva_consultation', 'eva_resultat_externe',
    'eva_enseignement_externe', 'eva_moyenne_externe'
]);
