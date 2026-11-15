-- Socle 7, lot D2 — migration 3703 : mises à disposition, consultations, journal. Réf. : 07_DOCUMENTS.md §8.5, §8.14
-- (notification effective, R-D10), §17.2, §19.
--
-- Mise à disposition : fait qu'un exemplaire est accessible à une personne (coffre), imprimé, remis au guichet ou posté.
-- Elle porte l'accusé (première ouverture, remise) et la date de NOTIFICATION que la nature retient (le délai d'une
-- procédure court depuis la notification, jamais depuis l'émission, D80). Une mise à disposition ne se supprime pas :
-- elle se RETIRE (motif, date).

CREATE TABLE IF NOT EXISTS ent_doc_mise_a_disposition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    document_id UUID NOT NULL,
    exemplaire_id UUID NOT NULL,
    -- Titulaire du coffre : une seule des trois colonnes.
    personne_id UUID,
    agent_id UUID,
    eleve_id UUID,
    canal TEXT NOT NULL CHECK (canal IN ('coffre', 'papier', 'guichet', 'courrier_postal', 'externe')),
    depuis TIMESTAMPTZ NOT NULL DEFAULT now(),
    jusqu_au TIMESTAMPTZ,
    retiree_le TIMESTAMPTZ,
    retrait_motif TEXT CHECK (retrait_motif IS NULL OR length(btrim(retrait_motif)) BETWEEN 3 AND 500),
    premiere_ouverture_le TIMESTAMPTZ,
    remise_le TIMESTAMPTZ,
    notifie_le TIMESTAMPTZ,
    mode_notification TEXT CHECK (mode_notification IS NULL OR mode_notification IN (
        'ouverture_coffre', 'remise_contre_signature', 'avis_reception', 'remise_guichet', 'envoi_postal')),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_mad_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_doc_mad_document_cfk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_mad_exemplaire_cfk FOREIGN KEY (etablissement_id, exemplaire_id)
        REFERENCES ent_doc_exemplaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_mad_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_mad_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_mad_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_mad_titulaire_chk CHECK (num_nonnulls(personne_id, agent_id, eleve_id) = 1),
    CONSTRAINT ent_doc_mad_retrait_chk CHECK ((retiree_le IS NULL) = (retrait_motif IS NULL)),
    CONSTRAINT ent_doc_mad_notification_chk CHECK ((notifie_le IS NULL) = (mode_notification IS NULL))
);
-- Un exemplaire n'est mis qu'une fois à disposition par canal et par titulaire (une seule des trois colonnes est
-- renseignée : COALESCE donne le titulaire).
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_mad_uq
    ON ent_doc_mise_a_disposition (exemplaire_id, canal, COALESCE(personne_id, agent_id, eleve_id));
CREATE INDEX IF NOT EXISTS ent_doc_mad_personne_idx
    ON ent_doc_mise_a_disposition (etablissement_id, personne_id, depuis DESC) WHERE personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_mad_eleve_idx
    ON ent_doc_mise_a_disposition (etablissement_id, eleve_id, depuis DESC) WHERE eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_doc_mad_document_idx
    ON ent_doc_mise_a_disposition (etablissement_id, document_id);

-- Seuls les accusés et le retrait bougent, chacun une seule fois (NULL → valeur) ; le reste est figé ; pas de DELETE.
CREATE OR REPLACE FUNCTION ent_doc_mad_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_mise_a_disposition : suppression interdite (retirer avec un motif)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.document_id IS DISTINCT FROM OLD.document_id
       OR NEW.exemplaire_id IS DISTINCT FROM OLD.exemplaire_id
       OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.canal IS DISTINCT FROM OLD.canal
       OR NEW.depuis IS DISTINCT FROM OLD.depuis
       OR NEW.cree_par IS DISTINCT FROM OLD.cree_par
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le
       OR (OLD.retiree_le IS NOT NULL AND (NEW.retiree_le IS DISTINCT FROM OLD.retiree_le
                                           OR NEW.retrait_motif IS DISTINCT FROM OLD.retrait_motif))
       OR (OLD.premiere_ouverture_le IS NOT NULL AND NEW.premiere_ouverture_le IS DISTINCT FROM OLD.premiere_ouverture_le)
       OR (OLD.remise_le IS NOT NULL AND NEW.remise_le IS DISTINCT FROM OLD.remise_le)
       OR (OLD.notifie_le IS NOT NULL AND (NEW.notifie_le IS DISTINCT FROM OLD.notifie_le
                                           OR NEW.mode_notification IS DISTINCT FROM OLD.mode_notification)) THEN
        RAISE EXCEPTION 'ent_doc_mise_a_disposition : seuls les accusés et le retrait s''ajoutent, une fois'
            USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_mad_garde_trg ON ent_doc_mise_a_disposition;
CREATE TRIGGER ent_doc_mad_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_mise_a_disposition
    FOR EACH ROW EXECUTE FUNCTION ent_doc_mad_garde();

-- ───────────────────────── Consultations (ajout seul) ─────────────────────────
-- Alimente le journal des accès visible de la famille (socle 27, A11). Lecture, téléchargement, impression et
-- vérification des documents C2 et plus y laissent une ligne.
CREATE TABLE IF NOT EXISTS ent_doc_consultation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    document_id UUID NOT NULL,
    version_id UUID NOT NULL,
    acteur_user_id UUID NOT NULL,
    profil TEXT NOT NULL CHECK (length(profil) BETWEEN 1 AND 40),
    canal TEXT NOT NULL CHECK (canal IN ('web', 'mobile', 'guichet', 'systeme')),
    motif TEXT NOT NULL CHECK (motif IN ('lecture', 'telechargement', 'impression', 'verification')),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_consultation_document_cfk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_doc_consultation_version_cfk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_doc_version (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_doc_consultation_document_idx
    ON ent_doc_consultation (etablissement_id, document_id, le DESC);
DROP TRIGGER IF EXISTS ent_doc_consultation_ajout_seul_trg ON ent_doc_consultation;
CREATE TRIGGER ent_doc_consultation_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_consultation
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

-- ───────────────────────── Journal (ajout seul) ─────────────────────────
-- Un fait, une ligne, dans la transaction du fait : émission, réémission, annulation, mise à disposition, retrait,
-- modification du paramétrage. Texte seulement (jamais le contenu d'un document).
CREATE TABLE IF NOT EXISTS ent_doc_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    document_id UUID,
    numero_version INT,
    action TEXT NOT NULL CHECK (action ~ '^[a-z][a-z0-9_]{0,59}$'),
    acteur_user_id UUID NOT NULL,
    etat_avant TEXT,
    etat_apres TEXT,
    cible TEXT CHECK (cible IS NULL OR length(cible) <= 200),
    avant TEXT CHECK (avant IS NULL OR length(avant) <= 2000),
    apres TEXT CHECK (apres IS NULL OR length(apres) <= 2000),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_journal_document_cfk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_doc_journal_document_idx ON ent_doc_journal (etablissement_id, document_id, le);
CREATE INDEX IF NOT EXISTS ent_doc_journal_le_idx ON ent_doc_journal (etablissement_id, le DESC);
DROP TRIGGER IF EXISTS ent_doc_journal_ajout_seul_trg ON ent_doc_journal;
CREATE TRIGGER ent_doc_journal_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_doc_journal
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_doc_mise_a_disposition',
    'ent_doc_consultation',
    'ent_doc_journal'
]);
