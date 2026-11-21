-- Socle 2 (l'élève), lot E5 : garde « aucune source externe n'écrit sans proposition ».
--
-- Une version d'identité dont la source est externe (`import`, `famille`, `eleve`) ne peut
-- être insérée que si elle référence (`source_ref`) une proposition DÉCIDÉE (acceptée,
-- acceptée partiellement ou appliquée automatiquement) du même élève et du même
-- établissement. Les versions de création, de reprise et de fusion (écrites par les
-- déclencheurs et les commandes de cycle de vie) restent hors de cette règle.
--
-- La garde protège contre l'erreur de code et contre une route mal câblée ; elle ne
-- protège pas contre un compte applicatif propriétaire des tables (voir D3 de la revue
-- de sécurité du socle 1).

CREATE OR REPLACE FUNCTION ent_eleve_identite_source_externe_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.source IN ('import', 'famille', 'eleve')
       AND NEW.nature NOT IN ('creation', 'reprise', 'fusion') THEN
        IF NEW.source_ref IS NULL OR NOT EXISTS (
            SELECT 1 FROM ent_eleve_proposition p
             WHERE p.id = NEW.source_ref
               AND p.etablissement_id = NEW.etablissement_id
               AND p.eleve_id = NEW.eleve_id
               AND p.statut IN ('acceptee', 'acceptee_partiellement', 'appliquee_auto')
        ) THEN
            RAISE EXCEPTION 'identité : une source externe n''écrit que par une proposition décidée'
                USING ERRCODE = '42501';
        END IF;
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identite_source_externe ON ent_eleve_identite;
CREATE TRIGGER ent_eleve_identite_source_externe
    BEFORE INSERT ON ent_eleve_identite
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identite_source_externe_garde();
