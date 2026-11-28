-- 279 — Transverse « la deuxième personne fondée sur la personne physique », lot 1 : l'individu.
-- Conception : docs/socles/TRANSVERSE_INDIVIDU.md. Plage 260–279 du socle 4 (279 libre, vérifiée sur toutes les branches).
--
-- Un compte a au plus une fiche personne par établissement (120). Un même humain à deux comptes (adresse professionnelle
-- et adresse personnelle), ou un agent et une fiche parent sans compte, a donc deux fiches que rien ne relie : les règles
-- « une autre personne » et « jamais pour son enfant » comparent des comptes et ne le reconnaissent pas.
--
-- Ici :
--  * `ent_individu_lien` : deux fiches du même établissement sont le même individu (posé par un humain, levé une fois,
--    sinon immuable) ;
--  * `ent_individu_declaration` : déclaration depuis un compte, confirmée depuis l'autre (preuve de possession des deux) ;
--  * `ent_individu_proposition_ecartee` : ressemblance écartée par la direction, jamais reproposée ;
--  * fonctions `ent_individu_personnes`, `ent_individu_personnes_du_compte`, `ent_individu_comptes`,
--    `ent_meme_individu` : la SEULE définition, pour l'application et pour les gardes de la base (lots 2 et 3).
--
-- L'individu ne fait que restreindre : aucune fonction ici n'accorde un droit. Aucune fusion (D-013, 123).
-- Règle 38 : en fin de tableau. Fonctions : `SET search_path = pg_catalog, public, pg_temp`, relations `public.`.
-- Additive et rejouable.

-- 1. Liens ------------------------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_individu_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_a UUID NOT NULL,
    personne_b UUID NOT NULL,
    origine TEXT NOT NULL
        CHECK (origine IN ('declaration_direction', 'declaration_confirmee', 'proposition_acceptee')),
    motif_code TEXT NOT NULL
        CHECK (motif_code IN ('deux_comptes', 'fiche_sans_compte', 'doublon_agent', 'autre')),
    pose_par_user_id UUID NOT NULL,
    pose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    declaration_id UUID,
    leve_par_user_id UUID,
    leve_le TIMESTAMPTZ,
    leve_motif_code TEXT CHECK (leve_motif_code IS NULL OR leve_motif_code IN ('erreur', 'homonyme', 'autre')),
    CONSTRAINT ent_individu_lien_ordre CHECK (personne_a < personne_b),
    CONSTRAINT ent_individu_lien_levee CHECK (
        (leve_le IS NULL) = (leve_par_user_id IS NULL) AND (leve_le IS NULL) = (leve_motif_code IS NULL)),
    CONSTRAINT ent_individu_lien_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_individu_lien_a_fk FOREIGN KEY (etablissement_id, personne_a)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_individu_lien_b_fk FOREIGN KEY (etablissement_id, personne_b)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
-- Un seul lien actif par paire.
CREATE UNIQUE INDEX IF NOT EXISTS ent_individu_lien_actif_uq
    ON ent_individu_lien (etablissement_id, personne_a, personne_b) WHERE leve_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_individu_lien_b_idx
    ON ent_individu_lien (etablissement_id, personne_b) WHERE leve_le IS NULL;

-- 2. Déclarations en deux temps ---------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_individu_declaration (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    declarant_user_id UUID NOT NULL,
    cible_user_id UUID NOT NULL,
    declare_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    expire_le TIMESTAMPTZ NOT NULL,
    issue TEXT CHECK (issue IS NULL OR issue IN ('confirmee', 'refusee')),
    issue_le TIMESTAMPTZ,
    lien_id UUID,
    CONSTRAINT ent_individu_declaration_distincts CHECK (declarant_user_id <> cible_user_id),
    CONSTRAINT ent_individu_declaration_issue CHECK ((issue IS NULL) = (issue_le IS NULL)),
    CONSTRAINT ent_individu_declaration_lien CHECK (lien_id IS NULL OR issue = 'confirmee'),
    CONSTRAINT ent_individu_declaration_delai CHECK (expire_le > declare_le),
    CONSTRAINT ent_individu_declaration_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_individu_declaration_cible_idx
    ON ent_individu_declaration (etablissement_id, cible_user_id) WHERE issue IS NULL;
CREATE INDEX IF NOT EXISTS ent_individu_declaration_declarant_idx
    ON ent_individu_declaration (etablissement_id, declarant_user_id, declare_le);

-- 3. Propositions écartées --------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_individu_proposition_ecartee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_a UUID NOT NULL,
    personne_b UUID NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN ('homonyme', 'coordonnee_partagee', 'autre')),
    par_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_individu_ecartee_ordre CHECK (personne_a < personne_b),
    CONSTRAINT ent_individu_ecartee_paire_uq UNIQUE (etablissement_id, personne_a, personne_b),
    CONSTRAINT ent_individu_ecartee_a_fk FOREIGN KEY (etablissement_id, personne_a)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_individu_ecartee_b_fk FOREIGN KEY (etablissement_id, personne_b)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);

