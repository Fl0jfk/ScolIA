-- 523 — Finances, lot F3 : catalogue d'articles qualifiés, grilles tarifaires versionnées et votées,
-- tranches de quotient, simulateur de scénarios, familles types (socle 24, §5.1, §5.3, §10.1, §10.2,
-- §23, MIG-6).
--
-- Réf. : docs/socles/24_FINANCES.md R-F9 (nature légale et régime de TVA obligatoires), R-F10 (grille
-- votée figée, correction par nouvelle version datée), R-F23 (paramétrer ≠ voter), FD-17, FD-19,
-- FAC-103 à FAC-106, FAC-116 à FAC-129, FAC-196, MIG-6.
--
-- * **Article qualifié** : la nature légale (R. 442-48) et le régime de TVA se **posent** ou se
--   **valident** par la comptabilité (`qualifie_le`). Les articles existants ne sont **pas** modifiés
--   (aucune donnée monétaire touchée) : ils restent « non qualifiés » avec une proposition par règle côté
--   application. Une fois qualifiée, la nature d'un article qui porte des lignes ne change plus (FAC-104).
-- * **Version de grille** : brouillon → proposée → votée → en vigueur → close (+ abandonnée). Les
--   transitions sont contrôlées en base ; une version non brouillon ne se modifie plus, ses tarifs et ses
--   tranches non plus (déclencheurs) : une correction est une nouvelle version datée (FD-19, R-F10).
-- * Aucun JSON métier (D-036), aucun `ON DELETE CASCADE`, RLS ENABLE + FORCE sur chaque table.
-- * **Effacement d'un utilisateur (F8)** : les colonnes d'auteur de ce lot (`cree_par`, `propose_par`, `vote_par`,
--   `en_vigueur_par`, `qualifie_par`, `modifie_par`, `calcule_par`, `second_regard_desactive_par`, `created_by`)
--   référencent `ent_user` sans cascade. Un utilisateur qui a agi sur des grilles, des tarifs ou le catalogue ne
--   se supprime **jamais** physiquement : son compte est **anonymisé** (identité et identifiants effacés, la ligne
--   `ent_user` et son identifiant restent) ; les références d'auteur sont conservées, car elles prouvent le second
--   regard et le vote (R-F23) et font partie de la preuve comptable (conservation `fin.piece`, 10 ans).

