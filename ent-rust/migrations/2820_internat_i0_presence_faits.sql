-- Socle 41 (internat), lot I0 — migration 2820 : correctifs urgents de l'existant (066).
-- Réf. : docs/socles/41_INTERNAT.md §25.1 ; RI-01, RI-02, RI-10 ; R-IT3, R-IT5, R-IT10, R-IT13 ;
-- NJ-23 (deux appels divergents), NJ-54 (affectation non atomique) du registre des défauts Next.js.
--
-- Contenu :
--   1. `modifie_le` (E6) et `UNIQUE (etablissement_id, id)` sur les tables ent_internat_* ;
--   2. clés étrangères COMPOSITES (E2), posées NOT VALID puis validées ;
--   3. nuit métier (R-IT5) : fonction `ent_nuit_etab` — date du SOIR, dans le fuseau de l'établissement ;
--   4. présence en AJOUT SEUL : `ent_internat_presence_fait` (RI-02, R-IT13), reprise des lignes 066,
--      ancienne table `ent_internat_presence` rendue lecture seule ;
--   5. capacité d'une chambre atomique : verrou de ligne sur la chambre (RI-10, R-IT10).
-- Additive et rejouable (E8). Aucune colonne JSON (D-036), aucun BYTEA (E5).

-- ───────────────────────── 1. Horodatage de modification et unicité composite ─────────────────────────
ALTER TABLE ent_internat_chambre
    ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE ent_internat_affectation
    ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_chambre_etab_id_uq') THEN
        ALTER TABLE ent_internat_chambre
            ADD CONSTRAINT ent_internat_chambre_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_etab_id_uq') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_presence_etab_id_uq') THEN
        ALTER TABLE ent_internat_presence
            ADD CONSTRAINT ent_internat_presence_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_suivi_etab_id_uq') THEN
        ALTER TABLE ent_internat_suivi
            ADD CONSTRAINT ent_internat_suivi_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END
$$;

-- ───────────────────────── 2. Clés étrangères composites (E2) ─────────────────────────
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_eleve_cfk') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_eleve_cfk
            FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_affectation_chambre_cfk') THEN
        ALTER TABLE ent_internat_affectation
            ADD CONSTRAINT ent_internat_affectation_chambre_cfk
            FOREIGN KEY (etablissement_id, chambre_id) REFERENCES ent_internat_chambre (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_presence_eleve_cfk') THEN
        ALTER TABLE ent_internat_presence
            ADD CONSTRAINT ent_internat_presence_eleve_cfk
            FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_presence_affectation_cfk') THEN
        ALTER TABLE ent_internat_presence
            ADD CONSTRAINT ent_internat_presence_affectation_cfk
            FOREIGN KEY (etablissement_id, affectation_id) REFERENCES ent_internat_affectation (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_suivi_eleve_cfk') THEN
        ALTER TABLE ent_internat_suivi
            ADD CONSTRAINT ent_internat_suivi_eleve_cfk
            FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id) NOT VALID;
    END IF;
END
$$;
DO $$
DECLARE
    c RECORD;
BEGIN
    FOR c IN SELECT conrelid::regclass AS tbl, conname FROM pg_constraint
              WHERE NOT convalidated AND conname IN (
                  'ent_internat_affectation_eleve_cfk', 'ent_internat_affectation_chambre_cfk',
                  'ent_internat_presence_eleve_cfk', 'ent_internat_presence_affectation_cfk',
                  'ent_internat_suivi_eleve_cfk')
    LOOP
        EXECUTE format('ALTER TABLE %s VALIDATE CONSTRAINT %I', c.tbl, c.conname);
    END LOOP;
END
$$;

-- La nuit n'est jamais « effacée » : une affectation n'est pas supprimée (R-IT3), elle est close.
CREATE OR REPLACE FUNCTION ent_internat_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : suppression interdite (R-IT3), clore par une date de fin', TG_TABLE_NAME
        USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_internat_affectation_sans_suppression ON ent_internat_affectation;
CREATE TRIGGER ent_internat_affectation_sans_suppression
    BEFORE DELETE ON ent_internat_affectation
    FOR EACH ROW EXECUTE FUNCTION ent_internat_sans_suppression();

-- ───────────────────────── 3. Nuit métier (R-IT5, corrige DI-16) ─────────────────────────
-- La « nuit » est identifiée par la DATE DU SOIR, calculée dans le fuseau de l'établissement
-- (Europe/Paris par défaut), y compris aux changements d'heure : l'instant est converti en heure
-- murale, puis décalé de 12 h (midi → midi). Un geste à 02 h 30 le dimanche du changement d'heure
-- appartient à la nuit du samedi ; un geste à 13 h le dimanche, à la nuit du dimanche.
CREATE OR REPLACE FUNCTION ent_nuit_etab(etab UUID, instant TIMESTAMPTZ DEFAULT now())
RETURNS DATE
LANGUAGE sql STABLE
AS $$
    SELECT ((instant AT TIME ZONE COALESCE(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = etab),
        'Europe/Paris')) - INTERVAL '12 hours')::date
