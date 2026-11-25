-- 2544 — Socle 27, lot G3a : **guichet des demandes de droits** (dépôt, qualification, identité, horloge).
-- Réf. : 27_RGPD_ARCHIVES.md §9.1 à §9.4, §12.1, §17.4, §21 ; ajout personas §26.6 (`recue_le` distinct de `cree_le`).
-- Migration **additive et rejouable**, sans JSON, sans tableau, sans BYTEA (D-036, E1–E14).
-- Règle 38 : placée à la FIN du tableau de `migrate.rs` (après 690) ; chaque fonction créée ici porte
-- `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`.
--
-- Ce que cette migration ne crée PAS (lots suivants, plage réservée) : `rgp_extrait*`, `rgp_requisition`
-- (2545, lot G3b), `rgp_demande_sujet` (2556, demande à plusieurs sujets, RGP-302). Le sujet principal
-- reste porté par `rgp_demande.sujet_type` / `sujet_id`.
--
-- Tables :
-- * `rgp_demande_compteur` + `rgp_demande_numero_suivant()` : numéro par établissement et par année civile,
--   **sans trou visible** (l'incrément vit dans la transaction du dépôt : une transaction annulée ne consomme
--   pas de numéro, comme `ent_doc_compteur` de 3702).
-- * `rgp_demande` : une demande, son demandeur, la personne concernée, sa qualification, son horloge.
-- * `rgp_demande_droit` : les droits demandés et la décision prise pour chacun (motif par article).
-- * `rgp_demande_identite` : la vérification d'identité et de qualité ; la **pièce** est un document du socle 7,
--   détruite à la clôture (R-RG16) — seules les mentions « vérifiée le, par » restent.
-- * `rgp_demande_journal` : transitions en AJOUT SEUL, écrites dans la transaction du fait (R-RG40).

-- ─────────────────────────── Compteur de numéros (E3) ───────────────────────────
CREATE TABLE IF NOT EXISTS rgp_demande_compteur (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee INT NOT NULL CHECK (annee BETWEEN 2000 AND 2999),
    dernier INT NOT NULL DEFAULT 0 CHECK (dernier >= 0),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, annee)
);

-- Numéro suivant, atomique : un seul INSERT … ON CONFLICT DO UPDATE (deux dépôts simultanés n'obtiennent
-- jamais le même numéro). À appeler UNIQUEMENT dans la transaction du dépôt, jamais « pour voir ».
CREATE OR REPLACE FUNCTION rgp_demande_numero_suivant(p_etab UUID, p_annee INT)
RETURNS INT
LANGUAGE sql
SET search_path = pg_catalog, public, pg_temp AS $$
    INSERT INTO public.rgp_demande_compteur (etablissement_id, annee, dernier)
    VALUES (p_etab, p_annee, 1)
    ON CONFLICT (etablissement_id, annee)
    DO UPDATE SET dernier = public.rgp_demande_compteur.dernier + 1, maj_le = now()
    RETURNING dernier
$$;

-- Un compteur ne recule jamais et ne se supprime pas (même garde que 3702).
CREATE OR REPLACE FUNCTION rgp_demande_compteur_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_demande_compteur : suppression interdite' USING ERRCODE = '23001';
    END IF;
    IF NEW.dernier < OLD.dernier OR NEW.dernier > OLD.dernier + 1
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.annee IS DISTINCT FROM OLD.annee THEN
        RAISE EXCEPTION 'rgp_demande_compteur : le compteur avance d''un pas, jamais en arrière'
            USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_demande_compteur_garde_trg ON rgp_demande_compteur;
CREATE TRIGGER rgp_demande_compteur_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_demande_compteur
    FOR EACH ROW EXECUTE FUNCTION rgp_demande_compteur_garde();

