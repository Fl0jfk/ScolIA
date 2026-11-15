-- Socle 18, lot FO1 — migration 2361 : objets d'accord, catalogue de référence et usages
-- (`docs/socles/18_FORMULAIRES.md` §6.2, §8.1, §27 ; règles R-FC1, R-FC6, R-FC21, R-FC46 ; actions FRM-100, FRM-101).
--
-- Additif, rejouable, sans JSON, sans date UTC. Plage 2360–2379 : 2361 objets et catalogue · 2362 versions et décisions ·
-- 2363 journal · 2369 reprise de 193 et de 269/275.
--
-- * `ent_acc_catalogue` : référence **globale** fournie par ScolIA (aucun établissement : hors RLS ; l'application ne
--   fait que la lire, elle évolue par migration) ; `ent_acc_catalogue_usage` : ses usages. Les textes de départ sont **provisoires** (R-FC72) : ils
--   doivent être relus (juriste, direction) avant tout usage réel.
-- * `ent_acc_objet` : un objet d'accord d'un établissement (nature, règle, droit du lien, validité). La nature ne change
--   jamais (R-FC21) ; une règle ne baisse jamais sous celle du catalogue (R-FC6, contrôlé par le code et rappelé ici).
-- * `ent_acc_usage` : à quoi sert l'accord (`fil_classe`, `site_public`, `participation`…) ; un usage « mission » n'a pas
--   besoin d'accord (verdict `non_requis`).

CREATE TABLE IF NOT EXISTS ent_acc_catalogue (
    code TEXT PRIMARY KEY CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    nature TEXT NOT NULL CHECK (nature IN ('information', 'collecte', 'autorisation', 'consentement')),
    regle_min TEXT NOT NULL CHECK (regle_min IN ('un', 'tous', 'eleve_seul', 'eleve_et_titulaires', 'soi')),
    avis_eleve_des_ans SMALLINT CHECK (avis_eleve_des_ans IS NULL OR avis_eleve_des_ans BETWEEN 6 AND 17),
    droit_lien_code TEXT CHECK (droit_lien_code IS NULL OR droit_lien_code IN (
        'signer_quotidien', 'signer_documents', 'repondre_formulaires', 'autoriser_sorties')),
    preuve_min TEXT NOT NULL CHECK (preuve_min IN ('S0', 'S1', 'S2')),
    validite TEXT NOT NULL CHECK (validite IN ('annee_scolaire', 'jusqu_a', 'evenement', 'scolarite', 'sans_terme')),
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne')),
    desaccord_objet TEXT CHECK (desaccord_objet IS NULL OR desaccord_objet IN ('image', 'sortie', 'general')),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    texte_depart TEXT NOT NULL CHECK (length(btrim(texte_depart)) BETWEEN 1 AND 10000),
    version_catalogue INTEGER NOT NULL DEFAULT 1 CHECK (version_catalogue >= 1),
    CONSTRAINT ent_acc_catalogue_soi_chk CHECK ((regle_min = 'soi') = (sujet_type = 'personne'))
);

CREATE TABLE IF NOT EXISTS ent_acc_catalogue_usage (
    catalogue_code TEXT NOT NULL REFERENCES ent_acc_catalogue (code) ON DELETE CASCADE,
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    mission BOOLEAN NOT NULL DEFAULT false,
    ordre SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (catalogue_code, code)
);

-- Catalogue prêt (§8.1). Idempotent : une ligne existante n'est jamais réécrite (la direction a pu l'activer).
INSERT INTO ent_acc_catalogue
    (code, nature, regle_min, avis_eleve_des_ans, droit_lien_code, preuve_min, validite, sujet_type, desaccord_objet, libelle, texte_depart)
