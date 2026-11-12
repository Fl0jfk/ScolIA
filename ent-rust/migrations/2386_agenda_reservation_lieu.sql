-- 2386 — Socle 19, lot AG3 : réservation de lieux (salles du référentiel du socle 5, salles de classe
-- comprises, D-058). Réf. : 19_AGENDA_RDV.md §14, §19.7, R-AG3/R-AG18/R-AG19 ; ND-01 (double réservation
-- en concurrence : contrainte d'EXCLUSION en base), ND-07/NJ-62 (lecture bornée et minimisée), ND-08/NJ-76
-- (propriété par compte, jamais par e-mail).
--
-- * Une réservation `demandee` ou `confirmee` BLOQUE la plage : deux réservations qui se chevauchent
--   dans la même salle sont impossibles (EXCLUDE USING gist), quel que soit le chemin d'écriture.
-- * `commentaire` n'est lisible que du demandeur, du bénéficiaire et du gestionnaire (R-AG? §14.5).
-- * Le fil des faits est en AJOUT SEUL (`ent_agenda_ajout_seul`, migration 2380).
-- * Aucune colonne JSON (D-036). Rejouable.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- Règle par salle (D-058) : réservable toujours, sur validation, ou jamais. Sans ligne : « toujours »
-- (l'emploi du temps réel et les autres réservations empêchent déjà les conflits).
CREATE TABLE IF NOT EXISTS ent_agenda_lieu_regle (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    salle_id UUID NOT NULL,
    mode TEXT NOT NULL DEFAULT 'toujours' CHECK (mode IN ('toujours', 'sur_validation', 'jamais')),
    valideur_user_id UUID,
    delai_min_heures INTEGER NOT NULL DEFAULT 0 CHECK (delai_min_heures BETWEEN 0 AND 720),
    duree_max_minutes INTEGER NOT NULL DEFAULT 600 CHECK (duree_max_minutes BETWEEN 15 AND 1440),
    horizon_jours INTEGER NOT NULL DEFAULT 56 CHECK (horizon_jours BETWEEN 1 AND 400),
    modifie_par_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, salle_id),
    CONSTRAINT ent_agenda_lieu_regle_salle_fk FOREIGN KEY (etablissement_id, salle_id)
        REFERENCES ent_salle (etablissement_id, id),
    CONSTRAINT ent_agenda_lieu_regle_valideur_chk CHECK (mode <> 'sur_validation' OR valideur_user_id IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS ent_agenda_reservation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    salle_id UUID NOT NULL,
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    demandeur_user_id UUID NOT NULL,
    -- Bénéficiaire quand le demandeur réserve pour un collègue ; NULL = le demandeur lui-même.
    pour_user_id UUID,
    intitule TEXT NOT NULL CHECK (length(btrim(intitule)) BETWEEN 1 AND 200),
    motif_categorie TEXT NOT NULL DEFAULT 'autre'
        CHECK (motif_categorie IN ('enseignement', 'reunion', 'evenement', 'autre')),
    effectif INTEGER CHECK (effectif IS NULL OR effectif BETWEEN 1 AND 2000),
    mise_en_place TEXT CHECK (mise_en_place IS NULL OR length(mise_en_place) <= 500),
    commentaire TEXT CHECK (commentaire IS NULL OR length(commentaire) <= 1000),
    evenement_id UUID,
    serie_id UUID,
    statut TEXT NOT NULL DEFAULT 'confirmee'
        CHECK (statut IN ('demandee', 'confirmee', 'refusee', 'annulee', 'tombee')),
    decide_par_user_id UUID,
    decide_le TIMESTAMPTZ,
    motif_decision TEXT CHECK (motif_decision IS NULL OR length(motif_decision) <= 300),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_agenda_reservation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_reservation_plage_chk CHECK (fin > debut AND fin <= debut + interval '24 hours'),
    CONSTRAINT ent_agenda_reservation_salle_fk FOREIGN KEY (etablissement_id, salle_id)
        REFERENCES ent_salle (etablissement_id, id),
    CONSTRAINT ent_agenda_reservation_evenement_fk FOREIGN KEY (etablissement_id, evenement_id)
        REFERENCES ent_agenda_evenement (etablissement_id, id),
    CONSTRAINT ent_agenda_reservation_serie_fk FOREIGN KEY (etablissement_id, serie_id)
        REFERENCES ent_agenda_serie (etablissement_id, id),
    CONSTRAINT ent_agenda_reservation_decision_chk CHECK (
        statut NOT IN ('refusee', 'tombee') OR length(btrim(coalesce(motif_decision, ''))) > 0),
    -- ND-01 : aucune double réservation, même en concurrence.
    CONSTRAINT ent_agenda_reservation_pas_de_chevauchement EXCLUDE USING gist (
        etablissement_id WITH =, salle_id WITH =, tstzrange(debut, fin) WITH &&)
        WHERE (statut IN ('demandee', 'confirmee'))
);
CREATE INDEX IF NOT EXISTS ent_agenda_reservation_salle_idx
    ON ent_agenda_reservation (etablissement_id, salle_id, debut);
CREATE INDEX IF NOT EXISTS ent_agenda_reservation_demandeur_idx
    ON ent_agenda_reservation (etablissement_id, demandeur_user_id, debut);

CREATE TABLE IF NOT EXISTS ent_agenda_reservation_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    reservation_id UUID NOT NULL,
    fait TEXT NOT NULL CHECK (fait IN (
        'demande', 'confirmee', 'refusee', 'annulee_demandeur', 'annulee_gestionnaire', 'tombee')),
    par_user_id UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    motif_public TEXT CHECK (motif_public IS NULL OR length(motif_public) <= 300),
    CONSTRAINT ent_agenda_reservation_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_agenda_reservation_fait_reservation_fk FOREIGN KEY (etablissement_id, reservation_id)
        REFERENCES ent_agenda_reservation (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_agenda_reservation_fait_idx
    ON ent_agenda_reservation_fait (etablissement_id, reservation_id, le);
DROP TRIGGER IF EXISTS ent_agenda_reservation_fait_immuable ON ent_agenda_reservation_fait;
CREATE TRIGGER ent_agenda_reservation_fait_immuable
    BEFORE UPDATE OR DELETE ON ent_agenda_reservation_fait
    FOR EACH ROW EXECUTE FUNCTION ent_agenda_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_agenda_lieu_regle', 'ent_agenda_reservation', 'ent_agenda_reservation_fait'
]);
