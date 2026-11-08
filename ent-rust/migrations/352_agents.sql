-- Socle 6 (personnels), lot P1, migration 352 : agents et engagements
-- (`docs/socles/06_PERSONNELS.md` §6.2, §8.1, §8.2, §8.14, §17 ; R-P1, R-P4, R-P13).
--
-- Avant : le personnel n'était qu'un compte (`ent_user` + `ent_membership`). Impossible de préparer
-- une rentrée avant l'arrivée d'un contractuel, de saisir un intervenant sans compte, de distinguer
-- l'arrivée prévue de la prise de fonctions, ni d'enregistrer un maître du privé sous contrat qui est
-- aussi salarié de l'OGEC ; une disponibilité ne pouvait se dire qu'en fermant l'accès.
--
-- Après :
--  * `ent_agent` : une personne (socle 4, `ent_personne`) en tant qu'elle travaille ici. Le
--    professeur qui est aussi parent a UNE personne, avec un lien de responsabilité ET un agent ;
--  * `ent_agent_engagement` : périodes de présence (nature, employeur, quotité, motifs codés).
--    R-P1 : l'exclusion de chevauchement porte sur (agent, employeur, nature), pas sur l'agent seul ;
--  * `ent_agent_engagement_suspension` : disponibilité, détachement, congé parental sans clôture
--    de l'engagement (R-P4). Par défaut l'accès est suspendu pendant la suspension ;
--  * fonctions SQL `ent_agent_etat` (état calculé à une date) et `ent_agent_acces_suspendu` ;
--    `ent_acces_ouvert` (350) tient désormais compte de la suspension d'engagement ;
--  * un déclencheur sur `ent_membership` garantit « un agent par compte du personnel », quel que
--    soit le chemin de création (provisionnement, SSO, labo, SQL) ;
--  * reprise de l'existant : une personne (réutilisée si le compte en a déjà une — professeur-
--    parent —, sinon créée) et un agent par accès du personnel, un engagement `a_preciser`.
--
-- Additive et rejouable. RLS (même politique que les migrations 100 et 215) sur toutes les tables.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- 1. Agent ---------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    -- Numéro propre à l'établissement, facultatif. Jamais de NUMEN, de date de naissance, d'adresse,
    -- de nationalité ni de situation de famille (§13.4).
    matricule_interne TEXT CHECK (matricule_interne IS NULL OR length(matricule_interne) <= 40),
    -- NULL = civilité + nom d'usage ; sinon choix encadré de l'agent (§13.3).
    nom_affiche_familles TEXT CHECK (nom_affiche_familles IS NULL OR length(btrim(nom_affiche_familles)) BETWEEN 1 AND 120),
    -- Cas P35 : agent menacé. Posé par la direction seule ; effet sur l'affichage familles et élèves.
    identite_restreinte BOOLEAN NOT NULL DEFAULT false,
    identite_restreinte_motif TEXT
        CHECK (identite_restreinte_motif IS NULL OR identite_restreinte_motif IN ('menace', 'protection_judiciaire', 'autre')),
    identite_restreinte_depuis TIMESTAMPTZ,
    CONSTRAINT ent_agent_restreinte_coherente CHECK (identite_restreinte OR identite_restreinte_motif IS NULL),
    -- Cas P32 : personnel mineur (déclaré, la date de naissance n'étant pas tenue ici).
    personnel_mineur BOOLEAN NOT NULL DEFAULT false,
    -- Photo : le fichier relève du socle 7 (référence sans clé étrangère tant qu'il n'existe pas) ;
    -- la visibilité est choisie par l'agent, collègues / élèves / familles séparément (PRS-09).
    photo_document_id UUID,
    photo_visible_collegues BOOLEAN NOT NULL DEFAULT false,
    photo_visible_eleves BOOLEAN NOT NULL DEFAULT false,
    photo_visible_familles BOOLEAN NOT NULL DEFAULT false,
    photo_consentement_le DATE,
    -- Cas P10 : pause des notifications choisie par l'agent (les avis de sécurité ne sont pas coupés).
    notifications_en_pause_jusqu_au DATE,
    -- Reprise / création depuis un compte : le nom affiché y est une chaîne unique repliée dans
    -- `ent_personne.nom` ; à séparer en nom et prénom (alerte de qualité).
    identite_a_separer BOOLEAN NOT NULL DEFAULT false,
    source TEXT NOT NULL CHECK (source IN ('saisie', 'import_sts', 'import_aaf', 'import_csv', 'reprise')),
    -- Fusion de doublons (PRS-13) : l'agent absorbé pointe l'agent conservé ; ses engagements ont été
    -- déplacés (traçabilité : `ent_agent_engagement.agent_origine_id`).
    fusionne_dans_agent_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_agent_pas_soi_meme CHECK (fusionne_dans_agent_id IS NULL OR fusionne_dans_agent_id <> id),
    CONSTRAINT ent_agent_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agent_personne_uq UNIQUE (etablissement_id, personne_id),
    CONSTRAINT ent_agent_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_agent_fusion_fk FOREIGN KEY (etablissement_id, fusionne_dans_agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);

-- 2. Engagement ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent_engagement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE,
    nature TEXT NOT NULL CHECK (nature IN (
        'titulaire', 'stagiaire', 'contractuel', 'vacataire',
        'maitre_contractuel', 'maitre_agree', 'delegue_auxiliaire',
        'aed', 'aesh', 'assistant_langue',
        'salarie_organisme', 'agent_collectivite', 'mis_a_disposition',
        'intervenant', 'benevole', 'service_civique', 'apprenti', 'stagiaire_ecole',
        'a_preciser')),
    employeur_type TEXT NOT NULL CHECK (employeur_type IN (
        'etat', 'collectivite', 'organisme_gestion', 'association', 'autre_etablissement', 'aucun', 'inconnu')),
    -- Organisme du socle 4 (région, département, OGEC…) : la table n'existe pas encore, pas de clé.
    employeur_organisme_id UUID,
    quotite_pct NUMERIC(5, 2) CHECK (quotite_pct IS NULL OR (quotite_pct > 0 AND quotite_pct <= 100)),
    -- 1 = lundi … 6 = samedi ; NULL = tous les jours.
    jours_travailles SMALLINT[]
        CHECK (jours_travailles IS NULL OR (cardinality(jours_travailles) BETWEEN 1 AND 6
                                            AND jours_travailles <@ ARRAY[1, 2, 3, 4, 5, 6]::SMALLINT[])),
    heures_dues_hebdo NUMERIC(4, 2) CHECK (heures_dues_hebdo IS NULL OR (heures_dues_hebdo >= 0 AND heures_dues_hebdo <= 60)),
    motif_arrivee TEXT CHECK (motif_arrivee IS NULL OR motif_arrivee IN (
        'rentree', 'nomination', 'mutation', 'remplacement', 'creation_poste', 'renouvellement', 'autre')),
    -- Jamais de texte libre. Codes complétés par R-P13.
    motif_fin TEXT CHECK (motif_fin IS NULL OR motif_fin IN (
        'fin_contrat', 'mutation', 'retraite', 'demission', 'deces', 'fin_mission', 'autre',
        'fin_periode_essai', 'rupture_conventionnelle', 'licenciement', 'abandon_de_poste',
        'detachement', 'radiation')),
    remplace_agent_id UUID,
    -- Obligatoire pour les natures limitées (règle ci-dessous).
    referent_agent_id UUID,
    -- Prolongation ou correction : l'engagement remplacé reste en base.
    remplace_engagement_id UUID,
    -- PRS-116 : prise de fonctions réelle, distincte de la date prévue `du`.
    installe_le DATE,
    -- Fusion de deux agents : agent d'où l'engagement a été déplacé.
    agent_origine_id UUID,
    -- Correction d'une saisie erronée : la ligne est annulée, jamais supprimée.
    annule_le TIMESTAMPTZ,
    annule_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_engagement_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_engagement_motif_fin_chk CHECK (motif_fin IS NULL OR au IS NOT NULL),
    -- Natures limitées : date de fin et référent obligatoires (§8.2). « Au plus un an » est vérifié par
    -- la commande (prolongeable par un nouvel engagement).
    CONSTRAINT ent_engagement_nature_limitee_chk CHECK (
        nature NOT IN ('intervenant', 'benevole', 'stagiaire_ecole', 'service_civique')
        OR (au IS NOT NULL AND referent_agent_id IS NOT NULL)),
    CONSTRAINT ent_engagement_referent_chk CHECK (referent_agent_id IS NULL OR referent_agent_id <> agent_id),
    CONSTRAINT ent_engagement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_engagement_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_engagement_referent_fk FOREIGN KEY (etablissement_id, referent_agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_engagement_remplace_agent_fk FOREIGN KEY (etablissement_id, remplace_agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_engagement_remplace_fk FOREIGN KEY (etablissement_id, remplace_engagement_id)
        REFERENCES ent_agent_engagement (etablissement_id, id),
    -- R-P1 : deux engagements simultanés sont permis s'ils diffèrent par l'employeur ou la nature
    -- (maître contractuel de l'État + salarié de l'OGEC ; AED + vacataire d'étude). Une alerte de
    -- qualité signale deux engagements simultanés de même employeur.
    CONSTRAINT ent_engagement_sans_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =,
        agent_id WITH =,
        employeur_type WITH =,
        (COALESCE(employeur_organisme_id, '00000000-0000-0000-0000-000000000000'::uuid)) WITH =,
        nature WITH =,
        daterange(du, au, '[]') WITH &&
    ) WHERE (annule_le IS NULL)
);

CREATE INDEX IF NOT EXISTS ent_engagement_agent_idx ON ent_agent_engagement (etablissement_id, agent_id, du);
CREATE INDEX IF NOT EXISTS ent_engagement_actifs_idx
    ON ent_agent_engagement (etablissement_id, du, au) WHERE annule_le IS NULL;

-- 3. Suspension d'engagement (R-P4) ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent_engagement_suspension (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    engagement_id UUID NOT NULL,
    du DATE NOT NULL,
    au DATE,
    motif_code TEXT NOT NULL CHECK (motif_code IN (
        'disponibilite', 'detachement', 'conge_parental', 'conge_long', 'formation', 'autre')),
    -- Par défaut l'accès est suspendu pendant la suspension (la casquette parent est conservée,
    -- cf. lot P2) ; la direction peut le conserver (congé de formation, par exemple).
    acces_suspendu BOOLEAN NOT NULL DEFAULT true,
    -- Reprise anticipée ou à date : la suspension est close, jamais supprimée.
    levee_le TIMESTAMPTZ,
    levee_par_user_id UUID,
    annule_le TIMESTAMPTZ,
    annule_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_suspension_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_suspension_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_suspension_engagement_fk FOREIGN KEY (etablissement_id, engagement_id)
        REFERENCES ent_agent_engagement (etablissement_id, id),
    CONSTRAINT ent_suspension_sans_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =,
        engagement_id WITH =,
        daterange(du, au, '[]') WITH &&
    ) WHERE (annule_le IS NULL)
);

