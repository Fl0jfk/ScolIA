-- Lot I3 (socle 3, §8.6, INS-33 à INS-38) — règles de pièces par campagne, conditions typées, état
-- des pièces d'un dossier.
--
-- Additif et rejouable. Le catalogue des TYPES de pièces est celui du socle 7 (`documents_pieces::CATALOGUE`,
-- livré par ScolIA, fermé) : le socle 3 ne le double pas. Le socle 3 garde l'EXIGENCE de sa démarche
-- (quelle pièce, quand, bloquante ou non, sous quelle condition) et l'état de conformité du dossier ; le
-- fichier, le contrôle du type réel, la conversion en PDF, l'antivirus et la purge passent par le socle 7.
--
-- * `ent_inscription_piece_regle` : paramétrage par campagne, **seule source** des pièces demandées
--   (reprise de `ent_inscription_campagne_piece`, qui n'est plus lue). Une règle dit « cette pièce, à ce
--   moment, bloquante ou non, déposée une fois par dossier ou une fois pour toute la démarche (pièce de
--   famille) ».
-- * `ent_inscription_piece_regle_condition` : les conditions sont des LIGNES typées (D-036 : pas de JSON qui
--   sert de mémoire). Toutes les clés d'une règle doivent être vraies (ET) ; plusieurs lignes de même clé
--   textuelle valent « l'une ou l'autre » (OU : `niveau`).
-- * `ent_inscription_piece` : une pièce attendue d'un dossier, calculée depuis les règles à la création du
--   dossier puis à chaque changement de situation (responsables de foyers distincts, tuteur…).

CREATE TABLE IF NOT EXISTS ent_inscription_piece_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL,
    code_type TEXT NOT NULL CHECK (code_type ~ '^[a-z][a-z0-9_]{0,39}$'),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    consigne TEXT CHECK (consigne IS NULL OR char_length(consigne) <= 500),
    moment TEXT NOT NULL DEFAULT 'depot' CHECK (moment IN ('depot', 'admission', 'confirmation', 'rentree')),
    -- Empêche la confirmation ou la validation tant qu'elle n'est pas acceptée (ou dispensée).
    bloquante BOOLEAN NOT NULL DEFAULT true,
    -- `demarche` : pièce de famille (livret, justificatif de domicile), déposée UNE fois pour tous les enfants.
    portee TEXT NOT NULL DEFAULT 'dossier' CHECK (portee IN ('dossier', 'demarche')),
    ordre SMALLINT NOT NULL DEFAULT 0,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_piece_regle_code_uq UNIQUE (campagne_id, code_type),
    CONSTRAINT ent_inscription_piece_regle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_piece_regle_campagne_fk
        FOREIGN KEY (etablissement_id, campagne_id)
        REFERENCES ent_inscription_campagne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_piece_regle_campagne_idx
    ON ent_inscription_piece_regle (etablissement_id, campagne_id, ordre);

CREATE TABLE IF NOT EXISTS ent_inscription_piece_regle_condition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    regle_id UUID NOT NULL,
    -- foyers_distincts, autorite_exclusive, responsable_tuteur, autre_etablissement : valeur_bool.
    -- niveau, regime, nature : valeur_texte (plusieurs lignes = l'une ou l'autre).
    cle TEXT NOT NULL CHECK (cle IN ('foyers_distincts', 'autorite_exclusive', 'responsable_tuteur',
                                      'autre_etablissement', 'niveau', 'regime', 'nature')),
    valeur_bool BOOLEAN,
    valeur_texte TEXT CHECK (valeur_texte IS NULL OR char_length(valeur_texte) BETWEEN 1 AND 40),
    CONSTRAINT ent_inscription_piece_regle_condition_valeur_chk CHECK (
        (cle IN ('niveau', 'regime', 'nature') AND valeur_texte IS NOT NULL AND valeur_bool IS NULL)
        OR (cle NOT IN ('niveau', 'regime', 'nature') AND valeur_bool IS NOT NULL AND valeur_texte IS NULL)),
    CONSTRAINT ent_inscription_piece_regle_condition_fk
        FOREIGN KEY (etablissement_id, regle_id)
        REFERENCES ent_inscription_piece_regle (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_piece_regle_condition_idx
    ON ent_inscription_piece_regle_condition (etablissement_id, regle_id);

-- Reprise : chaque pièce d'une campagne devient une règle sans condition (déposée au dépôt, bloquante si
-- elle était obligatoire). Rejouable (conflit sur la campagne et le code).
INSERT INTO ent_inscription_piece_regle
    (id, etablissement_id, campagne_id, code_type, libelle, moment, bloquante, ordre)
SELECT gen_random_uuid(), p.etablissement_id, p.campagne_id, p.code, p.libelle, 'depot', p.obligatoire, p.ordre
  FROM ent_inscription_campagne_piece p
  JOIN ent_inscription_campagne c ON c.etablissement_id = p.etablissement_id AND c.id = p.campagne_id
 WHERE p.code ~ '^[a-z][a-z0-9_]{0,39}$'
ON CONFLICT (campagne_id, code_type) DO NOTHING;

-- Pièce attendue d'un dossier (141, étendue).
ALTER TABLE ent_inscription_piece
    ADD COLUMN IF NOT EXISTS demarche_id UUID,
    ADD COLUMN IF NOT EXISTS regle_id UUID,
    ADD COLUMN IF NOT EXISTS moment TEXT NOT NULL DEFAULT 'depot',
    ADD COLUMN IF NOT EXISTS bloquante BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS portee TEXT NOT NULL DEFAULT 'dossier',
    -- Faux quand la condition de la règle n'est plus vraie : la ligne reste (historique), elle ne compte plus.
    ADD COLUMN IF NOT EXISTS applicable BOOLEAN NOT NULL DEFAULT true,
    -- Le fichier : une pièce du socle 7 (quarantaine, antivirus, type réel, PDF désarmé).
    ADD COLUMN IF NOT EXISTS piece_doc_id UUID,
    -- « Déjà fournie pour le frère » : la pièce d'un autre dossier de la même démarche, confirmée.
    ADD COLUMN IF NOT EXISTS reprise_de_piece_id UUID,
    ADD COLUMN IF NOT EXISTS dispense_motif TEXT,
    ADD COLUMN IF NOT EXISTS dispense_par UUID,
    ADD COLUMN IF NOT EXISTS dispense_le TIMESTAMPTZ,
    -- Rattachement à l'élève à la validation (socle 7) : la pièce rattachée et le jour.
    ADD COLUMN IF NOT EXISTS rattachee_doc_id UUID,
    ADD COLUMN IF NOT EXISTS rattachee_le TIMESTAMPTZ,
    -- Purge des pièces d'un dossier non abouti : le fichier est détruit, la ligne reste (preuve).
    ADD COLUMN IF NOT EXISTS purgee_le TIMESTAMPTZ;

DO $$
BEGIN
    ALTER TABLE ent_inscription_piece DROP CONSTRAINT IF EXISTS ent_inscription_piece_statut_check;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_statut_i3_chk') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_statut_i3_chk
            CHECK (statut IN ('attendue', 'deposee', 'acceptee', 'refusee', 'dispensee', 'reprise'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_moment_chk') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_moment_chk
            CHECK (moment IN ('depot', 'admission', 'confirmation', 'rentree')
                   AND portee IN ('dossier', 'demarche')
                   AND (dispense_motif IS NULL OR char_length(dispense_motif) BETWEEN 3 AND 500));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_dispense_chk') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_dispense_chk
            CHECK (statut <> 'dispensee' OR (dispense_motif IS NOT NULL AND dispense_par IS NOT NULL));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_etab_id_uq') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END $$;

-- Démarche des pièces déjà existantes (celle du dossier).
UPDATE ent_inscription_piece p SET demarche_id = d.demarche_id
  FROM ent_inscription_dossier d
 WHERE p.demarche_id IS NULL AND d.etablissement_id = p.etablissement_id AND d.id = p.dossier_id;

-- Règle de chaque pièce existante (même code dans la campagne du dossier).
UPDATE ent_inscription_piece p SET regle_id = r.id, bloquante = r.bloquante, moment = r.moment, portee = r.portee
  FROM ent_inscription_dossier d
  JOIN ent_inscription_piece_regle r ON r.etablissement_id = d.etablissement_id AND r.campagne_id = d.campagne_id
 WHERE p.regle_id IS NULL AND d.etablissement_id = p.etablissement_id AND d.id = p.dossier_id
   AND r.code_type = p.code;

CREATE INDEX IF NOT EXISTS ent_inscription_piece_demarche_idx
    ON ent_inscription_piece (etablissement_id, demarche_id, code) WHERE demarche_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_inscription_piece_doc_idx
    ON ent_inscription_piece (etablissement_id, piece_doc_id) WHERE piece_doc_id IS NOT NULL;
