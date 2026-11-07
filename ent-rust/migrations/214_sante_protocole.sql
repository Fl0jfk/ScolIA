-- 214 — Registre des protocoles d'urgence de l'établissement (SAN-08). Additif et
-- idempotent. Texte générique de l'établissement (conduite à tenir type), jamais
-- lié à un élève : ce n'est pas une donnée de santé personnelle.

CREATE TABLE IF NOT EXISTS ent_sante_protocole (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 120),
    categorie TEXT NOT NULL CHECK (categorie IN ('urgence', 'traitement', 'autre')),
    contenu TEXT NOT NULL CHECK (length(contenu) <= 8000),
    version INTEGER NOT NULL DEFAULT 1,
    actif BOOLEAN NOT NULL DEFAULT true,
    mis_a_jour_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sante_protocole_titre_uq UNIQUE (etablissement_id, titre)
);
