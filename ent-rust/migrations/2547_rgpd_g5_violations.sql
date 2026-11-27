-- 2547 — Socle 27, palier G5 : **registre des violations de données** (déclaration, horloge des 72 h,
-- qualification, risque, notification CNIL, information des personnes, violation de plateforme propagée).
-- Réf. : 27_RGPD_ARCHIVES.md §8.7, §13.3 (CL27-41 à CL27-47), §14.10 (RGP-240 à RGP-259), §17.6, §21 ;
-- ajout personas §26.2 (RGP-311, RGP-312) et §26.3 (R-RG50). RGPD art. 33 et 34.
-- Migration **additive et rejouable**, sans JSON, sans tableau, sans BYTEA (D-036, E1–E14).
-- Règle 38 : placée à la FIN du tableau de `migrate.rs` ; chaque fonction créée ici porte
-- `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`.
--
-- Ce que cette migration ne crée PAS : `rgp_violation_echange` (conversation d'aide pseudonymisée, lot G11, IA) :
-- aucune table vide ne part sans son code (elle prendra la réserve 2557).
--
-- Tables :
-- * `rgp_violation_compteur` + `rgp_violation_numero_suivant()` : numéro par établissement et par année, sans trou
--   visible (même schéma que `rgp_demande_compteur`, 2544).
-- * `rgp_violation` : un incident, son horloge, sa qualification, son risque, sa notification, sa clôture.
-- * `rgp_violation_categorie` : catégories de données touchées (codes du catalogue de conservation, validés par le code).
-- * `rgp_violation_mesure` : mesures prises et envisagées (art. 33.3 d).
-- * `rgp_violation_notification` : notifications à la CNIL, initiale et complémentaires (art. 33.4), saisies après
--   coup : ScolIA ne transmet rien à la CNIL [H], l'établissement notifie sur le téléservice.
-- * `rgp_violation_personne` : personnes concernées calculées depuis l'origine (RGP-244), pour l'information (art. 34).
-- * `rgp_violation_etape` : chronologie en AJOUT SEUL, écrite dans la transaction du fait (R-RG40).
-- * `rgp_violation_plateforme` (**P**, sans établissement, sans RLS) et `rgp_violation_plateforme_etab` : violation
--   déclarée une fois par l'éditeur et propagée à chaque établissement concerné avec ses seuls éléments (R-RG36).

-- ─────────────────────────── Compteur de numéros (E3) ───────────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation_compteur (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee INT NOT NULL CHECK (annee BETWEEN 2000 AND 2999),
    dernier INT NOT NULL DEFAULT 0 CHECK (dernier >= 0),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, annee)
);

