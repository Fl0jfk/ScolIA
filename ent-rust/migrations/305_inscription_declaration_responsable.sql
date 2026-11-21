-- Lot I2 (socle 3, §8.4, §16 M5) — déclarations de responsables.
--
-- Une déclaration = ce que la famille déclare d'un responsable, **une fois par démarche**
-- (et non plus une fois par dossier) : identité, lien, autorité parentale, foyer, enfants
-- concernés. Elle ne devient une personne qu'à la validation, par la commande du socle 4
-- (`integrer_declaration`) : le socle 3 ne crée aucune personne ici.
--
-- Les lignes `ent_inscription_responsable` (par dossier) restent la projection lue par la
-- validation et les envois jusqu'au lot I6 ; un déclencheur tient la déclaration d'accord avec
-- elles (même chemin pour la reprise, l'insertion et la mise à jour : la fonction
-- `ent_inscription_declaration_sync`). Les valeurs scellées (adresse à protéger, référence de
-- « ne pas solliciter ») restent **scellées sur la ligne du dossier** : le chiffrement est lié à
-- la ligne (table, colonne, identifiant) ; la déclaration n'en garde que l'indicateur.
--
-- N déclarations : tuteur + deux parents, famille recomposée, aide sociale à l'enfance (les
-- rangs 1 à 6 sont admis au lieu de 1 et 2). Un beau-parent peut ne concerner qu'un enfant de
-- la fratrie : `pour_enfants` liste les dossiers concernés.

ALTER TABLE ent_inscription_responsable DROP CONSTRAINT IF EXISTS ent_inscription_responsable_rang_check;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_responsable_rang_i2_chk') THEN
        ALTER TABLE ent_inscription_responsable
            ADD CONSTRAINT ent_inscription_responsable_rang_i2_chk CHECK (rang BETWEEN 1 AND 6);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS ent_inscription_declaration_responsable (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demarche_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre BETWEEN 1 AND 6),
    est_deposant BOOLEAN NOT NULL DEFAULT false,
    civilite TEXT CHECK (civilite IS NULL OR civilite IN ('mme', 'm', 'autre')),
    nom TEXT NOT NULL CHECK (char_length(btrim(nom)) BETWEEN 1 AND 120),
    prenom TEXT CHECK (prenom IS NULL OR char_length(prenom) <= 120),
    email TEXT CHECK (email IS NULL OR char_length(email) <= 254),
    telephone TEXT CHECK (telephone IS NULL OR char_length(telephone) <= 40),
    adresse TEXT CHECK (adresse IS NULL OR char_length(adresse) <= 500),
    lien_code TEXT NOT NULL DEFAULT 'autre' CHECK (lien_code IN ('mere', 'pere', 'tuteur', 'autre')),
    autorite_parentale TEXT NOT NULL DEFAULT 'oui' CHECK (autorite_parentale IN ('oui', 'non', 'inconnue')),
    autorite_exclusive_declaree BOOLEAN NOT NULL DEFAULT false,
    foyer_cle TEXT NOT NULL DEFAULT 'A' CHECK (foyer_cle ~ '^[A-Z]$'),
    payeur_propose BOOLEAN NOT NULL DEFAULT false,
    pour_enfants UUID[] NOT NULL DEFAULT '{}',
    personne_id UUID REFERENCES ent_personne (id) ON DELETE SET NULL,
    ne_pas_solliciter BOOLEAN NOT NULL DEFAULT false,
    adresse_a_proteger BOOLEAN NOT NULL DEFAULT false,
    langue TEXT,
    besoin_interprete BOOLEAN NOT NULL DEFAULT false,
    organisme TEXT,
    nature_code TEXT,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_declaration_ordre_uq UNIQUE (demarche_id, ordre),
    CONSTRAINT ent_inscription_declaration_demarche_fk
        FOREIGN KEY (etablissement_id, demarche_id)
        REFERENCES ent_inscription_demarche (etablissement_id, id) ON DELETE CASCADE,
    -- D-014 : un foyer différent de celui du déposant impose un e-mail propre.
    CONSTRAINT ent_inscription_declaration_foyer_email_chk
        CHECK (foyer_cle = 'A' OR email IS NOT NULL OR ne_pas_solliciter)
);
CREATE INDEX IF NOT EXISTS ent_inscription_declaration_demarche_idx
    ON ent_inscription_declaration_responsable (etablissement_id, demarche_id);

