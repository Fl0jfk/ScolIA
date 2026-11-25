-- Socle 11, lot MB1 (plage 2220–2239) : registre d'idempotence générique (`Idempotency-Key`).
-- Réf. : docs/socles/11_APPLICATIONS_MOBILES.md §5.2 et §5.5 ; docs/socles/11_MOBILE.md (table 2227).
-- Aucune fonction créée ici (règle 38 : rien à fixer côté `search_path`) ; aucune colonne JSON ni binaire (D-036) :
-- le corps rejoué est un texte UTF-8 (réponses JSON des écritures, bornées à 256 Kio ; au-delà, l'opération est
-- conservée mais déclarée non rejouable).
--
-- Une ligne = une clé reçue sur une écriture authentifiée. Cloisonnée par établissement ET par session : la clé d'une
-- autre session ou d'une autre personne ne rejoue rien. Purgée à 24 h (`cree_le`). Neutralisée (vidée) à la
-- restauration et jamais exportée : une écriture non rejouée est réémise par l'appareil.

CREATE TABLE IF NOT EXISTS ent_idempotence_cle (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    session_id UUID NOT NULL,
    cle UUID NOT NULL,
    -- SHA-256 (hexadécimal) de « méthode, chemin avec requête, corps » : même clé + autre empreinte = conflit.
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    statut TEXT NOT NULL CHECK (statut IN ('en_cours', 'terminee')),
    statut_http SMALLINT CHECK (statut_http BETWEEN 200 AND 399),
    type_contenu TEXT CHECK (length(type_contenu) <= 120),
    emplacement TEXT CHECK (length(emplacement) <= 500),
    corps TEXT CHECK (length(corps) <= 262144),
    -- Faux : la réponse n'a pas pu être conservée (trop longue, non textuelle) ; un rejeu répond alors par un refus
    -- explicite plutôt que de réexécuter l'écriture.
    rejouable BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    termine_le TIMESTAMPTZ,
    PRIMARY KEY (etablissement_id, session_id, cle),
    CONSTRAINT ent_idempotence_cle_terminee_chk CHECK (
        (statut = 'en_cours' AND statut_http IS NULL AND termine_le IS NULL)
        OR (statut = 'terminee' AND statut_http IS NOT NULL AND termine_le IS NOT NULL)),
    CONSTRAINT ent_idempotence_cle_corps_chk CHECK (rejouable OR corps IS NULL)
);

CREATE INDEX IF NOT EXISTS ent_idempotence_cle_cree_idx ON ent_idempotence_cle (etablissement_id, cree_le);

SELECT ent_rls_poser(ARRAY['ent_idempotence_cle']);