CREATE INDEX IF NOT EXISTS ent_suspension_engagement_idx
    ON ent_agent_engagement_suspension (etablissement_id, engagement_id) WHERE annule_le IS NULL;

-- 4. États calculés --------------------------------------------------------------------------
-- Un engagement est « courant » à une date : non annulé, commencé, non terminé.
-- Il est « suspendu avec accès suspendu » s'il porte une suspension courante qui coupe l'accès.
CREATE OR REPLACE FUNCTION ent_engagement_acces_coupe(p_etab UUID, p_engagement UUID, p_jour DATE)
RETURNS BOOLEAN
LANGUAGE sql STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_agent_engagement_suspension s
         WHERE s.etablissement_id = p_etab AND s.engagement_id = p_engagement
           AND s.annule_le IS NULL AND s.acces_suspendu
           AND s.du <= p_jour AND (s.au IS NULL OR s.au >= p_jour))
$$;

-- L'accès d'une personne est coupé par la suspension de ses engagements si, à la date, elle a au
-- moins un engagement courant et que TOUS ses engagements courants coupent l'accès (un maître en
-- disponibilité qui reste salarié de l'OGEC garde son accès pour cet engagement-là).
CREATE OR REPLACE FUNCTION ent_agent_acces_suspendu(p_user UUID, p_etab UUID)
RETURNS BOOLEAN
LANGUAGE sql STABLE
AS $$
    WITH jour AS (SELECT ent_aujourdhui_etab(p_etab) AS d),
    courants AS (
        SELECT e.id, ent_engagement_acces_coupe(p_etab, e.id, (SELECT d FROM jour)) AS coupe
          FROM ent_personne p
          JOIN ent_agent a ON a.etablissement_id = p.etablissement_id AND a.personne_id = p.id
          JOIN ent_agent_engagement e ON e.etablissement_id = a.etablissement_id AND e.agent_id = a.id
         WHERE p.etablissement_id = p_etab AND p.user_id = p_user
           AND e.annule_le IS NULL
           AND e.du <= (SELECT d FROM jour) AND (e.au IS NULL OR e.au >= (SELECT d FROM jour)))
    SELECT COALESCE(bool_and(coupe), false) FROM courants
