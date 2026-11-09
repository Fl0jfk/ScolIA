-- Socle 4, lot A2 : versions d'identité des personnes (ADM-03, ADM-10) et divergences entre le COMPTE de connexion et
-- la FICHE de la personne. Elles sont rapportées, jamais écrasées en silence : ni l'e-mail ou le
-- téléphone du compte ne remplace la fiche, ni l'inverse. Une personne (ou le secrétariat) prend
-- une décision explicite, tracée, qui reste valable tant que les deux valeurs ne changent pas.
-- Migration additive et rejouable.

-- Versions de l'identité d'une personne (§7, §8.1, R-A28) : nom, prénom, civilité, datés, en ajout
-- seul. Une CORRECTION (faute de frappe) et un CHANGEMENT (mariage, jugement, prénom d'usage) ne
-- sont pas la même chose. Un changement de prénom ou de civilité est « sensible » : les anciennes
-- valeurs sont scellées (AES-256-GCM, `valeurs_anterieures`) et lisibles avec `responsables.protege.lire`
-- seulement. La projection courante reste `ent_personne` (nom, prenom, civilite).
CREATE TABLE IF NOT EXISTS ent_personne_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    nom TEXT,
    prenom TEXT,
    civilite TEXT CHECK (civilite IS NULL OR civilite IN ('mme', 'm', 'autre')),
    nature TEXT NOT NULL CHECK (nature IN ('creation', 'reprise', 'correction', 'changement',
                                           'separation_noms')),
    effet_du DATE NOT NULL,
    remplace_version_id UUID,
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 3 AND 500),
    sensible BOOLEAN NOT NULL DEFAULT false,
    -- Anciennes valeurs d'un changement sensible : JSON scellé `enc:v1:…`.
    valeurs_anterieures TEXT,
    -- Version dont les valeurs en clair ont été retirées (scellées dans la version suivante).
    masquee BOOLEAN NOT NULL DEFAULT false,
    enregistre_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    enregistre_par UUID,
    CONSTRAINT ent_personne_version_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_personne_version_personne_idx
    ON ent_personne_version (etablissement_id, personne_id, enregistre_le);

-- Toute naissance d'une fiche, et tout changement d'identité fait hors des commandes du lot
-- (écrivain historique), laissent une version : la projection `ent_personne` ne bouge jamais
-- sans trace. Les commandes du lot posent `ent.identite_ecrite = on` et écrivent leur propre version.
CREATE OR REPLACE FUNCTION ent_personne_version_depuis_legacy() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    v_par UUID := NULLIF(NULLIF(current_setting('ent.coord_acteur', true), ''), '')::UUID;
    v_precedente UUID;
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO ent_personne_version
            (id, etablissement_id, personne_id, nom, prenom, civilite, nature, effet_du,
             enregistre_par)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.nom, NEW.prenom, NEW.civilite,
                'creation', ent_aujourdhui_etab(NEW.etablissement_id, now()), v_par);
        RETURN NULL;
    END IF;
    IF current_setting('ent.identite_ecrite', true) = 'on' THEN
        RETURN NULL;
    END IF;
    IF NEW.nom IS DISTINCT FROM OLD.nom OR NEW.prenom IS DISTINCT FROM OLD.prenom
       OR NEW.civilite IS DISTINCT FROM OLD.civilite THEN
        SELECT id INTO v_precedente FROM ent_personne_version
         WHERE personne_id = NEW.id ORDER BY enregistre_le DESC, id DESC LIMIT 1;
        INSERT INTO ent_personne_version
            (id, etablissement_id, personne_id, nom, prenom, civilite, nature, effet_du,
             remplace_version_id, enregistre_par)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.nom, NEW.prenom, NEW.civilite,
                'correction', ent_aujourdhui_etab(NEW.etablissement_id, now()), v_precedente, v_par);
    END IF;
    RETURN NULL;
END
$$;

-- Reprise unique : une version « reprise » par personne existante.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ent_coord_reprise_marque WHERE cle = '276_versions') THEN
        RETURN;
    END IF;
    INSERT INTO ent_personne_version
        (id, etablissement_id, personne_id, nom, prenom, civilite, nature, effet_du)
    SELECT gen_random_uuid(), p.etablissement_id, p.id, p.nom, p.prenom, p.civilite, 'reprise',
           p.created_at::DATE
      FROM ent_personne p
     WHERE NOT EXISTS (SELECT 1 FROM ent_personne_version v WHERE v.personne_id = p.id);
    INSERT INTO ent_coord_reprise_marque (cle) VALUES ('276_versions') ON CONFLICT DO NOTHING;
END
$$;