-- Numéro suivant, atomique (deux déclarations simultanées n'obtiennent jamais le même numéro). À appeler UNIQUEMENT
-- dans la transaction de la déclaration : une transaction annulée ne consomme pas de numéro.
CREATE OR REPLACE FUNCTION rgp_violation_numero_suivant(p_etab UUID, p_annee INT)
RETURNS INT
LANGUAGE sql
SET search_path = pg_catalog, public, pg_temp AS $$
    INSERT INTO public.rgp_violation_compteur (etablissement_id, annee, dernier)
    VALUES (p_etab, p_annee, 1)
    ON CONFLICT (etablissement_id, annee)
    DO UPDATE SET dernier = public.rgp_violation_compteur.dernier + 1, maj_le = now()
    RETURNING dernier
$$;

CREATE OR REPLACE FUNCTION rgp_violation_compteur_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_violation_compteur : suppression interdite' USING ERRCODE = '23001';
    END IF;
    IF NEW.dernier < OLD.dernier OR NEW.dernier > OLD.dernier + 1
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.annee IS DISTINCT FROM OLD.annee THEN
        RAISE EXCEPTION 'rgp_violation_compteur : le compteur avance d''un pas, jamais en arrière'
            USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_violation_compteur_garde_trg ON rgp_violation_compteur;
CREATE TRIGGER rgp_violation_compteur_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_violation_compteur
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_compteur_garde();

-- ─────────────────────── Violation de plateforme (P, sans établissement) ───────────────────────
-- Déclarée une fois par l'éditeur (opérateur ScolIA, appel système de la console plateforme). Aucune donnée d'un
-- établissement ici : seulement ce que l'éditeur sait de l'incident. Les éléments propres à chaque établissement vivent
-- dans `rgp_violation_plateforme_etab`, sous RLS (R-RG36).
CREATE TABLE IF NOT EXISTS rgp_violation_plateforme (
    id UUID PRIMARY KEY,
    -- Référence lisible de l'éditeur (« P-2027-002 »), unique.
    reference TEXT NOT NULL UNIQUE CHECK (reference ~ '^P-[0-9]{4}-[0-9]{3,}$'),
    -- Prise de connaissance par l'éditeur : horloge « dans les meilleurs délais » de l'art. 33.2 envers les
    -- établissements ; pour la couche compte, horloge de 72 h de ScolIA responsable (R-RG50).
    connue_le TIMESTAMPTZ NOT NULL,
    declaree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    declaree_par TEXT NOT NULL CHECK (length(btrim(declaree_par)) BETWEEN 2 AND 120),
    intitule TEXT NOT NULL CHECK (length(btrim(intitule)) BETWEEN 3 AND 200),
    description TEXT NOT NULL CHECK (length(btrim(description)) BETWEEN 3 AND 4000),
    confidentialite BOOLEAN NOT NULL DEFAULT false,
    integrite BOOLEAN NOT NULL DEFAULT false,
    disponibilite BOOLEAN NOT NULL DEFAULT false,
    -- R-RG50 : la violation touche-t-elle la couche compte (identifiants, comptes), dont ScolIA est responsable ?
    couche_compte BOOLEAN NOT NULL DEFAULT false,
    -- Notification de ScolIA à la CNIL pour sa part (RGP-311), saisie après coup.
    notifiee_cnil_le TIMESTAMPTZ,
    reference_cnil TEXT CHECK (reference_cnil IS NULL OR length(btrim(reference_cnil)) BETWEEN 2 AND 120),
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('ouverte', 'close')),
    close_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_plateforme_nature CHECK (confidentialite OR integrite OR disponibilite),
    CONSTRAINT rgp_violation_plateforme_connue_avant CHECK (connue_le <= declaree_le),
    CONSTRAINT rgp_violation_plateforme_cnil_coherente CHECK (
        (notifiee_cnil_le IS NULL) = (reference_cnil IS NULL)),
    CONSTRAINT rgp_violation_plateforme_cnil_couche_compte CHECK (notifiee_cnil_le IS NULL OR couche_compte),
    CONSTRAINT rgp_violation_plateforme_cloture CHECK ((etat = 'close') = (close_le IS NOT NULL))
);

