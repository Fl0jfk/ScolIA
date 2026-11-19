-- Socle 41 (internat), lot I3 « Absences et sorties » — migration 2826.
-- Réf. : docs/socles/41_INTERNAT.md §10, §14.2, §17 (CIT-05 à CIT-11, CIT-23, CIT-33, CIT-37 à CIT-39),
-- §19.5 (INT-150 à INT-169), §20 (R-IT19 à R-IT23, R-IT39), §22, §24.
-- 2824 et 2825 sont versées : on ne les modifie pas (règle 25). Cette migration est additive et rejouable
-- (CREATE OR REPLACE, IF NOT EXISTS). Aucune colonne JSON, aucun BYTEA, aucune donnée de santé (R-IT6).
--
-- Contenu :
--   1. Suites de la revue d'I2 : appel « soir » complémentaire (M-1) et transmission avec non-couverts ;
--      colonne `ensembles_connus` (F-2, tolérance des transmissions écrites avant I3).
--   2. Catalogue du socle 18 : objet « sortie_internat » (autorisations annuelles de sortie).
--   3. Types de sortie (réglables, S-4105), correspondants, absences annoncées, sorties individuelles,
--      journal des faits de l'absence, de la sortie et du correspondant (ajout seul).
--   4. L'attendu de la nuit (I2) lit les absences et les sorties : une décision se reflète sans fausse alerte.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- ───────────────────────── 1. Suites d'I2 ─────────────────────────
-- M-1 : un appel « soir » COMPLÉMENTAIRE couvre les élèves attendus ajoutés après la validation du soir.
-- `complement` = 0 pour l'appel principal, n >= 1 pour le n-ième complément (un seul ouvert à la fois,
-- contrôlé par le service). Un complément est toujours de type « soir » et de périmètre « internat ».
ALTER TABLE ent_internat_appel ADD COLUMN IF NOT EXISTS complement SMALLINT NOT NULL DEFAULT 0;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_appel_complement_chk') THEN
        ALTER TABLE ent_internat_appel ADD CONSTRAINT ent_internat_appel_complement_chk
            CHECK (complement >= 0 AND (complement = 0 OR (type = 'soir' AND perimetre = 'internat')));
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_appel_perimetre_complement_uq ON ent_internat_appel (
    etablissement_id, nuit, type, perimetre,
    COALESCE(batiment_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(zone_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(liste_id, '00000000-0000-0000-0000-000000000000'::uuid),
    complement);
-- L'index de 2824 (sans le complément) interdisait tout second appel « soir » : remplacé par le précédent.
DROP INDEX IF EXISTS ent_internat_appel_perimetre_uq;

-- Garde de l'appel : celle de 2824, avec le complément figé et un complément qui exige un soir validé.
CREATE OR REPLACE FUNCTION ent_internat_appel_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_appel : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.nuit > ent_nuit_etab(NEW.etablissement_id, now()) THEN
            RAISE EXCEPTION 'appel d''une nuit future refusé' USING ERRCODE = '22007';
        END IF;
        IF NEW.complement > 0 AND NOT EXISTS (
               SELECT 1 FROM ent_internat_appel s
                WHERE s.etablissement_id = NEW.etablissement_id AND s.nuit = NEW.nuit
                  AND s.type = 'soir' AND s.etat = 'valide') THEN
            RAISE EXCEPTION 'appel complémentaire : aucun appel du soir validé pour cette nuit' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF OLD.etat = 'valide' THEN
        RAISE EXCEPTION 'appel validé : corriger par un fait motivé (R-IT17)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.nuit, OLD.type, OLD.perimetre, OLD.batiment_id, OLD.zone_id,
        OLD.liste_id, OLD.ouvert_par_user_id, OLD.ouvert_le, OLD.nb_attendus, OLD.a_posteriori, OLD.cree_le,
        OLD.complement)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.nuit, NEW.type, NEW.perimetre, NEW.batiment_id,
        NEW.zone_id, NEW.liste_id, NEW.ouvert_par_user_id, NEW.ouvert_le, NEW.nb_attendus, NEW.a_posteriori,
        NEW.cree_le, NEW.complement) THEN
        RAISE EXCEPTION 'appel : nuit, type, périmètre, effectif attendu et saisie a posteriori ne se modifient pas'
            USING ERRCODE = '42501';
    END IF;
    IF NOT ((OLD.etat = 'ouvert' AND NEW.etat = 'clos') OR (OLD.etat = 'clos' AND NEW.etat = 'valide')) THEN
        RAISE EXCEPTION 'appel : transition % → % refusée (ouvert → clos → validé seulement)', OLD.etat, NEW.etat
            USING ERRCODE = '42501';
    END IF;
    IF OLD.etat = 'clos' AND (OLD.nb_ecarts, OLD.clos_par_user_id, OLD.clos_le)
       IS DISTINCT FROM (NEW.nb_ecarts, NEW.clos_par_user_id, NEW.clos_le) THEN
        RAISE EXCEPTION 'appel clos : écarts, auteur et heure de clôture sont figés' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;

-- Transmission du matin : élèves attendus que AUCUN appel du soir validé n'a couverts, comptés à part.
-- `ensembles_connus` (F-2) : faux pour une ligne écrite avant I3 (colonnes par défaut de 2825, ensembles
-- des partis et des non-couverts inconnus) ; la relecture ne la compare alors qu'avec les ensembles
-- qu'elle sait avoir portés (tolérance documentée : pas de rectificative artificielle).
ALTER TABLE ent_internat_transmission
    ADD COLUMN IF NOT EXISTS ensembles_connus BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS nb_non_couverts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS eleves_non_couverts UUID[] NOT NULL DEFAULT ARRAY[]::uuid[];
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_transmission_non_couverts_chk') THEN
        ALTER TABLE ent_internat_transmission ADD CONSTRAINT ent_internat_transmission_non_couverts_chk
            CHECK (nb_non_couverts >= 0 AND nb_non_couverts = cardinality(eleves_non_couverts));
    END IF;
END
$$;

-- ───────────────────────── 2. Catalogue du socle 18 : autorisations annuelles de sortie ─────────────────────────
-- Objet « sortie_internat » : sorties permises, horaires, personnes (§10.2). Texte de départ provisoire,
-- à relire par la direction et un juriste avant tout usage réel (R-FC72). Désaccord connu : objet « sortie ».
INSERT INTO ent_acc_catalogue
    (code, nature, regle_min, avis_eleve_des_ans, droit_lien_code, preuve_min, validite, sujet_type,
     desaccord_objet, libelle, texte_depart)
