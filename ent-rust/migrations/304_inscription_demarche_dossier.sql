-- Lot I2 (socle 3, §6, §7, §8.3, §8.5, §8.7, §16 M4) — démarches, dossiers, états, références,
-- vœux et décisions.
--
-- Additif et rejouable. Le dossier garde ses colonnes d'origine (`statut`, `eleve_*`,
-- `decision_*`) le temps de migrer les lecteurs (M12) ; **`etat` est le nouvel état de
-- référence** (§7.1 à §7.3). Les deux colonnes sont tenues d'accord par un déclencheur de
-- synchronisation (`statut` n'est plus que la vue « historique » de l'état) :
--   * un écrivain qui change `etat` (le service du lot I2) voit `statut` suivre ;
--   * un écrivain historique qui change `statut` voit `etat` suivre.
-- Les **transitions** ne sont PAS contrôlées ici : elles le sont par le service
-- (`ent-db::inscriptions_etats`), qui est le seul à connaître l'auteur, la campagne et les
-- règles ; un test compare la table du service à `ent_inscription_etat_valide`.
--
-- * Une `démarche` = ce qu'une famille dépose en une fois (fratrie native). Chaque dossier en
--   a une ; un dossier écrit par un code historique en reçoit une automatiquement.
-- * `reference` : numéro lisible donné à la famille (`ADM-2026-0142`), unique par
--   établissement, attribué à l'insertion par un compteur atomique.
-- * `ent_inscription_voeu` et `ent_inscription_decision` : §8.5 et §8.7 (la décision n'est
--   jamais un simple changement de colonne).

