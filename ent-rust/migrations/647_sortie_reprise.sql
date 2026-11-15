-- 647 — Socle 42, lot V0d (3/3) : reprise des séjours de Next.js (ombre en lecture, rapprochement des
-- participants, écarts, tableau de bascule, propriété unique). Réf. : 42_SORTIES_VOYAGES.md §26, §32.1 à
-- §32.4 ; VOY-258 à VOY-263 ; R-SV37, R-SV38, R-SV39, R-SV40, R-SV46.
--
-- Aucun accès à la base de production : la reprise charge un EXPORT fourni par Florian (anonymisé pour les
-- essais), jamais une connexion. Les tables ci-dessous sont la « zone de reprise » : rien n'y est un séjour
-- ScolIA tant que le séjour n'a pas basculé (la bascule crée alors `ent_sortie`).
--
-- Exigences E1 à E14 (PRA) : `etablissement_id` NOT NULL partout, clés étrangères COMPOSITES, UUID fournis par
-- l'application, aucun BYTEA, aucun JSON (D-036 : le dossier aplati de Next.js est converti en colonnes
-- typées, jamais conservé tel quel), horodatages, migration rejouable (aucune modification des migrations
-- 640 à 646), RLS posée en fin de fichier.
--
-- * Idempotence par `source_id` ; un séjour dont ScolIA est propriétaire n'est JAMAIS réécrit (R-SV37) : un
--   écart constaté après la bascule est signalé, pas appliqué.
-- * Nom, prénom et INE d'un export ne servent qu'au rapprochement : ils sont effacés dès que le participant
--   est rapproché (minimisation) ; aucun rapprochement approximatif automatique.

