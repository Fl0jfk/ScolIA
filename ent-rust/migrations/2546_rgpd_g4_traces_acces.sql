-- 2546 — Socle 27, palier G3b + G4 (lot 4) : **traces d'accès séparées** du journal métier (§11, §17.5).
-- Réf. : 27_RGPD_ARCHIVES.md §11.1 à §11.6, §17.5 ; RGP-191, RGP-192, RGP-196, RGP-198 ; R-RG8, R-RG28, R-RG29,
-- R-RG30, R-RG31, R-RG46 ; S-541, S-542, S-543, S-554 ; consigne du palier (D-069).
-- Migration **additive et rejouable**, sans JSON, sans tableau, sans BYTEA (D-036).
-- Règle 38 : placée à la FIN du tableau de `migrate.rs` ; chaque fonction porte
-- `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`.
--
-- **Premier usage du partitionnement dans ScolIA** : `acc_trace` est partitionnée par MOIS sur `le`. Une partition
-- n'hérite PAS de la RLS du parent (une requête qui vise la partition directement ne passe pas par la politique du
-- parent) : la fonction `acc_trace_assurer_partition` pose donc la RLS stricte, FORCE et la politique d'établissement
-- sur CHAQUE partition qu'elle crée, ainsi qu'un déclencheur TRUNCATE (un déclencheur d'instruction du parent ne se
-- déclenche pas pour un TRUNCATE visant une partition). L'ajout seul (ni UPDATE ni DELETE) est un déclencheur de ligne
-- du parent, cloné sur les partitions. La **purge** se fait par DROP de la partition (lot 5, RGP-198) : un
-- DROP ne déclenche ni DELETE ni TRUNCATE, aucune dérogation n'est donc nécessaire dans les déclencheurs.
--
-- Fonction et service de l'agent sont **figés à l'écriture** (R-RG31) : un changement de poste ne réécrit jamais
-- l'historique des consultations. Le nom de l'agent n'est PAS dans la trace affichée aux familles (§11.3, S-549 :
-- non) ; l'identifiant du compte est gardé pour la direction et le DPO (contrôles, alertes, lot 5).

-- ─────────────────────── Réglages de l'établissement (colonnes ajoutées à `rgp_reglage`, 2545) ───────────────────────
ALTER TABLE rgp_reglage
    -- S-541 : durée de conservation des traces d'accès, en mois (12 par défaut ; la purge se fait par partition mensuelle,
    -- la partition d'un mois n'est supprimée qu'une fois la plus longue durée des établissements écoulée).
    ADD COLUMN IF NOT EXISTS acces_trace_mois INT NOT NULL DEFAULT 12
        CHECK (acces_trace_mois BETWEEN 12 AND 60),
    -- S-543 : rafale de consultations hors périmètre (nombre dans la fenêtre, fenêtre en minutes).
    ADD COLUMN IF NOT EXISTS acces_rafale_seuil INT NOT NULL DEFAULT 15
        CHECK (acces_rafale_seuil BETWEEN 3 AND 500),
    ADD COLUMN IF NOT EXISTS acces_rafale_fenetre_min INT NOT NULL DEFAULT 10
        CHECK (acces_rafale_fenetre_min BETWEEN 1 AND 240),
    -- S-554 : seuil de consultations de fiches distinctes par jour, par famille de rôle (alerte, lot 5).
    ADD COLUMN IF NOT EXISTS acces_seuil_direction INT NOT NULL DEFAULT 300
        CHECK (acces_seuil_direction BETWEEN 10 AND 5000),
    ADD COLUMN IF NOT EXISTS acces_seuil_enseignant INT NOT NULL DEFAULT 80
        CHECK (acces_seuil_enseignant BETWEEN 10 AND 5000),
    ADD COLUMN IF NOT EXISTS acces_seuil_vie_scolaire INT NOT NULL DEFAULT 200
        CHECK (acces_seuil_vie_scolaire BETWEEN 10 AND 5000),
    ADD COLUMN IF NOT EXISTS acces_seuil_administratif INT NOT NULL DEFAULT 150
        CHECK (acces_seuil_administratif BETWEEN 10 AND 5000);