VALUES
 ('sortie_internat', 'autorisation', 'un', NULL, 'autoriser_sorties', 'S1', 'annee_scolaire', 'eleve', 'sortie',
  'Sorties de l''internat',
  'Autorisation, pour l''année scolaire, de sorties de l''interne dans le cadre du règlement de l''internat : sortie du mercredi après-midi, sortie de soirée, retour tardif, départ seul le week-end. Chaque usage se choisit séparément, un refus n''empêche pas le séjour à l''internat. Vous pouvez retirer cet accord à tout moment : les sorties à venir sont alors suspendues.')
ON CONFLICT (code) DO NOTHING;
INSERT INTO ent_acc_catalogue_usage (catalogue_code, code, libelle, mission, ordre)
VALUES
 ('sortie_internat', 'sortie_mercredi', 'Sortie du mercredi après-midi', false, 1),
 ('sortie_internat', 'sortie_soiree', 'Sortie de soirée (activité, entraînement)', false, 2),
 ('sortie_internat', 'retour_tardif', 'Retour tardif après une soirée', false, 3),
 ('sortie_internat', 'week_end_seul', 'Départ seul le week-end (transports)', false, 4),
 ('sortie_internat', 'nuit_correspondant', 'Nuit chez le correspondant', false, 5)
ON CONFLICT (catalogue_code, code) DO NOTHING;

