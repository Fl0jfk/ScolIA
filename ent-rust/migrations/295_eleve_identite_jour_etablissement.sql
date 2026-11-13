-- Correctif : la version courante d'identité se choisit au **jour de l'établissement**
-- (fuseau, Europe/Paris par défaut — migration 265), comme le code Rust (`jour_paris`).
-- Le déclencheur de 280 prenait `current_date`, soit le jour du serveur (UTC) : entre minuit
-- et 1 h (2 h l'été) à Paris, une version datée du jour n'était pas projetée sur la fiche
-- (filtre `effet_du <= jour UTC` = la veille). Idempotent.
CREATE OR REPLACE FUNCTION ent_eleve_identite_apres_insert() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE courante UUID;
BEGIN
    courante := ent_eleve_identite_courante(NEW.etablissement_id, NEW.eleve_id,
                                            ent_aujourdhui_etab(NEW.etablissement_id, now()));
    IF courante = NEW.id THEN
        PERFORM ent_eleve_identite_projeter(NEW);
    END IF;
    RETURN NULL;
END $$;
