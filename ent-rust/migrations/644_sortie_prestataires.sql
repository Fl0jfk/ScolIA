-- 644 — Socle 42, lot V0c : annuaire des prestataires, demandes de devis, devis, signature, bons de
-- commande, avenants, boîte de réception des devis et file de tri, envois sortants.
-- Réf. : 42_SORTIES_VOYAGES.md §13, §19.4 (VOY-155 à VOY-168), NX-41 à NX-50, NX-70 à NX-74 ; corrige
-- NJ-07 à NJ-13, NJ-27, NJ-28, NJ-55 (docs/ALERTES_NEXTJS_PROD.md).
--
-- Exigences E1 à E14 (PRA) : `etablissement_id` NOT NULL partout, clés étrangères COMPOSITES, UUID
-- fournis par l'application, aucun BYTEA (les fichiers vont au magasin d'objets), aucun JSON (D-036),
-- horodatages, tables d'ajout seul pour les faits (signature, avenants, courrier trié),
-- migration rejouable (aucune modification des migrations 640 à 643), RLS posée en fin de fichier.
--
-- * Une adresse de prestataire n'entre dans l'annuaire que par un humain (`verifiee_par_user_id`).
-- * Un envoi ne vise qu'une adresse DE CE prestataire (clé étrangère composite) : jamais d'adresse
--   fournie par le navigateur (corrige NJ-11).
-- * La signature d'un devis scelle l'empreinte de la pièce : elle ne se remplace pas (NJ-13).

CREATE TABLE IF NOT EXISTS ent_prestataire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    raison_sociale TEXT NOT NULL CHECK (length(btrim(raison_sociale)) BETWEEN 1 AND 200),
    siren TEXT CHECK (siren IS NULL OR siren ~ '^[0-9]{9}$'),
    telephone TEXT CHECK (telephone IS NULL OR length(telephone) BETWEEN 3 AND 30),
    actif BOOLEAN NOT NULL DEFAULT true,
    version INTEGER NOT NULL DEFAULT 1,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_prestataire_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_prestataire_nom_uq
    ON ent_prestataire (etablissement_id, lower(btrim(raison_sociale)));

CREATE TABLE IF NOT EXISTS ent_prestataire_categorie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    prestataire_id UUID NOT NULL,
    categorie TEXT NOT NULL CHECK (categorie IN ('transporteur', 'hebergeur', 'agence', 'lieu', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_prestataire_categorie_uq UNIQUE (prestataire_id, categorie),
    CONSTRAINT ent_prestataire_categorie_fk FOREIGN KEY (etablissement_id, prestataire_id)
        REFERENCES ent_prestataire (etablissement_id, id)
);

-- Adresses vérifiées par un HUMAIN (VD-17 : jamais lues dans un PDF par une IA).
CREATE TABLE IF NOT EXISTS ent_prestataire_adresse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    prestataire_id UUID NOT NULL,
    email TEXT NOT NULL CHECK (
        length(email) BETWEEN 6 AND 254 AND email = lower(btrim(email))
        AND strpos(email, chr(59)) = 0
        AND email ~ '^[^@[:space:]<>,"]+@[^@[:space:]<>,"]+\.[^@[:space:]<>,"]+$'),
    domaine TEXT NOT NULL CHECK (domaine = split_part(email, '@', 2)),
    verifiee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    verifiee_par_user_id UUID NOT NULL,
    desactivee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_prestataire_adresse_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_prestataire_adresse_presta_uq UNIQUE (etablissement_id, prestataire_id, id),
    CONSTRAINT ent_prestataire_adresse_fk FOREIGN KEY (etablissement_id, prestataire_id)
        REFERENCES ent_prestataire (etablissement_id, id)
);
-- Une adresse active appartient à un seul prestataire de l'établissement (rattachement univoque).
CREATE UNIQUE INDEX IF NOT EXISTS ent_prestataire_adresse_active_uq
    ON ent_prestataire_adresse (etablissement_id, email) WHERE desactivee_le IS NULL;

