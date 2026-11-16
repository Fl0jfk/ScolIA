-- Messagerie, lot M4 (`docs/socles/16_MESSAGERIE.md` §20 R-M13, R-M20, R-M21, R-M27, R-M28 ; CM-26, CM-45,
-- CM-54 à CM-56, CM-79) : lots individuels (30 fils au plus, aperçu et confirmation), modèles à portée,
-- programmation, correction (15 min), retrait (10 min), brouillons, fusion et scission de fils. Plage 480–499.
-- Rejouable. Aucune suppression physique par un utilisateur (R-M28) : un retrait est un masquage tracé.

-- 1. Lots individuels -------------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_lot (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    auteur_user_id UUID NOT NULL,
    -- SHA-256 (hex) du texte et des élèves de l'aperçu : la confirmation doit rendre exactement le même contenu.
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    nb_eleves SMALLINT NOT NULL CHECK (nb_eleves BETWEEN 1 AND 30),
    -- R-M13 : un lot individuel ne dépasse jamais 30 fils (S-139) ; au-delà, publication ou diffusion.
    nb_fils SMALLINT NOT NULL CHECK (nb_fils BETWEEN 1 AND 30),
    nb_exclus SMALLINT NOT NULL DEFAULT 0 CHECK (nb_exclus >= 0),
    etat TEXT NOT NULL DEFAULT 'apercu' CHECK (etat IN ('apercu', 'confirme', 'expire')),
    assurance TEXT NOT NULL DEFAULT 'simple' CHECK (assurance IN ('simple', 'passkey')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    expire_le TIMESTAMPTZ NOT NULL,
    confirme_le TIMESTAMPTZ,
    CONSTRAINT ent_msg_lot_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_msg_lot_confirme_chk CHECK ((etat = 'confirme') = (confirme_le IS NOT NULL))
);
CREATE TABLE IF NOT EXISTS ent_msg_lot_eleve (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lot_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    PRIMARY KEY (lot_id, eleve_id),
    CONSTRAINT ent_msg_lot_eleve_lot_fk FOREIGN KEY (etablissement_id, lot_id)
        REFERENCES ent_msg_lot (etablissement_id, id)
);

-- 2. Colonnes des fils et des messages ----------------------------------------------------------------------------
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS lot_id UUID;
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS scinde_de UUID;
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS fusionne_dans UUID;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS programme_le TIMESTAMPTZ;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS retire_le TIMESTAMPTZ;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS retire_par UUID;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS retrait_par TEXT;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS retrait_motif_code TEXT;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS fil_origine_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_lot_fk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_lot_fk
            FOREIGN KEY (etablissement_id, lot_id) REFERENCES ent_msg_lot (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_scinde_fk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_scinde_fk
            FOREIGN KEY (etablissement_id, scinde_de) REFERENCES ent_msg_fil (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_fusionne_fk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_fusionne_fk
            FOREIGN KEY (etablissement_id, fusionne_dans) REFERENCES ent_msg_fil (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_retrait_chk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_retrait_chk CHECK (
            (retire_le IS NULL) = (retire_par IS NULL)
            AND (retire_le IS NULL) = (retrait_par IS NULL)
            AND (retrait_par IS NULL OR retrait_par IN ('auteur', 'etablissement'))
            AND (retrait_motif_code IS NULL OR retrait_motif_code ~ '^[a-z][a-z0-9_]{1,39}$'));
    END IF;
END $$;

-- 3. Modèles à portée : personnel, boîte, établissement (§19.2) --------------------------------------------------------
-- Le propriétaire d'un modèle personnel est `created_by_user_id`. Les lignes existantes restent à la portée
-- « établissement » (comportement actuel).
ALTER TABLE ent_msg_modele ADD COLUMN IF NOT EXISTS portee TEXT NOT NULL DEFAULT 'etablissement';
ALTER TABLE ent_msg_modele ADD COLUMN IF NOT EXISTS boite_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_modele_portee_chk') THEN
        ALTER TABLE ent_msg_modele ADD CONSTRAINT ent_msg_modele_portee_chk CHECK (
            portee IN ('personnel', 'boite', 'etablissement') AND ((portee = 'boite') = (boite_id IS NOT NULL)));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_modele_boite_fk') THEN
        ALTER TABLE ent_msg_modele ADD CONSTRAINT ent_msg_modele_boite_fk
            FOREIGN KEY (etablissement_id, boite_id) REFERENCES ent_msg_boite (etablissement_id, id);
    END IF;
END $$;

-- 4. Brouillons : strictement personnels --------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_brouillon (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- Réponse dans un fil existant, ou premier message à propos d'un élève.
    fil_id UUID,
    eleve_id UUID,
    sujet TEXT NOT NULL DEFAULT '' CHECK (char_length(sujet) <= 200),
    corps TEXT NOT NULL DEFAULT '' CHECK (char_length(corps) <= 20000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Un brouillon oublié n'est pas gardé indéfiniment (minimisation).
    expire_le TIMESTAMPTZ NOT NULL DEFAULT now() + interval '90 days',
    CONSTRAINT ent_msg_brouillon_cible_chk CHECK (fil_id IS NOT NULL OR eleve_id IS NOT NULL),
    CONSTRAINT ent_msg_brouillon_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_brouillon_auteur_idx ON ent_msg_brouillon (etablissement_id, auteur_user_id, maj_le DESC);

-- 5. Fusion de deux fils d'une même famille (MSG-174) ----------------------------------------------------------------
-- Rend le nombre de messages déplacés, ou NULL si les fils ne sont pas fusionnables : même élève, même nature,
-- même sensibilité, même audience, même foyer, mêmes participants **famille et élève**, mêmes boîtes. Les messages
-- sont déplacés (jamais copiés), avec leurs pièces et accusés ; `fil_origine_id` garde d'où ils viennent. Les
-- notes internes (ajout seul) sont copiées. Le fil absorbé est clos, archivé, rattaché (`fusionne_dans`).
CREATE OR REPLACE FUNCTION ent_msg_fusionner(p_garde UUID, p_absorbe UUID) RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE
    a ent_msg_fil%ROWTYPE;
    b ent_msg_fil%ROWTYPE;
    k INTEGER;
BEGIN
    IF p_garde = p_absorbe THEN
        RETURN NULL;
    END IF;
    SELECT * INTO a FROM ent_msg_fil WHERE id = p_garde FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT * INTO b FROM ent_msg_fil WHERE id = p_absorbe AND etablissement_id = a.etablissement_id FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF a.etat <> 'ouvert' OR b.etat <> 'ouvert' OR a.fusionne_dans IS NOT NULL OR b.fusionne_dans IS NOT NULL THEN
        RETURN NULL;
    END IF;
    IF (a.eleve_id, a.nature, a.sensibilite, a.audience, a.vie_scolaire)
         IS DISTINCT FROM (b.eleve_id, b.nature, b.sensibilite, b.audience, b.vie_scolaire)
       OR a.foyer_id IS DISTINCT FROM b.foyer_id
       OR a.foyer_lecteur_id IS DISTINCT FROM b.foyer_lecteur_id
       OR a.prive_user_id IS DISTINCT FROM b.prive_user_id THEN
        RETURN NULL;
    END IF;
    IF (SELECT COALESCE(array_agg(x ORDER BY x), ARRAY[]::uuid[]) FROM (
            SELECT COALESCE(personne_id, eleve_id, boite_id) AS x FROM ent_msg_participant
             WHERE etablissement_id = a.etablissement_id AND fil_id = a.id AND au IS NULL) s)
       IS DISTINCT FROM
       (SELECT COALESCE(array_agg(x ORDER BY x), ARRAY[]::uuid[]) FROM (
            SELECT COALESCE(personne_id, eleve_id, boite_id) AS x FROM ent_msg_participant
             WHERE etablissement_id = b.etablissement_id AND fil_id = b.id AND au IS NULL) s) THEN
        -- Le personnel peut différer (un autre professeur y a écrit) ; famille, élève et boîtes non.
        IF (SELECT COALESCE(array_agg(x ORDER BY x), ARRAY[]::uuid[]) FROM (
                SELECT COALESCE(personne_id, eleve_id, boite_id) AS x FROM ent_msg_participant
                 WHERE etablissement_id = a.etablissement_id AND fil_id = a.id AND au IS NULL
                   AND (cote <> 'etablissement' OR boite_id IS NOT NULL)) s)
           IS DISTINCT FROM
           (SELECT COALESCE(array_agg(x ORDER BY x), ARRAY[]::uuid[]) FROM (
                SELECT COALESCE(personne_id, eleve_id, boite_id) AS x FROM ent_msg_participant
                 WHERE etablissement_id = b.etablissement_id AND fil_id = b.id AND au IS NULL
                   AND (cote <> 'etablissement' OR boite_id IS NOT NULL)) s) THEN
            RETURN NULL;
        END IF;
    END IF;

    UPDATE ent_msg SET fil_origine_id = p_absorbe, fil_id = p_garde
     WHERE etablissement_id = a.etablissement_id AND fil_id = p_absorbe;
    GET DIAGNOSTICS k = ROW_COUNT;
    UPDATE ent_msg_accuse SET fil_id = p_garde
     WHERE etablissement_id = a.etablissement_id AND fil_id = p_absorbe;
    UPDATE ent_msg_piece SET fil_id = p_garde
     WHERE etablissement_id = a.etablissement_id AND fil_id = p_absorbe;
    -- Le personnel de l'absorbé suit (ceux qui n'y étaient pas déjà), pour ne perdre aucun lecteur.
    INSERT INTO ent_msg_participant
        (id, etablissement_id, fil_id, personne_id, cote, qualite, raison, peut_ecrire, du)
    SELECT gen_random_uuid(), p.etablissement_id, p_garde, p.personne_id, p.cote, p.qualite, p.raison,
           p.peut_ecrire, p.du
      FROM ent_msg_participant p
     WHERE p.etablissement_id = a.etablissement_id AND p.fil_id = p_absorbe AND p.au IS NULL
       AND p.personne_id IS NOT NULL AND p.cote = 'etablissement'
       AND NOT EXISTS (SELECT 1 FROM ent_msg_participant q
                        WHERE q.fil_id = p_garde AND q.personne_id = p.personne_id AND q.au IS NULL);
    -- Notes internes (ajout seul) : copiées vers le fil gardé, dans la même boîte.
    INSERT INTO ent_msg_note_interne
        (id, etablissement_id, fil_id, boite_id, auteur_personne_id, corps, transfert, cree_le)
    SELECT gen_random_uuid(), n.etablissement_id, p_garde, n.boite_id, n.auteur_personne_id, n.corps,
           n.transfert, n.cree_le
      FROM ent_msg_note_interne n
     WHERE n.etablissement_id = a.etablissement_id AND n.fil_id = p_absorbe;
    UPDATE ent_msg_participant SET au = now(), motif_sortie = 'autre'
     WHERE etablissement_id = a.etablissement_id AND fil_id = p_absorbe AND au IS NULL;
    UPDATE ent_msg_fil SET etat = 'clos', clos_le = now(), archive = true, archived_at = now(),
                           fusionne_dans = p_garde
     WHERE id = p_absorbe;
    UPDATE ent_msg_fil SET last_message_at = GREATEST(a.last_message_at, b.last_message_at)
     WHERE id = p_garde;
    RETURN k;
END
$$;

-- 6. Scission d'un fil (à partir d'un message) ------------------------------------------------------------------------
-- Crée un fil frère (mêmes participants, même portée), y déplace le message `p_message` et les suivants. Rend le
-- nouveau fil, ou NULL si le message est le premier du fil (rien à scinder) ou n'appartient pas au fil.
CREATE OR REPLACE FUNCTION ent_msg_scinder(p_fil UUID, p_message UUID, p_sujet TEXT) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE
    f ent_msg_fil%ROWTYPE;
    g ent_msg_fil%ROWTYPE;
    m0 ent_msg%ROWTYPE;
    v_new UUID := gen_random_uuid();
BEGIN
    SELECT * INTO f FROM ent_msg_fil WHERE id = p_fil FOR UPDATE;
    IF NOT FOUND OR f.etat <> 'ouvert' THEN RETURN NULL; END IF;
    SELECT * INTO m0 FROM ent_msg WHERE id = p_message AND fil_id = p_fil;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_msg m WHERE m.fil_id = p_fil
                      AND (m.created_at, m.id) < (m0.created_at, m0.id)) THEN
        RETURN NULL;
    END IF;
    g := f;
    g.id := v_new;
    g.sujet := p_sujet;
    g.scinde_de := f.id;
    g.created_at := now();
    g.last_message_at := m0.created_at;
    g.reprise_m1_le := now();
    INSERT INTO ent_msg_fil SELECT (g).*;
    INSERT INTO ent_msg_fil_eleve (etablissement_id, fil_id, eleve_id)
    SELECT e.etablissement_id, v_new, e.eleve_id FROM ent_msg_fil_eleve e WHERE e.fil_id = p_fil
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_msg_participant
        (id, etablissement_id, fil_id, personne_id, eleve_id, boite_id, cote, qualite, raison, peut_ecrire, du)
    SELECT gen_random_uuid(), p.etablissement_id, v_new, p.personne_id, p.eleve_id, p.boite_id, p.cote,
           p.qualite, p.raison, p.peut_ecrire, p.du
      FROM ent_msg_participant p
     WHERE p.etablissement_id = f.etablissement_id AND p.fil_id = p_fil AND p.au IS NULL;
    INSERT INTO ent_msg_traitement (etablissement_id, fil_id, boite_id, statut, urgence)
    SELECT t.etablissement_id, v_new, t.boite_id, 'nouveau', false
      FROM ent_msg_traitement t
     WHERE t.fil_id = p_fil AND t.statut <> 'transfere';
    UPDATE ent_msg SET fil_origine_id = p_fil, fil_id = v_new
     WHERE etablissement_id = f.etablissement_id AND fil_id = p_fil
       AND (created_at, id) >= (m0.created_at, m0.id);
    UPDATE ent_msg_accuse a SET fil_id = v_new
     WHERE a.etablissement_id = f.etablissement_id AND a.fil_id = p_fil
       AND a.message_id IN (SELECT id FROM ent_msg WHERE fil_id = v_new);
    UPDATE ent_msg_piece pc SET fil_id = v_new
     WHERE pc.etablissement_id = f.etablissement_id AND pc.fil_id = p_fil
       AND pc.message_id IN (SELECT id FROM ent_msg WHERE fil_id = v_new);
    UPDATE ent_msg_fil SET last_message_at = COALESCE(
        (SELECT max(created_at) FROM ent_msg WHERE fil_id = p_fil), f.created_at)
     WHERE id = p_fil;
    UPDATE ent_msg_fil SET last_message_at = COALESCE(
        (SELECT max(created_at) FROM ent_msg WHERE fil_id = v_new), now())
     WHERE id = v_new;
    RETURN v_new;
END
$$;

-- 7. RLS permissive ------------------------------------------------------------------------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_msg_lot', 'ent_msg_lot_eleve', 'ent_msg_brouillon'];
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
