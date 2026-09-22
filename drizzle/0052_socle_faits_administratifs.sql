-- Faits administratifs du socle. Ajout uniquement. Aucun DELETE.
-- Comptabilité : nature d'avoir sur la facture + vue d'export. Pas de grand livre.
--> statement-breakpoint

ALTER TABLE "facture" ADD COLUMN IF NOT EXISTS "nature" text DEFAULT 'facture' NOT NULL;
--> statement-breakpoint

ALTER TABLE "facture" ADD COLUMN IF NOT EXISTS "facture_origine_id" uuid;
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "facture"
    ADD CONSTRAINT "facture_origine_id_facture_id_fk"
    FOREIGN KEY ("facture_origine_id") REFERENCES "public"."facture"("id")
    ON DELETE set null ON UPDATE no action;
EXCEPTION
  WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "facture" ADD CONSTRAINT "facture_nature_chk" CHECK ("nature" in ('facture', 'avoir'));
EXCEPTION
  WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint

ALTER TABLE "foyer_responsable" ADD COLUMN IF NOT EXISTS "peut_recuperer" boolean DEFAULT false NOT NULL;
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "passage" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid,
  "personnel_id" text,
  "invite_nom" text,
  "sens" text NOT NULL,
  "lieu" text NOT NULL,
  "horodatage" timestamp with time zone NOT NULL,
  "source" text DEFAULT 'manuel' NOT NULL,
  "annee_scolaire_id" uuid,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "passage_sujet_chk" CHECK (
    "eleve_id" is not null
    or "personnel_id" is not null
    or ("invite_nom" is not null and btrim("invite_nom") <> '')
  ),
  CONSTRAINT "passage_sens_chk" CHECK ("sens" in ('entree', 'sortie')),
  CONSTRAINT "passage_lieu_chk" CHECK ("lieu" in ('portail', 'self', 'internat', 'infirmerie', 'autre'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "infirmerie_passage" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "arrivee" timestamp with time zone NOT NULL,
  "sortie" timestamp with time zone,
  "motif_court" text DEFAULT '' NOT NULL,
  "signal_vie_scolaire" boolean DEFAULT true NOT NULL,
  "auteur_user_id" text,
  "auteur_nom" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "sante_extrait" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "portee" text NOT NULL,
  "libelle" text NOT NULL,
  "document_id" uuid,
  "actif" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "sante_extrait_portee_chk" CHECK ("portee" in ('cantine', 'eps', 'voyage', 'internat', 'periscolaire'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "conseil_seance" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "annee_scolaire_id" uuid,
  "periode_id" uuid,
  "classe" text NOT NULL,
  "date_seance" date NOT NULL,
  "statut" text DEFAULT 'preparee' NOT NULL,
  "pv" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "conseil_seance_statut_chk" CHECK ("statut" in ('preparee', 'tenue', 'close'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "conseil_avis" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "seance_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "decision" text,
  "mention" text,
  "appreciation" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "bulletin" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "periode_id" uuid NOT NULL,
  "annee_scolaire_id" uuid,
  "statut" text DEFAULT 'brouillon' NOT NULL,
  "appreciation_generale" text,
  "publie_at" timestamp with time zone,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "bulletin_statut_chk" CHECK ("statut" in ('brouillon', 'verrouille', 'publie'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "bulletin_ligne" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "bulletin_id" uuid NOT NULL,
  "matiere_id" uuid,
  "libelle" text NOT NULL,
  "appreciation" text,
  "moyenne_figee" numeric(5, 2),
  "ordre" integer DEFAULT 1 NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "cahier_texte" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "annee_scolaire_id" uuid,
  "date_seance" date NOT NULL,
  "classe" text,
  "groupe_id" uuid,
  "creneau_id" uuid,
  "matiere_libelle" text,
  "contenu" text DEFAULT '' NOT NULL,
  "travail" text DEFAULT '' NOT NULL,
  "a_rendre_le" date,
  "enseignant_user_id" text,
  "enseignant_nom" text,
  "visible_famille" boolean DEFAULT true NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "annee_bascule" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "annee_source_id" uuid NOT NULL,
  "annee_cible_id" uuid NOT NULL,
  "statut" text DEFAULT 'preparee' NOT NULL,
  "valide_par_user_id" text,
  "valide_at" timestamp with time zone,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "annee_bascule_annees_chk" CHECK ("annee_source_id" <> "annee_cible_id"),
  CONSTRAINT "annee_bascule_statut_chk" CHECK ("statut" in ('preparee', 'validee', 'annulee'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "internat_batiment" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "label" text NOT NULL,
  "notes" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "internat_chambre" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "batiment_id" uuid NOT NULL,
  "label" text NOT NULL,
  "etage" text,
  "capacite" integer DEFAULT 2 NOT NULL,
  "aile" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "internat_chambre_capacite_chk" CHECK ("capacite" between 1 and 8)
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "internat_affectation" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "chambre_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "annee_scolaire_id" uuid,
  "date_debut" date NOT NULL,
  "date_fin" date,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "internat_appel" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "batiment_id" uuid,
  "date_appel" date NOT NULL,
  "statut" text DEFAULT 'ouverte' NOT NULL,
  "valide_at" timestamp with time zone,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "internat_appel_statut_chk" CHECK ("statut" in ('ouverte', 'validee'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "internat_appel_ligne" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "appel_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "marque" text DEFAULT 'present' NOT NULL,
  "note" text,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "internat_appel_ligne_marque_chk" CHECK ("marque" in ('present', 'absent', 'excuse', 'activite'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "internat_sortie" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "eleve_id" uuid NOT NULL,
  "date_debut" date NOT NULL,
  "date_fin" date NOT NULL,
  "motif" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "internat_sortie_dates_chk" CHECK ("date_fin" >= "date_debut")
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "paie_periode" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "label" text NOT NULL,
  "date_debut" date NOT NULL,
  "date_fin" date NOT NULL,
  "statut" text DEFAULT 'brouillon' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "paie_periode_statut_chk" CHECK ("statut" in ('brouillon', 'figee')),
  CONSTRAINT "paie_periode_dates_chk" CHECK ("date_fin" >= "date_debut")
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "paie_element" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "periode_id" uuid NOT NULL,
  "personnel_id" text NOT NULL,
  "nature" text NOT NULL,
  "libelle" text NOT NULL,
  "quantite" numeric(10, 2),
  "montant" numeric(12, 2),
  "absence_id" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "paie_element_nature_chk" CHECK ("nature" in ('absence', 'heure', 'prime', 'retenue'))
);
--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "export_academique" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "canal" text NOT NULL,
  "annee_scolaire_id" uuid,
  "periode_id" uuid,
  "statut" text DEFAULT 'brouillon' NOT NULL,
  "genere_at" timestamp with time zone,
  "envoye_at" timestamp with time zone,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL,
  CONSTRAINT "export_academique_canal_chk" CHECK ("canal" in ('lsu', 'lsl', 'siecle', 'sts')),
  CONSTRAINT "export_academique_statut_chk" CHECK ("statut" in ('brouillon', 'envoye'))
);
--> statement-breakpoint

DO $$ BEGIN
  ALTER TABLE "passage" ADD CONSTRAINT "passage_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "passage" ADD CONSTRAINT "passage_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "passage" ADD CONSTRAINT "passage_personnel_id_personnel_id_fk" FOREIGN KEY ("personnel_id") REFERENCES "public"."personnel"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "passage" ADD CONSTRAINT "passage_annee_scolaire_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_scolaire_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "infirmerie_passage" ADD CONSTRAINT "infirmerie_passage_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "infirmerie_passage" ADD CONSTRAINT "infirmerie_passage_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_extrait" ADD CONSTRAINT "sante_extrait_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_extrait" ADD CONSTRAINT "sante_extrait_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "sante_extrait" ADD CONSTRAINT "sante_extrait_document_id_eleve_document_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."eleve_document"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "conseil_seance" ADD CONSTRAINT "conseil_seance_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "conseil_seance" ADD CONSTRAINT "conseil_seance_annee_scolaire_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_scolaire_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "conseil_seance" ADD CONSTRAINT "conseil_seance_periode_id_note_periode_id_fk" FOREIGN KEY ("periode_id") REFERENCES "public"."note_periode"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "conseil_avis" ADD CONSTRAINT "conseil_avis_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "conseil_avis" ADD CONSTRAINT "conseil_avis_seance_id_conseil_seance_id_fk" FOREIGN KEY ("seance_id") REFERENCES "public"."conseil_seance"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "conseil_avis" ADD CONSTRAINT "conseil_avis_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin" ADD CONSTRAINT "bulletin_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin" ADD CONSTRAINT "bulletin_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin" ADD CONSTRAINT "bulletin_periode_id_note_periode_id_fk" FOREIGN KEY ("periode_id") REFERENCES "public"."note_periode"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin" ADD CONSTRAINT "bulletin_annee_scolaire_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_scolaire_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin_ligne" ADD CONSTRAINT "bulletin_ligne_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin_ligne" ADD CONSTRAINT "bulletin_ligne_bulletin_id_bulletin_id_fk" FOREIGN KEY ("bulletin_id") REFERENCES "public"."bulletin"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "bulletin_ligne" ADD CONSTRAINT "bulletin_ligne_matiere_id_note_matiere_id_fk" FOREIGN KEY ("matiere_id") REFERENCES "public"."note_matiere"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "cahier_texte" ADD CONSTRAINT "cahier_texte_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "cahier_texte" ADD CONSTRAINT "cahier_texte_annee_scolaire_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_scolaire_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "cahier_texte" ADD CONSTRAINT "cahier_texte_creneau_id_edt_creneau_id_fk" FOREIGN KEY ("creneau_id") REFERENCES "public"."edt_creneau"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "cahier_texte" ADD CONSTRAINT "cahier_texte_groupe_id_groupe_pedagogique_id_fk" FOREIGN KEY ("groupe_id") REFERENCES "public"."groupe_pedagogique"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "annee_bascule" ADD CONSTRAINT "annee_bascule_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "annee_bascule" ADD CONSTRAINT "annee_bascule_annee_source_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_source_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "annee_bascule" ADD CONSTRAINT "annee_bascule_annee_cible_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_cible_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_batiment" ADD CONSTRAINT "internat_batiment_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_chambre" ADD CONSTRAINT "internat_chambre_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_chambre" ADD CONSTRAINT "internat_chambre_batiment_id_internat_batiment_id_fk" FOREIGN KEY ("batiment_id") REFERENCES "public"."internat_batiment"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_affectation" ADD CONSTRAINT "internat_affectation_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_affectation" ADD CONSTRAINT "internat_affectation_chambre_id_internat_chambre_id_fk" FOREIGN KEY ("chambre_id") REFERENCES "public"."internat_chambre"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_affectation" ADD CONSTRAINT "internat_affectation_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_affectation" ADD CONSTRAINT "internat_affectation_annee_scolaire_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_scolaire_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_appel" ADD CONSTRAINT "internat_appel_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_appel" ADD CONSTRAINT "internat_appel_batiment_id_internat_batiment_id_fk" FOREIGN KEY ("batiment_id") REFERENCES "public"."internat_batiment"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_appel_ligne" ADD CONSTRAINT "internat_appel_ligne_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_appel_ligne" ADD CONSTRAINT "internat_appel_ligne_appel_id_internat_appel_id_fk" FOREIGN KEY ("appel_id") REFERENCES "public"."internat_appel"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_appel_ligne" ADD CONSTRAINT "internat_appel_ligne_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_sortie" ADD CONSTRAINT "internat_sortie_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "internat_sortie" ADD CONSTRAINT "internat_sortie_eleve_id_eleve_id_fk" FOREIGN KEY ("eleve_id") REFERENCES "public"."eleve"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "paie_periode" ADD CONSTRAINT "paie_periode_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "paie_element" ADD CONSTRAINT "paie_element_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "paie_element" ADD CONSTRAINT "paie_element_periode_id_paie_periode_id_fk" FOREIGN KEY ("periode_id") REFERENCES "public"."paie_periode"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "paie_element" ADD CONSTRAINT "paie_element_personnel_id_personnel_id_fk" FOREIGN KEY ("personnel_id") REFERENCES "public"."personnel"("id") ON DELETE restrict ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "paie_element" ADD CONSTRAINT "paie_element_absence_id_absence_id_fk" FOREIGN KEY ("absence_id") REFERENCES "public"."absence"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "export_academique" ADD CONSTRAINT "export_academique_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "export_academique" ADD CONSTRAINT "export_academique_annee_scolaire_id_annee_scolaire_id_fk" FOREIGN KEY ("annee_scolaire_id") REFERENCES "public"."annee_scolaire"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint
DO $$ BEGIN
  ALTER TABLE "export_academique" ADD CONSTRAINT "export_academique_periode_id_note_periode_id_fk" FOREIGN KEY ("periode_id") REFERENCES "public"."note_periode"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION WHEN duplicate_object THEN null; END $$;
--> statement-breakpoint

CREATE INDEX IF NOT EXISTS "passage_etab_horaire_idx" ON "passage" ("etablissement_id", "horodatage");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "passage_eleve_idx" ON "passage" ("etablissement_id", "eleve_id", "horodatage");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "passage_lieu_idx" ON "passage" ("etablissement_id", "lieu", "horodatage");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "infirmerie_passage_eleve_idx" ON "infirmerie_passage" ("etablissement_id", "eleve_id", "arrivee");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "infirmerie_passage_ouverts_idx" ON "infirmerie_passage" ("etablissement_id", "sortie");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_extrait_eleve_idx" ON "sante_extrait" ("etablissement_id", "eleve_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sante_extrait_portee_idx" ON "sante_extrait" ("etablissement_id", "portee");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "conseil_seance_classe_idx" ON "conseil_seance" ("etablissement_id", "classe", "date_seance");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "conseil_seance_uidx" ON "conseil_seance" ("etablissement_id", "classe", "date_seance");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "conseil_avis_eleve_uidx" ON "conseil_avis" ("etablissement_id", "seance_id", "eleve_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "conseil_avis_eleve_idx" ON "conseil_avis" ("etablissement_id", "eleve_id");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "bulletin_eleve_periode_uidx" ON "bulletin" ("etablissement_id", "eleve_id", "periode_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bulletin_statut_idx" ON "bulletin" ("etablissement_id", "statut");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bulletin_ligne_bulletin_idx" ON "bulletin_ligne" ("etablissement_id", "bulletin_id");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "bulletin_ligne_matiere_uidx" ON "bulletin_ligne" ("bulletin_id", "matiere_id") WHERE "matiere_id" is not null;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "cahier_texte_date_idx" ON "cahier_texte" ("etablissement_id", "date_seance");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "cahier_texte_classe_idx" ON "cahier_texte" ("etablissement_id", "classe", "date_seance");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "cahier_texte_creneau_idx" ON "cahier_texte" ("etablissement_id", "creneau_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "annee_bascule_etab_idx" ON "annee_bascule" ("etablissement_id", "statut");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "annee_bascule_one_preparee_uidx" ON "annee_bascule" ("etablissement_id") WHERE "statut" = 'preparee';
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "internat_batiment_label_uidx" ON "internat_batiment" ("etablissement_id", "label");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "internat_chambre_label_uidx" ON "internat_chambre" ("etablissement_id", "batiment_id", "label");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "internat_chambre_batiment_idx" ON "internat_chambre" ("etablissement_id", "batiment_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "internat_affectation_chambre_idx" ON "internat_affectation" ("etablissement_id", "chambre_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "internat_affectation_eleve_idx" ON "internat_affectation" ("etablissement_id", "eleve_id");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "internat_affectation_open_uidx" ON "internat_affectation" ("etablissement_id", "eleve_id") WHERE "date_fin" is null;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "internat_appel_date_idx" ON "internat_appel" ("etablissement_id", "date_appel");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "internat_appel_global_uidx" ON "internat_appel" ("etablissement_id", "date_appel") WHERE "batiment_id" is null;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "internat_appel_batiment_uidx" ON "internat_appel" ("etablissement_id", "date_appel", "batiment_id") WHERE "batiment_id" is not null;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "internat_appel_ligne_uidx" ON "internat_appel_ligne" ("etablissement_id", "appel_id", "eleve_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "internat_sortie_eleve_idx" ON "internat_sortie" ("etablissement_id", "eleve_id", "date_debut");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "paie_periode_label_uidx" ON "paie_periode" ("etablissement_id", "label");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "paie_element_periode_idx" ON "paie_element" ("etablissement_id", "periode_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "paie_element_personnel_idx" ON "paie_element" ("etablissement_id", "personnel_id");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "export_academique_canal_idx" ON "export_academique" ("etablissement_id", "canal", "statut");
--> statement-breakpoint

CREATE OR REPLACE VIEW "export_comptable_famille" AS
SELECT
  f.etablissement_id,
  f.date_emission AS date_piece,
  'facture'::text AS type_piece,
  f.nature,
  f.numero,
  f.foyer_id,
  f.total_ttc AS montant,
  f.statut
FROM facture f
WHERE f.statut <> 'brouillon'
UNION ALL
SELECT
  e.etablissement_id,
  e.date_encaissement,
  'encaissement'::text,
  'encaissement'::text,
  coalesce(e.reference, e.id::text),
  e.foyer_id,
  e.montant,
  'encaisse'::text
FROM encaissement e;
