-- 2245 — Socle 12, lot AS3 (justificatifs, demandes, propositions). Réf. : 12_ASSIDUITE.md §10.3, §10.4,
-- §17.3 (AS-124 à AS-128), §22.4, R-AS19, R-AS22, E5, E7.
-- Justificatif : pièce FACULTATIVE (RA-08) ; l'objet est immuable dans `ent-object` (clé opaque
-- `{etablissement}/assiduite/{uuid}`, empreinte) ; AUCUN nom de fichier n'est conservé ; une pièce
-- médicale est classée C3 ; conservation un an (RA-34) puis destruction avec trace d'existence.
-- Additive, rejouable (E8), sans colonne JSON (D-036), sans BYTEA ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_justificatif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    presente TEXT NOT NULL CHECK (presente IN ('depose', 'texte', 'vu_non_conserve', 'retire')),
    -- Pièce : clé d'objet générée par le serveur (jamais fournie par le client, NJ-29/NJ-39).
    cle_objet TEXT,
    sha256 TEXT CHECK (sha256 IS NULL OR sha256 ~ '^[0-9a-f]{64}$'),
    taille_octets BIGINT CHECK (taille_octets IS NULL OR taille_octets > 0),
    mime TEXT CHECK (mime IS NULL OR char_length(mime) BETWEEN 3 AND 100),
    -- « Mot des parents » saisi en texte (justificatif texte seul).
    texte TEXT CHECK (texte IS NULL OR length(btrim(texte)) BETWEEN 1 AND 1000),
    medical BOOLEAN NOT NULL DEFAULT false,
    depose_par UUID NOT NULL,
    depose_qualite TEXT NOT NULL CHECK (depose_qualite IN ('famille', 'eleve_majeur', 'accueil', 'vie_scolaire')),
    conserve_jusqu_au DATE,
    retire_par UUID,
    retire_le TIMESTAMPTZ,
    detruit_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_justificatif_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_justificatif_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_justificatif_forme_chk CHECK (
        (presente = 'depose' AND cle_objet IS NOT NULL AND sha256 IS NOT NULL AND taille_octets IS NOT NULL AND mime IS NOT NULL)
        OR (presente = 'texte' AND texte IS NOT NULL)
        OR presente IN ('vu_non_conserve', 'retire')),
    CONSTRAINT ent_asd_justificatif_cle_chk CHECK (cle_objet IS NULL OR starts_with(cle_objet, etablissement_id::text || '/assiduite/'))
);
CREATE INDEX IF NOT EXISTS ent_asd_justificatif_absence_idx ON ent_asd_justificatif (etablissement_id, absence_id, cree_le);
CREATE INDEX IF NOT EXISTS ent_asd_justificatif_purge_idx ON ent_asd_justificatif (etablissement_id, conserve_jusqu_au)
    WHERE detruit_le IS NULL AND cle_objet IS NOT NULL;

-- Un justificatif ne se supprime jamais : il passe `retire`, puis sa pièce est détruite (trace conservée).
CREATE OR REPLACE FUNCTION ent_asd_justificatif_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_asd_justificatif ne se supprime pas (retrait puis destruction de la pièce)' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.absence_id IS DISTINCT FROM OLD.absence_id OR NEW.cle_objet IS DISTINCT FROM OLD.cle_objet
       OR NEW.sha256 IS DISTINCT FROM OLD.sha256 OR NEW.depose_par IS DISTINCT FROM OLD.depose_par THEN
        RAISE EXCEPTION 'les champs d''origine d''un justificatif sont figés' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_justificatif_garde_trg ON ent_asd_justificatif;
CREATE TRIGGER ent_asd_justificatif_garde_trg BEFORE UPDATE OR DELETE ON ent_asd_justificatif
    FOR EACH ROW EXECUTE FUNCTION ent_asd_justificatif_garde();

