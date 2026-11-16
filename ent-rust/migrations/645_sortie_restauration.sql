-- 645 — Socle 42, lot V0d (1/3) : restauration (commande, « qui mange », convives externes, avenants,
-- avis à la cuisine) et ports de courrier sortant de D-065 (adresse de réponse à usage unique).
-- Réf. : 42_SORTIES_VOYAGES.md §14 (NX-38 à NX-40, NX-86, VOY-256), R-SV12, R-SV13, VD-06, VD-16, VD-44 ;
-- D-065 (envoi depuis le sous-domaine UAI, réponses `r-<clé>@<uai>.scolia.fr` à usage unique).
--
-- Exigences E1 à E14 (PRA) : `etablissement_id` NOT NULL partout, clés étrangères COMPOSITES, UUID fournis par
-- l'application, aucun BYTEA, aucun JSON (D-036), horodatages, tables d'ajout seul pour les faits (avis),
-- migration rejouable (aucune modification des migrations 640 à 644), RLS posée en fin de fichier.
--
-- * Le destinataire d'un avis de restauration est la boîte de l'ÉTABLISSEMENT (réglage), jamais une adresse
--   fournie par le navigateur (corrige VD-06).
-- * Aucune ligne ne porte de numéro INE ni de nom de maladie : une variante est un code alimentaire (VD-16).
-- * L'adresse de réponse ne porte aucune donnée en clair : seule l'empreinte SHA-256 de la clé est conservée.

-- Boîte de la restauration de l'établissement (socle 5 ou 40 à terme) : colonne du réglage de V0c.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = 'ent_sortie_reglage'
                      AND column_name = 'boite_restauration_email') THEN
        ALTER TABLE ent_sortie_reglage ADD COLUMN boite_restauration_email TEXT
            CHECK (boite_restauration_email IS NULL
                   OR (length(boite_restauration_email) BETWEEN 6 AND 254
                       AND boite_restauration_email = lower(btrim(boite_restauration_email))
                       AND strpos(boite_restauration_email, chr(59)) = 0
                       AND boite_restauration_email ~ '^[^@[:space:]<>,"]+@[^@[:space:]<>,"]+\.[^@[:space:]<>,"]+$'));
    END IF;
END
$$;

-- Clé composite (E2) sur les encadrants : les attributions de repas et les délégués du journal la référencent.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_encadrant_etab_id_uq') THEN
        ALTER TABLE ent_sortie_encadrant ADD CONSTRAINT ent_sortie_encadrant_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END
$$;

-- Une commande de restauration par sortie : le lieu et l'heure de livraison, la révision (avenants).
CREATE TABLE IF NOT EXISTS ent_sortie_repas_commande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    livraison_heure TIME,
    livraison_lieu TEXT CHECK (livraison_lieu IS NULL OR length(livraison_lieu) <= 300),
    -- 0 = commande initiale ; chaque avis envoyé après modification incrémente.
    numero_avis INTEGER NOT NULL DEFAULT 0 CHECK (numero_avis >= 0),
    -- Compteur d'écritures : la commande lue est celle qu'on modifie (VD-26, écritures concurrentes).
    rev INTEGER NOT NULL DEFAULT 1,
    -- Révision déjà portée par un avis : tant que `rev` la dépasse, il reste des changements à transmettre.
    rev_transmise INTEGER NOT NULL DEFAULT 0 CHECK (rev_transmise >= 0),
    statut TEXT NOT NULL DEFAULT 'brouillon' CHECK (statut IN ('brouillon', 'passee', 'annulee')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_repas_commande_sortie_uq UNIQUE (sortie_id),
    CONSTRAINT ent_sortie_repas_commande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_commande_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);

-- Lignes de commande par jour de la sortie (les jours de semaine de Next.js sont convertis en dates).
CREATE TABLE IF NOT EXISTS ent_sortie_repas_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    commande_id UUID NOT NULL,
    jour DATE NOT NULL,
    categorie TEXT NOT NULL CHECK (categorie IN ('panier', 'self_eleve', 'self_adulte', 'boisson', 'gouter')),
    variante TEXT NOT NULL DEFAULT 'standard' CHECK (variante IN (
        'standard', 'sans_porc', 'vegetarien', 'sans_arachide', 'sans_gluten', 'sans_lactose')),
    quantite INTEGER NOT NULL CHECK (quantite BETWEEN 0 AND 2000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_repas_ligne_uq UNIQUE (commande_id, jour, categorie, variante),
    CONSTRAINT ent_sortie_repas_ligne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_ligne_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_repas_commande (etablissement_id, id)
);