-- S-542 : liste des motifs proposés pour une consultation hors périmètre, par établissement. L'absence de ligne =
-- liste par défaut du code ; une ligne ne sert qu'à renommer, réordonner ou désactiver un motif.
CREATE TABLE IF NOT EXISTS acc_motif (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 2 AND 120),
    rang SMALLINT NOT NULL DEFAULT 0 CHECK (rang BETWEEN 0 AND 99),
    actif BOOLEAN NOT NULL DEFAULT true,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, code)
);

-- ─────────────────────── Classement d'une action du journal en nature / section / affichage ───────────────────────
-- Source unique (reprise des anciennes traces ET écriture courante) : le code Rust appelle ces fonctions.
-- Sections = codes GÉNÉRIQUES : jamais le nom d'une donnée protégée (§11.4, R-RG46).
CREATE OR REPLACE FUNCTION acc_trace_section(p_action TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT CASE
        WHEN p_action IN ('fiche_protegee', 'synthese_protegee', 'pastilles_sensibles') THEN 'section_protegee'
        WHEN p_action IN ('etat_civil', 'identite_historique_complet', 'identite_historique', 'identifiants',
                          'identifiants_qualite') THEN 'identite'
        WHEN p_action = 'coordonnees' THEN 'coordonnees'
        WHEN p_action = 'scolarite' THEN 'scolarite'
        WHEN p_action LIKE 'internat%' THEN 'internat'
        WHEN p_action = 'extrait_droits' OR p_action LIKE 'acces_traces%' THEN 'dossier_droits'
        WHEN p_action LIKE 'sante%' OR p_action LIKE 'psy%' OR p_action LIKE 'infirmerie%' THEN 'sante'
        WHEN p_action LIKE 'liste_%' THEN 'liste'
        ELSE 'dossier'
    END
$$;

CREATE OR REPLACE FUNCTION acc_trace_nature(p_action TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT CASE
        WHEN p_action LIKE 'liste_%' THEN 'liste'
        WHEN p_action = 'extrait_droits' THEN 'extrait'
        ELSE 'consultation'
    END
$$;

-- R-RG29 : santé et suivi psychologique ne sont jamais montrés ; R-RG28 / extrait des droits : réservés à la
-- conformité ; une liste n'est pas une consultation du dossier d'un élève.
CREATE OR REPLACE FUNCTION acc_trace_affichable(p_section TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT p_section NOT IN ('sante', 'dossier_droits', 'liste')
$$;

-- R-RG46 : une trace est « protégée » quand elle porte sur une section protégée, ou sur les coordonnées d'un élève dont
-- la famille compte une protection (adresse mise de côté, « ne pas solliciter » d'un responsable). Elle n'est alors
-- jamais montrée à un parent, hormis la personne protégée elle-même (page « Consultations du dossier »).
CREATE OR REPLACE FUNCTION acc_trace_protegee(p_etab UUID, p_eleve UUID, p_section TEXT) RETURNS BOOLEAN
LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT p_section = 'section_protegee' OR (p_section = 'coordonnees' AND p_eleve IS NOT NULL AND (
        EXISTS (SELECT 1 FROM public.ent_eleve_confidentialite c
                 WHERE c.etablissement_id = p_etab AND c.eleve_id = p_eleve AND c.leve_le IS NULL)
        OR EXISTS (SELECT 1 FROM public.ent_adresse_protegee a
                    JOIN public.ent_personne p ON p.id = a.personne_id AND p.etablissement_id = a.etablissement_id
                   WHERE a.etablissement_id = p_etab
                     AND (p.eleve_id = p_eleve
                          OR p.user_id IN (SELECT l.user_id FROM public.ent_eleve_lien l
                                            WHERE l.etablissement_id = p_etab AND l.eleve_id = p_eleve
                                              AND l.user_id IS NOT NULL)))
        OR EXISTS (SELECT 1 FROM public.ent_personne_reserve r
                    JOIN public.ent_personne p ON p.id = r.personne_id AND p.etablissement_id = r.etablissement_id
                   WHERE r.etablissement_id = p_etab AND r.ne_pas_solliciter
                     AND (p.eleve_id = p_eleve
                          OR p.user_id IN (SELECT l.user_id FROM public.ent_eleve_lien l
                                            WHERE l.etablissement_id = p_etab AND l.eleve_id = p_eleve
                                              AND l.user_id IS NOT NULL)))
    ))
$$;

-- Section **finale** : les coordonnées d'un élève protégé portent le code générique `coordonnees_protegees`.
CREATE OR REPLACE FUNCTION acc_trace_section_finale(p_action TEXT, p_etab UUID, p_eleve UUID) RETURNS TEXT
LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT CASE WHEN public.acc_trace_section(p_action) = 'coordonnees'
                 AND public.acc_trace_protegee(p_etab, p_eleve, 'coordonnees')
                THEN 'coordonnees_protegees' ELSE public.acc_trace_section(p_action) END
$$;

-- ─────────────────────── La table des traces, partitionnée par mois ───────────────────────
CREATE TABLE IF NOT EXISTS acc_trace (
    id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    le TIMESTAMPTZ NOT NULL,
    -- Compte de l'agent : jamais montré aux familles (§11.3) ; sans clé étrangère (la trace survit au compte).
    acteur_compte_id UUID NOT NULL,
    acteur_role TEXT NOT NULL CHECK (length(acteur_role) BETWEEN 1 AND 40),
    -- Fonction et service **figés** (R-RG31). Le service est la famille de la fonction (direction, enseignement,
    -- éducation, administration…) : ScolIA ne tient pas d'organigramme plus fin.
    fonction_code TEXT NOT NULL CHECK (length(fonction_code) BETWEEN 1 AND 60),
    fonction_libelle TEXT NOT NULL CHECK (length(btrim(fonction_libelle)) BETWEEN 1 AND 120),
    service_code TEXT NOT NULL CHECK (length(service_code) BETWEEN 1 AND 40),
    service_libelle TEXT NOT NULL CHECK (length(btrim(service_libelle)) BETWEEN 1 AND 120),
    nature TEXT NOT NULL CHECK (nature IN ('consultation', 'liste', 'extrait', 'export')),
    section TEXT NOT NULL CHECK (section ~ '^[a-z][a-z_]{1,39}$'),
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 120),
    sujet_eleve_id UUID,
    -- Dossier d'un agent consulté (page « Consultations du dossier » du personnel, son propre dossier).
    sujet_personne_id UUID,
    -- Hors périmètre (RGP-191) : motif (liste S-542) et périmètre temporaire éventuel (RGP-310).
    hors_perimetre BOOLEAN NOT NULL DEFAULT false,
    perimetre_temporaire TEXT CHECK (perimetre_temporaire IS NULL
        OR perimetre_temporaire IN ('remplacement', 'conseil_classe', 'sortie')),
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code ~ '^[a-z][a-z0-9_]{1,39}$'),
    -- Montrée à la famille / à l'élève majeur / à l'agent ? (R-RG29 : jamais santé ni suivi psychologique.)
    affichable BOOLEAN NOT NULL DEFAULT true,
    -- Fiche protégée (R-RG46) : jamais montrée à un parent, hormis la personne protégée.
    protegee BOOLEAN NOT NULL DEFAULT false,
    -- Support de l'éditeur : affiché sans fonction ni service, avec la seule mention « intervention autorisée le … ».
    support_editeur BOOLEAN NOT NULL DEFAULT false,
    support_intervention_le DATE,
    CONSTRAINT acc_trace_sujet_chk CHECK (sujet_eleve_id IS NULL OR sujet_personne_id IS NULL),
    CONSTRAINT acc_trace_support_chk CHECK (NOT support_editeur OR support_intervention_le IS NOT NULL),
    CONSTRAINT acc_trace_hors_perimetre_chk CHECK (hors_perimetre OR motif_code IS NULL),
    PRIMARY KEY (id, le)
) PARTITION BY RANGE (le);

CREATE INDEX IF NOT EXISTS acc_trace_sujet_idx ON acc_trace (etablissement_id, sujet_eleve_id, le DESC);
CREATE INDEX IF NOT EXISTS acc_trace_personne_idx ON acc_trace (etablissement_id, sujet_personne_id, le DESC)
    WHERE sujet_personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS acc_trace_acteur_idx ON acc_trace (etablissement_id, acteur_compte_id, le DESC);

-- Ajout seul : déclencheur de ligne sur le parent (cloné sur chaque partition, existante ou future).
CREATE OR REPLACE TRIGGER acc_trace_ajout_seul_trg BEFORE UPDATE OR DELETE ON acc_trace
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();

-- Création d'une partition mensuelle, avec RLS stricte et ajout seul. Idempotente ; ne fait rien si elle existe.
-- SECURITY DEFINER : le rôle applicatif n'a pas le droit de créer des tables ; le nom est entièrement dérivé d'une date.
CREATE OR REPLACE FUNCTION acc_trace_assurer_partition(p_jour DATE) RETURNS TEXT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_debut DATE := date_trunc('month', p_jour::timestamp)::date;
    v_fin DATE := (date_trunc('month', p_jour::timestamp) + interval '1 month')::date;
    v_nom TEXT := 'acc_trace_' || to_char(date_trunc('month', p_jour::timestamp), 'YYYYMM');
BEGIN
    IF to_regclass('public.' || quote_ident(v_nom)) IS NOT NULL THEN
        RETURN v_nom;
    END IF;
    EXECUTE format('CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.acc_trace FOR VALUES FROM (%L) TO (%L)',
                   v_nom, v_debut::text || ' 00:00:00+00', v_fin::text || ' 00:00:00+00');
    PERFORM public.ent_rls_poser(ARRAY[v_nom]);
    EXECUTE format('CREATE OR REPLACE TRIGGER %I BEFORE TRUNCATE ON public.%I FOR EACH STATEMENT '
                   'EXECUTE FUNCTION public.rgp_g3b_ajout_seul()', v_nom || '_sans_truncate', v_nom);
    RETURN v_nom;
END;
$$;

-- Le mois courant et les deux suivants (créés à l'avance).
CREATE OR REPLACE FUNCTION acc_trace_assurer_partitions(p_jour DATE) RETURNS INT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    i INT;
BEGIN
    FOR i IN 0..2 LOOP
        PERFORM public.acc_trace_assurer_partition((date_trunc('month', p_jour::timestamp) + make_interval(months => i))::date);
    END LOOP;
    RETURN 3;
END;
$$;

SELECT acc_trace_assurer_partitions(CURRENT_DATE);

-- ─────────────────────── Contrôles internes et alertes (lot 5) ───────────────────────
-- Contrôle interne sur un agent (RGP-194, RGP-195, R-RG27) : motif, durée **bornée** (31 jours au plus), clôture.
-- Aucune statistique individuelle, aucun classement (§11.4).
CREATE TABLE IF NOT EXISTS acc_controle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_compte_id UUID NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN ('signalement', 'alerte', 'plainte', 'verification_periodique')),
    motif_texte TEXT NOT NULL CHECK (length(btrim(motif_texte)) BETWEEN 5 AND 1000),
    ouvert_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    ouvert_le TIMESTAMPTZ NOT NULL,
    echeance_le TIMESTAMPTZ NOT NULL,
    clos_par_compte_id UUID REFERENCES ent_user (id),
    clos_le TIMESTAMPTZ,
    issue TEXT CHECK (issue IS NULL OR issue IN ('rien', 'violation', 'transmis')),
    CONSTRAINT acc_controle_duree_chk CHECK (echeance_le > ouvert_le AND echeance_le <= ouvert_le + interval '31 days'),
    CONSTRAINT acc_controle_cloture_chk CHECK ((clos_le IS NULL) = (issue IS NULL) AND (clos_le IS NULL) = (clos_par_compte_id IS NULL)),
    CONSTRAINT acc_controle_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS acc_controle_agent_idx ON acc_controle (etablissement_id, agent_compte_id, ouvert_le DESC);

-- Un contrôle ne se réécrit pas : seule sa clôture (une fois) est permise.
CREATE OR REPLACE FUNCTION acc_controle_garde() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'acc_controle est en ajout seul (DELETE interdit)' USING ERRCODE = '42501';
    END IF;
    IF OLD.clos_le IS NOT NULL THEN
        RAISE EXCEPTION 'contrôle déjà clos' USING ERRCODE = '42501';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.agent_compte_id <> OLD.agent_compte_id
       OR NEW.motif_code <> OLD.motif_code OR NEW.motif_texte <> OLD.motif_texte
       OR NEW.ouvert_par_compte_id <> OLD.ouvert_par_compte_id OR NEW.ouvert_le <> OLD.ouvert_le
       OR NEW.echeance_le <> OLD.echeance_le THEN
        RAISE EXCEPTION 'seule la clôture d''un contrôle est modifiable' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE TRIGGER acc_controle_garde_trg BEFORE UPDATE OR DELETE ON acc_controle
    FOR EACH ROW EXECUTE FUNCTION acc_controle_garde();

-- Alerte déterministe (RGP-196, §11.5) : une seule par règle, agent et fenêtre (idempotence). Jamais notifiée à l'agent.
CREATE TABLE IF NOT EXISTS acc_alerte (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    regle TEXT NOT NULL CHECK (regle IN ('rafale_hors_perimetre', 'seuil_jour', 'adresse_protegee', 'conservation')),
    agent_compte_id UUID NOT NULL,
    fenetre_debut TIMESTAMPTZ NOT NULL,
    fenetre_fin TIMESTAMPTZ NOT NULL,
    nombre INT NOT NULL CHECK (nombre >= 1),
    cree_le TIMESTAMPTZ NOT NULL,
    suite TEXT NOT NULL DEFAULT 'a_traiter' CHECK (suite IN ('a_traiter', 'rien', 'violation', 'controle')),
    suite_par_compte_id UUID REFERENCES ent_user (id),
    suite_le TIMESTAMPTZ,
    controle_id UUID,
    CONSTRAINT acc_alerte_fenetre_chk CHECK (fenetre_fin >= fenetre_debut),
    CONSTRAINT acc_alerte_suite_chk CHECK ((suite = 'a_traiter') = (suite_le IS NULL)),
    CONSTRAINT acc_alerte_unique UNIQUE (etablissement_id, regle, agent_compte_id, fenetre_debut),
    CONSTRAINT acc_alerte_controle_fk FOREIGN KEY (etablissement_id, controle_id)
        REFERENCES acc_controle (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS acc_alerte_a_traiter_idx ON acc_alerte (etablissement_id, cree_le DESC);

CREATE OR REPLACE FUNCTION acc_alerte_garde() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'acc_alerte est en ajout seul (DELETE interdit)' USING ERRCODE = '42501';
    END IF;
    IF OLD.suite <> 'a_traiter' THEN
        RAISE EXCEPTION 'alerte déjà traitée' USING ERRCODE = '42501';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.regle <> OLD.regle
       OR NEW.agent_compte_id <> OLD.agent_compte_id OR NEW.fenetre_debut <> OLD.fenetre_debut
       OR NEW.fenetre_fin <> OLD.fenetre_fin OR NEW.nombre <> OLD.nombre OR NEW.cree_le <> OLD.cree_le THEN
        RAISE EXCEPTION 'seule la suite donnée à une alerte est modifiable' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE TRIGGER acc_alerte_garde_trg BEFORE UPDATE OR DELETE ON acc_alerte
    FOR EACH ROW EXECUTE FUNCTION acc_alerte_garde();

-- ─────────────────────── Registre des purges (plateforme, sans établissement) ───────────────────────
-- Les partitions mensuelles couvrent TOUS les établissements : la purge (RGP-198, lot 5) est une opération de
-- plateforme. `cons_execution` n'existe qu'en G6 (2549) : cette table est reprise alors.
CREATE TABLE IF NOT EXISTS acc_purge (
    id UUID PRIMARY KEY,
    partition_mois DATE NOT NULL CHECK (partition_mois = date_trunc('month', partition_mois::timestamp)::date),
    issue TEXT NOT NULL CHECK (issue IN ('purgee', 'differee')),
    lignes BIGINT NOT NULL CHECK (lignes >= 0),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 200),
    le TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS acc_purge_mois_idx ON acc_purge (partition_mois, le DESC);
CREATE OR REPLACE TRIGGER acc_purge_ajout_seul_trg BEFORE UPDATE OR DELETE ON acc_purge
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();

-- ─────────────────────── Reprise des traces de consultation existantes (copie, rien n'est supprimé) ───────────────────────
-- Le journal `ent_journal_modif` n'est pas modifié (2559 reste réservée) ; les lignes sont copiées avec **le même
-- identifiant** : la reprise se rejoue sans doublon, et l'écriture courante (double écriture) réutilise l'identifiant
-- de la ligne de journal. Seules les traces de la durée de conservation par défaut (12 mois) sont reprises.
DO $$
DECLARE
    v_min DATE;
    v_mois DATE;
BEGIN
    SELECT date_trunc('month', min(created_at::timestamp))::date INTO v_min
      FROM public.ent_journal_modif
     WHERE domaine = 'consultation' AND actor_user_id IS NOT NULL
       AND created_at >= date_trunc('month', now()) - interval '12 months';
    IF v_min IS NULL THEN
        RETURN;
    END IF;
    v_mois := v_min;
    WHILE v_mois <= CURRENT_DATE LOOP
        PERFORM public.acc_trace_assurer_partition(v_mois);
        v_mois := (v_mois + interval '1 month')::date;
    END LOOP;
    INSERT INTO public.acc_trace (id, etablissement_id, le, acteur_compte_id, acteur_role, fonction_code,
                                  fonction_libelle, service_code, service_libelle, nature, section, action,
                                  sujet_eleve_id, hors_perimetre, affichable, protegee)
    SELECT j.id, j.etablissement_id, j.created_at, j.actor_user_id,
           COALESCE(NULLIF(j.payload ->> 'role', ''), 'inconnu'),
           COALESCE(f.code, 'a_preciser'), COALESCE(f.libelle, 'Fonction non renseignée'),
           COALESCE(f.famille, 'autre'),
           CASE COALESCE(f.famille, 'autre')
               WHEN 'direction' THEN 'Direction' WHEN 'enseignement' THEN 'Enseignement'
               WHEN 'education' THEN 'Éducation' WHEN 'administration' THEN 'Administration'
               WHEN 'sante_social' THEN 'Santé et social' WHEN 'accompagnement' THEN 'Accompagnement'
               WHEN 'service_agents' THEN 'Services généraux' WHEN 'technique' THEN 'Technique'
               WHEN 'intervention' THEN 'Intervenants' ELSE 'Établissement' END,
           public.acc_trace_nature(j.action), public.acc_trace_section_finale(j.action, j.etablissement_id, j.eleve_id),
           left(j.action, 120), j.eleve_id, false,
           public.acc_trace_affichable(public.acc_trace_section(j.action)),
           public.acc_trace_protegee(j.etablissement_id, j.eleve_id, public.acc_trace_section(j.action))
      FROM public.ent_journal_modif j
      LEFT JOIN LATERAL (
            SELECT fo.code, fo.libelle, fo.famille
              FROM public.ent_personne p
              JOIN public.ent_agent a ON a.personne_id = p.id AND a.etablissement_id = p.etablissement_id
              JOIN public.ent_agent_fonction af ON af.agent_id = a.id AND af.etablissement_id = a.etablissement_id
              JOIN public.ent_fonction fo ON fo.id = af.fonction_id
             WHERE p.etablissement_id = j.etablissement_id AND p.user_id = j.actor_user_id
               AND af.annule_le IS NULL AND af.du <= j.created_at::date
               AND (af.au IS NULL OR af.au >= j.created_at::date)
             ORDER BY af.principale DESC, af.du DESC
             LIMIT 1) f ON true
     WHERE j.domaine = 'consultation' AND j.actor_user_id IS NOT NULL
       AND j.created_at >= date_trunc('month', now()) - interval '12 months'
    ON CONFLICT DO NOTHING;
END;
$$;


-- ─────────────────────── Consultation différée (RGP-197, R-RG28) ───────────────────────
-- Une consultation liée à un signalement n'est pas montrée à la famille tant que l'enquête dure : décision motivée de la
-- direction (`droits.arbitrer`), **bornée** (90 jours au plus, renouvelable par une nouvelle décision), jamais effacée.
-- Pas de clé étrangère vers `acc_trace` (partitionnée, purgée par partition) : la trace est désignée par (id, le).
CREATE TABLE IF NOT EXISTS acc_differe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    trace_id UUID NOT NULL,
    trace_le TIMESTAMPTZ NOT NULL,
    -- Le signalement est un contrôle interne ouvert pour ce motif sur l'agent qui a consulté.
    controle_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 5 AND 500),
    decide_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    decide_le TIMESTAMPTZ NOT NULL,
    jusqu_au TIMESTAMPTZ NOT NULL,
    CONSTRAINT acc_differe_controle_fk FOREIGN KEY (etablissement_id, controle_id)
        REFERENCES acc_controle (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT acc_differe_borne_chk CHECK (jusqu_au > decide_le AND jusqu_au <= decide_le + interval '90 days')
);
CREATE INDEX IF NOT EXISTS acc_differe_trace_idx ON acc_differe (etablissement_id, trace_id);
CREATE OR REPLACE TRIGGER acc_differe_ajout_seul_trg BEFORE UPDATE OR DELETE ON acc_differe
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();

-- Nom d'un agent communiqué à la personne concernée (CL27-16, §11.3) : arbitrage **motivé** de la direction, pour une demande.
CREATE TABLE IF NOT EXISTS acc_nom_communique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    trace_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 5 AND 500),
    decide_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    decide_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT acc_nom_communique_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES rgp_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT acc_nom_communique_uq UNIQUE (etablissement_id, demande_id, trace_id)
);
CREATE OR REPLACE TRIGGER acc_nom_communique_ajout_seul_trg BEFORE UPDATE OR DELETE ON acc_nom_communique
    FOR EACH ROW EXECUTE FUNCTION rgp_g3b_ajout_seul();

-- ─────────────────────── Purge par partition (RGP-198, S-541) ───────────────────────
-- Les partitions couvrent tous les établissements : la purge est une opération de **plateforme**, jamais un `DELETE`
-- applicatif. La décision (durée de chaque établissement, demande ouverte, contrôle ouvert) est prise par le service, par
-- établissement et sous RLS ; ces deux fonctions ne lisent aucune ligne : elles listent les partitions d'après le
-- catalogue et en suppriment une, après un plancher de 12 mois révolus qu'aucun réglage ne peut franchir.
CREATE OR REPLACE FUNCTION acc_trace_partitions_anciennes(p_avant DATE) RETURNS SETOF DATE
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT to_date(substr(c.relname, 11), 'YYYYMM')
      FROM pg_catalog.pg_inherits i
      JOIN pg_catalog.pg_class c ON c.oid = i.inhrelid
      JOIN pg_catalog.pg_class p ON p.oid = i.inhparent
      JOIN pg_catalog.pg_namespace n ON n.oid = p.relnamespace
     WHERE n.nspname = 'public' AND p.relname = 'acc_trace' AND c.relname ~ '^acc_trace_[0-9]{6}$'
       AND to_date(substr(c.relname, 11), 'YYYYMM') < p_avant
     ORDER BY 1
$$;

CREATE OR REPLACE FUNCTION acc_trace_purger_partition(p_mois DATE, p_maintenant TIMESTAMPTZ, p_lignes BIGINT, p_motif TEXT)
RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_nom TEXT;
BEGIN
    IF p_mois IS DISTINCT FROM date_trunc('month', p_mois::timestamp)::date THEN
        RAISE EXCEPTION 'mois de partition invalide' USING ERRCODE = '22023';
    END IF;
    -- Plancher : aucune partition n'est supprimée avant 12 mois après sa fin, quel que soit le réglage.
    IF p_maintenant < ((p_mois + interval '1 month') + interval '12 months') THEN
        RAISE EXCEPTION 'partition trop récente pour être purgée' USING ERRCODE = '22023';
    END IF;
    v_nom := 'acc_trace_' || to_char(p_mois, 'YYYYMM');
    IF to_regclass('public.' || v_nom) IS NULL THEN
        RETURN false;
    END IF;
    EXECUTE format('DROP TABLE public.%I', v_nom);
    INSERT INTO public.acc_purge (id, partition_mois, issue, lignes, motif, le)
    VALUES (gen_random_uuid(), p_mois, 'purgee', GREATEST(p_lignes, 0), left(p_motif, 200), p_maintenant);
    RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION acc_trace_noter_purge_differee(p_mois DATE, p_maintenant TIMESTAMPTZ, p_lignes BIGINT, p_motif TEXT)
RETURNS VOID
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
    INSERT INTO public.acc_purge (id, partition_mois, issue, lignes, motif, le)
    VALUES (gen_random_uuid(), p_mois, 'differee', GREATEST(p_lignes, 0), left(p_motif, 200), p_maintenant)
$$;

-- ─────────────────────── Reprise du balayage des alertes (suites du palier) ───────────────────────
-- Instant jusqu'auquel les règles d'alerte d'un établissement ont été évaluées. Le balayage horaire repart de là
-- (borné à 35 jours) : un serveur arrêté plus de 26 h ne laisse plus de trou dans la surveillance (RGP-196).
CREATE TABLE IF NOT EXISTS acc_balayage (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    jusqu_au TIMESTAMPTZ NOT NULL,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

SELECT ent_rls_poser(ARRAY['acc_trace', 'acc_motif', 'acc_controle', 'acc_alerte', 'acc_differe', 'acc_nom_communique',
                           'acc_balayage']);