$$;

-- Un accès permet d'agir si et seulement si : état ouvert, date de fin non dépassée (jour de
-- l'établissement) ET accès non coupé par une suspension d'engagement. Définition de 350 + suspension.
CREATE OR REPLACE FUNCTION ent_acces_ouvert(p_user UUID, p_etab UUID)
RETURNS BOOLEAN
LANGUAGE sql STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_membership m
         WHERE m.user_id = p_user AND m.etablissement_id = p_etab
           AND m.etat = 'ouvert'
           AND (m.date_fin IS NULL OR m.date_fin >= ent_aujourdhui_etab(p_etab)))
       AND NOT ent_agent_acces_suspendu(p_user, p_etab)
$$;

-- État d'un agent à une date : prevu | actif | suspendu | parti (`archive` est réservé à la
-- conservation, lot P13). Calculé des engagements ET de l'accès, jamais stocké (aucune dérive).
CREATE OR REPLACE FUNCTION ent_agent_etat(p_etab UUID, p_agent UUID, p_jour DATE)
RETURNS TEXT
LANGUAGE plpgsql STABLE
AS $$
DECLARE
    courants INT;
    coupes INT;
    futurs INT;
    passes INT;
    acces_etat TEXT;
    acces_fin DATE;
BEGIN
    SELECT count(*),
           count(*) FILTER (WHERE ent_engagement_acces_coupe(p_etab, e.id, p_jour))
      INTO courants, coupes
      FROM ent_agent_engagement e
     WHERE e.etablissement_id = p_etab AND e.agent_id = p_agent AND e.annule_le IS NULL
       AND e.du <= p_jour AND (e.au IS NULL OR e.au >= p_jour);
    IF courants > 0 THEN
        SELECT m.etat, m.date_fin INTO acces_etat, acces_fin
          FROM ent_agent a
          JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          JOIN ent_membership m ON m.user_id = p.user_id AND m.etablissement_id = a.etablissement_id
         WHERE a.etablissement_id = p_etab AND a.id = p_agent;
        IF acces_etat = 'ferme' OR (acces_fin IS NOT NULL AND acces_fin < p_jour) THEN
            RETURN 'parti';
        END IF;
        IF acces_etat = 'suspendu' OR coupes = courants THEN
            RETURN 'suspendu';
        END IF;
        RETURN 'actif';
    END IF;
    SELECT count(*) FILTER (WHERE e.du > p_jour), count(*) FILTER (WHERE e.au < p_jour)
      INTO futurs, passes
      FROM ent_agent_engagement e
     WHERE e.etablissement_id = p_etab AND e.agent_id = p_agent AND e.annule_le IS NULL;
    IF futurs > 0 THEN RETURN 'prevu'; END IF;
    IF passes > 0 THEN RETURN 'parti'; END IF;
    RETURN 'prevu';
