-- Socle 41 (internat), lot I1 — migration 2823 : séjours, rythmes, affectations étendues, dérogations,
-- interdictions de contact (port provisoire du socle 13).
-- Réf. : docs/socles/41_INTERNAT.md §8.2, §8.3, §24 ; R-IT1, R-IT3, R-IT4, R-IT10, R-IT11, R-IT12.
-- Additive et rejouable (E8). Aucune colonne JSON (D-036).

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- ───────────────────────── Séjours d'internat (R-IT1) ─────────────────────────
-- Un séjour naît du RÉGIME daté du socle 2 (origine « regime ») ; jamais d'interne créé à la main.
-- Période [du, au] inclusive ; au NULL = séjour ouvert. Pas de suppression (R-IT3) : fin datée.
CREATE TABLE IF NOT EXISTS ent_internat_sejour (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE,
    origine TEXT NOT NULL CHECK (origine IN ('regime', 'reprise', 'manuel_direction')),
    ferme_motif_code TEXT CHECK (ferme_motif_code IS NULL OR ferme_motif_code IN (
        'changement_regime', 'depart', 'fin_annee', 'correction_regime', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_sejour_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_sejour_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_sejour_periode_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_internat_sejour_sans_chevauchement
        EXCLUDE USING gist (etablissement_id WITH =, eleve_id WITH =, daterange(du, au, '[]') WITH &&)
);
CREATE INDEX IF NOT EXISTS ent_internat_sejour_eleve_idx
    ON ent_internat_sejour (etablissement_id, eleve_id, du DESC);

DROP TRIGGER IF EXISTS ent_internat_sejour_sans_suppression ON ent_internat_sejour;
CREATE TRIGGER ent_internat_sejour_sans_suppression
    BEFORE DELETE ON ent_internat_sejour
    FOR EACH ROW EXECUTE FUNCTION ent_internat_sans_suppression();

-- ───────────────────────── Rythmes versionnés (R-IT4) ─────────────────────────
-- Une version vaut À PARTIR DE `du` ; la version en vigueur un jour donné est la plus récente dont
-- `du` ne dépasse pas ce jour. Ajout seul : un changement de rythme est une nouvelle version.
CREATE TABLE IF NOT EXISTS ent_internat_rythme (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sejour_id UUID NOT NULL,
    du DATE NOT NULL,
    dimanche BOOLEAN NOT NULL,
    lundi BOOLEAN NOT NULL,
    mardi BOOLEAN NOT NULL,
    mercredi BOOLEAN NOT NULL,
    jeudi BOOLEAN NOT NULL,
    vendredi BOOLEAN NOT NULL,
    samedi BOOLEAN NOT NULL,
    cree_par_user_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_rythme_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_rythme_version_uq UNIQUE (etablissement_id, sejour_id, du),
    CONSTRAINT ent_internat_rythme_sejour_cfk FOREIGN KEY (etablissement_id, sejour_id)
        REFERENCES ent_internat_sejour (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_internat_rythme_ajout_seul ON ent_internat_rythme;
CREATE TRIGGER ent_internat_rythme_ajout_seul
    BEFORE UPDATE OR DELETE ON ent_internat_rythme
    FOR EACH ROW EXECUTE FUNCTION ent_internat_ajout_seul();

-- ───────────────────────── Affectations étendues (R-IT10) ─────────────────────────
ALTER TABLE ent_internat_affectation ADD COLUMN IF NOT EXISTS sejour_id UUID;
ALTER TABLE ent_internat_affectation ADD COLUMN IF NOT EXISTS lit_id UUID;
ALTER TABLE ent_internat_affectation ADD COLUMN IF NOT EXISTS motif_code TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_motif_chk') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_motif_chk CHECK (motif_code IS NULL OR motif_code IN (
                'rentree', 'demande_eleve', 'conflit', 'travaux', 'sante', 'relogement', 'autre'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_sejour_cfk') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_sejour_cfk
            FOREIGN KEY (etablissement_id, sejour_id) REFERENCES ent_internat_sejour (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_lit_cfk') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_lit_cfk
            FOREIGN KEY (etablissement_id, lit_id) REFERENCES ent_internat_lit (etablissement_id, id);
    END IF;
    -- Un lit, un élève, une date (R-IT10). Les affectations héritées de 066 (sans lit) restent
    -- bornées par la capacité de la chambre (migration 2820).
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_lit_sans_chevauchement') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_lit_sans_chevauchement
            EXCLUDE USING gist (etablissement_id WITH =, lit_id WITH =, daterange(debut, fin, '[]') WITH &&)
            WHERE (lit_id IS NOT NULL);
    END IF;
END
$$;

-- ───────────────────────── Dérogations (R-IT11, R-IT12) ─────────────────────────
-- Décision de la direction ; jamais de motif en clair (code seulement, S-4100 série).
CREATE TABLE IF NOT EXISTS ent_internat_derogation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('public_chambre', 'interdiction_contact')),
    chambre_id UUID,
    avec_eleve_id UUID,
    du DATE NOT NULL,
    au DATE,
    decide_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_derogation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_derogation_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_derogation_chambre_cfk FOREIGN KEY (etablissement_id, chambre_id)
        REFERENCES ent_internat_chambre (etablissement_id, id),
    CONSTRAINT ent_internat_derogation_avec_cfk FOREIGN KEY (etablissement_id, avec_eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_derogation_periode_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_internat_derogation_cible_chk CHECK (
        (nature = 'public_chambre' AND chambre_id IS NOT NULL AND avec_eleve_id IS NULL)
        OR (nature = 'interdiction_contact' AND avec_eleve_id IS NOT NULL AND chambre_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_internat_derogation_eleve_idx
    ON ent_internat_derogation (etablissement_id, eleve_id, nature, du);
DROP TRIGGER IF EXISTS ent_internat_derogation_ajout_seul ON ent_internat_derogation;
CREATE TRIGGER ent_internat_derogation_ajout_seul
    BEFORE UPDATE OR DELETE ON ent_internat_derogation
    FOR EACH ROW EXECUTE FUNCTION ent_internat_ajout_seul();

-- ───────────────────────── Interdictions de contact — PORT PROVISOIRE (socle 13) ─────────────────────────
-- Le socle 13 (discipline) est propriétaire de la mesure conservatoire « interdiction de contact ».
-- Il n'est pas encore codé dans ENT : cette table est son PORT provisoire (paires d'élèves, période),
-- alimenté par la direction ; elle sera remplacée par la lecture de la table du 13. Elle ne porte
-- aucun motif (le motif reste au 13, R-IT12 : « on partage des conséquences, jamais des causes »).
CREATE TABLE IF NOT EXISTS ent_internat_interdiction_contact (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_a_id UUID NOT NULL,
    eleve_b_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE,
    cree_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_interdiction_contact_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_interdiction_contact_a_cfk FOREIGN KEY (etablissement_id, eleve_a_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_interdiction_contact_b_cfk FOREIGN KEY (etablissement_id, eleve_b_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_interdiction_contact_paire_chk CHECK (eleve_a_id < eleve_b_id),
    CONSTRAINT ent_internat_interdiction_contact_periode_chk CHECK (au IS NULL OR au >= du)
);
CREATE INDEX IF NOT EXISTS ent_internat_interdiction_contact_idx
    ON ent_internat_interdiction_contact (etablissement_id, eleve_a_id, eleve_b_id);

SELECT ent_rls_poser(ARRAY['ent_internat_sejour', 'ent_internat_rythme', 'ent_internat_derogation',
                           'ent_internat_interdiction_contact']);
