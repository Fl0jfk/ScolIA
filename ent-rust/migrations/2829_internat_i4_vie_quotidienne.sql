-- Socle 41 (internat), lot I4 « Vie quotidienne », seconde moitié — migration 2829 : liaison codée, événements de
-- nuit, temps du soir (études, activités), préoccupation.
-- Réf. : docs/socles/41_INTERNAT.md §11, §19.7, §19.8, §20 (R-IT7, R-IT8, R-IT9, R-IT29), §22, §24.
-- 2828 et les migrations antérieures sont versées : on ne les modifie pas (règle 25). Additive et rejouable.
-- Aucune colonne JSON, aucun BYTEA, aucune donnée de santé (R-IT6).
--
-- Écart assumé avec §24 : le catalogue des faits de liaison est copié PAR ÉTABLISSEMENT à la première lecture (comme les
-- types de sortie d'I3) et non porté en `etablissement_id NULL` : la politique de ligne reste unique et l'établissement
-- peut l'adapter (INT-247) sans toucher aux autres.

-- ───────────────────────── 1. Catalogue et faits de liaison ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_liaison_code (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 80),
    -- Fait que la vie scolaire de jour voit le matin (R-IT8), sans jamais le texte libre.
    pour_le_jour BOOLEAN NOT NULL DEFAULT false,
    valorisation BOOLEAN NOT NULL DEFAULT false,
    ordre SMALLINT NOT NULL DEFAULT 0,
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, code)
);

CREATE TABLE IF NOT EXISTS ent_internat_liaison (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    code TEXT NOT NULL,
    -- Texte court : 300 caractères, sans donnée de santé (contrôle du service, R-IT7).
    texte TEXT CHECK (texte IS NULL OR char_length(btrim(texte)) BETWEEN 1 AND 300),
    nuit DATE NOT NULL,
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- Photographie du réglage du code au moment de l'écriture : changer le catalogue ne rend pas visible
    -- rétroactivement un fait au personnel de jour.
    pour_le_jour BOOLEAN NOT NULL,
    masque_le TIMESTAMPTZ,
    masque_par_user_id UUID REFERENCES ent_user (id),
    a_revoir_le TIMESTAMPTZ,
    a_revoir_par_user_id UUID REFERENCES ent_user (id),
    signale_pp_le TIMESTAMPTZ,
    signale_pp_par_user_id UUID REFERENCES ent_user (id),
    -- Étude du soir : le fait d'absence à l'étude renvoie à la séance (INT-194).
    temps_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_liaison_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_liaison_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_liaison_code_cfk FOREIGN KEY (etablissement_id, code)
        REFERENCES ent_internat_liaison_code (etablissement_id, code),
    CONSTRAINT ent_internat_liaison_masque_chk CHECK ((masque_le IS NULL) = (masque_par_user_id IS NULL)),
    CONSTRAINT ent_internat_liaison_revoir_chk CHECK ((a_revoir_le IS NULL) = (a_revoir_par_user_id IS NULL)),
    CONSTRAINT ent_internat_liaison_pp_chk CHECK ((signale_pp_le IS NULL) = (signale_pp_par_user_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_internat_liaison_eleve_idx
    ON ent_internat_liaison (etablissement_id, eleve_id, nuit DESC);
CREATE INDEX IF NOT EXISTS ent_internat_liaison_nuit_idx
    ON ent_internat_liaison (etablissement_id, nuit);

-- Journal des gestes sur un fait de liaison (masquage, « à revoir », signalement au professeur principal) : ajout seul,
-- chaque geste porte un motif codé et son auteur.
CREATE TABLE IF NOT EXISTS ent_internat_liaison_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    liaison_id UUID NOT NULL,
    acte TEXT NOT NULL CHECK (acte IN ('masquage', 'a_revoir', 'a_revoir_leve', 'signal_pp')),
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'erreur', 'donnee_sensible', 'hors_perimetre', 'doublon', 'suivi_cpe', 'autre')),
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    a TIMESTAMPTZ NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT ent_internat_liaison_fait_liaison_cfk FOREIGN KEY (etablissement_id, liaison_id)
        REFERENCES ent_internat_liaison (etablissement_id, id),
    CONSTRAINT ent_internat_liaison_fait_masquage_chk CHECK (acte <> 'masquage' OR motif_code IS NOT NULL)
);