END
$$;

-- 4 bis. Garde-fou du dernier administrateur (350) : un administrateur dont l'accès est coupé en ce
-- moment par une suspension d'engagement (disponibilité, congé) n'est plus un administrateur durable.
-- Même fonction que 350, une condition de plus dans le décompte.
CREATE OR REPLACE FUNCTION ent_membership_garde_dernier_admin()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    restant BIGINT;
BEGIN
    IF OLD.role IS DISTINCT FROM 'admin' OR OLD.etat <> 'ouvert' OR OLD.date_fin IS NOT NULL THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE'
       AND NEW.user_id = OLD.user_id AND NEW.etablissement_id = OLD.etablissement_id
       AND NEW.role = 'admin' AND NEW.etat = 'ouvert' AND NEW.date_fin IS NULL THEN
        RETURN NEW;
    END IF;
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;

    PERFORM pg_advisory_xact_lock(hashtextextended('ent_dernier_admin:' || OLD.etablissement_id::text, 0));
    SELECT count(*) INTO restant
      FROM ent_membership m
     WHERE m.etablissement_id = OLD.etablissement_id
       AND m.user_id <> OLD.user_id
       AND m.role = 'admin' AND m.etat = 'ouvert' AND m.date_fin IS NULL
       AND NOT ent_agent_acces_suspendu(m.user_id, m.etablissement_id);
    IF restant = 0 THEN
        RAISE EXCEPTION 'dernier_administrateur' USING ERRCODE = '23514',
            HINT = 'Un établissement ne peut pas perdre son dernier administrateur.';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$;