VALUES
 ('droit_image', 'consentement', 'tous', 11, 'signer_documents', 'S1', 'annee_scolaire', 'eleve', 'image',
  'Droit à l''image',
  'Photographies et vidéos de votre enfant : vous choisissez, canal par canal, ce que l''établissement peut publier pendant l''année scolaire. Un refus n''a aucune conséquence sur la scolarité. Vous pouvez retirer votre accord à tout moment, aussi simplement que vous l''avez donné , l''avis de votre enfant est recueilli à partir de 11 ans et son refus l''emporte.'),
 ('voix', 'consentement', 'tous', 11, 'signer_documents', 'S1', 'annee_scolaire', 'eleve', 'image',
  'Voix et enregistrements sonores',
  'Enregistrements de la voix de votre enfant (classe, podcast, diffusion). Vous choisissez usage par usage , retrait possible à tout moment , l''avis de l''enfant est recueilli à partir de 11 ans.'),
 ('publication_productions', 'consentement', 'eleve_et_titulaires', 11, 'signer_documents', 'S1', 'annee_scolaire', 'eleve', NULL,
  'Publication des productions d''élèves',
  'Publication de travaux de votre enfant (textes, dessins, vidéos). Vous choisissez les canaux et le mode de signature (nom, prénom, initiale ou anonyme). L''élève co-signe , retrait possible à tout moment.'),
 ('mention_nominative', 'consentement', 'tous', 11, 'signer_documents', 'S1', 'annee_scolaire', 'eleve', NULL,
  'Mention nominative (félicitations, palmarès)',
  'Mention du nom de votre enfant dans une publication (félicitations, palmarès). Retrait possible à tout moment.'),
 ('enregistrement_visio', 'consentement', 'tous', 11, 'signer_documents', 'S1', 'annee_scolaire', 'eleve', NULL,
  'Enregistrement des cours en visioconférence',
  'Enregistrement d''un cours à distance auquel participe votre enfant, mis à la disposition de sa classe. Retrait possible à tout moment.'),
 ('sortie_facultative', 'autorisation', 'un', NULL, 'autoriser_sorties', 'S1', 'evenement', 'eleve', 'sortie',
  'Sortie facultative',
  'Autorisation de participer à une sortie facultative. Chaque sortie précise la date, le lieu, les horaires et l''encadrement. Sans réponse, votre enfant ne participe pas.'),
 ('voyage_france', 'autorisation', 'un', NULL, 'autoriser_sorties', 'S1', 'evenement', 'eleve', 'sortie',
  'Voyage scolaire en France',
  'Autorisation de participer à un voyage scolaire en France. Sans réponse, votre enfant ne participe pas.'),
 ('voyage_etranger', 'autorisation', 'tous', NULL, 'signer_documents', 'S1', 'evenement', 'eleve', 'sortie',
  'Voyage scolaire à l''étranger',
  'Autorisation de participer à un voyage scolaire à l''étranger. L''accord des deux titulaires de l''autorité parentale est demandé. L''autorisation de sortie du territoire est une pièce distincte.'),
 ('transport_vehicule_accompagnateur', 'autorisation', 'un', NULL, 'signer_quotidien', 'S1', 'evenement', 'eleve', NULL,
  'Transport dans le véhicule d''un accompagnateur',
  'Autorisation de transporter votre enfant dans le véhicule personnel d''un accompagnateur pour une sortie ou une activité.'),
 ('soins_urgence', 'autorisation', 'un', NULL, 'signer_quotidien', 'S1', 'annee_scolaire', 'eleve', NULL,
  'Transport et soins d''urgence',
  'Autorisation de faire transporter votre enfant et de lui faire donner les soins d''urgence. Un refus n''empêche jamais l''appel des secours : en cas d''urgence vitale, les services de secours interviennent. Ne saisissez ici aucune information de santé.'),
 ('activite_risque', 'autorisation', 'un', NULL, 'signer_quotidien', 'S1', 'annee_scolaire', 'eleve', NULL,
  'Activité à risque (escalade, natation hors programme…)',
  'Autorisation de participer aux activités physiques à risque organisées par l''établissement.'),
 ('sortie_seul', 'autorisation', 'un', NULL, 'autoriser_sorties', 'S1', 'annee_scolaire', 'eleve', 'sortie',
  'Régime de sortie de l''élève',
  'Autorisation de quitter seul l''établissement dans les cas prévus par le règlement (fin des cours, absence d''un professeur).'),
 ('liste_representants', 'consentement', 'soi', NULL, NULL, 'S1', 'sans_terme', 'personne', NULL,
  'Liste des parents de la classe',
  'Apparaître dans la liste des parents de la classe remise aux représentants des parents. Vous pouvez retirer cet accord à tout moment.'),
 ('coordonnees_associations', 'consentement', 'soi', NULL, NULL, 'S1', 'sans_terme', 'personne', NULL,
  'Coordonnées transmises aux associations de parents',
  'Transmission de vos coordonnées aux associations de parents d''élèves (art. D111-8). Refusé par défaut , retrait possible à tout moment.'),
 ('coordonnees_transport', 'consentement', 'soi', NULL, NULL, 'S1', 'annee_scolaire', 'personne', NULL,
  'Coordonnées transmises au transporteur',
  'Transmission de vos coordonnées au transporteur scolaire pour l''année scolaire.'),
 ('photo_personne_autorisee', 'consentement', 'soi', NULL, NULL, 'S1', 'sans_terme', 'personne', NULL,
  'Photo des personnes autorisées à récupérer l''enfant',
  'Conservation de votre photo pour que l''accueil vous reconnaisse lors de la remise de l''enfant. Retrait possible à tout moment.'),
 ('charte_numerique', 'information', 'eleve_et_titulaires', NULL, 'signer_quotidien', 'S0', 'scolarite', 'eleve', NULL,
  'Charte du numérique',
  'Prise de connaissance de la charte d''usage du numérique, par l''élève et par ses responsables. N''empêche jamais l''accès d''un parent à l''ENT.'),
 ('reglement_interieur', 'information', 'eleve_et_titulaires', NULL, 'signer_quotidien', 'S0', 'scolarite', 'eleve', NULL,
  'Règlement intérieur',
  'Prise de connaissance du règlement intérieur, par l''élève et par ses responsables.'),
 ('outils_ia_eleve', 'information', 'un', NULL, 'signer_quotidien', 'S0', 'annee_scolaire', 'eleve', NULL,
  'Outils d''intelligence artificielle pour l''élève',
  'Information sur l''usage d''outils d''assistance par intelligence artificielle par l''élève, avec possibilité de s''y opposer.')
