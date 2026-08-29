import {
  boolean,
  date,
  index,
  integer,
  jsonb,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

/**
 * Fiches de dialogue (orientation année suivante).
 * Campagnes configurables : trimestre / semestre, étapes modulaires.
 * Pas d’import depuis schema.ts (évite cycle) — FKs eleve / annee renforcées en SQL.
 */

export type FdCalendrierMode = "trimestre" | "semestre" | "personnalise";

export type FdEtapeKind =
  | "saisie_famille"
  | "conseil"
  | "choix_definitifs"
  | "decision_finale_conseil"
  | "acceptation_famille"
  | "appel";

export type FdCampagneStatut = "brouillon" | "active" | "cloturee" | "archivee";

export type FdFicheStatut =
  | "a_envoyer"
  | "en_attente_famille"
  | "saisie_recue"
  | "en_conseil"
  | "decision_envoyee"
  | "en_attente_acceptation"
  | "acceptee"
  | "refusee"
  | "en_appel"
  | "cloturee"
  | "figee";

export type FdCatalogueChoix = {
  destinations: Array<{ id: string; label: string; niveauCible?: string }>;
  options: Array<{
    id: string;
    label: string;
    kind: "lv" | "option_interne" | "specialite" | "autre";
  }>;
  /** Champs du formulaire famille (ordre d’affichage). */
  fields: Array<{
    id: string;
    type: "select" | "multiselect" | "text" | "textarea" | "checkbox";
    label: string;
    required?: boolean;
    /** Référence destinations | options | liste inline */
    optionsFrom?: "destinations" | "options";
    inlineOptions?: Array<{ id: string; label: string }>;
    helpText?: string;
  }>;
};

export type FdReponsePayload = {
  values: Record<string, string | string[] | boolean | null>;
  comment?: string;
  /** Si la famille force malgré un avis conseil précédent */
  forceMalgreAvis?: boolean;
};

export type FdConseilDecisionPayload = {
  avis: "favorable" | "reserve" | "defavorable" | "autre";
  destinationProposee?: string;
  optionsProposees?: string[];
  motif?: string;
  commentaire?: string;
};

export type FdAcceptationPayload = {
  accepte: boolean;
  motifRefus?: string;
};

export type FdAppelConfig = {
  enabled: boolean;
  dateLimite?: string;
  procedureHtml?: string;
  documentsLabels?: string[];
};

export const fdCampagne = pgTable(
  "fd_campagne",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    label: text("label").notNull(),
    anneeLabel: text("annee_label").notNull(),
    siteKey: text("site_key"),
    calendrierMode: text("calendrier_mode").$type<FdCalendrierMode>().notNull().default("trimestre"),
    templateKey: text("template_key"),
    statut: text("statut").$type<FdCampagneStatut>().notNull().default("brouillon"),
    catalogue: jsonb("catalogue").$type<FdCatalogueChoix>().notNull().default({
      destinations: [],
      options: [],
      fields: [],
    }),
    appelConfig: jsonb("appel_config").$type<FdAppelConfig>().notNull().default({
      enabled: true,
    }),
    delaiFamilleJours: integer("delai_famille_jours").notNull().default(7),
    classesCibles: jsonb("classes_cibles").$type<string[]>().notNull().default([]),
    createdByUserId: text("created_by_user_id"),
    openedAt: timestamp("opened_at", { withTimezone: true }),
    closedAt: timestamp("closed_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("fd_campagne_etablissement_idx").on(t.etablissementId),
    index("fd_campagne_annee_idx").on(t.etablissementId, t.anneeLabel),
    index("fd_campagne_statut_idx").on(t.etablissementId, t.statut),
  ],
);

export const fdEtape = pgTable(
  "fd_etape",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    campagneId: uuid("campagne_id")
      .notNull()
      .references(() => fdCampagne.id, { onDelete: "cascade" }),
    ordre: integer("ordre").notNull(),
    kind: text("kind").$type<FdEtapeKind>().notNull(),
    label: text("label").notNull(),
    description: text("description"),
    /** Ouverture / fermeture (saisie famille) ou date de conseil (gel). */
    opensAt: timestamp("opens_at", { withTimezone: true }),
    closesAt: timestamp("closes_at", { withTimezone: true }),
    conseilDate: date("conseil_date"),
    /** true = étape optionnelle (ex. appel, activée seulement si refus). */
    optionnelle: boolean("optionnelle").notNull().default(false),
    /** true = plus aucune modification famille sur cette étape. */
    gelee: boolean("gelee").notNull().default(false),
    frozenAt: timestamp("frozen_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("fd_etape_campagne_idx").on(t.campagneId),
    uniqueIndex("fd_etape_campagne_ordre_uidx").on(t.campagneId, t.ordre),
    index("fd_etape_etablissement_idx").on(t.etablissementId),
  ],
);

export const fdFiche = pgTable(
  "fd_fiche",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    campagneId: uuid("campagne_id")
      .notNull()
      .references(() => fdCampagne.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    /** Snapshot identité au moment de la création (préremplissage). */
    eleveNom: text("eleve_nom").notNull(),
    elevePrenom: text("eleve_prenom").notNull(),
    classeActuelle: text("classe_actuelle").notNull().default(""),
    optionsActuelles: jsonb("options_actuelles").$type<string[]>().notNull().default([]),
    parentEmails: jsonb("parent_emails").$type<string[]>().notNull().default([]),
    statut: text("statut").$type<FdFicheStatut>().notNull().default("a_envoyer"),
    etapeCouranteId: uuid("etape_courante_id").references(() => fdEtape.id, {
      onDelete: "set null",
    }),
    lastSentAt: timestamp("last_sent_at", { withTimezone: true }),
    lastReminderAt: timestamp("last_reminder_at", { withTimezone: true }),
    reminderCount: integer("reminder_count").notNull().default(0),
    acceptation: jsonb("acceptation").$type<FdAcceptationPayload | null>(),
    acceptedAt: timestamp("accepted_at", { withTimezone: true }),
    refusedAt: timestamp("refused_at", { withTimezone: true }),
    closedAt: timestamp("closed_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("fd_fiche_campagne_eleve_uidx").on(t.campagneId, t.eleveId),
    index("fd_fiche_campagne_idx").on(t.campagneId),
    index("fd_fiche_eleve_idx").on(t.eleveId),
    index("fd_fiche_statut_idx").on(t.etablissementId, t.statut),
    index("fd_fiche_classe_idx").on(t.campagneId, t.classeActuelle),
  ],
);

export const fdReponse = pgTable(
  "fd_reponse",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    ficheId: uuid("fiche_id")
      .notNull()
      .references(() => fdFiche.id, { onDelete: "cascade" }),
    etapeId: uuid("etape_id")
      .notNull()
      .references(() => fdEtape.id, { onDelete: "cascade" }),
    auteurRole: text("auteur_role").notNull(),
    auteurUserId: text("auteur_user_id"),
    auteurLabel: text("auteur_label"),
    payload: jsonb("payload")
      .$type<FdReponsePayload | FdConseilDecisionPayload | FdAcceptationPayload>()
      .notNull(),
    submittedAt: timestamp("submitted_at", { withTimezone: true }).notNull().defaultNow(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("fd_reponse_fiche_idx").on(t.ficheId),
    index("fd_reponse_etape_idx").on(t.etapeId),
    uniqueIndex("fd_reponse_fiche_etape_auteur_uidx").on(t.ficheId, t.etapeId, t.auteurRole),
  ],
);

export const fdSignature = pgTable(
  "fd_signature",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    ficheId: uuid("fiche_id")
      .notNull()
      .references(() => fdFiche.id, { onDelete: "cascade" }),
    etapeId: uuid("etape_id")
      .notNull()
      .references(() => fdEtape.id, { onDelete: "cascade" }),
    role: text("role").notNull(),
    signerName: text("signer_name").notNull(),
    signerEmail: text("signer_email"),
    method: text("method").notNull().default("pad"),
    signaturePngBase64: text("signature_png_base64"),
    signedAt: timestamp("signed_at", { withTimezone: true }).notNull().defaultNow(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("fd_signature_fiche_idx").on(t.ficheId),
    uniqueIndex("fd_signature_fiche_etape_role_uidx").on(t.ficheId, t.etapeId, t.role),
  ],
);

export const fdToken = pgTable(
  "fd_token",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    ficheId: uuid("fiche_id")
      .notNull()
      .references(() => fdFiche.id, { onDelete: "cascade" }),
    etapeId: uuid("etape_id").references(() => fdEtape.id, { onDelete: "set null" }),
    token: text("token").notNull(),
    secureCode: text("secure_code"),
    email: text("email"),
    purpose: text("purpose").notNull().default("saisie"),
    expiresAt: timestamp("expires_at", { withTimezone: true }),
    usedAt: timestamp("used_at", { withTimezone: true }),
    revokedAt: timestamp("revoked_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("fd_token_token_uidx").on(t.token),
    index("fd_token_fiche_idx").on(t.ficheId),
    index("fd_token_email_code_idx").on(t.email, t.secureCode),
  ],
);

export const fdPdfArchive = pgTable(
  "fd_pdf_archive",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    ficheId: uuid("fiche_id")
      .notNull()
      .references(() => fdFiche.id, { onDelete: "cascade" }),
    etapeId: uuid("etape_id").references(() => fdEtape.id, { onDelete: "set null" }),
    kind: text("kind").notNull(),
    title: text("title").notNull(),
    s3Key: text("s3_key"),
    eleveDocumentId: uuid("eleve_document_id"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("fd_pdf_archive_fiche_idx").on(t.ficheId),
    index("fd_pdf_archive_etablissement_idx").on(t.etablissementId),
  ],
);

export const fichesDialogueSchema = {
  fdCampagne,
  fdEtape,
  fdFiche,
  fdReponse,
  fdSignature,
  fdToken,
  fdPdfArchive,
};

export type FdCampagneRow = typeof fdCampagne.$inferSelect;
export type FdEtapeRow = typeof fdEtape.$inferSelect;
export type FdFicheRow = typeof fdFiche.$inferSelect;
export type FdReponseRow = typeof fdReponse.$inferSelect;
export type FdSignatureRow = typeof fdSignature.$inferSelect;
export type FdTokenRow = typeof fdToken.$inferSelect;
export type FdPdfArchiveRow = typeof fdPdfArchive.$inferSelect;
