-- Socle 4, lot A0 bis, A0-d (R-A40, ADM-82). Trois voies de confirmation d'un lien en attente :
--   * `invitation`         : jeton reçu par e-mail (socle 3) ;
--   * `personne_connectee` : la personne, connectée, confirme en un geste ;
--   * `guichet`            : le secrétariat confirme après vérification d'une pièce d'identité.
-- Pour le guichet : type de la pièce et auteur sont tracés (`decide_par`), JAMAIS la copie ni le
-- numéro de la pièce. Migration additive et rejouable.

ALTER TABLE ent_lien_attente ADD COLUMN IF NOT EXISTS confirmation_voie TEXT;
ALTER TABLE ent_lien_attente ADD COLUMN IF NOT EXISTS piece_identite_type TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_lien_attente_voie_chk') THEN
        ALTER TABLE ent_lien_attente ADD CONSTRAINT ent_lien_attente_voie_chk CHECK (
            (confirmation_voie IS NULL
                OR confirmation_voie IN ('invitation', 'personne_connectee', 'guichet'))
            AND (piece_identite_type IS NULL OR piece_identite_type IN
                ('carte_identite', 'passeport', 'titre_sejour', 'permis_conduire', 'livret_famille',
                 'autre'))
            -- Une confirmation au guichet porte toujours un type de pièce ; les autres voies, jamais.
            AND (COALESCE(confirmation_voie = 'guichet', false) = (piece_identite_type IS NOT NULL))
        );
    END IF;
END $$;
