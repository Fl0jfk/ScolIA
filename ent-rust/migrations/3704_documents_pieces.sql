-- Socle 7, lot D5 — migration 3704 : pièces justificatives (demandes, dépôts en quarantaine, contrôle, réutilisation).
-- Réf. : 07_DOCUMENTS.md §8.6 (pièces), §11 (catalogue, dépôt, analyse, contrôle), §17 (sécurité).
--
-- Principes inscrits en base (la base garde les garde-fous même si le service est contourné) :
--   * un fichier déposé n'est JAMAIS servi tant qu'il est en quarantaine : la colonne `cle_objet_servi` reste NULL
--     (CHECK) tant que la pièce est en quarantaine, infectée ou rejetée ; elle ne s'écrit qu'une fois, au passage
--     « reçue » (analyse saine), et ne change plus ;
--   * l'original est gardé scellé (jamais servi), le fichier servi est la version désarmée ;
--   * machine à états : quarantaine → reçue | infectée | rejetée | retirée ; reçue → acceptée | refusée | sans objet |
--     retirée ; tout le reste est final ;
--   * identité, sujet, déposant et fichiers d'origine immuables ; aucune suppression ;
--   * décisions en ajout seul ;
--   * D-036 : ni JSON, ni tableau, ni BYTEA ; aucune date « du jour » en base (le service passe le jour de l'établissement).

-- ───────────────────────── Demandes de pièces ─────────────────────────
-- Une brique (inscriptions, voyages, bourses…) ou un agent DEMANDE une pièce pour un sujet. Le service cherche d'abord
-- une pièce valable au dossier (même sujet, ou même foyer pour une pièce de portée « foyer ») et la propose.
CREATE TABLE IF NOT EXISTS ent_doc_piece_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type_code TEXT NOT NULL CHECK (type_code ~ '^[a-z][a-z0-9_]{0,59}$'),
    -- Sujet : exactement un des trois (l'élève pour une pièce d'élève ou de foyer ; la personne ; l'agent).
    eleve_id UUID,
    personne_id UUID,
    agent_id UUID,
    portee TEXT NOT NULL CHECK (portee IN ('foyer', 'eleve', 'personne', 'agent', 'etablissement')),
    motif_affiche TEXT NOT NULL CHECK (length(btrim(motif_affiche)) BETWEEN 3 AND 300),
    echeance DATE,
    obligatoire BOOLEAN NOT NULL DEFAULT true,
    brique_demandeuse TEXT NOT NULL CHECK (brique_demandeuse ~ '^[a-z][a-z0-9_]{0,39}$'),
    reference_externe UUID,
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('ouverte', 'satisfaite', 'sans_objet', 'annulee')),
    -- La pièce qui satisfait la demande (déposée pour elle, ou réutilisée).
    piece_id UUID,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cloturee_le TIMESTAMPTZ,
    cloture_motif TEXT CHECK (cloture_motif IS NULL OR length(btrim(cloture_motif)) BETWEEN 3 AND 500),
    CONSTRAINT ent_doc_piece_demande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_piece_demande_sujet_chk CHECK (num_nonnulls(eleve_id, personne_id, agent_id) = 1),
    CONSTRAINT ent_doc_piece_demande_cloture_chk CHECK ((etat = 'ouverte') = (cloturee_le IS NULL)),
    CONSTRAINT ent_doc_piece_demande_satisfaite_chk CHECK ((etat = 'satisfaite') = (piece_id IS NOT NULL)),
    CONSTRAINT ent_doc_piece_demande_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_demande_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_demande_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);
-- Une seule demande OUVERTE par type et par sujet (sauf le type « autre », motivé à chaque fois).
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_piece_demande_ouverte_uq
    ON ent_doc_piece_demande (etablissement_id, type_code, COALESCE(eleve_id, personne_id, agent_id))
    WHERE etat = 'ouverte' AND type_code <> 'autre';
