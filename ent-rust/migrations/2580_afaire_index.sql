-- 2580 — Socle 29, lot P2 : index des tâches (« à-faire unifié »). Réf. : 29_PORTAILS.md §8.1, §22.1.
--
-- L'index est une PROJECTION jetable : il ne possède aucune donnée métier, seulement qui doit faire
-- quoi, sur quel objet, pour quand, dans quel état. Il se reconstruit à partir des socles
-- propriétaires (rapprochement, §8.10) et se neutralise à la restauration (E9, registre des tables).
--
-- Exigences : E1 `etablissement_id NOT NULL` partout (tables filles comprises) ; E2 FK composites
-- `(etablissement_id, x_id)` ; E3 UUID applicatifs ; E6 `cree_le` / `modifie_le`, retrait daté plutôt
-- que `DELETE` ; E10/D-036 aucune colonne JSON ni tableau ; E13 RLS standard (`ent_rls_poser`).
-- R-PO5 : le titre est neutre (composé par le propriétaire depuis un gabarit typé), ≤ 120 caractères.
-- Migration additive et rejouable (rejouée à chaque démarrage).

CREATE TABLE IF NOT EXISTS ent_afaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Code déclaré par la brique propriétaire (`vs.justifier_absence`) ; contrôlé par le registre
    -- statique du code (une nature inconnue est refusée à l'ouverture).
    nature_code TEXT NOT NULL CHECK (length(nature_code) BETWEEN 3 AND 80),
    socle_proprietaire SMALLINT NOT NULL CHECK (socle_proprietaire BETWEEN 1 AND 99),
    objet_type TEXT NOT NULL CHECK (length(objet_type) BETWEEN 1 AND 60),
    -- Identifiant chez le propriétaire : pas de FK (objet d'une autre brique), le rapprochement
    -- répare les écarts.
    objet_id UUID NOT NULL,
    eleve_id UUID,
    titre_neutre TEXT NOT NULL CHECK (length(btrim(titre_neutre)) BETWEEN 1 AND 120),
    regroupement_cle TEXT CHECK (regroupement_cle IS NULL OR length(regroupement_cle) <= 120),
    priorite TEXT NOT NULL CHECK (priorite IN ('bloquante', 'due', 'a_faire', 'information')),
    echeance_le TIMESTAMPTZ,
    echeance_dure BOOLEAN NOT NULL DEFAULT false,
    -- Fin de la plage calme (R-PO31) quand la tâche s'ouvre pendant la plage : une famille ou un
    -- élève ne la voit pas avant.
    visible_a_partir_de TIMESTAMPTZ NOT NULL DEFAULT now(),
    sensibilite SMALLINT NOT NULL CHECK (sensibilite BETWEEN 0 AND 3),
    etat TEXT NOT NULL DEFAULT 'ouverte' CHECK (etat IN (
        'ouverte', 'prise', 'en_attente', 'faite', 'sans_objet', 'expiree', 'annulee')),
    annee_scolaire_id UUID,
    -- Ordre des faits du propriétaire (CPO-51) : un fait plus ancien n'écrase pas un plus récent.
    version_source BIGINT NOT NULL DEFAULT 0 CHECK (version_source >= 0),
    ouverte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    close_le TIMESTAMPTZ,
    -- Motif NEUTRE de clôture (code fermé) : jamais de texte libre.
    issue TEXT CHECK (issue IS NULL OR issue IN (
        'faite', 'sans_objet', 'expiree', 'annulee', 'restauration', 'deces', 'depart', 'droit_perdu')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_afaire_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_afaire_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_afaire_annee_fk FOREIGN KEY (annee_scolaire_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_afaire_close_chk CHECK (
        (etat IN ('faite', 'sans_objet', 'expiree', 'annulee')) = (close_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_afaire_etat_echeance_idx
    ON ent_afaire (etablissement_id, etat, echeance_le);
CREATE INDEX IF NOT EXISTS ent_afaire_nature_objet_idx
    ON ent_afaire (etablissement_id, nature_code, objet_id);
CREATE INDEX IF NOT EXISTS ent_afaire_eleve_ouvertes_idx
    ON ent_afaire (etablissement_id, eleve_id)
    WHERE etat IN ('ouverte', 'prise', 'en_attente');

-- À qui la tâche est due. `personne` = un responsable (D-020, avec ou sans compte) ; `compte` = un
-- membre du personnel nominatif (pas de fiche personne : un compte) ; `foyer` ; `file` (collective).
CREATE TABLE IF NOT EXISTS ent_afaire_destinataire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    afaire_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('personne', 'compte', 'foyer', 'file')),
    personne_id UUID,
    compte_id UUID REFERENCES ent_user (id),
    foyer_id UUID,
    file_code TEXT CHECK (file_code IS NULL OR length(file_code) BETWEEN 3 AND 80),
    -- Délégation (§8.9) : personne délégante.
    pour_le_compte_de UUID,
    role_dans_tache TEXT NOT NULL DEFAULT 'titulaire'
        CHECK (role_dans_tache IN ('titulaire', 'delegataire', 'escalade', 'repli')),
    -- Neutralité temporelle (R-PO38) : instant à partir duquel une tâche CLOSE par un autre
    -- n'apparaît plus chez ce destinataire (prochain point de rafraîchissement neutre). NULL =
    -- disparaît dès la clôture.
    masque_a_partir_de TIMESTAMPTZ,
    ajoute_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retire_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Clé textuelle de l'identité du destinataire (`p:{uuid}`, `c:{uuid}`, `f:{uuid}`, `q:{code}`).
    destinataire_cle TEXT NOT NULL CHECK (length(destinataire_cle) BETWEEN 3 AND 100),
    CONSTRAINT ent_afaire_destinataire_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_afaire_destinataire_afaire_fk FOREIGN KEY (etablissement_id, afaire_id)
        REFERENCES ent_afaire (etablissement_id, id),
    CONSTRAINT ent_afaire_destinataire_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_afaire_destinataire_foyer_fk FOREIGN KEY (etablissement_id, foyer_id)
        REFERENCES ent_foyer (etablissement_id, id),
    CONSTRAINT ent_afaire_destinataire_type_chk CHECK (
        (type = 'personne' AND personne_id IS NOT NULL AND compte_id IS NULL AND foyer_id IS NULL AND file_code IS NULL)
        OR (type = 'compte' AND compte_id IS NOT NULL AND personne_id IS NULL AND foyer_id IS NULL AND file_code IS NULL)
        OR (type = 'foyer' AND foyer_id IS NOT NULL AND personne_id IS NULL AND compte_id IS NULL AND file_code IS NULL)
        OR (type = 'file' AND file_code IS NOT NULL AND personne_id IS NULL AND compte_id IS NULL AND foyer_id IS NULL))
);
-- Un destinataire actif une seule fois par tâche.
CREATE UNIQUE INDEX IF NOT EXISTS ent_afaire_destinataire_actif_uq
    ON ent_afaire_destinataire (afaire_id, destinataire_cle) WHERE retire_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_afaire_destinataire_personne_idx
    ON ent_afaire_destinataire (etablissement_id, personne_id) WHERE retire_le IS NULL AND personne_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_afaire_destinataire_compte_idx
    ON ent_afaire_destinataire (etablissement_id, compte_id) WHERE retire_le IS NULL AND compte_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_afaire_destinataire_file_idx
    ON ent_afaire_destinataire (etablissement_id, file_code) WHERE retire_le IS NULL AND file_code IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_afaire_destinataire_afaire_idx
    ON ent_afaire_destinataire (etablissement_id, afaire_id);

-- Idempotence des ouvertures : l'outbox est rejouée, le rapprochement repasse. L'identité d'une
-- tâche est `(nature, objet, destinataire)` ; une identité ne produit qu'une seule tâche.
CREATE TABLE IF NOT EXISTS ent_afaire_cle (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL,
    objet_id UUID NOT NULL,
    destinataire_cle TEXT NOT NULL CHECK (length(destinataire_cle) BETWEEN 3 AND 100),
    afaire_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, nature_code, objet_id, destinataire_cle),
    CONSTRAINT ent_afaire_cle_afaire_fk FOREIGN KEY (etablissement_id, afaire_id)
        REFERENCES ent_afaire (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_afaire_cle_afaire_idx ON ent_afaire_cle (etablissement_id, afaire_id);

SELECT ent_rls_poser(ARRAY['ent_afaire', 'ent_afaire_destinataire', 'ent_afaire_cle']);