-- ── Paramètres : exigence de qualification, second regard sur le vote ───────────────────────────────
-- Valeurs prudentes (D-069). Second regard activé. **Exigence de qualification à l'émission : vraie par défaut
-- pour tout établissement créé après la 523 et quand la ligne de paramètres est absente (M7)** ; fausse
-- seulement pour les établissements qui existent au moment de la migration (transition MIG-6 : qualifier
-- d'abord, exiger ensuite), avec une alerte « à activer avant la rentrée 2027 » côté direction.
ALTER TABLE ent_fin_parametres
    ADD COLUMN IF NOT EXISTS exiger_qualification_articles BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS grille_second_regard BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS second_regard_desactive_le TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS second_regard_desactive_par UUID REFERENCES ent_user (id);
-- Les établissements existants reçoivent leur ligne **avec** la valeur transitoire (faux) ; les suivants, créés
-- après, prennent le défaut « vrai ».
INSERT INTO ent_fin_parametres (etablissement_id)
SELECT e.id FROM ent_etablissement e
ON CONFLICT (etablissement_id) DO NOTHING;
ALTER TABLE ent_fin_parametres ALTER COLUMN exiger_qualification_articles SET DEFAULT true;

-- Le second regard ne se désactive pas pendant qu'une grille est proposée (règle 3, vérifié aussi en base).
CREATE OR REPLACE FUNCTION ent_trg_fin_parametres_second_regard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT NEW.grille_second_regard AND (TG_OP = 'INSERT' OR OLD.grille_second_regard)
       AND EXISTS (SELECT 1 FROM ent_fin_grille_version g
                    WHERE g.etablissement_id = NEW.etablissement_id AND g.statut = 'proposee') THEN
        RAISE EXCEPTION 'second regard : désactivation interdite pendant qu''une grille est proposée'
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END
$$;

-- ── Articles enrichis (§10.1) ────────────────────────────────────────────────────────────────────
ALTER TABLE ent_fin_article
    ADD COLUMN IF NOT EXISTS nature_legale TEXT
        CHECK (nature_legale IS NULL OR nature_legale IN (
            'contribution_r442_48_1', 'contribution_r442_48_2', 'demi_pension', 'internat',
            'externat_surveille', 'prestation_facultative', 'sortie_voyage', 'fournitures',
            'frais_dossier', 'autre_motivee')),
    ADD COLUMN IF NOT EXISTS regime_tva TEXT
        CHECK (regime_tva IS NULL OR regime_tva IN (
            'exonere_261_4_4', 'taxable', 'hors_champ', 'pour_compte_tiers')),
    ADD COLUMN IF NOT EXISTS taux_tva_bps INTEGER
        CHECK (taux_tva_bps IS NULL OR taux_tva_bps BETWEEN 1 AND 2500),
    ADD COLUMN IF NOT EXISTS unite TEXT
        CHECK (unite IS NULL OR unite IN ('annee', 'mois', 'repas', 'nuit', 'seance', 'unite')),
    ADD COLUMN IF NOT EXISTS mode TEXT NOT NULL DEFAULT 'forfait'
        CHECK (mode IN ('forfait', 'reel')),
    ADD COLUMN IF NOT EXISTS remboursable BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS soumis_reductions BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS soumis_remise_ordre BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS modele_code TEXT CHECK (modele_code IS NULL OR length(modele_code) <= 40),
    ADD COLUMN IF NOT EXISTS qualifie_le TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS qualifie_par UUID REFERENCES ent_user (id);

ALTER TABLE ent_fin_article DROP CONSTRAINT IF EXISTS ent_fin_article_qualification_chk;
ALTER TABLE ent_fin_article ADD CONSTRAINT ent_fin_article_qualification_chk CHECK (
    -- un article qualifié porte une nature, un régime et une unité
    (qualifie_le IS NULL OR (nature_legale IS NOT NULL AND regime_tva IS NOT NULL AND unite IS NOT NULL))
    -- le taux n'existe que pour un régime taxable, et un régime taxable porte son taux (FD-17)
    AND ((regime_tva IS NULL AND taux_tva_bps IS NULL)
         OR (regime_tva IS NOT NULL AND ((regime_tva = 'taxable') = (taux_tva_bps IS NOT NULL))))
);

-- FAC-104 : nature, régime et catégorie d'un article qualifié ne changent plus dès qu'il porte des lignes
-- (on crée un nouvel article) ; la qualification ne se retire jamais.
CREATE OR REPLACE FUNCTION ent_trg_fin_article_qualification() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.qualifie_le IS NOT NULL THEN
        IF NEW.qualifie_le IS NULL THEN
            RAISE EXCEPTION 'article % : la qualification ne se retire pas', OLD.code
                USING ERRCODE = 'restrict_violation';
        END IF;
        IF (NEW.nature_legale IS DISTINCT FROM OLD.nature_legale
            OR NEW.regime_tva IS DISTINCT FROM OLD.regime_tva
            OR NEW.taux_tva_bps IS DISTINCT FROM OLD.taux_tva_bps
            OR NEW.categorie IS DISTINCT FROM OLD.categorie)
           AND EXISTS (SELECT 1 FROM ent_fac_ligne l
                        WHERE l.etablissement_id = OLD.etablissement_id AND l.article_id = OLD.id) THEN
            RAISE EXCEPTION 'article % : nature, TVA et catégorie figées (des lignes existent) : créer un nouvel article', OLD.code
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_fin_article_qualification ON ent_fin_article;
CREATE TRIGGER ent_fin_article_qualification BEFORE UPDATE ON ent_fin_article
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_article_qualification();

-- ── Versions de grille (§10.2) ─────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_grille_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_scolaire TEXT NOT NULL CHECK (annee_scolaire ~ '^[0-9]{4}-[0-9]{4}$'),
    numero INTEGER NOT NULL CHECK (numero >= 1),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    statut TEXT NOT NULL DEFAULT 'brouillon'
        CHECK (statut IN ('brouillon', 'proposee', 'votee', 'en_vigueur', 'close', 'abandonnee', 'supplantee')),
    -- Correction d'une grille votée (FAC-128) : version remplacée, motif, effet rétroactif seulement proposé.
    remplace_id UUID,
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    effet_retroactif BOOLEAN NOT NULL DEFAULT false,
    cree_par UUID REFERENCES ent_user (id),
    propose_le TIMESTAMPTZ,
    propose_par UUID REFERENCES ent_user (id),
    -- Vote (FAC-126) : date et référence de la délibération, pièce au socle 7.
    vote_le DATE,
    vote_reference TEXT CHECK (vote_reference IS NULL OR length(btrim(vote_reference)) BETWEEN 1 AND 200),
    vote_document_id UUID,
    vote_par UUID REFERENCES ent_user (id),
    en_vigueur_du DATE,
    en_vigueur_par UUID REFERENCES ent_user (id),
    close_le TIMESTAMPTZ,
    publiee_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_grille_version_numero_uq UNIQUE (etablissement_id, annee_scolaire, numero),
    CONSTRAINT ent_fin_grille_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_fin_grille_version_remplace_fk
        FOREIGN KEY (etablissement_id, remplace_id) REFERENCES ent_fin_grille_version (etablissement_id, id),
    -- une version votée, en vigueur ou close porte son vote (date + référence + auteur)
    CONSTRAINT ent_fin_grille_version_vote_chk CHECK (
        statut NOT IN ('votee', 'en_vigueur')
        OR (vote_le IS NOT NULL AND vote_reference IS NOT NULL AND vote_par IS NOT NULL)),
    CONSTRAINT ent_fin_grille_version_vigueur_chk CHECK (statut <> 'en_vigueur' OR en_vigueur_du IS NOT NULL),
    -- une date de vigueur ne précède jamais le vote (D-069, règle 2)
    CONSTRAINT ent_fin_grille_version_vigueur_vote_chk CHECK (
        en_vigueur_du IS NULL OR vote_le IS NULL OR en_vigueur_du >= vote_le),
    CONSTRAINT ent_fin_grille_version_correction_chk CHECK (remplace_id IS NULL OR motif IS NOT NULL)
);
-- Une seule version en vigueur par année et par établissement (FAC-127).
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_grille_version_vigueur_uq
    ON ent_fin_grille_version (etablissement_id, annee_scolaire) WHERE statut = 'en_vigueur';

