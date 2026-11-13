-- Socle N1 (« Demandes et flux de travail »), lot N1-0 — migration 3040 : cœur du modèle.
-- Réf. : docs/socles/N1_DEMANDES.md §15, §23, §28 (R-DM6, R-DM9, R-DM11, R-DM28, R-DM29), §38 (D-061) ;
-- NJ-82, NJ-84, NJ-89, NJ-90 du registre ALERTES_NEXTJS_PROD.md.
--
-- Contenu : ent_dem_numerotation, ent_dem_demande, ent_dem_valeur (valeurs TYPÉES, D-036),
-- ent_dem_fait (ajout seul, chaîné par empreinte SHA-256 calculée PAR LA BASE), ent_dem_echeance.
-- Additive et rejouable (E8). Aucune donnée semi-structurée, aucun tableau, aucun contenu binaire (D-036, E5).
-- Toutes les clés étrangères sont composites (etablissement_id, …) et en ON DELETE RESTRICT (NJ-89) :
-- rien ne se supprime en cascade. Les références vers la file de service (ent_dem_file, migration 3042 du
-- lot N1-1) sont des UUID sans clé étrangère tant que la table n'existe pas.

-- ───────────────────────── Unicité composite des tables de structure référencées ─────────────────────────
-- Nécessaire aux clés étrangères composites (E2) ; additif, sans effet sur les lignes existantes.
CREATE UNIQUE INDEX IF NOT EXISTS ent_annee_scolaire_etab_id_uq ON ent_annee_scolaire (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_site_etab_id_uq ON ent_site (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_lieu_etab_id_uq ON ent_lieu (etablissement_id, id);

-- ───────────────────────── Garde de purge ─────────────────────────
-- Seule la tâche de purge de l'ordonnanceur (`purger_echues`) pose ce réglage, LOCAL à sa transaction.
-- Hors de ce cas, rien ne s'efface ni ne se réécrit dans les tables de preuve (R-DM28, R-DM29, NJ-89).
CREATE OR REPLACE FUNCTION ent_dem_purge_en_cours() RETURNS boolean
LANGUAGE sql STABLE AS $$
    SELECT COALESCE(current_setting('ent.dem_purge', true), '') = 'on'
$$;

CREATE OR REPLACE FUNCTION ent_dem_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND TG_TABLE_NAME IN ('ent_dem_valeur', 'ent_dem_echeance')
       AND ent_dem_purge_en_cours() THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION '% : suppression interdite (R-DM28, NJ-89) ; la conservation passe par la purge de l''ordonnanceur', TG_TABLE_NAME
        USING ERRCODE = '23001';
END
$$;

-- ───────────────────────── Numérotation lisible par établissement et année ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_numerotation (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    dernier INT NOT NULL DEFAULT 0 CHECK (dernier >= 0),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, annee_id),
    CONSTRAINT ent_dem_numerotation_annee_cfk FOREIGN KEY (etablissement_id, annee_id)
        REFERENCES ent_annee_scolaire (etablissement_id, id) ON DELETE RESTRICT
);

-- Numéro suivant, atomique (un seul UPDATE … ON CONFLICT : deux dépôts simultanés n'obtiennent jamais le même).
CREATE OR REPLACE FUNCTION ent_dem_numero_suivant(p_etab UUID, p_annee UUID) RETURNS INT
LANGUAGE sql AS $$
    INSERT INTO ent_dem_numerotation (etablissement_id, annee_id, dernier)
    VALUES (p_etab, p_annee, 1)
    ON CONFLICT (etablissement_id, annee_id)
    DO UPDATE SET dernier = ent_dem_numerotation.dernier + 1, maj_le = now()
    RETURNING dernier
$$;

-- ───────────────────────── La demande ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    numero TEXT NOT NULL CHECK (numero ~ '^[A-Z0-9-]{4,40}$'),
    annee_id UUID NOT NULL,
    nature_code TEXT NOT NULL CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    nature_version SMALLINT NOT NULL CHECK (nature_version >= 1),
    statut TEXT NOT NULL CHECK (statut IN (
        'brouillon', 'deposee', 'prise', 'en_attente_demandeur', 'en_attente_tiers',
        'en_validation', 'traitee', 'close', 'rouverte', 'annulee')),
    issue TEXT CHECK (issue IS NULL OR issue IN (
        'traitee', 'accordee', 'accordee_partiellement', 'refusee', 'sans_objet',
        'annulee_par_demandeur', 'doublon', 'transmise_hors_scolia', 'close_sans_reponse_du_demandeur')),
    confidentialite TEXT NOT NULL CHECK (confidentialite IN ('C0', 'C1', 'C2', 'C3')),
    urgence TEXT NOT NULL DEFAULT 'normale' CHECK (urgence IN ('normale', 'haute', 'securite')),
    -- Trois personnes, trois colonnes, toutes issues du serveur (§7.1, NJ-84, NJ-91). Aucune adresse
    -- e-mail ni téléphone n'est stocké ici (§24.4) : ils sont lus au socle 4 au moment de l'avis.
    demandeur_personne_id UUID,
    deposant_personne_id UUID,
    canal_depot TEXT NOT NULL CHECK (canal_depot IN (
        'web', 'mobile', 'comptoir', 'telephone', 'courrier', 'courriel_recu', 'conversion_message', 'reprise')),
    beneficiaire_agent_id UUID,
    eleve_id UUID,
    lieu_id UUID,
    site_id UUID,
    objet TEXT CHECK (objet IS NULL OR length(objet) <= 140),
    -- Texte libre : chiffré (champs_chiffres) dès C2 ; `texte_scelle` le dit sans l'ouvrir.
    texte TEXT CHECK (texte IS NULL OR length(texte) <= 20000),
    texte_scelle BOOLEAN NOT NULL DEFAULT false,
    -- File de service (R-DM6 : une demande a toujours une file). Clé étrangère ajoutée avec la table (3042).
    file_id UUID NOT NULL,
    attribuee_a_agent_id UUID,
    attribuee_le TIMESTAMPTZ,
    depose_le TIMESTAMPTZ,
    clos_le TIMESTAMPTZ,
    decision TEXT CHECK (decision IS NULL OR decision IN ('accordee', 'accordee_partiellement', 'refusee')),
    conservation_jusqu_au DATE,
    gel BOOLEAN NOT NULL DEFAULT false,
    purgee_le TIMESTAMPTZ,
    version INT NOT NULL DEFAULT 1 CHECK (version >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_demande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_demande_numero_uq UNIQUE (etablissement_id, numero),
    CONSTRAINT ent_dem_demande_annee_cfk FOREIGN KEY (etablissement_id, annee_id)
        REFERENCES ent_annee_scolaire (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_demandeur_cfk FOREIGN KEY (etablissement_id, demandeur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_deposant_cfk FOREIGN KEY (etablissement_id, deposant_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_beneficiaire_cfk FOREIGN KEY (etablissement_id, beneficiaire_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_lieu_cfk FOREIGN KEY (etablissement_id, lieu_id)
        REFERENCES ent_lieu (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_demande_traitant_cfk FOREIGN KEY (etablissement_id, attribuee_a_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    -- R-DM9 : une seule prise active par demande = UNE colonne ; elle va avec sa date, jamais l'une sans l'autre.
    CONSTRAINT ent_dem_demande_prise_coherente CHECK ((attribuee_a_agent_id IS NULL) = (attribuee_le IS NULL)),
    -- Clôture : date de clôture si et seulement si le statut est final ; une issue accompagne toute clôture.
    CONSTRAINT ent_dem_demande_cloture_coherente CHECK (
        ((statut IN ('close', 'annulee')) = (clos_le IS NOT NULL))
        AND (statut NOT IN ('close', 'annulee') OR issue IS NOT NULL)),
    -- Une nature C2+ n'a jamais de texte en clair (R-DM16, §24).
    CONSTRAINT ent_dem_demande_texte_scelle_chk CHECK (
        confidentialite IN ('C0', 'C1') OR texte IS NULL OR texte_scelle),
    -- Le demandeur n'est effacé que par la purge (anonymisation).
    CONSTRAINT ent_dem_demande_demandeur_chk CHECK (demandeur_personne_id IS NOT NULL OR purgee_le IS NOT NULL),
    -- Le brouillon n'a pas de date de dépôt ; toute demande déposée en a une.
    CONSTRAINT ent_dem_demande_depot_chk CHECK (
        (statut <> 'brouillon' OR depose_le IS NULL)
        AND (statut IN ('brouillon', 'annulee') OR depose_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_dem_demande_demandeur_idx
    ON ent_dem_demande (etablissement_id, demandeur_personne_id, cree_le DESC);
CREATE INDEX IF NOT EXISTS ent_dem_demande_file_idx
    ON ent_dem_demande (etablissement_id, file_id, statut);
CREATE INDEX IF NOT EXISTS ent_dem_demande_traitant_idx
    ON ent_dem_demande (etablissement_id, attribuee_a_agent_id) WHERE attribuee_a_agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_dem_demande_conservation_idx
    ON ent_dem_demande (etablissement_id, conservation_jusqu_au)
    WHERE conservation_jusqu_au IS NOT NULL AND purgee_le IS NULL AND NOT gel;

-- Garde de la ligne : pas de suppression ; version strictement incrémentée (contrôle de version jusque dans la
-- base, R-DM11, NJ-90) ; identité de la demande figée ; ligne purgée figée.
CREATE OR REPLACE FUNCTION ent_dem_demande_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_demande : suppression interdite (R-DM28, NJ-89) ; purge par l''ordonnanceur seulement'
            USING ERRCODE = '23001';
    END IF;
    IF OLD.purgee_le IS NOT NULL THEN
        RAISE EXCEPTION 'ent_dem_demande : demande purgée, plus aucune écriture' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.numero IS DISTINCT FROM OLD.numero
       OR NEW.annee_id IS DISTINCT FROM OLD.annee_id
       OR NEW.nature_code IS DISTINCT FROM OLD.nature_code
       OR NEW.nature_version IS DISTINCT FROM OLD.nature_version
       OR NEW.deposant_personne_id IS DISTINCT FROM OLD.deposant_personne_id
       OR NEW.canal_depot IS DISTINCT FROM OLD.canal_depot
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'ent_dem_demande : identité de la demande figée' USING ERRCODE = '23001';
    END IF;
    IF NEW.purgee_le IS NULL AND NEW.demandeur_personne_id IS DISTINCT FROM OLD.demandeur_personne_id THEN
        RAISE EXCEPTION 'ent_dem_demande : le demandeur ne change jamais (R-DM4)' USING ERRCODE = '23001';
    END IF;
    IF NEW.purgee_le IS NOT NULL AND NOT ent_dem_purge_en_cours() THEN
        RAISE EXCEPTION 'ent_dem_demande : purge réservée à l''ordonnanceur (R-DM29)' USING ERRCODE = '23001';
    END IF;
    IF NEW.version <> OLD.version + 1 THEN
        RAISE EXCEPTION 'ent_dem_demande : version périmée ou non incrémentée (% -> %)', OLD.version, NEW.version
            USING ERRCODE = '40001';
    END IF;
    NEW.maj_le := now();
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_dem_demande_garde_trg ON ent_dem_demande;
CREATE TRIGGER ent_dem_demande_garde_trg
    BEFORE UPDATE OR DELETE ON ent_dem_demande
    FOR EACH ROW EXECUTE FUNCTION ent_dem_demande_garde();

-- ───────────────────────── Valeurs des champs déclarés, TYPÉES (D-036) ─────────────────────────
-- Une ligne par champ et par rang (champ répété). Code de champ déclaré dans le code de la nature et validé
-- à l'écriture ; une colonne par type, exactement une valeur. Pas de sac d'attributs texte (NJ-90).
-- Valeurs figées : une correction crée une nouvelle `valeurs_version` (§15.1), jamais une réécriture.
CREATE TABLE IF NOT EXISTS ent_dem_valeur (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    valeurs_version SMALLINT NOT NULL DEFAULT 1 CHECK (valeurs_version >= 1),
    champ_code TEXT NOT NULL CHECK (champ_code ~ '^[a-z][a-z0-9_]{0,59}$'),
    rang SMALLINT NOT NULL DEFAULT 0 CHECK (rang >= 0),
    valeur_texte TEXT CHECK (valeur_texte IS NULL OR length(valeur_texte) <= 2000),
    valeur_entier BIGINT,
    valeur_decimal NUMERIC(12, 2),
    valeur_date DATE,
    valeur_heure TIME,
    valeur_bool BOOLEAN,
    valeur_code TEXT CHECK (valeur_code IS NULL OR valeur_code ~ '^[a-z0-9_]{1,60}$'),
    valeur_ref UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_valeur_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_valeur_champ_uq UNIQUE (etablissement_id, demande_id, valeurs_version, champ_code, rang),
    CONSTRAINT ent_dem_valeur_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_valeur_une_seule CHECK (num_nonnulls(
        valeur_texte, valeur_entier, valeur_decimal, valeur_date, valeur_heure, valeur_bool, valeur_code, valeur_ref) = 1)
);
CREATE INDEX IF NOT EXISTS ent_dem_valeur_demande_idx ON ent_dem_valeur (etablissement_id, demande_id);

CREATE OR REPLACE FUNCTION ent_dem_valeur_figee() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_dem_valeur : valeurs figées, une correction est une nouvelle version (§15.1)'
        USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_valeur_figee_trg ON ent_dem_valeur;
CREATE TRIGGER ent_dem_valeur_figee_trg
    BEFORE UPDATE ON ent_dem_valeur
    FOR EACH ROW EXECUTE FUNCTION ent_dem_valeur_figee();
DROP TRIGGER IF EXISTS ent_dem_valeur_sans_suppression_trg ON ent_dem_valeur;
CREATE TRIGGER ent_dem_valeur_sans_suppression_trg
    BEFORE DELETE ON ent_dem_valeur
    FOR EACH ROW EXECUTE FUNCTION ent_dem_sans_suppression();

-- ───────────────────────── Faits : ajout seul, chaînés (R-DM28, §15.1) ─────────────────────────
-- Le rang, l'empreinte précédente et l'empreinte (SHA-256) sont calculés par la BASE à l'insertion, sous un
-- verrou par demande : aucun appelant ne peut forger ni sauter un maillon. `verifier_chaine` recalcule.
-- Un fait ne porte que des identifiants et des codes : jamais le texte d'une demande ni une adresse.
CREATE TABLE IF NOT EXISTS ent_dem_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    rang INT NOT NULL DEFAULT 0,
    type TEXT NOT NULL CHECK (type ~ '^[a-z][a-z0-9_]{1,39}$'),
    auteur_personne_id UUID,
    -- Tâche système déclarée ; jamais l'IA comme auteur d'un geste (R-DM32).
    auteur_tache TEXT CHECK (auteur_tache IS NULL OR auteur_tache IN (
        'ordonnanceur_echeances', 'purge_conservation', 'reprise', 'routage_deterministe')),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    statut_avant TEXT,
    statut_apres TEXT,
    file_avant_id UUID,
    file_apres_id UUID,
    agent_avant_id UUID,
    agent_apres_id UUID,
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code ~ '^[a-z0-9_]{1,60}$'),
    ref_id UUID,
    empreinte TEXT NOT NULL DEFAULT '' ,
    empreinte_precedente TEXT NOT NULL DEFAULT '',
    CONSTRAINT ent_dem_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_fait_rang_uq UNIQUE (etablissement_id, demande_id, rang),
    CONSTRAINT ent_dem_fait_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_fait_auteur_cfk FOREIGN KEY (etablissement_id, auteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_fait_auteur_chk CHECK (auteur_personne_id IS NOT NULL OR auteur_tache IS NOT NULL),
    CONSTRAINT ent_dem_fait_statuts_chk CHECK (
        (statut_avant IS NULL OR statut_avant IN (
            'brouillon', 'deposee', 'prise', 'en_attente_demandeur', 'en_attente_tiers',
            'en_validation', 'traitee', 'close', 'rouverte', 'annulee'))
        AND (statut_apres IS NULL OR statut_apres IN (
            'brouillon', 'deposee', 'prise', 'en_attente_demandeur', 'en_attente_tiers',
            'en_validation', 'traitee', 'close', 'rouverte', 'annulee')))
);
CREATE INDEX IF NOT EXISTS ent_dem_fait_demande_idx ON ent_dem_fait (etablissement_id, demande_id, rang);

-- Texte canonique d'un fait, à empreinter (fonction pure, réutilisée par la vérification).
CREATE OR REPLACE FUNCTION ent_dem_fait_canonique(f ent_dem_fait) RETURNS text
LANGUAGE sql IMMUTABLE AS $$
    SELECT concat_ws('|',
        f.etablissement_id::text, f.demande_id::text, f.rang::text, f.type,
        COALESCE(f.auteur_personne_id::text, ''), COALESCE(f.auteur_tache, ''),
        to_char(f.le AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US'),
        COALESCE(f.statut_avant, ''), COALESCE(f.statut_apres, ''),
        COALESCE(f.file_avant_id::text, ''), COALESCE(f.file_apres_id::text, ''),
        COALESCE(f.agent_avant_id::text, ''), COALESCE(f.agent_apres_id::text, ''),
        COALESCE(f.motif_code, ''), COALESCE(f.ref_id::text, ''), f.empreinte_precedente)
$$;

CREATE OR REPLACE FUNCTION ent_dem_fait_chainer() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    dernier RECORD;
BEGIN
    -- Un seul ajout à la fois par demande : le rang suit sans trou et la chaîne reste linéaire.
    PERFORM pg_advisory_xact_lock(hashtextextended('ent_dem_fait:' || NEW.demande_id::text, 0));
    SELECT rang, empreinte INTO dernier
      FROM ent_dem_fait
     WHERE etablissement_id = NEW.etablissement_id AND demande_id = NEW.demande_id
     ORDER BY rang DESC LIMIT 1;
    IF FOUND THEN
        NEW.rang := dernier.rang + 1;
        NEW.empreinte_precedente := dernier.empreinte;
    ELSE
        NEW.rang := 1;
        NEW.empreinte_precedente := repeat('0', 64);
    END IF;
    NEW.empreinte := encode(sha256(convert_to(ent_dem_fait_canonique(NEW), 'UTF8')), 'hex');
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_dem_fait_chainer_trg ON ent_dem_fait;
CREATE TRIGGER ent_dem_fait_chainer_trg
    BEFORE INSERT ON ent_dem_fait
    FOR EACH ROW EXECUTE FUNCTION ent_dem_fait_chainer();

CREATE OR REPLACE FUNCTION ent_dem_fait_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_dem_fait : ajout seul, ni modification ni suppression (R-DM28, NJ-89)'
        USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_fait_ajout_seul_trg ON ent_dem_fait;
CREATE TRIGGER ent_dem_fait_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_dem_fait
    FOR EACH ROW EXECUTE FUNCTION ent_dem_fait_ajout_seul();
DROP TRIGGER IF EXISTS ent_dem_fait_sans_vidage_trg ON ent_dem_fait;
CREATE TRIGGER ent_dem_fait_sans_vidage_trg
    BEFORE TRUNCATE ON ent_dem_fait
    FOR EACH STATEMENT EXECUTE FUNCTION ent_dem_fait_ajout_seul();

-- Vérification de la chaîne d'une demande : renvoie le rang du premier maillon altéré (NULL = intacte).
CREATE OR REPLACE FUNCTION ent_dem_verifier_chaine(p_etab UUID, p_demande UUID) RETURNS INT
LANGUAGE plpgsql STABLE AS $$
DECLARE
    f ent_dem_fait;
    precedent TEXT := repeat('0', 64);
    attendu_rang INT := 1;
BEGIN
    FOR f IN SELECT * FROM ent_dem_fait WHERE etablissement_id = p_etab AND demande_id = p_demande ORDER BY rang
    LOOP
        IF f.rang <> attendu_rang OR f.empreinte_precedente <> precedent
           OR f.empreinte <> encode(sha256(convert_to(ent_dem_fait_canonique(f), 'UTF8')), 'hex') THEN
            RETURN f.rang;
        END IF;
        precedent := f.empreinte;
        attendu_rang := attendu_rang + 1;
    END LOOP;
    RETURN NULL;
END
$$;

-- ───────────────────────── Échéances ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_echeance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'prise_en_charge', 'traitement', 'complement', 'cloture_sans_reponse', 'decision_implicite',
        'verification', 'pour_quand', 'escalade')),
    palier SMALLINT CHECK (palier IS NULL OR palier >= 1),
    due_le TIMESTAMPTZ NOT NULL,
    -- Suspension (attente du demandeur, fermeture, restauration) : le décompte ne court plus (R-DM20).
    suspendue_depuis TIMESTAMPTZ,
    traitee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_echeance_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_echeance_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_dem_echeance_ouvertes_idx
    ON ent_dem_echeance (etablissement_id, due_le) WHERE traitee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_dem_echeance_demande_idx ON ent_dem_echeance (etablissement_id, demande_id);
DROP TRIGGER IF EXISTS ent_dem_echeance_sans_suppression_trg ON ent_dem_echeance;
CREATE TRIGGER ent_dem_echeance_sans_suppression_trg
    BEFORE DELETE ON ent_dem_echeance
    FOR EACH ROW EXECUTE FUNCTION ent_dem_sans_suppression();

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_demande',
    'ent_dem_echeance',
    'ent_dem_fait',
    'ent_dem_numerotation',
    'ent_dem_valeur'
]);
