-- Socle 2, lot S2-1 (D-020) : la personne est distincte du compte.
--
-- `ent_personne` : un responsable, un contact, ou l'élève majeur lui-même, dans UN
-- établissement (copie propre à l'établissement, D-013 / D-014). Le compte de
-- connexion (`ent_user`) est facultatif : parent sans e-mail, contact d'urgence.
-- `ent_eleve_lien` devient le lien de responsabilité personne ↔ élève :
--   * `personne_id` obligatoire (reprise des liens existants : une personne par
--     compte et par établissement) ;
--   * `user_id` devient facultatif et suit TOUJOURS le compte de la personne
--     (déclencheurs ci-dessous) : le code existant qui lit `user_id` reste juste.
-- Migration additive et rejouable : aucune donnée supprimée.

CREATE TABLE IF NOT EXISTS ent_personne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 120),
    prenom TEXT CHECK (prenom IS NULL OR length(prenom) <= 120),
    civilite TEXT CHECK (civilite IS NULL OR civilite IN ('mme', 'm', 'autre')),
    email TEXT CHECK (email IS NULL OR length(email) <= 254),
    telephone TEXT CHECK (telephone IS NULL OR length(telephone) <= 40),
    telephone_2 TEXT CHECK (telephone_2 IS NULL OR length(telephone_2) <= 40),
    adresse TEXT CHECK (adresse IS NULL OR length(adresse) <= 500),
    -- Compte de connexion rattaché (facultatif).
    user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    -- Personne représentant l'élève lui-même (élève majeur : payeur, contact).
    eleve_id UUID REFERENCES ent_eleve (id) ON DELETE CASCADE,
    -- Fusion de doublons (réversible, voir 123) : la personne absorbée pointe la cible.
    fusionnee_dans UUID REFERENCES ent_personne (id),
    source TEXT NOT NULL DEFAULT 'saisie'
        CHECK (source IN ('saisie', 'reprise', 'compte', 'preinscription')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_pas_soi_meme CHECK (fusionnee_dans IS NULL OR fusionnee_dans <> id)
);

-- Une seule personne par compte et par établissement ; une seule par élève.
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_compte_uq
    ON ent_personne (etablissement_id, user_id) WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_eleve_uq
    ON ent_personne (etablissement_id, eleve_id) WHERE eleve_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_personne_nom_idx
    ON ent_personne (etablissement_id, lower(nom), lower(coalesce(prenom, '')));

ALTER TABLE ent_eleve_lien
    ADD COLUMN IF NOT EXISTS personne_id UUID REFERENCES ent_personne (id);
ALTER TABLE ent_eleve_lien ALTER COLUMN user_id DROP NOT NULL;

-- La suppression d'un compte ne supprime plus le lien de responsabilité :
-- la personne (et son autorité) survit au compte.
ALTER TABLE ent_eleve_lien DROP CONSTRAINT IF EXISTS ent_eleve_lien_user_id_fkey;
ALTER TABLE ent_eleve_lien
    ADD CONSTRAINT ent_eleve_lien_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES ent_user (id) ON DELETE SET NULL;

-- Reprise (idempotente) : une personne par (établissement, compte) déjà lié.
-- Le nom affiché du compte est repris tel quel dans `nom` (aucun découpage
-- prénom/nom deviné) ; l'établissement corrige ensuite la fiche.
INSERT INTO ent_personne (id, etablissement_id, nom, email, telephone, user_id, source)
SELECT gen_random_uuid(), x.etablissement_id,
       left(coalesce(nullif(btrim(u.display_name), ''), u.email), 120),
       u.email, left(u.telephone, 40), u.id, 'reprise'
FROM (
    SELECT DISTINCT etablissement_id, user_id
    FROM ent_eleve_lien
    WHERE user_id IS NOT NULL AND personne_id IS NULL
) x
JOIN ent_user u ON u.id = x.user_id
WHERE NOT EXISTS (
    SELECT 1 FROM ent_personne p
    WHERE p.etablissement_id = x.etablissement_id AND p.user_id = x.user_id
);

UPDATE ent_eleve_lien l
SET personne_id = p.id
FROM ent_personne p
WHERE l.personne_id IS NULL
  AND p.etablissement_id = l.etablissement_id
  AND p.user_id = l.user_id;

-- Cohérence lien ↔ personne ↔ compte, garantie par la base :
--   * lien créé avec un compte seulement (code existant) → personne trouvée ou créée ;
--   * lien créé avec une personne → le compte du lien est celui de la personne ;
--   * personne d'un autre établissement → refus.
CREATE OR REPLACE FUNCTION ent_eleve_lien_personne_sync() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    pid UUID;
    uid UUID;
BEGIN
    IF NEW.personne_id IS NULL THEN
        IF NEW.user_id IS NULL THEN
            RAISE EXCEPTION 'lien de responsabilite sans personne ni compte'
                USING ERRCODE = '23502';
        END IF;
        SELECT id INTO pid FROM ent_personne
         WHERE etablissement_id = NEW.etablissement_id AND user_id = NEW.user_id;
        IF pid IS NULL THEN
            pid := gen_random_uuid();
            INSERT INTO ent_personne (id, etablissement_id, nom, email, telephone, user_id, source)
            SELECT pid, NEW.etablissement_id,
                   left(coalesce(nullif(btrim(u.display_name), ''), u.email), 120),
                   u.email, left(u.telephone, 40), u.id, 'compte'
            FROM ent_user u WHERE u.id = NEW.user_id;
        END IF;
        NEW.personne_id := pid;
    ELSE
        SELECT user_id INTO uid FROM ent_personne
         WHERE id = NEW.personne_id AND etablissement_id = NEW.etablissement_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'personne inconnue dans cet etablissement'
                USING ERRCODE = '23503';
        END IF;
        -- Compte en cours de suppression (actions des clés étrangères) : détaché.
        IF uid IS NOT NULL AND NOT EXISTS (SELECT 1 FROM ent_user WHERE id = uid) THEN
            uid := NULL;
        END IF;
        NEW.user_id := uid;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_eleve_lien_personne_sync_trg ON ent_eleve_lien;
CREATE TRIGGER ent_eleve_lien_personne_sync_trg
    BEFORE INSERT OR UPDATE OF user_id, personne_id ON ent_eleve_lien
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_lien_personne_sync();

CREATE OR REPLACE FUNCTION ent_personne_compte_sync() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.user_id IS DISTINCT FROM OLD.user_id THEN
        UPDATE ent_eleve_lien SET user_id = NEW.user_id
         WHERE personne_id = NEW.id AND etablissement_id = NEW.etablissement_id;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_personne_compte_sync_trg ON ent_personne;
CREATE TRIGGER ent_personne_compte_sync_trg
    AFTER UPDATE OF user_id ON ent_personne
    FOR EACH ROW EXECUTE FUNCTION ent_personne_compte_sync();

ALTER TABLE ent_eleve_lien ALTER COLUMN personne_id SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_lien_personne_uq
    ON ent_eleve_lien (etablissement_id, eleve_id, personne_id);
CREATE INDEX IF NOT EXISTS ent_eleve_lien_personne_idx
    ON ent_eleve_lien (etablissement_id, personne_id);