-- 4. Gardes : immuables, sauf l'issue (posée une fois) ; suppression seulement quand une fiche disparaît (cascade) ou
--    à la purge volontaire d'un établissement (`SET LOCAL ent.purge_etablissement = 'on'`, jamais posé par l'application).
CREATE OR REPLACE FUNCTION ent_individu_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF pg_catalog.current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN COALESCE(NEW, OLD);
    END IF;
    IF TG_OP = 'TRUNCATE' THEN
        RAISE EXCEPTION '% est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF TG_TABLE_NAME IN ('ent_individu_lien', 'ent_individu_proposition_ecartee')
           AND (NOT EXISTS (SELECT 1 FROM public.ent_personne p
                             WHERE p.etablissement_id = OLD.etablissement_id AND p.id = OLD.personne_a)
                OR NOT EXISTS (SELECT 1 FROM public.ent_personne p
                                WHERE p.etablissement_id = OLD.etablissement_id AND p.id = OLD.personne_b)) THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION '% est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
    END IF;
    -- UPDATE
    IF TG_TABLE_NAME = 'ent_individu_lien' THEN
        IF OLD.leve_le IS NOT NULL
           OR NEW.leve_le IS NULL
           OR (NEW.id, NEW.etablissement_id, NEW.personne_a, NEW.personne_b, NEW.origine, NEW.motif_code,
               NEW.pose_par_user_id, NEW.pose_le, NEW.declaration_id)
              IS DISTINCT FROM
              (OLD.id, OLD.etablissement_id, OLD.personne_a, OLD.personne_b, OLD.origine, OLD.motif_code,
               OLD.pose_par_user_id, OLD.pose_le, OLD.declaration_id) THEN
            RAISE EXCEPTION 'un lien d''individu ne change pas : il se lève une seule fois' USING ERRCODE = '23514';
        END IF;
        -- Lever retire des refus : jamais par l'individu concerné, même en SQL direct (l'application le refuse déjà).
        IF NEW.leve_par_user_id IS NULL
           OR public.ent_individu_personnes(NEW.etablissement_id, ARRAY[NEW.personne_a])
              && public.ent_individu_personnes_du_compte(NEW.etablissement_id, NEW.leve_par_user_id) THEN
            RAISE EXCEPTION 'un lien d''individu ne se lève pas par la personne concernée' USING ERRCODE = '23514';
        END IF;
        NEW.leve_le := pg_catalog.now();
        RETURN NEW;
    ELSIF TG_TABLE_NAME = 'ent_individu_declaration' THEN
        IF OLD.issue IS NOT NULL
           OR NEW.issue IS NULL
           OR (NEW.id, NEW.etablissement_id, NEW.declarant_user_id, NEW.cible_user_id, NEW.declare_le, NEW.expire_le)
              IS DISTINCT FROM
              (OLD.id, OLD.etablissement_id, OLD.declarant_user_id, OLD.cible_user_id, OLD.declare_le, OLD.expire_le) THEN
            RAISE EXCEPTION 'une déclaration d''individu ne change pas : son issue se pose une seule fois' USING ERRCODE = '23514';
        END IF;
        IF NEW.issue = 'confirmee' AND pg_catalog.now() > OLD.expire_le THEN
            RAISE EXCEPTION 'une déclaration d''individu expirée ne se confirme plus' USING ERRCODE = '23514';
        END IF;
        NEW.issue_le := pg_catalog.now();
        RETURN NEW;
    END IF;
    RAISE EXCEPTION '% est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;

-- À l'insertion : la ligne naît active et datée par la base ; un lien relie deux fiches qui ne sont pas déjà le même
-- individu (sinon il ne dit rien de plus et masquerait la levée de l'autre chemin).
CREATE OR REPLACE FUNCTION ent_individu_insertion() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF TG_TABLE_NAME = 'ent_individu_lien' THEN
        IF NEW.leve_le IS NOT NULL OR NEW.leve_par_user_id IS NOT NULL OR NEW.leve_motif_code IS NOT NULL THEN
            RAISE EXCEPTION 'un lien d''individu naît actif' USING ERRCODE = '23514';
        END IF;
        NEW.pose_le := pg_catalog.now();
        -- Verrou de l'établissement : deux liens concurrents ne contournent pas le contrôle ci-dessous.
        PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('individu:' || NEW.etablissement_id::text, 0));
        IF NEW.personne_b = ANY(public.ent_individu_personnes(NEW.etablissement_id, ARRAY[NEW.personne_a])) THEN
            RAISE EXCEPTION 'ces deux fiches sont déjà le même individu' USING ERRCODE = '23505';
        END IF;
        -- Borne : un individu compte au plus 16 fiches (fermeture courte, chaîne d'erreurs visible).
        IF pg_catalog.cardinality(public.ent_individu_personnes(NEW.etablissement_id,
                                                                ARRAY[NEW.personne_a, NEW.personne_b])) > 16 THEN
            RAISE EXCEPTION 'un individu compte au plus 16 fiches reliées' USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'ent_individu_declaration' THEN
        IF NEW.issue IS NOT NULL OR NEW.issue_le IS NOT NULL OR NEW.lien_id IS NOT NULL THEN
            RAISE EXCEPTION 'une déclaration d''individu naît en attente' USING ERRCODE = '23514';
        END IF;
        NEW.declare_le := pg_catalog.now();
        NEW.expire_le := NEW.declare_le + interval '24 hours';
    ELSE
        NEW.le := pg_catalog.now();
    END IF;
    RETURN NEW;
END
$$;

DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_individu_lien', 'ent_individu_declaration', 'ent_individu_proposition_ecartee'] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_garde', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION ent_individu_garde()',
                       t || '_garde', t);
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_vidage', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION ent_individu_garde()',
                       t || '_vidage', t);
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_insertion', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION ent_individu_insertion()',
                       t || '_insertion', t);
    END LOOP;
