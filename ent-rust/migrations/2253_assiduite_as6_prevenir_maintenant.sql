-- 2253 — Socle 12, lot AS6. Réf. : 12_ASSIDUITE.md AS-155, CA-50, R-AS13, 08_NOTIFICATIONS.md (trou « absence
-- inquiétante »). « Prévenir maintenant » : la direction ou la CPE déclenche, pour une absence inquiétante
-- (fugue, élève signalé en danger), un avis de la brique de notifications (socle 8) en urgence U3, sans délai de
-- confirmation. La décision est TRACÉE : qui, quand, pour quel élève et quel jour, raison codée, avis émis.
-- Aucun texte libre (R-AS25). Ajout seul (déclencheur). Cette table référence `ent_ntf_avis` (3801) : elle est
-- appliquée après les migrations 38xx (voir le tableau `MIGRATIONS`).
-- Additive, rejouable (E8), sans colonne JSON ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_prevenir (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    jour DATE NOT NULL,
    raison_code TEXT NOT NULL CHECK (raison_code IN (
        'fugue_suspectee', 'eleve_en_danger', 'absence_inexpliquee_grave', 'demande_de_la_famille', 'autre_urgence')),
    auteur_user_id UUID NOT NULL,
    -- Avis U3 émis par le moteur (jamais un envoi direct) ; avis de demi-journée ordinaire retiré s'il n'était
    -- pas encore parti (la famille ne reçoit pas deux messages pour le même fait).
    avis_id UUID NOT NULL,
    avis_remplace_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_prevenir_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_prevenir_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_prevenir_avis_fk FOREIGN KEY (etablissement_id, avis_id)
        REFERENCES ent_ntf_avis (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_prevenir_eleve_idx ON ent_asd_prevenir (etablissement_id, eleve_id, jour);

DROP TRIGGER IF EXISTS ent_asd_prevenir_ajout_seul ON ent_asd_prevenir;
CREATE TRIGGER ent_asd_prevenir_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_prevenir
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_asd_prevenir']);

-- Le registre des avis par demi-journée (AS5) apprend une cause de plus : l'avis ordinaire retiré avant son
-- départ parce que « prévenir maintenant » a alerté la famille en urgence. Seule la liste fermée des causes change.
ALTER TABLE ent_asd_avis_demi_journee DROP CONSTRAINT IF EXISTS ent_asd_avis_demi_journee_cause_code_check;
ALTER TABLE ent_asd_avis_demi_journee ADD CONSTRAINT ent_asd_avis_demi_journee_cause_code_check CHECK (
    cause_code IS NULL OR cause_code IN (
        'correction', 'neutralisation', 'localisation', 'retard', 'present', 'sujet_gele', 'urgence'));
