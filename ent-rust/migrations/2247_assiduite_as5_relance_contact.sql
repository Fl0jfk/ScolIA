-- 2247 — Socle 12, lot AS5 (familles et notifications). Réf. : 12_ASSIDUITE.md §10.5, §14.4, R-AS18, R-AS25.
-- Deux tables : la RELANCE GRADUÉE d'un épisode sans motif (une ligne par étape, jamais deux fois la même),
-- et le CONTACT tracé avec la famille (un geste = une ligne, en ajout seul). Aucun texte d'avis ici : le
-- moteur de notifications (socle 8) garde les avis, les destinataires et les remises ; ces tables ne
-- gardent que le suivi de la vie scolaire. Additive, rejouable (E8), sans colonne JSON (D-036), sans BYTEA ;
-- RLS par `ent_rls_poser` (580), clés étrangères composites (établissement, id).

CREATE TABLE IF NOT EXISTS ent_asd_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    -- Étapes de §10.5 : J+1 rappel (ENT, push), J+3 canal externe (SMS ou courriel), J+5 « sans réponse »
    -- (courrier postal à décider), J+8 proposition de qualification « sans motif » (et de 1° de L131-8).
    etape TEXT NOT NULL CHECK (etape IN ('rappel', 'canal_externe', 'sans_reponse', 'proposition_sans_motif')),
    -- Jour scolaire de l'établissement visé par l'étape (jamais la date UTC du serveur).
    jour_prevu DATE NOT NULL,
    statut TEXT NOT NULL CHECK (statut IN ('envoyee', 'proposee', 'sautee')),
    -- Pourquoi une étape a été sautée (liste fermée) ; vide pour une étape faite. `rattrape` : une étape plus
    -- avancée était déjà due (tâche arrêtée, établissement activé tard) : on ne relance pas trois fois d'un coup.
    saut_code TEXT CHECK (saut_code IS NULL OR saut_code IN (
        'rattrape', 'nature_desactivee', 'sujet_gele', 'sans_destinataire')),
    avis_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_relance_etab_id_uq UNIQUE (etablissement_id, id),
    -- Une étape ne part qu'une fois par épisode (rejeu et passages répétés de la tâche : aucun doublon).
    CONSTRAINT ent_asd_relance_etape_uq UNIQUE (etablissement_id, absence_id, etape),
    CONSTRAINT ent_asd_relance_saut_chk CHECK ((statut = 'sautee') = (saut_code IS NOT NULL)),
    CONSTRAINT ent_asd_relance_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_relance_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_relance_avis_fk FOREIGN KEY (etablissement_id, avis_id)
        REFERENCES ent_ntf_avis (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_relance_eleve_idx ON ent_asd_relance (etablissement_id, eleve_id, cree_le);

-- Contact avec la famille : appel, SMS, courriel, rencontre, courrier (§14.4). Ajout seul : on ne réécrit
-- pas ce qui a été dit ; une erreur se corrige par un nouveau contact.
CREATE TABLE IF NOT EXISTS ent_asd_contact (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    -- Épisode concerné (facultatif : un contact peut porter sur l'élève en général).
    absence_id UUID,
    -- Responsable joint (facultatif : « ne répond pas » sans savoir qui).
    personne_id UUID,
    canal TEXT NOT NULL CHECK (canal IN ('telephone', 'sms', 'courriel', 'rencontre', 'courrier')),
    sens TEXT NOT NULL DEFAULT 'sortant' CHECK (sens IN ('sortant', 'entrant')),
    issue TEXT NOT NULL CHECK (issue IN ('joint', 'message_laisse', 'ne_repond_pas', 'numero_faux', 'envoye', 'rencontre_faite')),
    -- Note interne courte : jamais visible de la famille ni du professeur (R-AS22).
    note_interne TEXT CHECK (note_interne IS NULL OR length(btrim(note_interne)) BETWEEN 1 AND 300),
    auteur_user_id UUID NOT NULL,
    -- Un numéro faux crée une tâche au secrétariat (socle 4) : elle se clôt une seule fois.
    tache_numero_le TIMESTAMPTZ,
    tache_numero_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_contact_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_contact_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_contact_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_contact_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_asd_contact_numero_chk CHECK (
        tache_numero_le IS NULL OR (issue = 'numero_faux' AND tache_numero_par IS NOT NULL)),
    CONSTRAINT ent_asd_contact_issue_canal_chk CHECK (
        (issue = 'numero_faux' AND canal IN ('telephone', 'sms'))
        OR (issue = 'envoye' AND canal IN ('sms', 'courriel', 'courrier'))
        OR (issue = 'rencontre_faite' AND canal = 'rencontre')
        OR issue IN ('joint', 'message_laisse', 'ne_repond_pas'))
);
CREATE INDEX IF NOT EXISTS ent_asd_contact_eleve_idx ON ent_asd_contact (etablissement_id, eleve_id, cree_le DESC);
CREATE INDEX IF NOT EXISTS ent_asd_contact_absence_idx ON ent_asd_contact (etablissement_id, absence_id, cree_le)
    WHERE absence_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_asd_contact_numero_idx ON ent_asd_contact (etablissement_id, cree_le)
    WHERE issue = 'numero_faux' AND tache_numero_le IS NULL;

-- Ajout seul, sauf la clôture de la tâche « numéro faux » (posée une fois, sans autre changement).
CREATE OR REPLACE FUNCTION ent_asd_contact_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_asd_contact est en ajout seul (DELETE interdit)' USING ERRCODE = '23514';
    END IF;
    IF OLD.tache_numero_le IS NULL AND NEW.tache_numero_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.eleve_id = OLD.eleve_id
       AND NEW.absence_id IS NOT DISTINCT FROM OLD.absence_id AND NEW.personne_id IS NOT DISTINCT FROM OLD.personne_id
       AND NEW.canal = OLD.canal AND NEW.sens = OLD.sens AND NEW.issue = OLD.issue
       AND NEW.note_interne IS NOT DISTINCT FROM OLD.note_interne AND NEW.auteur_user_id = OLD.auteur_user_id
       AND NEW.cree_le = OLD.cree_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_asd_contact est en ajout seul (UPDATE interdit)' USING ERRCODE = '23514';
END
$$;
DROP TRIGGER IF EXISTS ent_asd_contact_garde_trg ON ent_asd_contact;
CREATE TRIGGER ent_asd_contact_garde_trg BEFORE UPDATE OR DELETE ON ent_asd_contact
    FOR EACH ROW EXECUTE FUNCTION ent_asd_contact_garde();

SELECT ent_rls_poser(ARRAY['ent_asd_relance', 'ent_asd_contact']);
