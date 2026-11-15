-- Socle N1 (« Demandes et flux de travail »), lot N1-2 — migration 3044 : échanges, pièces, abonnés.
-- Réf. : docs/socles/N1_DEMANDES.md §12 (échanges, pièces, abonnés), §37 (trois preuves), §38 (D-061) ;
-- NJ-80 (écriture sans droit), NJ-83 (pièces de toutes les files lisibles), NJ-89 (preuve conservée).
--
-- Contenu : ent_dem_message (échanges au demandeur / internes / de consultation), ent_dem_piece (RÉFÉRENCES au
-- socle 7 : un `document_id`, un rôle, jamais un nom de fichier, jamais d'octets : D-036, E5), ent_dem_abonne,
-- et l'unicité du fait « vue par le service » (R-DM53).
-- Additive et rejouable (E8). Aucune donnée semi-structurée, aucun tableau, aucun contenu binaire.
-- Clés étrangères composites (etablissement_id, …) et ON DELETE RESTRICT partout (NJ-89). Le lien
-- message → consultation est une clé étrangère ajoutée par la migration 3045 (la table n'existe pas encore).

-- Réglage LOCAL à la transaction posé par le seul chemin de requalification (CDM-01, CDM-11) : il autorise le
-- changement de nature (garde de la demande, migration 3045) et le scellement de textes déjà écrits en clair quand
-- la demande devient confidentielle (C2+).
CREATE OR REPLACE FUNCTION ent_dem_requalif_en_cours() RETURNS boolean
LANGUAGE sql STABLE AS $$
    SELECT COALESCE(current_setting('ent.dem_requalif', true), '') = 'on'
$$;

-- ───────────────────────── Messages (le fil de la demande) ─────────────────────────
-- Trois visibilités : au demandeur, interne (membres de la file), de consultation (un tiers consulté et la file).
-- Aucune modification d'un message envoyé : une correction est un nouveau message (§12.1). Seul le MASQUAGE
-- (faute de frappe, dans les 10 minutes, par l'auteur d'une note interne) pose `masque_le`, une seule fois.
-- Le texte d'une nature C2+ est scellé (champs_chiffres) comme celui de la demande (R-DM16).
CREATE TABLE IF NOT EXISTS ent_dem_message (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    auteur_personne_id UUID NOT NULL,
    visibilite TEXT NOT NULL CHECK (visibilite IN ('demandeur', 'interne', 'consultation')),
    -- Genre : échange ordinaire, demande de complément (met la demande en attente du demandeur),
    -- réponse du demandeur à un complément, correction d'un message précédent.
    genre TEXT NOT NULL DEFAULT 'echange'
        CHECK (genre IN ('echange', 'complement_demande', 'complement_recu', 'correction')),
    consultation_id UUID,
    texte TEXT NOT NULL CHECK (length(texte) BETWEEN 1 AND 30000),
    texte_scelle BOOLEAN NOT NULL DEFAULT false,
    corrige_message_id UUID,
    masque_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_message_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_message_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_message_auteur_cfk FOREIGN KEY (etablissement_id, auteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_message_corrige_cfk FOREIGN KEY (etablissement_id, corrige_message_id)
        REFERENCES ent_dem_message (etablissement_id, id) ON DELETE RESTRICT,
    -- Un message de consultation porte sa consultation ; aucun autre n'en porte.
    CONSTRAINT ent_dem_message_consultation_chk CHECK ((visibilite = 'consultation') = (consultation_id IS NOT NULL)),
    -- Un complément (demandé ou reçu) est toujours visible du demandeur ; une correction ne se corrige pas.
    CONSTRAINT ent_dem_message_genre_chk CHECK (
        (genre NOT IN ('complement_demande', 'complement_recu') OR visibilite = 'demandeur')
        AND (genre <> 'correction' OR corrige_message_id IS NOT NULL)),
    -- Seul un message interne se masque (le demandeur ne se voit pas retirer ce qu'on lui a écrit).
    CONSTRAINT ent_dem_message_masque_chk CHECK (masque_le IS NULL OR visibilite = 'interne')
);
CREATE INDEX IF NOT EXISTS ent_dem_message_demande_idx ON ent_dem_message (etablissement_id, demande_id, cree_le, id);

CREATE OR REPLACE FUNCTION ent_dem_message_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_message : suppression interdite (NJ-89) ; un message se masque, il ne se supprime pas'
            USING ERRCODE = '23001';
    END IF;
    -- Requalification en nature confidentielle : un texte écrit en clair est SCELLÉ, rien d'autre ne change.
    IF ent_dem_requalif_en_cours() AND NOT OLD.texte_scelle AND NEW.texte_scelle
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.demande_id = OLD.demande_id
       AND NEW.auteur_personne_id = OLD.auteur_personne_id AND NEW.visibilite = OLD.visibilite
       AND NEW.genre = OLD.genre AND NEW.consultation_id IS NOT DISTINCT FROM OLD.consultation_id
       AND NEW.corrige_message_id IS NOT DISTINCT FROM OLD.corrige_message_id
       AND NEW.masque_le IS NOT DISTINCT FROM OLD.masque_le AND NEW.cree_le = OLD.cree_le THEN
        RETURN NEW;
    END IF;
    -- La seule autre écriture admise sur un message existant : poser `masque_le`, une fois.
    IF OLD.masque_le IS NULL AND NEW.masque_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.demande_id = OLD.demande_id
       AND NEW.auteur_personne_id = OLD.auteur_personne_id AND NEW.visibilite = OLD.visibilite
       AND NEW.genre = OLD.genre AND NEW.consultation_id IS NOT DISTINCT FROM OLD.consultation_id
       AND NEW.texte = OLD.texte AND NEW.texte_scelle = OLD.texte_scelle
       AND NEW.corrige_message_id IS NOT DISTINCT FROM OLD.corrige_message_id AND NEW.cree_le = OLD.cree_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_dem_message : un message envoyé ne se modifie pas (§12.1) ; écrire un message de correction'
        USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_message_garde_trg ON ent_dem_message;
CREATE TRIGGER ent_dem_message_garde_trg
    BEFORE UPDATE OR DELETE ON ent_dem_message
    FOR EACH ROW EXECUTE FUNCTION ent_dem_message_garde();

-- ───────────────────────── Pièces : références au socle 7 ─────────────────────────
-- N1 garde un identifiant de document, un RÔLE et la visibilité ; le fichier, son type réel, son antivirus et son
-- nom restent au socle 7. Aucune colonne de nom de fichier, de chemin, de type MIME ni d'octets (NJ-83, NJ-78).
CREATE TABLE IF NOT EXISTS ent_dem_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    message_id UUID,
    document_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN (
        'justificatif', 'annexe', 'reponse', 'photo_lieu', 'fichier_a_imprimer', 'accord')),
    -- Qui peut la lire en plus des membres de la file : le demandeur (pièce déposée par lui ou qui lui est adressée)
    -- ou la file seule (pièce interne). Un consulté ne lit que ce qui lui est partagé (ent_dem_consultation_partage).
    visibilite TEXT NOT NULL CHECK (visibilite IN ('demandeur', 'interne')),
    depose_par_personne_id UUID NOT NULL,
    retire_le TIMESTAMPTZ,
    retire_par_personne_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_piece_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_piece_document_uq UNIQUE (etablissement_id, demande_id, document_id),
    CONSTRAINT ent_dem_piece_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_piece_message_cfk FOREIGN KEY (etablissement_id, message_id)
        REFERENCES ent_dem_message (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_piece_depose_cfk FOREIGN KEY (etablissement_id, depose_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_piece_retrait_chk CHECK ((retire_le IS NULL) = (retire_par_personne_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_dem_piece_demande_idx ON ent_dem_piece (etablissement_id, demande_id, cree_le);

-- Une pièce ne se supprime pas et ne change pas de document ni de visibilité : seul le retrait (une fois) s'écrit.
CREATE OR REPLACE FUNCTION ent_dem_piece_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_piece : suppression interdite (NJ-89) ; une pièce se retire, la trace reste'
            USING ERRCODE = '23001';
    END IF;
    IF OLD.retire_le IS NULL AND NEW.retire_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.demande_id = OLD.demande_id
       AND NEW.message_id IS NOT DISTINCT FROM OLD.message_id AND NEW.document_id = OLD.document_id
       AND NEW.role = OLD.role AND NEW.visibilite = OLD.visibilite
       AND NEW.depose_par_personne_id = OLD.depose_par_personne_id AND NEW.cree_le = OLD.cree_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_dem_piece : une pièce déposée ne se modifie pas' USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_piece_garde_trg ON ent_dem_piece;
CREATE TRIGGER ent_dem_piece_garde_trg
    BEFORE UPDATE OR DELETE ON ent_dem_piece
    FOR EACH ROW EXECUTE FUNCTION ent_dem_piece_garde();

-- ───────────────────────── Abonnés ─────────────────────────
-- Co-demandeur (autre responsable du même foyer), appui (« +1 »), copie interne (collègue tenu informé).
-- Un abonné ne voit que ce que sa qualité lui ouvre (§12.3). Une fin de période (`au`) ferme l'abonnement.
CREATE TABLE IF NOT EXISTS ent_dem_abonne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    qualite TEXT NOT NULL CHECK (qualite IN ('co_demandeur', 'appui', 'copie_interne')),
    du DATE NOT NULL,
    au DATE,
    ajoute_par_personne_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_abonne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_abonne_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_abonne_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_abonne_ajoute_cfk FOREIGN KEY (etablissement_id, ajoute_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_abonne_periode_chk CHECK (au IS NULL OR au >= du)
);
-- Un abonnement actif par personne, demande et qualité.
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_abonne_actif_uq
    ON ent_dem_abonne (etablissement_id, demande_id, personne_id, qualite) WHERE au IS NULL;
CREATE INDEX IF NOT EXISTS ent_dem_abonne_personne_idx ON ent_dem_abonne (etablissement_id, personne_id) WHERE au IS NULL;
DROP TRIGGER IF EXISTS ent_dem_abonne_sans_suppression_trg ON ent_dem_abonne;
CREATE TRIGGER ent_dem_abonne_sans_suppression_trg
    BEFORE DELETE ON ent_dem_abonne
    FOR EACH ROW EXECUTE FUNCTION ent_dem_sans_suppression();

-- ───────────────────────── « Vue par le service » : un seul fait par demande (R-DM53, CDM-82) ─────────────────────────
-- Deux membres qui ouvrent la demande en même temps ne posent qu'UN fait ; le premier l'emporte.
-- Un fait par demande ET par file de destination : après un transfert, la première ouverture par la nouvelle file
-- est de nouveau montrée au demandeur.
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_fait_vue_uq
    ON ent_dem_fait (etablissement_id, demande_id, file_apres_id) WHERE type = 'vue_par_service';
-- Un fait de retard par demande et par échéance : l'ordonnanceur est rejouable sans doublon (R-DM54).
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_fait_retard_uq
    ON ent_dem_fait (etablissement_id, demande_id, motif_code) WHERE type = 'retard_signale';

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_abonne',
    'ent_dem_message',
    'ent_dem_piece'
]);
