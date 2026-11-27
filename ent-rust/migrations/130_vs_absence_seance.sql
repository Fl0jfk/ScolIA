-- Socle 2, lot S2-4 (D-020) : absences à la SÉANCE, demi-journées DÉRIVÉES.
--
-- Additive : `ent_absence` reste l'unité de justification (statut, décision) ;
-- `ent_absence_seance` détaille ce qui a été manqué (une séance, ou une
-- demi-journée entière pour les absences historiques). Les demi-journées ne
-- sont jamais saisies : elles sont dérivées (ent-db/src/assiduite.rs) selon les
-- règles de l'établissement ci-dessous.
--
-- Règles réglementaires [H, à vérifier sur les textes officiels avant pilote] :
-- toutes sont paramétrables, aucune n'est figée dans le code.

CREATE TABLE IF NOT EXISTS ent_vs_regle_assiduite (
    etablissement_id UUID PRIMARY KEY,
    -- Heure de coupure matin / après-midi (une séance à cheval est ventilée).
    heure_coupure TIME NOT NULL DEFAULT '12:00',
    -- Minutes d'absence cumulées dans une demi-journée pour qu'elle compte.
    seuil_minutes INTEGER NOT NULL DEFAULT 30,
    -- Nombre de demi-journées non justifiées dans le mois déclenchant le signalement.
    seuil_signalement INTEGER NOT NULL DEFAULT 4,
    -- Délai (minutes après le début de séance) avant de considérer un appel « non fait ».
    delai_verification_minutes INTEGER NOT NULL DEFAULT 15,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_vs_regle_seuil_chk CHECK (seuil_minutes BETWEEN 1 AND 600),
    CONSTRAINT ent_vs_regle_signal_chk CHECK (seuil_signalement BETWEEN 1 AND 62),
    CONSTRAINT ent_vs_regle_delai_chk CHECK (delai_verification_minutes BETWEEN 0 AND 240)
);

CREATE TABLE IF NOT EXISTS ent_absence_seance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    absence_id UUID NOT NULL REFERENCES ent_absence (id) ON DELETE CASCADE,
    jour DATE NOT NULL,
    -- 'seance' : séance datée ; 'demi_journee' : absence historique à la journée.
    kind TEXT NOT NULL DEFAULT 'seance',
    periode TEXT,
    -- Séance : identifiant du créneau EDT (séance provisoire, S2-2 le remplacera
    -- par la séance datée du référentiel) et horaires figés au moment du constat.
    seance_ref UUID,
    seance_source TEXT NOT NULL DEFAULT 'edt_creneau',
    heure_debut TIME,
    heure_fin TIME,
    appel_id UUID,
    source TEXT NOT NULL DEFAULT 'appel',
    -- Déclarée à l'avance (accueil / famille) : voir ent_absence_rapprochement.
    prevenue BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absence_seance_kind_chk CHECK (kind IN ('seance', 'demi_journee')),
    CONSTRAINT ent_absence_seance_periode_chk CHECK (periode IS NULL OR periode IN ('matin', 'apres_midi')),
    CONSTRAINT ent_absence_seance_source_chk
        CHECK (source IN ('appel', 'accueil', 'famille', 'vie_scolaire', 'reprise')),
    CONSTRAINT ent_absence_seance_forme_chk CHECK (
        (kind = 'seance' AND heure_debut IS NOT NULL AND heure_fin IS NOT NULL
            AND heure_fin > heure_debut AND periode IS NULL)
        OR (kind = 'demi_journee' AND periode IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_absence_seance_uq
    ON ent_absence_seance (
        absence_id, jour, kind,
        COALESCE(seance_ref, '00000000-0000-0000-0000-000000000000'::uuid),
        COALESCE(periode, '')
    );

CREATE INDEX IF NOT EXISTS ent_absence_seance_eleve_jour_idx
    ON ent_absence_seance (etablissement_id, eleve_id, jour);

CREATE INDEX IF NOT EXISTS ent_absence_seance_jour_idx
    ON ent_absence_seance (etablissement_id, jour);

-- Reprise des absences existantes (à la journée) : chaque jour ouvré (lundi à
-- vendredi, plafond d'un an par absence) devient deux demi-journées. Idempotente :
-- une absence qui a déjà des lignes n'est pas retouchée. Les jours fériés et les
-- vacances ne sont pas exclus (limite connue, cf. docs/SOCLE2_DOMAINES.md §6.2).
INSERT INTO ent_absence_seance (
    id, etablissement_id, eleve_id, absence_id, jour, kind, periode,
    seance_source, source
)
SELECT gen_random_uuid(), a.etablissement_id, a.eleve_id, a.id, d::date,
       'demi_journee', p.periode, 'reprise', 'reprise'
FROM ent_absence a
CROSS JOIN LATERAL generate_series(
    a.debut::timestamp,
    LEAST(a.fin, a.debut + 365)::timestamp,
    interval '1 day'
) AS d
CROSS JOIN (VALUES ('matin'), ('apres_midi')) AS p (periode)
WHERE EXTRACT(ISODOW FROM d) < 6
  AND NOT EXISTS (SELECT 1 FROM ent_absence_seance s WHERE s.absence_id = a.id)
ON CONFLICT DO NOTHING;