-- ───────────────────────── 2. Événements de nuit ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_evenement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature IN (
        'incident', 'comportement', 'intrusion', 'alarme', 'malaise', 'degradation', 'objet_dangereux')),
    -- Alerte d'urgence (NI-40) : trois niveaux ; « critique » prévient direction et CPE d'astreinte (U3).
    niveau TEXT NOT NULL DEFAULT 'information' CHECK (niveau IN ('information', 'important', 'critique')),
    a TIMESTAMPTZ NOT NULL,
    nuit DATE NOT NULL,
    zone_id UUID,
    chambre_id UUID,
    mesures_code TEXT CHECK (mesures_code IS NULL OR mesures_code IN (
        'aucune', 'adultes_sur_place', 'direction_prevenue', 'secours_appeles', 'chambre_separee',
        'objet_confisque', 'famille_prevenue', 'autre')),
    declare_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- Transmissions (le 41 garde le fait et le lien ; les socles 13, 34 et 44 qualifient).
    transmis_13_le TIMESTAMPTZ,
    transmis_13_par_user_id UUID REFERENCES ent_user (id),
    intervention_demandee_le TIMESTAMPTZ,
    intervention_demandee_par_user_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_evenement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_evenement_zone_cfk FOREIGN KEY (etablissement_id, zone_id)
        REFERENCES ent_internat_zone (etablissement_id, id),
    CONSTRAINT ent_internat_evenement_chambre_cfk FOREIGN KEY (etablissement_id, chambre_id)
        REFERENCES ent_internat_chambre (etablissement_id, id),
    CONSTRAINT ent_internat_evenement_t13_chk CHECK ((transmis_13_le IS NULL) = (transmis_13_par_user_id IS NULL)),
    CONSTRAINT ent_internat_evenement_t44_chk CHECK (
        (intervention_demandee_le IS NULL) = (intervention_demandee_par_user_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_internat_evenement_nuit_idx ON ent_internat_evenement (etablissement_id, nuit, a);

CREATE TABLE IF NOT EXISTS ent_internat_evenement_participant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    -- Exactement l'un des deux : un élève ou un adulte (compte du personnel). Jamais un nom.
    eleve_id UUID,
    user_id UUID REFERENCES ent_user (id),
    role TEXT NOT NULL CHECK (role IN ('auteur_presume', 'victime', 'temoin', 'present')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_evt_part_un_chk CHECK ((eleve_id IS NULL) <> (user_id IS NULL)),
    CONSTRAINT ent_internat_evt_part_adulte_chk CHECK (user_id IS NULL OR role IN ('temoin', 'present')),
    CONSTRAINT ent_internat_evt_part_evt_cfk FOREIGN KEY (etablissement_id, evenement_id)
        REFERENCES ent_internat_evenement (etablissement_id, id),
    CONSTRAINT ent_internat_evt_part_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_evt_part_eleve_uq
    ON ent_internat_evenement_participant (evenement_id, eleve_id, role) WHERE eleve_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_evt_part_user_uq
    ON ent_internat_evenement_participant (evenement_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_internat_evt_part_eleve_idx
    ON ent_internat_evenement_participant (etablissement_id, eleve_id) WHERE eleve_id IS NOT NULL;

-- Faits ajoutés à un événement : jamais de suppression, la correction est un fait de plus (R-IT3).
CREATE TABLE IF NOT EXISTS ent_internat_evenement_complement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    texte TEXT NOT NULL CHECK (char_length(btrim(texte)) BETWEEN 1 AND 1000),
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT ent_internat_evt_compl_evt_cfk FOREIGN KEY (etablissement_id, evenement_id)
        REFERENCES ent_internat_evenement (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_internat_evt_compl_idx
    ON ent_internat_evenement_complement (etablissement_id, evenement_id, cree_le);

-- ───────────────────────── 3. Temps du soir : études et activités ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_temps (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature IN ('etude', 'activite')),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 120),
    lieu TEXT CHECK (lieu IS NULL OR char_length(btrim(lieu)) BETWEEN 1 AND 80),
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    encadrant_user_id UUID REFERENCES ent_user (id),
    places SMALLINT CHECK (places IS NULL OR places BETWEEN 1 AND 500),
    statut TEXT NOT NULL DEFAULT 'prevu' CHECK (statut IN ('prevu', 'annule')),
    cree_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_temps_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_temps_periode_chk CHECK (fin > debut AND fin <= debut + interval '12 hours')
);
CREATE INDEX IF NOT EXISTS ent_internat_temps_debut_idx ON ent_internat_temps (etablissement_id, debut);

CREATE TABLE IF NOT EXISTS ent_internat_temps_participant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    temps_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    inscrit_le TIMESTAMPTZ NOT NULL,
    inscrit_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    retire_le TIMESTAMPTZ,
    retire_par_user_id UUID REFERENCES ent_user (id),
    CONSTRAINT ent_internat_temps_part_temps_cfk FOREIGN KEY (etablissement_id, temps_id)
        REFERENCES ent_internat_temps (etablissement_id, id),
    CONSTRAINT ent_internat_temps_part_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_temps_part_retire_chk CHECK ((retire_le IS NULL) = (retire_par_user_id IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_temps_part_actif_uq
    ON ent_internat_temps_participant (temps_id, eleve_id) WHERE retire_le IS NULL;

-- ───────────────────────── 4. Préoccupation (socle 36) ─────────────────────────
-- Le 41 ouvre le circuit et ne garde AUCUN contenu : l'adulte, l'élève, l'instant. Rien d'autre (R-IT29).
CREATE TABLE IF NOT EXISTS ent_internat_preoccupation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    a TIMESTAMPTZ NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_preoccupation_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);

-- ───────────────────────── 5. Vigilance codée et référent d'internat (INT-257) ─────────────────────────
-- Une vigilance (« suivi rapproché », « premier mois », « à accompagner au coucher ») et un référent, datés, revus
-- chaque semaine ; aucun texte (parité NI-06 « sous surveillance »).
CREATE TABLE IF NOT EXISTS ent_internat_vigilance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN ('suivi_rapproche', 'premier_mois', 'accompagner_coucher')),
    referent_user_id UUID REFERENCES ent_user (id),
    du DATE NOT NULL,
    au DATE,
    revue_le TIMESTAMPTZ NOT NULL,
    revue_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    pose_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_vigilance_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_vigilance_periode_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_internat_vigilance_sans_chevauchement
        EXCLUDE USING gist (etablissement_id WITH =, eleve_id WITH =, code WITH =, daterange(du, au, '[]') WITH &&)
);
CREATE INDEX IF NOT EXISTS ent_internat_vigilance_eleve_idx ON ent_internat_vigilance (etablissement_id, eleve_id, du DESC);

CREATE OR REPLACE FUNCTION ent_internat_vigilance_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_vigilance : suppression interdite (R-IT3), la lever par une date' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.code, OLD.du, OLD.pose_par_user_id, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.code, NEW.du, NEW.pose_par_user_id, NEW.cree_le) THEN
        RAISE EXCEPTION 'vigilance : élève, code et début ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_vigilance_garde ON ent_internat_vigilance;
CREATE TRIGGER ent_internat_vigilance_garde BEFORE UPDATE OR DELETE ON ent_internat_vigilance
    FOR EACH ROW EXECUTE FUNCTION ent_internat_vigilance_garde();

-- ───────────────────────── Gardes ─────────────────────────
-- Fait de liaison : ajout seul, sauf les trois gestes (masquer, « à revoir », signaler au professeur principal),
-- qui ne font que poser une date et un auteur (le « à revoir » se lève). Le contenu ne change jamais.
CREATE OR REPLACE FUNCTION ent_internat_liaison_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_liaison : suppression interdite (R-IT3), masquer avec un motif' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.code, OLD.texte, OLD.nuit, OLD.par_user_id, OLD.pour_le_jour,
        OLD.temps_id, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.code, NEW.texte, NEW.nuit, NEW.par_user_id,
        NEW.pour_le_jour, NEW.temps_id, NEW.cree_le) THEN
        RAISE EXCEPTION 'fait de liaison : le contenu ne se modifie pas (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF OLD.masque_le IS NOT NULL AND (OLD.masque_le, OLD.masque_par_user_id)
       IS DISTINCT FROM (NEW.masque_le, NEW.masque_par_user_id) THEN
        RAISE EXCEPTION 'fait de liaison masqué : le masquage est définitif' USING ERRCODE = '42501';
    END IF;
    IF OLD.signale_pp_le IS NOT NULL AND (OLD.signale_pp_le, OLD.signale_pp_par_user_id)
       IS DISTINCT FROM (NEW.signale_pp_le, NEW.signale_pp_par_user_id) THEN
        RAISE EXCEPTION 'fait de liaison : le signalement est définitif' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_liaison_garde ON ent_internat_liaison;
CREATE TRIGGER ent_internat_liaison_garde BEFORE UPDATE OR DELETE ON ent_internat_liaison
    FOR EACH ROW EXECUTE FUNCTION ent_internat_liaison_garde();

-- Événement : contenu figé, seules les deux transmissions se posent (une fois chacune).
CREATE OR REPLACE FUNCTION ent_internat_evenement_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_evenement : suppression interdite (R-IT3), compléter par un fait ajouté'
            USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.nature, OLD.niveau, OLD.a, OLD.nuit, OLD.zone_id, OLD.chambre_id,
        OLD.mesures_code, OLD.declare_par_user_id, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.nature, NEW.niveau, NEW.a, NEW.nuit, NEW.zone_id,
        NEW.chambre_id, NEW.mesures_code, NEW.declare_par_user_id, NEW.cree_le) THEN
        RAISE EXCEPTION 'événement de nuit : le contenu ne se modifie pas (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF OLD.transmis_13_le IS NOT NULL AND (OLD.transmis_13_le, OLD.transmis_13_par_user_id)
       IS DISTINCT FROM (NEW.transmis_13_le, NEW.transmis_13_par_user_id) THEN
        RAISE EXCEPTION 'événement de nuit : transmission déjà faite' USING ERRCODE = '42501';
    END IF;
    IF OLD.intervention_demandee_le IS NOT NULL AND (OLD.intervention_demandee_le, OLD.intervention_demandee_par_user_id)
       IS DISTINCT FROM (NEW.intervention_demandee_le, NEW.intervention_demandee_par_user_id) THEN
        RAISE EXCEPTION 'événement de nuit : intervention déjà demandée' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_evenement_garde ON ent_internat_evenement;
