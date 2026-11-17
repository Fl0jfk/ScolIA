-- Socle 32, lot ET1 — migration 2641 : espaces, ancrage, membres (`32_BUREAUTIQUE.md` §7, §8, §24, R-ET1, R-ET11, R-ET12, R-ET25).
--
-- `ent_espace` : lieu de travail daté ; ET1 n'ouvre que deux types (`personnel`, `libre_personnels`), les autres types
-- sont déjà admis par la contrainte pour ne pas rouvrir la table aux lots suivants (ET2, ET3). Les réglages propres aux
-- espaces avec élèves (ouverture aux élèves, archivage, commentaires, IA) arrivent avec la migration 2644 (ET2).
-- Les membres sont **calculés à la date** (ET2, ports 5 et 6) ou **ajoutés** nominativement, datés ; jamais recopiés
-- (R-ET1, ND32-03). Un membre est une `personne` (socle 2/4/6), du même établissement (clé composite).
-- `ent_espace_membre_effectif` : calculés − exclus + ajoutés ; en ET1 il n'y a que des ajoutés (le calcul des membres
-- d'un ancrage de structure est le lot ET2). Le filtre de date est celui de l'appelant (jour de l'établissement).
-- Sans JSON, sans tableau, sans BYTEA. Additif, rejouable.

CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE IF NOT EXISTS ent_espace (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type TEXT NOT NULL CHECK (type IN
        ('personnel', 'cours', 'groupe', 'classe', 'equipe', 'service', 'instance', 'delegues', 'projet', 'libre_personnels')),
    annee_id UUID,
    parent_espace_id UUID,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    description TEXT CHECK (description IS NULL OR length(description) <= 500),
    -- Catalogues fermés (R-ET5 : pas de texte libre, pas de téléversement d'icône).
    couleur_code TEXT NOT NULL DEFAULT 'bleu' CHECK (couleur_code IN
        ('bleu', 'vert', 'rouge', 'orange', 'violet', 'gris', 'rose', 'turquoise')),
    icone_code TEXT NOT NULL DEFAULT 'dossier' CHECK (icone_code IN
        ('dossier', 'livre', 'crayon', 'equipe', 'calendrier', 'etoile', 'ampoule', 'projet')),
    etat TEXT NOT NULL DEFAULT 'ouvert' CHECK (etat IN ('propose', 'ouvert', 'archive', 'efface')),
    quota_octets BIGINT NOT NULL CHECK (quota_octets >= 1048576),
    cree_par_personne_id UUID NOT NULL,
    -- Dernière activité (dépôt, version, partage, membre) : tri « activité récente » et revue annuelle (§8.4).
    derniere_activite_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_createur_fk FOREIGN KEY (etablissement_id, cree_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_parent_fk FOREIGN KEY (etablissement_id, parent_espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_espace_etab_type_idx ON ent_espace (etablissement_id, type, etat);

CREATE TABLE IF NOT EXISTS ent_espace_ancrage (
    espace_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type TEXT NOT NULL CHECK (type IN
        ('cours', 'groupe', 'classe', 'equipe', 'service', 'instance', 'delegues', 'sortie', 'personne')),
    -- Une seule référence non nulle, celle du type. Seule `personne_id` porte une clé étrangère en ET1 ; les autres
    -- références sont rattachées par les lots qui branchent l'ancrage (ET2 : cours, groupe ; ET3 : le reste).
    cours_id UUID,
    groupe_id UUID,
    classe_id UUID,
    equipe_id UUID,
    service_id UUID,
    instance_id UUID,
    sortie_id UUID,
    personne_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, espace_id),
    CONSTRAINT ent_espace_ancrage_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_ancrage_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_ancrage_une_reference_chk CHECK (
        (type = 'cours' AND cours_id IS NOT NULL AND num_nonnulls(groupe_id, classe_id, equipe_id, service_id, instance_id, sortie_id, personne_id) = 0)
     OR (type = 'groupe' AND groupe_id IS NOT NULL AND num_nonnulls(cours_id, classe_id, equipe_id, service_id, instance_id, sortie_id, personne_id) = 0)
     OR (type = 'classe' AND classe_id IS NOT NULL AND num_nonnulls(cours_id, groupe_id, equipe_id, service_id, instance_id, sortie_id, personne_id) = 0)
     OR (type = 'equipe' AND equipe_id IS NOT NULL AND num_nonnulls(cours_id, groupe_id, classe_id, service_id, instance_id, sortie_id, personne_id) = 0)
     OR (type = 'service' AND service_id IS NOT NULL AND num_nonnulls(cours_id, groupe_id, classe_id, equipe_id, instance_id, sortie_id, personne_id) = 0)
     OR (type IN ('instance', 'delegues') AND instance_id IS NOT NULL AND num_nonnulls(cours_id, groupe_id, classe_id, equipe_id, service_id, sortie_id, personne_id) = 0)
     OR (type = 'sortie' AND sortie_id IS NOT NULL AND num_nonnulls(cours_id, groupe_id, classe_id, equipe_id, service_id, instance_id, personne_id) = 0)
     OR (type = 'personne' AND personne_id IS NOT NULL AND num_nonnulls(cours_id, groupe_id, classe_id, equipe_id, service_id, instance_id, sortie_id) = 0))
);
-- Un seul espace personnel par personne.
CREATE UNIQUE INDEX IF NOT EXISTS ent_espace_ancrage_personne_uq
    ON ent_espace_ancrage (etablissement_id, personne_id) WHERE type = 'personne';

CREATE TABLE IF NOT EXISTS ent_espace_membre_ajoute (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('animateur', 'contributeur', 'lecteur')),
    debut DATE NOT NULL,
    fin DATE,
    motif_code TEXT NOT NULL CHECK (motif_code ~ '^[a-z0-9_]{1,40}$'),
    ajoute_par_personne_id UUID NOT NULL,
    -- Badge « invitation non vue » (NX32-19) : levé par une commande explicite, jamais par une lecture (ND32-11).
    vu_le TIMESTAMPTZ,
    -- Retrait immédiat (sortie, retrait par un animateur) : la ligne reste, l'historique aussi.
    retire_le TIMESTAMPTZ,
    retire_par_personne_id UUID,
    retire_motif_code TEXT CHECK (retire_motif_code IS NULL OR retire_motif_code ~ '^[a-z0-9_]{1,40}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_membre_ajoute_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_membre_ajoute_dates_chk CHECK (fin IS NULL OR fin >= debut),
    CONSTRAINT ent_espace_membre_ajoute_retrait_chk CHECK ((retire_le IS NULL) = (retire_par_personne_id IS NULL)),
    CONSTRAINT ent_espace_membre_ajoute_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_membre_ajoute_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_membre_ajoute_par_fk FOREIGN KEY (etablissement_id, ajoute_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    -- Une personne n'a qu'une appartenance active par espace et par période (le changement de rôle remplace la ligne).
    CONSTRAINT ent_espace_membre_ajoute_pas_de_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, espace_id WITH =, personne_id WITH =, daterange(debut, fin, '[]') WITH &&)
        WHERE (retire_le IS NULL)
);
CREATE INDEX IF NOT EXISTS ent_espace_membre_ajoute_personne_idx
    ON ent_espace_membre_ajoute (etablissement_id, personne_id) WHERE retire_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_espace_membre_exclu (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    debut DATE NOT NULL,
    fin DATE,
    motif_code TEXT NOT NULL CHECK (motif_code ~ '^[a-z0-9_]{1,40}$'),
    exclu_par_personne_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_membre_exclu_dates_chk CHECK (fin IS NULL OR fin >= debut),
    CONSTRAINT ent_espace_membre_exclu_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_membre_exclu_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_membre_exclu_par_fk FOREIGN KEY (etablissement_id, exclu_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);

-- Règle par type (jamais une liste de personnes) : alimentée par ET2.
CREATE TABLE IF NOT EXISTS ent_espace_role_calcule (
    espace_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    source TEXT NOT NULL CHECK (source IN ('enseignant', 'eleve', 'membre_equipe', 'membre_instance', 'pp', 'cpe', 'aesh')),
    role TEXT NOT NULL CHECK (role IN ('animateur', 'contributeur', 'lecteur')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, espace_id, source),
    CONSTRAINT ent_espace_role_calcule_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT
);

-- Clés étrangères de la migration 2640 (la table `ent_espace` n'existait pas encore).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_espace_occupation_espace_fk') THEN
        ALTER TABLE ent_espace_occupation ADD CONSTRAINT ent_espace_occupation_espace_fk
            FOREIGN KEY (etablissement_id, espace_id) REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_espace_reservation_espace_fk') THEN
        ALTER TABLE ent_espace_reservation ADD CONSTRAINT ent_espace_reservation_espace_fk
            FOREIGN KEY (etablissement_id, espace_id) REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END $$;

-- Membres effectifs (R-ET11) : en ET1, les membres ajoutés non retirés ; le jour est filtré par l'appelant.
CREATE OR REPLACE VIEW ent_espace_membre_effectif WITH (security_invoker = true) AS
SELECT a.etablissement_id, a.espace_id, a.personne_id, a.role, 'ajoutee'::text AS origine, a.debut, a.fin
  FROM ent_espace_membre_ajoute a
 WHERE a.retire_le IS NULL;

SELECT ent_rls_poser(ARRAY[
    'ent_espace', 'ent_espace_ancrage', 'ent_espace_membre_ajoute', 'ent_espace_membre_exclu', 'ent_espace_role_calcule']);