-- ───────────────────────────── États valides par nature ─────────────────────────────
CREATE OR REPLACE FUNCTION ent_inscription_etat_valide(p_nature TEXT, p_etat TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE p_nature
        WHEN 'admission' THEN p_etat IN (
            'brouillon', 'deposee', 'en_instruction', 'attente_famille', 'liste_attente', 'admis',
            'refuse', 'confirme', 'validee', 'retire', 'desiste', 'sans_suite', 'bloque_desaccord',
            'annule')
        WHEN 'en_cours_annee' THEN p_etat IN (
            'brouillon', 'deposee', 'en_instruction', 'attente_famille', 'confirme', 'validee',
            'retire', 'desiste', 'sans_suite', 'bloque_desaccord', 'annule')
        WHEN 'reinscription' THEN p_etat IN (
            'a_confirmer', 'confirmee', 'en_attente_passage', 'validee',
            'non_reinscription_famille', 'non_reinscription_etablissement', 'sans_reponse',
            'bloque_desaccord', 'annule')
        WHEN 'affectes' THEN p_etat IN (
            'importe', 'a_completer', 'complete', 'validee', 'non_presente', 'desiste',
            'reaffecte', 'bloque_desaccord', 'annule')
        ELSE false
    END
$$;

-- Vue « historique » d'un état (colonne `statut`, 9 valeurs) : ce que lisent encore les
-- anciens lecteurs. `bloque_desaccord` ne change pas la vue historique.
CREATE OR REPLACE FUNCTION ent_inscription_statut_depuis_etat(p_nature TEXT, p_etat TEXT, p_statut TEXT)
RETURNS TEXT LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE
        WHEN p_etat = 'bloque_desaccord' THEN p_statut
        WHEN p_nature IN ('admission', 'en_cours_annee') THEN CASE p_etat
            WHEN 'attente_famille' THEN 'incomplet'
            WHEN 'confirme' THEN 'admis'
            WHEN 'retire' THEN 'desistement'
            WHEN 'desiste' THEN 'desistement'
            WHEN 'sans_suite' THEN 'desistement'
            WHEN 'annule' THEN 'desistement'
            ELSE p_etat END
        WHEN p_nature = 'reinscription' THEN CASE p_etat
            WHEN 'a_confirmer' THEN 'brouillon'
            WHEN 'sans_reponse' THEN 'brouillon'
            WHEN 'confirmee' THEN 'deposee'
            WHEN 'en_attente_passage' THEN 'deposee'
            WHEN 'non_reinscription_famille' THEN 'desistement'
            WHEN 'non_reinscription_etablissement' THEN 'desistement'
            WHEN 'annule' THEN 'desistement'
            ELSE p_etat END
        WHEN p_nature = 'affectes' THEN CASE p_etat
            WHEN 'importe' THEN 'brouillon'
            WHEN 'a_completer' THEN 'brouillon'
            WHEN 'complete' THEN 'deposee'
            WHEN 'non_presente' THEN 'desistement'
            WHEN 'desiste' THEN 'desistement'
            WHEN 'reaffecte' THEN 'desistement'
            WHEN 'annule' THEN 'desistement'
            ELSE p_etat END
        ELSE p_statut
    END
$$;

-- Sens inverse, pour un écrivain historique. Un statut sans correspondance est refusé.
CREATE OR REPLACE FUNCTION ent_inscription_etat_depuis_statut(p_nature TEXT, p_statut TEXT, p_etat TEXT)
RETURNS TEXT LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    v TEXT;
BEGIN
    v := CASE
        WHEN p_nature IN ('admission', 'en_cours_annee') THEN CASE p_statut
            WHEN 'incomplet' THEN 'attente_famille'
            WHEN 'desistement' THEN 'desiste'
            WHEN 'brouillon' THEN 'brouillon' WHEN 'deposee' THEN 'deposee'
            WHEN 'en_instruction' THEN 'en_instruction' WHEN 'liste_attente' THEN 'liste_attente'
            WHEN 'admis' THEN 'admis' WHEN 'refuse' THEN 'refuse' WHEN 'validee' THEN 'validee'
            ELSE NULL END
        WHEN p_nature = 'reinscription' THEN CASE p_statut
            WHEN 'brouillon' THEN 'a_confirmer'
            WHEN 'deposee' THEN 'confirmee' WHEN 'en_instruction' THEN 'confirmee'
            WHEN 'incomplet' THEN 'confirmee'
            WHEN 'validee' THEN 'validee'
            WHEN 'desistement' THEN 'non_reinscription_famille'
            ELSE NULL END
        WHEN p_nature = 'affectes' THEN CASE p_statut
            WHEN 'brouillon' THEN 'a_completer'
            WHEN 'deposee' THEN 'complete' WHEN 'en_instruction' THEN 'complete'
            WHEN 'validee' THEN 'validee'
            WHEN 'desistement' THEN 'desiste'
            ELSE NULL END
        ELSE NULL END;
    IF v IS NULL THEN
        RAISE EXCEPTION 'statut % incompatible avec la nature % du dossier', p_statut, p_nature
            USING ERRCODE = '23514';
    END IF;
    RETURN v;
END
$$;

-- ───────────────────────────── Références de dossier ─────────────────────────────
CREATE TABLE IF NOT EXISTS ent_inscription_reference_compteur (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    prefixe TEXT NOT NULL CHECK (prefixe ~ '^[A-Z]{3}$'),
    annee INTEGER NOT NULL CHECK (annee BETWEEN 2000 AND 2999),
    dernier INTEGER NOT NULL CHECK (dernier >= 1),
    PRIMARY KEY (etablissement_id, prefixe, annee)
);

-- Prochaine référence (`ADM-2026-0142`) : compteur atomique par (établissement, préfixe, année).
-- L'incrément est verrouillé par la ligne du compteur : deux insertions concurrentes ne
-- reçoivent jamais le même numéro ; un abandon de transaction rend le numéro.
CREATE OR REPLACE FUNCTION ent_inscription_nouvelle_reference(p_etab UUID, p_nature TEXT, p_campagne UUID)
RETURNS TEXT LANGUAGE plpgsql AS $$
DECLARE
    v_annee TEXT; v_an INTEGER; v_prefixe TEXT; v_n INTEGER;
BEGIN
    SELECT annee_cible INTO v_annee FROM ent_inscription_campagne
     WHERE id = p_campagne AND etablissement_id = p_etab;
    v_an := COALESCE(NULLIF(substr(COALESCE(v_annee, ''), 1, 4), '')::INTEGER, extract(year FROM now())::INTEGER);
    v_prefixe := CASE p_nature
        WHEN 'admission' THEN 'ADM' WHEN 'affectes' THEN 'AFF'
        WHEN 'reinscription' THEN 'REI' ELSE 'ARR' END;
    INSERT INTO ent_inscription_reference_compteur AS c (etablissement_id, prefixe, annee, dernier)
    VALUES (p_etab, v_prefixe, v_an, 1)
    ON CONFLICT (etablissement_id, prefixe, annee) DO UPDATE SET dernier = c.dernier + 1
    RETURNING dernier INTO v_n;
    RETURN v_prefixe || '-' || v_an || '-' || lpad(v_n::TEXT, 4, '0');
END
$$;

-- ───────────────────────────── Démarche familiale ─────────────────────────────
CREATE TABLE IF NOT EXISTS ent_inscription_demarche (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- E-mail VÉRIFIÉ du déposant (D-014) ; absent pour une réinscription préparée ou un guichet.
    deposant_email TEXT CHECK (deposant_email IS NULL OR char_length(deposant_email) <= 254),
    deposant_user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    deposant_role TEXT NOT NULL DEFAULT 'responsable'
        CHECK (deposant_role IN ('responsable', 'eleve_majeur', 'etablissement')),
    reutilise_identite BOOLEAN NOT NULL DEFAULT false,
    canal TEXT NOT NULL DEFAULT 'en_ligne'
        CHECK (canal IN ('en_ligne', 'guichet', 'import', 'reinscription')),
    contact_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_demarche_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_inscription_demarche_deposant_idx
    ON ent_inscription_demarche (etablissement_id, deposant_email);

-- ───────────────────────────── Dossier : colonnes du modèle ─────────────────────────────
ALTER TABLE ent_inscription_dossier
    ADD COLUMN IF NOT EXISTS demarche_id UUID,
    ADD COLUMN IF NOT EXISTS etat TEXT,
    ADD COLUMN IF NOT EXISTS etat_avant_blocage TEXT,
    ADD COLUMN IF NOT EXISTS etape_code TEXT,
    ADD COLUMN IF NOT EXISTS reference TEXT,
    ADD COLUMN IF NOT EXISTS echeance_famille DATE,
    ADD COLUMN IF NOT EXISTS assigne_a UUID,
    ADD COLUMN IF NOT EXISTS rang_attente INTEGER,
    ADD COLUMN IF NOT EXISTS priorite_codes TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS voeu_retenu_id UUID,
    ADD COLUMN IF NOT EXISTS scolarite_actuelle JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS besoins_annonces TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS clos_le TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS motif_cloture TEXT,
    ADD COLUMN IF NOT EXISTS cand_nom_usage TEXT,
    ADD COLUMN IF NOT EXISTS cand_prenoms TEXT[],
    ADD COLUMN IF NOT EXISTS cand_naissance_commune TEXT,
    ADD COLUMN IF NOT EXISTS cand_naissance_pays TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_etab_id_uq') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
    ALTER TABLE ent_inscription_dossier DROP CONSTRAINT IF EXISTS ent_inscription_dossier_nature_check;
    ALTER TABLE ent_inscription_dossier DROP CONSTRAINT IF EXISTS ent_inscription_dossier_nature_i2_chk;
    ALTER TABLE ent_inscription_dossier
        ADD CONSTRAINT ent_inscription_dossier_nature_i2_chk
        CHECK (nature IN ('admission', 'affectes', 'reinscription', 'en_cours_annee'));
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_etat_chk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_etat_chk
            CHECK (etat IS NULL OR ent_inscription_etat_valide(nature, etat));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_champs_i2_chk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_champs_i2_chk
            CHECK ((etape_code IS NULL OR char_length(etape_code) <= 40)
               AND (motif_cloture IS NULL OR char_length(motif_cloture) <= 1000)
               AND (rang_attente IS NULL OR rang_attente >= 1)
               AND (etat <> 'validee' OR eleve_id IS NOT NULL)) NOT VALID;
    END IF;
    -- Clés composées « établissement » vers les parents : contrôlées pour toute nouvelle ligne
    -- (les lignes antérieures ne sont pas réécrites ; la validation complète relève du lot M9).
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_campagne_etab_fk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_campagne_etab_fk
            FOREIGN KEY (etablissement_id, campagne_id)
            REFERENCES ent_inscription_campagne (etablissement_id, id) NOT VALID;
    END IF;
END $$;

-- ───────────────────────────── Déclencheur : synchronisation, démarche, référence ─────────────────────────────
CREATE OR REPLACE FUNCTION ent_inscription_dossier_avant_ecriture() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.etat IS NULL THEN
            NEW.etat := ent_inscription_etat_depuis_statut(NEW.nature, NEW.statut, NULL);
        ELSE
            NEW.statut := ent_inscription_statut_depuis_etat(NEW.nature, NEW.etat, NEW.statut);
        END IF;
        IF NEW.demarche_id IS NULL THEN
            NEW.demarche_id := gen_random_uuid();
            INSERT INTO ent_inscription_demarche
                (id, etablissement_id, deposant_email, deposant_user_id, deposant_role,
                 reutilise_identite, canal)
            VALUES (NEW.demarche_id, NEW.etablissement_id, NEW.deposant_email, NEW.deposant_user_id,
                    'responsable', COALESCE(NEW.reutilise_identite, false),
                    CASE NEW.nature WHEN 'reinscription' THEN 'reinscription'
                                    WHEN 'affectes' THEN 'import' ELSE 'en_ligne' END);
        END IF;
        IF NEW.reference IS NULL THEN
            NEW.reference := ent_inscription_nouvelle_reference(NEW.etablissement_id, NEW.nature, NEW.campagne_id);
        END IF;
        RETURN NEW;
    END IF;
    -- UPDATE : l'écrivain qui change `etat` est prioritaire ; sinon `statut` fait suivre `etat`.
    IF NEW.etat IS DISTINCT FROM OLD.etat THEN
        IF OLD.etat IS NOT NULL THEN
            NEW.statut := ent_inscription_statut_depuis_etat(NEW.nature, NEW.etat, OLD.statut);
        END IF;
    ELSIF NEW.statut IS DISTINCT FROM OLD.statut AND OLD.etat IS NOT NULL THEN
        NEW.etat := ent_inscription_etat_depuis_statut(NEW.nature, NEW.statut, OLD.etat);
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_inscription_dossier_avant_ecriture_trg ON ent_inscription_dossier;
CREATE TRIGGER ent_inscription_dossier_avant_ecriture_trg
    BEFORE INSERT OR UPDATE ON ent_inscription_dossier
    FOR EACH ROW EXECUTE FUNCTION ent_inscription_dossier_avant_ecriture();

-- ───────────────────────────── Reprise de l'existant (idempotente) ─────────────────────────────
-- 1. Une démarche par dossier existant.
WITH d AS (
    SELECT id, etablissement_id, deposant_email, deposant_user_id, reutilise_identite, nature,
           gen_random_uuid() AS demarche_id
      FROM ent_inscription_dossier WHERE demarche_id IS NULL
), ins AS (
    INSERT INTO ent_inscription_demarche
        (id, etablissement_id, deposant_email, deposant_user_id, deposant_role,
         reutilise_identite, canal)
    SELECT demarche_id, etablissement_id, deposant_email, deposant_user_id, 'responsable',
           COALESCE(reutilise_identite, false),
           CASE nature WHEN 'reinscription' THEN 'reinscription' ELSE 'en_ligne' END
      FROM d
    RETURNING id
)
UPDATE ent_inscription_dossier x SET demarche_id = d.demarche_id
  FROM d WHERE x.id = d.id;

-- 2. L'état depuis le statut historique (sans toucher au statut).
UPDATE ent_inscription_dossier
   SET etat = ent_inscription_etat_depuis_statut(nature, statut, NULL)
 WHERE etat IS NULL;

-- 3. Références, dans l'ordre de création.
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT id, etablissement_id, nature, campagne_id
               FROM ent_inscription_dossier WHERE reference IS NULL
              ORDER BY created_at, id
    LOOP
        UPDATE ent_inscription_dossier
           SET reference = ent_inscription_nouvelle_reference(r.etablissement_id, r.nature, r.campagne_id)
         WHERE id = r.id;
    END LOOP;
END $$;

ALTER TABLE ent_inscription_dossier ALTER COLUMN etat SET NOT NULL;
ALTER TABLE ent_inscription_dossier ALTER COLUMN demarche_id SET NOT NULL;
ALTER TABLE ent_inscription_dossier ALTER COLUMN reference SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_demarche_fk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_demarche_fk
            FOREIGN KEY (etablissement_id, demarche_id)
            REFERENCES ent_inscription_demarche (etablissement_id, id);
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_dossier_reference_uq
    ON ent_inscription_dossier (etablissement_id, reference);
CREATE INDEX IF NOT EXISTS ent_inscription_dossier_demarche_idx
    ON ent_inscription_dossier (etablissement_id, demarche_id);
CREATE INDEX IF NOT EXISTS ent_inscription_dossier_etat_idx
    ON ent_inscription_dossier (etablissement_id, campagne_id, etat);
-- Un élève ne figure qu'une fois par campagne, sauf dossier annulé (§8.3, « resserré »).
DROP INDEX IF EXISTS ent_inscription_dossier_eleve_uq;
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_dossier_eleve_actif_uq
    ON ent_inscription_dossier (campagne_id, eleve_id)
    WHERE eleve_id IS NOT NULL AND etat <> 'annule';

-- ───────────────────────────── Vœux (§8.5) ─────────────────────────────
CREATE TABLE IF NOT EXISTS ent_inscription_voeu (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang BETWEEN 1 AND 5),
    offre_id UUID NOT NULL,
    unite_id UUID,
    statut TEXT NOT NULL DEFAULT 'demande'
        CHECK (statut IN ('demande', 'retenu', 'non_retenu', 'retire')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_voeu_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_voeu_rang_uq UNIQUE (dossier_id, rang),
    CONSTRAINT ent_inscription_voeu_offre_uq UNIQUE (dossier_id, offre_id),
    CONSTRAINT ent_inscription_voeu_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_inscription_voeu_offre_fk
        FOREIGN KEY (etablissement_id, offre_id)
        REFERENCES ent_inscription_offre (etablissement_id, id),
    CONSTRAINT ent_inscription_voeu_unite_fk
        FOREIGN KEY (etablissement_id, unite_id)
        REFERENCES ent_inscription_offre_unite (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_inscription_voeu_offre_idx
    ON ent_inscription_voeu (etablissement_id, offre_id, statut);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_dossier_voeu_retenu_fk') THEN
        ALTER TABLE ent_inscription_dossier
            ADD CONSTRAINT ent_inscription_dossier_voeu_retenu_fk
            FOREIGN KEY (etablissement_id, voeu_retenu_id)
            REFERENCES ent_inscription_voeu (etablissement_id, id);
    END IF;
END $$;

-- ───────────────────────────── Décisions (§8.7) ─────────────────────────────
-- Une décision est une ligne : auteur, date, motif interne (obligatoire), motif communiqué
-- (facultatif selon la campagne). Une décision revue ne remplace pas l'ancienne : une nouvelle
-- ligne la référence (`remplace_decision_id`) ; rien n'est effacé.
CREATE TABLE IF NOT EXISTS ent_inscription_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    sens TEXT NOT NULL
        CHECK (sens IN ('admis', 'admis_sous_reserve', 'liste_attente', 'refuse', 'ajourne')),
    offre_id UUID,
    reserve TEXT CHECK (reserve IS NULL OR char_length(reserve) <= 500),
    motif_interne TEXT NOT NULL CHECK (char_length(motif_interne) BETWEEN 3 AND 1000),
    motif_communique TEXT CHECK (motif_communique IS NULL OR char_length(motif_communique) <= 1000),
    depassement_capacite BOOLEAN NOT NULL DEFAULT false,
    remplace_decision_id UUID REFERENCES ent_inscription_decision (id),
    decide_par UUID,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    courrier_document_id UUID,
    commission_id UUID,
    CONSTRAINT ent_inscription_decision_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_decision_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_inscription_decision_offre_fk
        FOREIGN KEY (etablissement_id, offre_id)
        REFERENCES ent_inscription_offre (etablissement_id, id),
    CONSTRAINT ent_inscription_decision_reserve_chk
        CHECK (sens <> 'admis_sous_reserve' OR reserve IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_inscription_decision_dossier_idx
    ON ent_inscription_decision (etablissement_id, dossier_id, decide_le);

-- Décisions déjà prises : reprise depuis `decision_*` (une ligne par dossier décidé).
INSERT INTO ent_inscription_decision
    (id, etablissement_id, dossier_id, sens, motif_interne, decide_par, decide_le)
SELECT gen_random_uuid(), d.etablissement_id, d.id,
       CASE WHEN d.statut IN ('admis', 'validee') THEN 'admis' ELSE d.statut END,
       CASE WHEN char_length(btrim(COALESCE(d.decision_motif, ''))) >= 3 THEN btrim(d.decision_motif)
            ELSE 'Motif non renseigné (reprise des décisions antérieures)' END,
       d.decide_par, COALESCE(d.decide_le, d.updated_at)
  FROM ent_inscription_dossier d
 WHERE d.nature = 'admission'
   AND d.statut IN ('admis', 'refuse', 'liste_attente', 'validee')
   AND (d.decide_le IS NOT NULL OR d.decision_motif IS NOT NULL OR d.statut = 'validee')
   AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision x WHERE x.dossier_id = d.id);