-- Machine à états et gel (M3, règle 1 : ce qui est voté est figé **en base**). Une version naît brouillon et
-- vierge ; seules les colonnes posées par la transition elle-même changent, et seulement si elles étaient
-- vides ; hors transition, un brouillon garde son libellé, son motif et son effet rétroactif modifiables, une
-- version votée ou en vigueur seulement sa date de publication (posée une fois). Jamais de suppression ni de
-- vidage. Le second regard est vérifié ici aussi (règle 3).
CREATE OR REPLACE FUNCTION ent_trg_fin_grille_version() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    ok BOOLEAN;
    modifiees TEXT[];
    permises TEXT[];
    second BOOLEAN;
BEGIN
    IF TG_OP IN ('DELETE', 'TRUNCATE') THEN
        RAISE EXCEPTION 'ent_fin_grille_version : ni suppression ni vidage (R-F10)'
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.statut <> 'brouillon'
           OR NEW.propose_le IS NOT NULL OR NEW.propose_par IS NOT NULL
           OR NEW.vote_le IS NOT NULL OR NEW.vote_reference IS NOT NULL OR NEW.vote_document_id IS NOT NULL
           OR NEW.vote_par IS NOT NULL OR NEW.en_vigueur_du IS NOT NULL OR NEW.en_vigueur_par IS NOT NULL
           OR NEW.close_le IS NOT NULL OR NEW.publiee_le IS NOT NULL THEN
            RAISE EXCEPTION 'grille : une version naît brouillon, sans proposition, vote ni vigueur (R-F10)'
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    -- UPDATE : colonnes réellement modifiées
    SELECT COALESCE(array_agg(n.key), ARRAY[]::TEXT[]) INTO modifiees
      FROM jsonb_each(to_jsonb(NEW)) n
     WHERE to_jsonb(OLD) -> n.key IS DISTINCT FROM n.value;
    IF modifiees && ARRAY['id', 'etablissement_id', 'annee_scolaire', 'numero', 'remplace_id', 'cree_par', 'created_at'] THEN
        RAISE EXCEPTION 'ent_fin_grille_version : identité figée (établissement, année, numéro, version remplacée, auteur)'
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF NEW.statut IS DISTINCT FROM OLD.statut THEN
        ok := (OLD.statut, NEW.statut) IN (
            ('brouillon', 'proposee'), ('proposee', 'brouillon'), ('brouillon', 'abandonnee'),
            ('proposee', 'abandonnee'), ('proposee', 'votee'), ('votee', 'en_vigueur'),
            ('votee', 'close'), ('votee', 'supplantee'), ('en_vigueur', 'close'));
        IF NOT ok THEN
            RAISE EXCEPTION 'grille : transition % → % interdite', OLD.statut, NEW.statut
                USING ERRCODE = 'restrict_violation';
        END IF;
        permises := CASE
            WHEN NEW.statut = 'proposee' THEN ARRAY['statut', 'propose_le', 'propose_par']
            WHEN NEW.statut = 'brouillon' THEN ARRAY['statut', 'propose_le', 'propose_par']
            WHEN NEW.statut = 'abandonnee' THEN ARRAY['statut']
            WHEN NEW.statut = 'votee' THEN
                ARRAY['statut', 'vote_le', 'vote_reference', 'vote_document_id', 'vote_par']
            WHEN NEW.statut = 'en_vigueur' THEN ARRAY['statut', 'en_vigueur_du', 'en_vigueur_par']
            ELSE ARRAY['statut', 'close_le'] END;
        IF NOT (modifiees <@ permises) THEN
            RAISE EXCEPTION 'grille : la transition % → % ne change que % (R-F10)', OLD.statut, NEW.statut, permises
                USING ERRCODE = 'restrict_violation';
        END IF;
        -- ce qui est posé ne se repose pas : colonnes de la transition vides avant (sauf remise à zéro du retour)
        IF NEW.statut <> 'brouillon' THEN
            IF (NEW.statut = 'proposee' AND OLD.propose_le IS NOT NULL)
               OR (NEW.statut = 'votee' AND (OLD.vote_le IS NOT NULL OR OLD.vote_par IS NOT NULL))
               OR (NEW.statut = 'en_vigueur' AND (OLD.en_vigueur_du IS NOT NULL OR OLD.en_vigueur_par IS NOT NULL))
               OR (NEW.statut IN ('close', 'supplantee') AND OLD.close_le IS NOT NULL) THEN
                RAISE EXCEPTION 'grille : valeur déjà posée, figée (R-F10)' USING ERRCODE = 'restrict_violation';
            END IF;
        END IF;
        IF NEW.statut = 'proposee' AND (NEW.propose_le IS NULL OR NEW.propose_par IS NULL) THEN
            RAISE EXCEPTION 'grille : la proposition porte sa date et son auteur' USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.statut = 'brouillon' AND (NEW.propose_le IS NOT NULL OR NEW.propose_par IS NOT NULL) THEN
            RAISE EXCEPTION 'grille : le retour en brouillon efface la proposition' USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.statut IN ('close', 'supplantee') AND NEW.close_le IS NULL THEN
            RAISE EXCEPTION 'grille : la clôture porte sa date' USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.statut = 'votee' THEN
            IF NEW.vote_le IS NULL OR NEW.vote_reference IS NULL OR NEW.vote_par IS NULL THEN
                RAISE EXCEPTION 'grille : le vote porte sa date, sa référence et son auteur'
                    USING ERRCODE = 'check_violation';
            END IF;
            -- second regard (règle 3) : le votant n'est ni le créateur, ni le proposant, ni l'auteur d'un tarif
            SELECT COALESCE((SELECT p.grille_second_regard FROM ent_fin_parametres p
                              WHERE p.etablissement_id = NEW.etablissement_id), true) INTO second;
            IF second AND (NEW.vote_par IS NOT DISTINCT FROM NEW.cree_par
                           OR NEW.vote_par IS NOT DISTINCT FROM NEW.propose_par
                           OR EXISTS (SELECT 1 FROM ent_fin_tarif t
                                       WHERE t.etablissement_id = NEW.etablissement_id
                                         AND t.grille_version_id = NEW.id
                                         AND (t.created_by = NEW.vote_par OR t.modifie_par = NEW.vote_par))) THEN
                RAISE EXCEPTION 'second regard : le votant est l''auteur de la grille (créateur, proposant ou auteur de tarif)'
                    USING ERRCODE = 'restrict_violation';
            END IF;
            -- pièce de la délibération : dans l'établissement (la table des documents est créée après la 523,
            -- une clé étrangère est impossible ici : contrôle à l'écriture)
            IF NEW.vote_document_id IS NOT NULL AND NOT EXISTS (
                SELECT 1 FROM ent_doc_document d
                 WHERE d.etablissement_id = NEW.etablissement_id AND d.id = NEW.vote_document_id) THEN
                RAISE EXCEPTION 'grille : pièce de la délibération inconnue dans l''établissement'
                    USING ERRCODE = 'foreign_key_violation';
            END IF;
        END IF;
        IF NEW.statut = 'en_vigueur' THEN
            IF NEW.en_vigueur_du IS NULL OR NEW.en_vigueur_par IS NULL THEN
                RAISE EXCEPTION 'grille : la mise en vigueur porte sa date et son auteur'
                    USING ERRCODE = 'check_violation';
            END IF;
            -- une grille ne tarifie qu'à partir de sa date : elle n'en précède aucune déjà en vigueur
            IF EXISTS (SELECT 1 FROM ent_fin_grille_version g
                        WHERE g.etablissement_id = NEW.etablissement_id AND g.annee_scolaire = NEW.annee_scolaire
                          AND g.id <> NEW.id AND g.statut IN ('en_vigueur', 'close')
                          AND g.en_vigueur_du > NEW.en_vigueur_du) THEN
                RAISE EXCEPTION 'grille : entrée en vigueur antérieure à celle d''une version déjà appliquée'
                    USING ERRCODE = 'check_violation';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    -- hors transition
    IF OLD.statut = 'brouillon' THEN
        IF NOT (modifiees <@ ARRAY['libelle', 'motif', 'effet_retroactif']) THEN
            RAISE EXCEPTION 'grille : un brouillon ne change que son libellé, son motif et son effet rétroactif'
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    -- hors brouillon : seule la publication (posée une fois, version votée ou en vigueur)
    IF NOT (modifiees <@ ARRAY['publiee_le']) THEN
        RAISE EXCEPTION 'grille % : une version proposée, votée ou en vigueur ne se modifie plus (R-F10)', OLD.statut
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF OLD.publiee_le IS NOT NULL THEN
        RAISE EXCEPTION 'grille : date de publication déjà posée' USING ERRCODE = 'restrict_violation';
    END IF;
    IF OLD.statut NOT IN ('votee', 'en_vigueur') THEN
        RAISE EXCEPTION 'grille : seule une grille votée ou en vigueur se publie' USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_fin_grille_version_garde ON ent_fin_grille_version;