-- ───────────────────────── 3. Types de sortie (S-4105) ─────────────────────────
-- Réglage par établissement, créé à la première lecture à partir des défauts du §10.2 (colonnes typées).
CREATE TABLE IF NOT EXISTS ent_internat_sortie_type (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code IN (
        'mercredi', 'soiree', 'nuit_correspondant', 'week_end', 'retour_tardif', 'exceptionnelle')),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 80),
    actif BOOLEAN NOT NULL DEFAULT true,
    -- Ce que la famille doit donner : accord annuel (18), l'un ou l'autre, accord sur la sortie, ou rien
    -- (règle implicite du week-end : l'enfant rentre chez lui, avec une personne autorisée).
    autorisation TEXT NOT NULL CHECK (autorisation IN ('annuelle', 'annuelle_ou_ponctuelle', 'ponctuelle', 'aucune')),
    decideur TEXT NOT NULL CHECK (decideur IN ('titulaires', 'cpe', 'titulaires_et_cpe')),
    seul_permis BOOLEAN NOT NULL,
    eleve_peut_demander BOOLEAN NOT NULL,
    correspondant_requis BOOLEAN NOT NULL DEFAULT false,
    duree_max_heures SMALLINT NOT NULL CHECK (duree_max_heures BETWEEN 1 AND 200),
    usage_accord_code TEXT NOT NULL CHECK (usage_accord_code ~ '^[a-z][a-z0-9_]{1,39}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, code)
);

CREATE OR REPLACE FUNCTION ent_internat_i3_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : suppression interdite (R-IT3), désactiver ou terminer par un fait daté', TG_TABLE_NAME
        USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_internat_sortie_type_sans_suppression ON ent_internat_sortie_type;
CREATE TRIGGER ent_internat_sortie_type_sans_suppression BEFORE DELETE ON ent_internat_sortie_type
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i3_sans_suppression();

-- Correspondants (§14.2) : désignés par un titulaire (ou le majeur) parmi les personnes autorisées à
-- récupérer l'élève (socle 4), avec leurs jours ; l'adresse n'est jamais portée ici.
CREATE TABLE IF NOT EXISTS ent_internat_correspondant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    lundi BOOLEAN NOT NULL DEFAULT false,
    mardi BOOLEAN NOT NULL DEFAULT false,
    mercredi BOOLEAN NOT NULL DEFAULT false,
    jeudi BOOLEAN NOT NULL DEFAULT false,
    vendredi BOOLEAN NOT NULL DEFAULT false,
    samedi BOOLEAN NOT NULL DEFAULT false,
    dimanche BOOLEAN NOT NULL DEFAULT false,
    du DATE NOT NULL,
    au DATE,
    statut TEXT NOT NULL DEFAULT 'valide' CHECK (statut IN ('valide', 'suspendu', 'termine')),
    designe_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_correspondant_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_correspondant_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_correspondant_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_correspondant_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_internat_correspondant_jours_chk CHECK (
        lundi OR mardi OR mercredi OR jeudi OR vendredi OR samedi OR dimanche)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_correspondant_actif_uq
    ON ent_internat_correspondant (etablissement_id, eleve_id, personne_id) WHERE statut <> 'termine';
CREATE INDEX IF NOT EXISTS ent_internat_correspondant_eleve_idx
    ON ent_internat_correspondant (etablissement_id, eleve_id) WHERE statut <> 'termine';

-- Absences d'internat annoncées (§10.1, R-IT19, R-IT20, CIT-37).
--   * annonce : avant l'heure limite → « annoncee », en vigueur aussitôt (attendu recalculé) ; après l'heure
--     limite → « demandee » (le CPE décide, aucun effet avant) ;
--   * regularisation : nuit déjà passée (CIT-37) → « demandee » puis « regularisee » par le CPE ;
--   * « bloquee » : désaccord connu des titulaires ou opposition d'un titulaire (le plus restrictif l'emporte
--     tant que la direction n'a pas tranché) ; aucun effet sur l'attendu.
CREATE TABLE IF NOT EXISTS ent_internat_absence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    du_nuit DATE NOT NULL,
    au_nuit DATE NOT NULL,
    mode TEXT NOT NULL DEFAULT 'annonce' CHECK (mode IN ('annonce', 'regularisation')),
    annonce_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    annonce_qualite TEXT NOT NULL CHECK (annonce_qualite IN ('titulaire', 'eleve_majeur', 'personnel')),
    recupere_par_personne_id UUID,
    retour_prevu TIMESTAMPTZ,
    statut TEXT NOT NULL CHECK (statut IN (
        'demandee', 'annoncee', 'acceptee', 'refusee', 'bloquee', 'regularisee', 'annulee')),
    tardive BOOLEAN NOT NULL DEFAULT false,
    -- Dernier état en vigueur avant un blocage : la décision de la direction le rétablit ou le refuse.
    avant_blocage TEXT CHECK (avant_blocage IS NULL OR avant_blocage IN ('demandee', 'annoncee', 'acceptee')),
    decide_par_user_id UUID REFERENCES ent_user (id),
    decide_le TIMESTAMPTZ,
    motif_refus_texte TEXT CHECK (motif_refus_texte IS NULL OR char_length(btrim(motif_refus_texte)) BETWEEN 1 AND 300),
    -- Départ contrôlé au portail (C-1) : heure du geste et personne à qui l'élève a été remis (NULL : départ seul,
    -- autorisé par l'accord annuel du socle 18 ou élève majeur).
    depart_effectif TIMESTAMPTZ,
    depart_remis_a_personne_id UUID,
    -- Blocage posé après le début de la nuit : il ne porte que sur les nuits SUIVANTES. L'absence continue de
    -- retirer l'élève de l'attendu des nuits strictement antérieures à cette date (la nuit en cours reste couverte,
    -- le CPE est alerté sans que l'élève revienne à l'attendu).
    blocage_effet_nuit DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_absence_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_absence_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_absence_personne_cfk FOREIGN KEY (etablissement_id, recupere_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_absence_periode_chk CHECK (au_nuit >= du_nuit AND au_nuit - du_nuit <= 60),
    CONSTRAINT ent_internat_absence_blocage_chk CHECK ((statut = 'bloquee') = (avant_blocage IS NOT NULL)),
    CONSTRAINT ent_internat_absence_remis_cfk FOREIGN KEY (etablissement_id, depart_remis_a_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_absence_effet_chk CHECK (blocage_effet_nuit IS NULL OR statut = 'bloquee'),
    -- Une seule absence à la fois par élève sur une nuit (états en vigueur ou en attente) : le doublon est refusé
    -- par la base, y compris entre deux annonces simultanées de deux titulaires.
    CONSTRAINT ent_internat_absence_chevauchement_excl EXCLUDE USING gist (
        etablissement_id WITH =, eleve_id WITH =, daterange(du_nuit, au_nuit, '[]') WITH &&)
        WHERE (statut IN ('demandee', 'annoncee', 'acceptee', 'bloquee', 'regularisee'))
);
-- Rejeu sur une base où la table existait déjà (sans ces colonnes) : ajouts idempotents.
ALTER TABLE ent_internat_absence
    ADD COLUMN IF NOT EXISTS depart_effectif TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS depart_remis_a_personne_id UUID,
    ADD COLUMN IF NOT EXISTS blocage_effet_nuit DATE;
CREATE INDEX IF NOT EXISTS ent_internat_absence_eleve_idx
    ON ent_internat_absence (etablissement_id, eleve_id, du_nuit, au_nuit);
CREATE INDEX IF NOT EXISTS ent_internat_absence_a_traiter_idx
    ON ent_internat_absence (etablissement_id, statut) WHERE statut IN ('demandee', 'bloquee');

-- Sorties individuelles (§10.2, §10.4) : une sortie est un engagement daté. Statuts :
-- demandee → autorisee (accord de la famille et/ou décision du CPE selon le type) → partie → rentree ;
-- refusee, annulee, expiree (non utilisée), suspendue (autorisation retirée, interdiction) et bloquee
-- (désaccord) sont des états d'attente ou finaux.
CREATE TABLE IF NOT EXISTS ent_internat_sortie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    type_code TEXT NOT NULL,
    debut TIMESTAMPTZ NOT NULL,
    fin_prevue TIMESTAMPTZ NOT NULL,
    -- Retour attendu à jour : égal à `fin_prevue` sauf retard annoncé (CIT-23) ; sert à l'attendu.
    fin_ajustee TIMESTAMPTZ NOT NULL,
    seul BOOLEAN NOT NULL,
    avec_personne_id UUID,
    correspondant_id UUID,
    -- NULL pour une sortie créée par le système (départ du vendredi, INT-157).
    demande_par_user_id UUID REFERENCES ent_user (id),
    demande_qualite TEXT NOT NULL CHECK (demande_qualite IN ('titulaire', 'eleve_majeur', 'eleve', 'personnel', 'systeme')),
    statut TEXT NOT NULL CHECK (statut IN (
        'demandee', 'autorisee', 'refusee', 'bloquee', 'suspendue', 'partie', 'rentree', 'annulee', 'expiree')),
    avant_blocage TEXT CHECK (avant_blocage IS NULL OR avant_blocage IN ('demandee', 'autorisee', 'suspendue')),
    accord_titulaire_le TIMESTAMPTZ,
    accord_titulaire_user_id UUID REFERENCES ent_user (id),
    decision_cpe_le TIMESTAMPTZ,
    decision_cpe_user_id UUID REFERENCES ent_user (id),
    par_autorisation_annuelle BOOLEAN NOT NULL DEFAULT false,
    depart_effectif TIMESTAMPTZ,
    retour_effectif TIMESTAMPTZ,
    -- Retard annoncé qui ferait couvrir la nuit à une sortie qui ne la couvrait pas (E-2) : demande au CPE, sans
    -- effet tant qu'il n'a pas décidé (`fin_ajustee` ne bouge pas).
    retard_demande_fin TIMESTAMPTZ,
    retard_demande_par_user_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_sortie_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_sortie_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_sortie_type_cfk FOREIGN KEY (etablissement_id, type_code)
        REFERENCES ent_internat_sortie_type (etablissement_id, code),
    CONSTRAINT ent_internat_sortie_personne_cfk FOREIGN KEY (etablissement_id, avec_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_sortie_correspondant_cfk FOREIGN KEY (etablissement_id, correspondant_id)
        REFERENCES ent_internat_correspondant (etablissement_id, id),
    CONSTRAINT ent_internat_sortie_periode_chk CHECK (fin_prevue > debut AND fin_ajustee >= fin_prevue),
    CONSTRAINT ent_internat_sortie_demandeur_chk CHECK ((demande_qualite = 'systeme') = (demande_par_user_id IS NULL)),
    CONSTRAINT ent_internat_sortie_seul_chk CHECK (
        NOT seul OR (avec_personne_id IS NULL AND correspondant_id IS NULL)),
    CONSTRAINT ent_internat_sortie_blocage_chk CHECK ((statut = 'bloquee') = (avant_blocage IS NOT NULL)),
    CONSTRAINT ent_internat_sortie_effectif_chk CHECK (
        (depart_effectif IS NULL OR statut IN ('partie', 'rentree'))
        AND (retour_effectif IS NULL OR (statut = 'rentree' AND depart_effectif IS NOT NULL
                                         AND retour_effectif >= depart_effectif))
        AND (statut NOT IN ('partie', 'rentree') OR depart_effectif IS NOT NULL)
        AND (statut <> 'rentree' OR retour_effectif IS NOT NULL)),
    -- Une sortie à la fois par élève (états en vigueur) : le chevauchement est refusé par la base.
    CONSTRAINT ent_internat_sortie_chevauchement_excl EXCLUDE USING gist (
        etablissement_id WITH =, eleve_id WITH =, tstzrange(debut, fin_ajustee, '[)') WITH &&)
        WHERE (statut IN ('demandee', 'autorisee', 'suspendue', 'partie', 'bloquee'))
);
ALTER TABLE ent_internat_sortie
    ADD COLUMN IF NOT EXISTS retard_demande_fin TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS retard_demande_par_user_id UUID REFERENCES ent_user (id);
CREATE INDEX IF NOT EXISTS ent_internat_sortie_eleve_idx
    ON ent_internat_sortie (etablissement_id, eleve_id, debut);
CREATE INDEX IF NOT EXISTS ent_internat_sortie_jour_idx
    ON ent_internat_sortie (etablissement_id, debut) WHERE statut IN ('autorisee', 'partie', 'suspendue', 'bloquee', 'demandee');

-- Faits de l'absence, de la sortie et du correspondant : ajout seul (qui, quand, quoi, pourquoi), jamais de
-- nom ni de santé. Le motif est un code ; le texte (300 caractères) est contrôlé par le service.
CREATE TABLE IF NOT EXISTS ent_internat_i3_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID,
    sortie_id UUID,
    correspondant_id UUID,
    acte TEXT NOT NULL CHECK (acte IN (
        'annonce', 'demande', 'regularisation_demandee', 'accord_titulaire', 'acceptation', 'refus', 'blocage',
        'opposition', 'tranchee_maintenue', 'tranchee_levee', 'regularisation', 'annulation', 'depart', 'retour',
        'retard_annonce', 'revocation', 'suspension', 'reprise', 'expiration', 'designation', 'fin',
        'depart_refuse', 'accord_remis_a_zero', 'retard_demande', 'retard_refuse', 'signalement')),
    qualite TEXT NOT NULL CHECK (qualite IN (
        'titulaire', 'eleve_majeur', 'eleve', 'personnel', 'cpe', 'direction', 'systeme')),
    par_user_id UUID REFERENCES ent_user (id),
    a TIMESTAMPTZ NOT NULL,
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'famille', 'tardive', 'desaccord_connu', 'desaccord_titulaires', 'autorisation_revoquee',
        'interdiction_remise', 'interdiction_auteur', 'personne_non_autorisee', 'correspondant_suspendu', 'examen', 'reglement',
        'erreur', 'train', 'autre')),
    motif_texte TEXT CHECK (motif_texte IS NULL OR char_length(btrim(motif_texte)) BETWEEN 1 AND 300),
    -- Personne à qui l'élève est remis (départ) ; correspondant ou opposant : jamais un nom.
    personne_id UUID,
    jusqu_a TIMESTAMPTZ,
    -- clock_timestamp() : l'ordre d'écriture de deux faits d'une même transaction (opposition, puis blocage) est
    -- déterministe pour l'historique.
    cree_le TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT ent_internat_i3_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_i3_fait_absence_cfk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_internat_absence (etablissement_id, id),
    CONSTRAINT ent_internat_i3_fait_sortie_cfk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_internat_sortie (etablissement_id, id),
    CONSTRAINT ent_internat_i3_fait_correspondant_cfk FOREIGN KEY (etablissement_id, correspondant_id)
        REFERENCES ent_internat_correspondant (etablissement_id, id),
    CONSTRAINT ent_internat_i3_fait_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_i3_fait_objet_chk CHECK (
        (absence_id IS NOT NULL)::int + (sortie_id IS NOT NULL)::int + (correspondant_id IS NOT NULL)::int = 1)
);
CREATE INDEX IF NOT EXISTS ent_internat_i3_fait_absence_idx
    ON ent_internat_i3_fait (etablissement_id, absence_id, a) WHERE absence_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_internat_i3_fait_sortie_idx
    ON ent_internat_i3_fait (etablissement_id, sortie_id, a) WHERE sortie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_internat_i3_fait_correspondant_idx
    ON ent_internat_i3_fait (etablissement_id, correspondant_id, a) WHERE correspondant_id IS NOT NULL;