-- Réglages typés de l'établissement pour les sorties (corrige NJ-12 : la boîte comptabilité est celle
-- de l'établissement, jamais une valeur codée en dur).
CREATE TABLE IF NOT EXISTS ent_sortie_reglage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    boite_comptabilite_email TEXT CHECK (
        boite_comptabilite_email IS NULL
        OR (length(boite_comptabilite_email) BETWEEN 6 AND 254
            AND boite_comptabilite_email = lower(btrim(boite_comptabilite_email))
            AND strpos(boite_comptabilite_email, chr(59)) = 0
            AND boite_comptabilite_email ~ '^[^@[:space:]<>,"]+@[^@[:space:]<>,"]+\.[^@[:space:]<>,"]+$')),
    pied_de_page TEXT CHECK (pied_de_page IS NULL OR length(pied_de_page) <= 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_user_id UUID,
    CONSTRAINT ent_sortie_reglage_etab_uq UNIQUE (etablissement_id)
);

-- Les documents émis ou reçus de V0c s'ajoutent aux types de V0b (budget, circulaire).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_document_type_v0c_chk') THEN
        ALTER TABLE ent_sortie_document DROP CONSTRAINT IF EXISTS ent_sortie_document_type_check;
        ALTER TABLE ent_sortie_document ADD CONSTRAINT ent_sortie_document_type_v0c_chk CHECK (
            type IN ('budget', 'circulaire', 'demande_devis', 'devis', 'bon_commande', 'avenant'));
    END IF;
    -- Clé composite (E2) : les envois, devis et avenants référencent un document de LEUR établissement.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_document_etab_id_uq') THEN
        ALTER TABLE ent_sortie_document ADD CONSTRAINT ent_sortie_document_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END
$$;

