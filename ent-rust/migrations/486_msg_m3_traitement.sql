-- Messagerie, lot M3 (suite) : traitement des fils d'une boîte (états, prise en charge, échéance, étiquettes),
-- notes internes, transferts, messages téléphoniques. Plage 480–499. Rejouable.
--
-- Écart assumé avec le §25 (qui rangeait la note interne dans `ent_msg`, type `note_interne`) : une note interne
-- est dans **sa propre table**. Aucune lecture d'`ent_msg` (liste, accusés, compteurs, pièces) ne peut donc la
-- montrer à une famille : la fuite est impossible par construction, pas par un filtre à ne pas oublier.

-- 1. Traitement : un par (fil, boîte) --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_traitement (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    boite_id UUID NOT NULL,
    statut TEXT NOT NULL DEFAULT 'nouveau'
        CHECK (statut IN ('nouveau', 'pris', 'attente_famille', 'traite', 'transfere')),
    pris_par_personne_id UUID,
    pris_le TIMESTAMPTZ,
    -- Échéance : un jour d'établissement, jamais un instant.
    echeance DATE,
    -- Tri déterministe (mots d'alerte), jamais une décision : il ordonne la file.
    urgence BOOLEAN NOT NULL DEFAULT false,
    reponse_auto_le TIMESTAMPTZ,
    traite_le TIMESTAMPTZ,
    traite_par_personne_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (fil_id, boite_id),
    CONSTRAINT ent_msg_traitement_pris_chk CHECK (statut <> 'pris' OR pris_par_personne_id IS NOT NULL),
    CONSTRAINT ent_msg_traitement_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_traitement_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_traitement_pris_par_fk FOREIGN KEY (etablissement_id, pris_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_traitement_file_idx
    ON ent_msg_traitement (etablissement_id, boite_id, statut, echeance);

-- 2. Étiquettes (liste courte par boîte, S-135) -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_etiquette (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    boite_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,29}$'),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 40),
    CONSTRAINT ent_msg_etiquette_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_msg_etiquette_code_uq UNIQUE (boite_id, code),
    CONSTRAINT ent_msg_etiquette_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id)
);
CREATE TABLE IF NOT EXISTS ent_msg_traitement_etiquette (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    boite_id UUID NOT NULL,
    etiquette_id UUID NOT NULL,
    PRIMARY KEY (fil_id, boite_id, etiquette_id),
    CONSTRAINT ent_msg_traitement_etiquette_tr_fk FOREIGN KEY (fil_id, boite_id)
        REFERENCES ent_msg_traitement (fil_id, boite_id),
    CONSTRAINT ent_msg_traitement_etiquette_et_fk FOREIGN KEY (etablissement_id, etiquette_id)
        REFERENCES ent_msg_etiquette (etablissement_id, id)
);

-- 3. Notes internes : invisibles de la famille (MSG-165) ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_note_interne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    boite_id UUID NOT NULL,
    auteur_personne_id UUID NOT NULL,
    corps TEXT NOT NULL CHECK (char_length(btrim(corps)) BETWEEN 1 AND 5000),
    -- Note posée par le système lors d'un transfert (jamais un texte de la famille).
    transfert BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_note_interne_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_note_interne_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_note_interne_auteur_fk FOREIGN KEY (etablissement_id, auteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_note_interne_fil_idx ON ent_msg_note_interne (etablissement_id, fil_id, cree_le);
DROP TRIGGER IF EXISTS ent_msg_note_interne_ajout_seul ON ent_msg_note_interne;
CREATE TRIGGER ent_msg_note_interne_ajout_seul BEFORE UPDATE OR DELETE ON ent_msg_note_interne
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- 4. Transferts d'une boîte à l'autre (MSG-164) : ajout seul ------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_transfert (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    de_boite_id UUID NOT NULL,
    vers_boite_id UUID NOT NULL,
    par_personne_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_transfert_distinct_chk CHECK (de_boite_id <> vers_boite_id),
    CONSTRAINT ent_msg_transfert_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_transfert_de_fk FOREIGN KEY (etablissement_id, de_boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_transfert_vers_fk FOREIGN KEY (etablissement_id, vers_boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_transfert_par_fk FOREIGN KEY (etablissement_id, par_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_msg_transfert_ajout_seul ON ent_msg_transfert;
CREATE TRIGGER ent_msg_transfert_ajout_seul BEFORE UPDATE OR DELETE ON ent_msg_transfert
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- 5. Messages téléphoniques (MSG-171, CM-49) -------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_msg_telephone (
    message_id UUID PRIMARY KEY REFERENCES ent_msg (id),
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    boite_id UUID NOT NULL,
    -- Personne présumée (fiche), sinon libellé libre : jamais un numéro ni une adresse (R-M4).
    appelant_personne_id UUID,
    appelant_libelle TEXT CHECK (appelant_libelle IS NULL OR char_length(btrim(appelant_libelle)) BETWEEN 1 AND 120),
    motif_code TEXT NOT NULL CHECK (motif_code IN ('absence_signalee', 'information', 'rendez_vous', 'autre')),
    rappel_demande BOOLEAN NOT NULL DEFAULT false,
    saisi_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_telephone_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_telephone_boite_fk FOREIGN KEY (etablissement_id, boite_id)
        REFERENCES ent_msg_boite (etablissement_id, id),
    CONSTRAINT ent_msg_telephone_appelant_chk CHECK (appelant_personne_id IS NOT NULL OR appelant_libelle IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_msg_telephone_saisie_idx ON ent_msg_telephone (etablissement_id, saisi_par_user_id, cree_le);

-- 6. RLS permissive -------------------------------------------------------------------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_msg_traitement', 'ent_msg_etiquette', 'ent_msg_traitement_etiquette',
                           'ent_msg_note_interne', 'ent_msg_transfert', 'ent_msg_telephone'];
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