-- ─────────────────────────────── La demande ───────────────────────────────
CREATE TABLE IF NOT EXISTS rgp_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Numéro lisible par l'usager : « 2027-014 ». Unique par établissement et par année.
    annee INT NOT NULL CHECK (annee BETWEEN 2000 AND 2999),
    numero INT NOT NULL CHECK (numero > 0),

    -- Entrée (§9.1). `dpo` et `cnil` : demande transmise par le DPO ou par l'autorité.
    canal TEXT NOT NULL CHECK (canal IN ('en_ligne', 'courrier', 'courriel', 'guichet', 'dpo', 'cnil')),
    -- Jour de RÉCEPTION : fait courir le délai (art. 12.3). Pour une demande saisie au guichet, c'est la date
    -- du courrier reçu, qui peut précéder la saisie (`cree_le`) : RGP-303 compare les deux.
    recue_le DATE NOT NULL,
    -- Échéance légale d'un mois, calculée au jour de l'établissement par le code (jamais `now()` ici).
    echeance_le DATE NOT NULL,
    -- Prolongation de deux mois au plus (art. 12.3), toujours motivée.
    prolongee_jusqu_au DATE,
    prolongation_motif TEXT CHECK (prolongation_motif IS NULL OR length(btrim(prolongation_motif)) BETWEEN 3 AND 1000),
    prolongee_le TIMESTAMPTZ,
    prolongee_par_compte_id UUID REFERENCES ent_user (id),

    -- Demandeur. `demandeur_compte_id` nul pour un dépôt sans compte (ancien élève, parent sans compte) :
    -- le nom saisi et un moyen de contact prennent le relais. Aucune donnée d'identité au-delà.
    demandeur_compte_id UUID REFERENCES ent_user (id),
    demandeur_personne_id UUID,
    demandeur_nom_saisi TEXT CHECK (demandeur_nom_saisi IS NULL OR length(btrim(demandeur_nom_saisi)) BETWEEN 2 AND 200),
    demandeur_contact TEXT CHECK (demandeur_contact IS NULL OR length(btrim(demandeur_contact)) BETWEEN 3 AND 254),
    -- Qualité invoquée (§9.3) ; vérifiée par le socle 4 À LA DATE de réception.
    qualite TEXT NOT NULL CHECK (qualite IN (
        'soi_meme', 'eleve_mineur', 'titulaire_autorite', 'parent_sans_autorite',
        'representant_majeur_protege', 'mandataire', 'ancien_eleve', 'personnel', 'tiers')),

    -- Personne concernée (sujet principal). `sujet_id` nul quand la personne n'est pas (encore) identifiée
    -- dans la base : un ancien élève qui écrit sans compte, par exemple.
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne', 'agent', 'inconnu')),
    sujet_id UUID,
    sujet_nom_saisi TEXT CHECK (sujet_nom_saisi IS NULL OR length(btrim(sujet_nom_saisi)) BETWEEN 2 AND 200),

    -- Qualification motivée et modifiable (RGP-141). `a_qualifier` tant que personne ne l'a tranchée.
    qualification TEXT NOT NULL DEFAULT 'a_qualifier'
        CHECK (qualification IN ('a_qualifier', 'rgpd', 'cada', 'document')),
    qualification_motif TEXT CHECK (qualification_motif IS NULL OR length(btrim(qualification_motif)) BETWEEN 3 AND 1000),
    qualifiee_le TIMESTAMPTZ,
    qualifiee_par_compte_id UUID REFERENCES ent_user (id),

    -- Périmètre demandé, tel que l'usager l'a décrit (texte court, jamais une donnée d'élève).
    perimetre_texte TEXT CHECK (perimetre_texte IS NULL OR length(btrim(perimetre_texte)) BETWEEN 3 AND 2000),
    perimetre_du DATE,
    perimetre_au DATE,

    -- États du guichet. `a_relire`, `prete` et `remise` servent à l'extrait assemblé (lot G3b) : la colonne
    -- les accepte dès maintenant pour qu'une migration versée n'ait pas à changer (règle 25).
    etat TEXT NOT NULL DEFAULT 'recue' CHECK (etat IN (
        'recue', 'identite_a_verifier', 'en_cours', 'a_relire', 'prete', 'remise', 'close', 'refusee')),
    gestionnaire_personne_id UUID,
    affectee_le TIMESTAMPTZ,

    -- Horloge : précision d'identité demandée. Par défaut le délai NE SUSPEND PAS (§9.2 : la vérification ne
    -- suspend pas le délai [H, à confirmer DPO]) ; l'attente est comptée et affichée, et seul un établissement
    -- qui décide le contraire l'ajoutera à l'échéance (réglage à venir). Valeur par défaut prudente (D-069) :
    -- le délai court, c'est la lecture la plus protectrice pour la personne.
    identite_demandee_le DATE,
    identite_fournie_le DATE,

    -- Clôture.
    close_le TIMESTAMPTZ,
    close_par_compte_id UUID REFERENCES ent_user (id),
    reponse_resume TEXT CHECK (reponse_resume IS NULL OR length(btrim(reponse_resume)) BETWEEN 3 AND 4000),
    refus_motif TEXT CHECK (refus_motif IS NULL OR length(btrim(refus_motif)) BETWEEN 3 AND 2000),
    -- Réouverture sur réclamation (RGP-167) : compteur, pour ne pas perdre l'historique de la clôture.
    reouvertures INT NOT NULL DEFAULT 0 CHECK (reouvertures >= 0),

    -- Information des titulaires quand l'élève mineur demande lui-même (S-550, CL27-04).
    titulaires_informes_le TIMESTAMPTZ,
    titulaires_non_informes_motif TEXT
        CHECK (titulaires_non_informes_motif IS NULL OR length(btrim(titulaires_non_informes_motif)) BETWEEN 3 AND 1000),

    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_compte_id UUID REFERENCES ent_user (id),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT rgp_demande_numero_uq UNIQUE (etablissement_id, annee, numero),
    CONSTRAINT rgp_demande_personne_fk FOREIGN KEY (etablissement_id, demandeur_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT rgp_demande_gestionnaire_fk FOREIGN KEY (etablissement_id, gestionnaire_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    -- Un demandeur est identifié par un compte OU par un nom saisi (jamais ni l'un ni l'autre).
    CONSTRAINT rgp_demande_demandeur_connu CHECK (
        demandeur_compte_id IS NOT NULL OR demandeur_nom_saisi IS NOT NULL),
    -- Sujet « inconnu » : un nom saisi, pas d'identifiant ; sujet connu : un identifiant.
    CONSTRAINT rgp_demande_sujet_coherent CHECK (
        (sujet_type = 'inconnu' AND sujet_id IS NULL AND sujet_nom_saisi IS NOT NULL)
        OR (sujet_type <> 'inconnu' AND sujet_id IS NOT NULL)),
    CONSTRAINT rgp_demande_echeance_apres_reception CHECK (echeance_le >= recue_le),
    CONSTRAINT rgp_demande_prolongation_coherente CHECK (
        (prolongee_jusqu_au IS NULL AND prolongation_motif IS NULL AND prolongee_le IS NULL
            AND prolongee_par_compte_id IS NULL)
        OR (prolongee_jusqu_au IS NOT NULL AND prolongation_motif IS NOT NULL AND prolongee_le IS NOT NULL
            AND prolongee_jusqu_au > echeance_le)),
    CONSTRAINT rgp_demande_qualification_motivee CHECK (
        qualification = 'a_qualifier'
        OR (qualification_motif IS NOT NULL AND qualifiee_le IS NOT NULL)),
    CONSTRAINT rgp_demande_perimetre_dates CHECK (
        perimetre_du IS NULL OR perimetre_au IS NULL OR perimetre_au >= perimetre_du),
    -- Une identité peut être établie SANS qu'une précision ait été demandée (session forte, personne vue au
    -- guichet) : seules les deux dates ensemble doivent être dans l'ordre.
    CONSTRAINT rgp_demande_identite_dates CHECK (
        identite_demandee_le IS NULL OR identite_fournie_le IS NULL
        OR identite_fournie_le >= identite_demandee_le),
    -- Close ou refusée : la date et l'auteur de la clôture existent, et réciproquement.
    CONSTRAINT rgp_demande_cloture_coherente CHECK (
        (etat IN ('close', 'refusee')) = (close_le IS NOT NULL)),
    CONSTRAINT rgp_demande_refus_motive CHECK (etat <> 'refusee' OR refus_motif IS NOT NULL),
    CONSTRAINT rgp_demande_affectation_coherente CHECK (
        (gestionnaire_personne_id IS NULL) = (affectee_le IS NULL)),
    CONSTRAINT rgp_demande_titulaires_exclusif CHECK (
        titulaires_informes_le IS NULL OR titulaires_non_informes_motif IS NULL)
);

