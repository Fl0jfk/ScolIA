-- 640 — Socle 42 (sorties et voyages), lot V0a, étape (a) : cœur de la sortie.
-- Réf. : docs/socles/42_SORTIES_VOYAGES.md §8, §25, §32.6 (D-036 : aucune donnée métier en JSON).
-- Séries hebdomadaires, sortie, public visé, étapes, journal en AJOUT SEUL (colonnes texte :
-- plus aucune exception jsonb, contrairement à ce que prévoyait le §32.6).
-- Clés composites (etablissement_id, id) pour le cloisonnement (R-SV14) ; RLS en fin de fichier
-- (ent_rls_poser, définie en 580). Idempotent (rejoué à chaque démarrage).

CREATE TABLE IF NOT EXISTS ent_sortie_serie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    jour_semaine SMALLINT NOT NULL CHECK (jour_semaine BETWEEN 1 AND 7),
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    du DATE NOT NULL,
    au DATE NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_serie_heures_chk CHECK (heure_fin > heure_debut),
    CONSTRAINT ent_sortie_serie_dates_chk CHECK (au >= du),
    CONSTRAINT ent_sortie_serie_etab_id_uq UNIQUE (etablissement_id, id)
);

-- Dates écartées d'une série : fériés et vacances calculés à la création, exclusions manuelles.
CREATE TABLE IF NOT EXISTS ent_sortie_serie_exclusion (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    serie_id UUID NOT NULL,
    jour DATE NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN ('ferie', 'vacances', 'ferme', 'manuel')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (serie_id, jour),
    CONSTRAINT ent_sortie_serie_exclusion_fk FOREIGN KEY (etablissement_id, serie_id)
        REFERENCES ent_sortie_serie (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS ent_sortie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    serie_id UUID,
    reference TEXT NOT NULL CHECK (length(reference) BETWEEN 6 AND 40),
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    objectifs TEXT CHECK (objectifs IS NULL OR length(objectifs) <= 4000),
    nature TEXT NOT NULL CHECK (nature IN ('obligatoire', 'facultative')),
    forme TEXT NOT NULL CHECK (forme IN ('sans_nuitee', 'voyage')),
    destination TEXT NOT NULL CHECK (destination IN ('proximite', 'france', 'frontalier', 'etranger')),
    pays TEXT CHECK (pays IS NULL OR length(btrim(pays)) BETWEEN 2 AND 80),
    destination_libelle TEXT CHECK (destination_libelle IS NULL OR length(destination_libelle) <= 300),
    programme_libelle TEXT CHECK (programme_libelle IS NULL OR length(programme_libelle) <= 4000),
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    porteur_personne_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN (
        'brouillon', 'soumise', 'a_completer', 'preparation', 'a_signer', 'budget_a_valider',
        'vote', 'a_autoriser', 'ouverte', 'prete', 'en_cours', 'rentree', 'close', 'refusee', 'annulee')),
    -- Étape d'origine mémorisée quand la direction demande des modifications (NX-05).
    etat_retour TEXT CHECK (etat_retour IS NULL OR etat_retour IN (
        'soumise', 'preparation', 'a_signer', 'budget_a_valider', 'a_autoriser')),
    motif_decision TEXT CHECK (motif_decision IS NULL OR length(motif_decision) <= 2000),
    effectif_mode TEXT NOT NULL DEFAULT 'maximum' CHECK (effectif_mode IN ('maximum', 'exact')),
    places INTEGER CHECK (places IS NULL OR places > 0),
    effectif_eleves_declare INTEGER CHECK (effectif_eleves_declare IS NULL OR effectif_eleves_declare >= 0),
    effectif_adultes_declare INTEGER CHECK (effectif_adultes_declare IS NULL OR effectif_adultes_declare >= 0),
    besoin_car BOOLEAN NOT NULL DEFAULT false,
    car_sur_place BOOLEAN NOT NULL DEFAULT false,
    point_rendez_vous TEXT CHECK (point_rendez_vous IS NULL OR length(point_rendez_vous) <= 300),
    consignes_transport TEXT CHECK (consignes_transport IS NULL OR length(consignes_transport) <= 2000),
    places_encadrant_a_pourvoir INTEGER NOT NULL DEFAULT 0 CHECK (places_encadrant_a_pourvoir >= 0),
    -- NX-07 : passage aux finances sans devis signé, motif obligatoire (V0c lèvera la nécessité
    -- quand un devis signé existe).
    contournement_devis_motif TEXT
        CHECK (contournement_devis_motif IS NULL OR length(btrim(contournement_devis_motif)) BETWEEN 1 AND 1000),
    liste_confirmee_le TIMESTAMPTZ,
    liste_confirmee_par_user_id UUID,
    version INTEGER NOT NULL DEFAULT 1,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_dates_chk CHECK (fin > debut),
    CONSTRAINT ent_sortie_etranger_pays_chk CHECK (destination <> 'etranger' OR pays IS NOT NULL),
    CONSTRAINT ent_sortie_retour_chk CHECK ((etat = 'a_completer') = (etat_retour IS NOT NULL)),
    CONSTRAINT ent_sortie_liste_chk CHECK ((liste_confirmee_le IS NULL) = (liste_confirmee_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_reference_uq UNIQUE (etablissement_id, reference),
    CONSTRAINT ent_sortie_serie_fk FOREIGN KEY (etablissement_id, serie_id)
        REFERENCES ent_sortie_serie (etablissement_id, id),
    CONSTRAINT ent_sortie_porteur_fk FOREIGN KEY (etablissement_id, porteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
-- Une séance de série par date de début.
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_serie_seance_uq
    ON ent_sortie (serie_id, debut) WHERE serie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_sortie_etab_debut_idx ON ent_sortie (etablissement_id, debut);
CREATE INDEX IF NOT EXISTS ent_sortie_porteur_idx ON ent_sortie (etablissement_id, porteur_personne_id);
CREATE INDEX IF NOT EXISTS ent_sortie_etat_idx ON ent_sortie (etablissement_id, etat);

-- Public visé : classes ou groupes (la sélection libre d'élèves passe par les participants).
CREATE TABLE IF NOT EXISTS ent_sortie_public (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    classe_id UUID REFERENCES ent_classe (id),
    groupe_id UUID REFERENCES ent_groupe (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_public_cible_chk CHECK ((classe_id IS NULL) <> (groupe_id IS NULL)),
    CONSTRAINT ent_sortie_public_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_public_classe_uq
    ON ent_sortie_public (sortie_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_public_groupe_uq
    ON ent_sortie_public (sortie_id, groupe_id) WHERE groupe_id IS NOT NULL;

-- Étapes du programme (calendrier des familles, base des appels d'étape).
CREATE TABLE IF NOT EXISTS ent_sortie_etape (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre BETWEEN 1 AND 200),
    type TEXT NOT NULL CHECK (type IN
        ('depart', 'depot', 'reprise', 'visite', 'hebergement', 'retour', 'reunion', 'autre')),
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ,
    duree_minutes INTEGER CHECK (duree_minutes IS NULL OR duree_minutes > 0),
    lieu TEXT CHECK (lieu IS NULL OR length(lieu) <= 300),
    note TEXT CHECK (note IS NULL OR length(note) <= 1000),
    appel_requis BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_etape_fin_chk CHECK (fin IS NULL OR fin >= debut),
    CONSTRAINT ent_sortie_etape_ordre_uq UNIQUE (sortie_id, ordre),
    CONSTRAINT ent_sortie_etape_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);

-- Journal du séjour, AJOUT SEUL (R-SV1) : qui, quand, quoi, d'où à où. Texte uniquement.
CREATE TABLE IF NOT EXISTS ent_sortie_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    a TIMESTAMPTZ NOT NULL DEFAULT now(),
    acteur_user_id UUID,
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 60),
    etat_avant TEXT,
    etat_apres TEXT,
    champ TEXT CHECK (champ IS NULL OR length(champ) <= 60),
    avant TEXT CHECK (avant IS NULL OR length(avant) <= 2000),
    apres TEXT CHECK (apres IS NULL OR length(apres) <= 2000),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 2000),
    CONSTRAINT ent_sortie_journal_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_journal_sortie_idx
    ON ent_sortie_journal (etablissement_id, sortie_id, a);

CREATE OR REPLACE FUNCTION ent_sortie_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_journal_immuable ON ent_sortie_journal;
CREATE TRIGGER ent_sortie_journal_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_journal
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_serie',
    'ent_sortie_serie_exclusion',
    'ent_sortie',
    'ent_sortie_public',
    'ent_sortie_etape',
    'ent_sortie_journal'
]);