CREATE TRIGGER ent_internat_evenement_garde BEFORE UPDATE OR DELETE ON ent_internat_evenement
    FOR EACH ROW EXECUTE FUNCTION ent_internat_evenement_garde();

-- Participant d'événement, complément, journal de liaison, préoccupation : ajout seul.
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'ent_internat_evenement_participant', 'ent_internat_evenement_complement', 'ent_internat_liaison_fait',
        'ent_internat_preoccupation']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_ajout_seul', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW '
                       'EXECUTE FUNCTION ent_internat_i4_ajout_seul()', t || '_ajout_seul', t);
    END LOOP;
END
$$;

-- Temps du soir : jamais supprimés (annulés), un participant se retire par une date.
CREATE OR REPLACE FUNCTION ent_internat_temps_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_temps : suppression interdite (R-IT3), annuler' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut = 'annule' AND OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'temps du soir annulé : plus aucune modification' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.nature, OLD.cree_par_user_id, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.nature, NEW.cree_par_user_id, NEW.cree_le) THEN
        RAISE EXCEPTION 'temps du soir : nature et créateur ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_temps_garde ON ent_internat_temps;
CREATE TRIGGER ent_internat_temps_garde BEFORE UPDATE OR DELETE ON ent_internat_temps
    FOR EACH ROW EXECUTE FUNCTION ent_internat_temps_garde();

