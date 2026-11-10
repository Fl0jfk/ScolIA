-- D-036 — `ent_inscription_dossier.resolutions` (JSONB, « trace d'audit » des choix humains de la
-- validation) devient une table typée : une ligne pour le choix sur l'élève, une par responsable
-- déclaré. Choix de rapprochement, de foyer et d'élève existant sont des colonnes contraintes.
--
-- Reprise idempotente : seuls les dossiers SANS ligne de résolution sont repris ; le code
-- n'écrit plus dans l'ancienne colonne (NULL), conservée en lecture seule puis retirée dans un lot
-- ultérieur (lot D-036 « retrait des colonnes reprises »).

CREATE TABLE IF NOT EXISTS ent_inscription_resolution (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL,
    cible TEXT NOT NULL CHECK (cible IN ('eleve', 'responsable')),
    -- Responsable déclaré concerné (cible = 'responsable' seulement).
    responsable_id UUID REFERENCES ent_inscription_responsable (id) ON DELETE SET NULL,
    -- 'nouveau' : nouvelle fiche ; 'existant' : fiche (élève ou personne) existante.
    choix TEXT NOT NULL CHECK (choix IN ('nouveau', 'existant')),
    eleve_id UUID,
    confirme_responsables_existants BOOLEAN,
    personne_id UUID,
    foyer_choix TEXT CHECK (foyer_choix IS NULL OR foyer_choix IN ('nouveau', 'existant')),
    foyer_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_resolution_dossier_fk
        FOREIGN KEY (etablissement_id, dossier_id)
        REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_inscription_resolution_forme_chk CHECK (
        (cible = 'eleve'
            AND responsable_id IS NULL AND personne_id IS NULL
            AND foyer_choix IS NULL AND foyer_id IS NULL
            AND (choix = 'existant') = (eleve_id IS NOT NULL)
            AND (choix = 'existant') = (confirme_responsables_existants IS NOT NULL))
        OR
        (cible = 'responsable'
            AND eleve_id IS NULL AND confirme_responsables_existants IS NULL
            AND (choix = 'existant') = (personne_id IS NOT NULL)
            AND (foyer_choix = 'existant') = (foyer_id IS NOT NULL))
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_resolution_eleve_uq
    ON ent_inscription_resolution (dossier_id) WHERE cible = 'eleve';
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_resolution_resp_uq
    ON ent_inscription_resolution (dossier_id, responsable_id) WHERE cible = 'responsable';
CREATE INDEX IF NOT EXISTS ent_inscription_resolution_dossier_idx
    ON ent_inscription_resolution (etablissement_id, dossier_id);

-- Reprise : choix sur l'élève.
INSERT INTO ent_inscription_resolution
    (etablissement_id, dossier_id, cible, choix, eleve_id, confirme_responsables_existants)
SELECT d.etablissement_id, d.id, 'eleve',
       CASE WHEN d.resolutions -> 'eleve' ->> 'choix' = 'existant' THEN 'existant' ELSE 'nouveau' END,
       CASE WHEN d.resolutions -> 'eleve' ->> 'choix' = 'existant'
                 AND d.resolutions -> 'eleve' ->> 'eleve_id' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
            THEN (d.resolutions -> 'eleve' ->> 'eleve_id')::uuid END,
       CASE WHEN d.resolutions -> 'eleve' ->> 'choix' = 'existant'
            THEN coalesce((d.resolutions -> 'eleve' ->> 'confirme_responsables_existants')::boolean, false) END
  FROM ent_inscription_dossier d
 WHERE jsonb_typeof(d.resolutions) = 'object'
   AND d.resolutions -> 'eleve' ->> 'choix' IN ('nouveau', 'existant')
   -- « existant » sans identifiant exploitable : non repris (la contrainte de forme l'exigerait).
   AND (d.resolutions -> 'eleve' ->> 'choix' = 'nouveau'
        OR d.resolutions -> 'eleve' ->> 'eleve_id' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$')
   AND NOT EXISTS (SELECT 1 FROM ent_inscription_resolution r WHERE r.dossier_id = d.id AND r.cible = 'eleve')
ON CONFLICT DO NOTHING;

-- Reprise : choix sur chaque responsable (le responsable doit encore exister).
INSERT INTO ent_inscription_resolution
    (etablissement_id, dossier_id, cible, responsable_id, choix, personne_id, foyer_choix, foyer_id)
SELECT d.etablissement_id, d.id, 'responsable', rs.id,
       CASE WHEN x.e -> 'personne' ->> 'choix' = 'existante' THEN 'existant' ELSE 'nouveau' END,
       CASE WHEN x.e -> 'personne' ->> 'choix' = 'existante' THEN (x.e -> 'personne' ->> 'personne_id')::uuid END,
       CASE x.e -> 'foyer' ->> 'choix' WHEN 'existant' THEN 'existant' WHEN 'nouveau' THEN 'nouveau' END,
       CASE WHEN x.e -> 'foyer' ->> 'choix' = 'existant' THEN (x.e -> 'foyer' ->> 'foyer_id')::uuid END
  FROM ent_inscription_dossier d
  CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(d.resolutions -> 'responsables') = 'array'
             THEN d.resolutions -> 'responsables' ELSE '[]'::jsonb END) AS x(e)
  JOIN ent_inscription_responsable rs
    ON rs.dossier_id = d.id
   AND x.e ->> 'responsable_id' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
   AND rs.id = (CASE WHEN x.e ->> 'responsable_id' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
                     THEN (x.e ->> 'responsable_id')::uuid END)
 WHERE jsonb_typeof(d.resolutions) = 'object'
   AND x.e -> 'personne' ->> 'choix' IN ('nouvelle', 'existante')
   AND (x.e -> 'personne' ->> 'choix' = 'nouvelle'
        OR x.e -> 'personne' ->> 'personne_id' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$')
   AND (coalesce(x.e -> 'foyer' ->> 'choix', '') <> 'existant'
        OR x.e -> 'foyer' ->> 'foyer_id' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$')
   AND NOT EXISTS (SELECT 1 FROM ent_inscription_resolution r WHERE r.dossier_id = d.id AND r.cible = 'responsable')
ON CONFLICT DO NOTHING;

-- RLS permissive (politique des migrations 100 et 480).
ALTER TABLE ent_inscription_resolution ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_inscription_resolution FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_inscription_resolution;
CREATE POLICY ent_rls_etablissement ON ent_inscription_resolution FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
