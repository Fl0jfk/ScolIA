-- Socle 4, lot A1 (§8.8, R-A9/R-A10 en partie) : interdiction visant une personne (remise,
-- communication, visite, contact avec l'élève). Sous-ensemble de la table cible : la pièce
-- (`piece_document_id`), le rappel à 48 h et la propagation aux postes d'accueil relèvent du lot A5.
-- Le motif est SCELLÉ (AES-256-GCM, `champs_chiffres`) et n'est lisible qu'avec F4 ; la consigne
-- (ce que doit faire l'accueil) est lisible avec `remise.lire`. Jamais dans un événement.

CREATE TABLE IF NOT EXISTS ent_eleve_interdiction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    personne_id UUID,
    personne_libelle TEXT CHECK (personne_libelle IS NULL OR length(btrim(personne_libelle)) BETWEEN 1 AND 120),
    objet TEXT[] NOT NULL
        CHECK (cardinality(objet) >= 1
               AND objet <@ ARRAY['remise', 'communication', 'visite', 'contact_eleve']::text[]),
    fondement TEXT NOT NULL CHECK (fondement IN
        ('decision_justice', 'ordonnance_protection', 'decision_ase', 'demande_titulaire',
         'decision_etablissement')),
    motif_scelle TEXT,
    consigne TEXT NOT NULL CHECK (length(btrim(consigne)) BETWEEN 1 AND 300),
    du DATE NOT NULL,
    au DATE,
    decide_par UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    levee_par UUID,
    levee_le TIMESTAMPTZ,
    levee_motif TEXT CHECK (levee_motif IS NULL OR length(levee_motif) <= 300),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_interdiction_cible_chk CHECK (personne_id IS NOT NULL OR personne_libelle IS NOT NULL),
    CONSTRAINT ent_eleve_interdiction_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_eleve_interdiction_levee_chk CHECK ((levee_le IS NULL) = (levee_par IS NULL)),
    CONSTRAINT ent_eleve_interdiction_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_interdiction_eleve_idx
    ON ent_eleve_interdiction (etablissement_id, eleve_id) WHERE levee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_eleve_interdiction_personne_idx
    ON ent_eleve_interdiction (etablissement_id, personne_id) WHERE levee_le IS NULL;
