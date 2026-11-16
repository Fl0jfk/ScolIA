-- Messagerie, lot M3 (`docs/socles/16_MESSAGERIE.md` §11.1, §19.3, R-M11, R-M26, R-M41, R-M45) : boîtes de
-- service, membres par mission, horaires, astreinte, désignations datées (boîtes C3). Plage 480–499.
--
-- Pas de JSON ni de tableau (D-036) : horaires, missions, astreintes, désignations sont des tables.
-- Pas de `current_date` : le jour d'établissement est `ent_aujourdhui_etab`.
-- Une boîte n'a **jamais** de liste de membres saisie à la main : les membres sont les agents porteurs
-- d'une mission rattachée à la boîte, à la date, plus les désignations datées (remplaçante d'une boîte
-- C3, MSG-175). Rejouable.

-- 1. Boîtes -----------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_boite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    -- Boîte du catalogue fourni (§11.1) ou boîte personnalisée de l'établissement (D-022).
    catalogue BOOLEAN NOT NULL DEFAULT true,
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 80),
    sensibilite TEXT NOT NULL DEFAULT 'C2' CHECK (sensibilite IN ('C2', 'C3')),
    actif BOOLEAN NOT NULL DEFAULT true,
    message_accueil TEXT CHECK (message_accueil IS NULL OR char_length(message_accueil) <= 1000),
    reponse_auto TEXT CHECK (reponse_auto IS NULL OR char_length(reponse_auto) <= 1000),
    delai_vise_jours SMALLINT NOT NULL DEFAULT 2 CHECK (delai_vise_jours BETWEEN 1 AND 30),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_boite_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_msg_boite_etab_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_msg_boite_catalogue_chk CHECK (NOT catalogue OR code IN
        ('vie_scolaire', 'secretariat', 'direction', 'infirmerie', 'psychologue', 'assistante_sociale',
         'intendance', 'orientation', 'cdi', 'stages', 'harcelement')),
    -- R-M11 : infirmerie, psychologue, assistante sociale sont toujours C3, jamais déclassables.
    CONSTRAINT ent_msg_boite_c3_chk CHECK (
        code NOT IN ('infirmerie', 'psychologue', 'assistante_sociale') OR sensibilite = 'C3')
);

-- 2. Membres : missions rattachées (socle 6), jamais une liste figée ---------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_boite_mission (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    boite_id UUID NOT NULL,
    mission_type_id UUID NOT NULL REFERENCES ent_mission_type (id),
    PRIMARY KEY (boite_id, mission_type_id),
    CONSTRAINT ent_msg_boite_mission_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_boite_mission_type_idx ON ent_msg_boite_mission (etablissement_id, mission_type_id);

-- Un type de mission d'un autre établissement ne peut pas ouvrir une boîte (le type fourni, sans
-- établissement, est commun).
CREATE OR REPLACE FUNCTION ent_msg_boite_mission_etab() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM ent_mission_type t
                WHERE t.id = NEW.mission_type_id
                  AND t.etablissement_id IS NOT NULL AND t.etablissement_id <> NEW.etablissement_id) THEN
        RAISE EXCEPTION 'type de mission d''un autre établissement' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_msg_boite_mission_etab_trg ON ent_msg_boite_mission;
CREATE TRIGGER ent_msg_boite_mission_etab_trg BEFORE INSERT OR UPDATE ON ent_msg_boite_mission
    FOR EACH ROW EXECUTE FUNCTION ent_msg_boite_mission_etab();

-- 3. Horaires (jour ISO 1 = lundi … 7 = dimanche), heure locale de l'établissement -------------------
CREATE TABLE IF NOT EXISTS ent_msg_boite_horaire (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    boite_id UUID NOT NULL,
    jour_iso SMALLINT NOT NULL CHECK (jour_iso BETWEEN 1 AND 7),
    debut TIME NOT NULL,
    fin TIME NOT NULL,
    PRIMARY KEY (boite_id, jour_iso, debut),
    CONSTRAINT ent_msg_boite_horaire_sens_chk CHECK (fin > debut),
    CONSTRAINT ent_msg_boite_horaire_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id)
);