-- ─────────────────────────────── La violation ───────────────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee INT NOT NULL CHECK (annee BETWEEN 2000 AND 2999),
    numero INT NOT NULL CHECK (numero > 0),
    -- `violation` : atteinte à des données personnelles (art. 4.12) ; `incident_securite` : incident sans atteinte
    -- à des données personnelles, tenu dans le même registre (RGP-259), sans horloge CNIL exigée.
    nature_incident TEXT NOT NULL DEFAULT 'violation'
        CHECK (nature_incident IN ('violation', 'incident_securite')),
    -- Origine : une personne qui déclare, la supervision ou un socle (port `violations::declarer`), ou la plateforme.
    origine TEXT NOT NULL CHECK (origine IN ('declaration', 'supervision', 'plateforme')),
    plateforme_violation_id UUID REFERENCES rgp_violation_plateforme (id) ON DELETE RESTRICT,
    -- Source d'une déclaration automatique : code court du détecteur et objet à l'origine (une alerte, un export).
    -- Sert la déduplication : la même alerte ne crée jamais deux violations.
    source_code TEXT CHECK (source_code IS NULL OR source_code ~ '^[a-z0-9_.]{3,60}$'),
    source_ref UUID,
    declaree_par_compte_id UUID REFERENCES ent_user (id),
    -- Les trois champs de la déclaration (RGP-240) : ce qui s'est passé, depuis quand, ce qui est touché.
    intitule TEXT NOT NULL CHECK (length(btrim(intitule)) BETWEEN 3 AND 200),
    description TEXT CHECK (description IS NULL OR length(btrim(description)) BETWEEN 3 AND 4000),
    constatee_le TIMESTAMPTZ,
    -- Prise de connaissance (art. 33.1) : l'instant de la déclaration ; une saisie au guichet peut la dater plus tôt
    -- (incident signalé par téléphone la veille), jamais après la saisie. Figée.
    connue_le TIMESTAMPTZ NOT NULL,
    -- Échéance CNIL : prise de connaissance + 72 h, exactement. **Jamais bloquante** (R-RG21) : une notification tardive
    -- est acceptée, avec un motif.
    echeance_cnil_le TIMESTAMPTZ NOT NULL,
    -- Qualification (RGP-243) : nature de l'atteinte.
    confidentialite BOOLEAN NOT NULL DEFAULT false,
    integrite BOOLEAN NOT NULL DEFAULT false,
    disponibilite BOOLEAN NOT NULL DEFAULT false,
    -- Personnes concernées : nombre (calculé depuis l'origine quand elle est connue, sinon estimé).
    personnes_nombre INT CHECK (personnes_nombre IS NULL OR personnes_nombre >= 0),
    personnes_estime BOOLEAN NOT NULL DEFAULT true,
    personnes_mineurs BOOLEAN NOT NULL DEFAULT false,
    personnes_origine_type TEXT CHECK (personnes_origine_type IS NULL
        OR personnes_origine_type IN ('classe', 'eleves', 'comptes')),
    personnes_origine_id UUID,
    consequences TEXT CHECK (consequences IS NULL OR length(btrim(consequences)) BETWEEN 3 AND 4000),
    qualifiee_le TIMESTAMPTZ,
    qualifiee_par_compte_id UUID REFERENCES ent_user (id),
    -- Risque proposé par la grille déterministe (jamais décidé par elle) et risque décidé par la direction, après avis
    -- du DPO tracé (RGP-245 ★).
    risque_propose TEXT CHECK (risque_propose IS NULL OR risque_propose IN ('faible', 'avere', 'eleve')),
    avis_dpo TEXT CHECK (avis_dpo IS NULL OR length(btrim(avis_dpo)) BETWEEN 3 AND 2000),
    avis_dpo_le TIMESTAMPTZ,
    avis_dpo_par_compte_id UUID REFERENCES ent_user (id),
    risque TEXT CHECK (risque IS NULL OR risque IN ('faible', 'avere', 'eleve')),
    risque_motif TEXT CHECK (risque_motif IS NULL OR length(btrim(risque_motif)) BETWEEN 3 AND 2000),
    risque_decide_le TIMESTAMPTZ,
    risque_decide_par_compte_id UUID REFERENCES ent_user (id),
    -- Notification à la CNIL (art. 33) : première notification (les suivantes sont dans `rgp_violation_notification`).
    notifiee_cnil_le TIMESTAMPTZ,
    reference_cnil TEXT CHECK (reference_cnil IS NULL OR length(btrim(reference_cnil)) BETWEEN 2 AND 120),
    retard_motif TEXT CHECK (retard_motif IS NULL OR length(btrim(retard_motif)) BETWEEN 3 AND 2000),
    -- Non-notification justifiée (art. 33.5) : seulement pour un risque décidé « faible ».
    non_notification_motif TEXT CHECK (non_notification_motif IS NULL
        OR length(btrim(non_notification_motif)) BETWEEN 3 AND 2000),
    -- Information des personnes (art. 34), exigée pour un risque élevé, sauf exception motivée (art. 34.3).
    personnes_informees_le TIMESTAMPTZ,
    personnes_informees_nombre INT CHECK (personnes_informees_nombre IS NULL OR personnes_informees_nombre >= 0),
    personnes_non_information_motif TEXT CHECK (personnes_non_information_motif IS NULL
        OR length(btrim(personnes_non_information_motif)) BETWEEN 3 AND 2000),
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('ouverte', 'qualifiee', 'notifiee', 'close')),
    close_le TIMESTAMPTZ,
    close_par_compte_id UUID REFERENCES ent_user (id),
    retour_experience TEXT CHECK (retour_experience IS NULL OR length(btrim(retour_experience)) BETWEEN 3 AND 4000),
    -- Revue à 30 jours de la clôture (§8.7 point 8).
    revue_le DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_numero_uq UNIQUE (etablissement_id, annee, numero),
    CONSTRAINT rgp_violation_echeance_72h CHECK (echeance_cnil_le = connue_le + interval '72 hours'),
    CONSTRAINT rgp_violation_constatee_avant CHECK (constatee_le IS NULL OR constatee_le <= connue_le),
    CONSTRAINT rgp_violation_connue_avant_saisie CHECK (connue_le <= cree_le),
    CONSTRAINT rgp_violation_origine_coherente CHECK (
        (origine = 'plateforme') = (plateforme_violation_id IS NOT NULL)
        AND (origine <> 'supervision' OR source_code IS NOT NULL)
        AND (origine <> 'declaration' OR declaree_par_compte_id IS NOT NULL)),
    CONSTRAINT rgp_violation_personnes_origine CHECK (
        (personnes_origine_type IS NULL) = (personnes_origine_id IS NULL)
        OR personnes_origine_type IN ('eleves', 'comptes')),
    CONSTRAINT rgp_violation_qualification_datee CHECK ((qualifiee_le IS NULL) = (qualifiee_par_compte_id IS NULL)),
    CONSTRAINT rgp_violation_avis_complet CHECK (
        (avis_dpo IS NULL AND avis_dpo_le IS NULL AND avis_dpo_par_compte_id IS NULL)
        OR (avis_dpo IS NOT NULL AND avis_dpo_le IS NOT NULL AND avis_dpo_par_compte_id IS NOT NULL)),
    -- Un risque décidé l'est par quelqu'un, à une date, avec un motif, et **après** l'avis du DPO (RGP-245).
    CONSTRAINT rgp_violation_risque_decide CHECK (
        (risque IS NULL AND risque_decide_le IS NULL AND risque_decide_par_compte_id IS NULL AND risque_motif IS NULL)
        OR (risque IS NOT NULL AND risque_decide_le IS NOT NULL AND risque_decide_par_compte_id IS NOT NULL
            AND risque_motif IS NOT NULL AND avis_dpo IS NOT NULL)),
    CONSTRAINT rgp_violation_cnil_coherente CHECK ((notifiee_cnil_le IS NULL) = (reference_cnil IS NULL)),
    CONSTRAINT rgp_violation_cnil_apres_connaissance CHECK (notifiee_cnil_le IS NULL OR notifiee_cnil_le >= connue_le),
    -- R-RG21 : une notification au-delà des 72 h est acceptée, avec le motif du retard (art. 33.1).
    CONSTRAINT rgp_violation_retard_motive CHECK (
        notifiee_cnil_le IS NULL OR notifiee_cnil_le <= echeance_cnil_le OR retard_motif IS NOT NULL),
    -- Notifier ou justifier la non-notification, jamais les deux ; la non-notification exige un risque faible décidé.
    CONSTRAINT rgp_violation_notification_exclusive CHECK (notifiee_cnil_le IS NULL OR non_notification_motif IS NULL),
    CONSTRAINT rgp_violation_non_notification_faible CHECK (non_notification_motif IS NULL OR risque = 'faible'),
    CONSTRAINT rgp_violation_information_exclusive CHECK (
        personnes_informees_le IS NULL OR personnes_non_information_motif IS NULL),
    CONSTRAINT rgp_violation_information_datee CHECK (
        (personnes_informees_le IS NULL) = (personnes_informees_nombre IS NULL)),
    CONSTRAINT rgp_violation_cloture_coherente CHECK (
        (etat = 'close') = (close_le IS NOT NULL AND close_par_compte_id IS NOT NULL AND revue_le IS NOT NULL)),
    -- Une violation close l'est avec sa décision complète (§8.7 point 7, art. 33.5) : risque décidé ; notifiée ou
    -- non-notification justifiée ; risque élevé → personnes informées ou exception motivée. Un incident de sécurité
    -- sans atteinte aux données se clôt sans risque ni notification.
    CONSTRAINT rgp_violation_cloture_complete CHECK (
        etat <> 'close' OR nature_incident = 'incident_securite'
        OR (risque IS NOT NULL
            AND (notifiee_cnil_le IS NOT NULL OR non_notification_motif IS NOT NULL)
            AND (risque <> 'eleve' OR personnes_informees_le IS NOT NULL
                 OR personnes_non_information_motif IS NOT NULL)))
);
CREATE UNIQUE INDEX IF NOT EXISTS rgp_violation_etab_id_uq ON rgp_violation (etablissement_id, id);
-- Registre et file : les violations ouvertes d'un établissement, par échéance.
CREATE INDEX IF NOT EXISTS rgp_violation_file_idx
    ON rgp_violation (etablissement_id, echeance_cnil_le) WHERE etat <> 'close';
