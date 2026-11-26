-- 2545 — Socle 27, palier G3b + G4 (lot 1) : **extrait assemblé**, sujets d'une demande, réglages RGPD.
-- Réf. : 27_RGPD_ARCHIVES.md §9.4, §17.4, §26.6 (RGP-302), R-RG14 à R-RG19, R-RG40 ; consigne du palier (D-069).
-- Migration **additive et rejouable**, sans JSON, sans tableau, sans BYTEA (D-036, E1–E14).
-- Règle 38 : placée à la FIN du tableau de `migrate.rs` ; chaque fonction créée ici porte
-- `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`.
--
-- **Le contenu d'un extrait n'est jamais stocké en base** (§17.4) : il est reconstruit à la demande depuis les
-- fournisseurs des socles ; la base ne garde que la structure (sections, statuts, nombres de lignes),
-- l'**empreinte** SHA-256 de chaque section (preuve qu'on relit ce qu'on a assemblé) et les décisions humaines.
-- `rgp_requisition` n'est PAS créée ici (réquisitions = G10, 2553 : pas de table sans usage).
-- 2556 garde `cons_visa_exclusion` et `cons_categorie.source_ligne` (`rgp_demande_sujet` en est sortie).

-- ─────────────────────── Réglages RGPD de l'établissement (une ligne par établissement) ───────────────────────
-- Colonnes **typées**, valeur par défaut prudente (D-069) ; la direction les règle, chaque changement est
-- gardé dans `rgp_reglage_journal` (ajout seul). L'absence de ligne = valeurs par défaut (le code les lit ainsi).
CREATE TABLE IF NOT EXISTS rgp_reglage (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Pf1 : plafond des dépôts publics sans compte par jour et par établissement (anti-spam, 30 par défaut).
    plafond_depots_publics_jour INT NOT NULL DEFAULT 30
        CHECK (plafond_depots_publics_jour BETWEEN 1 AND 1000),
    -- Lot G3b : durée de mise à disposition de l'extrait remis en ligne (7 jours, puis objets supprimés).
    extrait_disponible_jours INT NOT NULL DEFAULT 7
        CHECK (extrait_disponible_jours BETWEEN 1 AND 60),
    modifie_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rgp_reglage_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    reglage TEXT NOT NULL CHECK (reglage ~ '^[a-z0-9_]{2,60}$'),
    ancienne_valeur TEXT CHECK (ancienne_valeur IS NULL OR length(ancienne_valeur) <= 60),
    nouvelle_valeur TEXT NOT NULL CHECK (length(nouvelle_valeur) <= 60),
    par_compte_id UUID REFERENCES ent_user (id),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS rgp_reglage_journal_idx ON rgp_reglage_journal (etablissement_id, reglage, le DESC);

CREATE OR REPLACE FUNCTION rgp_reglage_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION 'rgp_reglage_journal est en ajout seul (% interdit)', TG_OP USING ERRCODE = '42501';
END;
$$;
CREATE OR REPLACE TRIGGER rgp_reglage_journal_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON rgp_reglage_journal
    FOR EACH ROW EXECUTE FUNCTION rgp_reglage_journal_ajout_seul();
CREATE OR REPLACE TRIGGER rgp_reglage_journal_sans_truncate_trg
    BEFORE TRUNCATE ON rgp_reglage_journal
    FOR EACH STATEMENT EXECUTE FUNCTION rgp_reglage_journal_ajout_seul();

-- ─────────────────────── Sujets d'une demande (RGP-302 : fratrie) ───────────────────────
-- `rgp_demande.sujet_*` reste le sujet principal ; chaque sujet (principal compris) a une ligne ici dès
-- qu'un extrait existe ou qu'un sujet est ajouté. Un extrait par sujet, une réponse commune.
CREATE TABLE IF NOT EXISTS rgp_demande_sujet (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne', 'agent')),
    sujet_id UUID NOT NULL,
    principal BOOLEAN NOT NULL DEFAULT false,
    -- Qualité du demandeur **pour ce sujet**, vérifiée par le socle 4 à la date de réception (même grille que
    -- le sujet principal). `restreinte` / `absente` / `non_verifiable` : l'extrait de ce sujet attend la direction.
    qualite_constatee TEXT NOT NULL DEFAULT 'non_verifiable'
        CHECK (qualite_constatee IN ('confirmee', 'confirmee_sans_autorite', 'restreinte', 'absente',
                                     'non_verifiable', 'sans_objet')),
    ajoute_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_demande_sujet_uq UNIQUE (etablissement_id, demande_id, sujet_type, sujet_id),
    CONSTRAINT rgp_demande_sujet_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS rgp_demande_sujet_etab_id_uq ON rgp_demande_sujet (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS rgp_demande_sujet_principal_uq
    ON rgp_demande_sujet (etablissement_id, demande_id) WHERE principal;

-- ─────────────────────── L'extrait (une ligne par sujet et par version) ───────────────────────
CREATE TABLE IF NOT EXISTS rgp_extrait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne', 'agent')),
    sujet_id UUID NOT NULL,
    version INT NOT NULL CHECK (version >= 1),
    -- Période demandée (la demande peut la borner) : copiée de la demande à l'assemblage.
    periode_du DATE,
    periode_au DATE,
    assemble_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    assemble_par_compte_id UUID REFERENCES ent_user (id),
    -- R-RG18 : faux dès qu'une section est en échec ou non couverte. Un extrait incomplet ne se remet
    -- jamais comme complet ; sa remise exige une décision motivée de la direction, section par section.
    complet BOOLEAN NOT NULL,
    nb_sections INT NOT NULL CHECK (nb_sections >= 0),
    nb_lignes INT NOT NULL CHECK (nb_lignes >= 0),
    -- Empreinte globale : SHA-256 des empreintes de section, dans l'ordre des fournisseurs.
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    -- Périmé : remplacé par une version plus récente, ou contenu changé depuis l'assemblage relu.
    perime_le TIMESTAMPTZ,
    perime_motif TEXT CHECK (perime_motif IS NULL OR perime_motif IN ('remplace', 'empreinte_changee')),
    -- Relecture du DPO (RGP-152) : seule une demande qu'on lui **soumet** lui ouvre l'extrait (§12.2).
    soumis_dpo_le TIMESTAMPTZ,
    soumis_dpo_par_compte_id UUID REFERENCES ent_user (id),
    dpo_avis_le TIMESTAMPTZ,
    dpo_avis_par_compte_id UUID REFERENCES ent_user (id),
    dpo_avis_code TEXT CHECK (dpo_avis_code IS NULL OR dpo_avis_code IN ('favorable', 'reserves')),
    dpo_avis_texte TEXT CHECK (dpo_avis_texte IS NULL OR length(btrim(dpo_avis_texte)) BETWEEN 3 AND 4000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_extrait_version_uq UNIQUE (etablissement_id, demande_id, sujet_type, sujet_id, version),
    CONSTRAINT rgp_extrait_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT rgp_extrait_periode CHECK (periode_du IS NULL OR periode_au IS NULL OR periode_au >= periode_du),
    CONSTRAINT rgp_extrait_perime_coherent CHECK ((perime_le IS NULL) = (perime_motif IS NULL)),
    CONSTRAINT rgp_extrait_dpo_coherent CHECK (
        (soumis_dpo_le IS NULL) = (soumis_dpo_par_compte_id IS NULL)
        AND ((dpo_avis_le IS NULL) = (dpo_avis_code IS NULL))
        AND (dpo_avis_le IS NULL OR soumis_dpo_le IS NOT NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS rgp_extrait_etab_id_uq ON rgp_extrait (etablissement_id, id);
CREATE INDEX IF NOT EXISTS rgp_extrait_demande_idx ON rgp_extrait (etablissement_id, demande_id, version DESC);

-- Rattachement d'un sujet à son extrait courant (la dernière version non périmée) : posé par le code.
ALTER TABLE rgp_demande_sujet ADD COLUMN IF NOT EXISTS extrait_courant_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'rgp_demande_sujet_extrait_fk') THEN
        ALTER TABLE rgp_demande_sujet ADD CONSTRAINT rgp_demande_sujet_extrait_fk
            FOREIGN KEY (etablissement_id, extrait_courant_id) REFERENCES rgp_extrait (etablissement_id, id);
    END IF;
END
$$;

-- ─────────────────────── Sections de l'extrait ───────────────────────
CREATE TABLE IF NOT EXISTS rgp_extrait_section (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    extrait_id UUID NOT NULL,
    socle TEXT NOT NULL CHECK (socle ~ '^[0-9]{1,3}$'),
    fournisseur_code TEXT NOT NULL CHECK (fournisseur_code ~ '^[a-z_]{2,40}$'),
    -- `exclue` : le domaine est écarté par règle (santé tant que le module est inactif, R-RG29) : listé avec
    -- son motif, sans rendre l'extrait incomplet. `non_couverte` : la donnée existe peut-être mais ce
    -- fournisseur n'est pas écrit pour ce sujet : l'extrait est incomplet.
    statut TEXT NOT NULL CHECK (statut IN ('ok', 'echec', 'vide', 'non_couverte', 'exclue')),
    -- Code neutre (jamais une donnée) : `module_sante_inactif`, `fournisseur_en_echec`, `non_ecrit_pour_ce_sujet`…
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code ~ '^[a-z0-9_]{2,60}$'),
    lignes INT NOT NULL DEFAULT 0 CHECK (lignes >= 0),
    empreinte TEXT CHECK (empreinte IS NULL OR empreinte ~ '^[0-9a-f]{64}$'),
    -- Remise d'un extrait incomplet : décision de la direction (`droits.arbitrer`), motivée par section.
    remise_incomplete_motif TEXT
        CHECK (remise_incomplete_motif IS NULL OR length(btrim(remise_incomplete_motif)) BETWEEN 3 AND 2000),
    remise_incomplete_par_compte_id UUID REFERENCES ent_user (id),
    remise_incomplete_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_extrait_section_uq UNIQUE (etablissement_id, extrait_id, fournisseur_code),
    CONSTRAINT rgp_extrait_section_extrait_fk FOREIGN KEY (etablissement_id, extrait_id)
        REFERENCES rgp_extrait (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT rgp_extrait_section_decision_coherente CHECK (
        (remise_incomplete_motif IS NULL) = (remise_incomplete_le IS NULL)
        AND (remise_incomplete_motif IS NULL) = (remise_incomplete_par_compte_id IS NULL)),
    -- Une section en échec ou non couverte dit pourquoi ; une section ok ou vide a son empreinte.
    CONSTRAINT rgp_extrait_section_motif CHECK (
        statut NOT IN ('echec', 'non_couverte', 'exclue') OR motif_code IS NOT NULL),
    CONSTRAINT rgp_extrait_section_empreinte CHECK (
        statut NOT IN ('ok', 'vide') OR empreinte IS NOT NULL)
);
CREATE UNIQUE INDEX IF NOT EXISTS rgp_extrait_section_etab_id_uq ON rgp_extrait_section (etablissement_id, id);

-- Le contenu et la structure d'une version d'extrait ne changent pas après l'assemblage : seules les décisions
-- humaines (avis du DPO, périmé, remise d'une section incomplète) s'ajoutent. Une section ne se supprime pas.
CREATE OR REPLACE FUNCTION rgp_extrait_section_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_extrait_section : suppression interdite' USING ERRCODE = '42501';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.extrait_id IS DISTINCT FROM OLD.extrait_id
       OR NEW.fournisseur_code IS DISTINCT FROM OLD.fournisseur_code
       OR NEW.statut IS DISTINCT FROM OLD.statut
       OR NEW.lignes IS DISTINCT FROM OLD.lignes
       OR NEW.empreinte IS DISTINCT FROM OLD.empreinte THEN
        RAISE EXCEPTION 'rgp_extrait_section : le contenu assemblé est figé (nouvelle version à la place)'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_extrait_section_garde_trg ON rgp_extrait_section;
CREATE TRIGGER rgp_extrait_section_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_extrait_section
    FOR EACH ROW EXECUTE FUNCTION rgp_extrait_section_garde();

CREATE OR REPLACE FUNCTION rgp_extrait_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_extrait : suppression interdite (conservation rgpd.demande)' USING ERRCODE = '42501';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.demande_id IS DISTINCT FROM OLD.demande_id
       OR NEW.sujet_type IS DISTINCT FROM OLD.sujet_type
       OR NEW.sujet_id IS DISTINCT FROM OLD.sujet_id
       OR NEW.version IS DISTINCT FROM OLD.version
       OR NEW.complet IS DISTINCT FROM OLD.complet
       OR NEW.empreinte IS DISTINCT FROM OLD.empreinte
       OR NEW.nb_sections IS DISTINCT FROM OLD.nb_sections
       OR NEW.nb_lignes IS DISTINCT FROM OLD.nb_lignes
       OR NEW.assemble_le IS DISTINCT FROM OLD.assemble_le THEN
        RAISE EXCEPTION 'rgp_extrait : version, sujet, empreinte et complétude sont figés'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_extrait_garde_trg ON rgp_extrait;
CREATE TRIGGER rgp_extrait_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_extrait
    FOR EACH ROW EXECUTE FUNCTION rgp_extrait_garde();

-- Événements du journal de la demande : l'extrait en ajoute (la contrainte d'origine est élargie, jamais restreinte).
ALTER TABLE rgp_demande_journal DROP CONSTRAINT IF EXISTS rgp_demande_journal_evenement_check;
ALTER TABLE rgp_demande_journal ADD CONSTRAINT rgp_demande_journal_evenement_check CHECK (evenement IN (
    'recue', 'qualifiee', 'identite_demandee', 'identite_verifiee', 'identite_echouee',
    'affectee', 'prolongee', 'droit_ajoute', 'droit_decide', 'titulaires_informes',
    'close', 'refusee', 'rouverte', 'piece_destruction_demandee', 'piece_destruction_en_attente',
    'piece_detruite', 'accuse_envoye', 'echeance_alertee', 'numero_attribue', 'sujet_rapproche',
    'identite_a_arbitrer', 'identite_arbitree', 'decision_sans_identite',
    -- palier G3b + G4
    'sujet_ajoute', 'extrait_assemble', 'extrait_perime', 'extrait_lu', 'extrait_incomplet_arbitre',
    'extrait_soumis_dpo', 'extrait_avis_dpo', 'masquage_decide', 'masquage_ajoute',
    'extrait_remis', 'extrait_telecharge', 'extrait_purge', 'renvoi_trace'));

-- ─────────────────────── Marquages d'un extrait (lot G3b-2) ───────────────────────
-- Un marquage désigne un **élément** de l'extrait (table, ligne ou rang, colonne), jamais une copie de la donnée.
-- Posé par des règles déterministes (`propose_par = 'sys'`, jamais une IA, R-RG15) ou à la main par un agent.
-- `a_decider` tant que personne n'a tranché : aucune remise tant qu'il en reste (RGP-150).
CREATE TABLE IF NOT EXISTS rgp_extrait_masquage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    extrait_id UUID NOT NULL,
    fournisseur_code TEXT NOT NULL CHECK (fournisseur_code ~ '^[a-z][a-z0-9_]{1,39}$'),
    table_nom TEXT NOT NULL CHECK (table_nom ~ '^[a-z][a-z0-9_]{1,62}$'),
    ligne_id UUID,
    rang INT NOT NULL DEFAULT 0 CHECK (rang >= 0),
    colonne TEXT CHECK (colonne IS NULL OR colonne ~ '^[a-z][a-z0-9_]{0,62}$'),
    raison TEXT NOT NULL CHECK (raison IN (
        'autre_parent', 'adresse_protegee', 'parent_donnees_propres', 'autre_eleve', 'redacteur',
        'signalement', 'piece_tiers', 'manuel')),
    propose_par TEXT NOT NULL CHECK (propose_par IN ('sys', 'agent')),
    decision TEXT NOT NULL DEFAULT 'a_decider' CHECK (decision IN ('a_decider', 'masque', 'leve')),
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 3 AND 1000),
    decide_par_compte_id UUID REFERENCES ent_user (id),
    decide_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Lever un marquage exige un motif écrit.
    CONSTRAINT rgp_extrait_masquage_leve_motive CHECK (decision <> 'leve' OR motif IS NOT NULL),
    CONSTRAINT rgp_extrait_masquage_decision_tracee CHECK (
        (decision = 'a_decider' AND decide_le IS NULL) OR (decision <> 'a_decider' AND decide_le IS NOT NULL)),
    CONSTRAINT rgp_extrait_masquage_extrait_fk FOREIGN KEY (etablissement_id, extrait_id)
        REFERENCES rgp_extrait (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS rgp_extrait_masquage_extrait_idx
    ON rgp_extrait_masquage (etablissement_id, extrait_id, decision);

-- ─────────────────────── Remise, téléchargements, clôture (lot G3b-3) ───────────────────────
-- Les fichiers remis (PDF, CSV) sont **scellés** au repos (SecretBox, usage dérivé « extrait-rgpd:v1 », AAD liée à
-- l'extrait, à sa version et au fichier). Seules les métadonnées et les empreintes SHA-256 sont en base. Après la
-- mise à disposition (7 jours par défaut), l'objet est supprimé ; « purgé » n'est écrit qu'après confirmation.
CREATE TABLE IF NOT EXISTS rgp_extrait_fichier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    extrait_id UUID NOT NULL,
    nom TEXT NOT NULL CHECK (nom ~ '^[a-z0-9][a-z0-9_.-]{0,80}$'),
    type TEXT NOT NULL CHECK (type IN ('pdf', 'csv')),
    cle_objet TEXT NOT NULL CHECK (length(cle_objet) BETWEEN 10 AND 300),
    octets INT NOT NULL CHECK (octets >= 0),
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    empreinte_scelle TEXT NOT NULL CHECK (empreinte_scelle ~ '^[0-9a-f]{64}$'),
    disponible_jusqu_le TIMESTAMPTZ NOT NULL,
    purge_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_extrait_fichier_nom_uq UNIQUE (etablissement_id, extrait_id, nom),
    CONSTRAINT rgp_extrait_fichier_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT rgp_extrait_fichier_extrait_fk FOREIGN KEY (etablissement_id, extrait_id)
        REFERENCES rgp_extrait (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS rgp_extrait_fichier_purge_idx
    ON rgp_extrait_fichier (etablissement_id, disponible_jusqu_le) WHERE purge_le IS NULL;

CREATE OR REPLACE FUNCTION rgp_extrait_fichier_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'rgp_extrait_fichier : suppression interdite' USING ERRCODE = '42501';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.extrait_id IS DISTINCT FROM OLD.extrait_id OR NEW.nom IS DISTINCT FROM OLD.nom
       OR NEW.cle_objet IS DISTINCT FROM OLD.cle_objet OR NEW.empreinte IS DISTINCT FROM OLD.empreinte
       OR NEW.empreinte_scelle IS DISTINCT FROM OLD.empreinte_scelle
       OR NEW.disponible_jusqu_le IS DISTINCT FROM OLD.disponible_jusqu_le
       OR (OLD.purge_le IS NOT NULL AND NEW.purge_le IS DISTINCT FROM OLD.purge_le) THEN
        RAISE EXCEPTION 'rgp_extrait_fichier : seule la date de purge s''écrit, une fois' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS rgp_extrait_fichier_garde_trg ON rgp_extrait_fichier;
CREATE TRIGGER rgp_extrait_fichier_garde_trg
    BEFORE UPDATE OR DELETE ON rgp_extrait_fichier
    FOR EACH ROW EXECUTE FUNCTION rgp_extrait_fichier_garde();

-- Une remise par extrait. En ligne : sur le compte du demandeur dans cet établissement, jamais sur une adresse
-- fournie par la demande. En main propre : émargement (nom saisi, pièce vue, gestionnaire).
CREATE TABLE IF NOT EXISTS rgp_remise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    extrait_id UUID NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('en_ligne', 'main_propre')),
    remis_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    remis_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    destinataire_compte_id UUID REFERENCES ent_user (id),
    empreinte_extrait TEXT NOT NULL CHECK (empreinte_extrait ~ '^[0-9a-f]{64}$'),
    emargement_nom TEXT CHECK (emargement_nom IS NULL OR length(btrim(emargement_nom)) BETWEEN 2 AND 200),
    emargement_piece_vue TEXT CHECK (emargement_piece_vue IS NULL OR length(btrim(emargement_piece_vue)) BETWEEN 2 AND 100),
    emargement_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_remise_extrait_uq UNIQUE (etablissement_id, extrait_id),
    CONSTRAINT rgp_remise_extrait_fk FOREIGN KEY (etablissement_id, extrait_id)
        REFERENCES rgp_extrait (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT rgp_remise_mode_coherent CHECK (
        (mode = 'en_ligne' AND destinataire_compte_id IS NOT NULL AND emargement_nom IS NULL)
        OR (mode = 'main_propre' AND emargement_nom IS NOT NULL AND emargement_piece_vue IS NOT NULL
            AND emargement_le IS NOT NULL AND destinataire_compte_id IS NULL))
);

-- Défis de passkey récente : un par geste, lié à l'empreinte de l'extrait et à l'intention (remettre, télécharger).
CREATE TABLE IF NOT EXISTS rgp_defi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    extrait_id UUID NOT NULL,
    usage TEXT NOT NULL CHECK (usage IN ('remettre', 'telecharger')),
    compte_id UUID NOT NULL REFERENCES ent_user (id),
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    intention TEXT NOT NULL CHECK (length(intention) BETWEEN 10 AND 300),
    etat_webauthn TEXT NOT NULL,
    expire_le TIMESTAMPTZ NOT NULL,
    consomme_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_defi_extrait_fk FOREIGN KEY (etablissement_id, extrait_id)
        REFERENCES rgp_extrait (etablissement_id, id) ON DELETE RESTRICT
);

-- Journal des téléchargements : ajout seul.
CREATE TABLE IF NOT EXISTS rgp_telechargement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fichier_id UUID NOT NULL,
    compte_id UUID NOT NULL REFERENCES ent_user (id),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_telechargement_fichier_fk FOREIGN KEY (etablissement_id, fichier_id)
        REFERENCES rgp_extrait_fichier (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS rgp_telechargement_fichier_idx ON rgp_telechargement (etablissement_id, fichier_id);

-- Clôture d'une demande (RGP-166 ; une ligne par clôture, une demande rouverte puis reclose en ajoute une) : éléments remis par **empreinte** (jamais les données), refus partiels motivés.
CREATE TABLE IF NOT EXISTS rgp_cloture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    elements JSONB NOT NULL DEFAULT '[]'::jsonb,
    refus JSONB NOT NULL DEFAULT '[]'::jsonb,
    cloture_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    cloture_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_cloture_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT
);

-- Renvois tracés : rectification vers le socle propriétaire (RGP-156), retrait de consentement relié au socle 18
-- (RGP-159), CADA d'un tiers : document agrégé ou occulté (RGP-316).
CREATE TABLE IF NOT EXISTS rgp_renvoi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('rectification', 'retrait_consentement', 'cada_tiers')),
    cible TEXT NOT NULL CHECK (length(btrim(cible)) BETWEEN 2 AND 100),
    texte TEXT NOT NULL CHECK (length(btrim(texte)) BETWEEN 3 AND 2000),
    cree_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_renvoi_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS rgp_renvoi_demande_idx ON rgp_renvoi (etablissement_id, demande_id);

CREATE OR REPLACE FUNCTION rgp_g3b_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP USING ERRCODE = '42501';
END;
$$;
DROP TRIGGER IF EXISTS rgp_telechargement_ajout_seul_trg ON rgp_telechargement;
CREATE TRIGGER rgp_telechargement_ajout_seul_trg BEFORE UPDATE OR DELETE ON rgp_telechargement
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();
DROP TRIGGER IF EXISTS rgp_remise_ajout_seul_trg ON rgp_remise;
CREATE TRIGGER rgp_remise_ajout_seul_trg BEFORE UPDATE OR DELETE ON rgp_remise
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();
DROP TRIGGER IF EXISTS rgp_cloture_ajout_seul_trg ON rgp_cloture;
CREATE TRIGGER rgp_cloture_ajout_seul_trg BEFORE UPDATE OR DELETE ON rgp_cloture
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();
DROP TRIGGER IF EXISTS rgp_renvoi_ajout_seul_trg ON rgp_renvoi;
CREATE TRIGGER rgp_renvoi_ajout_seul_trg BEFORE UPDATE OR DELETE ON rgp_renvoi
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'rgp_reglage',
    'rgp_reglage_journal',
    'rgp_demande_sujet',
    'rgp_extrait',
    'rgp_extrait_section',
    'rgp_extrait_masquage',
    'rgp_extrait_fichier',
    'rgp_remise',
    'rgp_defi',
    'rgp_telechargement',
    'rgp_cloture',
    'rgp_renvoi']);
