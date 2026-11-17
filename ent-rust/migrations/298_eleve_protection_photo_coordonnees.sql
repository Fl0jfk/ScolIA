-- Socle 2 (l'élève), lot E7 : confidentialité renforcée (ELV-15), photos (ELV-16, ELV-42),
-- coordonnées propres de l'élève (ELV-17), scellement des versions d'identité sensibles.
--
-- Additif, idempotent, rejoué à chaque démarrage. Numéro 298 (plage 290-299 du socle 2).
-- Aucune migration existante n'est modifiée : les tables et fonctions des lots précédents sont
-- complétées ici (colonnes, gardes remplacées par `CREATE OR REPLACE`).
--
-- Principes :
--   * rien de sensible en clair : le motif d'une mesure de confidentialité et les anciennes
--     valeurs d'une version d'identité sensible sont scellés (`champs_chiffres`, AES-256-GCM) ;
--   * rien ne s'efface : une mesure se lève, une photo se retire, une coordonnée se retire ;
--     chaque table garde ses lignes (gardes ci-dessous) ;
--   * pas de JSON métier (D-036) : colonnes typées seulement ;
--   * RLS d'établissement forcée sur toutes les tables nouvelles (mode permissif / strict de
--     `ent_rls_strict()`), comme les lots précédents ;
--   * la consultation sensible n'a pas de table propre : elle s'écrit dans `ent_journal_modif`
--     (ajout seul, migration 091), domaine `eleve_consultation`.

-- ───────────────────────── Confidentialité renforcée (ELV-15) ─────────────────────────
-- La table existe depuis 290 (E6). E7 ajoute le motif codé de la levée et les gardes.
ALTER TABLE ent_eleve_confidentialite ADD COLUMN IF NOT EXISTS leve_motif_code TEXT;

DO $$
BEGIN
    -- Posées NOT VALID : elles protègent toute écriture à venir sans jamais faire échouer la
    -- reprise d'une ligne imprévue.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_confidentialite_regles_chk') THEN
        ALTER TABLE ent_eleve_confidentialite ADD CONSTRAINT ent_eleve_confidentialite_regles_chk CHECK (
            mesures <@ ARRAY['masquer_photo', 'hors_annuaire', 'hors_trombinoscope_exporte',
                             'adresse_protegee', 'alerte_consultation']::TEXT[]
            AND motif_scelle LIKE 'enc:v%'
            AND (leve_motif_code IS NULL OR leve_motif_code IN
                 ('fin_de_mesure', 'decision_levee', 'erreur_de_saisie', 'changement_de_mesure', 'autre'))
            AND (leve_le IS NULL OR leve_par IS NOT NULL OR decide_par IS NULL)
        ) NOT VALID;
    END IF;
END $$;

-- Une mesure ne se modifie pas (la lever et en poser une autre) et ne se supprime pas : seule sa
-- levée s'écrit, ou le repointage d'une fusion de fiches (drapeau `ent.fusion_eleve`, 293).
CREATE OR REPLACE FUNCTION ent_eleve_confidentialite_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'confidentialité : une mesure ne se supprime pas (la lever)' USING ERRCODE = '42501';
    END IF;
    IF ent_fusion_en_cours() AND (to_jsonb(NEW) - 'eleve_id') = (to_jsonb(OLD) - 'eleve_id') THEN
        RETURN NEW;
    END IF;
    IF OLD.leve_le IS NULL AND NEW.leve_le IS NOT NULL
       AND (to_jsonb(NEW) - 'leve_le' - 'leve_par' - 'au' - 'leve_motif_code')
         = (to_jsonb(OLD) - 'leve_le' - 'leve_par' - 'au' - 'leve_motif_code') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'confidentialité : une mesure est immuable (la lever, puis en poser une autre)'
        USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_confidentialite_garde ON ent_eleve_confidentialite;
CREATE TRIGGER ent_eleve_confidentialite_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_confidentialite
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_confidentialite_garde();

-- ───────────────────────── Photos (ELV-16, ELV-42) ─────────────────────────
-- Une photo = un objet immuable `{etab}/trombi/{eleve}/{uuid}.{ext}` (stockage objet privé, jamais
-- servie sans lien court émis après contrôle des droits). États : proposée → validée | rejetée ;
-- validée → retirée. Au plus une photo validée par élève ; `ent_eleve.photo_key` / `photo_id`
-- en sont la projection (maintenue par le déclencheur ci-dessous, les lecteurs hérités ne
-- changent pas).
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS photo_id UUID;

