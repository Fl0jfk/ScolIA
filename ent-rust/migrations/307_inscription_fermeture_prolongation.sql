-- Lot I2 (socle 3, INS-79, INS-80, R-I6) — fermetures du secrétariat et prolongations d'échéance.
--
-- * Une **fermeture** (vacances d'août, grève postale…) suspend les délais des familles : toute
--   échéance qui tombe pendant la fermeture est reportée au jour de réouverture + N jours
--   (`report_jours`), les relances sont suspendues pendant la fermeture. Annulable (rien n'est
--   effacé).
-- * Une **prolongation** décale une échéance pour toute une campagne ou pour une sélection de
--   dossiers (panne, intempérie). Jamais rétroactive au détriment d'une famille : une échéance
--   ne recule jamais. Chaque prolongation est tracée (ancienne et nouvelle date, motif public
--   court, nombre de dossiers touchés).

CREATE TABLE IF NOT EXISTS ent_inscription_fermeture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    date_debut DATE NOT NULL,
    date_fin DATE NOT NULL,
    -- Jours après la réouverture avant lesquels une famille n'est pas relancée ni échue.
    report_jours SMALLINT NOT NULL DEFAULT 3 CHECK (report_jours BETWEEN 0 AND 30),
    motif_public TEXT NOT NULL CHECK (char_length(btrim(motif_public)) BETWEEN 3 AND 200),
    contact_urgence TEXT CHECK (contact_urgence IS NULL OR char_length(contact_urgence) <= 200),
    echeances_reportees INTEGER NOT NULL DEFAULT 0 CHECK (echeances_reportees >= 0),
    declaree_par UUID,
    declaree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    annulee_le TIMESTAMPTZ,
    annulee_par UUID,
    CONSTRAINT ent_inscription_fermeture_dates_chk CHECK (date_fin >= date_debut),
    CONSTRAINT ent_inscription_fermeture_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_inscription_fermeture_etab_idx
    ON ent_inscription_fermeture (etablissement_id, date_debut, date_fin) WHERE annulee_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_inscription_prolongation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (objet IN ('date_limite_famille', 'cloture', 'echeance_dossiers')),
    ancienne_date DATE,
    nouvelle_date DATE NOT NULL,
    motif_public TEXT NOT NULL CHECK (char_length(btrim(motif_public)) BETWEEN 3 AND 200),
    nb_dossiers INTEGER NOT NULL DEFAULT 0 CHECK (nb_dossiers >= 0),
    familles_prevenues INTEGER NOT NULL DEFAULT 0 CHECK (familles_prevenues >= 0),
    decide_par UUID,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_prolongation_fk
        FOREIGN KEY (etablissement_id, campagne_id)
        REFERENCES ent_inscription_campagne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_prolongation_campagne_idx
    ON ent_inscription_prolongation (etablissement_id, campagne_id, decide_le);