CREATE INDEX IF NOT EXISTS rgp_violation_registre_idx ON rgp_violation (etablissement_id, annee, numero);
CREATE INDEX IF NOT EXISTS rgp_violation_declarant_idx
    ON rgp_violation (etablissement_id, declaree_par_compte_id) WHERE declaree_par_compte_id IS NOT NULL;
-- La même alerte de supervision ne crée jamais deux violations (déduplication du port `violations::declarer`).
CREATE UNIQUE INDEX IF NOT EXISTS rgp_violation_source_uq
    ON rgp_violation (etablissement_id, source_code, source_ref)
    WHERE source_code IS NOT NULL AND source_ref IS NOT NULL;
-- Une violation de plateforme n'apparaît qu'une fois au registre d'un établissement.
CREATE UNIQUE INDEX IF NOT EXISTS rgp_violation_plateforme_uq
    ON rgp_violation (etablissement_id, plateforme_violation_id) WHERE plateforme_violation_id IS NOT NULL;

-- ─────────────────────────── Catégories de données touchées ───────────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation_categorie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    violation_id UUID NOT NULL,
    -- Code du catalogue de conservation (`conservation.rs`), validé par le code.
    categorie_code TEXT NOT NULL CHECK (categorie_code ~ '^[a-z0-9_.]{3,60}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_categorie_uq UNIQUE (etablissement_id, violation_id, categorie_code),
    CONSTRAINT rgp_violation_categorie_violation_fk FOREIGN KEY (etablissement_id, violation_id)
        REFERENCES rgp_violation (etablissement_id, id) ON DELETE RESTRICT
);