-- 5. Un agent par compte du personnel ---------------------------------------------------------
-- Tout accès de profil « personnel » (ni parent ni élève) a une personne (réutilisée si le compte
-- en a déjà une — professeur-parent —, sinon créée avec le nom affiché replié dans `nom`, signalé
-- « à séparer ») et un agent, avec un engagement `a_preciser` qui commence le jour de la création
-- de l'accès. Le téléphone du compte n'est PAS copié sur la personne (minimisation, §13.4).
CREATE OR REPLACE FUNCTION ent_membership_agent_sync() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    pid UUID;
    aid UUID;
    cree BOOLEAN := false;
    debut DATE;
BEGIN
    IF NEW.role IN ('parent', 'eleve') THEN
        RETURN NEW;
    END IF;
    SELECT id INTO pid FROM ent_personne
     WHERE etablissement_id = NEW.etablissement_id AND user_id = NEW.user_id;
    IF pid IS NULL THEN
        INSERT INTO ent_personne (id, etablissement_id, nom, email, user_id, source)
        SELECT gen_random_uuid(), NEW.etablissement_id,
               left(COALESCE(NULLIF(btrim(u.display_name), ''), u.email), 120),
               lower(u.email), u.id, 'compte'
          FROM ent_user u WHERE u.id = NEW.user_id
        ON CONFLICT (etablissement_id, user_id) WHERE user_id IS NOT NULL DO NOTHING;
        SELECT id INTO pid FROM ent_personne
         WHERE etablissement_id = NEW.etablissement_id AND user_id = NEW.user_id;
        cree := true;
    END IF;
    IF pid IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT id INTO aid FROM ent_agent WHERE etablissement_id = NEW.etablissement_id AND personne_id = pid;
    IF aid IS NULL THEN
        aid := gen_random_uuid();
        INSERT INTO ent_agent (id, etablissement_id, personne_id, source, identite_a_separer)
        VALUES (aid, NEW.etablissement_id, pid, 'saisie', cree);
        debut := ent_aujourdhui_etab(NEW.etablissement_id, COALESCE(NEW.created_at, now()));
        INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, au, nature, employeur_type)
        VALUES (gen_random_uuid(), NEW.etablissement_id, aid, debut,
                CASE WHEN NEW.date_fin IS NULL THEN NULL ELSE GREATEST(debut, NEW.date_fin) END,
                'a_preciser', 'inconnu')
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_membership_agent_sync_trg ON ent_membership;
CREATE TRIGGER ent_membership_agent_sync_trg
    AFTER INSERT OR UPDATE OF role ON ent_membership
    FOR EACH ROW EXECUTE FUNCTION ent_membership_agent_sync();

