-- Socle 28, lot R2 (MS4 c) — migration 2564 : exécutions de reprise, comptes, refus, lignes « à décider ».
-- Réf. : docs/socles/28_MISE_EN_SERVICE.md §15 (rep_execution, rep_execution_compte, rep_refus), §6.4 pt 6
-- (simulation par défaut), T0 §7.2 (invariant lues = reprises + archivées + ignorées + refusées + à décider).
--
-- Une EXÉCUTION est un passage de l'outil (simulation, répétition ou application), numéroté par
-- établissement. Ses COMPTES portent l'invariant par objet (table ou collection source) ; ses REFUS
-- listent ce que le décodage strict n'a pas compris (code, colonne, message SANS donnée personnelle) ;
-- ses lignes « À DÉCIDER » préparent la file humaine du lot R3. Aucun JSON (D-036) : le rapport machine
-- complet est un fichier de l'exécution, son empreinte est ici.
-- Migration additive et rejouable. RLS d'établissement posée en fin de fichier (`ent_rls_poser`, 580).

CREATE TABLE IF NOT EXISTS rep_execution (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    numero INTEGER NOT NULL CHECK (numero >= 1),
    source TEXT NOT NULL CHECK (source ~ '^[a-z][a-z_]{0,29}$'),
    format_version TEXT NOT NULL CHECK (length(format_version) BETWEEN 1 AND 60),
    mode TEXT NOT NULL CHECK (mode IN ('simulation', 'repetition', 'application')),
    cible TEXT NOT NULL CHECK (cible IN ('jetable', 'preprod', 'production')),
    etat TEXT NOT NULL DEFAULT 'en_cours' CHECK (etat IN ('en_cours', 'terminee', 'echec', 'annulee')),
    debut_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    fin_le TIMESTAMPTZ,
    duree_s INTEGER CHECK (duree_s IS NULL OR duree_s >= 0),
    operateur_user_id UUID,
    -- SHA-256 (hex) du rapport machine : deux simulations identiques ont la même empreinte (rejeu).
    rapport_empreinte TEXT CHECK (rapport_empreinte IS NULL OR length(rapport_empreinte) = 64),
    -- Date de purge de la zone de reprise de cette exécution (MSE-179).
    zone_videe_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rep_execution_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT rep_execution_numero_uq UNIQUE (etablissement_id, numero),
    CONSTRAINT rep_execution_fin_chk CHECK ((fin_le IS NULL) = (etat = 'en_cours'))
);

CREATE TABLE IF NOT EXISTS rep_execution_compte (
    execution_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Table source du Next.js, `collection:<nom>` ou `s3:<nature>`.
    objet TEXT NOT NULL CHECK (length(objet) BETWEEN 1 AND 120),
    lues BIGINT NOT NULL CHECK (lues >= 0),
    creees BIGINT NOT NULL CHECK (creees >= 0),
    mises_a_jour BIGINT NOT NULL CHECK (mises_a_jour >= 0),
    inchangees BIGINT NOT NULL CHECK (inchangees >= 0),
    archivees BIGINT NOT NULL CHECK (archivees >= 0),
    ignorees BIGINT NOT NULL CHECK (ignorees >= 0),
    refusees BIGINT NOT NULL CHECK (refusees >= 0),
    a_decider BIGINT NOT NULL CHECK (a_decider >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (execution_id, objet),
    -- L'invariant de T0 §7.2 est une contrainte de la base : un compte qui ne tombe pas juste est refusé.
    CONSTRAINT rep_execution_compte_invariant_chk CHECK (
        lues = creees + mises_a_jour + inchangees + archivees + ignorees + refusees + a_decider),
    CONSTRAINT rep_execution_compte_exec_cfk FOREIGN KEY (etablissement_id, execution_id)
        REFERENCES rep_execution (etablissement_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS rep_refus (
    id UUID PRIMARY KEY,
    execution_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    source_objet TEXT NOT NULL CHECK (length(source_objet) BETWEEN 1 AND 120),
    -- Identifiant technique sûr, ou NULL : jamais une valeur de donnée.
    source_id TEXT CHECK (source_id IS NULL OR length(source_id) <= 200),
    source_ligne BIGINT CHECK (source_ligne IS NULL OR source_ligne >= 0),
    regle_code TEXT NOT NULL CHECK (regle_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    colonne TEXT CHECK (colonne IS NULL OR length(colonne) <= 80),
    message TEXT NOT NULL CHECK (length(message) <= 500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rep_refus_exec_cfk FOREIGN KEY (etablissement_id, execution_id)
        REFERENCES rep_execution (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS rep_refus_exec_idx ON rep_refus (etablissement_id, execution_id, source_objet);

CREATE TABLE IF NOT EXISTS rep_execution_a_decider (
    execution_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (length(objet) BETWEEN 1 AND 120),
    source_id TEXT NOT NULL CHECK (length(source_id) BETWEEN 1 AND 400),
    cible_objet TEXT NOT NULL CHECK (length(cible_objet) BETWEEN 1 AND 60),
    cible_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (motif IN ('rapprochement_a_decider', 'modifie_depuis_reprise')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (execution_id, objet, source_id),
    CONSTRAINT rep_execution_a_decider_exec_cfk FOREIGN KEY (etablissement_id, execution_id)
        REFERENCES rep_execution (etablissement_id, id) ON DELETE CASCADE
);

SELECT ent_rls_poser(ARRAY[
        'rep_execution',
        'rep_execution_compte',
        'rep_refus',
        'rep_execution_a_decider'
    ]);