DROP TRIGGER IF EXISTS ent_internat_i3_fait_ajout_seul ON ent_internat_i3_fait;
CREATE TRIGGER ent_internat_i3_fait_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_i3_fait
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

-- Gardes de transition : ni suppression, ni modification de ce qui fonde l'engagement ; seules les
-- transitions du cycle de vie sont possibles (R-IT3).
CREATE OR REPLACE FUNCTION ent_internat_absence_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_absence : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.du_nuit, OLD.au_nuit, OLD.mode, OLD.annonce_par_user_id,
        OLD.annonce_qualite, OLD.tardive, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.du_nuit, NEW.au_nuit, NEW.mode,
        NEW.annonce_par_user_id, NEW.annonce_qualite, NEW.tardive, NEW.cree_le) THEN
        RAISE EXCEPTION 'absence annoncée : élève, nuits, auteur et mode ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.statut <> OLD.statut AND NOT (
           (OLD.statut = 'demandee' AND NEW.statut IN ('acceptee', 'refusee', 'bloquee', 'annulee', 'regularisee'))
        OR (OLD.statut = 'annoncee' AND NEW.statut IN ('acceptee', 'refusee', 'bloquee', 'annulee'))
        OR (OLD.statut = 'acceptee' AND NEW.statut IN ('bloquee', 'annulee'))
        OR (OLD.statut = 'bloquee' AND NEW.statut IN ('demandee', 'annoncee', 'acceptee', 'refusee', 'annulee'))) THEN
        RAISE EXCEPTION 'absence annoncée : transition % → % refusée', OLD.statut, NEW.statut USING ERRCODE = '42501';
    END IF;
    IF OLD.depart_effectif IS NOT NULL AND (OLD.depart_effectif, OLD.depart_remis_a_personne_id)
       IS DISTINCT FROM (NEW.depart_effectif, NEW.depart_remis_a_personne_id) THEN
        RAISE EXCEPTION 'absence annoncée : le départ est un fait, il ne se modifie pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.statut <> 'bloquee' AND NEW.blocage_effet_nuit IS NOT NULL THEN
        RAISE EXCEPTION 'absence annoncée : l''effet différé d''un blocage ne vaut que tant qu''elle est bloquée'
            USING ERRCODE = '42501';
    END IF;
    IF OLD.statut IN ('refusee', 'annulee', 'regularisee') AND NEW.statut = OLD.statut
       AND (OLD.recupere_par_personne_id, OLD.retour_prevu, OLD.decide_par_user_id, OLD.decide_le,
            OLD.motif_refus_texte)
          IS DISTINCT FROM (NEW.recupere_par_personne_id, NEW.retour_prevu, NEW.decide_par_user_id, NEW.decide_le,
            NEW.motif_refus_texte) THEN
        RAISE EXCEPTION 'absence annoncée : état final, plus de modification' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_absence_garde ON ent_internat_absence;