CREATE TRIGGER ent_fin_grille_version_garde BEFORE INSERT OR UPDATE ON ent_fin_grille_version
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_grille_version();
DROP TRIGGER IF EXISTS ent_fin_grille_version_sans_suppression ON ent_fin_grille_version;
CREATE TRIGGER ent_fin_grille_version_sans_suppression BEFORE DELETE ON ent_fin_grille_version
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_grille_version();
DROP TRIGGER IF EXISTS ent_fin_grille_version_sans_vidage ON ent_fin_grille_version;
CREATE TRIGGER ent_fin_grille_version_sans_vidage BEFORE TRUNCATE ON ent_fin_grille_version
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_fin_grille_version();

DROP TRIGGER IF EXISTS ent_fin_parametres_second_regard ON ent_fin_parametres;
CREATE TRIGGER ent_fin_parametres_second_regard BEFORE INSERT OR UPDATE ON ent_fin_parametres
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_parametres_second_regard();

-- ── Tarifs rattachés à une version (FD-19) ─────────────────────────────────────────────────────────
ALTER TABLE ent_fin_tarif
    ADD COLUMN IF NOT EXISTS grille_version_id UUID,
    ADD COLUMN IF NOT EXISTS tarif_par_defaut BOOLEAN NOT NULL DEFAULT false,
    -- dernier auteur d'une modification du montant (second regard : le votant n'a rien écrit, règle 3)
    ADD COLUMN IF NOT EXISTS modifie_par UUID REFERENCES ent_user (id);