END
$$;

-- 5. Fonctions de l'individu ------------------------------------------------------------------------------------

-- Fermeture : les fiches du même individu que `personnes`, dans l'établissement : liens actifs (dans les deux sens),
-- chaîne de fusion (la fiche absorbée suit sa cible ; la cible retrouve ses absorbées). Ensemble fini : l'UNION
-- récursive s'arrête d'elle-même ; la pose d'un lien est refusée au-delà de 16 fiches (garde d'insertion).
CREATE OR REPLACE FUNCTION ent_individu_personnes(etab UUID, personnes UUID[]) RETURNS UUID[]
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    WITH RECURSIVE f(id) AS (
        SELECT x FROM pg_catalog.unnest(COALESCE(personnes, ARRAY[]::uuid[])) AS x WHERE x IS NOT NULL
        UNION
        SELECT n.id
          FROM f
          CROSS JOIN LATERAL (
              SELECT l.personne_b FROM public.ent_individu_lien l
               WHERE l.etablissement_id = etab AND l.leve_le IS NULL AND l.personne_a = f.id
              UNION ALL
              SELECT l.personne_a FROM public.ent_individu_lien l
               WHERE l.etablissement_id = etab AND l.leve_le IS NULL AND l.personne_b = f.id
              UNION ALL
              SELECT p.fusionnee_dans FROM public.ent_personne p
               WHERE p.etablissement_id = etab AND p.id = f.id AND p.fusionnee_dans IS NOT NULL
              UNION ALL
              SELECT p.id FROM public.ent_personne p
               WHERE p.etablissement_id = etab AND p.fusionnee_dans = f.id
          ) AS n(id)
    )
    SELECT COALESCE(pg_catalog.array_agg(DISTINCT f.id), ARRAY[]::uuid[]) FROM f
$$;

-- Fiches de l'individu d'un compte : la fiche du compte, la fiche de l'élève dont c'est le compte, et leur fermeture.
CREATE OR REPLACE FUNCTION ent_individu_personnes_du_compte(etab UUID, usr UUID) RETURNS UUID[]
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT public.ent_individu_personnes(etab, ARRAY(
        SELECT p.id FROM public.ent_personne p
         WHERE p.etablissement_id = etab AND p.user_id = usr
        UNION
        SELECT p.id FROM public.ent_personne p
          JOIN public.ent_eleve_compte ec ON ec.etablissement_id = p.etablissement_id AND ec.eleve_id = p.eleve_id
         WHERE p.etablissement_id = etab AND ec.user_id = usr))
$$;

-- Comptes de l'individu d'un compte (le compte lui-même compris) : comptes des fiches, comptes d'élève des fiches
-- d'élève. NULL en entrée : tableau vide.
CREATE OR REPLACE FUNCTION ent_individu_comptes(etab UUID, usr UUID) RETURNS UUID[]
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT CASE WHEN usr IS NULL THEN ARRAY[]::uuid[] ELSE ARRAY(
        SELECT usr
        UNION
        SELECT p.user_id FROM public.ent_personne p
         WHERE p.etablissement_id = etab AND p.user_id IS NOT NULL
           AND p.id = ANY(public.ent_individu_personnes_du_compte(etab, usr))
        UNION
        SELECT ec.user_id FROM public.ent_personne p
          JOIN public.ent_eleve_compte ec ON ec.etablissement_id = p.etablissement_id AND ec.eleve_id = p.eleve_id
         WHERE p.etablissement_id = etab
           AND p.id = ANY(public.ent_individu_personnes_du_compte(etab, usr))) END
$$;

-- Deux comptes sont-ils le même individu dans cet établissement ? Faux si l'un manque.
CREATE OR REPLACE FUNCTION ent_meme_individu(etab UUID, u1 UUID, u2 UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp
AS $$
    SELECT CASE
        WHEN u1 IS NULL OR u2 IS NULL THEN false
        WHEN u1 = u2 THEN true
        ELSE u2 = ANY(public.ent_individu_comptes(etab, u1))
    END
$$;

SELECT ent_rls_poser(ARRAY['ent_individu_lien', 'ent_individu_declaration', 'ent_individu_proposition_ecartee']);