CREATE TRIGGER ent_internat_absence_garde BEFORE UPDATE OR DELETE ON ent_internat_absence
    FOR EACH ROW EXECUTE FUNCTION ent_internat_absence_garde();

CREATE OR REPLACE FUNCTION ent_internat_sortie_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_sortie : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.type_code, OLD.debut, OLD.fin_prevue,
        OLD.demande_par_user_id, OLD.demande_qualite, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.type_code, NEW.debut, NEW.fin_prevue,
        NEW.demande_par_user_id, NEW.demande_qualite, NEW.cree_le) THEN
        RAISE EXCEPTION 'sortie : élève, type, horaires prévus et demandeur ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.statut <> OLD.statut AND NOT (
           (OLD.statut = 'demandee' AND NEW.statut IN ('autorisee', 'refusee', 'bloquee', 'annulee', 'expiree'))
        OR (OLD.statut = 'autorisee' AND NEW.statut IN ('partie', 'suspendue', 'bloquee', 'annulee', 'expiree', 'demandee'))
        OR (OLD.statut = 'suspendue' AND NEW.statut IN ('autorisee', 'bloquee', 'annulee', 'refusee', 'expiree', 'demandee'))
        OR (OLD.statut = 'bloquee' AND NEW.statut IN ('demandee', 'autorisee', 'suspendue', 'refusee', 'annulee', 'expiree'))
        OR (OLD.statut = 'partie' AND NEW.statut = 'rentree')) THEN
        RAISE EXCEPTION 'sortie : transition % → % refusée', OLD.statut, NEW.statut USING ERRCODE = '42501';
    END IF;
    IF OLD.statut IN ('refusee', 'annulee', 'rentree', 'expiree') AND (OLD.statut, OLD.fin_ajustee, OLD.seul,
        OLD.avec_personne_id, OLD.correspondant_id, OLD.depart_effectif, OLD.retour_effectif)
       IS DISTINCT FROM (NEW.statut, NEW.fin_ajustee, NEW.seul, NEW.avec_personne_id, NEW.correspondant_id,
        NEW.depart_effectif, NEW.retour_effectif) THEN
        RAISE EXCEPTION 'sortie : état final, plus de modification' USING ERRCODE = '42501';
    END IF;
    IF OLD.depart_effectif IS NOT NULL AND OLD.depart_effectif IS DISTINCT FROM NEW.depart_effectif THEN
        RAISE EXCEPTION 'sortie : l''heure de départ est un fait, elle ne se modifie pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.retour_effectif IS NOT NULL AND OLD.retour_effectif IS DISTINCT FROM NEW.retour_effectif THEN
        RAISE EXCEPTION 'sortie : l''heure de retour est un fait, elle ne se modifie pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.fin_ajustee < OLD.fin_ajustee AND OLD.statut IN ('partie') THEN
        RAISE EXCEPTION 'sortie : un retard annoncé ne raccourcit jamais le retour attendu' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_sortie_garde ON ent_internat_sortie;
CREATE TRIGGER ent_internat_sortie_garde BEFORE UPDATE OR DELETE ON ent_internat_sortie
    FOR EACH ROW EXECUTE FUNCTION ent_internat_sortie_garde();

CREATE OR REPLACE FUNCTION ent_internat_correspondant_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_correspondant : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.personne_id, OLD.du, OLD.designe_par_user_id, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.personne_id, NEW.du,
        NEW.designe_par_user_id, NEW.cree_le) THEN
        RAISE EXCEPTION 'correspondant : élève, personne et début ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    IF NEW.statut <> OLD.statut AND NOT (
           (OLD.statut = 'valide' AND NEW.statut IN ('suspendu', 'termine'))
        OR (OLD.statut = 'suspendu' AND NEW.statut IN ('valide', 'termine'))) THEN
        RAISE EXCEPTION 'correspondant : transition % → % refusée', OLD.statut, NEW.statut USING ERRCODE = '42501';
    END IF;
    IF OLD.statut = 'termine' AND (OLD.statut, OLD.au, OLD.lundi, OLD.mardi, OLD.mercredi, OLD.jeudi,
        OLD.vendredi, OLD.samedi, OLD.dimanche)
       IS DISTINCT FROM (NEW.statut, NEW.au, NEW.lundi, NEW.mardi, NEW.mercredi, NEW.jeudi, NEW.vendredi,
        NEW.samedi, NEW.dimanche) THEN
        RAISE EXCEPTION 'correspondant : désignation terminée, plus de modification' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_correspondant_garde ON ent_internat_correspondant;
CREATE TRIGGER ent_internat_correspondant_garde BEFORE UPDATE OR DELETE ON ent_internat_correspondant
    FOR EACH ROW EXECUTE FUNCTION ent_internat_correspondant_garde();

-- TRUNCATE interdit sur les tables de ce lot (E2) : les déclencheurs de ligne ne couvrent pas TRUNCATE.
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'ent_internat_sortie_type', 'ent_internat_correspondant', 'ent_internat_absence',
        'ent_internat_sortie', 'ent_internat_i3_fait', 'ent_internat_transmission']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_truncate', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT '
                       'EXECUTE FUNCTION ent_internat_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
END
$$;