ALTER TABLE ent_fin_tarif DROP CONSTRAINT IF EXISTS ent_fin_tarif_version_fk;
ALTER TABLE ent_fin_tarif ADD CONSTRAINT ent_fin_tarif_version_fk
    FOREIGN KEY (etablissement_id, grille_version_id) REFERENCES ent_fin_grille_version (etablissement_id, id);
CREATE INDEX IF NOT EXISTS ent_fin_tarif_version_idx
    ON ent_fin_tarif (etablissement_id, grille_version_id) WHERE grille_version_id IS NOT NULL;
-- Un tarif par défaut par article et par version (FAC-119).
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_tarif_defaut_uq
    ON ent_fin_tarif (etablissement_id, grille_version_id, article_id)
    WHERE tarif_par_defaut AND grille_version_id IS NOT NULL;

CREATE OR REPLACE FUNCTION ent_trg_fin_tarif_version() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v ent_fin_grille_version%ROWTYPE;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.grille_version_id IS NOT NULL THEN
            -- FOR SHARE : une transition concurrente de la version attend la fin de cette écriture (F3)
            SELECT * INTO v FROM ent_fin_grille_version
             WHERE etablissement_id = NEW.etablissement_id AND id = NEW.grille_version_id FOR SHARE;
            IF v.statut IS DISTINCT FROM 'brouillon' THEN
                RAISE EXCEPTION 'tarif : la version de grille n''est plus un brouillon (R-F10)'
                    USING ERRCODE = 'restrict_violation';
            END IF;
            IF v.annee_scolaire <> NEW.annee_scolaire THEN
                RAISE EXCEPTION 'tarif : année différente de celle de la version' USING ERRCODE = 'check_violation';
            END IF;
        ELSIF EXISTS (SELECT 1 FROM ent_fin_grille_version g
                       WHERE g.etablissement_id = NEW.etablissement_id AND g.annee_scolaire = NEW.annee_scolaire
                         AND g.statut IN ('votee', 'en_vigueur')) THEN
            RAISE EXCEPTION 'tarif : l''année a une grille votée, passer par une version de grille (R-F10)'
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF NEW.grille_version_id IS DISTINCT FROM OLD.grille_version_id
           OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id THEN
            RAISE EXCEPTION 'tarif : version de grille et établissement figés' USING ERRCODE = 'restrict_violation';
        END IF;
        IF OLD.grille_version_id IS NULL THEN
            -- tarif historique (avant versions) : seule la désactivation reste possible
            IF (NEW.annee_scolaire, NEW.article_id, NEW.niveau, NEW.regime, NEW.qf_min_centimes,
                NEW.qf_max_centimes, NEW.montant_annuel_centimes, NEW.tarif_par_defaut)
               IS DISTINCT FROM (OLD.annee_scolaire, OLD.article_id, OLD.niveau, OLD.regime, OLD.qf_min_centimes,
                OLD.qf_max_centimes, OLD.montant_annuel_centimes, OLD.tarif_par_defaut) THEN
                RAISE EXCEPTION 'tarif : un tarif ne se modifie pas (désactiver puis créer, FD-19)'
                    USING ERRCODE = 'restrict_violation';
            END IF;
            RETURN NEW;
        END IF;
    END IF;
    -- UPDATE ou DELETE d'un tarif rattaché à une version : brouillon seulement
    SELECT * INTO v FROM ent_fin_grille_version
     WHERE etablissement_id = OLD.etablissement_id AND id = OLD.grille_version_id FOR SHARE;
    IF OLD.grille_version_id IS NOT NULL AND v.statut IS DISTINCT FROM 'brouillon' THEN
        RAISE EXCEPTION 'tarif : la grille % est figée (R-F10)', v.statut USING ERRCODE = 'restrict_violation';
    END IF;
    IF TG_OP = 'DELETE' AND OLD.grille_version_id IS NULL THEN
        RAISE EXCEPTION 'tarif : un tarif historique ne se supprime pas' USING ERRCODE = 'restrict_violation';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_fin_tarif_garde ON ent_fin_tarif;