$$;

-- ───────────────────────── 4. Présence en ajout seul (RI-02, R-IT13) ─────────────────────────
-- Un pointage est un FAIT horodaté, jamais une valeur écrasée. La valeur courante est le fait le
-- plus récent. On corrige en AJOUTANT un fait (motif obligatoire pour une nuit passée).
CREATE TABLE IF NOT EXISTS ent_internat_presence_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    affectation_id UUID,
    -- Lien avec l'appel (table du lot I2, migration 2824 : la clé composite y sera posée).
    appel_id UUID,
    nuit DATE NOT NULL,
    marque TEXT NOT NULL CHECK (marque IN (
        'present', 'absent', 'excuse', 'activite', 'infirmerie', 'parti',
        'non_attendu_present', 'vu', 'retard', 'permis_sortie')),
    note TEXT NOT NULL DEFAULT '' CHECK (char_length(note) <= 500),
    retour_attendu TIMESTAMPTZ,
    a TIMESTAMPTZ NOT NULL DEFAULT now(),
    recu_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    appareil_id UUID,
    correction_de UUID,
    motif_correction_code TEXT CHECK (motif_correction_code IS NULL OR motif_correction_code IN (
        'oubli', 'erreur_saisie', 'information_tardive', 'rattrapage', 'autre', 'reprise')),
    motif_correction_texte TEXT CHECK (motif_correction_texte IS NULL
        OR char_length(motif_correction_texte) <= 300),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_presence_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_presence_fait_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_presence_fait_affectation_cfk FOREIGN KEY (etablissement_id, affectation_id)
        REFERENCES ent_internat_affectation (etablissement_id, id),
    CONSTRAINT ent_internat_presence_fait_correction_cfk FOREIGN KEY (etablissement_id, correction_de)
        REFERENCES ent_internat_presence_fait (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_internat_presence_fait_nuit_idx
    ON ent_internat_presence_fait (etablissement_id, nuit, eleve_id, recu_le DESC, id DESC);
CREATE INDEX IF NOT EXISTS ent_internat_presence_fait_eleve_idx
    ON ent_internat_presence_fait (etablissement_id, eleve_id, nuit DESC);

-- Ajout seul : ni UPDATE ni DELETE, y compris pour le rôle applicatif.
CREATE OR REPLACE FUNCTION ent_internat_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit) : corriger par un nouveau fait', TG_TABLE_NAME, TG_OP
        USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_internat_presence_fait_ajout_seul ON ent_internat_presence_fait;
CREATE TRIGGER ent_internat_presence_fait_ajout_seul
    BEFORE UPDATE OR DELETE ON ent_internat_presence_fait
    FOR EACH ROW EXECUTE FUNCTION ent_internat_ajout_seul();

-- Garde de date (défense en profondeur ; le service renvoie une erreur lisible avant) :
-- une nuit FUTURE est refusée ; une nuit PASSÉE exige un motif de correction. Les faits issus
-- de la reprise de 066 (motif « reprise ») échappent à la garde.
CREATE OR REPLACE FUNCTION ent_internat_presence_fait_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    courante DATE := ent_nuit_etab(NEW.etablissement_id, now());
BEGIN
    IF NEW.motif_correction_code = 'reprise' THEN
        RETURN NEW;
    END IF;
    IF NEW.nuit > courante THEN
        RAISE EXCEPTION 'pointage d''une nuit future refusé' USING ERRCODE = '22007';
    END IF;
    IF NEW.nuit < courante AND NEW.motif_correction_code IS NULL THEN
        RAISE EXCEPTION 'pointage d''une nuit passée : motif de correction obligatoire' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_presence_fait_garde ON ent_internat_presence_fait;
