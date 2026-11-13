-- 2242 — Socle 12, lot AS2 (appel v2). Réf. : 12_ASSIDUITE.md §8, §9, §22.2, §29.3.
-- Appel typé (cours, permanence…), états de ligne étendus, historique des lignes en AJOUT SEUL,
-- délégation datée. Additive et rejouable (E8) ; aucune colonne JSON ; aucun BYTEA.
-- `seance_id` : colonne sans clé étrangère tant que la séance datée du socle 10 n'existe pas.

ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS seance_id UUID;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS type_activite TEXT NOT NULL DEFAULT 'cours';
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS activite_cle TEXT;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS activite_ref_type TEXT;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS activite_ref_id UUID;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS heure_debut TIME;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS heure_fin TIME;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'en_cours';
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS mode_pointage TEXT NOT NULL DEFAULT 'tous_presents';
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS effectif_complet BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS ouvert_par UUID;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS canal_validation TEXT;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS delegue_a UUID;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS neutralise_le TIMESTAMPTZ;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS neutralise_cause TEXT;
ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();

-- Reprise : état v2 déduit du statut historique (« clos » = validé).
UPDATE ent_appel SET etat = 'valide' WHERE statut = 'clos' AND etat = 'en_cours';
UPDATE ent_appel SET ouvert_par = auteur_user_id WHERE ouvert_par IS NULL;
-- Les appels sur créneau : la séance de repli est identifiée par (créneau, jour).
-- Un appel neutralisé garde le statut « clos » (projection historique).

ALTER TABLE ent_appel DROP CONSTRAINT IF EXISTS ent_appel_type_activite_chk;
ALTER TABLE ent_appel ADD CONSTRAINT ent_appel_type_activite_chk CHECK (type_activite IN (
    'cours', 'permanence', 'etude', 'cdi', 'demi_pension', 'accompagnement', 'sortie', 'internat', 'activite'));
ALTER TABLE ent_appel DROP CONSTRAINT IF EXISTS ent_appel_etat_chk;
ALTER TABLE ent_appel ADD CONSTRAINT ent_appel_etat_chk CHECK (etat IN ('en_cours', 'valide', 'neutralise'));
ALTER TABLE ent_appel DROP CONSTRAINT IF EXISTS ent_appel_mode_pointage_chk;
ALTER TABLE ent_appel ADD CONSTRAINT ent_appel_mode_pointage_chk CHECK (mode_pointage IN ('tous_presents', 'presents_seulement'));
ALTER TABLE ent_appel DROP CONSTRAINT IF EXISTS ent_appel_canal_chk;
ALTER TABLE ent_appel ADD CONSTRAINT ent_appel_canal_chk CHECK (canal_validation IS NULL OR canal_validation IN ('web', 'mobile', 'hors_ligne'));
ALTER TABLE ent_appel DROP CONSTRAINT IF EXISTS ent_appel_activite_forme_chk;
ALTER TABLE ent_appel ADD CONSTRAINT ent_appel_activite_forme_chk CHECK (
    type_activite = 'cours' OR (activite_cle IS NOT NULL AND heure_debut IS NOT NULL AND heure_fin IS NOT NULL AND heure_fin > heure_debut));
ALTER TABLE ent_appel DROP CONSTRAINT IF EXISTS ent_appel_neutralise_chk;
ALTER TABLE ent_appel ADD CONSTRAINT ent_appel_neutralise_chk CHECK ((etat = 'neutralise') = (neutralise_le IS NOT NULL));

-- Une séance = un appel ; une activité (clé stable) = un appel par jour (R-AS1).
CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_seance_uq ON ent_appel (etablissement_id, seance_id) WHERE seance_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_activite_uq
    ON ent_appel (etablissement_id, type_activite, activite_cle, jour) WHERE activite_cle IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_etab_id_uq ON ent_appel (etablissement_id, id);