-- 6. Reprise de l'existant (idempotente) ------------------------------------------------------
-- 6.1 Personnes manquantes (comptes du personnel sans personne).
INSERT INTO ent_personne (id, etablissement_id, nom, email, user_id, source)
SELECT gen_random_uuid(), m.etablissement_id,
       left(COALESCE(NULLIF(btrim(u.display_name), ''), u.email), 120),
       lower(u.email), u.id, 'reprise'
  FROM ent_membership m
  JOIN ent_user u ON u.id = m.user_id
 WHERE m.role NOT IN ('parent', 'eleve')
   AND NOT EXISTS (SELECT 1 FROM ent_personne p
                    WHERE p.etablissement_id = m.etablissement_id AND p.user_id = m.user_id)
ON CONFLICT (etablissement_id, user_id) WHERE user_id IS NOT NULL DO NOTHING;

-- 6.2 Un agent par personne de compte du personnel. `identite_a_separer` : seulement quand la
-- personne vient d'être créée par la reprise (source `reprise`), pas pour un professeur-parent dont
-- la personne a déjà été saisie par un secrétariat.
INSERT INTO ent_agent (id, etablissement_id, personne_id, source, identite_a_separer)
SELECT gen_random_uuid(), p.etablissement_id, p.id, 'reprise', (p.source = 'reprise')
  FROM ent_membership m
  JOIN ent_personne p ON p.etablissement_id = m.etablissement_id AND p.user_id = m.user_id
 WHERE m.role NOT IN ('parent', 'eleve')
   AND NOT EXISTS (SELECT 1 FROM ent_agent a
                    WHERE a.etablissement_id = p.etablissement_id AND a.personne_id = p.id);

-- 6.3 Un engagement `a_preciser` par agent sans engagement : début = création de l'accès (date de
-- l'établissement), fin = date de fin de l'accès, ou date de fermeture d'un accès fermé ; toujours
-- au plus tôt le début (une date de fin saisie avant la création de l'accès ne doit pas échouer).
INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, au, nature, employeur_type)
SELECT gen_random_uuid(), a.etablissement_id, a.id, d.debut,
       CASE
           WHEN m.etat = 'ferme' THEN GREATEST(d.debut, LEAST(
                COALESCE(m.date_fin, ent_aujourdhui_etab(a.etablissement_id, COALESCE(m.etat_depuis, now()))),
                ent_aujourdhui_etab(a.etablissement_id, COALESCE(m.etat_depuis, now()))))
           WHEN m.date_fin IS NOT NULL THEN GREATEST(d.debut, m.date_fin)
           ELSE NULL
       END,
       'a_preciser', 'inconnu'
  FROM ent_agent a
  JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
  JOIN ent_membership m ON m.user_id = p.user_id AND m.etablissement_id = a.etablissement_id
 CROSS JOIN LATERAL (SELECT ent_aujourdhui_etab(a.etablissement_id, COALESCE(m.created_at, now())) AS debut) d
 WHERE m.role NOT IN ('parent', 'eleve')
   AND NOT EXISTS (SELECT 1 FROM ent_agent_engagement e
                    WHERE e.etablissement_id = a.etablissement_id AND e.agent_id = a.id);

-- 7. RLS (politique des migrations 100 et 215) ------------------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_agent', 'ent_agent_engagement', 'ent_agent_engagement_suspension'];
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