-- 4. Désignations datées (MSG-175 : remplaçante d'une boîte C3 ; membre ajouté à une date) ------------
CREATE TABLE IF NOT EXISTS ent_msg_boite_designation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    boite_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    qualite TEXT NOT NULL CHECK (qualite IN ('membre', 'remplacante')),
    du DATE NOT NULL,
    au DATE,
    designe_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_boite_designation_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_msg_boite_designation_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_boite_designation_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_boite_designation_idx
    ON ent_msg_boite_designation (etablissement_id, boite_id, du);

-- 5. Astreinte (messages marqués urgents, CM-44) ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_astreinte (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    boite_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    du TIMESTAMPTZ NOT NULL,
    au TIMESTAMPTZ NOT NULL,
    designe_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_astreinte_sens_chk CHECK (au > du),
    CONSTRAINT ent_msg_astreinte_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_astreinte_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_astreinte_idx ON ent_msg_astreinte (etablissement_id, boite_id, du);

-- 6. Membres d'une boîte à la date : l'unique définition (R-M1, R-M11) -------------------------------------
-- Rend (personne, compte, source) : `mission`, `membre` ou `remplacante`. Une boîte inactive n'a aucun
-- membre. Jour d'établissement : `ent_aujourdhui_etab`.
CREATE OR REPLACE FUNCTION ent_msg_boite_membres(p_etab UUID, p_boite UUID)
RETURNS TABLE (personne_id UUID, user_id UUID, source TEXT)
LANGUAGE sql STABLE AS $$
    SELECT pe.id, pe.user_id, 'mission'::text
      FROM ent_msg_boite b
      JOIN ent_msg_boite_mission bm
        ON bm.etablissement_id = b.etablissement_id AND bm.boite_id = b.id
      JOIN ent_mission m
        ON m.etablissement_id = b.etablissement_id AND m.type_id = bm.mission_type_id
      JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
      JOIN ent_personne pe ON pe.etablissement_id = a.etablissement_id AND pe.id = a.personne_id
     WHERE b.etablissement_id = p_etab AND b.id = p_boite AND b.actif
       AND pe.user_id IS NOT NULL AND pe.fusionnee_dans IS NULL
       AND m.annule_le IS NULL
       AND m.du <= ent_aujourdhui_etab(p_etab)
       AND (m.au IS NULL OR m.au >= ent_aujourdhui_etab(p_etab))
    UNION
    SELECT pe.id, pe.user_id, d.qualite
      FROM ent_msg_boite b
      JOIN ent_msg_boite_designation d
        ON d.etablissement_id = b.etablissement_id AND d.boite_id = b.id
      JOIN ent_personne pe ON pe.etablissement_id = d.etablissement_id AND pe.id = d.personne_id
     WHERE b.etablissement_id = p_etab AND b.id = p_boite AND b.actif
       AND pe.user_id IS NOT NULL AND pe.fusionnee_dans IS NULL
       AND d.du <= ent_aujourdhui_etab(p_etab)
       AND (d.au IS NULL OR d.au >= ent_aujourdhui_etab(p_etab))
$$;

-- 7. Participation d'une boîte dans un fil -----------------------------------------------------------------
ALTER TABLE ent_msg_participant ADD COLUMN IF NOT EXISTS boite_id UUID;
ALTER TABLE ent_msg_participant DROP CONSTRAINT IF EXISTS ent_msg_participant_un_seul_chk;
ALTER TABLE ent_msg_participant DROP CONSTRAINT IF EXISTS ent_msg_participant_cote_chk;
ALTER TABLE ent_msg_participant ADD CONSTRAINT ent_msg_participant_un_seul_chk
    CHECK (num_nonnulls(personne_id, eleve_id, boite_id) = 1);
ALTER TABLE ent_msg_participant ADD CONSTRAINT ent_msg_participant_cote_chk
    CHECK ((cote = 'eleve') = (eleve_id IS NOT NULL) AND (boite_id IS NULL OR cote = 'etablissement'));
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_participant_boite_fk') THEN
        ALTER TABLE ent_msg_participant ADD CONSTRAINT ent_msg_participant_boite_fk
            FOREIGN KEY (etablissement_id, boite_id) REFERENCES ent_msg_boite (etablissement_id, id);
    END IF;
END $$;
CREATE UNIQUE INDEX IF NOT EXISTS ent_msg_participant_boite_uq
    ON ent_msg_participant (fil_id, boite_id) WHERE au IS NULL AND boite_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_msg_participant_boite_idx
    ON ent_msg_participant (etablissement_id, boite_id) WHERE au IS NULL AND boite_id IS NOT NULL;

-- Messages : au nom d'une boîte, expéditeur présumé et canal d'un message entrant (R-M41).
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS au_nom_de_boite_id UUID;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS expediteur_presume_personne_id UUID;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS canal_entrant TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_canal_entrant_chk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_canal_entrant_chk
            CHECK (canal_entrant IS NULL OR canal_entrant IN ('sms', 'courriel', 'telephone'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_au_nom_boite_fk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_au_nom_boite_fk
            FOREIGN KEY (etablissement_id, au_nom_de_boite_id) REFERENCES ent_msg_boite (etablissement_id, id);
    END IF;
END $$;

-- Lecture : un membre actif d'une boîte participante lit le fil (R-M1 : lire = participer). Copie de
-- `ent_msg_fil_lisible` (482) avec une branche de plus : la participation d'une boîte, relue à chaque
-- lecture (un agent qui quitte la mission quitte la boîte, sans ligne à fermer à la main).
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
    ), mes_boites AS (
        SELECT b.id FROM ent_msg_boite b
         WHERE b.etablissement_id = p_etab AND b.actif
           AND EXISTS (SELECT 1 FROM ent_msg_boite_membres(p_etab, b.id) bm WHERE bm.user_id = p_user)
    )
    SELECT f.id
      FROM ent_msg_fil f
     WHERE f.etablissement_id = p_etab
       AND EXISTS (
            SELECT 1 FROM ent_msg_participant p
             WHERE p.etablissement_id = f.etablissement_id AND p.fil_id = f.id
               AND p.au IS NULL AND p.suspendu_le IS NULL
               AND (p.personne_id IN (SELECT id FROM moi)
                    OR p.eleve_id IN (SELECT eleve_id FROM mon_eleve)
                    OR p.boite_id IN (SELECT id FROM mes_boites))
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

-- Un membre d'une boîte qui répond n'y devient pas participant nominatif : il resterait lecteur du
-- fil en quittant la mission. `ent_msg_ajouter_participant` (482) rend faux pour lui.
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
    IF EXISTS (SELECT 1 FROM ent_msg_participant bp
                WHERE bp.etablissement_id = f.etablissement_id AND bp.fil_id = f.id
                  AND bp.boite_id IS NOT NULL AND bp.au IS NULL
                  AND EXISTS (SELECT 1 FROM ent_msg_boite_membres(f.etablissement_id, bp.boite_id) bm
                               WHERE bm.user_id = p_user)) THEN
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

-- 8. RLS permissive des nouvelles tables (même politique que 480 et 482) ------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_msg_boite', 'ent_msg_boite_mission', 'ent_msg_boite_horaire',
                           'ent_msg_boite_designation', 'ent_msg_astreinte'];
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
