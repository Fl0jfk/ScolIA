/**
 * Partenariats & offres — catalogue public + inscriptions (coupon numérique).
 */
import {
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

export type PartenariatCtaLink = {
  label: string;
  url: string;
};

export type PartenariatTarif = {
  label: string;
  amountCents?: number | null;
  amountLabel?: string;
  note?: string;
};

/** Une fiche publique (partenaire externe ou offre interne). */
export const partenariatOffre = pgTable(
  "partenariat_offre",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Slug URL unique par établissement. */
    slug: text("slug").notNull(),
    title: text("title").notNull(),
    shortDescription: text("short_description").notNull().default(""),
    body: text("body").notNull().default(""),
    logoS3Key: text("logo_s3_key"),
    /** info | inscription */
    kind: text("kind").notNull().default("info"),
    /** ecole | college | lycee — jsonb string[] */
    cycles: jsonb("cycles").$type<string[]>().notNull().default([]),
    /** Codes niveau libres (3eme, 2nde…) — vide = tous les niveaux des cycles. */
    niveaux: jsonb("niveaux").$type<string[]>().notNull().default([]),
    categoryLabel: text("category_label").notNull().default(""),
    contactName: text("contact_name").notNull().default(""),
    contactRole: text("contact_role").notNull().default(""),
    contactEmail: text("contact_email").notNull().default(""),
    contactPhone: text("contact_phone").notNull().default(""),
    partnerContactName: text("partner_contact_name").notNull().default(""),
    partnerContactRole: text("partner_contact_role").notNull().default(""),
    partnerContactEmail: text("partner_contact_email").notNull().default(""),
    partnerContactPhone: text("partner_contact_phone").notNull().default(""),
    ctaLinks: jsonb("cta_links").$type<PartenariatCtaLink[]>().notNull().default([]),
    tarifs: jsonb("tarifs").$type<PartenariatTarif[]>().notNull().default([]),
    demarche: text("demarche").notNull().default(""),
    engagementText: text("engagement_text")
      .notNull()
      .default(
        "Je m’engage à régler les sommes dues liées à cette inscription selon les modalités communiquées par l’établissement.",
      ),
    requireSignature: integer("require_signature").notNull().default(1),
    notifyEmail: text("notify_email"),
    maxPlaces: integer("max_places"),
    inscriptionOpensAt: timestamp("inscription_opens_at", { withTimezone: true }),
    inscriptionClosesAt: timestamp("inscription_closes_at", { withTimezone: true }),
    enabled: integer("enabled").notNull().default(0),
    sortOrder: integer("sort_order").notNull().default(0),
    publishedAt: timestamp("published_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("partenariat_offre_etab_slug_uidx").on(t.etablissementId, t.slug),
    index("partenariat_offre_etab_idx").on(t.etablissementId),
    index("partenariat_offre_etab_enabled_idx").on(t.etablissementId, t.enabled),
    index("partenariat_offre_etab_sort_idx").on(t.etablissementId, t.sortOrder),
  ],
);

/** Réunion d’information / événement lié à une offre. */
export const partenariatEvenement = pgTable(
  "partenariat_evenement",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    offreId: uuid("offre_id")
      .notNull()
      .references(() => partenariatOffre.id, { onDelete: "cascade" }),
    title: text("title").notNull(),
    startsAt: timestamp("starts_at", { withTimezone: true }).notNull(),
    endsAt: timestamp("ends_at", { withTimezone: true }),
    location: text("location").notNull().default(""),
    notes: text("notes").notNull().default(""),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("partenariat_evenement_etab_idx").on(t.etablissementId),
    index("partenariat_evenement_offre_idx").on(t.etablissementId, t.offreId),
    index("partenariat_evenement_starts_idx").on(t.offreId, t.startsAt),
  ],
);

/** Coupon numérique d’inscription (sans paiement). */
export const partenariatInscription = pgTable(
  "partenariat_inscription",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    offreId: uuid("offre_id")
      .notNull()
      .references(() => partenariatOffre.id, { onDelete: "cascade" }),
    eleveFirstName: text("eleve_first_name").notNull(),
    eleveLastName: text("eleve_last_name").notNull(),
    eleveNiveau: text("eleve_niveau").notNull().default(""),
    eleveClasse: text("eleve_classe").notNull().default(""),
    eleveBirthDate: text("eleve_birth_date"),
    parentFirstName: text("parent_first_name").notNull(),
    parentLastName: text("parent_last_name").notNull(),
    parentEmail: text("parent_email").notNull(),
    parentPhone: text("parent_phone").notNull().default(""),
    engagementAcceptedAt: timestamp("engagement_accepted_at", { withTimezone: true }).notNull(),
    signatureS3Key: text("signature_s3_key"),
    /** soumise | validee | refusee | annulee */
    status: text("status").notNull().default("soumise"),
    adminNote: text("admin_note").notNull().default(""),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("partenariat_inscription_etab_idx").on(t.etablissementId),
    index("partenariat_inscription_offre_idx").on(t.etablissementId, t.offreId),
    index("partenariat_inscription_status_idx").on(t.offreId, t.status),
    index("partenariat_inscription_email_idx").on(t.offreId, t.parentEmail),
  ],
);

export const partenariatsSchema = {
  partenariatOffre,
  partenariatEvenement,
  partenariatInscription,
};
