-- 641 — Socle 42, lot V0a : participants (R-SV2 : toujours un eleve_id du cœur, jamais d'empreinte
-- de nom) et leur fil d'états daté, en AJOUT SEUL. Réf. : 42_SORTIES_VOYAGES.md §9.1, §9.2.

CREATE TABLE IF NOT EXISTS ent_sortie_participant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'inscrit' CHECK (etat IN (
        'propose', 'inscrit', 'liste_attente', 'pret', 'bloque', 'parti', 'absent_depart',
        'rentre', 'remis', 'parti_seul_autorise', 'desiste', 'retire', 'exclu_disciplinaire')),
    rang_attente INTEGER CHECK (rang_attente IS NULL OR rang_attente > 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_participant_attente_chk CHECK ((etat = 'liste_attente') = (rang_attente IS NOT NULL)),
    CONSTRAINT ent_sortie_participant_uq UNIQUE (sortie_id, eleve_id),
    CONSTRAINT ent_sortie_participant_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_participant_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_participant_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_participant_eleve_idx
    ON ent_sortie_participant (etablissement_id, eleve_id);

CREATE TABLE IF NOT EXISTS ent_sortie_participant_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    participant_id UUID NOT NULL,
    a TIMESTAMPTZ NOT NULL DEFAULT now(),
    acteur_user_id UUID,
    type TEXT NOT NULL CHECK (type IN ('inscription', 'etat', 'desistement', 'retrait', 'reintegration')),
    etat_avant TEXT,
    etat_apres TEXT NOT NULL,
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'sante', 'famille', 'decision_personnelle', 'financier', 'exigence_manquante',
        'decision_etablissement', 'autre')),
    motif_texte TEXT CHECK (motif_texte IS NULL OR length(motif_texte) <= 1000),
    CONSTRAINT ent_sortie_participant_fait_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_participant_fait_idx
    ON ent_sortie_participant_fait (etablissement_id, participant_id, a);

DROP TRIGGER IF EXISTS ent_sortie_participant_fait_immuable ON ent_sortie_participant_fait;
CREATE TRIGGER ent_sortie_participant_fait_immuable
    BEFORE UPDATE OR DELETE ON ent_sortie_participant_fait
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_sortie_participant', 'ent_sortie_participant_fait']);