-- Tient la déclaration d'accord avec la ligne du dossier (insertion, mise à jour, reprise).
CREATE OR REPLACE FUNCTION ent_inscription_declaration_sync(r ent_inscription_responsable) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_demarche UUID;
BEGIN
    SELECT demarche_id INTO v_demarche FROM ent_inscription_dossier
     WHERE id = r.dossier_id AND etablissement_id = r.etablissement_id;
    IF v_demarche IS NULL THEN
        RETURN;
    END IF;
    INSERT INTO ent_inscription_declaration_responsable AS x
        (id, etablissement_id, demarche_id, ordre, est_deposant, civilite, nom, prenom, email,
         telephone, adresse, lien_code, autorite_parentale, foyer_cle, pour_enfants, personne_id,
         ne_pas_solliciter, adresse_a_proteger, langue, besoin_interprete, organisme, nature_code)
    VALUES (gen_random_uuid(), r.etablissement_id, v_demarche, r.rang, r.rang = 1, r.civilite,
            r.nom, r.prenom, r.email, r.telephone,
            CASE WHEN r.adresse_a_proteger THEN NULL ELSE r.adresse END,
            r.lien, CASE WHEN r.autorite_parentale THEN 'oui' ELSE 'non' END,
            CASE WHEN r.rang = 1 OR r.meme_foyer THEN 'A' ELSE 'B' END,
            ARRAY[r.dossier_id], r.personne_id, r.ne_pas_solliciter, r.adresse_a_proteger,
            r.langue, r.besoin_interprete, r.organisme, r.nature_code)
    ON CONFLICT (demarche_id, ordre) DO UPDATE SET
        civilite = EXCLUDED.civilite, nom = EXCLUDED.nom, prenom = EXCLUDED.prenom,
        email = EXCLUDED.email, telephone = EXCLUDED.telephone, adresse = EXCLUDED.adresse,
        lien_code = EXCLUDED.lien_code, autorite_parentale = EXCLUDED.autorite_parentale,
        foyer_cle = EXCLUDED.foyer_cle,
        pour_enfants = CASE WHEN r.dossier_id = ANY (x.pour_enfants) THEN x.pour_enfants
                            ELSE array_append(x.pour_enfants, r.dossier_id) END,
        personne_id = COALESCE(EXCLUDED.personne_id, x.personne_id),
        -- La marque « à ne pas solliciter » ne se perd jamais par une réécriture.
        ne_pas_solliciter = x.ne_pas_solliciter OR EXCLUDED.ne_pas_solliciter,
        adresse_a_proteger = x.adresse_a_proteger OR EXCLUDED.adresse_a_proteger,
        langue = EXCLUDED.langue, besoin_interprete = EXCLUDED.besoin_interprete,
        organisme = EXCLUDED.organisme, nature_code = EXCLUDED.nature_code,
        modifie_le = now();
END
$$;

CREATE OR REPLACE FUNCTION ent_inscription_responsable_apres_ecriture() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM ent_inscription_declaration_sync(NEW);
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS ent_inscription_responsable_declaration_trg ON ent_inscription_responsable;
CREATE TRIGGER ent_inscription_responsable_declaration_trg
    AFTER INSERT OR UPDATE ON ent_inscription_responsable
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_responsable_apres_ecriture();

-- Reprise : une déclaration par ligne reprise (idempotente : conflit sur (démarche, ordre)).
DO $$
DECLARE
    r ent_inscription_responsable;
BEGIN
    FOR r IN SELECT * FROM ent_inscription_responsable ORDER BY dossier_id, rang LOOP
        PERFORM ent_inscription_declaration_sync(r);
    END LOOP;
END $$;
