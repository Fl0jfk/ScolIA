-- Socle 2 (l'élève), lot E8 : pastilles (indicateurs) et synthèse de la fiche (§7.11, §14).
--
-- Additif, idempotent, rejoué à chaque démarrage. Numéro 299 (dernier de la plage 290-299).
--
-- * `ent_catalogue_indicateur` : le catalogue du LOGICIEL (commun à tous les établissements) ; chaque
--   domaine y déclare ses pastilles et l'action qui les ouvre. La source de vérité est la constante
--   `domain_eleve::CATALOGUE_INDICATEURS` ; un test garantit que cette table lui est égale.
-- * `ent_eleve_indicateur` : la PROJECTION des pastilles actuellement vraies d'un élève, maintenue
--   par le consommateur D0 de l'outbox (`e8.indicateurs`) à chaque événement qui les concerne, et
--   reconstructible à tout moment depuis les tables sources (`ent_eleve_indicateurs_recalculer`). Pas de
--   JSON (D-036) : une valeur de code et une valeur de date suffisent aux pastilles du catalogue.
-- * Une pastille n'est JAMAIS un droit : qui la voit est décidé par le service, selon l'action du
--   catalogue sur CET élève ; la RLS ne fait que cloisonner par établissement.

CREATE TABLE IF NOT EXISTS ent_catalogue_indicateur (
    code              TEXT PRIMARY KEY,
    domaine           TEXT NOT NULL,
    libelle           TEXT NOT NULL,
    icone             TEXT NOT NULL,
    gravite           TEXT NOT NULL CHECK (gravite IN ('alerte', 'attention', 'information')),
    actions_requises  TEXT[] NOT NULL CHECK (cardinality(actions_requises) >= 1),
    depend_du_temps   BOOLEAN NOT NULL
);

INSERT INTO ent_catalogue_indicateur (code, domaine, libelle, icone, gravite, actions_requises, depend_du_temps) VALUES
    ('interne', 'scolarite', 'Interne', 'lit', 'information', ARRAY['eleves.lire', 'internat.gerer']::TEXT[], true),
    ('pai_actif', 'sante', 'PAI en cours', 'croix', 'alerte', ARRAY['sante.lire']::TEXT[], true),
    ('sans_ine', 'identifiants', 'Sans INE', 'identifiant', 'attention', ARRAY['eleves.identifiants.lire']::TEXT[], false),
    ('ine_conteste', 'identifiants', 'INE contesté', 'identifiant', 'alerte', ARRAY['eleves.identifiants.lire']::TEXT[], false),
    ('confidentialite_renforcee', 'protection', 'Confidentialité renforcée', 'cadenas', 'alerte', ARRAY['eleves.confidentiel.gerer']::TEXT[], true),
    ('majeur', 'identite', 'Majeur', 'calendrier', 'information', ARRAY['eleves.lire']::TEXT[], true),
    ('en_mobilite', 'scolarite', 'En mobilité', 'avion', 'information', ARRAY['eleves.lire']::TEXT[], true),
    ('redoublant', 'scolarite', 'Redoublant', 'boucle', 'information', ARRAY['eleves.lire']::TEXT[], false)
ON CONFLICT (code) DO UPDATE SET
    domaine = EXCLUDED.domaine, libelle = EXCLUDED.libelle, icone = EXCLUDED.icone,
    gravite = EXCLUDED.gravite, actions_requises = EXCLUDED.actions_requises,
    depend_du_temps = EXCLUDED.depend_du_temps;

CREATE TABLE IF NOT EXISTS ent_eleve_indicateur (
    etablissement_id  UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id          UUID NOT NULL,
    code              TEXT NOT NULL REFERENCES ent_catalogue_indicateur (code),
    valeur_code       TEXT,
    valeur_date       DATE,
    du                DATE NOT NULL,
    calcule_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    evenement_id      UUID,
    PRIMARY KEY (etablissement_id, eleve_id, code),
    CONSTRAINT ent_eleve_indicateur_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_indicateur_code_idx ON ent_eleve_indicateur (etablissement_id, code);

-- PAI en cours (actif, commencé, non échu) d'un élève ou de tout l'établissement.
CREATE OR REPLACE FUNCTION ent_eleve_pai_courant(p_etab UUID, p_eleve UUID, p_jour DATE)
RETURNS TABLE (eleve_id UUID, date_debut DATE, date_fin DATE)
LANGUAGE sql STABLE AS $$
    SELECT p.eleve_id, p.date_debut, p.date_fin FROM ent_pai p
     WHERE p.etablissement_id = p_etab AND (p_eleve IS NULL OR p.eleve_id = p_eleve)
       AND p.actif AND p.date_debut <= p_jour AND (p.date_fin IS NULL OR p.date_fin >= p_jour)
$$;

-- Pastilles attendues, calculées depuis les tables sources, à un jour donné (le jour de
-- l'établissement par défaut), pour un élève ou pour tout l'établissement. UNE seule définition :
-- le consommateur, la reconstruction et la tâche quotidienne l'appellent toutes.
CREATE OR REPLACE FUNCTION ent_eleve_indicateurs_attendus(p_etab UUID, p_jour DATE, p_eleve UUID DEFAULT NULL)
RETURNS TABLE (eleve_id UUID, code TEXT, valeur_code TEXT, valeur_date DATE, du DATE)
LANGUAGE sql STABLE AS $$
    WITH j AS (SELECT coalesce(p_jour, ent_jour_etablissement(p_etab)) AS jour),
    e AS (
        SELECT e.id, e.regime, e.redoublant, e.date_naissance, e.date_naissance_precision,
               e.date_naissance_statut, e.statut
          FROM ent_eleve e
         WHERE e.etablissement_id = p_etab AND e.fusionne_dans IS NULL AND (p_eleve IS NULL OR e.id = p_eleve)
    ),
    scolarises AS (SELECT * FROM e WHERE e.statut IN ('scolarise', 'a_venir'))
    -- interne : régime « interne » (seul régime hébergé, R-IT1)
    SELECT s.id, 'interne'::text, s.regime, NULL::date,
           coalesce((SELECT p.date_debut FROM ent_eleve_periode p
                      WHERE p.etablissement_id = p_etab AND p.eleve_id = s.id AND p.kind = 'regime'
                        AND p.date_fin IS NULL), (SELECT jour FROM j))
      FROM scolarises s WHERE s.regime = 'interne'
    UNION ALL
    -- pai_actif : PAI actif et en cours (échéance = date de fin)
    SELECT s.id, 'pai_actif', NULL, p.date_fin, p.date_debut
      FROM scolarises s
      JOIN ent_eleve_pai_courant(p_etab, p_eleve, (SELECT jour FROM j)) p ON p.eleve_id = s.id
    UNION ALL
    -- sans_ine : aucun INE (déclaré, à vérifier, confirmé ou contesté) en vigueur
    SELECT s.id, 'sans_ine', NULL, NULL, (SELECT jour FROM j)
      FROM scolarises s
     WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                        WHERE i.etablissement_id = p_etab AND i.eleve_id = s.id AND i.systeme = 'INE'
                          AND i.valide_au IS NULL AND i.statut IN ('declare', 'a_verifier', 'confirme', 'conteste'))
    UNION ALL
    -- ine_conteste : un INE contesté en vigueur (souvent le signe d'un doublon)
    SELECT s.id, 'ine_conteste', NULL, NULL, (SELECT min(i.valide_du) FROM ent_eleve_identifiant i
                                              WHERE i.etablissement_id = p_etab AND i.eleve_id = s.id
                                                AND i.systeme = 'INE' AND i.valide_au IS NULL AND i.statut = 'conteste')
      FROM scolarises s
     WHERE EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                    WHERE i.etablissement_id = p_etab AND i.eleve_id = s.id AND i.systeme = 'INE'
                      AND i.valide_au IS NULL AND i.statut = 'conteste')
    UNION ALL
    -- confidentialite_renforcee : mesure en vigueur (échéance = date de fin prévue)
    SELECT e.id, 'confidentialite_renforcee', NULL, c.au, c.du
      FROM e
      JOIN LATERAL (SELECT c.du, c.au FROM ent_eleve_confidentialite c
                     WHERE c.etablissement_id = p_etab AND c.eleve_id = e.id AND c.leve_le IS NULL
                       AND c.du <= (SELECT jour FROM j) AND (c.au IS NULL OR c.au > (SELECT jour FROM j))
                     ORDER BY c.du DESC LIMIT 1) c ON true
    UNION ALL
    -- majeur : 18 ans révolus à la date de naissance retenue, jamais sur une date contestée ou imprécise
    SELECT s.id, 'majeur', NULL, (s.date_naissance + INTERVAL '18 years')::date, (s.date_naissance + INTERVAL '18 years')::date
      FROM scolarises s
     WHERE s.date_naissance IS NOT NULL AND coalesce(s.date_naissance_precision, 'jour') = 'jour'
       AND coalesce(s.date_naissance_statut, 'declaree') <> 'contestee'
       AND (s.date_naissance + INTERVAL '18 years')::date <= (SELECT jour FROM j)
    UNION ALL
    -- en_mobilite : période de mobilité en cours (échéance = retour prévu)
    SELECT s.id, 'en_mobilite', m.type, m.retour_prevu, m.du
      FROM scolarises s
      JOIN LATERAL (SELECT m.type, m.retour_prevu, m.du FROM ent_eleve_mobilite m
                     WHERE m.etablissement_id = p_etab AND m.eleve_id = s.id
                       AND m.du <= (SELECT jour FROM j) AND (m.au IS NULL OR m.au > (SELECT jour FROM j))
                     ORDER BY m.du DESC LIMIT 1) m ON true
    UNION ALL
    -- redoublant
    SELECT s.id, 'redoublant', NULL, NULL, (SELECT jour FROM j)
      FROM scolarises s WHERE s.redoublant
$$;

-- Réconcilie la projection avec les pastilles attendues, pour un élève (ou tout l'établissement si
-- `p_eleve` est NULL) et pour les codes demandés (tous si `p_codes` est NULL). Idempotente : ne
-- touche que ce qui change. Renvoie le nombre de lignes insérées, modifiées ou supprimées.
CREATE OR REPLACE FUNCTION ent_eleve_indicateurs_recalculer(
    p_etab UUID, p_eleve UUID, p_codes TEXT[] DEFAULT NULL, p_evenement UUID DEFAULT NULL, p_jour DATE DEFAULT NULL)
RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT;
BEGIN
    WITH att AS (
        SELECT a.eleve_id, a.code, a.valeur_code, a.valeur_date, a.du
          FROM ent_eleve_indicateurs_attendus(p_etab, p_jour, p_eleve) a
         WHERE p_codes IS NULL OR a.code = ANY (p_codes)
    ),
    del AS (
        DELETE FROM ent_eleve_indicateur i
         WHERE i.etablissement_id = p_etab AND (p_eleve IS NULL OR i.eleve_id = p_eleve)
           AND (p_codes IS NULL OR i.code = ANY (p_codes))
           AND NOT EXISTS (SELECT 1 FROM att a WHERE a.eleve_id = i.eleve_id AND a.code = i.code)
        RETURNING 1
    ),
    ups AS (
        INSERT INTO ent_eleve_indicateur AS i (etablissement_id, eleve_id, code, valeur_code, valeur_date, du, evenement_id)
        SELECT p_etab, a.eleve_id, a.code, a.valeur_code, a.valeur_date, a.du, p_evenement FROM att a
        ON CONFLICT (etablissement_id, eleve_id, code) DO UPDATE
           SET valeur_code = EXCLUDED.valeur_code, valeur_date = EXCLUDED.valeur_date, du = EXCLUDED.du,
               calcule_le = now(), evenement_id = coalesce(EXCLUDED.evenement_id, i.evenement_id)
         WHERE (i.valeur_code, i.valeur_date, i.du)
               IS DISTINCT FROM (EXCLUDED.valeur_code, EXCLUDED.valeur_date, EXCLUDED.du)
        RETURNING 1
    )
    SELECT (SELECT count(*) FROM del) + (SELECT count(*) FROM ups) INTO n;
    RETURN n;
END $$;

-- ───────────────────────── RLS d'établissement (M9) ─────────────────────────
-- Le catalogue est global (aucune donnée d'établissement) : pas de RLS ; la projection est cloisonnée.
DO $$
BEGIN
    ALTER TABLE ent_eleve_indicateur ENABLE ROW LEVEL SECURITY;
    ALTER TABLE ent_eleve_indicateur FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_indicateur;
    CREATE POLICY ent_rls_etablissement ON ent_eleve_indicateur FOR ALL
        USING (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
        WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
END $$;
