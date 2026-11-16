-- 2252 — Socle 12, lot AS6. Réf. : 12_ASSIDUITE.md AS-152, CA-41.
-- « Faire l'appel à la place de l'intervenant » : la vie scolaire (`appel.faire_tout`) ouvre l'appel d'une
-- séance dont l'intervenant ne l'a pas fait. La substitution est TRACÉE (qui, pour quel intervenant prévu,
-- pour quelle raison codée) en ajout seul. L'appel lui-même garde son auteur (`ent_appel.auteur_user_id`) et
-- son validateur (`valide_par`) : la substitution dit pourquoi ce n'est pas l'intervenant.
-- Additive, rejouable (E8), sans colonne JSON ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_appel_substitution (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appel_id UUID NOT NULL,
    creneau_id UUID,
    jour DATE NOT NULL,
    auteur_user_id UUID NOT NULL,
    -- Intervenant prévu (titulaire ou remplaçant du jour) : identifiant seulement.
    intervenant_user_id UUID,
    -- Raison (liste fermée) : jamais de texte libre.
    raison_code TEXT NOT NULL CHECK (raison_code IN (
        'oubli', 'intervenant_injoignable', 'intervenant_absent', 'panne', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_appel_subst_etab_id_uq UNIQUE (etablissement_id, id),
    -- Une substitution par appel (idempotente : la même personne qui rouvre l'appel ne la double pas).
    CONSTRAINT ent_asd_appel_subst_appel_uq UNIQUE (etablissement_id, appel_id),
    CONSTRAINT ent_asd_appel_subst_appel_fk FOREIGN KEY (etablissement_id, appel_id)
        REFERENCES ent_appel (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_appel_subst_jour_idx ON ent_asd_appel_substitution (etablissement_id, jour);

DROP TRIGGER IF EXISTS ent_asd_appel_subst_ajout_seul ON ent_asd_appel_substitution;
CREATE TRIGGER ent_asd_appel_subst_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_appel_substitution
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_asd_appel_substitution']);
