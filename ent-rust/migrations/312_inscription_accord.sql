-- Lot I4 (socle 3, §4.4, §8.4, INS-45 à INS-57) — accords des responsables.
--
-- Une **demande d'accord** part à un responsable ; il répond en ligne (lien à usage unique et code de
-- vérification, SANS compte : aucun compte n'est créé avant la validation, D-014 ; ou depuis son compte), ou
-- sur papier (scanné et saisi par le secrétariat, avec son nom). Un **désaccord** déclaré par un titulaire de
-- l'autorité parentale bloque le dossier (`bloque_desaccord`) : aucune décision, confirmation ni validation ;
-- il se lève avec une PIÈCE (accord écrit, décision de justice).
--
-- * `ent_inscription_accord` : l'état d'un accord attendu (un par dossier, responsable et objet).
-- * `ent_inscription_accord_trace` : ajout seul — chaque changement (qui, comment, quand, quelle pièce).
-- * `ent_inscription_accord_demande` : la demande envoyée (lien, code, expiration, essais). Le jeton et le
--   code se DÉRIVENT (HMAC) d'un secret serveur : rien de secret n'est stocké ici.
--
-- Le moteur de notifications du socle 8 prévient les familles qui ont un élève et des liens ; une famille
-- d'une admission n'en a pas encore : le lien part par la file d'envoi des inscriptions (`ent_inscription_envoi`).

CREATE TABLE IF NOT EXISTS ent_inscription_accord (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    declaration_id UUID NOT NULL REFERENCES ent_inscription_declaration_responsable (id) ON DELETE CASCADE,
    objet TEXT NOT NULL CHECK (objet IN ('candidature', 'admission', 'contrat', 'reinscription', 'depart')),
    reponse TEXT NOT NULL DEFAULT 'attendu' CHECK (reponse IN ('attendu', 'donne', 'refuse', 'dispense')),
    moyen TEXT CHECK (moyen IS NULL OR moyen IN ('en_ligne_code', 'en_ligne_compte', 'papier', 'depot_conjoint', 'decision_direction')),
    -- Accord papier, jugement, accord écrit qui lève un désaccord : la pièce du dossier.
    piece_id UUID,
    motif TEXT CHECK (motif IS NULL OR char_length(btrim(motif)) BETWEEN 3 AND 1000),
    repondu_le TIMESTAMPTZ,
    saisi_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_accord_uq UNIQUE (dossier_id, declaration_id, objet),
    CONSTRAINT ent_inscription_accord_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_accord_dossier_fk FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_inscription_accord_piece_fk FOREIGN KEY (etablissement_id, piece_id)
        REFERENCES ent_inscription_piece (etablissement_id, id) ON DELETE SET NULL (piece_id),
    -- Une réponse a un moyen et une date ; « attendu » n'en a pas ; une dispense est décidée et motivée.
    CONSTRAINT ent_inscription_accord_reponse_chk CHECK (
        (reponse = 'attendu' AND moyen IS NULL AND repondu_le IS NULL)
        OR (reponse IN ('donne', 'refuse') AND moyen IS NOT NULL AND repondu_le IS NOT NULL)
        OR (reponse = 'dispense' AND moyen = 'decision_direction' AND repondu_le IS NOT NULL AND motif IS NOT NULL)),
    -- Un accord papier a sa pièce : sans elle, rien ne prouve la réponse.
    CONSTRAINT ent_inscription_accord_papier_chk CHECK (moyen IS DISTINCT FROM 'papier' OR piece_id IS NOT NULL OR reponse = 'refuse')
);
CREATE INDEX IF NOT EXISTS ent_inscription_accord_dossier_idx
    ON ent_inscription_accord (etablissement_id, dossier_id, objet);
CREATE INDEX IF NOT EXISTS ent_inscription_accord_refus_idx
    ON ent_inscription_accord (etablissement_id, dossier_id) WHERE reponse = 'refuse';

CREATE TABLE IF NOT EXISTS ent_inscription_accord_trace (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    accord_id UUID NOT NULL,
    de_reponse TEXT NOT NULL,
    vers_reponse TEXT NOT NULL CHECK (vers_reponse IN ('attendu', 'donne', 'refuse', 'dispense')),
    moyen TEXT,
    -- Qui : un agent (identifiant), ou le responsable lui-même (libellé « responsable », sans compte).
    acteur_user_id UUID,
    acteur_type TEXT NOT NULL CHECK (acteur_type IN ('responsable', 'secretariat', 'direction', 'systeme')),
    piece_id UUID,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 1000),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_accord_trace_fk FOREIGN KEY (etablissement_id, accord_id)
        REFERENCES ent_inscription_accord (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_accord_trace_idx
    ON ent_inscription_accord_trace (etablissement_id, accord_id, le);
DROP TRIGGER IF EXISTS ent_inscription_accord_trace_ajout_seul_trg ON ent_inscription_accord_trace;
CREATE TRIGGER ent_inscription_accord_trace_ajout_seul_trg
    BEFORE UPDATE ON ent_inscription_accord_trace
    FOR EACH ROW EXECUTE FUNCTION ent_doc_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_inscription_accord_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    accord_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'envoyee' CHECK (etat IN ('envoyee', 'repondue', 'annulee', 'expiree')),
    expire_le TIMESTAMPTZ NOT NULL,
    -- Le code à 6 chiffres se dérive de (demande, compteur) : un nouvel envoi incrémente le compteur et rend
    -- l'ancien code inutile. Cinq essais, dix minutes.
    code_compteur SMALLINT NOT NULL DEFAULT 0 CHECK (code_compteur BETWEEN 0 AND 20),
    code_expire_le TIMESTAMPTZ,
    code_essais SMALLINT NOT NULL DEFAULT 0 CHECK (code_essais BETWEEN 0 AND 5),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    close_le TIMESTAMPTZ,
    CONSTRAINT ent_inscription_accord_demande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_accord_demande_fk FOREIGN KEY (etablissement_id, accord_id)
        REFERENCES ent_inscription_accord (etablissement_id, id) ON DELETE CASCADE
);
-- Une seule demande ouverte par accord.
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_accord_demande_ouverte_uq
    ON ent_inscription_accord_demande (etablissement_id, accord_id) WHERE etat = 'envoyee';

-- La file d'envoi porte aussi la demande d'accord et son code.
ALTER TABLE ent_inscription_envoi ADD COLUMN IF NOT EXISTS accord_demande_id UUID;
-- Le type d'envoi reste `information` (contrainte de 309, rejouée à chaque reprise : elle ne tolère pas de nouveaux types) ;
-- la demande d'accord et son code se reconnaissent à `accord_demande_id` et au préfixe de `cle` (`accord:`, `accord-code:`).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_envoi_accord_fk') THEN
        ALTER TABLE ent_inscription_envoi ADD CONSTRAINT ent_inscription_envoi_accord_fk
            FOREIGN KEY (etablissement_id, accord_demande_id)
            REFERENCES ent_inscription_accord_demande (etablissement_id, id) ON DELETE CASCADE;
    END IF;
END $$;

SELECT ent_rls_poser(ARRAY[
    'ent_inscription_piece_regle',
    'ent_inscription_piece_regle_condition',
    'ent_inscription_accord',
    'ent_inscription_accord_trace',
    'ent_inscription_accord_demande'
]);