-- `rgp_demande` est référencée par (etablissement_id, id) : la clé composite exige son propre index unique.
CREATE UNIQUE INDEX IF NOT EXISTS rgp_demande_etab_id_uq ON rgp_demande (etablissement_id, id);

-- File du guichet : les demandes ouvertes d'un établissement, par échéance.
CREATE INDEX IF NOT EXISTS rgp_demande_file_idx
    ON rgp_demande (etablissement_id, echeance_le)
    WHERE etat NOT IN ('close', 'refusee');
CREATE INDEX IF NOT EXISTS rgp_demande_sujet_idx
    ON rgp_demande (etablissement_id, sujet_type, sujet_id) WHERE sujet_id IS NOT NULL;
-- Suivi côté demandeur (RGP-155).
CREATE INDEX IF NOT EXISTS rgp_demande_demandeur_idx
    ON rgp_demande (etablissement_id, demandeur_compte_id) WHERE demandeur_compte_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS rgp_demande_gestionnaire_idx
    ON rgp_demande (etablissement_id, gestionnaire_personne_id) WHERE gestionnaire_personne_id IS NOT NULL;

-- ───────────────────────── Les droits demandés et leurs décisions ─────────────────────────
CREATE TABLE IF NOT EXISTS rgp_demande_droit (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    droit TEXT NOT NULL CHECK (droit IN (
        'acces', 'rectification', 'effacement', 'limitation', 'opposition', 'portabilite', 'consultations')),
    -- Décision prise pour CE droit. `en_examen` tant que rien n'est tranché : jamais de refus par défaut
    -- (R-RG17 pour l'excessif, §9.5 pour l'opposition).
    decision TEXT NOT NULL DEFAULT 'en_examen'
        CHECK (decision IN ('en_examen', 'accorde', 'partiel', 'refuse', 'sans_objet')),
    -- Code court de motif (article invoqué, catégorie) + texte lisible par la personne.
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code ~ '^[a-z0-9_.]{2,60}$'),
    motif_texte TEXT CHECK (motif_texte IS NULL OR length(btrim(motif_texte)) BETWEEN 3 AND 4000),
    decide_le TIMESTAMPTZ,
    decide_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_demande_droit_uq UNIQUE (etablissement_id, demande_id, droit),
    CONSTRAINT rgp_demande_droit_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT,
    -- Une décision prise est datée et motivée ; un refus ou un partiel l'est toujours par un texte.
    CONSTRAINT rgp_demande_droit_decision_datee CHECK (
        (decision = 'en_examen') = (decide_le IS NULL)),
    CONSTRAINT rgp_demande_droit_refus_motive CHECK (
        decision NOT IN ('refuse', 'partiel') OR (motif_code IS NOT NULL AND motif_texte IS NOT NULL))
);