CREATE TABLE IF NOT EXISTS ent_personne_divergence_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    champ TEXT NOT NULL CHECK (champ IN ('email', 'telephone')),
    -- Empreinte (SHA-256) du couple de valeurs jugé : la décision cesse de valoir si l'une change.
    empreinte TEXT NOT NULL,
    choix TEXT NOT NULL CHECK (choix IN ('distinct_voulu', 'conserver_fiche', 'adopter_compte')),
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 3 AND 500),
    decide_par UUID,
    decide_via TEXT NOT NULL CHECK (decide_via IN ('personnel', 'personne')),
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_divergence_decision_uq UNIQUE (personne_id, champ),
    CONSTRAINT ent_personne_divergence_decision_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);

-- Divergences ouvertes, calculées à la lecture (aucune copie à tenir à jour : un compte dont
-- l'e-mail change par ailleurs, par exemple par la confirmation en deux temps, apparaît tout de
-- suite). `absent_de_la_fiche` : la valeur du compte n'est pas une coordonnée de la fiche ;
-- `different_du_principal` : elle l'est, mais les envois vont à une autre valeur.
CREATE OR REPLACE VIEW ent_personne_divergence_compte_v AS
WITH compte AS (
    SELECT p.etablissement_id, p.id AS personne_id, u.id AS user_id,
           NULLIF(lower(btrim(u.email)), '') AS email_compte,
           COALESCE(ent_tel_e164(u.telephone), NULLIF(btrim(u.telephone), '')) AS tel_compte
      FROM ent_personne p
      JOIN ent_user u ON u.id = p.user_id
     WHERE p.fusionnee_dans IS NULL
),
brut AS (
    SELECT c.etablissement_id, c.personne_id, c.user_id, 'email'::TEXT AS champ,
           c.email_compte AS valeur_compte,
           (SELECT valeur FROM ent_coord_projetees(c.etablissement_id, c.personne_id, 'email') LIMIT 1)
               AS valeur_fiche,
           EXISTS (SELECT 1 FROM ent_personne_coordonnee k
                    WHERE k.etablissement_id = c.etablissement_id AND k.personne_id = c.personne_id
                      AND k.au IS NULL AND k.type = 'email' AND k.valeur = c.email_compte) AS connue
      FROM compte c WHERE c.email_compte IS NOT NULL
    UNION ALL
    SELECT c.etablissement_id, c.personne_id, c.user_id, 'telephone'::TEXT,
           c.tel_compte,
           (SELECT valeur FROM ent_coord_projetees(c.etablissement_id, c.personne_id, 'telephone') LIMIT 1),
           EXISTS (SELECT 1 FROM ent_personne_coordonnee k
                    WHERE k.etablissement_id = c.etablissement_id AND k.personne_id = c.personne_id
                      AND k.au IS NULL AND ent_coord_groupe(k.type) = 'telephone'
                      AND k.valeur = c.tel_compte)
      FROM compte c WHERE c.tel_compte IS NOT NULL
),
ecart AS (
    SELECT b.*,
           CASE WHEN NOT b.connue THEN 'absent_de_la_fiche'
                WHEN b.valeur_fiche IS NOT NULL AND b.valeur_fiche <> b.valeur_compte
                    THEN 'different_du_principal' END AS nature
      FROM brut b
)
SELECT e.etablissement_id, e.personne_id, e.user_id, e.champ, e.nature,
       e.valeur_compte, e.valeur_fiche,
       encode(sha256(convert_to(e.champ || '|' || e.nature || '|' || e.valeur_compte || '|'
                                || COALESCE(e.valeur_fiche, ''), 'UTF8')), 'hex') AS empreinte
  FROM ecart e
 WHERE e.nature IS NOT NULL
   AND NOT EXISTS (
       SELECT 1 FROM ent_personne_divergence_decision d
        WHERE d.personne_id = e.personne_id AND d.champ = e.champ
          AND d.empreinte = encode(sha256(convert_to(e.champ || '|' || e.nature || '|'
                                   || e.valeur_compte || '|' || COALESCE(e.valeur_fiche, ''), 'UTF8')), 'hex'));

DROP TRIGGER IF EXISTS ent_personne_version_creation ON ent_personne;
CREATE TRIGGER ent_personne_version_creation
    AFTER INSERT ON ent_personne
    FOR EACH ROW EXECUTE FUNCTION ent_personne_version_depuis_legacy();
DROP TRIGGER IF EXISTS ent_personne_version_changement ON ent_personne;
CREATE TRIGGER ent_personne_version_changement
    AFTER UPDATE OF nom, prenom, civilite ON ent_personne
    FOR EACH ROW EXECUTE FUNCTION ent_personne_version_depuis_legacy();