ON CONFLICT (code) DO NOTHING;

INSERT INTO ent_acc_catalogue_usage (catalogue_code, code, libelle, mission, ordre)
VALUES
 ('droit_image', 'interne_personnels', 'Trombinoscope des personnels (mission)', true, 0),
 ('droit_image', 'fil_classe', 'Fil de la classe (ENT)', false, 1),
 ('droit_image', 'fil_etablissement', 'Fil de l''établissement (ENT)', false, 2),
 ('droit_image', 'lettre_information', 'Lettre d''information', false, 3),
 ('droit_image', 'affiche_imprimee', 'Affiche imprimée', false, 4),
 ('droit_image', 'ecran_hall', 'Écran du hall', false, 5),
 ('droit_image', 'journal_papier', 'Journal papier', false, 6),
 ('droit_image', 'site_public', 'Site public de l''établissement', false, 7),
 ('droit_image', 'reseaux_etablissement', 'Réseaux sociaux de l''établissement', false, 8),
 ('droit_image', 'presse', 'Presse', false, 9),
 ('droit_image', 'video_projet', 'Vidéo de projet', false, 10),
 ('droit_image', 'photo_classe_prestataire', 'Photo de classe par un photographe', false, 11),
 ('voix', 'classe', 'Dans la classe', false, 1),
 ('voix', 'podcast_etablissement', 'Podcast de l''établissement', false, 2),
 ('voix', 'diffusion_publique', 'Diffusion publique', false, 3),
 ('publication_productions', 'fil_classe', 'Fil de la classe (ENT)', false, 1),
 ('publication_productions', 'fil_etablissement', 'Fil de l''établissement (ENT)', false, 2),
 ('publication_productions', 'site_public', 'Site public de l''établissement', false, 3),
 ('publication_productions', 'journal_papier', 'Journal papier', false, 4),
 ('mention_nominative', 'fil_etablissement', 'Fil de l''établissement (ENT)', false, 1),
 ('mention_nominative', 'lettre', 'Lettre d''information', false, 2),
 ('mention_nominative', 'site_public', 'Site public de l''établissement', false, 3),
 ('mention_nominative', 'presse', 'Presse', false, 4),
 ('enregistrement_visio', 'cours_enregistre_classe', 'Cours enregistré, visible de la classe', false, 1),
 ('sortie_facultative', 'participation', 'Participation à la sortie', false, 1),
 ('voyage_france', 'participation', 'Participation au voyage', false, 1),
 ('voyage_etranger', 'participation', 'Participation au voyage', false, 1),
 ('transport_vehicule_accompagnateur', 'transport', 'Transport en véhicule d''accompagnateur', false, 1),
 ('soins_urgence', 'transport_urgence', 'Transport d''urgence', false, 1),
 ('soins_urgence', 'soins_urgence', 'Soins d''urgence', false, 2),
 ('activite_risque', 'participation', 'Participation à l''activité', false, 1),
 ('sortie_seul', 'sortie_libre', 'Sortie libre', false, 1),
 ('sortie_seul', 'sortie_fin_cours', 'Sortie en fin de cours', false, 2),
 ('sortie_seul', 'sortie_absence_prof', 'Sortie en cas d''absence de professeur', false, 3),
 ('liste_representants', 'liste_classe', 'Liste de la classe', false, 1),
 ('coordonnees_associations', 'liste_parents_association', 'Liste transmise aux associations de parents', false, 1),
 ('coordonnees_transport', 'transporteur', 'Transmission au transporteur', false, 1),
 ('photo_personne_autorisee', 'remise_accueil', 'Remise de l''enfant à l''accueil', false, 1),
 ('charte_numerique', 'prise_de_connaissance', 'Prise de connaissance', false, 1),
 ('reglement_interieur', 'prise_de_connaissance', 'Prise de connaissance', false, 1),
 ('outils_ia_eleve', 'assistant_eleve', 'Assistant pour l''élève', false, 1)
