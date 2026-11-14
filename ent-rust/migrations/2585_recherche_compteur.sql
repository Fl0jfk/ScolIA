-- 2585 — Socle 29, lot P5 : compteur de recherches (anti-énumération). Réf. : 29_PORTAILS.md §12.1, §22.4, S-2907.
--
-- Table VOLATILE : un compteur par personne et par jour, rien d'autre. **Aucun texte de requête n'est
-- jamais conservé** (R-PO19) : ni la recherche, ni un extrait, ni son empreinte. Purge à 30 jours,
-- vidée à la restauration (registre des tables). Pas de JSON ni de tableau (D-036).
--
-- * `nb`              : recherches du jour ;
-- * `nb_courtes`      : recherches de 2 caractères ou moins après normalisation — le motif « recherche
--                       systématique de lettres » (a, b, c…) se détecte à ce seul compteur ;
-- * `bloque_jusqu_a`  : blocage temporaire (plafond dépassé ou lettres) ;
-- * `debloque_soi_le` + `motif_deblocage` : déblocage personnel, une fois par jour, avec un motif de la
--                       liste fermée (POR-259) ; au-delà, seul `recherche.debloquer` (POR-196).
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_recherche_compteur (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    compte_id UUID NOT NULL REFERENCES ent_user (id),
    jour DATE NOT NULL,
    nb INTEGER NOT NULL DEFAULT 0 CHECK (nb >= 0),
    nb_courtes INTEGER NOT NULL DEFAULT 0 CHECK (nb_courtes >= 0),
    bloque_jusqu_a TIMESTAMPTZ,
    debloque_soi_le TIMESTAMPTZ,
    motif_deblocage TEXT CHECK (motif_deblocage IS NULL OR motif_deblocage IN ('rentree', 'inscriptions', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, compte_id, jour),
    CONSTRAINT ent_recherche_compteur_deblocage_chk CHECK ((debloque_soi_le IS NULL) = (motif_deblocage IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_recherche_compteur_bloques_idx
    ON ent_recherche_compteur (etablissement_id, bloque_jusqu_a) WHERE bloque_jusqu_a IS NOT NULL;

SELECT ent_rls_poser(ARRAY['ent_recherche_compteur']);