-- ── Lignes d'appel : états étendus ──────────────────────────────────────────────────────────
ALTER TABLE ent_appel_ligne DROP CONSTRAINT IF EXISTS ent_appel_ligne_etat_chk;
ALTER TABLE ent_appel_ligne ADD CONSTRAINT ent_appel_ligne_etat_chk CHECK (etat IN (
    'non_pointe', 'present', 'absent', 'retard', 'localise', 'attendu_ailleurs', 'present_dispense'));
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS minutes_retard INTEGER;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS lieu_code TEXT;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS situation_id UUID;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS exclusion_cours_le TIMESTAMPTZ;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS arrivee_vs_confirmee_le TIMESTAMPTZ;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS arrivee_vs_par UUID;
ALTER TABLE ent_appel_ligne ADD COLUMN IF NOT EXISTS cree_le TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE ent_appel_ligne DROP CONSTRAINT IF EXISTS ent_appel_ligne_minutes_chk;
ALTER TABLE ent_appel_ligne ADD CONSTRAINT ent_appel_ligne_minutes_chk CHECK (
    minutes_retard IS NULL OR (etat = 'retard' AND minutes_retard BETWEEN 1 AND 600));
ALTER TABLE ent_appel_ligne DROP CONSTRAINT IF EXISTS ent_appel_ligne_lieu_chk;
ALTER TABLE ent_appel_ligne ADD CONSTRAINT ent_appel_ligne_lieu_chk CHECK (
    lieu_code IS NULL OR (etat IN ('localise', 'attendu_ailleurs') AND
        lieu_code IN ('infirmerie', 'vie_scolaire', 'cdi', 'entretien', 'autre_activite', 'sortie', 'examen',
                      'instance', 'dispense', 'stage', 'prise_en_charge_exterieure')));
CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_ligne_etab_id_uq ON ent_appel_ligne (etablissement_id, id);

-- ── Historique d'une ligne (ajout seul, R-AS8) ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_appel_ligne_histo (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appel_id UUID NOT NULL,
    ligne_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    etat_avant TEXT,
    etat_apres TEXT NOT NULL,
    minutes INTEGER,
    lieu_code TEXT,
    auteur_id UUID,
    canal TEXT NOT NULL DEFAULT 'web' CHECK (canal IN ('web', 'mobile', 'hors_ligne', 'systeme')),
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'saisie', 'erreur_saisie', 'billet_entree', 'oubli', 'situation', 'validation', 'neutralisation',
        'correction_vie_scolaire', 'exclusion_cours', 'arrivee_vie_scolaire')),
    motif_texte TEXT CHECK (motif_texte IS NULL OR length(motif_texte) <= 200),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_appel_ligne_histo_fk FOREIGN KEY (etablissement_id, ligne_id)
        REFERENCES ent_appel_ligne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_appel_ligne_histo_ligne_idx ON ent_appel_ligne_histo (etablissement_id, ligne_id, version);
CREATE INDEX IF NOT EXISTS ent_appel_ligne_histo_appel_idx ON ent_appel_ligne_histo (etablissement_id, appel_id);

-- Délégation datée d'un appel (AS-105) : une séance, un jour, visible de la vie scolaire.
CREATE TABLE IF NOT EXISTS ent_appel_delegation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appel_id UUID NOT NULL,
    de_user UUID NOT NULL,
    a_user UUID NOT NULL,
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 200),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoquee_le TIMESTAMPTZ,
    CONSTRAINT ent_appel_delegation_fk FOREIGN KEY (etablissement_id, appel_id)
        REFERENCES ent_appel (etablissement_id, id),
    CONSTRAINT ent_appel_delegation_distincts_chk CHECK (de_user <> a_user)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_appel_delegation_active_uq
    ON ent_appel_delegation (etablissement_id, appel_id) WHERE revoquee_le IS NULL;

CREATE OR REPLACE FUNCTION ent_appel_histo_ajout_seul() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- La suppression en cascade d'un appel de test est tolérée seulement si la ligne parente disparaît.
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM ent_appel_ligne l WHERE l.id = OLD.ligne_id) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'ent_appel_ligne_histo est en ajout seul (% interdit)', TG_OP;
END
$$;
DROP TRIGGER IF EXISTS ent_appel_ligne_histo_ajout_seul ON ent_appel_ligne_histo;
CREATE TRIGGER ent_appel_ligne_histo_ajout_seul BEFORE UPDATE OR DELETE ON ent_appel_ligne_histo
    FOR EACH ROW EXECUTE FUNCTION ent_appel_histo_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_appel_ligne_histo', 'ent_appel_delegation']);