-- ───────────────────────── 4. L'attendu de la nuit lit les absences et les sorties ─────────────────────────
-- Celle de 2825, plus deux sources de retrait :
--   * une absence (M-B, même raisonnement qu'E-3 : « pas de départ constaté = attendu »). Pour un MINEUR, une absence
--     annoncée ou acceptée ne retire l'élève qu'à partir du départ effectif contrôlé au portail (`depart_effectif`,
--     antérieur à l'ancre de la nuit) : un départ refusé, ou jamais venu, le laisse attendu. Un MAJEUR (il part seul) et
--     une régularisation (nuit passée, aucun départ à contrôler) retirent dès l'annonce. Un blocage ne prolonge le
--     retrait de la nuit en cours que pour un élève réellement parti (`blocage_effet_nuit`) ;
--   * une sortie PARTIE ou rentrée (E-3 : jamais une sortie seulement autorisée) qui COUVRE la nuit : commencée au plus tard à 22 h (heure murale
--     de l'établissement) et dont le retour (effectif, sinon attendu à jour) tombe après 6 h du lendemain.
--     Une sortie plus courte (mercredi après-midi, soirée) ne retire jamais l'élève de l'attendu : il
--     reste attendu et se pointe « en activité » avec son retour attendu, sans quoi son absence passerait
--     inaperçue si le retour n'a pas lieu.
-- `ent_internat_eleve_majeur` : majorité (socle 2) et capacité juridique (émancipé = majeur ; tutelle = jamais), seule
-- source de vérité, relue par le service Rust.
CREATE OR REPLACE FUNCTION ent_internat_eleve_majeur(etab UUID, eleve UUID, jour DATE)
RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT (EXISTS (SELECT 1 FROM ent_eleve_capacite k WHERE k.etablissement_id = etab AND k.eleve_id = eleve
                       AND k.nature = 'emancipation' AND k.levee_le IS NULL AND k.du <= jour
                       AND (k.au IS NULL OR k.au >= jour))
            OR COALESCE((SELECT ent_eleve_majorite_le(e.date_naissance, e.date_naissance_precision,
                                                      e.date_naissance_statut) <= jour
                           FROM ent_eleve e WHERE e.etablissement_id = etab AND e.id = eleve), false))
       AND NOT EXISTS (SELECT 1 FROM ent_eleve_capacite k WHERE k.etablissement_id = etab AND k.eleve_id = eleve
                        AND k.nature = 'tutelle' AND k.levee_le IS NULL AND k.du <= jour
                        AND (k.au IS NULL OR k.au >= jour))
$$;

-- ───────────────────────── Interdictions et remise : règles en SQL (passe 3) ─────────────────────────
-- Une seule définition, appelée une fois par ligne (contrôle de départ) ou en lot (revue à la minute : une requête par
-- établissement, jamais une par ligne). Aucune ne lit de donnée hors de l'établissement passé en paramètre.

-- Une personne et TOUTE sa chaîne de fusions, dans les deux sens : les fiches fusionnées dans elle, et la fiche dans
-- laquelle elle est elle-même fusionnée (et ainsi de suite). Une interdiction posée sur n'importe quelle fiche de la
-- chaîne vise la même personne réelle.
CREATE OR REPLACE FUNCTION ent_internat_personne_chaine(p_etab UUID, p_personne UUID)
RETURNS SETOF UUID LANGUAGE sql STABLE AS $$
    WITH RECURSIVE chaine(id) AS (
        SELECT p_personne
        UNION
        SELECT CASE WHEN s.id = c.id THEN s.fusionnee_dans ELSE s.id END
          FROM ent_personne s
          JOIN chaine c ON s.etablissement_id = p_etab
                       AND ((s.id = c.id AND s.fusionnee_dans IS NOT NULL) OR s.fusionnee_dans = c.id))
    SELECT id FROM chaine
$$;

-- Date de pose (la plus récente) des interdictions de remise, de visite ou de contact qui visent la personne (chaîne de
-- fusions comprise), non levées et non terminées ; NULL si elle n'est pas visée. La date de début est ignorée (décision
-- pessimiste : une interdiction posée en urgence joue tout de suite).
CREATE OR REPLACE FUNCTION ent_internat_personne_visee(p_etab UUID, p_eleve UUID, p_personne UUID, p_jour DATE)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT max(i.created_at) FROM ent_eleve_interdiction i
     WHERE i.etablissement_id = p_etab AND i.eleve_id = p_eleve AND i.levee_le IS NULL
       AND i.objet && ARRAY['remise', 'visite', 'contact_eleve']::text[]
       AND (i.au IS NULL OR i.au >= p_jour) AND i.personne_id IS NOT NULL
       AND i.personne_id IN (SELECT ent_internat_personne_chaine(p_etab, p_personne))
$$;

-- Le compte est-il « visé » : l'une des fiches qui sont les siennes (fiche personne rattachée au compte, ou lien avec
-- l'élève) est visée par une interdiction (voir ci-dessus). Date de pose la plus récente, NULL sinon.
CREATE OR REPLACE FUNCTION ent_internat_user_vise(p_etab UUID, p_eleve UUID, p_user UUID, p_jour DATE)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT max(ent_internat_personne_visee(p_etab, p_eleve, x.pid, p_jour))
      FROM (SELECT id AS pid FROM ent_personne WHERE etablissement_id = p_etab AND user_id = p_user
            UNION
            SELECT personne_id FROM ent_eleve_lien
             WHERE etablissement_id = p_etab AND eleve_id = p_eleve AND user_id = p_user) x
$$;

-- Date de pose de l'interdiction SANS personne identifiée (non nominative) la plus récente, non levée et non terminée ;
-- `p_remise_seule` limite aux interdictions d'objet « remise ».
CREATE OR REPLACE FUNCTION ent_internat_interdiction_sans_cible(
    p_etab UUID, p_eleve UUID, p_jour DATE, p_remise_seule BOOLEAN)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT max(i.created_at) FROM ent_eleve_interdiction i
     WHERE i.etablissement_id = p_etab AND i.eleve_id = p_eleve AND i.levee_le IS NULL AND i.personne_id IS NULL
       AND (CASE WHEN p_remise_seule THEN 'remise' = ANY (i.objet)
                 ELSE i.objet && ARRAY['remise', 'visite', 'contact_eleve']::text[] END)
       AND (i.au IS NULL OR i.au >= p_jour)
$$;

-- Une personne peut-elle prendre l'élève ce jour-là ? Code : « interdite » (visée par une interdiction, toujours
-- prioritaire), « sans_cible » (une interdiction sans personne identifiée est en vigueur et la direction ne l'a pas levée
-- pour ce cas : `p_sans_cible_levee`), « autorisee » (+ relation) ou « non_autorisee ».
--   * titulaire : lien actif, autorité parentale, garde non « aucune », non fusionnée ;
--   * personne autorisée : contact validé, « peut récupérer », dont aucun lien avec l'élève (sur toute la chaîne de
--     fusions) n'est suspendu, en garde « aucune », ni celui d'un parent ou tuteur privé de l'autorité parentale, ni celui
--     d'un parent ou tuteur dont le lien est TERMINÉ (date de fin passée) ou PAS ENCORE COMMENCÉ : décision prudente (un
--     parent dont le lien est simplement échu ne prend pas l'élève comme « contact » ; la direction rétablit le lien) ;
--     un proche de lien « autre » (grand-parent, beau-parent) n'a jamais eu l'autorité parentale et reste autorisé.
CREATE OR REPLACE FUNCTION ent_internat_remise(
    p_etab UUID, p_eleve UUID, p_personne UUID, p_jour DATE, p_sans_cible_levee BOOLEAN)