CREATE TRIGGER ent_fin_tarif_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_fin_tarif
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_tarif_version();
-- F4 : le vidage d'une table de tarifs contournerait le gel ligne à ligne
CREATE OR REPLACE FUNCTION ent_trg_fin_tarif_sans_vidage() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_fin_tarif : pas de vidage (R-F10)' USING ERRCODE = 'restrict_violation';
END
$$;
DROP TRIGGER IF EXISTS ent_fin_tarif_sans_vidage ON ent_fin_tarif;
CREATE TRIGGER ent_fin_tarif_sans_vidage BEFORE TRUNCATE ON ent_fin_tarif
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_fin_tarif_sans_vidage();

-- ── Tranches de quotient de la version (FAC-118) ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_tranche_qf (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    grille_version_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[A-Z0-9_]{1,8}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 60),
    qf_min_centimes BIGINT NOT NULL CHECK (qf_min_centimes >= 0),
    qf_max_centimes BIGINT CHECK (qf_max_centimes IS NULL OR qf_max_centimes > qf_min_centimes),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_tranche_qf_code_uq UNIQUE (grille_version_id, code),
    CONSTRAINT ent_fin_tranche_qf_version_fk
        FOREIGN KEY (etablissement_id, grille_version_id) REFERENCES ent_fin_grille_version (etablissement_id, id),
    -- bornes [min, max[ sans chevauchement (continuité contrôlée à la proposition)
    CONSTRAINT ent_fin_tranche_qf_chevauche EXCLUDE USING gist (
        grille_version_id WITH =,
        int8range(qf_min_centimes, qf_max_centimes, '[)') WITH &&)
);

CREATE OR REPLACE FUNCTION ent_trg_fin_tranche_qf() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    st TEXT;
BEGIN
    IF TG_OP = 'TRUNCATE' THEN
        RAISE EXCEPTION 'ent_fin_tranche_qf : pas de vidage' USING ERRCODE = 'restrict_violation';
    END IF;
    -- M4 : une tranche ne change ni de version ni d'établissement (déplacement de ligne fille interdit)
    IF TG_OP = 'UPDATE' AND (NEW.grille_version_id IS DISTINCT FROM OLD.grille_version_id
                             OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id) THEN
        RAISE EXCEPTION 'tranches : version de grille et établissement figés' USING ERRCODE = 'restrict_violation';
    END IF;
    -- l'ancienne version (UPDATE, DELETE) et la nouvelle (INSERT, UPDATE) doivent être des brouillons
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        SELECT statut INTO st FROM ent_fin_grille_version
         WHERE etablissement_id = OLD.etablissement_id AND id = OLD.grille_version_id FOR SHARE;
        IF st IS DISTINCT FROM 'brouillon' THEN
            RAISE EXCEPTION 'tranches : la grille est figée (R-F10)' USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        SELECT statut INTO st FROM ent_fin_grille_version
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.grille_version_id FOR SHARE;
        IF st IS DISTINCT FROM 'brouillon' THEN
            RAISE EXCEPTION 'tranches : la grille est figée (R-F10)' USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_fin_tranche_qf_garde ON ent_fin_tranche_qf;
CREATE TRIGGER ent_fin_tranche_qf_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_fin_tranche_qf
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_tranche_qf();
DROP TRIGGER IF EXISTS ent_fin_tranche_qf_sans_vidage ON ent_fin_tranche_qf;
CREATE TRIGGER ent_fin_tranche_qf_sans_vidage BEFORE TRUNCATE ON ent_fin_tranche_qf
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_fin_tranche_qf();

-- ── Familles types (fictives) et scénarios de simulation (FAC-121 à FAC-124) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_fin_famille_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    -- code de tranche de quotient (le plus bas du quotient de la tranche est utilisé) ; NULL : tarif plein
    tranche_code TEXT CHECK (tranche_code IS NULL OR tranche_code ~ '^[A-Z0-9_]{1,8}$'),
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_famille_type_uq UNIQUE (etablissement_id, libelle),
    CONSTRAINT ent_fin_famille_type_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE TABLE IF NOT EXISTS ent_fin_famille_type_enfant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    famille_type_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang BETWEEN 1 AND 10),
    niveau TEXT NOT NULL CHECK (length(btrim(niveau)) BETWEEN 1 AND 20),
    regime TEXT NOT NULL CHECK (regime IN ('externe', 'demi_pension', 'interne')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_famille_type_enfant_uq UNIQUE (famille_type_id, rang),
    CONSTRAINT ent_fin_famille_type_enfant_fk
        FOREIGN KEY (etablissement_id, famille_type_id) REFERENCES ent_fin_famille_type (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS ent_fin_simulation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_scolaire TEXT NOT NULL CHECK (annee_scolaire ~ '^[0-9]{4}-[0-9]{4}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    base_version_id UUID NOT NULL,
    -- résultat calculé et figé à chaque calcul : produit attendu avant et après, en centimes
    produit_base_centimes BIGINT,
    produit_simule_centimes BIGINT,
    calcule_le TIMESTAMPTZ,
    cree_par UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_simulation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_fin_simulation_version_fk
        FOREIGN KEY (etablissement_id, base_version_id) REFERENCES ent_fin_grille_version (etablissement_id, id)
);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_fin_article_etab_id_uq') THEN
        ALTER TABLE ent_fin_article ADD CONSTRAINT ent_fin_article_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END
$$;
-- Règle de hausse (article NULL : tous les articles de la grille).
CREATE TABLE IF NOT EXISTS ent_fin_simulation_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    simulation_id UUID NOT NULL,
    article_id UUID,
    hausse_mode TEXT NOT NULL CHECK (hausse_mode IN ('pourcentage', 'montant')),
    -- points de base (pourcentage) ou centimes (montant) ; négatif = baisse ; bornes raisonnables
    hausse_valeur BIGINT NOT NULL CHECK (hausse_valeur BETWEEN -10000000 AND 10000000),
    arrondi TEXT NOT NULL DEFAULT 'aucun' CHECK (arrondi IN ('aucun', 'euro', 'cinq_euros')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_simulation_regle_fk
        FOREIGN KEY (etablissement_id, simulation_id) REFERENCES ent_fin_simulation (etablissement_id, id),
    -- F1 : l'article est un article de **l'établissement** (clé composite)
    CONSTRAINT ent_fin_simulation_regle_article_fk
        FOREIGN KEY (etablissement_id, article_id) REFERENCES ent_fin_article (etablissement_id, id)
);
-- Effectifs prévus (FAC-121) : aucun nom, des nombres par niveau et régime.
CREATE TABLE IF NOT EXISTS ent_fin_simulation_effectif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    simulation_id UUID NOT NULL,
    niveau TEXT NOT NULL CHECK (length(btrim(niveau)) BETWEEN 1 AND 20),
    regime TEXT NOT NULL CHECK (regime IN ('externe', 'demi_pension', 'interne')),
    effectif INTEGER NOT NULL CHECK (effectif BETWEEN 0 AND 100000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_simulation_effectif_uq UNIQUE (simulation_id, niveau, regime),
    CONSTRAINT ent_fin_simulation_effectif_fk
        FOREIGN KEY (etablissement_id, simulation_id) REFERENCES ent_fin_simulation (etablissement_id, id)
);

-- FD-19 : la suppression d'une réduction ne supprime plus en cascade les attributions.
ALTER TABLE ent_fin_reduction_eleve DROP CONSTRAINT IF EXISTS ent_fin_reduction_eleve_reduction_id_fkey;
ALTER TABLE ent_fin_reduction_eleve ADD CONSTRAINT ent_fin_reduction_eleve_reduction_id_fkey
    FOREIGN KEY (reduction_id) REFERENCES ent_fin_reduction (id) ON DELETE RESTRICT;

-- ── Historique des calculs d'une simulation (F4) ────────────────────────────────────────────────────
-- Recalculer une simulation n'écrase plus rien en silence : chaque calcul ajoute une ligne (produits avant et
-- après, date, auteur) ; la simulation porte le dernier résultat. Ajout seul ; la ligne disparaît avec sa
-- simulation (suppression volontaire, F5), jamais autrement.
CREATE TABLE IF NOT EXISTS ent_fin_simulation_calcul (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    simulation_id UUID NOT NULL,
    produit_base_centimes BIGINT NOT NULL,
    produit_simule_centimes BIGINT NOT NULL,
    calcule_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    calcule_par UUID REFERENCES ent_user (id),
    CONSTRAINT ent_fin_simulation_calcul_fk
        FOREIGN KEY (etablissement_id, simulation_id) REFERENCES ent_fin_simulation (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_fin_simulation_calcul_idx
    ON ent_fin_simulation_calcul (etablissement_id, simulation_id, calcule_le);

-- Les familles types, les simulations et leurs lignes se suppriment volontairement (F5, avec plafond côté
-- service) ; le vidage d'une table reste interdit, et un calcul historisé ne se modifie pas.
CREATE OR REPLACE FUNCTION ent_trg_fin_f3_sans_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : pas de vidage ni de modification (historique des scénarios)', TG_TABLE_NAME
        USING ERRCODE = 'restrict_violation';
END
$$;
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_fin_famille_type', 'ent_fin_famille_type_enfant', 'ent_fin_simulation',
                             'ent_fin_simulation_regle', 'ent_fin_simulation_effectif',
                             'ent_fin_simulation_calcul']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_suppression', t);
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_vidage', t);
        EXECUTE format(
            'CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_fin_f3_sans_suppression()',
            t || '_sans_vidage', t);
    END LOOP;
END
$$;
DROP TRIGGER IF EXISTS ent_fin_simulation_calcul_fige ON ent_fin_simulation_calcul;
CREATE TRIGGER ent_fin_simulation_calcul_fige BEFORE UPDATE ON ent_fin_simulation_calcul
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_f3_sans_suppression();

-- ── RLS d'établissement (même politique que les migrations 100, 206, 521 et 522) ───────────────────
DO $$
DECLARE
    t TEXT;
    tables TEXT[] := ARRAY['ent_fin_grille_version', 'ent_fin_tranche_qf', 'ent_fin_famille_type',
                           'ent_fin_famille_type_enfant', 'ent_fin_simulation', 'ent_fin_simulation_regle',
                           'ent_fin_simulation_effectif', 'ent_fin_simulation_calcul'];
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

-- ── Purge volontaire d'établissement : fonction de la 522 étendue aux tables de scénarios (enfants d'abord) ─
-- Les versions de grille, tarifs et articles restent (clés étrangères des pièces et des tarifs, conservés).
CREATE OR REPLACE FUNCTION ent_fin_purger_etablissement(p_etab UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_fin_journal', 'ent_fin_tache', 'ent_fin_consequence', 'ent_fin_designation',
                             'ent_fin_prise_en_charge', 'ent_fin_reprise_rapport',
                             'ent_fin_simulation_effectif', 'ent_fin_simulation_regle', 'ent_fin_simulation_calcul',
                             'ent_fin_simulation',
                             'ent_fin_famille_type_enfant', 'ent_fin_famille_type']
    LOOP
        EXECUTE format('ALTER TABLE %I DISABLE TRIGGER USER', t);
        EXECUTE format('DELETE FROM %I WHERE etablissement_id = $1', t) USING p_etab;
        EXECUTE format('ALTER TABLE %I ENABLE TRIGGER USER', t);
    END LOOP;
END
$$;
REVOKE ALL ON FUNCTION ent_fin_purger_etablissement(UUID) FROM PUBLIC;