CREATE TABLE IF NOT EXISTS ent_sortie_reprise_sejour (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    source_id TEXT NOT NULL CHECK (length(btrim(source_id)) BETWEEN 1 AND 100),
    proprietaire TEXT NOT NULL DEFAULT 'nextjs' CHECK (proprietaire IN ('nextjs', 'scolia')),
    regime TEXT NOT NULL DEFAULT 'normal' CHECK (regime IN ('normal', 'en_vol', 'archive')),
    statut_source TEXT NOT NULL CHECK (statut_source IN (
        'BROUILLON', 'EN_ATTENTE_VALIDATION', 'EN_ATTENTE_DIR_INITIAL', 'PROF_LOGISTICS',
        'EN_ATTENTE_BUS_SIGNATURE', 'EN_ATTENTE_COMPTA', 'EN_ATTENTE_DIR_FINAL',
        'FINALISE_DIR_ATTENTE_ELEVES', 'VALIDE', 'BESOIN_MODIFICATION', 'REJETE', 'ANNULE',
        'SEANCE_ANNULEE')),
    -- État cible (§32.3) : calculé par le serveur à l'import, jamais saisi.
    etat_cible TEXT NOT NULL CHECK (etat_cible IN (
        'brouillon', 'soumise', 'a_completer', 'preparation', 'a_signer', 'budget_a_valider', 'a_autoriser',
        'ouverte', 'close', 'refusee', 'annulee')),
    -- Étape d'origine d'un dossier « à compléter » (`previousStatus` de Next.js), traduite en état ScolIA.
    etat_retour_cible TEXT CHECK (etat_retour_cible IS NULL OR etat_retour_cible IN (
        'soumise', 'preparation', 'a_signer', 'budget_a_valider', 'a_autoriser')),
    type_source TEXT NOT NULL CHECK (type_source IN ('SIMPLE', 'COMPLEX')),
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    nature TEXT NOT NULL DEFAULT 'facultative' CHECK (nature IN ('obligatoire', 'facultative')),
    forme TEXT NOT NULL CHECK (forme IN ('sans_nuitee', 'voyage')),
    besoin_car BOOLEAN NOT NULL DEFAULT false,
    classes_texte TEXT CHECK (classes_texte IS NULL OR length(classes_texte) <= 500),
    porteur_personne_id UUID,
    -- Montants repris en CENTIMES (jamais en flottant) ; la part annoncée est conservée telle quelle (R-SV46).
    budget_total_centimes BIGINT CHECK (budget_total_centimes IS NULL OR budget_total_centimes >= 0),
    part_annoncee_centimes BIGINT CHECK (part_annoncee_centimes IS NULL OR part_annoncee_centimes >= 0),
    pieces_attendues INTEGER NOT NULL DEFAULT 0 CHECK (pieces_attendues >= 0),
    -- Pièces copiées vers le dépôt du socle 7 avec contrôle d'empreinte.
    pieces_copiees INTEGER NOT NULL DEFAULT 0 CHECK (pieces_copiees >= 0),
    sortie_id UUID,
    derogation_motif TEXT CHECK (derogation_motif IS NULL OR length(btrim(derogation_motif)) BETWEEN 1 AND 1000),
    derogation_par_user_id UUID,
    basculee_le TIMESTAMPTZ,
    basculee_par_user_id UUID,
    importe_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    importe_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_reprise_sejour_dates_chk CHECK (fin > debut),
    CONSTRAINT ent_sortie_reprise_sejour_retour_chk CHECK ((etat_cible = 'a_completer') = (etat_retour_cible IS NOT NULL)),
    CONSTRAINT ent_sortie_reprise_sejour_pieces_chk CHECK (pieces_copiees <= pieces_attendues),
    CONSTRAINT ent_sortie_reprise_sejour_derog_chk CHECK ((derogation_motif IS NULL) = (derogation_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_reprise_sejour_bascule_chk CHECK (
        (proprietaire = 'scolia') = (basculee_le IS NOT NULL AND sortie_id IS NOT NULL
                                     AND basculee_par_user_id IS NOT NULL)),
    CONSTRAINT ent_sortie_reprise_sejour_source_uq UNIQUE (etablissement_id, source_id),
    CONSTRAINT ent_sortie_reprise_sejour_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_reprise_sejour_sortie_uq UNIQUE (sortie_id),
    CONSTRAINT ent_sortie_reprise_sejour_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_reprise_sejour_porteur_fk FOREIGN KEY (etablissement_id, porteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS ent_sortie_reprise_participant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sejour_id UUID NOT NULL,
    source_id TEXT NOT NULL CHECK (length(btrim(source_id)) BETWEEN 1 AND 100),
    -- Identité de l'export : nulle dès que le participant est rapproché (minimisation).
    nom_source TEXT CHECK (nom_source IS NULL OR length(nom_source) <= 100),
    prenom_source TEXT CHECK (prenom_source IS NULL OR length(prenom_source) <= 100),
    ine_source TEXT CHECK (ine_source IS NULL OR ine_source ~ '^[0-9A-Za-z]{11}$'),
    classe_source TEXT CHECK (classe_source IS NULL OR length(classe_source) <= 50),
    eleve_id UUID,
    statut TEXT NOT NULL DEFAULT 'a_rapprocher' CHECK (statut IN ('a_rapprocher', 'rapproche', 'exclu')),
    methode TEXT CHECK (methode IS NULL OR methode IN ('eleve_id', 'ine', 'humain')),
    rapproche_le TIMESTAMPTZ,
    rapproche_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_reprise_participant_statut_chk CHECK (
        (statut = 'rapproche') = (eleve_id IS NOT NULL AND methode IS NOT NULL AND rapproche_le IS NOT NULL)),
    CONSTRAINT ent_sortie_reprise_participant_minimisation_chk CHECK (
        statut <> 'rapproche' OR (nom_source IS NULL AND prenom_source IS NULL AND ine_source IS NULL)),
    CONSTRAINT ent_sortie_reprise_participant_source_uq UNIQUE (sejour_id, source_id),
    CONSTRAINT ent_sortie_reprise_participant_eleve_uq UNIQUE (sejour_id, eleve_id),
    CONSTRAINT ent_sortie_reprise_participant_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_reprise_participant_sejour_fk FOREIGN KEY (etablissement_id, sejour_id)
        REFERENCES ent_sortie_reprise_sejour (etablissement_id, id),
    CONSTRAINT ent_sortie_reprise_participant_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);

-- Écarts à traiter avant la bascule (rapport de reprise). Texte court, jamais de donnée personnelle.
CREATE TABLE IF NOT EXISTS ent_sortie_reprise_ecart (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sejour_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN (
        'montant_douteux', 'piece_manquante', 'statut_herite', 'aide_nominative_retiree',
        'accompagnant_a_rapprocher', 'part_annoncee_differente', 'ecart_apres_bascule', 'porteur_a_designer')),
    detail TEXT NOT NULL DEFAULT '' CHECK (length(detail) <= 300),
    -- Un écart se « traite » (décision humaine tracée) ou se lève ; il ne se supprime pas.
    traite_le TIMESTAMPTZ,
    traite_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_reprise_ecart_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_reprise_ecart_traite_chk CHECK ((traite_le IS NULL) = (traite_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_reprise_ecart_uq UNIQUE (sejour_id, code, detail),
    CONSTRAINT ent_sortie_reprise_ecart_sejour_fk FOREIGN KEY (etablissement_id, sejour_id)
        REFERENCES ent_sortie_reprise_sejour (etablissement_id, id)
);

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_reprise_sejour',
    'ent_sortie_reprise_participant',
    'ent_sortie_reprise_ecart'
]);
