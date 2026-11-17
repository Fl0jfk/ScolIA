-- Socle 32, lot ET0 — migration 2640 : quotas et réservations d'envoi (`32_BUREAUTIQUE.md` §15 point 4, §24, R-ET20, R-ET21).
--
-- Trois tables, toutes par établissement (E1), RLS standard (E13), sans JSON, sans tableau, sans BYTEA (D-036) :
--   * `ent_espace_quota_profil`  : réglage d'établissement des quotas par profil (bornes ScolIA : 1 Mio à 1 Tio ; les valeurs
--     par défaut vivent dans le code tant qu'une ligne n'existe pas, ET9 les règle) ;
--   * `ent_espace_occupation`    : compteur **verrouillé** par espace (`SELECT … FOR UPDATE`) : octets confirmés (versions
--     conservées, corbeille comprise) et octets réservés (envois annoncés, pas encore confirmés) ;
--   * `ent_espace_reservation`   : un envoi annoncé = une réservation de quota (taille **signée** dans l'URL d'envoi),
--     confirmée après réconciliation de la taille et de l'empreinte réelles, ou libérée (expiration à 15 minutes,
--     taille différente, refus, annulation). Table « volatile » (E9) : neutralisée à la restauration.
-- La clé étrangère vers `ent_espace` est posée par la migration 2641 (la table n'existe pas encore ici).
-- Additif et rejouable (E8).

CREATE TABLE IF NOT EXISTS ent_espace_quota_profil (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    profil TEXT NOT NULL CHECK (profil IN
        ('personnel', 'eleve', 'espace_cours', 'espace_equipe', 'espace_service', 'espace_libre')),
    octets BIGINT NOT NULL CHECK (octets BETWEEN 1048576 AND 1099511627776),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, profil)
);

CREATE TABLE IF NOT EXISTS ent_espace_occupation (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    octets_confirmes BIGINT NOT NULL DEFAULT 0 CHECK (octets_confirmes >= 0),
    octets_reserves BIGINT NOT NULL DEFAULT 0 CHECK (octets_reserves >= 0),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, espace_id)
);

CREATE TABLE IF NOT EXISTS ent_espace_reservation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    -- Taille annoncée = taille signée dans l'URL d'envoi : jamais plus de 64 Mio (plafond d'`ent-object`).
    octets BIGINT NOT NULL CHECK (octets > 0 AND octets <= 67108864),
    -- Clé opaque `{etablissement}/espaces/{uuid}` : aucun nom de fichier ni donnée personnelle (E14).
    cle_objet TEXT NOT NULL,
    demandeur_user_id UUID NOT NULL,
    expire_le TIMESTAMPTZ NOT NULL,
    confirmee_le TIMESTAMPTZ,
    liberee_le TIMESTAMPTZ,
    liberee_motif TEXT CHECK (liberee_motif IS NULL OR liberee_motif IN
        ('expiree', 'taille_differente', 'refusee', 'annulee', 'restauration')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_reservation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_reservation_cle_uq UNIQUE (etablissement_id, cle_objet),
    CONSTRAINT ent_espace_reservation_cle_etab_chk CHECK (cle_objet LIKE etablissement_id::text || '/%'),
    CONSTRAINT ent_espace_reservation_etat_chk CHECK (
        NOT (confirmee_le IS NOT NULL AND liberee_le IS NOT NULL)
        AND ((liberee_le IS NULL) = (liberee_motif IS NULL)))
);
CREATE INDEX IF NOT EXISTS ent_espace_reservation_ouvertes_idx
    ON ent_espace_reservation (etablissement_id, expire_le)
    WHERE confirmee_le IS NULL AND liberee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_espace_reservation_espace_idx
    ON ent_espace_reservation (etablissement_id, espace_id);

-- Index unique déjà posé par la migration 2402 (même nom) : repris ici pour ne pas dépendre de l'ordre.
CREATE UNIQUE INDEX IF NOT EXISTS ent_objet_etab_id_cdt_uq ON ent_objet (etablissement_id, id);

SELECT ent_rls_poser(ARRAY['ent_espace_quota_profil', 'ent_espace_occupation', 'ent_espace_reservation']);
