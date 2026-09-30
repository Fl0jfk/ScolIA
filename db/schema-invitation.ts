/**
 * Invitations cérémonies — pages RSVP publiques (remise bac / brevet, etc.).
 */
import {
  index,
  integer,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

/** Une page d’invitation publique (N par établissement). */
export const invitationPage = pgTable(
  "invitation_page",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Slug URL unique par établissement. */
    slug: text("slug").notNull(),
    title: text("title").notNull().default("Invitation"),
    intro: text("intro").notNull().default(""),
    /** remise_diplome | neutre */
    theme: text("theme").notNull().default("remise_diplome"),
    enabled: integer("enabled").notNull().default(0),
    startsAt: timestamp("starts_at", { withTimezone: true }),
    endsAt: timestamp("ends_at", { withTimezone: true }),
    location: text("location").notNull().default(""),
    /** none | bac | brevet | both */
    diplomaMode: text("diploma_mode").notNull().default("both"),
    maxTotalPersons: integer("max_total_persons").notNull().default(200),
    maxPersonsPerEleve: integer("max_persons_per_eleve").notNull().default(4),
    notifyEmail: text("notify_email"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("invitation_page_etab_slug_uidx").on(t.etablissementId, t.slug),
    index("invitation_page_etab_idx").on(t.etablissementId),
    index("invitation_page_etab_enabled_idx").on(t.etablissementId, t.enabled),
  ],
);

/** Réponse RSVP d’une famille pour une page. */
export const invitationRsvp = pgTable(
  "invitation_rsvp",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    pageId: uuid("page_id")
      .notNull()
      .references(() => invitationPage.id, { onDelete: "cascade" }),
    eleveFirstName: text("eleve_first_name").notNull(),
    eleveLastName: text("eleve_last_name").notNull(),
    /** Clé de dédoublonnage (minuscules, sans accents). */
    eleveNameNorm: text("eleve_name_norm").notNull(),
    /** oui | non */
    response: text("response").notNull(),
    /** Effectif si Oui ; 0 si Non. */
    presentCount: integer("present_count").notNull().default(0),
    parentEmail: text("parent_email").notNull(),
    /** bac | brevet | null */
    diploma: text("diploma"),
    /** Groupe de reliure admin (doublons confirmés). */
    duplicateGroupId: uuid("duplicate_group_id"),
    /** Admin a écarté le signal « doublon suspect ». */
    duplicateDismissedAt: timestamp("duplicate_dismissed_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("invitation_rsvp_etab_idx").on(t.etablissementId),
    index("invitation_rsvp_page_idx").on(t.etablissementId, t.pageId),
    index("invitation_rsvp_page_norm_idx").on(t.pageId, t.eleveNameNorm),
    index("invitation_rsvp_page_response_idx").on(t.pageId, t.response),
    index("invitation_rsvp_dup_group_idx").on(t.pageId, t.duplicateGroupId),
  ],
);

export const invitationSchema = {
  invitationPage,
  invitationRsvp,
};
