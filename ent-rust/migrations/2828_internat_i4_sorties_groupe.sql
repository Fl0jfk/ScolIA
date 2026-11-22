-- Socle 41 (internat), lot I4 « Vie quotidienne », première moitié — migration 2828 : sorties de groupe.
-- Réf. : docs/socles/41_INTERNAT.md §10.3, §19.6 (INT-170 à INT-179), §20 (R-IT21, R-IT24, R-IT25), §22, §24.
-- 2820 à 2827 sont versées : on ne les modifie pas (règle 25). Additive et rejouable. Aucune colonne JSON, aucun BYTEA,
-- aucune donnée de santé (R-IT6), aucun nom ni texte libre dans l'outbox.
--
-- Contenu :
--   1. Réglages de l'établissement pour la vie quotidienne (colonnes typées, valeurs prudentes, D-069).
--   2. Sorties de groupe : statut gardé (pas de suppression, pas de changement d'horaire après la soumission),
--      accompagnateurs (comptes du personnel, R-IT25), participants (un refus n'exclut que l'élève, R-IT21),
--      accords reçus hors ligne avec auteur et date (R-IT24), journal des faits en ajout seul.

-- ───────────────────────── 1. Réglages ─────────────────────────
-- Créés à la première lecture ; une ligne par établissement. Valeurs par défaut prudentes : préavis de 24 h pour
-- laisser le temps aux titulaires de répondre, 12 élèves par accompagnateur.
CREATE TABLE IF NOT EXISTS ent_internat_parametres_vie (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    groupe_eleves_par_encadrant SMALLINT NOT NULL DEFAULT 12 CHECK (groupe_eleves_par_encadrant BETWEEN 1 AND 60),
    groupe_preavis_heures SMALLINT NOT NULL DEFAULT 24 CHECK (groupe_preavis_heures BETWEEN 0 AND 720),
    liaison_texte_max SMALLINT NOT NULL DEFAULT 300 CHECK (liaison_texte_max BETWEEN 50 AND 300),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par UUID REFERENCES ent_user (id)
);

-- ───────────────────────── 2. Sorties de groupe ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_sortie_groupe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    titre TEXT NOT NULL CHECK (char_length(btrim(titre)) BETWEEN 1 AND 120),
    lieu TEXT NOT NULL CHECK (char_length(btrim(lieu)) BETWEEN 1 AND 120),
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    statut TEXT NOT NULL DEFAULT 'brouillon'
        CHECK (statut IN ('brouillon', 'soumise', 'acceptee', 'refusee', 'annulee', 'close')),
    -- Série : une sortie dupliquée (piscine du mardi) garde le lien avec son modèle (INT-177).
    modele_id UUID,
    cree_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    decide_par_user_id UUID REFERENCES ent_user (id),
    decide_le TIMESTAMPTZ,
    decision_motif_code TEXT CHECK (decision_motif_code IS NULL OR decision_motif_code IN (
        'securite', 'encadrement', 'calendrier', 'reglement', 'autre')),
    annule_motif_code TEXT CHECK (annule_motif_code IS NULL OR annule_motif_code IN (
        'meteo', 'encadrement', 'organisateur', 'securite', 'autre')),
    close_le TIMESTAMPTZ,
    close_par_user_id UUID REFERENCES ent_user (id),
    bilan_code TEXT CHECK (bilan_code IS NULL OR bilan_code IN ('sans_incident', 'retard', 'incident')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_sortie_groupe_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_sortie_groupe_periode_chk CHECK (fin > debut AND fin <= debut + interval '14 days'),
    CONSTRAINT ent_internat_sortie_groupe_decision_chk CHECK (
        (statut IN ('acceptee', 'refusee') AND decide_par_user_id IS NOT NULL AND decide_le IS NOT NULL)
        OR statut NOT IN ('acceptee', 'refusee') OR statut IS NULL),
    CONSTRAINT ent_internat_sortie_groupe_refus_chk CHECK (
        statut <> 'refusee' OR decision_motif_code IS NOT NULL),
    CONSTRAINT ent_internat_sortie_groupe_close_chk CHECK (
        (statut = 'close') = (close_le IS NOT NULL AND close_par_user_id IS NOT NULL AND bilan_code IS NOT NULL)),
    CONSTRAINT ent_internat_sortie_groupe_modele_cfk FOREIGN KEY (etablissement_id, modele_id)
        REFERENCES ent_internat_sortie_groupe (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_internat_sortie_groupe_debut_idx
    ON ent_internat_sortie_groupe (etablissement_id, debut);

-- Accompagnateurs : comptes du personnel (R-IT25), jamais un nom en texte libre.
CREATE TABLE IF NOT EXISTS ent_internat_sortie_groupe_encadrant (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_groupe_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (sortie_groupe_id, user_id),
    CONSTRAINT ent_internat_sg_encadrant_sg_cfk FOREIGN KEY (etablissement_id, sortie_groupe_id)
        REFERENCES ent_internat_sortie_groupe (etablissement_id, id)
);

-- Participants. `invite` : prévu, famille pas encore interrogée (brouillon) ; `en_attente` : famille interrogée ;
-- `autorise` : accord d'un titulaire (ou du majeur, ou accord annuel du socle 18, ou accord hors ligne daté) ;
-- `refuse` : refus d'un titulaire ; `non_autorise` : bloqué par le système (désaccord connu, interdiction, élève qui
-- n'est plus interne) ; `parti` / `rentre` : appels de départ et de retour. Un refus ou un blocage n'exclut que
-- l'élève concerné (R-IT21).
CREATE TABLE IF NOT EXISTS ent_internat_sortie_groupe_participant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_groupe_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'invite'
        CHECK (etat IN ('invite', 'en_attente', 'autorise', 'refuse', 'non_autorise', 'parti', 'rentre', 'retire')),
    -- Qui a répondu, et en quelle qualité (R-IT24 : jamais un « autorisé par l'équipe » anonyme).
    reponse_qualite TEXT CHECK (reponse_qualite IS NULL OR reponse_qualite IN (
        'titulaire', 'eleve_majeur', 'annuelle', 'hors_ligne')),
    reponse_user_id UUID REFERENCES ent_user (id),
    reponse_personne_id UUID,
    reponse_le TIMESTAMPTZ,
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'refus_famille', 'desaccord_connu', 'interdiction', 'plus_interne', 'retire_par_organisateur', 'majeur_refuse')),
    parti_le TIMESTAMPTZ,
    parti_par_user_id UUID REFERENCES ent_user (id),
    rentre_le TIMESTAMPTZ,
    rentre_par_user_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_sg_part_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_sg_part_uq UNIQUE (sortie_groupe_id, eleve_id),
    CONSTRAINT ent_internat_sg_part_sg_cfk FOREIGN KEY (etablissement_id, sortie_groupe_id)
        REFERENCES ent_internat_sortie_groupe (etablissement_id, id),
    CONSTRAINT ent_internat_sg_part_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_sg_part_reponse_chk CHECK (
        etat NOT IN ('autorise', 'refuse', 'parti', 'rentre')
        OR (reponse_qualite IS NOT NULL AND reponse_le IS NOT NULL
            AND (reponse_qualite = 'annuelle' OR reponse_user_id IS NOT NULL))),
    CONSTRAINT ent_internat_sg_part_hors_ligne_chk CHECK (
        reponse_qualite IS DISTINCT FROM 'hors_ligne' OR reponse_personne_id IS NOT NULL),
    CONSTRAINT ent_internat_sg_part_depart_chk CHECK (
        (etat IN ('parti', 'rentre')) = (parti_le IS NOT NULL AND parti_par_user_id IS NOT NULL)
        AND (etat = 'rentre') = (rentre_le IS NOT NULL AND rentre_par_user_id IS NOT NULL)),
    CONSTRAINT ent_internat_sg_part_bloque_chk CHECK (
        etat NOT IN ('refuse', 'non_autorise', 'retire') OR motif_code IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_internat_sg_part_eleve_idx
    ON ent_internat_sortie_groupe_participant (etablissement_id, eleve_id);

-- Accord reçu HORS LIGNE (téléphone, papier, en personne) : saisi par un membre du personnel, il nomme la personne
-- qui l'a donné (un titulaire du socle 4), la date de réception, le canal et la personne qui l'a reçu (compte). Pièce
-- déposée au socle 7 (facultative) ; sans pièce, la déclaration datée vaut preuve. Ajout seul (R-IT24).
CREATE TABLE IF NOT EXISTS ent_internat_sortie_groupe_accord_hors_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_groupe_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    donne_par_personne_id UUID NOT NULL,
    recu_le TIMESTAMPTZ NOT NULL,
    canal TEXT NOT NULL CHECK (canal IN ('telephone', 'papier', 'en_personne')),
    recu_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    piece_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_sg_hl_sg_cfk FOREIGN KEY (etablissement_id, sortie_groupe_id)
        REFERENCES ent_internat_sortie_groupe (etablissement_id, id),
    CONSTRAINT ent_internat_sg_hl_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);

-- Journal des faits en ajout seul : ce qui s'est passé, par qui, quand. Aucun texte libre.
CREATE TABLE IF NOT EXISTS ent_internat_sortie_groupe_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_groupe_id UUID NOT NULL,
    eleve_id UUID,
    acte TEXT NOT NULL CHECK (acte IN (
        'creation', 'duplication', 'modification', 'soumission', 'acceptation', 'refus_direction', 'annulation',
        'invitation', 'retrait', 'accord_famille', 'refus_famille', 'accord_annuel', 'accord_hors_ligne',
        'blocage_desaccord', 'blocage_interdiction', 'accord_tombe', 'depart', 'retour', 'cloture', 'encadrant_ajoute',
        'encadrant_retire')),
    par_user_id UUID REFERENCES ent_user (id),
    a TIMESTAMPTZ NOT NULL,
    motif_code TEXT,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT ent_internat_sg_fait_sg_cfk FOREIGN KEY (etablissement_id, sortie_groupe_id)
        REFERENCES ent_internat_sortie_groupe (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_internat_sg_fait_sg_idx
    ON ent_internat_sortie_groupe_fait (etablissement_id, sortie_groupe_id, cree_le);

-- ───────────────────────── Gardes ─────────────────────────
CREATE OR REPLACE FUNCTION ent_internat_i4_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : suppression interdite (R-IT3)', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;
CREATE OR REPLACE FUNCTION ent_internat_i4_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : ajout seul (R-IT3)', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;

-- Sortie de groupe : transitions permises, champs figés après la soumission.
CREATE OR REPLACE FUNCTION ent_internat_sortie_groupe_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_sortie_groupe : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.cree_par_user_id, OLD.cree_le, OLD.modele_id)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.cree_par_user_id, NEW.cree_le, NEW.modele_id) THEN
        RAISE EXCEPTION 'sortie de groupe : identité, créateur et modèle ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut <> 'brouillon'
       AND (OLD.titre, OLD.lieu, OLD.debut, OLD.fin) IS DISTINCT FROM (NEW.titre, NEW.lieu, NEW.debut, NEW.fin) THEN
        RAISE EXCEPTION 'sortie de groupe soumise : titre, lieu et horaires sont figés (annuler et recréer)'
            USING ERRCODE = '42501';
    END IF;
    IF OLD.statut <> NEW.statut AND NOT (
           (OLD.statut = 'brouillon' AND NEW.statut IN ('soumise', 'annulee'))
        OR (OLD.statut = 'soumise' AND NEW.statut IN ('acceptee', 'refusee', 'annulee'))
        OR (OLD.statut = 'acceptee' AND NEW.statut IN ('close', 'annulee'))) THEN
        RAISE EXCEPTION 'sortie de groupe : transition % → % refusée', OLD.statut, NEW.statut USING ERRCODE = '42501';
    END IF;
    IF OLD.statut IN ('refusee', 'annulee', 'close') AND OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'sortie de groupe terminée : plus aucune modification' USING ERRCODE = '42501';
    END IF;
    IF OLD.decide_le IS NOT NULL AND (OLD.decide_par_user_id, OLD.decide_le)
       IS DISTINCT FROM (NEW.decide_par_user_id, NEW.decide_le) THEN
        RAISE EXCEPTION 'sortie de groupe : la décision et son auteur sont figés' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_sortie_groupe_garde ON ent_internat_sortie_groupe;
CREATE TRIGGER ent_internat_sortie_groupe_garde BEFORE UPDATE OR DELETE ON ent_internat_sortie_groupe
    FOR EACH ROW EXECUTE FUNCTION ent_internat_sortie_groupe_garde();

-- Participant : identité figée, jamais supprimé ; un départ constaté ne se défait pas.
CREATE OR REPLACE FUNCTION ent_internat_sg_participant_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'participant de sortie de groupe : suppression interdite (R-IT3), le retirer par un fait'
            USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.sortie_groupe_id, OLD.eleve_id)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.sortie_groupe_id, NEW.eleve_id) THEN
        RAISE EXCEPTION 'participant : sortie et élève ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.etat IN ('parti', 'rentre') AND NEW.etat NOT IN ('parti', 'rentre') THEN
        RAISE EXCEPTION 'participant parti : le départ constaté ne se défait pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.etat = 'rentre' AND NEW.etat <> 'rentre' THEN
        RAISE EXCEPTION 'participant rentré : retour constaté figé' USING ERRCODE = '42501';
    END IF;
    IF OLD.etat IN ('retire') AND NEW.etat <> 'retire' THEN
        RAISE EXCEPTION 'participant retiré : réinviter par un nouveau fait n''est pas permis ici' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_sg_participant_garde ON ent_internat_sortie_groupe_participant;