-- ───────────────────────── Identité et qualité (§9.2, R-RG16) ─────────────────────────
CREATE TABLE IF NOT EXISTS rgp_demande_identite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('session_forte', 'code_courriel', 'piece', 'mandat')),
    -- Résultat de la vérification : `verifiee`, `echouee` (réponse motivée « nous ne pouvons pas vous
    -- identifier »), ou `attendue` (précision demandée, rien reçu).
    resultat TEXT NOT NULL DEFAULT 'attendue'
        CHECK (resultat IN ('attendue', 'verifiee', 'echouee')),
    -- Qualité constatée par le socle 4 À LA DATE de réception (lien actif, autorité, restriction).
    qualite_constatee TEXT CHECK (qualite_constatee IS NULL OR qualite_constatee IN (
        'confirmee', 'confirmee_sans_autorite', 'restreinte', 'absente', 'sans_objet')),
    qualite_note TEXT CHECK (qualite_note IS NULL OR length(btrim(qualite_note)) BETWEEN 3 AND 1000),
    -- Pièce consultée : document du coffre (socle 7), **détruit à la clôture** (R-RG16). Après destruction,
    -- la colonne est nulle et `piece_detruite_le` dit quand : la mention de la vérification subsiste seule.
    piece_document_id UUID,
    piece_detruite_le TIMESTAMPTZ,
    verifie_par_compte_id UUID REFERENCES ent_user (id),
    verifie_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_demande_identite_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT rgp_demande_identite_resultat_date CHECK (
        (resultat = 'attendue') = (verifie_le IS NULL)),
    -- Une pièce détruite n'est plus référencée (R-RG16).
    CONSTRAINT rgp_demande_identite_piece_detruite CHECK (
        piece_detruite_le IS NULL OR piece_document_id IS NULL)
);
CREATE INDEX IF NOT EXISTS rgp_demande_identite_demande_idx
    ON rgp_demande_identite (etablissement_id, demande_id, cree_le DESC);
