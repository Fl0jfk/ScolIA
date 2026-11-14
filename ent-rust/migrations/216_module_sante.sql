-- 216 — Modules activables par établissement (socle 2, S2-11 : santé).
--
-- Règle produit : pas de données de santé tant que l'hébergement n'est pas certifié HDS.
-- Le module « sante » est donc DÉSACTIVÉ par défaut : l'absence de ligne vaut « désactivé »
-- (fermé par défaut), et seule une action explicite d'un administrateur l'active. Le refus
-- est appliqué côté serveur (middleware d'ent-api), pas seulement dans l'interface.
--
-- Table de configuration sans donnée personnelle (un booléen par établissement et module) ;
-- elle est lue avant l'ouverture d'une transaction d'établissement, donc sans RLS.
CREATE TABLE IF NOT EXISTS ent_module_etablissement (
    etablissement_id UUID NOT NULL,
    module           TEXT NOT NULL CHECK (module IN ('sante')),
    actif            BOOLEAN NOT NULL DEFAULT false,
    modifie_par      UUID,
    modifie_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, module)
);
