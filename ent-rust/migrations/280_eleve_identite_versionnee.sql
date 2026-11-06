-- Socle 2 (l'élève), lot E2 / étapes M2 et M7 (partie identité) : identité versionnée.
--
-- Tout se modifie, rien ne s'efface : chaque version d'identité garde sa date d'effet,
-- sa nature (correction d'une erreur OU changement réel), sa source, son auteur et sa
-- pièce justificative (document privé de l'élève, servi par URL pré-signée). La valeur
-- COURANTE reste copiée sur `ent_eleve` (des dizaines de requêtes la lisent) mais cette
-- copie est maintenue PAR LA BASE et PROTÉGÉE : toute écriture directe des colonnes
-- d'identité est refusée, seul le trigger de projection (version insérée) les modifie.
--
-- Additif, idempotent, rejoué à chaque démarrage. Numéros 280-289 réservés au lot E2.
-- Jamais d'état civil dans un événement : la charge utile porte l'identifiant de
-- version et les NOMS des champs modifiés (jamais leurs valeurs).

-- ───────────────────────── Versions d'identité ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_eleve_identite (
    id                      UUID PRIMARY KEY,
    etablissement_id        UUID NOT NULL,
    eleve_id                UUID NOT NULL,
    nom_naissance           TEXT NOT NULL,
    nom_usage               TEXT,
    prenoms                 TEXT[] NOT NULL,
    rang_prenom_usuel       SMALLINT NOT NULL DEFAULT 1,
    prenom_usage            TEXT,
    sexe                    TEXT,
    date_naissance          DATE,
    naissance_commune_cog   TEXT,
    naissance_commune       TEXT,
    naissance_pays_cog      TEXT,
    naissance_lieu_etranger TEXT,
    nature                  TEXT NOT NULL,
    effet_du                DATE NOT NULL,
    remplace_version_id     UUID,
    remplacee_par           UUID,
    motif                   TEXT,
    piece_document_id       UUID,
    source                  TEXT NOT NULL DEFAULT 'saisie',
    source_ref              UUID,
    sensible                BOOLEAN NOT NULL DEFAULT false,
    enregistre_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    enregistre_par          UUID,
    CONSTRAINT ent_eleve_identite_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
);

-- Contraintes de valeur : posées NOT VALID (elles protègent toute écriture à venir sans
-- jamais faire échouer la reprise d'une donnée ancienne imprévue).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_identite_valeurs_chk') THEN
        ALTER TABLE ent_eleve_identite ADD CONSTRAINT ent_eleve_identite_valeurs_chk CHECK (
            char_length(btrim(nom_naissance)) BETWEEN 1 AND 120
            AND cardinality(prenoms) BETWEEN 1 AND 10
            AND rang_prenom_usuel BETWEEN 1 AND cardinality(prenoms)
            AND (nom_usage IS NULL OR char_length(btrim(nom_usage)) BETWEEN 1 AND 120)
            AND (prenom_usage IS NULL OR char_length(btrim(prenom_usage)) BETWEEN 1 AND 120)
            AND (sexe IS NULL OR sexe IN ('M', 'F'))
            AND (motif IS NULL OR char_length(motif) <= 500)
        ) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_identite_nature_chk') THEN
        ALTER TABLE ent_eleve_identite ADD CONSTRAINT ent_eleve_identite_nature_chk CHECK (
            nature IN ('creation', 'reprise', 'import', 'correction', 'changement_etat_civil', 'usage', 'fusion')
            AND source IN ('saisie', 'admission', 'import', 'famille', 'eleve', 'ancien_etablissement', 'reprise', 'fusion')
        );
    END IF;
    -- Une correction remplace une version et dit pourquoi ; un changement réel dit
    -- pourquoi ET sur quelle pièce.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_identite_regles_chk') THEN
        ALTER TABLE ent_eleve_identite ADD CONSTRAINT ent_eleve_identite_regles_chk CHECK (
            (nature NOT IN ('correction', 'changement_etat_civil') OR (motif IS NOT NULL AND btrim(motif) <> ''))
            AND (nature <> 'correction' OR remplace_version_id IS NOT NULL)
            AND (nature <> 'changement_etat_civil' OR piece_document_id IS NOT NULL)
            AND (nature NOT IN ('creation', 'reprise') OR remplace_version_id IS NULL)
        ) NOT VALID;
    END IF;
END $$;

-- Une seule version « en vigueur » (non remplacée) par élève et par date d'effet.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_identite_effet_uniq
    ON ent_eleve_identite (eleve_id, effet_du) WHERE remplacee_par IS NULL;
CREATE INDEX IF NOT EXISTS ent_eleve_identite_eleve_idx
    ON ent_eleve_identite (etablissement_id, eleve_id, effet_du DESC, enregistre_le DESC);

-- ───────────────────────── Projection sur ent_eleve ─────────────────────────
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS nom_naissance TEXT;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS prenoms TEXT[];
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS identite_version_id UUID;

-- Version courante d'un élève à une date : la version non remplacée la plus récente
-- dont la date d'effet est atteinte. Une version datée dans le futur (changement
-- annoncé) devient courante à sa date (cf. ent_eleve_identite_resynchroniser).
CREATE OR REPLACE FUNCTION ent_eleve_identite_courante(p_etab UUID, p_eleve UUID, p_jour DATE)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT id FROM ent_eleve_identite
    WHERE etablissement_id = p_etab AND eleve_id = p_eleve
      AND remplacee_par IS NULL AND effet_du <= p_jour
    ORDER BY effet_du DESC, enregistre_le DESC, id
    LIMIT 1
$$;

-- Recopie la version `v` sur la ligne d'élève (UNIQUE chemin d'écriture des colonnes
-- d'identité de ent_eleve). Le drapeau transactionnel `ent.identite_projection`, posé
-- ici seulement le temps de l'UPDATE, laisse passer la garde ci-dessous.
CREATE OR REPLACE FUNCTION ent_eleve_identite_projeter(v ent_eleve_identite) RETURNS VOID
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM set_config('ent.identite_projection', 'on', true);
    UPDATE ent_eleve SET
        nom = coalesce(nullif(btrim(v.nom_usage), ''), v.nom_naissance),
        prenom = coalesce(nullif(btrim(v.prenom_usage), ''), v.prenoms[v.rang_prenom_usuel]),
        nom_naissance = v.nom_naissance,
        prenoms = v.prenoms,
        sexe = v.sexe,
        date_naissance = v.date_naissance,
        identite_version_id = v.id,
        updated_at = now()
    WHERE etablissement_id = v.etablissement_id AND id = v.eleve_id;
    PERFORM set_config('ent.identite_projection', 'off', true);
END $$;

-- Après insertion d'une version : si elle est (ou devient) la courante, on la projette.
CREATE OR REPLACE FUNCTION ent_eleve_identite_apres_insert() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE courante UUID;
BEGIN
    courante := ent_eleve_identite_courante(NEW.etablissement_id, NEW.eleve_id, current_date);
    IF courante = NEW.id THEN
        PERFORM ent_eleve_identite_projeter(NEW);
    END IF;
    RETURN NULL;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identite_projection ON ent_eleve_identite;
CREATE TRIGGER ent_eleve_identite_projection
    AFTER INSERT ON ent_eleve_identite
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identite_apres_insert();

-- Les versions sont immuables : seule `remplacee_par` peut être posée (une fois), par
-- la correction qui remplace la version. Aucune suppression.
CREATE OR REPLACE FUNCTION ent_eleve_identite_immuable() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'identité : une version ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.remplacee_par IS NULL AND NEW.remplacee_par IS NOT NULL
       AND (to_jsonb(NEW) - 'remplacee_par') = (to_jsonb(OLD) - 'remplacee_par') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'identité : une version est immuable (créer une correction ou un changement)'
        USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identite_immuable ON ent_eleve_identite;
CREATE TRIGGER ent_eleve_identite_immuable
    BEFORE UPDATE OR DELETE ON ent_eleve_identite
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identite_immuable();

-- Garde : l'identité de ent_eleve ne s'écrit pas directement. Seule la projection d'une
-- version (ent_eleve_identite_projeter) passe ; toute autre modification d'une colonne
-- d'identité est refusée, quelle que soit la requête.
CREATE OR REPLACE FUNCTION ent_eleve_identite_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF coalesce(current_setting('ent.identite_projection', true), '') = 'on' THEN
        RETURN NEW;
    END IF;
    IF (NEW.nom, NEW.prenom, NEW.sexe, NEW.date_naissance, NEW.nom_naissance, NEW.prenoms, NEW.identite_version_id)
       IS NOT DISTINCT FROM
       (OLD.nom, OLD.prenom, OLD.sexe, OLD.date_naissance, OLD.nom_naissance, OLD.prenoms, OLD.identite_version_id) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'identité de l''élève : écriture directe interdite (utiliser les commandes de correction ou de changement)'
        USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identite_garde ON ent_eleve;
CREATE TRIGGER ent_eleve_identite_garde
    BEFORE UPDATE ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identite_garde();

-- ───────────────────────── Reprise de l'existant (idempotente) ─────────────────────────
-- Élèves déjà présents, ou créés par un chemin qui n'écrit pas la version (import,
-- inscription, labo) : version 1 = l'état civil tel qu'il est. Le nom repris est PRÉSUMÉ
-- nom de naissance (à vérifier à l'écran). Sexe hors M/F → NULL, listé dans
-- ent_eleve_identite_a_traiter (jamais perdu : la ligne d'élève garde sa valeur d'origine
-- jusqu'à la prochaine version).
CREATE TABLE IF NOT EXISTS ent_eleve_identite_a_traiter (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL,
    eleve_id         UUID NOT NULL,
    champ            TEXT NOT NULL,
    cause            TEXT NOT NULL,
    valeur_origine   TEXT,
    detecte_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le        TIMESTAMPTZ,
    UNIQUE (eleve_id, champ, cause)
);

CREATE OR REPLACE FUNCTION ent_eleve_sexe_normalise(s TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE upper(btrim(coalesce(s, '')))
        WHEN 'M' THEN 'M' WHEN 'MASCULIN' THEN 'M'
        WHEN 'F' THEN 'F' WHEN 'FEMININ' THEN 'F' WHEN 'FÉMININ' THEN 'F'
        ELSE NULL END
$$;

-- Crée la version initiale d'un élève qui n'en a pas. `p_nature` : 'reprise' (existant)
-- ou 'creation' (trigger d'insertion). Le trigger de projection recopie la version sur
-- la ligne d'élève (valeurs inchangées, sexe normalisé).
CREATE OR REPLACE FUNCTION ent_eleve_identite_initiale(e ent_eleve, p_nature TEXT)
RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE
    vid UUID := gen_random_uuid();
    sx TEXT := ent_eleve_sexe_normalise(e.sexe);
    nom_v TEXT := left(coalesce(nullif(btrim(e.nom), ''), '(à renseigner)'), 120);
    prenom_v TEXT := left(coalesce(nullif(btrim(e.prenom), ''), '(à renseigner)'), 120);
    acteur UUID := nullif(current_setting('ent.acteur_id', true), '')::uuid;
    src TEXT := coalesce(nullif(current_setting('ent.identite_source', true), ''), 'saisie');
BEGIN
    IF p_nature = 'reprise' THEN src := 'reprise'; acteur := NULL; END IF;
    IF e.sexe IS NOT NULL AND btrim(e.sexe) <> '' AND sx IS NULL THEN
        INSERT INTO ent_eleve_identite_a_traiter (etablissement_id, eleve_id, champ, cause, valeur_origine)
        VALUES (e.etablissement_id, e.id, 'sexe', 'valeur_inconnue', left(e.sexe, 40))
        ON CONFLICT DO NOTHING;
    END IF;
    IF nom_v IS DISTINCT FROM e.nom THEN
        INSERT INTO ent_eleve_identite_a_traiter (etablissement_id, eleve_id, champ, cause, valeur_origine)
        VALUES (e.etablissement_id, e.id, 'nom', 'valeur_invalide', left(e.nom, 200))
        ON CONFLICT DO NOTHING;
    END IF;
    IF prenom_v IS DISTINCT FROM e.prenom THEN
        INSERT INTO ent_eleve_identite_a_traiter (etablissement_id, eleve_id, champ, cause, valeur_origine)
        VALUES (e.etablissement_id, e.id, 'prenom', 'valeur_invalide', left(e.prenom, 200))
        ON CONFLICT DO NOTHING;
    END IF;
    INSERT INTO ent_eleve_identite (
        id, etablissement_id, eleve_id, nom_naissance, prenoms, rang_prenom_usuel, sexe,
        date_naissance, nature, effet_du, source, enregistre_le, enregistre_par
    ) VALUES (
        vid, e.etablissement_id, e.id, nom_v, ARRAY[prenom_v], 1, sx, e.date_naissance,
        p_nature,
        least(coalesce(e.date_entree, e.created_at::date), current_date),
        src, now(), acteur
    );
    RETURN vid;
END $$;

-- Après insertion d'un élève par n'importe quel chemin : version initiale (création).
-- L'auteur et la source viennent des réglages de transaction `ent.acteur_id` et
-- `ent.identite_source` (posés par les commandes), à défaut NULL / 'saisie'.
CREATE OR REPLACE FUNCTION ent_eleve_apres_insert_identite() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.identite_version_id IS NULL THEN
        PERFORM ent_eleve_identite_initiale(NEW, 'creation');
    END IF;
    RETURN NULL;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identite_initiale ON ent_eleve;
CREATE TRIGGER ent_eleve_identite_initiale
    AFTER INSERT ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_apres_insert_identite();

-- Reprise de l'existant : idempotente (ne traite que les élèves sans version courante
-- rattachée). Renvoie le nombre de versions créées.
CREATE OR REPLACE FUNCTION ent_eleve_identite_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; e ent_eleve;
BEGIN
    FOR e IN SELECT * FROM ent_eleve WHERE identite_version_id IS NULL LOOP
        PERFORM ent_eleve_identite_initiale(e, 'reprise');
        n := n + 1;
    END LOOP;
    RETURN n;
END $$;

SELECT ent_eleve_identite_reprendre();

-- Contraintes de valeur validées si les données existantes les respectent (sinon elles
-- restent NOT VALID : les nouvelles écritures sont déjà contrôlées).
DO $$
BEGIN
    BEGIN
        ALTER TABLE ent_eleve_identite VALIDATE CONSTRAINT ent_eleve_identite_valeurs_chk;
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        ALTER TABLE ent_eleve_identite VALIDATE CONSTRAINT ent_eleve_identite_regles_chk;
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END $$;

-- Recalcule la projection de tous les élèves dont la version courante a changé avec la
-- date (version annoncée devenue effective). Tâche quotidienne ; renvoie le nombre
-- d'élèves resynchronisés. Idempotente.
CREATE OR REPLACE FUNCTION ent_eleve_identite_resynchroniser(p_jour DATE DEFAULT current_date)
RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; r RECORD; v ent_eleve_identite;
BEGIN
    FOR r IN
        SELECT e.etablissement_id, e.id AS eleve_id,
               ent_eleve_identite_courante(e.etablissement_id, e.id, p_jour) AS courante
        FROM ent_eleve e
        WHERE e.identite_version_id IS DISTINCT FROM
              ent_eleve_identite_courante(e.etablissement_id, e.id, p_jour)
    LOOP
        CONTINUE WHEN r.courante IS NULL;
        SELECT * INTO v FROM ent_eleve_identite WHERE etablissement_id = r.etablissement_id AND id = r.courante;
        PERFORM ent_eleve_identite_projeter(v);
        n := n + 1;
    END LOOP;
    RETURN n;
END $$;

-- ───────────────────────── Isolation par établissement (RLS, mode permissif) ─────────────────────────
ALTER TABLE ent_eleve_identite ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_identite FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_identite;
CREATE POLICY ent_rls_etablissement ON ent_eleve_identite FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
