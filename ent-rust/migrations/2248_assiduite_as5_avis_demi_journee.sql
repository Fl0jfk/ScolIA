-- 2248 — Socle 12, lot AS5 : registre des avis d'absence par DEMI-JOURNÉE. Réf. : 12_ASSIDUITE.md §14.1,
-- §14.2, R-AS13, R-AS14, CA (avis par foyer et demi-journée, démenti).
-- Le moteur de notifications (socle 8) garde l'avis lui-même (`ent_ntf_avis`) ; cette table dit seulement
-- QUELLE demi-journée de QUEL élève un avis couvre, pour qu'une journée d'absence ne donne jamais plus de
-- deux avis (un par demi-journée) et pour retrouver l'avis à démentir quand l'absence est corrigée.
-- Une demi-journée n'a qu'UN avis vivant à la fois (index unique partiel) ; un avis annulé ou démenti
-- laisse la place à un nouveau (génération suivante, par exemple une absence constatée plus tard).
-- Additive, rejouable (E8), sans colonne JSON (D-036), sans BYTEA ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_avis_demi_journee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    jour DATE NOT NULL,
    periode TEXT NOT NULL CHECK (periode IN ('matin', 'apres_midi')),
    generation SMALLINT NOT NULL DEFAULT 1 CHECK (generation >= 1),
    avis_id UUID NOT NULL,
    -- programme : l'avis est planifié ou remis et reste exact ; annule : retiré avant toute remise (rien
    -- n'est parti, aucun démenti) ; dementi : l'avis était parti, un démenti a été émis.
    statut TEXT NOT NULL DEFAULT 'programme' CHECK (statut IN ('programme', 'annule', 'dementi')),
    cause_code TEXT CHECK (cause_code IS NULL OR cause_code IN (
        'correction', 'neutralisation', 'localisation', 'retard', 'present', 'sujet_gele')),
    dementi_avis_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_avis_dj_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_avis_dj_generation_uq UNIQUE (etablissement_id, eleve_id, jour, periode, generation),
    CONSTRAINT ent_asd_avis_dj_statut_chk CHECK (
        (statut = 'programme' AND cause_code IS NULL AND dementi_avis_id IS NULL)
        OR (statut = 'annule' AND dementi_avis_id IS NULL)
        OR (statut = 'dementi' AND cause_code IS NOT NULL)),
    CONSTRAINT ent_asd_avis_dj_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_avis_dj_avis_fk FOREIGN KEY (etablissement_id, avis_id)
        REFERENCES ent_ntf_avis (etablissement_id, id),
    CONSTRAINT ent_asd_avis_dj_dementi_fk FOREIGN KEY (etablissement_id, dementi_avis_id)
        REFERENCES ent_ntf_avis (etablissement_id, id)
);
-- Un seul avis vivant par élève et demi-journée : au plus deux avis par jour (R-AS13).
CREATE UNIQUE INDEX IF NOT EXISTS ent_asd_avis_dj_vivant_uq
    ON ent_asd_avis_demi_journee (etablissement_id, eleve_id, jour, periode) WHERE statut = 'programme';
CREATE INDEX IF NOT EXISTS ent_asd_avis_dj_jour_idx
    ON ent_asd_avis_demi_journee (etablissement_id, eleve_id, jour);

SELECT ent_rls_poser(ARRAY['ent_asd_avis_demi_journee']);
