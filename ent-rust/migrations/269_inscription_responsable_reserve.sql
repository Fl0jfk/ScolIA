-- Socle 4, lot A0 bis (A0-b, A0-c), côté formulaire d'inscription : le responsable déclaré dans
-- un dossier peut porter les champs réservés du contrat v1 (`ent_personne_reserve`, migration 266).
-- Une adresse à protéger n'est jamais stockée en clair dans `adresse` : elle est scellée dans
-- `adresse_protegee` (déclarée dans `champs_chiffres::CHAMPS_CHIFFRES`) et `adresse` reste vide.
-- `ne_pas_solliciter` empêche toute invitation / demande d'accord / relance dès le dépôt,
-- avant même que la personne existe. Migration additive et rejouable.

ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS ne_pas_solliciter BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS ne_pas_solliciter_ref TEXT;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS adresse_a_proteger BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS adresse_protegee TEXT;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS langue TEXT;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS besoin_interprete BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS organisme TEXT;
ALTER TABLE ent_inscription_responsable ADD COLUMN IF NOT EXISTS nature_code TEXT;