CREATE OR REPLACE FUNCTION ent_internat_temps_participant_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_temps_participant : suppression interdite (R-IT3), retirer par une date'
            USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.temps_id, OLD.eleve_id, OLD.inscrit_le, OLD.inscrit_par_user_id)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.temps_id, NEW.eleve_id, NEW.inscrit_le, NEW.inscrit_par_user_id) THEN
        RAISE EXCEPTION 'inscription : seul le retrait se pose' USING ERRCODE = '42501';
    END IF;
    IF OLD.retire_le IS NOT NULL AND OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'inscription retirée : plus aucune modification' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_temps_participant_garde ON ent_internat_temps_participant;
CREATE TRIGGER ent_internat_temps_participant_garde BEFORE UPDATE OR DELETE ON ent_internat_temps_participant
    FOR EACH ROW EXECUTE FUNCTION ent_internat_temps_participant_garde();

-- Catalogue : jamais supprimé (désactivé).
DROP TRIGGER IF EXISTS ent_internat_liaison_code_sans_suppression ON ent_internat_liaison_code;
CREATE TRIGGER ent_internat_liaison_code_sans_suppression BEFORE DELETE ON ent_internat_liaison_code
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i4_sans_suppression();

-- TRUNCATE interdit (E2).
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'ent_internat_liaison_code', 'ent_internat_liaison', 'ent_internat_liaison_fait', 'ent_internat_evenement',
        'ent_internat_evenement_participant', 'ent_internat_evenement_complement', 'ent_internat_temps',
        'ent_internat_temps_participant', 'ent_internat_preoccupation', 'ent_internat_vigilance']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_truncate', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT '
                       'EXECUTE FUNCTION ent_internat_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
END
$$;

SELECT ent_rls_poser(ARRAY[
    'ent_internat_liaison_code', 'ent_internat_liaison', 'ent_internat_liaison_fait', 'ent_internat_evenement',
    'ent_internat_evenement_participant', 'ent_internat_evenement_complement', 'ent_internat_temps',
    'ent_internat_temps_participant', 'ent_internat_preoccupation', 'ent_internat_vigilance']);

-- Journal des lectures sensibles (R-IT38) : deux objets de plus (événements, sorties de groupe). La contrainte
-- d'I3 est remplacée (même nom automatique), jamais retirée.
ALTER TABLE ent_internat_lecture_sensible DROP CONSTRAINT IF EXISTS ent_internat_lecture_sensible_objet_check;
ALTER TABLE ent_internat_lecture_sensible ADD CONSTRAINT ent_internat_lecture_sensible_objet_check CHECK (objet IN (
    'fiche_nuit', 'liaison', 'presents', 'registre', 'cas', 'appel', 'appel_pdf', 'attendu',
    'liste_appels', 'procedure', 'effectif_repas', 'listes', 'journal',
    'liste_remise', 'sorties', 'correspondants', 'evenement', 'sortie_groupe'));