-- « Qui mange » (NX-39) : un panier attribué à un participant OU à un encadrant, pour un jour. Jamais les
-- deux, jamais ni l'un ni l'autre. L'attribution d'un désisté est ignorée au calcul (le serveur le sait).
CREATE TABLE IF NOT EXISTS ent_sortie_repas_attribution (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    commande_id UUID NOT NULL,
    jour DATE NOT NULL,
    participant_id UUID,
    encadrant_id UUID,
    variante TEXT NOT NULL DEFAULT 'standard' CHECK (variante IN (
        'standard', 'sans_porc', 'vegetarien', 'sans_arachide', 'sans_gluten', 'sans_lactose')),
    -- Étiquette nominative du panier : seulement si la consigne l'exige pour éviter l'erreur (§14).
    etiquette_nominative BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_repas_attribution_cible_chk CHECK (
        (participant_id IS NOT NULL) <> (encadrant_id IS NOT NULL)),
    CONSTRAINT ent_sortie_repas_attribution_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_attribution_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_repas_commande (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_attribution_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_attribution_encadrant_fk FOREIGN KEY (etablissement_id, encadrant_id)
        REFERENCES ent_sortie_encadrant (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_repas_attribution_participant_uq
    ON ent_sortie_repas_attribution (commande_id, jour, participant_id) WHERE participant_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_repas_attribution_encadrant_uq
    ON ent_sortie_repas_attribution (commande_id, jour, encadrant_id) WHERE encadrant_id IS NOT NULL;

-- Convives externes (NX-86, VOY-256) : un libellé et un nombre, aucune donnée personnelle.
CREATE TABLE IF NOT EXISTS ent_sortie_repas_externe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    commande_id UUID NOT NULL,
    jour DATE NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 100),
    nombre INTEGER NOT NULL CHECK (nombre BETWEEN 1 AND 500),
    variante TEXT NOT NULL DEFAULT 'standard' CHECK (variante IN (
        'standard', 'sans_porc', 'vegetarien', 'sans_arachide', 'sans_gluten', 'sans_lactose')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_repas_externe_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_externe_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_repas_commande (etablissement_id, id)
);

-- Avis à la cuisine (commande, puis avenants « annule et remplace »), en AJOUT SEUL pour les faits ; seul
-- l'état d'envoi évolue. Le corps est un texte brut, composé par le serveur (R-SV12) ; le destinataire n'y
-- figure PAS : il est lu dans le réglage de l'établissement au moment de l'envoi.
CREATE TABLE IF NOT EXISTS ent_sortie_repas_avis (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    commande_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero >= 0),
    type TEXT NOT NULL CHECK (type IN ('commande', 'avenant')),
    objet TEXT NOT NULL CHECK (length(objet) BETWEEN 1 AND 300 AND strpos(objet, chr(10)) = 0 AND strpos(objet, chr(13)) = 0),
    corps TEXT NOT NULL CHECK (length(corps) <= 20000),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    statut TEXT NOT NULL DEFAULT 'a_envoyer' CHECK (statut IN ('a_envoyer', 'envoye', 'echec', 'sans_destinataire')),
    tentatives INTEGER NOT NULL DEFAULT 0 CHECK (tentatives >= 0),
    envoye_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_repas_avis_numero_uq UNIQUE (commande_id, numero),
    CONSTRAINT ent_sortie_repas_avis_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_repas_avis_commande_fk FOREIGN KEY (etablissement_id, commande_id)
        REFERENCES ent_sortie_repas_commande (etablissement_id, id)
);

-- Seuls l'état d'envoi, le nombre d'essais et la date d'envoi changent ; le contenu d'un avis est figé et
-- un avis ne se supprime pas (la remise est une preuve, R-SV13).
CREATE OR REPLACE FUNCTION ent_sortie_repas_avis_fige() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_sortie_repas_avis : suppression interdite' USING ERRCODE = '42501';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.commande_id, NEW.numero, NEW.type, NEW.objet, NEW.corps, NEW.motif)
       IS DISTINCT FROM (OLD.id, OLD.etablissement_id, OLD.commande_id, OLD.numero, OLD.type, OLD.objet, OLD.corps, OLD.motif) THEN
        RAISE EXCEPTION 'ent_sortie_repas_avis : contenu figé' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_repas_avis_fige ON ent_sortie_repas_avis;
CREATE TRIGGER ent_sortie_repas_avis_fige
    BEFORE UPDATE OR DELETE ON ent_sortie_repas_avis
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_repas_avis_fige();

-- Ports de courrier sortant (D-065) : une adresse de réponse à USAGE UNIQUE par destinataire et par dossier.
-- La clé aléatoire n'est jamais conservée : seule son empreinte renvoie vers (établissement, objet, dossier).
CREATE TABLE IF NOT EXISTS ent_sortie_reponse_cle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cle_hash TEXT NOT NULL CHECK (cle_hash ~ '^[0-9a-f]{64}$'),
    sortie_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (objet IN ('prestataire', 'restauration')),
    prestataire_id UUID,
    adresse_id UUID,
    expire_le TIMESTAMPTZ NOT NULL,
    utilisee_le TIMESTAMPTZ,
    revoquee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_reponse_cle_hash_uq UNIQUE (cle_hash),
    CONSTRAINT ent_sortie_reponse_cle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_reponse_cle_objet_chk CHECK (
        (objet = 'prestataire') = (prestataire_id IS NOT NULL AND adresse_id IS NOT NULL)),
    CONSTRAINT ent_sortie_reponse_cle_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_reponse_cle_adresse_fk FOREIGN KEY (etablissement_id, prestataire_id, adresse_id)
        REFERENCES ent_prestataire_adresse (etablissement_id, prestataire_id, id)
);

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_repas_commande',
    'ent_sortie_repas_ligne',
    'ent_sortie_repas_attribution',
    'ent_sortie_repas_externe',
    'ent_sortie_repas_avis',
    'ent_sortie_reponse_cle'
]);
