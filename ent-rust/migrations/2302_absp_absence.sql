-- 2302 — Socle 15 (absences des personnels), lot A1 : le cœur de l'absence
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §8, §9.4, §26, §28.2, §32.6).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; aucun BYTEA (E5) ; RLS posée ; clés
-- étrangères composites vers les lignes d'établissement (E2).
--
-- * `ent_absp_absence` : l'absence, FAIT RH confidentiel. SEULE table à porter la nature (marque santé).
-- * `ent_absp_plage` : plages (absence discontinue) ; chevauchement impossible dans une absence.
-- * `ent_absp_circuit` : copie FIGÉE à la création (valideur, instructeur) — un changement de règle
--   n'altère pas une absence déjà déclarée (AD-07).
-- * `ent_absp_decision` : faits décisionnels en ajout seul ; « personne ne valide sa propre absence »
--   tenu en base (R-AP10).
-- * `ent_absp_journal` : journal du domaine en ajout seul, colonnes typées avant / après (R-AP20).
-- * Projection de transition : `ent_absence_rh` reste lue par l'existant jusqu'au lot A14 ; un
--   déclencheur la tient à jour depuis `ent_absp_absence` avec le motif « Absence », jamais la nature
--   (ferme AR-01 même pour une route héritée oubliée).

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- 1. Absence ----------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_absence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    -- Circuit porté par l'ENGAGEMENT, pas par le rôle (R-AP4). `employeur` : classe de l'employeur de
    -- l'engagement au moment de la déclaration (figée) : `etat` (agent de l'État, maître) ou `ogec`
    -- (salarié de droit privé de l'organisme de gestion) ; elle cloisonne la comptabilité (§23.3).
    engagement_id UUID,
    employeur TEXT NOT NULL DEFAULT 'etat' CHECK (employeur IN ('etat', 'ogec')),
    -- `rh` : absence d'un agent ; `service` : indisponibilité posée par un autre socle (R-AP32).
    cause TEXT NOT NULL DEFAULT 'rh' CHECK (cause IN ('rh', 'service')),
    source_socle SMALLINT CHECK (source_socle IS NULL OR source_socle BETWEEN 1 AND 99),
    source_ref UUID,
    -- NULL tant qu'une absence signalée n'a pas de nature (R-AP46).
    nature_id UUID REFERENCES ent_absp_nature (id),
    -- Texte libre : natures non sensibles seulement, 200 caractères (R-AP1) ; contrôlé par déclencheur.
    texte_libre VARCHAR(200),
    -- Zone de qualification (reprise d'un ancien motif libre, Q-AP15) : réservée à la direction, vidée
    -- à la qualification.
    zone_qualification VARCHAR(200),
    etat TEXT NOT NULL CHECK (etat IN (
        'signalee', 'declaree', 'en_instruction', 'validee', 'refusee', 'prise_en_compte',
        'decision_employeur', 'contestee', 'traitee', 'close', 'annulee', 'prevue', 'passee')),
    -- Intervalle [debut, fin[ en instants (fuseau de l'établissement, R-AP6). `fin_provisoire` :
    -- « je prolongerai », la fin est une estimation.
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    fin_provisoire BOOLEAN NOT NULL DEFAULT false,
    -- L'absent, le déclarant et l'auteur de la saisie sont trois colonnes distinctes (R-AP5).
    declare_par_agent_id UUID,
    saisie_par_user_id UUID NOT NULL,
    saisie_canal TEXT NOT NULL CHECK (saisie_canal IN (
        'agent', 'collegue', 'accueil', 'import', 'reprise', 'systeme')),
    traitement TEXT CHECK (traitement IS NULL OR traitement IN (
        'sans_incidence', 'rattrapage_interne', 'transmission_rectorat', 'service_non_fait',
        'rattrapage', 'retenue_salaire', 'maintien_salaire', 'conge_paye')),
    preference_traitement TEXT CHECK (preference_traitement IS NULL OR preference_traitement IN (
        'sans_incidence', 'rattrapage_interne', 'transmission_rectorat', 'service_non_fait',
        'rattrapage', 'retenue_salaire', 'maintien_salaire', 'conge_paye')),
    -- R-AP46 : un signalement non confirmé n'a aucun effet extérieur.
    confirmee_par TEXT CHECK (confirmee_par IS NULL OR confirmee_par IN ('agent', 'rappel')),
    confirmee_le TIMESTAMPTZ,
    rappel_par_user_id UUID,
    confidentialite_renforcee BOOLEAN NOT NULL DEFAULT false,
    -- Double engagement : une absence par engagement touché, liées entre elles.
    absence_liee_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_absence_dates_chk CHECK (fin > debut),
    CONSTRAINT ent_absp_absence_confirmee_chk CHECK ((confirmee_par IS NULL) = (confirmee_le IS NULL)),
    CONSTRAINT ent_absp_absence_rappel_chk CHECK (confirmee_par IS DISTINCT FROM 'rappel' OR rappel_par_user_id IS NOT NULL),
    CONSTRAINT ent_absp_absence_service_chk CHECK (cause = 'rh' OR (source_socle IS NOT NULL AND source_ref IS NOT NULL)),
    CONSTRAINT ent_absp_absence_liee_chk CHECK (absence_liee_id IS NULL OR absence_liee_id <> id),
    CONSTRAINT ent_absp_absence_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_absence_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_absence_declarant_fk FOREIGN KEY (etablissement_id, declare_par_agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_absence_engagement_fk FOREIGN KEY (etablissement_id, engagement_id)
        REFERENCES ent_agent_engagement (etablissement_id, id),
    CONSTRAINT ent_absp_absence_liee_fk FOREIGN KEY (etablissement_id, absence_liee_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_absence_agent_idx
    ON ent_absp_absence (etablissement_id, agent_id, debut);
CREATE INDEX IF NOT EXISTS ent_absp_absence_etat_idx
    ON ent_absp_absence (etablissement_id, etat, debut);

-- Texte libre : jamais sur une nature sensible, ni sans nature (R-AP1, R-AP2).
CREATE OR REPLACE FUNCTION ent_trg_absp_absence_controle() RETURNS trigger AS $$
DECLARE
    sens BOOLEAN;
    libre BOOLEAN;
BEGIN
    IF NEW.texte_libre IS NOT NULL THEN
        IF NEW.nature_id IS NULL THEN
            RAISE EXCEPTION 'Pas de texte libre sans nature (R-AP1)' USING ERRCODE = '23514';
        END IF;
        SELECT sensible, texte_libre_permis INTO sens, libre FROM ent_absp_nature WHERE id = NEW.nature_id;
        IF sens OR NOT COALESCE(libre, false) THEN
            RAISE EXCEPTION 'Texte libre interdit pour cette nature (R-AP1, R-AP2)' USING ERRCODE = '23514';
        END IF;
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_absence_controle ON ent_absp_absence;
CREATE TRIGGER ent_absp_absence_controle BEFORE INSERT OR UPDATE ON ent_absp_absence
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_absence_controle();

-- 2. Plages (absence discontinue) -----------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_plage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retiree_le TIMESTAMPTZ,
    CONSTRAINT ent_absp_plage_dates_chk CHECK (fin > debut),
    CONSTRAINT ent_absp_plage_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_plage_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    -- Deux plages d'une même absence ne se chevauchent pas (une plage retirée n'est jamais supprimée).
    CONSTRAINT ent_absp_plage_sans_chevauchement EXCLUDE USING gist (
        absence_id WITH =, tstzrange(debut, fin) WITH &&) WHERE (retiree_le IS NULL)
);
CREATE INDEX IF NOT EXISTS ent_absp_plage_absence_idx ON ent_absp_plage (etablissement_id, absence_id);

-- 3. Circuit figé -----------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_circuit (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('valideur', 'instructeur', 'informe')),
    agent_id UUID NOT NULL,
    rang SMALLINT NOT NULL DEFAULT 1 CHECK (rang BETWEEN 1 AND 5),
    source TEXT NOT NULL CHECK (source IN ('regle', 'responsable_fonctionnel', 'defaut', 'delegation')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_circuit_uq UNIQUE (absence_id, role, agent_id),
    CONSTRAINT ent_absp_circuit_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_circuit_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_circuit_agent_idx
    ON ent_absp_circuit (etablissement_id, agent_id, role);

-- Le valideur n'est jamais l'absent (R-AP10) — aussi pour le circuit figé.
CREATE OR REPLACE FUNCTION ent_trg_absp_circuit_valideur() RETURNS trigger AS $$
DECLARE
    absent UUID;
BEGIN
    IF NEW.role = 'valideur' THEN
        SELECT agent_id INTO absent FROM ent_absp_absence
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.absence_id;
        IF absent = NEW.agent_id THEN
            RAISE EXCEPTION 'Le valideur d''une absence ne peut pas être l''absent (R-AP10)' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_circuit_valideur ON ent_absp_circuit;
CREATE TRIGGER ent_absp_circuit_valideur BEFORE INSERT OR UPDATE ON ent_absp_circuit
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_circuit_valideur();

-- 4. Décisions (ajout seul) ---------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'declaration', 'signalement', 'confirmation', 'validation', 'prise_en_compte', 'refus',
        'requalification', 'prolongation', 'ecourtement', 'reouverture', 'annulation', 'traitement',
        'cloture', 'transmission', 'instruction', 'plage_ajoutee', 'plage_retiree')),
    auteur_agent_id UUID,
    auteur_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    etat_avant TEXT,
    etat_apres TEXT,
    nature_avant_id UUID REFERENCES ent_absp_nature (id),
    nature_apres_id UUID REFERENCES ent_absp_nature (id),
    traitement_avant TEXT,
    traitement_apres TEXT,
    fin_avant TIMESTAMPTZ,
    fin_apres TIMESTAMPTZ,
    raison_code TEXT CHECK (raison_code IS NULL OR raison_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    note VARCHAR(1000),
    voie_transmission TEXT CHECK (voie_transmission IS NULL OR voie_transmission IN (
        'rectorat', 'dsden', 'paie', 'comptabilite', 'autre')),
    CONSTRAINT ent_absp_decision_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_decision_auteur_fk FOREIGN KEY (etablissement_id, auteur_agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_decision_absence_idx
    ON ent_absp_decision (etablissement_id, absence_id, le);

-- Ajout seul + « personne ne valide ni ne refuse sa propre absence » (R-AP10, en base).
CREATE OR REPLACE FUNCTION ent_trg_absp_decision() RETURNS trigger AS $$
DECLARE
    absent UUID;
    absent_user UUID;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
    END IF;
    IF NEW.type IN ('validation', 'prise_en_compte', 'refus') THEN
        SELECT a.agent_id, p.user_id INTO absent, absent_user
          FROM ent_absp_absence a
          LEFT JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
          LEFT JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
         WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.absence_id;
        IF (NEW.auteur_agent_id IS NOT NULL AND NEW.auteur_agent_id = absent)
           OR (absent_user IS NOT NULL AND NEW.auteur_user_id = absent_user) THEN
            RAISE EXCEPTION 'Personne ne valide ni ne refuse sa propre absence (R-AP10)' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_decision_garde ON ent_absp_decision;
CREATE TRIGGER ent_absp_decision_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_absp_decision
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_decision();

-- 5. Journal du domaine (ajout seul) ----------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (objet IN ('absence', 'circuit_regle', 'param', 'nature_etab')),
    objet_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (action ~ '^[a-z][a-z0-9_]{1,59}$'),
    auteur_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    colonne TEXT CHECK (colonne IS NULL OR colonne ~ '^[a-z][a-z0-9_]{1,59}$'),
    -- Codes seulement : jamais un texte libre ni un motif ; la nature n'y figure que par son code et la
    -- table n'est lisible que du cercle habilité (R-AP2).
    valeur_avant VARCHAR(120),
    valeur_apres VARCHAR(120)
);
CREATE INDEX IF NOT EXISTS ent_absp_journal_objet_idx
    ON ent_absp_journal (etablissement_id, objet, objet_id, le);

CREATE OR REPLACE FUNCTION ent_trg_absp_journal_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_journal_ajout_seul ON ent_absp_journal;
CREATE TRIGGER ent_absp_journal_ajout_seul BEFORE UPDATE OR DELETE ON ent_absp_journal
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_journal_ajout_seul();

-- 6. Projection de transition vers `ent_absence_rh` ------------------------------------------------------
-- Tant que l'existant (remplacements, tableau de bord) lit `ent_absence_rh` (retrait au lot A14). Même
-- identifiant : le lien absence → séance (`ent_edt_exception.absence_rh_id`) reste valable. Le motif
-- projeté est TOUJOURS « Absence » : la nature ne sort jamais de `ent_absp_absence`.
CREATE OR REPLACE FUNCTION ent_trg_absp_projection() RETURNS trigger AS $$
DECLARE
    u UUID;
    fus TEXT;
    d1 DATE;
    d2 DATE;
    st TEXT;
    conf BOOLEAN;
    vpar UUID;
    vle TIMESTAMPTZ;
    rpar UUID;
    rle TIMESTAMPTZ;
BEGIN
    IF NEW.cause <> 'rh' THEN
        RETURN NEW;
    END IF;
    -- Un agent sans compte ne peut être ni absent ni remplaçant dans l'existant (AR-19) : pas de ligne.
    SELECT p.user_id INTO u
      FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
     WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.agent_id;
    IF u IS NULL THEN
        RETURN NEW;
    END IF;
    fus := COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = NEW.etablissement_id),
                    'Europe/Paris');
    d1 := (NEW.debut AT TIME ZONE fus)::date;
    d2 := ((NEW.fin - interval '1 microsecond') AT TIME ZONE fus)::date;
    st := CASE NEW.etat
        WHEN 'signalee' THEN 'signalee'
        WHEN 'declaree' THEN 'signalee'
        WHEN 'en_instruction' THEN 'signalee'
        WHEN 'refusee' THEN 'refusee'
        WHEN 'annulee' THEN 'annulee'
        ELSE 'validee' END;
    -- `declaree` et `en_instruction` ont été confirmées par l'agent ; `signalee` seulement si confirmée.
    conf := NEW.etat <> 'signalee' OR NEW.confirmee_le IS NOT NULL;
    IF st = 'validee' THEN
        SELECT auteur_user_id, le INTO vpar, vle FROM ent_absp_decision
         WHERE etablissement_id = NEW.etablissement_id AND absence_id = NEW.id
           AND type IN ('validation', 'prise_en_compte') ORDER BY le DESC LIMIT 1;
    END IF;
    IF st = 'refusee' THEN
        SELECT auteur_user_id, le INTO rpar, rle FROM ent_absp_decision
         WHERE etablissement_id = NEW.etablissement_id AND absence_id = NEW.id AND type = 'refus'
         ORDER BY le DESC LIMIT 1;
    END IF;
    INSERT INTO ent_absence_rh (id, etablissement_id, user_id, agent_id, debut, fin, motif, statut,
                                created_by_user_id, declare_par_user_id, created_at, confirmee,
                                valide_par_user_id, valide_le, refuse_par_user_id, refuse_le)
    VALUES (NEW.id, NEW.etablissement_id, u, NEW.agent_id, d1, d2, 'Absence', st,
            NEW.saisie_par_user_id, NEW.saisie_par_user_id, NEW.cree_le, conf, vpar, vle, rpar, rle)
    ON CONFLICT (id) DO UPDATE SET
        user_id = EXCLUDED.user_id, agent_id = EXCLUDED.agent_id, debut = EXCLUDED.debut,
        fin = EXCLUDED.fin, motif = 'Absence', statut = EXCLUDED.statut, confirmee = EXCLUDED.confirmee,
        valide_par_user_id = COALESCE(EXCLUDED.valide_par_user_id, ent_absence_rh.valide_par_user_id),
        valide_le = COALESCE(EXCLUDED.valide_le, ent_absence_rh.valide_le),
        refuse_par_user_id = COALESCE(EXCLUDED.refuse_par_user_id, ent_absence_rh.refuse_par_user_id),
        refuse_le = COALESCE(EXCLUDED.refuse_le, ent_absence_rh.refuse_le),
        annule_le = CASE WHEN EXCLUDED.statut = 'annulee' THEN COALESCE(ent_absence_rh.annule_le, now()) END,
        annule_par_user_id = CASE WHEN EXCLUDED.statut = 'annulee' THEN COALESCE(ent_absence_rh.annule_par_user_id, NEW.saisie_par_user_id) END;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_projection ON ent_absp_absence;
CREATE TRIGGER ent_absp_projection AFTER INSERT OR UPDATE ON ent_absp_absence
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_projection();

-- 7. Reprise de l'existant (`ent_absence_rh` → `ent_absp_absence`) ------------------------------------------
-- Nature « inconnue » ; l'ancien motif libre n'est PAS repris en clair dans l'absence : il est copié en
-- zone de qualification (réservée à la direction, vidée à la qualification, Q-AP15) puis effacé de la
-- projection historique (qui ne porte plus que « Absence »). Une ligne dont l'agent est inconnu (compte
-- sans agent) n'est pas reprise : elle reste lisible, masquée, dans `ent_absence_rh`.
INSERT INTO ent_absp_absence
    (id, etablissement_id, agent_id, nature_id, zone_qualification, etat, debut, fin, saisie_par_user_id,
     saisie_canal, confirmee_par, confirmee_le, cree_le)
SELECT r.id, r.etablissement_id, COALESCE(r.agent_id, ent_agent_du_compte(r.etablissement_id, r.user_id)),
       CASE WHEN r.statut = 'signalee' AND r.created_by_user_id <> r.user_id THEN NULL
            ELSE (SELECT n.id FROM ent_absp_nature n WHERE n.etablissement_id IS NULL AND n.code = 'inconnue') END,
       CASE WHEN btrim(r.motif) NOT IN ('', 'Absence') THEN left(btrim(r.motif), 200) END,
       CASE r.statut WHEN 'validee' THEN 'validee' WHEN 'refusee' THEN 'refusee' WHEN 'annulee' THEN 'annulee'
            WHEN 'signalee' THEN CASE WHEN r.created_by_user_id = r.user_id THEN 'declaree' ELSE 'signalee' END
            ELSE 'declaree' END,
       (r.debut::timestamp AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = r.etablissement_id), 'Europe/Paris')),
       ((r.fin + 1)::timestamp AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = r.etablissement_id), 'Europe/Paris')),
       COALESCE(r.declare_par_user_id, r.created_by_user_id), 'reprise',
       CASE WHEN r.created_by_user_id = r.user_id OR r.statut = 'validee' THEN 'agent' END,
       CASE WHEN r.created_by_user_id = r.user_id OR r.statut = 'validee' THEN r.created_at END,
       r.created_at
  FROM ent_absence_rh r
 WHERE COALESCE(r.agent_id, ent_agent_du_compte(r.etablissement_id, r.user_id)) IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM ent_absp_absence a WHERE a.id = r.id)
ON CONFLICT DO NOTHING;

-- Plage unique des absences reprises.
INSERT INTO ent_absp_plage (id, etablissement_id, absence_id, debut, fin)
SELECT gen_random_uuid(), a.etablissement_id, a.id, a.debut, a.fin
  FROM ent_absp_absence a
 WHERE NOT EXISTS (SELECT 1 FROM ent_absp_plage p WHERE p.absence_id = a.id);

-- Efface l'ancien motif libre de la projection historique pour les lignes reprises.
UPDATE ent_absence_rh r SET motif = 'Absence'
 WHERE r.motif <> 'Absence' AND EXISTS (SELECT 1 FROM ent_absp_absence a WHERE a.id = r.id);

SELECT ent_rls_poser(ARRAY['ent_absp_absence', 'ent_absp_plage', 'ent_absp_circuit', 'ent_absp_decision', 'ent_absp_journal']);