CREATE TRIGGER ent_internat_sg_participant_garde BEFORE UPDATE OR DELETE ON ent_internat_sortie_groupe_participant
    FOR EACH ROW EXECUTE FUNCTION ent_internat_sg_participant_garde();

DROP TRIGGER IF EXISTS ent_internat_sg_encadrant_sans_suppression ON ent_internat_sortie_groupe_encadrant;
CREATE TRIGGER ent_internat_sg_encadrant_sans_suppression BEFORE UPDATE ON ent_internat_sortie_groupe_encadrant
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i4_ajout_seul();
-- Un accompagnateur peut être retiré tant que la sortie n'a pas eu lieu (brouillon, soumise, acceptée) : la
-- suppression de la ligne est permise, tracée par le fait `encadrant_retire`.

DROP TRIGGER IF EXISTS ent_internat_sg_hl_ajout_seul ON ent_internat_sortie_groupe_accord_hors_ligne;
CREATE TRIGGER ent_internat_sg_hl_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_sortie_groupe_accord_hors_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i4_ajout_seul();
DROP TRIGGER IF EXISTS ent_internat_sg_fait_ajout_seul ON ent_internat_sortie_groupe_fait;
CREATE TRIGGER ent_internat_sg_fait_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_sortie_groupe_fait
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i4_ajout_seul();
DROP TRIGGER IF EXISTS ent_internat_parametres_vie_sans_suppression ON ent_internat_parametres_vie;
CREATE TRIGGER ent_internat_parametres_vie_sans_suppression BEFORE DELETE ON ent_internat_parametres_vie
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i4_sans_suppression();

-- TRUNCATE interdit sur les tables journal ou gardées (E2).
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'ent_internat_parametres_vie', 'ent_internat_sortie_groupe', 'ent_internat_sortie_groupe_participant',
        'ent_internat_sortie_groupe_accord_hors_ligne', 'ent_internat_sortie_groupe_fait']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_truncate', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT '
                       'EXECUTE FUNCTION ent_internat_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
END
$$;

SELECT ent_rls_poser(ARRAY[
    'ent_internat_parametres_vie', 'ent_internat_sortie_groupe', 'ent_internat_sortie_groupe_encadrant',
    'ent_internat_sortie_groupe_participant', 'ent_internat_sortie_groupe_accord_hors_ligne',
    'ent_internat_sortie_groupe_fait']);