-- Une pièce encore détenue, par demande : la clôture les parcourt pour les détruire.
CREATE INDEX IF NOT EXISTS rgp_demande_identite_piece_idx
    ON rgp_demande_identite (etablissement_id, demande_id)
    WHERE piece_document_id IS NOT NULL;

-- ───────────────────────── Journal des transitions (R-RG40, ajout seul) ─────────────────────────
CREATE TABLE IF NOT EXISTS rgp_demande_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    evenement TEXT NOT NULL CHECK (evenement IN (
        'recue', 'qualifiee', 'identite_demandee', 'identite_verifiee', 'identite_echouee',
        'affectee', 'prolongee', 'droit_ajoute', 'droit_decide', 'titulaires_informes',
        'close', 'refusee', 'rouverte', 'piece_detruite', 'accuse_envoye', 'echeance_alertee')),
    -- Détail court et neutre : jamais une donnée d'élève, jamais le contenu d'un extrait (E14, R-RG32).
    detail TEXT CHECK (detail IS NULL OR length(btrim(detail)) BETWEEN 1 AND 1000),
    par_compte_id UUID,
    par_role TEXT CHECK (par_role IS NULL OR length(par_role) <= 60),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS rgp_demande_journal_demande_idx
    ON rgp_demande_journal (etablissement_id, demande_id, le);

-- Ajout seul : ni UPDATE, ni DELETE, ni TRUNCATE (même schéma que 091, 2540 et 2541).
CREATE OR REPLACE FUNCTION rgp_demande_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION 'rgp_demande_journal est en ajout seul (% interdit)', TG_OP
        USING ERRCODE = '42501';
END;
$$;

CREATE OR REPLACE TRIGGER rgp_demande_journal_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON rgp_demande_journal
    FOR EACH ROW EXECUTE FUNCTION rgp_demande_journal_ajout_seul();
CREATE OR REPLACE TRIGGER rgp_demande_journal_sans_truncate_trg
    BEFORE TRUNCATE ON rgp_demande_journal
    FOR EACH STATEMENT EXECUTE FUNCTION rgp_demande_journal_ajout_seul();

-- Le numéro, l'année et la date de réception d'une demande ne changent jamais (le délai légal en dépend) ;
-- une demande ne se supprime pas (sa conservation suit la catégorie `rgpd.demande`).
CREATE OR REPLACE FUNCTION rgp_demande_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_demande : suppression interdite (conservation rgpd.demande)'
            USING ERRCODE = '42501';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.annee IS DISTINCT FROM OLD.annee
       OR NEW.numero IS DISTINCT FROM OLD.numero
       OR NEW.recue_le IS DISTINCT FROM OLD.recue_le
       OR NEW.canal IS DISTINCT FROM OLD.canal
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'rgp_demande : numéro, année, canal, date de réception et date de saisie sont figés'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_demande_garde_trg ON rgp_demande;
CREATE TRIGGER rgp_demande_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_demande
    FOR EACH ROW EXECUTE FUNCTION rgp_demande_garde();

SELECT ent_rls_poser(ARRAY[
    'rgp_demande_compteur',
    'rgp_demande',
    'rgp_demande_droit',
    'rgp_demande_identite',
    'rgp_demande_journal']);