-- Courrier entrant par établissement (§13.3) : une adresse par établissement, jamais de devinette
-- entre établissements. Un message n'est JAMAIS « traité » sans rattachement confirmé (VD-45).
CREATE TABLE IF NOT EXISTS ent_courrier_entrant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Identifiant du message côté serveur de courrier : rend le dépôt idempotent.
    message_externe_id TEXT NOT NULL CHECK (length(message_externe_id) BETWEEN 1 AND 300),
    recu_le TIMESTAMPTZ NOT NULL,
    expediteur_email TEXT NOT NULL CHECK (length(expediteur_email) BETWEEN 3 AND 254),
    expediteur_domaine TEXT NOT NULL CHECK (length(expediteur_domaine) BETWEEN 1 AND 253),
    objet TEXT NOT NULL CHECK (length(objet) <= 300),
    extrait TEXT NOT NULL DEFAULT '' CHECK (length(extrait) <= 2000),
    pieces_refusees INTEGER NOT NULL DEFAULT 0 CHECK (pieces_refusees >= 0),
    statut TEXT NOT NULL DEFAULT 'a_trier' CHECK (statut IN ('a_trier', 'rattache', 'ignore')),
    sortie_id UUID,
    prestataire_id UUID,
    mode TEXT CHECK (mode IS NULL OR mode IN ('auto', 'humain')),
    raison TEXT CHECK (raison IS NULL OR raison IN (
        'reference_et_expediteur', 'reference_seule', 'expediteur_seul', 'ni_reference_ni_expediteur',
        'expediteur_inconnu', 'reference_inconnue', 'tri_humain')),
    -- Domaine voisin d'un prestataire connu (hameçonnage) : alerte à l'écran de tri.
    alerte_domaine_voisin BOOLEAN NOT NULL DEFAULT false,
    trie_par_user_id UUID,
    trie_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_courrier_entrant_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_courrier_entrant_message_uq UNIQUE (etablissement_id, message_externe_id),
    CONSTRAINT ent_courrier_entrant_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_courrier_entrant_presta_fk FOREIGN KEY (etablissement_id, prestataire_id)
        REFERENCES ent_prestataire (etablissement_id, id),
    CONSTRAINT ent_courrier_entrant_rattache_chk CHECK (
        (statut = 'rattache') = (sortie_id IS NOT NULL AND mode IS NOT NULL)),
    -- « ignoré » est un geste humain tracé, jamais un état par défaut.
    CONSTRAINT ent_courrier_entrant_ignore_chk CHECK (
        statut <> 'ignore' OR (trie_par_user_id IS NOT NULL AND trie_le IS NOT NULL)),
    CONSTRAINT ent_courrier_entrant_humain_chk CHECK (
        mode IS DISTINCT FROM 'humain' OR (trie_par_user_id IS NOT NULL AND trie_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_courrier_entrant_tri_idx
    ON ent_courrier_entrant (etablissement_id, statut, recu_le);

CREATE TABLE IF NOT EXISTS ent_courrier_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    courrier_id UUID NOT NULL,
    nom TEXT NOT NULL CHECK (length(nom) BETWEEN 1 AND 200),
    mime TEXT NOT NULL CHECK (length(mime) BETWEEN 3 AND 100),
    taille INTEGER NOT NULL CHECK (taille > 0),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    cle_objet TEXT NOT NULL CHECK (length(cle_objet) BETWEEN 1 AND 300),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_courrier_piece_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_courrier_piece_fk FOREIGN KEY (etablissement_id, courrier_id)
        REFERENCES ent_courrier_entrant (etablissement_id, id)
);

-- Devis reçus (saisis à la main ou issus du courrier). Le montant n'entre au budget que CONFIRMÉ par
-- un humain ; le montant « proposé » par la lecture du texte n'est jamais « le plus grand nombre ».
CREATE TABLE IF NOT EXISTS ent_sortie_devis (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    prestataire_id UUID NOT NULL,
    origine TEXT NOT NULL CHECK (origine IN ('manuel', 'courrier')),
    courrier_id UUID,
    document_id UUID,
    reference TEXT CHECK (reference IS NULL OR length(btrim(reference)) BETWEEN 1 AND 80),
    montant_centimes BIGINT CHECK (montant_centimes IS NULL OR montant_centimes BETWEEN 0 AND 100000000000),
    montant_propose_centimes BIGINT
        CHECK (montant_propose_centimes IS NULL OR montant_propose_centimes BETWEEN 0 AND 100000000000),
    montant_confirme_le TIMESTAMPTZ,
    montant_confirme_par_user_id UUID,
    valide_jusqu_au DATE,
    inclus TEXT CHECK (inclus IS NULL OR length(inclus) <= 2000),
    conditions_annulation TEXT CHECK (conditions_annulation IS NULL OR length(conditions_annulation) <= 2000),
    statut TEXT NOT NULL DEFAULT 'a_confirmer' CHECK (statut IN (
        'a_confirmer', 'recu', 'choisi', 'ecarte', 'signe', 'apres_commande')),
    -- Compteur d'écritures : une décision (confirmer, choisir, signer) porte sur ce qui a été lu.
    rev INTEGER NOT NULL DEFAULT 1 CHECK (rev > 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_devis_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_devis_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_devis_presta_fk FOREIGN KEY (etablissement_id, prestataire_id)
        REFERENCES ent_prestataire (etablissement_id, id),
    CONSTRAINT ent_sortie_devis_courrier_fk FOREIGN KEY (etablissement_id, courrier_id)
        REFERENCES ent_courrier_entrant (etablissement_id, id),
    CONSTRAINT ent_sortie_devis_document_fk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_sortie_document (etablissement_id, id),
    CONSTRAINT ent_sortie_devis_courrier_chk CHECK ((origine = 'courrier') = (courrier_id IS NOT NULL)),
    CONSTRAINT ent_sortie_devis_confirme_chk CHECK (
        (montant_centimes IS NULL) = (montant_confirme_le IS NULL)
        AND (montant_confirme_le IS NULL) = (montant_confirme_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_devis_statut_chk CHECK (
        statut IN ('a_confirmer', 'ecarte', 'apres_commande') OR montant_centimes IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_sortie_devis_sortie_idx
    ON ent_sortie_devis (etablissement_id, sortie_id, statut);
-- Un seul devis choisi ou signé à la fois par sortie.
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_devis_choisi_uq
    ON ent_sortie_devis (sortie_id) WHERE statut IN ('choisi', 'signe');

-- Signature du devis (socle 7). Niveau S1 en V0c : session authentifiée, intention exprimée sur
-- l'EMPREINTE de la pièce, horodatage. Ajout seul : la signature ne se remplace pas (NJ-13).
CREATE TABLE IF NOT EXISTS ent_sortie_signature (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    devis_id UUID NOT NULL,
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    niveau TEXT NOT NULL CHECK (niveau IN ('S1', 'S2')),
    signataire_user_id UUID NOT NULL,
    signataire_personne_id UUID,
    intention TEXT NOT NULL CHECK (length(btrim(intention)) BETWEEN 1 AND 500),
    signe_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_signature_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_signature_devis_uq UNIQUE (devis_id),
    CONSTRAINT ent_sortie_signature_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_signature_devis_fk FOREIGN KEY (etablissement_id, devis_id)
        REFERENCES ent_sortie_devis (etablissement_id, id),
    CONSTRAINT ent_sortie_signature_personne_fk FOREIGN KEY (etablissement_id, signataire_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_sortie_signature_immuable ON ent_sortie_signature;
CREATE TRIGGER ent_sortie_signature_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_signature
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Commande au prestataire (issue d'un devis signé, en une transaction avec la signature : VD-28).
CREATE TABLE IF NOT EXISTS ent_sortie_commande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    devis_id UUID NOT NULL,
    signature_id UUID NOT NULL,
    prestataire_id UUID NOT NULL,
    adresse_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero > 0),
    statut TEXT NOT NULL DEFAULT 'a_envoyer' CHECK (statut IN (
        'a_envoyer', 'envoyee', 'a_confirmer', 'confirmee', 'annulee')),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 0 AND 100000000000),
    effectif_eleves_commande INTEGER NOT NULL CHECK (effectif_eleves_commande >= 0),
    effectif_adultes_commande INTEGER NOT NULL CHECK (effectif_adultes_commande >= 0),
    debut_commande TIMESTAMPTZ NOT NULL,
    fin_commande TIMESTAMPTZ NOT NULL,
    confirmee_le TIMESTAMPTZ,
    -- Champs dédiés (pas de texte libre), visibles des seuls encadrants, effacés à la clôture.
    chauffeur_nom TEXT CHECK (chauffeur_nom IS NULL OR length(btrim(chauffeur_nom)) BETWEEN 1 AND 120),
    chauffeur_tel TEXT CHECK (chauffeur_tel IS NULL OR length(chauffeur_tel) BETWEEN 3 AND 30),
    contacts_effaces_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_commande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_commande_devis_uq UNIQUE (devis_id),
    CONSTRAINT ent_sortie_commande_numero_uq UNIQUE (sortie_id, numero),
    CONSTRAINT ent_sortie_commande_dates_chk CHECK (fin_commande > debut_commande),
    CONSTRAINT ent_sortie_commande_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_commande_devis_fk FOREIGN KEY (etablissement_id, devis_id)
        REFERENCES ent_sortie_devis (etablissement_id, id),
    CONSTRAINT ent_sortie_commande_signature_fk FOREIGN KEY (etablissement_id, signature_id)
        REFERENCES ent_sortie_signature (etablissement_id, id),
    CONSTRAINT ent_sortie_commande_adresse_fk FOREIGN KEY (etablissement_id, prestataire_id, adresse_id)
        REFERENCES ent_prestataire_adresse (etablissement_id, prestataire_id, id)
);

-- Envois sortants (demande de devis, relance, bon de commande, avenant) : file d'envoi à neutraliser
-- à la restauration (E9). Le destinataire est une adresse de l'annuaire, jamais une saisie libre.
CREATE TABLE IF NOT EXISTS ent_sortie_envoi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    prestataire_id UUID NOT NULL,
    adresse_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('demande_devis', 'relance', 'bon_commande', 'avenant')),
    statut TEXT NOT NULL DEFAULT 'a_envoyer' CHECK (statut IN ('a_envoyer', 'envoye', 'echec', 'annule')),
    -- Référence de correspondance (non devinable, somme de contrôle) placée dans l'objet du message.
    reference_courrier TEXT NOT NULL CHECK (length(reference_courrier) BETWEEN 6 AND 80),
    objet TEXT NOT NULL CHECK (length(btrim(objet)) BETWEEN 1 AND 300),
    corps TEXT NOT NULL CHECK (length(corps) BETWEEN 1 AND 6000),
    document_id UUID,
    commande_id UUID,
    -- Copie à la boîte comptabilité de l'ÉTABLISSEMENT (réglage), figée à la mise en file.
    copie_comptabilite_email TEXT CHECK (copie_comptabilite_email IS NULL OR length(copie_comptabilite_email) <= 254),
    tentatives INTEGER NOT NULL DEFAULT 0 CHECK (tentatives >= 0),
    derniere_erreur TEXT CHECK (derniere_erreur IS NULL OR length(derniere_erreur) <= 200),
    envoye_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_envoi_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_envoi_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_envoi_adresse_fk FOREIGN KEY (etablissement_id, prestataire_id, adresse_id)
        REFERENCES ent_prestataire_adresse (etablissement_id, prestataire_id, id),
    CONSTRAINT ent_sortie_envoi_document_fk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_sortie_document (etablissement_id, id),
    CONSTRAINT ent_sortie_envoi_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_commande (etablissement_id, id),
    CONSTRAINT ent_sortie_envoi_envoye_chk CHECK ((statut = 'envoye') = (envoye_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_sortie_envoi_file_idx
    ON ent_sortie_envoi (etablissement_id, statut, cree_le);
CREATE INDEX IF NOT EXISTS ent_sortie_envoi_sortie_idx
    ON ent_sortie_envoi (etablissement_id, sortie_id, type);

-- Avenant numéroté (NX-44) : « annule et remplace », au prestataire de la commande. La signature
-- précédente reste au dossier, non effacée (VD-07, VD-08).
CREATE TABLE IF NOT EXISTS ent_sortie_avenant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    commande_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero > 0),
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 1000),
    effectif_eleves_commande INTEGER NOT NULL CHECK (effectif_eleves_commande >= 0),
    effectif_adultes_commande INTEGER NOT NULL CHECK (effectif_adultes_commande >= 0),
    debut_commande TIMESTAMPTZ NOT NULL,
    fin_commande TIMESTAMPTZ NOT NULL,
    document_id UUID,
    envoi_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_avenant_uq UNIQUE (commande_id, numero),
    CONSTRAINT ent_sortie_avenant_dates_chk CHECK (fin_commande > debut_commande),
    CONSTRAINT ent_sortie_avenant_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_avenant_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_commande (etablissement_id, id),
    CONSTRAINT ent_sortie_avenant_document_fk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_sortie_document (etablissement_id, id),
    CONSTRAINT ent_sortie_avenant_envoi_fk FOREIGN KEY (etablissement_id, envoi_id)
        REFERENCES ent_sortie_envoi (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_sortie_avenant_immuable ON ent_sortie_avenant;
CREATE TRIGGER ent_sortie_avenant_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_avenant
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Alimentation du budget par un devis (NX-51) : une ligne du budget peut citer le devis dont elle
-- vient (le montant est celui CONFIRMÉ). Une même version ne porte qu'une ligne par devis.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = 'ent_sortie_budget_ligne'
                      AND column_name = 'devis_id') THEN
        ALTER TABLE ent_sortie_budget_ligne ADD COLUMN devis_id UUID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_budget_ligne_devis_fk') THEN
        ALTER TABLE ent_sortie_budget_ligne ADD CONSTRAINT ent_sortie_budget_ligne_devis_fk
            FOREIGN KEY (etablissement_id, devis_id) REFERENCES ent_sortie_devis (etablissement_id, id);
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_budget_ligne_devis_uq
    ON ent_sortie_budget_ligne (version_id, devis_id) WHERE devis_id IS NOT NULL;

SELECT ent_rls_poser(ARRAY[
    'ent_prestataire',
    'ent_prestataire_categorie',
    'ent_prestataire_adresse',
    'ent_sortie_reglage',
    'ent_courrier_entrant',
    'ent_courrier_piece',
    'ent_sortie_devis',
    'ent_sortie_signature',
    'ent_sortie_commande',
    'ent_sortie_envoi',
    'ent_sortie_avenant'
]);