CREATE TABLE IF NOT EXISTS ent_eleve_photo (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id          UUID NOT NULL,
    object_key        TEXT NOT NULL,
    mime              TEXT NOT NULL,
    taille_octets     INTEGER NOT NULL CHECK (taille_octets > 0),
    prise_le          DATE,
    source            TEXT NOT NULL CHECK (source IN ('etablissement', 'photographe', 'famille', 'eleve', 'import')),
    statut            TEXT NOT NULL DEFAULT 'proposee' CHECK (statut IN ('proposee', 'validee', 'retiree', 'rejetee')),
    propose_par       UUID,
    propose_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    decide_par        UUID,
    decide_le         TIMESTAMPTZ,
    motif_code        TEXT CHECK (motif_code IS NULL OR motif_code IN
                          ('non_conforme', 'illisible', 'autre_personne', 'refus_famille',
                           'remplacee', 'erreur_de_saisie', 'demande_famille', 'autre')),
    CONSTRAINT ent_eleve_photo_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    -- La clé d'objet appartient à CET élève de CET établissement (aucune clé étrangère possible).
    CONSTRAINT ent_eleve_photo_cle_chk CHECK (
        object_key LIKE etablissement_id::text || '/trombi/' || eleve_id::text || '/%'
        AND object_key NOT LIKE '%..%'),
    CONSTRAINT ent_eleve_photo_decision_chk CHECK (
        (statut = 'proposee' AND decide_le IS NULL)
        OR (statut IN ('validee', 'retiree', 'rejetee') AND decide_le IS NOT NULL)),
    CONSTRAINT ent_eleve_photo_motif_chk CHECK (statut NOT IN ('retiree', 'rejetee') OR motif_code IS NOT NULL)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_photo_validee_uniq
    ON ent_eleve_photo (etablissement_id, eleve_id) WHERE statut = 'validee';
CREATE INDEX IF NOT EXISTS ent_eleve_photo_eleve_idx
    ON ent_eleve_photo (etablissement_id, eleve_id, propose_le DESC);

-- Les seules transitions permises : proposée → validée | rejetée, validée → retirée ; l'objet,
-- l'élève, la source et la date de proposition ne changent jamais ; aucune suppression.
CREATE OR REPLACE FUNCTION ent_eleve_photo_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'photo : une ligne ne se supprime pas (la retirer)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.statut = 'proposee' AND NEW.statut IN ('validee', 'rejetee'))
       OR (OLD.statut = 'validee' AND NEW.statut = 'retiree') THEN
        IF (to_jsonb(NEW) - 'statut' - 'decide_par' - 'decide_le' - 'motif_code')
         = (to_jsonb(OLD) - 'statut' - 'decide_par' - 'decide_le' - 'motif_code') THEN
            RETURN NEW;
        END IF;
    END IF;
    RAISE EXCEPTION 'photo : transition ou modification refusée' USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_photo_garde ON ent_eleve_photo;
CREATE TRIGGER ent_eleve_photo_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_photo
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_photo_garde();

-- Projection sur la fiche : la photo validée est la photo courante ; sa levée retire la photo
-- courante (`NULL`). Une photo proposée ou rejetée ne touche pas la fiche.
CREATE OR REPLACE FUNCTION ent_eleve_photo_projeter() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.statut = 'validee' THEN
        UPDATE ent_eleve SET photo_key = NEW.object_key, photo_id = NEW.id, updated_at = now()
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.eleve_id;
    ELSIF TG_OP = 'UPDATE' AND OLD.statut = 'validee' AND NEW.statut = 'retiree' THEN
        UPDATE ent_eleve SET photo_key = NULL, photo_id = NULL, updated_at = now()
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.eleve_id AND photo_id = NEW.id;
    END IF;
    RETURN NULL;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_photo_projection ON ent_eleve_photo;
CREATE TRIGGER ent_eleve_photo_projection
    AFTER INSERT OR UPDATE ON ent_eleve_photo
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_photo_projeter();

-- ELV-42 : refus de la photo administrative (silhouette dans les listes, l'élève sort de
-- « photos manquantes », aucune relance). Révocable : une ligne, levée puis éventuellement reposée.
CREATE TABLE IF NOT EXISTS ent_eleve_photo_refus (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id          UUID NOT NULL,
    source            TEXT NOT NULL CHECK (source IN ('famille', 'eleve', 'etablissement')),
    du                DATE NOT NULL,
    enregistre_par    UUID,
    enregistre_le     TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_par          UUID,
    leve_le           TIMESTAMPTZ,
    CONSTRAINT ent_eleve_photo_refus_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_photo_refus_ouvert_uniq
    ON ent_eleve_photo_refus (etablissement_id, eleve_id) WHERE leve_le IS NULL;

CREATE OR REPLACE FUNCTION ent_eleve_photo_refus_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'refus de photo : une ligne ne se supprime pas (la lever)' USING ERRCODE = '42501';
    END IF;
    IF OLD.leve_le IS NULL AND NEW.leve_le IS NOT NULL
       AND (to_jsonb(NEW) - 'leve_le' - 'leve_par') = (to_jsonb(OLD) - 'leve_le' - 'leve_par') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'refus de photo : seule la levée se pose' USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_photo_refus_garde ON ent_eleve_photo_refus;
CREATE TRIGGER ent_eleve_photo_refus_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_photo_refus
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_photo_refus_garde();

-- ───────────────────────── Coordonnées propres de l'élève (ELV-17) ─────────────────────────
-- E-mail et mobile de l'élève LUI-MÊME (jamais ceux des responsables : socle 4). Une coordonnée
-- n'est utilisable pour un envoi sensible qu'après vérification par code (`verifiee_le`).
-- États : proposée (par la famille, à valider par le secrétariat) → active | refusée ; active →
-- retirée. La valeur ne change jamais : remplacer = retirer puis ajouter.
CREATE TABLE IF NOT EXISTS ent_eleve_coordonnee (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id          UUID NOT NULL,
    type              TEXT NOT NULL CHECK (type IN ('email', 'mobile')),
    valeur            TEXT NOT NULL,
    usages            TEXT[] NOT NULL DEFAULT '{}'::TEXT[] CHECK (usages <@ ARRAY['notifications', 'compte', 'urgence']::TEXT[]),
    source            TEXT NOT NULL CHECK (source IN ('etablissement', 'eleve', 'famille', 'import')),
    statut            TEXT NOT NULL CHECK (statut IN ('proposee', 'active', 'refusee', 'retiree')),
    verifiee_le       TIMESTAMPTZ,
    du                DATE NOT NULL,
    au                DATE,
    cree_par          UUID,
    cree_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    decide_par        UUID,
    decide_le         TIMESTAMPTZ,
    motif_code        TEXT CHECK (motif_code IS NULL OR motif_code IN
                          ('non_conforme', 'doublon', 'refus_etablissement', 'remplacee',
                           'erreur_de_saisie', 'demande_eleve', 'demande_famille', 'autre')),
    CONSTRAINT ent_eleve_coordonnee_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_coordonnee_valeur_chk CHECK (
        (type = 'email' AND char_length(valeur) BETWEEN 6 AND 254
            AND valeur ~ '^[^@[:space:]]+@[^@[:space:]]+\.[^@[:space:]]+$' AND valeur = lower(valeur))
        OR (type = 'mobile' AND valeur ~ '^\+[1-9][0-9]{6,14}$')),
    CONSTRAINT ent_eleve_coordonnee_periode_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_eleve_coordonnee_decision_chk CHECK (
        (statut = 'proposee' AND decide_le IS NULL)
        OR (statut <> 'proposee' AND (decide_le IS NOT NULL OR source <> 'famille'))),
    -- Seule une coordonnée active peut être vérifiée.
    CONSTRAINT ent_eleve_coordonnee_verif_chk CHECK (verifiee_le IS NULL OR statut IN ('active', 'retiree'))
);
-- Pas deux fois la même coordonnée vivante pour un élève.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_coordonnee_vivante_uniq
    ON ent_eleve_coordonnee (etablissement_id, eleve_id, type, valeur) WHERE statut IN ('proposee', 'active');
CREATE INDEX IF NOT EXISTS ent_eleve_coordonnee_eleve_idx
    ON ent_eleve_coordonnee (etablissement_id, eleve_id, cree_le DESC);

-- Transitions permises : proposée → active | refusée ; active → retirée ; `verifiee_le` posée une
-- fois (active seulement) ; `usages` modifiable tant que la coordonnée est vivante. Rien d'autre
-- ne change, rien ne se supprime.
CREATE OR REPLACE FUNCTION ent_eleve_coordonnee_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'coordonnée de l''élève : une ligne ne se supprime pas (la retirer)' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut IN ('refusee', 'retiree') THEN
        RAISE EXCEPTION 'coordonnée de l''élève : une coordonnée retirée ou refusée est figée' USING ERRCODE = '42501';
    END IF;
    IF OLD.statut = 'proposee' AND NEW.statut = 'active'
       AND (to_jsonb(NEW) - 'statut' - 'decide_par' - 'decide_le' - 'usages' - 'du')
         = (to_jsonb(OLD) - 'statut' - 'decide_par' - 'decide_le' - 'usages' - 'du') THEN
        RETURN NEW;
    END IF;
    IF OLD.statut = 'proposee' AND NEW.statut = 'refusee'
       AND (to_jsonb(NEW) - 'statut' - 'decide_par' - 'decide_le' - 'motif_code' - 'au')
         = (to_jsonb(OLD) - 'statut' - 'decide_par' - 'decide_le' - 'motif_code' - 'au') THEN
        RETURN NEW;
    END IF;
    IF OLD.statut = 'active' AND NEW.statut = 'retiree'
       AND (to_jsonb(NEW) - 'statut' - 'decide_par' - 'decide_le' - 'motif_code' - 'au')
         = (to_jsonb(OLD) - 'statut' - 'decide_par' - 'decide_le' - 'motif_code' - 'au') THEN
        RETURN NEW;
    END IF;
    IF OLD.statut = 'active' AND NEW.statut = 'active'
       AND OLD.verifiee_le IS NULL AND NEW.verifiee_le IS NOT NULL
       AND (to_jsonb(NEW) - 'verifiee_le') = (to_jsonb(OLD) - 'verifiee_le') THEN
        RETURN NEW;
    END IF;
    IF OLD.statut = 'active' AND NEW.statut = 'active'
       AND (to_jsonb(NEW) - 'usages') = (to_jsonb(OLD) - 'usages') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'coordonnée de l''élève : transition ou modification refusée' USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_coordonnee_garde ON ent_eleve_coordonnee;
CREATE TRIGGER ent_eleve_coordonnee_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_coordonnee
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_coordonnee_garde();

-- Vérification par code (même principe que `ent_coordonnee_verification` du socle 4) : seule
-- l'empreinte HMAC du code est stockée ; au plus un code en attente par coordonnée.
CREATE TABLE IF NOT EXISTS ent_eleve_coordonnee_verification (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id          UUID NOT NULL,
    coordonnee_id     UUID NOT NULL REFERENCES ent_eleve_coordonnee (id),
    canal             TEXT NOT NULL CHECK (canal IN ('email', 'sms')),
    code_hash         TEXT NOT NULL,
    expire_le         TIMESTAMPTZ NOT NULL,
    tentatives        SMALLINT NOT NULL DEFAULT 0 CHECK (tentatives >= 0),
    statut            TEXT NOT NULL DEFAULT 'envoye'
                          CHECK (statut IN ('envoye', 'verifiee', 'expiree', 'annulee', 'epuisee')),
    demande_par       UUID,
    envoye_le         TIMESTAMPTZ NOT NULL DEFAULT now(),
    verifiee_le       TIMESTAMPTZ,
    CONSTRAINT ent_eleve_coordonnee_verification_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_coordonnee_verification_attente_uniq
    ON ent_eleve_coordonnee_verification (coordonnee_id) WHERE statut = 'envoye';
CREATE INDEX IF NOT EXISTS ent_eleve_coordonnee_verification_eleve_idx
    ON ent_eleve_coordonnee_verification (etablissement_id, eleve_id, envoye_le);

-- ───────────────────────── Scellement des versions d'identité sensibles ─────────────────────
-- Une version SENSIBLE (prénom d'usage, mention de sexe, adoption, protection…) qui n'est plus la
-- courante garde sa trace (date, nature, motif codé, auteur, pièce) mais ses VALEURS intimes sont
-- scellées ici, une ligne par champ ; les colonnes en clair de la version sont remplacées par un
-- marqueur. Seul le service (qui détient la clé) scelle et descelle ; la base n'autorise que cette
-- opération sur une version sensible non courante (drapeau `ent.identite_scellement`).
CREATE TABLE IF NOT EXISTS ent_eleve_identite_scellee (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id          UUID NOT NULL,
    version_id        UUID NOT NULL REFERENCES ent_eleve_identite (id),
    champ             TEXT NOT NULL CHECK (champ IN ('nom_naissance', 'nom_usage', 'prenoms', 'rang_prenom_usuel',
                                                    'prenom_usage', 'sexe', 'nom_ecriture_origine',
                                                    'prenoms_ecriture_origine')),
    valeur_scellee    TEXT NOT NULL CHECK (valeur_scellee LIKE 'enc:v%'),
    scelle_le         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_identite_scellee_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_identite_scellee_uniq UNIQUE (version_id, champ)
);
CREATE INDEX IF NOT EXISTS ent_eleve_identite_scellee_eleve_idx
    ON ent_eleve_identite_scellee (etablissement_id, eleve_id);

CREATE OR REPLACE FUNCTION ent_eleve_identite_scellee_ajout_seul() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'identité scellée : ajout seul (% interdit)', TG_OP USING ERRCODE = '42501';
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identite_scellee_ajout_seul ON ent_eleve_identite_scellee;
CREATE TRIGGER ent_eleve_identite_scellee_ajout_seul
    BEFORE UPDATE OR DELETE ON ent_eleve_identite_scellee
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identite_scellee_ajout_seul();

-- Garde d'immuabilité des versions (280), élargie d'une seule opération : le scellement. Hors
-- scellement, le comportement est strictement celui d'avant (seule `remplacee_par` se pose).
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
    -- Scellement : version sensible, non courante, déjà non scellée ; seules les colonnes de
    -- valeurs intimes changent, et chacune des colonnes vidées a sa ligne scellée.
    IF coalesce(current_setting('ent.identite_scellement', true), '') = 'on'
       AND OLD.sensible
       AND OLD.nom_naissance <> '[scellé]'
       AND (OLD.remplacee_par IS NOT NULL
            OR (OLD.effet_du <= ent_jour_etablissement(OLD.etablissement_id)
                AND ent_eleve_identite_courante(OLD.etablissement_id, OLD.eleve_id,
                                                ent_jour_etablissement(OLD.etablissement_id))
                    IS DISTINCT FROM OLD.id))
       AND (to_jsonb(NEW) - 'nom_naissance' - 'nom_usage' - 'prenoms' - 'rang_prenom_usuel' - 'prenom_usage'
                          - 'sexe' - 'nom_ecriture_origine' - 'prenoms_ecriture_origine')
         = (to_jsonb(OLD) - 'nom_naissance' - 'nom_usage' - 'prenoms' - 'rang_prenom_usuel' - 'prenom_usage'
                          - 'sexe' - 'nom_ecriture_origine' - 'prenoms_ecriture_origine')
       AND NEW.nom_naissance = '[scellé]' AND NEW.prenoms = ARRAY['[scellé]']::TEXT[]
       AND NEW.rang_prenom_usuel = 1
       AND NEW.nom_usage IS NULL AND NEW.prenom_usage IS NULL AND NEW.sexe IS NULL
       AND NEW.nom_ecriture_origine IS NULL AND NEW.prenoms_ecriture_origine IS NULL
       AND EXISTS (SELECT 1 FROM ent_eleve_identite_scellee s
                    WHERE s.version_id = OLD.id AND s.champ = 'nom_naissance')
       AND EXISTS (SELECT 1 FROM ent_eleve_identite_scellee s
                    WHERE s.version_id = OLD.id AND s.champ = 'prenoms') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'identité : une version est immuable (créer une correction ou un changement)'
        USING ERRCODE = '42501';
END $$;

-- ───────────────────────── RLS d'établissement (M9) ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_photo', 'ent_eleve_photo_refus', 'ent_eleve_coordonnee',
                             'ent_eleve_coordonnee_verification', 'ent_eleve_identite_scellee']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL
                USING (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
                WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))', t);
    END LOOP;
END $$;