-- Qui a lu quelle pièce (E7) : ajout seul, conservé après la destruction de la pièce.
CREATE TABLE IF NOT EXISTS ent_asd_justificatif_lecture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    justificatif_id UUID NOT NULL,
    lecteur_user_id UUID NOT NULL,
    medical BOOLEAN NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_justificatif_lecture_fk FOREIGN KEY (etablissement_id, justificatif_id)
        REFERENCES ent_asd_justificatif (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_justificatif_lecture_idx ON ent_asd_justificatif_lecture (etablissement_id, justificatif_id, cree_le);
DROP TRIGGER IF EXISTS ent_asd_justificatif_lecture_ajout_seul ON ent_asd_justificatif_lecture;
CREATE TRIGGER ent_asd_justificatif_lecture_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_justificatif_lecture
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

-- Propositions de qualification (AS-124) : l'AED propose, la vie scolaire décide. Ajout seul pour la
-- proposition ; seule la décision (état, décideur, date) se complète.
CREATE TABLE IF NOT EXISTS ent_asd_proposition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    qualification TEXT NOT NULL CHECK (qualification IN ('reputee_legitime', 'excuse_valable', 'non_valable', 'sans_motif')),
    note TEXT CHECK (note IS NULL OR length(btrim(note)) BETWEEN 1 AND 500),
    proposee_par UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'en_attente' CHECK (etat IN ('en_attente', 'acceptee', 'ecartee')),
    decidee_par UUID,
    decidee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_proposition_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_proposition_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_proposition_decision_chk CHECK ((etat = 'en_attente') = (decidee_le IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_asd_proposition_idx ON ent_asd_proposition (etablissement_id, absence_id, cree_le);

-- Demandes (AS-125) : autorisation d'absence (fête religieuse, convocation, compétition…) ; les types
-- « dispense » (AS8) et « correction_appel » (AS-132) sont réservés aux lots suivants.
CREATE TABLE IF NOT EXISTS ent_asd_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('autorisation', 'dispense', 'correction_appel')),
    jour_debut DATE NOT NULL,
    jour_fin DATE NOT NULL,
    heure_debut TIME,
    heure_fin TIME,
    motif_id UUID,
    demandeur_user_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'en_attente' CHECK (etat IN ('en_attente', 'accordee', 'refusee', 'retiree', 'transmise_dasen')),
    decision_code TEXT CHECK (decision_code IS NULL OR decision_code IN (
        'accordee', 'refusee_motif_non_recevable', 'refusee_absences_repetees', 'refusee_delai',
        'refusee_autre', 'transmise_dasen')),
    decide_par UUID,
    decide_le TIMESTAMPTZ,
    situation_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_demande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_demande_jours_chk CHECK (jour_fin >= jour_debut AND jour_fin - jour_debut <= 120),
    CONSTRAINT ent_asd_demande_heures_chk CHECK (
        jour_fin > jour_debut OR heure_debut IS NULL OR heure_fin IS NULL OR heure_fin > heure_debut),
    CONSTRAINT ent_asd_demande_decision_chk CHECK (
        ((etat = 'en_attente') = (decide_le IS NULL)) OR etat = 'retiree'),
    CONSTRAINT ent_asd_demande_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_demande_motif_fk FOREIGN KEY (etablissement_id, motif_id)
        REFERENCES ent_asd_motif (etablissement_id, id),
    CONSTRAINT ent_asd_demande_situation_fk FOREIGN KEY (etablissement_id, situation_id)
        REFERENCES ent_asd_situation (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_demande_eleve_idx ON ent_asd_demande (etablissement_id, eleve_id, cree_le);
CREATE INDEX IF NOT EXISTS ent_asd_demande_file_idx ON ent_asd_demande (etablissement_id, cree_le) WHERE etat = 'en_attente';

SELECT ent_rls_poser(ARRAY['ent_asd_justificatif', 'ent_asd_justificatif_lecture', 'ent_asd_proposition', 'ent_asd_demande']);