-- ─────────────────────────── Mesures prises et envisagées ───────────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation_mesure (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    violation_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('prise', 'envisagee')),
    texte TEXT NOT NULL CHECK (length(btrim(texte)) BETWEEN 3 AND 2000),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_mesure_violation_fk FOREIGN KEY (etablissement_id, violation_id)
        REFERENCES rgp_violation (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS rgp_violation_mesure_idx ON rgp_violation_mesure (etablissement_id, violation_id, le);

-- ─────────────────────────── Notifications à la CNIL (art. 33.4) ───────────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation_notification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    violation_id UUID NOT NULL,
    rang INT NOT NULL CHECK (rang >= 1),
    nature TEXT NOT NULL CHECK (nature IN ('initiale', 'complementaire')),
    notifiee_le TIMESTAMPTZ NOT NULL,
    reference_cnil TEXT NOT NULL CHECK (length(btrim(reference_cnil)) BETWEEN 2 AND 120),
    resume TEXT CHECK (resume IS NULL OR length(btrim(resume)) BETWEEN 3 AND 2000),
    par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_notification_rang_uq UNIQUE (etablissement_id, violation_id, rang),
    CONSTRAINT rgp_violation_notification_nature_rang CHECK ((nature = 'initiale') = (rang = 1)),
    CONSTRAINT rgp_violation_notification_violation_fk FOREIGN KEY (etablissement_id, violation_id)
        REFERENCES rgp_violation (etablissement_id, id) ON DELETE RESTRICT
);

