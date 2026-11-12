-- 2381 — Socle 19, lot AG1 : noyau de l'agenda (événements, séries, audience, participants,
-- réponses, coupon, journal). Réf. : 19_AGENDA_RDV.md §25, §21.1, §22, R-AG3/R-AG5/R-AG8.
--
-- * Chaque table : `etablissement_id NOT NULL`, `UNIQUE (etablissement_id, id)`, références
--   composites, RLS posée par `ent_rls_poser`, aucune colonne JSON (D-036), aucun BYTEA (E5).
-- * Personnes : identifiant de compte (`ent_user`, table d'identité commune aux établissements,
--   donc sans clé composite possible) ; l'appartenance à l'établissement est contrôlée à l'écriture
--   et l'historique survit au retrait d'un accès (pas de cascade).
-- * Le journal est en AJOUT SEUL (`ent_agenda_ajout_seul`, migration 2380) ; `avant` / `apres`
--   sont des textes figés, jamais relus comme source de vérité.
-- * Rejouable (IF NOT EXISTS, contraintes nommées).

SELECT ent_d036_contrainte('ent_salle', 'ent_salle_etab_id_uq', 'UNIQUE (etablissement_id, id)');

-- Types d'événements propres à l'établissement (en plus des types intégrés du code).
CREATE TABLE IF NOT EXISTS ent_agenda_type_local (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 80),
    famille TEXT NOT NULL CHECK (famille IN ('reunion', 'evenement', 'manifestation', 'journee')),
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_type_local_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_type_local_code_uq UNIQUE (etablissement_id, code)
);

-- Série récurrente : les occurrences sont MATÉRIALISÉES (une ligne d'événement chacune), pour
-- pouvoir les modifier, les annuler ou les détacher une à une.
CREATE TABLE IF NOT EXISTS ent_agenda_serie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    frequence TEXT NOT NULL CHECK (frequence IN ('hebdomadaire')),
    intervalle SMALLINT NOT NULL DEFAULT 1 CHECK (intervalle BETWEEN 1 AND 8),
    du DATE NOT NULL,
    au DATE NOT NULL,
    cree_par_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_serie_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_serie_dates_chk CHECK (au >= du AND au <= du + 730)
);

