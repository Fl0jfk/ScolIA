-- Recette personas 1 (RP1-11), D-014 : séparation des foyers dans la messagerie.
-- `prive_user_id` : fil ouvert par un responsable, privé entre lui et l'établissement.
-- `foyer_id` : fil adressé aux payeurs d'un foyer (relance de facture).
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS prive_user_id UUID REFERENCES ent_user (id);
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS foyer_id UUID REFERENCES ent_foyer (id);

UPDATE ent_msg_fil f
   SET prive_user_id = f.created_by_user_id
 WHERE f.prive_user_id IS NULL
   AND EXISTS (SELECT 1 FROM ent_membership m
                WHERE m.user_id = f.created_by_user_id
                  AND m.etablissement_id = f.etablissement_id
                  AND m.role = 'parent');

UPDATE ent_msg_fil f
   SET foyer_id = r.foyer_id
  FROM ent_fac_rappel r
 WHERE r.fil_id = f.id AND f.foyer_id IS NULL;