-- ─────────────────────────── Personnes concernées (RGP-244, art. 34) ───────────────────────────
-- Une ligne par compte à informer (titulaire de l'autorité parentale, élève selon l'âge, personnel), calculée depuis
-- l'origine. Seule la direction qui gère la violation la lit ; l'information part par le socle 8 après confirmation.
CREATE TABLE IF NOT EXISTS rgp_violation_personne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    violation_id UUID NOT NULL,
    -- Personne touchée (l'élève, le personnel) et compte à informer (lui-même, ou un titulaire pour un mineur).
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'compte')),
    sujet_id UUID NOT NULL,
    destinataire_user_id UUID REFERENCES ent_user (id),
    informee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_personne_uq UNIQUE NULLS NOT DISTINCT
        (etablissement_id, violation_id, sujet_type, sujet_id, destinataire_user_id),
    CONSTRAINT rgp_violation_personne_violation_fk FOREIGN KEY (etablissement_id, violation_id)
        REFERENCES rgp_violation (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS rgp_violation_personne_idx ON rgp_violation_personne (etablissement_id, violation_id);

-- ─────────────────────────── Chronologie (R-RG40, ajout seul) ───────────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation_etape (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    violation_id UUID NOT NULL,
    etape_code TEXT NOT NULL CHECK (etape_code IN (
        'declaree', 'alerte_envoyee', 'qualifiee', 'categories', 'personnes_calculees', 'mesure',
        'risque_propose', 'avis_dpo', 'risque_decide', 'notifiee_cnil', 'notification_complementaire',
        'non_notification', 'personnes_informees', 'personnes_non_informees', 'echeance_alertee',
        'propagee', 'close')),
    -- Texte court et neutre : jamais le contenu d'une donnée touchée (E14).
    texte TEXT CHECK (texte IS NULL OR length(btrim(texte)) BETWEEN 1 AND 1000),
    par_compte_id UUID,
    par_role TEXT CHECK (par_role IS NULL OR length(par_role) <= 60),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Ordre d'écriture : deux étapes d'une même transaction ont le même `le` et le même `cree_le`.
    ordre BIGINT GENERATED ALWAYS AS IDENTITY,
    CONSTRAINT rgp_violation_etape_violation_fk FOREIGN KEY (etablissement_id, violation_id)
        REFERENCES rgp_violation (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS rgp_violation_etape_idx ON rgp_violation_etape (etablissement_id, violation_id, le);

CREATE OR REPLACE FUNCTION rgp_violation_etape_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION 'rgp_violation_etape est en ajout seul (% interdit)', TG_OP
        USING ERRCODE = '42501';
END;
$$;
CREATE OR REPLACE TRIGGER rgp_violation_etape_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON rgp_violation_etape
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_etape_ajout_seul();
CREATE OR REPLACE TRIGGER rgp_violation_etape_sans_truncate_trg
    BEFORE TRUNCATE ON rgp_violation_etape
    FOR EACH STATEMENT EXECUTE FUNCTION rgp_violation_etape_ajout_seul();

-- Les notifications à la CNIL sont des faits : ni modifiées, ni supprimées (une correction est une notification
-- complémentaire).
CREATE OR REPLACE FUNCTION rgp_violation_notification_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION 'rgp_violation_notification est en ajout seul (% interdit)', TG_OP
        USING ERRCODE = '42501';
END;
$$;
CREATE OR REPLACE TRIGGER rgp_violation_notification_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON rgp_violation_notification
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_notification_ajout_seul();
CREATE OR REPLACE TRIGGER rgp_violation_notification_sans_truncate_trg
    BEFORE TRUNCATE ON rgp_violation_notification
    FOR EACH STATEMENT EXECUTE FUNCTION rgp_violation_notification_ajout_seul();

-- Garde de la violation : jamais supprimée (art. 33.5, conservation `rgpd.violation`) ; numéro, origine, prise de
-- connaissance et échéance figés ; une notification CNIL posée ne change plus ; **une violation close ne change plus**
-- (le registre est une preuve).
CREATE OR REPLACE FUNCTION rgp_violation_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_violation : suppression interdite (registre, art. 33.5)' USING ERRCODE = '42501';
    END IF;
    IF OLD.etat = 'close' THEN
        RAISE EXCEPTION 'rgp_violation : une violation close ne se modifie plus' USING ERRCODE = '42501';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.annee IS DISTINCT FROM OLD.annee
       OR NEW.numero IS DISTINCT FROM OLD.numero
       OR NEW.origine IS DISTINCT FROM OLD.origine
       OR NEW.plateforme_violation_id IS DISTINCT FROM OLD.plateforme_violation_id
       OR NEW.source_code IS DISTINCT FROM OLD.source_code
       OR NEW.source_ref IS DISTINCT FROM OLD.source_ref
       OR NEW.declaree_par_compte_id IS DISTINCT FROM OLD.declaree_par_compte_id
       OR NEW.connue_le IS DISTINCT FROM OLD.connue_le
       OR NEW.echeance_cnil_le IS DISTINCT FROM OLD.echeance_cnil_le
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'rgp_violation : numéro, origine, déclarant, prise de connaissance et échéance sont figés'
            USING ERRCODE = '42501';
    END IF;
    IF OLD.notifiee_cnil_le IS NOT NULL AND (
           NEW.notifiee_cnil_le IS DISTINCT FROM OLD.notifiee_cnil_le
           OR NEW.reference_cnil IS DISTINCT FROM OLD.reference_cnil
           OR NEW.retard_motif IS DISTINCT FROM OLD.retard_motif) THEN
        RAISE EXCEPTION 'rgp_violation : la notification à la CNIL est figée (complément : notification complémentaire)'
            USING ERRCODE = '42501';
    END IF;
    IF OLD.personnes_informees_le IS NOT NULL AND (
           NEW.personnes_informees_le IS DISTINCT FROM OLD.personnes_informees_le
           OR NEW.personnes_informees_nombre IS DISTINCT FROM OLD.personnes_informees_nombre) THEN
        RAISE EXCEPTION 'rgp_violation : l''information des personnes est figée' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_violation_garde_trg ON rgp_violation;
CREATE TRIGGER rgp_violation_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_violation
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_garde();

-- Les lignes filles d'une violation close ne bougent plus (catégories, mesures, personnes concernées).
CREATE OR REPLACE FUNCTION rgp_violation_fille_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_etab UUID;
    v_violation UUID;
BEGIN
    IF TG_OP = 'DELETE' THEN
        v_etab := OLD.etablissement_id;
        v_violation := OLD.violation_id;
    ELSE
        v_etab := NEW.etablissement_id;
        v_violation := NEW.violation_id;
    END IF;
    IF EXISTS (SELECT 1 FROM public.rgp_violation v
                WHERE v.etablissement_id = v_etab AND v.id = v_violation AND v.etat = 'close') THEN
        RAISE EXCEPTION '% : la violation est close', TG_TABLE_NAME USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_violation_categorie_garde_trg ON rgp_violation_categorie;
CREATE TRIGGER rgp_violation_categorie_garde_trg
    BEFORE INSERT OR UPDATE OR DELETE ON rgp_violation_categorie
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_fille_garde();
DROP TRIGGER IF EXISTS rgp_violation_mesure_garde_trg ON rgp_violation_mesure;
CREATE TRIGGER rgp_violation_mesure_garde_trg
    BEFORE INSERT OR UPDATE OR DELETE ON rgp_violation_mesure
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_fille_garde();
DROP TRIGGER IF EXISTS rgp_violation_personne_garde_trg ON rgp_violation_personne;
CREATE TRIGGER rgp_violation_personne_garde_trg
    BEFORE INSERT OR UPDATE OR DELETE ON rgp_violation_personne
    FOR EACH ROW EXECUTE FUNCTION rgp_violation_fille_garde();

-- ─────────────────────── Propagation d'une violation de plateforme (R-RG36) ───────────────────────
CREATE TABLE IF NOT EXISTS rgp_violation_plateforme_etab (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    violation_plateforme_id UUID NOT NULL REFERENCES rgp_violation_plateforme (id) ON DELETE RESTRICT,
    -- La violation créée au registre de l'établissement (origine `plateforme`).
    violation_id UUID NOT NULL,
    -- Éléments propres à CET établissement (ses comptes, ses données) : jamais ceux d'un autre.
    elements TEXT NOT NULL CHECK (length(btrim(elements)) BETWEEN 3 AND 4000),
    personnes_nombre INT CHECK (personnes_nombre IS NULL OR personnes_nombre >= 0),
    informe_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_violation_plateforme_etab_uq UNIQUE (etablissement_id, violation_plateforme_id),
    CONSTRAINT rgp_violation_plateforme_etab_violation_fk FOREIGN KEY (etablissement_id, violation_id)
        REFERENCES rgp_violation (etablissement_id, id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY[
    'rgp_violation_compteur',
    'rgp_violation',
    'rgp_violation_categorie',
    'rgp_violation_mesure',
    'rgp_violation_notification',
    'rgp_violation_personne',
    'rgp_violation_etape',
    'rgp_violation_plateforme_etab']);