CREATE TABLE IF NOT EXISTS ent_agenda_serie_jour (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    serie_id UUID NOT NULL,
    jour_semaine SMALLINT NOT NULL CHECK (jour_semaine BETWEEN 1 AND 7),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, serie_id, jour_semaine),
    CONSTRAINT ent_agenda_serie_jour_serie_fk FOREIGN KEY (etablissement_id, serie_id)
        REFERENCES ent_agenda_serie (etablissement_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ent_agenda_evenement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Type intégré (code) ; `type_local_id` quand le code désigne un type de l'établissement.
    type_code TEXT NOT NULL CHECK (type_code ~ '^[a-z][a-z0-9_]{1,39}$'),
    type_local_id UUID,
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    description TEXT CHECK (description IS NULL OR length(description) <= 4000),
    -- Heures en `timestamptz` calculées depuis l'heure murale et le fuseau de l'établissement ;
    -- journée entière : `jour_debut` / `jour_fin` (inclus) et `debut` / `fin` à minuit local.
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    journee_entiere BOOLEAN NOT NULL DEFAULT false,
    jour_debut DATE NOT NULL,
    jour_fin DATE NOT NULL,
    lieu_kind TEXT NOT NULL DEFAULT 'aucun' CHECK (lieu_kind IN ('aucun', 'salle', 'texte')),
    salle_id UUID,
    lieu_texte TEXT CHECK (lieu_texte IS NULL OR length(btrim(lieu_texte)) BETWEEN 1 AND 200),
    organisateur_user_id UUID NOT NULL,
    -- Qui voit le titre et le motif (R-AG5) : `etablissement` (tout le monde, partie publique),
    -- `participants` (invités seulement), `restreint` (logistique seule hors participants).
    confidentialite TEXT NOT NULL DEFAULT 'participants'
        CHECK (confidentialite IN ('etablissement', 'participants', 'restreint')),
    statut TEXT NOT NULL DEFAULT 'brouillon'
        CHECK (statut IN ('brouillon', 'publie', 'annule', 'clos')),
    motif_annulation TEXT CHECK (motif_annulation IS NULL OR length(motif_annulation) <= 300),
    serie_id UUID,
    occurrence_detachee BOOLEAN NOT NULL DEFAULT false,
    -- Origine quand un autre socle ouvre l'événement (conseil de classe, voyage…) : contenu chez lui.
    origine_brique TEXT CHECK (origine_brique IS NULL OR length(origine_brique) <= 40),
    origine_id UUID,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    publie_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_evenement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_evenement_plage_chk CHECK (fin > debut AND jour_fin >= jour_debut),
    CONSTRAINT ent_agenda_evenement_duree_chk CHECK (fin <= debut + interval '400 days'),
    CONSTRAINT ent_agenda_evenement_lieu_chk CHECK (
        (lieu_kind = 'aucun' AND salle_id IS NULL AND lieu_texte IS NULL)
        OR (lieu_kind = 'salle' AND salle_id IS NOT NULL AND lieu_texte IS NULL)
        OR (lieu_kind = 'texte' AND salle_id IS NULL AND lieu_texte IS NOT NULL)),
    CONSTRAINT ent_agenda_evenement_annule_chk CHECK (
        statut <> 'annule' OR length(btrim(coalesce(motif_annulation, ''))) > 0),
    CONSTRAINT ent_agenda_evenement_type_local_fk FOREIGN KEY (etablissement_id, type_local_id)
        REFERENCES ent_agenda_type_local (etablissement_id, id),
    CONSTRAINT ent_agenda_evenement_salle_fk FOREIGN KEY (etablissement_id, salle_id)
        REFERENCES ent_salle (etablissement_id, id),
    CONSTRAINT ent_agenda_evenement_serie_fk FOREIGN KEY (etablissement_id, serie_id)
        REFERENCES ent_agenda_serie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_agenda_evenement_plage_idx
    ON ent_agenda_evenement (etablissement_id, debut, fin) WHERE statut <> 'brouillon';
CREATE INDEX IF NOT EXISTS ent_agenda_evenement_orga_idx
    ON ent_agenda_evenement (etablissement_id, organisateur_user_id, debut);
CREATE INDEX IF NOT EXISTS ent_agenda_evenement_serie_idx
    ON ent_agenda_evenement (etablissement_id, serie_id, debut) WHERE serie_id IS NOT NULL;

-- Audience calculée (R-AG8) : résolue À LA PUBLICATION, sous les droits de l'auteur.
CREATE TABLE IF NOT EXISTS ent_agenda_audience (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('classe', 'niveau', 'groupe', 'etablissement', 'tous_personnels')),
    cible_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_audience_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_audience_evenement_fk FOREIGN KEY (etablissement_id, evenement_id)
        REFERENCES ent_agenda_evenement (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_agenda_audience_cible_chk CHECK (
        (kind IN ('etablissement', 'tous_personnels')) = (cible_id IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_agenda_audience_uq
    ON ent_agenda_audience (etablissement_id, evenement_id, kind, coalesce(cible_id, '00000000-0000-0000-0000-000000000000'::uuid));

CREATE TABLE IF NOT EXISTS ent_agenda_participant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    user_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('organisateur', 'invite', 'famille')),
    obligatoire BOOLEAN NOT NULL DEFAULT false,
    source TEXT NOT NULL CHECK (source IN ('organisateur', 'nominatif', 'audience')),
    -- Enfant par lequel une famille est atteinte (le responsable n'est jamais invité « seul »).
    eleve_contexte_id UUID,
    reponse TEXT NOT NULL DEFAULT 'en_attente'
        CHECK (reponse IN ('en_attente', 'oui', 'non', 'peut_etre')),
    repondu_le TIMESTAMPTZ,
    motif_excuse TEXT CHECK (motif_excuse IS NULL OR length(motif_excuse) <= 300),
    ajoute_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_participant_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_participant_uq UNIQUE (etablissement_id, evenement_id, user_id),
    CONSTRAINT ent_agenda_participant_evenement_fk FOREIGN KEY (etablissement_id, evenement_id)
        REFERENCES ent_agenda_evenement (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_agenda_participant_eleve_fk FOREIGN KEY (etablissement_id, eleve_contexte_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE SET NULL (eleve_contexte_id)
);
CREATE INDEX IF NOT EXISTS ent_agenda_participant_user_idx
    ON ent_agenda_participant (etablissement_id, user_id, evenement_id);

-- Coupon-réponse : une question fermée à 2 ou 3 choix (M-355 du socle 8).
CREATE TABLE IF NOT EXISTS ent_agenda_coupon (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    question TEXT NOT NULL CHECK (length(btrim(question)) BETWEEN 1 AND 300),
    choix_1 TEXT NOT NULL CHECK (length(btrim(choix_1)) BETWEEN 1 AND 80),
    choix_2 TEXT NOT NULL CHECK (length(btrim(choix_2)) BETWEEN 1 AND 80),
    choix_3 TEXT CHECK (choix_3 IS NULL OR length(btrim(choix_3)) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_coupon_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_coupon_evenement_uq UNIQUE (etablissement_id, evenement_id),
    CONSTRAINT ent_agenda_coupon_evenement_fk FOREIGN KEY (etablissement_id, evenement_id)
        REFERENCES ent_agenda_evenement (etablissement_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ent_agenda_coupon_reponse (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    coupon_id UUID NOT NULL,
    user_id UUID NOT NULL,
    choix SMALLINT NOT NULL CHECK (choix BETWEEN 1 AND 3),
    repondu_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, coupon_id, user_id),
    CONSTRAINT ent_agenda_coupon_reponse_coupon_fk FOREIGN KEY (etablissement_id, coupon_id)
        REFERENCES ent_agenda_coupon (etablissement_id, id) ON DELETE CASCADE
);

-- Journal en ajout seul (R-AG3) : qui, quand, quoi. Texte figé, jamais relu comme vérité.
CREATE TABLE IF NOT EXISTS ent_agenda_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (objet IN ('evenement', 'serie', 'type')),
    objet_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 60),
    par_user_id UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    avant TEXT CHECK (avant IS NULL OR length(avant) <= 2000),
    apres TEXT CHECK (apres IS NULL OR length(apres) <= 2000),
    CONSTRAINT ent_agenda_journal_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_agenda_journal_objet_idx
    ON ent_agenda_journal (etablissement_id, objet, objet_id, le);
DROP TRIGGER IF EXISTS ent_agenda_journal_immuable ON ent_agenda_journal;
CREATE TRIGGER ent_agenda_journal_immuable
    BEFORE UPDATE OR DELETE ON ent_agenda_journal
    FOR EACH ROW EXECUTE FUNCTION ent_agenda_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_agenda_type_local', 'ent_agenda_serie', 'ent_agenda_serie_jour', 'ent_agenda_evenement',
    'ent_agenda_audience', 'ent_agenda_participant', 'ent_agenda_coupon',
    'ent_agenda_coupon_reponse', 'ent_agenda_journal'
]);