CREATE TRIGGER ent_internat_presence_fait_garde
    BEFORE INSERT ON ent_internat_presence_fait
    FOR EACH ROW EXECUTE FUNCTION ent_internat_presence_fait_garde();

-- Reprise des lignes de 066 en faits (id conservé : rejouable sans doublon).
INSERT INTO ent_internat_presence_fait
    (id, etablissement_id, eleve_id, affectation_id, nuit, marque, note, a, recu_le,
     par_user_id, motif_correction_code, cree_le)
SELECT p.id, p.etablissement_id, p.eleve_id, p.affectation_id, p.jour, p.statut, p.note,
       p.created_at, p.created_at, p.actor_user_id, 'reprise', p.created_at
  FROM ent_internat_presence p
ON CONFLICT (id) DO NOTHING;

-- L'ancienne table devient LECTURE SEULE (retrait au lot I14, sur décision et sauvegarde vérifiée).
CREATE OR REPLACE FUNCTION ent_internat_presence_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_internat_presence est en lecture seule : utiliser ent_internat_presence_fait (RI-02)'
        USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_internat_presence_lecture_seule ON ent_internat_presence;
CREATE TRIGGER ent_internat_presence_lecture_seule
    BEFORE INSERT OR UPDATE OR DELETE ON ent_internat_presence
    FOR EACH ROW EXECUTE FUNCTION ent_internat_presence_lecture_seule();

-- Valeur courante : le fait le plus récent par élève et par nuit.
CREATE OR REPLACE VIEW ent_internat_presence_courante_v WITH (security_invoker = true) AS
SELECT DISTINCT ON (etablissement_id, eleve_id, nuit)
       id, etablissement_id, eleve_id, affectation_id, nuit, marque, note, retour_attendu, a,
       recu_le, par_user_id, motif_correction_code, motif_correction_texte, correction_de
  FROM ent_internat_presence_fait
 ORDER BY etablissement_id, eleve_id, nuit, recu_le DESC, id DESC;

-- ───────────────────────── 5. Capacité atomique d'une chambre (RI-10, NJ-54) ─────────────────────────
-- Le verrou de ligne sur la chambre sérialise les affectations concurrentes ; l'occupation PIC sur
-- la période de la nouvelle affectation (et non le seul jour de début) est comparée à la capacité.
CREATE OR REPLACE FUNCTION ent_internat_affectation_capacite() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    cap SMALLINT;
    pic INTEGER;
BEGIN
    SELECT c.capacite INTO cap
      FROM ent_internat_chambre c
     WHERE c.etablissement_id = NEW.etablissement_id AND c.id = NEW.chambre_id
       FOR UPDATE;
    IF cap IS NULL THEN
        RAISE EXCEPTION 'chambre inconnue' USING ERRCODE = '23503';
    END IF;
    SELECT COALESCE(max(x.n), 0) INTO pic FROM (
        SELECT p.d,
               1 + (SELECT count(*) FROM ent_internat_affectation a
                     WHERE a.etablissement_id = NEW.etablissement_id
                       AND a.chambre_id = NEW.chambre_id
                       AND a.id <> NEW.id
                       AND a.debut <= p.d
                       AND (a.fin IS NULL OR a.fin >= p.d)) AS n
          FROM (SELECT NEW.debut AS d
                UNION
                SELECT a2.debut FROM ent_internat_affectation a2
                 WHERE a2.etablissement_id = NEW.etablissement_id
                   AND a2.chambre_id = NEW.chambre_id
                   AND a2.id <> NEW.id
                   AND a2.debut > NEW.debut
                   AND (NEW.fin IS NULL OR a2.debut <= NEW.fin)) p
    ) x;
    IF pic > cap THEN
        RAISE EXCEPTION 'chambre pleine (capacité %)', cap USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_affectation_capacite ON ent_internat_affectation;
CREATE TRIGGER ent_internat_affectation_capacite
    BEFORE INSERT OR UPDATE OF chambre_id, debut ON ent_internat_affectation
    FOR EACH ROW EXECUTE FUNCTION ent_internat_affectation_capacite();

SELECT ent_rls_poser(ARRAY['ent_internat_presence_fait']);
