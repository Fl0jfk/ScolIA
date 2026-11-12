-- Messagerie, lot M1 (`docs/socles/16_MESSAGERIE.md` §25, §26, R-M1, R-M2, R-M14) : participants
-- et contrôle d'accès. Plage de repli 480–499 (Q-B2 non tranchée) ; 480 = M0a, 481 = M0b,
-- 482 (ce fichier) = tables et fonctions, 483 = reprise des fils existants (MIG-1 à MIG-4),
-- 484 = matrice et réglages (M2). Aucune migration existante n'est modifiée (PILOTAGE règle 15).
--
-- Principe (R-M1) : **lire = participer**. Un compte lit un fil s'il en est participant actif ;
-- la fonction `ent_msg_fil_lisible(établissement, compte)` est la seule vue de lecture des
-- participants (R-M2). Les colonnes héritées (`prive_user_id`, `foyer_id`, `foyer_lecteur_id`,
-- `audience`, `vie_scolaire`, `eleve_id`) restent en sursis, lues seulement pour peupler les
-- participants à la création et à la reprise ; elles ne décident plus de la lecture d'une famille.
--
-- Écart assumé au §25 : un participant est une **personne** (`personne_id`) ou un **élève**
-- (`eleve_id`), car un élève mineur n'a pas toujours de fiche `ent_personne` ; la boîte de
-- service (`boite_id`) est ajoutée par M3 avec la table des boîtes.
--
-- D-036 : colonnes typées, aucun JSON ni tableau. E1–E14 : `etablissement_id NOT NULL` ≠ 0, clés
-- composites `(etablissement_id, id)`, UUID fournis par l'application, pas de BYTEA, horodatages,
-- migration rejouable, RLS à la création (politique inline : `ent_rls_poser` n'existe qu'en 580).

