/**
 * RDV inscriptions — pages publiques branchées sur Google Agenda (1 agenda / direction).
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

/** Config globale du module pour un établissement (1 ligne / etab). */
export const rdvInscriptionConfig = pgTable(
  "rdv_inscription_config",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    enabled: integer("enabled").notNull().default(0),
    title: text("title").notNull().default("Rendez-vous d’inscription"),
    intro: text("intro").notNull().default(""),
    /** Sous-chaîne attendue dans le titre Google (ex. « rendez-vous inscription »). */
    eventTitlePattern: text("event_title_pattern").notNull().default("rendez-vous inscription"),
    notifyEmail: text("notify_email"),
    location: text("location").notNull().default(""),
    consentLabel: text("consent_label").notNull().default(
      "J’accepte que mes coordonnées soient utilisées pour organiser ce rendez-vous d’inscription.",
    ),
    /** Horizon de listing des créneaux (jours). */
    horizonDays: integer("horizon_days").notNull().default(60),
    /** État public du lien OAuth (sans token). */
    googleLinked: integer("google_linked").notNull().default(0),
    googleLinkedEmail: text("google_linked_email"),
    googleLinkedAt: timestamp("google_linked_at", { withTimezone: true }),
    /** Refresh token OAuth Google (compte technique) — source de vérité pour l’API Agenda. */
    googleRefreshToken: text("google_refresh_token"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("rdv_inscription_config_etab_uidx").on(t.etablissementId),
    index("rdv_inscription_config_etab_idx").on(t.etablissementId),
  ],
);

/** Une direction = une page publique + un calendarId Google + ses propres textes / motif / notif. */
export const rdvInscriptionDirection = pgTable(
  "rdv_inscription_direction",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Slug URL : ecole | college | lycee | libre. */
    slug: text("slug").notNull(),
    label: text("label").notNull(),
    googleCalendarId: text("google_calendar_id").notNull().default(""),
    directriceDisplayName: text("directrice_display_name"),
    /** Titre affiché sur la page publique de cette direction. */
    title: text("title").notNull().default("Rendez-vous d’inscription"),
    intro: text("intro").notNull().default(""),
    /** Sous-chaîne exacte attendue dans le titre Google pour cette direction. */
    eventTitlePattern: text("event_title_pattern").notNull().default("rendez-vous inscription"),
    notifyEmail: text("notify_email"),
    location: text("location").notNull().default(""),
    consentLabel: text("consent_label").notNull().default(
      "J’accepte que mes coordonnées soient utilisées pour organiser ce rendez-vous d’inscription.",
    ),
    horizonDays: integer("horizon_days").notNull().default(60),
    active: integer("active").notNull().default(1),
    sortOrder: integer("sort_order").notNull().default(0),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("rdv_inscription_dir_etab_slug_uidx").on(t.etablissementId, t.slug),
    index("rdv_inscription_dir_etab_idx").on(t.etablissementId),
    index("rdv_inscription_dir_active_idx").on(t.etablissementId, t.active),
  ],
);

/** Miroir d’une réservation parent (source de vérité agenda = Google). */
export const rdvInscriptionBooking = pgTable(
  "rdv_inscription_booking",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    directionId: uuid("direction_id")
      .notNull()
      .references(() => rdvInscriptionDirection.id, { onDelete: "cascade" }),
    directionSlug: text("direction_slug").notNull(),
    googleEventId: text("google_event_id").notNull(),
    googleCalendarId: text("google_calendar_id").notNull(),
    googleHtmlLink: text("google_html_link"),
    startAt: timestamp("start_at", { withTimezone: true }).notNull(),
    endAt: timestamp("end_at", { withTimezone: true }).notNull(),
    studentFirstName: text("student_first_name").notNull(),
    studentLastName: text("student_last_name").notNull(),
    parentEmail: text("parent_email").notNull(),
    parentPhone: text("parent_phone").notNull(),
    /** confirmed | cancelled */
    status: text("status").notNull().default("confirmed"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("rdv_inscription_booking_etab_event_uidx").on(
      t.etablissementId,
      t.googleCalendarId,
      t.googleEventId,
    ),
    index("rdv_inscription_booking_etab_idx").on(t.etablissementId),
    index("rdv_inscription_booking_dir_idx").on(t.etablissementId, t.directionId),
    index("rdv_inscription_booking_status_idx").on(t.etablissementId, t.status),
    index("rdv_inscription_booking_start_idx").on(t.etablissementId, t.startAt),
  ],
);

export const rdvInscriptionSchema = {
  rdvInscriptionConfig,
  rdvInscriptionDirection,
  rdvInscriptionBooking,
};