RETURNS TABLE (code TEXT, relation TEXT) LANGUAGE sql STABLE AS $$
    WITH base AS (
        SELECT ent_internat_personne_visee(p_etab, p_eleve, p_personne, p_jour) IS NOT NULL AS visee,
               (NOT p_sans_cible_levee
                AND ent_internat_interdiction_sans_cible(p_etab, p_eleve, p_jour, false) IS NOT NULL) AS sans_cible,
               (SELECT x.rel FROM (
                    SELECT COALESCE(NULLIF(btrim(l.lien), ''), 'titulaire') AS rel
                      FROM ent_eleve_lien l
                      JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
                     WHERE l.etablissement_id = p_etab AND l.eleve_id = p_eleve AND l.personne_id = p_personne
                       AND l.suspendu_le IS NULL AND l.autorite_parentale AND COALESCE(l.garde, '') <> 'aucune'
                       AND (l.date_debut IS NULL OR l.date_debut <= p_jour)
                       AND (l.date_fin IS NULL OR l.date_fin >= p_jour)
                       AND p.fusionnee_dans IS NULL
                    UNION ALL
                    SELECT COALESCE(NULLIF(btrim(c.relation), ''), 'personne autorisée')
                      FROM ent_eleve_contact_urgence c
                      JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id
                     WHERE c.etablissement_id = p_etab AND c.eleve_id = p_eleve AND c.personne_id = p_personne
                       AND c.peut_recuperer AND c.statut = 'valide' AND p.fusionnee_dans IS NULL
                       AND NOT EXISTS (
                            SELECT 1 FROM ent_eleve_lien sl
                             WHERE sl.etablissement_id = p_etab AND sl.eleve_id = p_eleve
                               AND sl.personne_id IN (SELECT ent_internat_personne_chaine(p_etab, p_personne))
                               AND (sl.suspendu_le IS NOT NULL OR COALESCE(sl.garde, '') = 'aucune'
                                    OR (NOT sl.autorite_parentale AND sl.lien IN ('mere', 'pere', 'tuteur'))
                                    OR (sl.lien IN ('mere', 'pere', 'tuteur')
                                        AND ((sl.date_fin IS NOT NULL AND sl.date_fin < p_jour)
                                             OR (sl.date_debut IS NOT NULL AND sl.date_debut > p_jour)))))
                ) x LIMIT 1) AS rel
    )
    SELECT CASE WHEN visee THEN 'interdite'
                WHEN sans_cible THEN 'sans_cible'
                WHEN rel IS NOT NULL THEN 'autorisee'
                ELSE 'non_autorisee' END,
           CASE WHEN visee OR sans_cible THEN NULL ELSE rel END
      FROM base
$$;

CREATE OR REPLACE FUNCTION ent_internat_attendu_calcule(etab UUID, nuit DATE)
RETURNS TABLE (eleve_id UUID, chambre_id UUID, lit_id UUID, statut TEXT, motif_code TEXT)
LANGUAGE sql STABLE AS $$
    WITH ancre AS (
        SELECT ent_internat_ancre_nuit(etab, nuit) AS t, ent_internat_fuseau(etab) AS tz,
               (((nuit + 1) + TIME '06:00') AT TIME ZONE ent_internat_fuseau(etab)) AS matin
    ),
    du_rythme AS (
        SELECT s.eleve_id
          FROM ent_internat_sejour s
         WHERE s.etablissement_id = etab AND s.du <= nuit AND (s.au IS NULL OR s.au >= nuit)
           AND COALESCE((
                SELECT CASE extract(dow FROM nuit)::int
                         WHEN 0 THEN r.dimanche WHEN 1 THEN r.lundi WHEN 2 THEN r.mardi
                         WHEN 3 THEN r.mercredi WHEN 4 THEN r.jeudi WHEN 5 THEN r.vendredi
                         ELSE r.samedi END
                  FROM ent_internat_rythme r
                 WHERE r.etablissement_id = etab AND r.sejour_id = s.id AND r.du <= nuit
                 ORDER BY r.du DESC LIMIT 1), false)
    ),
    ajoutes AS (
        SELECT x.eleve_id FROM ent_internat_nuit_exceptionnelle x
         WHERE x.etablissement_id = etab AND x.nuit = ent_internat_attendu_calcule.nuit AND x.annulee_le IS NULL
    ),
    hebergees AS (
        SELECT DISTINCT ON (a.eleve_id) a.eleve_id, a.chambre_id, a.lit_id
          FROM ent_internat_affectation a
         WHERE a.etablissement_id = etab AND a.debut <= nuit AND (a.fin IS NULL OR a.fin >= nuit)
         ORDER BY a.eleve_id, a.debut DESC
    ),
    retraits AS (
        SELECT r.eleve_id, r.motif_code AS motif FROM ent_internat_retrait r
         WHERE r.etablissement_id = etab AND r.annule_le IS NULL AND r.du_nuit <= nuit AND r.au_nuit >= nuit
        UNION ALL
        SELECT ab.eleve_id, 'absence_annoncee' FROM ent_internat_absence ab, ancre
         WHERE ab.etablissement_id = etab AND ab.du_nuit <= nuit AND ab.au_nuit >= nuit
           AND (ab.statut = 'regularisee'
                -- Passe 3 : une absence dont l'auteur ou la personne remise est devenu visé APRÈS le départ ouvre un cas
                -- pour le CPE ; tant qu'il n'a pas tranché (cas ouvert), les nuits SUIVANTES ne retirent plus l'élève.
                OR (NOT EXISTS (SELECT 1 FROM ent_internat_cas cx
                                 WHERE cx.etablissement_id = etab AND cx.eleve_id = ab.eleve_id
                                   AND cx.nature = 'interdiction_auteur' AND cx.issue IS NULL
                                   AND cx.nuit < ent_internat_attendu_calcule.nuit)
                    AND ab.statut IN ('annoncee', 'acceptee')
                    AND (ent_internat_eleve_majeur(etab, ab.eleve_id, nuit)
                         OR (ab.depart_effectif IS NOT NULL AND ab.depart_effectif <= ancre.t)))
                -- Blocage posé après le début d'une nuit : il ne vaut, pour un élève réellement parti, que pour les
                -- nuits suivantes.
                OR (ab.statut = 'bloquee' AND ab.blocage_effet_nuit IS NOT NULL AND nuit < ab.blocage_effet_nuit
                    AND ab.depart_effectif IS NOT NULL AND ab.depart_effectif <= ancre.t))
        UNION ALL
        -- E-3 : seule une sortie effectivement PARTIE (ou rentrée) retire l'élève de l'attendu. Une sortie
        -- autorisée qui n'a jamais eu lieu (départ refusé au portail, famille absente) le laisse attendu : sans
        -- cela, un élève resté à l'internat ne serait ni appelé ni cherché.
        SELECT so.eleve_id, 'sortie' FROM ent_internat_sortie so, ancre
         WHERE so.etablissement_id = etab AND so.statut IN ('partie', 'rentree')
           AND NOT EXISTS (SELECT 1 FROM ent_internat_cas cx
                            WHERE cx.etablissement_id = etab AND cx.eleve_id = so.eleve_id
                              AND cx.nature = 'interdiction_auteur' AND cx.issue IS NULL
                              AND cx.nuit < ent_internat_attendu_calcule.nuit)
           AND so.debut <= ancre.t
           AND COALESCE(so.retour_effectif, so.fin_ajustee) >= ancre.matin
        UNION ALL
        SELECT sit.eleve_id,
               CASE sit.nature WHEN 'declaration' THEN 'absence_annoncee' WHEN 'sortie' THEN 'sortie'
                               WHEN 'mobilite' THEN 'mobilite' ELSE 'stage' END
          FROM ent_asd_situation sit, ancre
         WHERE sit.etablissement_id = etab AND sit.etat = 'active'
           AND sit.nature IN ('declaration', 'sortie', 'mobilite', 'pfmp')
           AND ((sit.debut_jour + COALESCE(sit.debut_heure, TIME '00:00')) AT TIME ZONE ancre.tz) <= ancre.t
           AND ancre.t < ((sit.fin_jour + COALESCE(sit.fin_heure, TIME '23:59:59')) AT TIME ZONE ancre.tz)
        UNION ALL
        SELECT m.eleve_id, 'mobilite' FROM ent_eleve_mobilite m
         WHERE m.etablissement_id = etab AND m.ferme_le IS NULL AND m.du <= nuit AND (m.au IS NULL OR m.au >= nuit)
        UNION ALL
        SELECT p.eleve_id, 'sejour_scolaire'
          FROM ent_sortie so
          JOIN ent_sortie_participant p ON p.etablissement_id = so.etablissement_id AND p.sortie_id = so.id
          CROSS JOIN ancre
         WHERE so.etablissement_id = etab AND so.etat IN ('prete', 'en_cours')
           AND p.etat IN ('pret', 'parti', 'remis', 'parti_seul_autorise')
           AND so.debut <= ancre.t AND ancre.t < so.fin
        UNION ALL
        SELECT d.eleve_id, 'fermeture'
          FROM (SELECT eleve_id FROM du_rythme UNION SELECT eleve_id FROM ajoutes) d
         WHERE EXISTS (
                SELECT 1 FROM ent_calendrier_evenement ev
                 WHERE ev.etablissement_id = etab AND ev.type = 'fermeture_exceptionnelle'
                   AND ev.annule_le IS NULL AND 'internat' = ANY (ev.services_touches)
                   AND NOT ev.internes_presents AND ev.du <= nuit AND ev.au >= nuit)
    ),
    retrait_retenu AS (
        SELECT DISTINCT ON (r.eleve_id) r.eleve_id, r.motif
          FROM retraits r
         ORDER BY r.eleve_id, CASE r.motif
               WHEN 'fermeture' THEN 1 WHEN 'exclusion_internat' THEN 2 WHEN 'sante' THEN 3
               WHEN 'sejour_scolaire' THEN 4 WHEN 'stage' THEN 5 WHEN 'mobilite' THEN 6
               WHEN 'sortie' THEN 7 ELSE 8 END
    ),
    concerne AS (
        SELECT eleve_id, 'attendu' AS origine FROM du_rythme
        UNION
        SELECT eleve_id, 'ajoute' FROM ajoutes
        UNION
        SELECT eleve_id, 'non_attendu' FROM hebergees
    ),
    unique_eleve AS (
        SELECT DISTINCT ON (c.eleve_id) c.eleve_id, c.origine
          FROM concerne c
          JOIN ent_eleve el ON el.etablissement_id = etab AND el.id = c.eleve_id
         WHERE el.date_sortie IS NULL OR el.date_sortie > nuit
         ORDER BY c.eleve_id, CASE c.origine WHEN 'attendu' THEN 1 WHEN 'ajoute' THEN 2 ELSE 3 END
    )
    SELECT u.eleve_id, h.chambre_id, h.lit_id,
           CASE WHEN u.origine = 'non_attendu' THEN 'non_attendu'
                WHEN rr.motif IS NOT NULL THEN 'retire'
                ELSE u.origine END,
           CASE WHEN u.origine = 'non_attendu' THEN NULL ELSE rr.motif END
      FROM unique_eleve u
      LEFT JOIN hebergees h ON h.eleve_id = u.eleve_id
      LEFT JOIN retrait_retenu rr ON rr.eleve_id = u.eleve_id