ON CONFLICT (catalogue_code, code) DO NOTHING;

-- Objets d'accord d'un établissement.
CREATE TABLE IF NOT EXISTS ent_acc_objet (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    nature TEXT NOT NULL CHECK (nature IN ('information', 'collecte', 'autorisation', 'consentement')),
    regle TEXT NOT NULL CHECK (regle IN ('un', 'tous', 'eleve_seul', 'eleve_et_titulaires', 'soi')),
    avis_eleve_des_ans SMALLINT CHECK (avis_eleve_des_ans IS NULL OR avis_eleve_des_ans BETWEEN 6 AND 17),
    droit_lien_code TEXT CHECK (droit_lien_code IS NULL OR droit_lien_code IN (
        'signer_quotidien', 'signer_documents', 'repondre_formulaires', 'autoriser_sorties')),
    preuve_min TEXT NOT NULL CHECK (preuve_min IN ('S0', 'S1', 'S2')),
    validite TEXT NOT NULL CHECK (validite IN ('annee_scolaire', 'jusqu_a', 'evenement', 'scolarite', 'sans_terme')),
    valide_jusqu_au DATE,
    evenement_fin TIMESTAMPTZ,
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne')),
    catalogue_code TEXT REFERENCES ent_acc_catalogue (code),
    desaccord_objet TEXT CHECK (desaccord_objet IS NULL OR desaccord_objet IN ('image', 'sortie', 'general')),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_acc_objet_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_acc_objet_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_acc_objet_soi_chk CHECK ((regle = 'soi') = (sujet_type = 'personne')),
    CONSTRAINT ent_acc_objet_validite_chk CHECK (validite <> 'jusqu_a' OR valide_jusqu_au IS NOT NULL),
    -- Une information ne se « refuse » pas et ne se retire pas : jamais de règle `tous` pour la seule prise de connaissance.
    CONSTRAINT ent_acc_objet_information_chk CHECK (nature <> 'information' OR regle IN ('un', 'eleve_seul', 'eleve_et_titulaires'))
);
CREATE INDEX IF NOT EXISTS ent_acc_objet_actif_idx ON ent_acc_objet (etablissement_id, actif, code);

-- Usages d'un objet. `mission` : l'usage relève de la mission de l'établissement, sans accord (R-FC1).
CREATE TABLE IF NOT EXISTS ent_acc_usage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    mission BOOLEAN NOT NULL DEFAULT false,
    ordre SMALLINT NOT NULL DEFAULT 0,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_acc_usage_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_acc_usage_code_uq UNIQUE (etablissement_id, objet_id, code),
    CONSTRAINT ent_acc_usage_objet_cfk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT
),

-- Nature, code et sujet d'un objet ne changent jamais (R-FC21) ; un objet ne se supprime pas (se désactive).
CREATE OR REPLACE FUNCTION ent_acc_objet_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_acc_objet : un objet ne se supprime pas (le désactiver)' USING ERRCODE = '42501';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.code IS DISTINCT FROM OLD.code
       OR NEW.nature IS DISTINCT FROM OLD.nature
       OR NEW.sujet_type IS DISTINCT FROM OLD.sujet_type
       OR NEW.catalogue_code IS DISTINCT FROM OLD.catalogue_code THEN
        RAISE EXCEPTION 'ent_acc_objet : code, nature, sujet et catalogue sont immuables' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_acc_objet_garde ON ent_acc_objet;
CREATE TRIGGER ent_acc_objet_garde BEFORE UPDATE OR DELETE ON ent_acc_objet
    FOR EACH ROW EXECUTE FUNCTION ent_acc_objet_garde();

SELECT ent_rls_poser(ARRAY['ent_acc_objet', 'ent_acc_usage']);