CREATE INDEX IF NOT EXISTS ent_doc_piece_demande_eleve_idx
    ON ent_doc_piece_demande (etablissement_id, eleve_id, etat) WHERE eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_piece_demande_personne_idx
    ON ent_doc_piece_demande (etablissement_id, personne_id, etat) WHERE personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_piece_demande_ouvertes_idx
    ON ent_doc_piece_demande (etablissement_id, echeance) WHERE etat = 'ouverte';

-- ───────────────────────── Pièces déposées ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type_code TEXT NOT NULL CHECK (type_code ~ '^[a-z][a-z0-9_]{0,59}$'),
    -- Sujet : exactement un des trois (l'élève pour une pièce d'élève ou de foyer).
    eleve_id UUID,
    personne_id UUID,
    agent_id UUID,
    -- La demande à laquelle ce dépôt répond (facultatif : dépôt spontané, dépôt au guichet).
    demande_id UUID,
    -- Motif obligatoire pour le type « autre » (affiché à la famille, revue du DPO).
    motif_autre TEXT CHECK (motif_autre IS NULL OR length(btrim(motif_autre)) BETWEEN 3 AND 300),
    deposant_user_id UUID NOT NULL,
    deposant_role TEXT NOT NULL CHECK (length(deposant_role) BETWEEN 1 AND 40),
    deposee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    etat TEXT NOT NULL DEFAULT 'en_quarantaine' CHECK (etat IN (
        'en_quarantaine', 'recue', 'acceptee', 'refusee', 'sans_objet', 'infectee', 'rejetee', 'retiree')),
    -- Original reçu : scellé, jamais servi.
    cle_objet_original TEXT NOT NULL,
    sha256_original TEXT NOT NULL CHECK (sha256_original ~ '^[0-9a-f]{64}$'),
    taille_originale BIGINT NOT NULL CHECK (taille_originale > 0),
    -- Plusieurs photos : l'original est l'archive des photos reçues (méthode « stored »).
    mime_original TEXT NOT NULL CHECK (mime_original IN ('application/pdf', 'image/jpeg', 'image/png', 'application/zip')),
    nb_fichiers_origine INT NOT NULL DEFAULT 1 CHECK (nb_fichiers_origine BETWEEN 1 AND 50),
    -- Fichier SERVI : version désarmée ou assemblée. NULL tant que la pièce n'a pas quitté la quarantaine.
    cle_objet_servi TEXT,
    sha256_servi TEXT CHECK (sha256_servi IS NULL OR sha256_servi ~ '^[0-9a-f]{64}$'),
    nb_pages INT CHECK (nb_pages IS NULL OR nb_pages BETWEEN 1 AND 50),
    -- Analyse : tentatives (l'antivirus indisponible laisse la pièce en quarantaine, on réessaie), verdict, motif.
    analyse_tentatives INT NOT NULL DEFAULT 0 CHECK (analyse_tentatives >= 0),
    derniere_analyse_le TIMESTAMPTZ,
    analyse_motif TEXT CHECK (analyse_motif IS NULL OR length(analyse_motif) <= 200),
    -- Ce que le désarmement a neutralisé (texte, séparé par des virgules) : information du contrôleur.
    desarmement TEXT CHECK (desarmement IS NULL OR length(desarmement) <= 300),
    -- Contrôle humain.
    valable_jusqu_au DATE,
    controle_par UUID,
    controle_le TIMESTAMPTZ,
    refus_motif TEXT CHECK (refus_motif IS NULL OR refus_motif IN (
        'illisible', 'incomplet', 'perime', 'ne_correspond_pas', 'mauvais_document', 'autre')),
    commentaire TEXT CHECK (commentaire IS NULL OR length(btrim(commentaire)) BETWEEN 1 AND 500),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_piece_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_piece_sujet_chk CHECK (num_nonnulls(eleve_id, personne_id, agent_id) = 1),
    CONSTRAINT ent_doc_piece_autre_chk CHECK ((type_code = 'autre') = (motif_autre IS NOT NULL)),
    -- Aucun fichier servi hors quarantaine levée, et un fichier servi dès qu'elle est levée.
    CONSTRAINT ent_doc_piece_servi_chk CHECK (
        (etat IN ('en_quarantaine', 'infectee', 'rejetee') AND cle_objet_servi IS NULL AND sha256_servi IS NULL)
        OR (etat IN ('recue', 'acceptee', 'refusee', 'sans_objet') AND cle_objet_servi IS NOT NULL AND sha256_servi IS NOT NULL AND nb_pages IS NOT NULL)
        OR etat = 'retiree'),
    CONSTRAINT ent_doc_piece_refus_chk CHECK ((etat = 'refusee') = (refus_motif IS NOT NULL)),
    CONSTRAINT ent_doc_piece_valable_chk CHECK (valable_jusqu_au IS NULL OR etat = 'acceptee'),
    CONSTRAINT ent_doc_piece_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_doc_piece_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_original_cfk FOREIGN KEY (etablissement_id, cle_objet_original)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_piece_servi_cfk FOREIGN KEY (etablissement_id, cle_objet_servi)
        REFERENCES ent_objet (etablissement_id, cle_objet) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_doc_piece_eleve_idx
    ON ent_doc_piece (etablissement_id, eleve_id, type_code, deposee_le DESC) WHERE eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_piece_personne_idx
    ON ent_doc_piece (etablissement_id, personne_id, type_code, deposee_le DESC) WHERE personne_id IS NOT NULL;
-- File du secrétariat (contrôle en rafale) et file d'analyse.
CREATE INDEX IF NOT EXISTS ent_doc_piece_a_controler_idx
    ON ent_doc_piece (etablissement_id, deposee_le) WHERE etat = 'recue';
CREATE INDEX IF NOT EXISTS ent_doc_piece_a_analyser_idx
    ON ent_doc_piece (etablissement_id, deposee_le) WHERE etat = 'en_quarantaine';

-- Garde : machine à états, champs immuables, fichier servi écrit une seule fois, aucune suppression.
CREATE OR REPLACE FUNCTION ent_doc_piece_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_piece : suppression interdite (retirer la pièce)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.type_code IS DISTINCT FROM OLD.type_code
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
       OR NEW.motif_autre IS DISTINCT FROM OLD.motif_autre
       OR NEW.demande_id IS DISTINCT FROM OLD.demande_id
       OR NEW.deposant_user_id IS DISTINCT FROM OLD.deposant_user_id
       OR NEW.deposant_role IS DISTINCT FROM OLD.deposant_role
       OR NEW.deposee_le IS DISTINCT FROM OLD.deposee_le
       OR NEW.cle_objet_original IS DISTINCT FROM OLD.cle_objet_original
       OR NEW.sha256_original IS DISTINCT FROM OLD.sha256_original
       OR NEW.taille_originale IS DISTINCT FROM OLD.taille_originale
       OR NEW.mime_original IS DISTINCT FROM OLD.mime_original
       OR NEW.nb_fichiers_origine IS DISTINCT FROM OLD.nb_fichiers_origine THEN
        RAISE EXCEPTION 'ent_doc_piece : identité, sujet, déposant et original sont immuables' USING ERRCODE = '23001';
    END IF;
    -- Le fichier servi ne s'écrit qu'une fois, au passage en quarantaine → reçue.
    IF OLD.cle_objet_servi IS NOT NULL
       AND (NEW.cle_objet_servi IS DISTINCT FROM OLD.cle_objet_servi OR NEW.sha256_servi IS DISTINCT FROM OLD.sha256_servi
            OR NEW.nb_pages IS DISTINCT FROM OLD.nb_pages) THEN
        RAISE EXCEPTION 'ent_doc_piece : le fichier servi ne change plus' USING ERRCODE = '23001';
    END IF;
    IF NEW.etat IS DISTINCT FROM OLD.etat THEN
        IF NOT (
               (OLD.etat = 'en_quarantaine' AND NEW.etat IN ('recue', 'infectee', 'rejetee', 'retiree'))
            OR (OLD.etat = 'recue' AND NEW.etat IN ('acceptee', 'refusee', 'sans_objet', 'retiree'))) THEN
            RAISE EXCEPTION 'ent_doc_piece : transition % → % interdite', OLD.etat, NEW.etat USING ERRCODE = '23001';
        END IF;
        IF OLD.etat = 'en_quarantaine' AND NEW.etat = 'recue' AND NEW.cle_objet_servi IS NULL THEN
            RAISE EXCEPTION 'ent_doc_piece : une pièce ne sort de quarantaine qu''avec son fichier servi' USING ERRCODE = '23001';
        END IF;
    ELSIF OLD.etat IN ('acceptee', 'refusee', 'sans_objet', 'infectee', 'rejetee', 'retiree')
          AND (NEW.valable_jusqu_au IS DISTINCT FROM OLD.valable_jusqu_au
               OR NEW.controle_par IS DISTINCT FROM OLD.controle_par OR NEW.controle_le IS DISTINCT FROM OLD.controle_le
               OR NEW.refus_motif IS DISTINCT FROM OLD.refus_motif OR NEW.commentaire IS DISTINCT FROM OLD.commentaire) THEN
        RAISE EXCEPTION 'ent_doc_piece : décision finale, plus modifiable' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_piece_garde_trg ON ent_doc_piece;
CREATE TRIGGER ent_doc_piece_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_piece
    FOR EACH ROW EXECUTE FUNCTION ent_doc_piece_garde();

-- La demande ne sort de « ouverte » qu'une fois ; identité figée.
CREATE OR REPLACE FUNCTION ent_doc_piece_demande_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_piece_demande : suppression interdite (annuler avec un motif)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.type_code IS DISTINCT FROM OLD.type_code
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
       OR NEW.portee IS DISTINCT FROM OLD.portee
       OR NEW.brique_demandeuse IS DISTINCT FROM OLD.brique_demandeuse
       OR NEW.cree_par IS DISTINCT FROM OLD.cree_par
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'ent_doc_piece_demande : identité et sujet immuables' USING ERRCODE = '23001';
    END IF;
    IF OLD.etat <> 'ouverte' AND (NEW.etat IS DISTINCT FROM OLD.etat OR NEW.piece_id IS DISTINCT FROM OLD.piece_id
                                  OR NEW.cloturee_le IS DISTINCT FROM OLD.cloturee_le) THEN
        RAISE EXCEPTION 'ent_doc_piece_demande : demande close, plus modifiable' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_piece_demande_garde_trg ON ent_doc_piece_demande;
CREATE TRIGGER ent_doc_piece_demande_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_piece_demande
    FOR EACH ROW EXECUTE FUNCTION ent_doc_piece_demande_garde();

-- La clé composite vers la pièce est posée après la création des deux tables.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_doc_piece_demande_piece_cfk') THEN
        ALTER TABLE ent_doc_piece_demande
            ADD CONSTRAINT ent_doc_piece_demande_piece_cfk FOREIGN KEY (etablissement_id, piece_id)
            REFERENCES ent_doc_piece (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;

-- ───────────────────────── Décisions et faits d'analyse (ajout seul) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_piece_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    piece_id UUID NOT NULL,
    demande_id UUID,
    decision TEXT NOT NULL CHECK (decision IN (
        'deposee', 'analysee_saine', 'infectee', 'rejetee', 'analyse_indisponible',
        'acceptee', 'refusee', 'sans_objet', 'retiree', 'reutilisee')),
    motif_code TEXT CHECK (motif_code IS NULL OR length(motif_code) <= 60),
    commentaire TEXT CHECK (commentaire IS NULL OR length(commentaire) <= 500),
    acteur_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_piece_decision_piece_cfk FOREIGN KEY (etablissement_id, piece_id)
        REFERENCES ent_doc_piece (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_doc_piece_decision_piece_idx
    ON ent_doc_piece_decision (etablissement_id, piece_id, le);
DROP TRIGGER IF EXISTS ent_doc_piece_decision_ajout_seul_trg ON ent_doc_piece_decision;
CREATE TRIGGER ent_doc_piece_decision_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_piece_decision
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_doc_piece_demande',
    'ent_doc_piece',
    'ent_doc_piece_decision'
]);