$$;

SELECT ent_rls_poser(ARRAY[
    'ent_internat_sortie_type', 'ent_internat_correspondant', 'ent_internat_absence',
    'ent_internat_sortie', 'ent_internat_i3_fait']);

-- Un cas peut être ouvert par le SYSTÈME (INT-161, INT-166 : retard de retour au-delà du seuil, élève non
-- récupéré à l'heure limite) : l'auteur d'ouverture n'est alors pas un compte (NULL, lisible comme « système »).
ALTER TABLE ent_internat_cas ALTER COLUMN ouvert_par_user_id DROP NOT NULL;
-- M-1 : un élève attendu qu'aucun appel du soir validé n'a couvert ouvre un cas « non couvert » pour le CPE
-- (nature ajoutée ; la contrainte de 2824 est remplacée, jamais retirée).
ALTER TABLE ent_internat_cas DROP CONSTRAINT IF EXISTS ent_internat_cas_nature_check;
ALTER TABLE ent_internat_cas ADD CONSTRAINT ent_internat_cas_nature_check CHECK (
    nature IN ('non_retrouve', 'non_rentre', 'non_recupere', 'retard_retour', 'non_couvert', 'interdiction_auteur'));
-- Passe 3 : « interdiction_auteur » = l'auteur d'un acte (demande, accord, désignation de la personne qui récupère,
-- correspondant, autorisation annuelle) ou la personne remise est devenu visé par une interdiction APRÈS le départ de
-- l'élève (absence ou sortie « partie ») : le CPE tranche ; les nuits suivantes ne retirent plus l'élève de l'attendu
-- tant que ce cas est ouvert (voir `ent_internat_attendu_calcule`).

-- Journal des lectures sensibles (R-IT38) : trois objets de plus pour ce lot (liste de remise, sorties d'un élève,
-- correspondants). La contrainte de 2824 est remplacée (même nom automatique), jamais retirée.
ALTER TABLE ent_internat_lecture_sensible DROP CONSTRAINT IF EXISTS ent_internat_lecture_sensible_objet_check;
ALTER TABLE ent_internat_lecture_sensible ADD CONSTRAINT ent_internat_lecture_sensible_objet_check CHECK (objet IN (
    'fiche_nuit', 'liaison', 'presents', 'registre', 'cas', 'appel', 'appel_pdf', 'attendu',
    'liste_appels', 'procedure', 'effectif_repas', 'listes', 'journal',
    'liste_remise', 'sorties', 'correspondants'));
