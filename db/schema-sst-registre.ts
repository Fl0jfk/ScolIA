/**
 * Registre Santé & Sécurité au Travail (SST) — émargement annuel + fiches.
 */
import {
  boolean,
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

export type SstFicheStatus =
  | "ouverte"
  | "visa_responsable"
  | "examen_cse"
  | "decision"
  | "realisation"
  | "cloturee";

export type SstFicheVisa = {
  notes: string;
  userId: string;
  name: string;
  at: string;
  signaturePngBase64?: string | null;
};

export type SstFicheWorkflow = {
  responsable?: SstFicheVisa;
  cse?: SstFicheVisa;
  decision?: SstFicheVisa;
  realisation?: SstFicheVisa;
  closedAt?: string | null;
};

/** Campagne d’émargement pour une année scolaire. */
export const sstCampagne = pgTable(
  "sst_campagne",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeLabel: text("annee_label").notNull(),
    title: text("title").notNull(),
    active: boolean("active").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("sst_campagne_etab_annee_uidx").on(t.etablissementId, t.anneeLabel),
    index("sst_campagne_etab_idx").on(t.etablissementId),
  ],
);

/** Signature d’information annuelle d’un collaborateur. */
export const sstEmargement = pgTable(
  "sst_emargement",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    campagneId: uuid("campagne_id")
      .notNull()
      .references(() => sstCampagne.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    firstName: text("first_name").notNull().default(""),
    lastName: text("last_name").notNull().default(""),
    fonction: text("fonction").notNull().default(""),
    signedAt: timestamp("signed_at", { withTimezone: true }).notNull().defaultNow(),
    signaturePngBase64: text("signature_png_base64").notNull(),
    remarques: text("remarques").notNull().default(""),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("sst_emargement_campagne_user_uidx").on(t.campagneId, t.userId),
    index("sst_emargement_etab_idx").on(t.etablissementId),
    index("sst_emargement_user_idx").on(t.etablissementId, t.userId),
  ],
);

/** Fiche de signalement / suggestion SST. */
export const sstFiche = pgTable(
  "sst_fiche",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    campagneId: uuid("campagne_id")
      .notNull()
      .references(() => sstCampagne.id, { onDelete: "cascade" }),
    numero: integer("numero").notNull(),
    createdByUserId: text("created_by_user_id").notNull(),
    declarantFirstName: text("declarant_first_name").notNull().default(""),
    declarantLastName: text("declarant_last_name").notNull().default(""),
    declarantFonction: text("declarant_fonction").notNull().default(""),
    observedDate: text("observed_date").notNull(),
    observedTime: text("observed_time").notNull().default(""),
    lieu: text("lieu").notNull().default(""),
    observations: text("observations").notNull(),
    suggestions: text("suggestions").notNull().default(""),
    signaturePngBase64: text("signature_png_base64"),
    status: text("status").notNull().default("ouverte"),
    workflow: jsonb("workflow").$type<SstFicheWorkflow>().notNull().default({}),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("sst_fiche_etab_numero_uidx").on(t.etablissementId, t.numero),
    index("sst_fiche_etab_idx").on(t.etablissementId),
    index("sst_fiche_campagne_idx").on(t.campagneId),
    index("sst_fiche_status_idx").on(t.etablissementId, t.status),
  ],
);

/** Visa de consultation du registre (direction, CSE, OGEC…). */
export const sstConsultation = pgTable(
  "sst_consultation",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    campagneId: uuid("campagne_id")
      .notNull()
      .references(() => sstCampagne.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    firstName: text("first_name").notNull().default(""),
    lastName: text("last_name").notNull().default(""),
    fonction: text("fonction").notNull().default(""),
    consultedAt: timestamp("consulted_at", { withTimezone: true }).notNull().defaultNow(),
    comments: text("comments").notNull().default(""),
    signaturePngBase64: text("signature_png_base64"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("sst_consultation_etab_idx").on(t.etablissementId),
    index("sst_consultation_campagne_idx").on(t.campagneId),
  ],
);

export const sstRegistreSchema = {
  sstCampagne,
  sstEmargement,
  sstFiche,
  sstConsultation,
};