-- 1. Fil enrichi (§25) ---------------------------------------------------------------------------
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS nature TEXT NOT NULL DEFAULT 'famille';
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS sensibilite TEXT NOT NULL DEFAULT 'C2';
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'ouvert';
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS origine TEXT NOT NULL DEFAULT 'personnel';
-- Fiche personne de l'ouvreur (peut manquer : compte sans fiche) ; `created_by_user_id` reste.
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS ouvert_par_personne_id UUID;
-- Fils jumeaux (R-M3) : le second fil d'un même envoi pointe le premier (même texte, un par foyer).
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS jumeau_de UUID;
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS clos_le TIMESTAMPTZ;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_nature_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_nature_chk CHECK (nature IN
            ('famille', 'eleve', 'interne', 'equipe', 'carnet', 'boite', 'systeme', 'classe', 'delegues'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_sensibilite_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_sensibilite_chk
            CHECK (sensibilite IN ('C1', 'C2', 'C3'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_etat_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_etat_chk
            CHECK (etat IN ('ouvert', 'clos', 'gele'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_origine_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_origine_chk CHECK (origine IN
            ('personnel', 'famille', 'eleve', 'systeme', 'telephone', 'sms_entrant', 'courriel_entrant'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_jumeau_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_jumeau_chk
            CHECK (jumeau_de IS NULL OR jumeau_de <> id);
    END IF;
END $$;

-- Clés composites (D19) : cibles des clés étrangères de la brique.
CREATE UNIQUE INDEX IF NOT EXISTS ent_msg_fil_etab_id_uq ON ent_msg_fil (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_msg_etab_id_uq ON ent_msg (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_etab_id_uq ON ent_eleve (etablissement_id, id);

-- 2. Messages enrichis (§25) ---------------------------------------------------------------------
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS type TEXT NOT NULL DEFAULT 'texte';
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS reponse_a UUID;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS auteur_personne_id UUID;
-- Faux : SMS ou e-mail entrant (M6) ; vrai pour tout message écrit dans l'ENT.
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS verifie BOOLEAN NOT NULL DEFAULT true;
-- Copie d'un message lors d'un dédoublement de fil (MIG-3, MSG-118) : le message d'origine.
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS copie_de UUID;
-- SHA-256 du corps : preuve d'intégrité (posée par déclencheur, y compris pour les anciennes lignes).
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS empreinte TEXT;

CREATE OR REPLACE FUNCTION ent_msg_empreinte_fn() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.empreinte := encode(sha256(convert_to(NEW.corps, 'UTF8')), 'hex');
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_msg_empreinte ON ent_msg;
CREATE TRIGGER ent_msg_empreinte BEFORE INSERT ON ent_msg
    FOR EACH ROW EXECUTE FUNCTION ent_msg_empreinte_fn();
UPDATE ent_msg SET empreinte = encode(sha256(convert_to(corps, 'UTF8')), 'hex') WHERE empreinte IS NULL;
ALTER TABLE ent_msg ALTER COLUMN empreinte SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_type_chk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_type_chk CHECK (type IN
            ('texte', 'note_interne', 'systeme', 'mot', 'carte', 'telephone'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_version_chk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_version_chk CHECK (version >= 1);
    END IF;
END $$;

-- Reprise de l'auteur : la fiche personne du compte, quand elle existe.
UPDATE ent_msg m SET auteur_personne_id = p.id
  FROM ent_personne p
 WHERE m.auteur_personne_id IS NULL AND p.etablissement_id = m.etablissement_id
   AND p.user_id = m.auteur_user_id AND p.fusionnee_dans IS NULL;

-- Versions successives d'un message corrigé (M4) : ajout seul.
CREATE TABLE IF NOT EXISTS ent_msg_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    message_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    corps TEXT NOT NULL CHECK (length(corps) <= 20000),
    remplace_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    par_user_id UUID,
    CONSTRAINT ent_msg_version_uq UNIQUE (etablissement_id, message_id, version),
    CONSTRAINT ent_msg_version_msg_fk FOREIGN KEY (etablissement_id, message_id)
        REFERENCES ent_msg (etablissement_id, id)
);

CREATE OR REPLACE FUNCTION ent_msg_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_msg_version_ajout_seul ON ent_msg_version;
CREATE TRIGGER ent_msg_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_msg_version
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- 3. « À propos de » : 0..n élèves par fil (CM-07) -----------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_fil_eleve (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (fil_id, eleve_id),
    CONSTRAINT ent_msg_fil_eleve_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_fil_eleve_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_fil_eleve_eleve_idx ON ent_msg_fil_eleve (etablissement_id, eleve_id);

-- 4. Participants (R-M1) -------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_participant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    -- Exactement un des deux : une personne (avec ou sans compte) ou un élève.
    personne_id UUID,
    eleve_id UUID,
    cote TEXT NOT NULL CHECK (cote IN ('etablissement', 'famille', 'eleve')),
    qualite TEXT NOT NULL CHECK (qualite IN
        ('auteur', 'destinataire', 'equipe', 'suppleant', 'ajoute', 'successeur')),
    -- Pourquoi cette personne est dans le fil (code, jamais de texte libre).
    raison TEXT NOT NULL CHECK (raison IN
        ('titulaire_foyer', 'payeur', 'eleve', 'professeur_cours', 'pp', 'mission', 'boite', 'equipe',
         'auteur', 'a_ecrit', 'vie_scolaire', 'reprise')),
    peut_ecrire BOOLEAN NOT NULL DEFAULT true,
    du TIMESTAMPTZ NOT NULL DEFAULT now(),
    au TIMESTAMPTZ,
    suspendu_le TIMESTAMPTZ,
    -- Codes neutres : jamais le motif en clair (une interdiction ne se devine pas).
    motif_sortie TEXT CHECK (motif_sortie IS NULL OR motif_sortie IN
        ('interdiction', 'lien_clos', 'deces', 'depart', 'sortie_volontaire', 'retire_par_direction',
         'separation', 'majorite', 'fin_de_cours', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_participant_un_seul_chk CHECK ((personne_id IS NULL) <> (eleve_id IS NULL)),
    CONSTRAINT ent_msg_participant_cote_chk CHECK (
        (cote = 'eleve') = (eleve_id IS NOT NULL)),
    CONSTRAINT ent_msg_participant_sortie_chk CHECK (au IS NULL OR motif_sortie IS NOT NULL),
    CONSTRAINT ent_msg_participant_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_msg_participant_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id)
);
-- Un seul participant actif par personne (ou élève) et par fil.
CREATE UNIQUE INDEX IF NOT EXISTS ent_msg_participant_personne_uq
    ON ent_msg_participant (fil_id, personne_id) WHERE au IS NULL AND personne_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_msg_participant_eleve_uq
    ON ent_msg_participant (fil_id, eleve_id) WHERE au IS NULL AND eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_msg_participant_personne_idx
    ON ent_msg_participant (etablissement_id, personne_id) WHERE au IS NULL AND personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_msg_participant_eleve_idx
    ON ent_msg_participant (etablissement_id, eleve_id) WHERE au IS NULL AND eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_msg_participant_fil_idx ON ent_msg_participant (etablissement_id, fil_id);

-- Rapport de reprise (MIG-1 à MIG-4) : un fil dédoublé ou à vérifier laisse une trace pour la
-- direction. Ajout seul ; aucun contenu de message, que des identifiants et un code.
CREATE TABLE IF NOT EXISTS ent_msg_reprise_rapport (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    fil_jumeau_id UUID,
    regle TEXT NOT NULL CHECK (regle IN
        ('MIG-1', 'MIG-2', 'MIG-3', 'MIG-4', 'VIE_SCOLAIRE', 'SEPARATION')),
    effet TEXT NOT NULL CHECK (effet IN
        ('dedouble_deux_foyers', 'un_fil_par_foyer', 'foyers_dans_un_fil', 'sans_participant_famille',
         'separation_demandee')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_reprise_rapport_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_msg_reprise_rapport_ajout_seul ON ent_msg_reprise_rapport;
CREATE TRIGGER ent_msg_reprise_rapport_ajout_seul BEFORE UPDATE OR DELETE ON ent_msg_reprise_rapport
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- 5. RLS permissive des nouvelles tables (même politique que 480 ; la stricte vient avec 600) ----
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_msg_version', 'ent_msg_fil_eleve', 'ent_msg_participant',
                           'ent_msg_reprise_rapport'];
BEGIN
    FOREACH t IN ARRAY tables LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
            'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
            'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
            t);
    END LOOP;
END
$$;

-- 6. Fiche personne d'un compte (créée au besoin) -------------------------------------------------
-- Un personnel qui écrit dans un fil y devient participant : il lui faut une fiche personne.
-- Les agents en ont déjà une (352) ; un compte sans fiche en reçoit une, source « compte ».
CREATE OR REPLACE FUNCTION ent_msg_personne_du_compte(p_etab UUID, p_user UUID) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE
    v_id UUID;
    v_nom TEXT;
BEGIN
    SELECT id INTO v_id FROM ent_personne
     WHERE etablissement_id = p_etab AND user_id = p_user AND fusionnee_dans IS NULL;
    IF v_id IS NOT NULL THEN
        RETURN v_id;
    END IF;
    SELECT COALESCE(NULLIF(btrim(display_name), ''), 'Compte') INTO v_nom FROM ent_user WHERE id = p_user;
    IF v_nom IS NULL THEN
        RETURN NULL;
    END IF;
    v_id := gen_random_uuid();
    INSERT INTO ent_personne (id, etablissement_id, nom, user_id, source)
    VALUES (v_id, p_etab, left(v_nom, 120), p_user, 'compte')
    ON CONFLICT DO NOTHING;
    SELECT id INTO v_id FROM ent_personne
     WHERE etablissement_id = p_etab AND user_id = p_user AND fusionnee_dans IS NULL;
    RETURN v_id;
END
$$;

-- 7. Participants posés à la création d'un fil -----------------------------------------------------
-- Peuple la participation famille et élève d'un fil **à partir de sa portée** (colonnes héritées,
-- posées à l'insertion) : un seul code pour la création et pour la reprise (483).
--   audience « eleve »   : l'élève ; jamais un parent (M0a D3, D23)
--   prive_user_id        : le responsable de ce compte (fil ouvert par lui, ou à lui seul)
--   foyer_id             : les payeurs du foyer (relance de facture)
--   foyer_lecteur_id     : les responsables communicants de ce foyer (D22, R-M3)
--   vie_scolaire         : les titulaires avec droit « vie scolaire » (M0a D5)
-- Un fil du personnel « seul » (aucun responsable joignable) n'a aucun participant famille.
-- Les liens interdits, suspendus ou échus sont exclus (`ent_eleve_lien_communicant`, R-M6, R-M8).
CREATE OR REPLACE FUNCTION ent_msg_peupler_famille(p_fil UUID) RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE
    f ent_msg_fil%ROWTYPE;
    n INTEGER := 0;
    k INTEGER;
BEGIN
    SELECT * INTO f FROM ent_msg_fil WHERE id = p_fil;
    IF NOT FOUND THEN
        RETURN 0;
    END IF;
    IF f.audience = 'eleve' THEN
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, eleve_id, cote, qualite, raison)
        VALUES (gen_random_uuid(), f.etablissement_id, f.id, f.eleve_id, 'eleve',
                CASE WHEN f.origine = 'eleve' THEN 'auteur' ELSE 'destinataire' END, 'eleve')
        ON CONFLICT DO NOTHING;
        GET DIAGNOSTICS k = ROW_COUNT;
        RETURN k;
    END IF;
    IF f.prive_user_id IS NOT NULL THEN
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
        SELECT gen_random_uuid(), f.etablissement_id, f.id, ent_msg_personne_du_compte(f.etablissement_id, l.user_id),
               'famille', CASE WHEN f.origine = 'famille' THEN 'auteur' ELSE 'destinataire' END,
               'titulaire_foyer'
          FROM ent_eleve_lien_communicant l
         WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
           AND l.user_id = f.prive_user_id
         LIMIT 1
        ON CONFLICT DO NOTHING;
        GET DIAGNOSTICS k = ROW_COUNT;
        RETURN k;
    END IF;
    IF f.foyer_id IS NOT NULL THEN
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
        SELECT gen_random_uuid(), f.etablissement_id, f.id, ent_msg_personne_du_compte(f.etablissement_id, l.user_id),
               'famille', 'destinataire', 'payeur'
          FROM ent_eleve_lien_communicant l
         WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
           AND l.foyer_id = f.foyer_id AND l.est_payeur
        ON CONFLICT DO NOTHING;
        GET DIAGNOSTICS k = ROW_COUNT;
        RETURN k;
    END IF;
    IF f.foyer_lecteur_id IS NOT NULL THEN
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
        SELECT gen_random_uuid(), f.etablissement_id, f.id, ent_msg_personne_du_compte(f.etablissement_id, l.user_id),
               'famille', 'destinataire', 'titulaire_foyer'
          FROM ent_eleve_lien_communicant l
         WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
           AND l.foyer_id = f.foyer_lecteur_id
        ON CONFLICT DO NOTHING;
        GET DIAGNOSTICS k = ROW_COUNT;
        RETURN k;
    END IF;
    IF f.vie_scolaire THEN
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
        SELECT gen_random_uuid(), f.etablissement_id, f.id, ent_msg_personne_du_compte(f.etablissement_id, l.user_id),
               'famille', 'destinataire', 'vie_scolaire'
          FROM ent_eleve_lien_communicant l
         WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = f.eleve_id
           AND l.peut_voir_vs AND l.autorite_parentale
        ON CONFLICT DO NOTHING;
        GET DIAGNOSTICS k = ROW_COUNT;
        RETURN k;
    END IF;
    RETURN n;
END
$$;

-- Ajoute (ou retrouve) le participant d'un compte dans un fil : personnel qui écrit, parent ou
-- élève. Rend vrai si une ligne a été créée. Idempotent (index unique partiel).
CREATE OR REPLACE FUNCTION ent_msg_ajouter_participant(
    p_fil UUID, p_user UUID, p_qualite TEXT, p_raison TEXT) RETURNS BOOLEAN
LANGUAGE plpgsql AS $$
DECLARE
    f ent_msg_fil%ROWTYPE;
    v_role TEXT;
    v_personne UUID;
    v_eleve UUID;
    k INTEGER;
BEGIN
    SELECT * INTO f FROM ent_msg_fil WHERE id = p_fil;
    IF NOT FOUND THEN
        RETURN false;
    END IF;
    SELECT role INTO v_role FROM ent_membership
     WHERE user_id = p_user AND etablissement_id = f.etablissement_id;
    IF v_role = 'eleve' THEN
        SELECT eleve_id INTO v_eleve FROM ent_eleve_compte
         WHERE etablissement_id = f.etablissement_id AND user_id = p_user;
        IF v_eleve IS NULL THEN
            RETURN false;
        END IF;
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, eleve_id, cote, qualite, raison)
        VALUES (gen_random_uuid(), f.etablissement_id, f.id, v_eleve, 'eleve', p_qualite, 'eleve')
        ON CONFLICT DO NOTHING;
    ELSE
        v_personne := ent_msg_personne_du_compte(f.etablissement_id, p_user);
        IF v_personne IS NULL THEN
            RETURN false;
        END IF;
        INSERT INTO ent_msg_participant
            (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
        VALUES (gen_random_uuid(), f.etablissement_id, f.id, v_personne,
                CASE WHEN v_role = 'parent' THEN 'famille' ELSE 'etablissement' END,
                p_qualite,
                CASE WHEN v_role = 'parent' THEN 'titulaire_foyer' ELSE p_raison END)
        ON CONFLICT DO NOTHING;
    END IF;
    GET DIAGNOSTICS k = ROW_COUNT;
    RETURN k > 0;
END
$$;

-- 8. Lecture : la seule vue des fils lisibles par participation (R-M1, R-M2) -----------------------
-- Rend les fils où le compte est participant **actif** (ni sorti, ni suspendu), avec, pour la
-- famille, les conditions relues à chaque lecture (R-M6, R-M8 : lien actif non visé par une
-- interdiction de communication ; vie scolaire : droit et autorité ; relance : payeur) et la
-- remise différée (M0b D24 : un fil dont aucun message n'est remis n'existe pas pour la famille,
-- sauf pour son auteur). Le personnel passe par le même chemin ; le « pont » des rôles réservés
-- (D26) est ajouté côté Rust, signalé provisoire jusqu'à M3.
CREATE OR REPLACE FUNCTION ent_msg_fil_lisible(p_etab UUID, p_user UUID)
RETURNS TABLE (fil_id UUID)
LANGUAGE sql STABLE AS $$
    WITH moi AS (
        SELECT pe.id FROM ent_personne pe
         WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user AND pe.fusionnee_dans IS NULL
        UNION
        SELECT s.id FROM ent_personne s
          JOIN ent_personne pe ON pe.id = s.fusionnee_dans AND pe.etablissement_id = s.etablissement_id
         WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user
    ), mon_eleve AS (
        SELECT c.eleve_id FROM ent_eleve_compte c
         WHERE c.etablissement_id = p_etab AND c.user_id = p_user
    )
    SELECT f.id
      FROM ent_msg_fil f
     WHERE f.etablissement_id = p_etab
       AND EXISTS (
            SELECT 1 FROM ent_msg_participant p
             WHERE p.etablissement_id = f.etablissement_id AND p.fil_id = f.id
               AND p.au IS NULL AND p.suspendu_le IS NULL
               AND (p.personne_id IN (SELECT id FROM moi)
                    OR p.eleve_id IN (SELECT eleve_id FROM mon_eleve))
               AND (p.cote <> 'famille'
                    OR EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                                WHERE l.etablissement_id = f.etablissement_id AND l.user_id = p_user
                                  AND (l.eleve_id = f.eleve_id
                                       OR l.eleve_id IN (SELECT fe.eleve_id FROM ent_msg_fil_eleve fe
                                                          WHERE fe.etablissement_id = f.etablissement_id
                                                            AND fe.fil_id = f.id))
                                  AND (NOT f.vie_scolaire OR (l.peut_voir_vs AND l.autorite_parentale))
                                  AND (f.foyer_id IS NULL
                                       OR (l.foyer_id = f.foyer_id AND l.est_payeur))))
               AND (p.cote = 'etablissement'
                    OR NOT EXISTS (SELECT 1 FROM ent_msg m
                                    WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id)
                    OR EXISTS (SELECT 1 FROM ent_msg m
                                WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id
                                  AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now()
                                       OR m.auteur_user_id = p_user))))
$$;

-- 9. Interdiction, lien suspendu ou clos, décès (CM-05, CM-15, CM-16, R-M6, R-M8) ---------------
-- Aucune clôture par déclencheur : l'exclusion est relue à **chaque lecture** par `ent_msg_fil_lisible`
-- (la famille doit encore avoir un lien communicant, `ent_eleve_lien_communicant`), donc une
-- interdiction pose son effet à l'instant, sa levée le rend, et aucune ligne de participation ne
-- doit être rouverte à la main. Un motif de sortie `interdiction` est réservé à la clôture
-- explicite par la direction (jamais affiché).
