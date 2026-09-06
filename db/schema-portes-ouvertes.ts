/**
 * Portes ouvertes — tables typées Postgres (config, créneaux, inscriptions, staffing).
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

/** Config texte / meta PO pour un tenant (1 ligne / établissement). */
export const portesOuvertesConfig = pgTable(
  "portes_ouvertes_config",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    title: text("title").notNull().default("Portes ouvertes"),
    intro: text("intro").notNull().default(""),
    address: text("address").notNull().default(""),
    mapsUrl: text("maps_url"),
    notifyEmail: text("notify_email"),
    /** Lien public vers le formulaire de préinscription. */
    preinscriptionUrl: text("preinscription_url"),
    /** Délai en minutes après check-in avant le mail de suivi (défaut 60). */
    followUpDelayMinutes: integer("follow_up_delay_minutes").notNull().default(60),
    consentLabel: text("consent_label").notNull().default(
      "J'accepte que mes coordonnées soient utilisées pour organiser ma visite et me recontacter si besoin.",
    ),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("portes_ouvertes_config_etab_uidx").on(t.etablissementId),
    index("portes_ouvertes_config_etab_idx").on(t.etablissementId),
  ],
);

/** Créneau horodaté, rattaché à un cycle (école / collège / lycée). */
export const portesOuvertesSlot = pgTable(
  "portes_ouvertes_slot",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    id: text("id").notNull(),
    /** ecole | college | lycee */
    cycle: text("cycle").notNull(),
    label: text("label").notNull(),
    startAt: timestamp("start_at", { withTimezone: true }).notNull(),
    endAt: timestamp("end_at", { withTimezone: true }).notNull(),
    maxPlaces: integer("max_places"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("portes_ouvertes_slot_pk_idx").on(t.etablissementId, t.id),
    uniqueIndex("portes_ouvertes_slot_etab_id_uidx").on(t.etablissementId, t.id),
    index("portes_ouvertes_slot_etab_cycle_idx").on(t.etablissementId, t.cycle),
    index("portes_ouvertes_slot_etab_start_idx").on(t.etablissementId, t.startAt),
  ],
);

/** Inscription visiteur (public ou saisie Accueil). */
export const portesOuvertesRegistration = pgTable(
  "portes_ouvertes_registration",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    id: text("id").notNull(),
    slotId: text("slot_id").notNull(),
    slotLabel: text("slot_label"),
    slotStartAt: timestamp("slot_start_at", { withTimezone: true }),
    slotEndAt: timestamp("slot_end_at", { withTimezone: true }),
    firstName: text("first_name").notNull(),
    lastName: text("last_name").notNull(),
    email: text("email").notNull(),
    phone: text("phone"),
    childrenInfo: text("children_info"),
    childFirstName: text("child_first_name"),
    childLastName: text("child_last_name"),
    /** ecole | college | lycee */
    cycle: text("cycle"),
    classeSouhaitee: text("classe_souhaitee"),
    consent: boolean("consent").notNull().default(true),
    /** public | accueil */
    source: text("source"),
    recordedByUserId: text("recorded_by_user_id"),
    recordedByName: text("recorded_by_name"),
    lastModifiedByUserId: text("last_modified_by_user_id"),
    lastModifiedByName: text("last_modified_by_name"),
    /** Visite physiquement effectuée (check-in Accueil). */
    visitedAt: timestamp("visited_at", { withTimezone: true }),
    /** Quand envoyer le mail de suivi (visited_at + délai config). */
    followUpDueAt: timestamp("follow_up_due_at", { withTimezone: true }),
    /** Mail de suivi déjà envoyé. */
    followUpEmailSentAt: timestamp("follow_up_email_sent_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }),
  },
  (t) => [
    uniqueIndex("portes_ouvertes_reg_etab_id_uidx").on(t.etablissementId, t.id),
    index("portes_ouvertes_reg_etab_idx").on(t.etablissementId),
    index("portes_ouvertes_reg_slot_idx").on(t.etablissementId, t.slotId),
    index("portes_ouvertes_reg_cycle_idx").on(t.etablissementId, t.cycle),
    index("portes_ouvertes_reg_email_idx").on(t.etablissementId, t.email),
    index("portes_ouvertes_reg_followup_idx").on(t.followUpDueAt, t.followUpEmailSentAt),
  ],
);

/** Équipe sur un créneau : ambassadeur (élève), enseignant, personnel. */
export const portesOuvertesSlotStaff = pgTable(
  "portes_ouvertes_slot_staff",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    id: uuid("id").primaryKey().defaultRandom(),
    slotId: text("slot_id").notNull(),
    /** ambassadeur | enseignant | personnel */
    role: text("role").notNull(),
    refId: text("ref_id").notNull(),
    displayName: text("display_name").notNull(),
    meta: jsonb("meta").$type<Record<string, string>>(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("portes_ouvertes_staff_etab_idx").on(t.etablissementId),
    index("portes_ouvertes_staff_slot_idx").on(t.etablissementId, t.slotId),
    index("portes_ouvertes_staff_role_idx").on(t.etablissementId, t.slotId, t.role),
    uniqueIndex("portes_ouvertes_staff_unique_uidx").on(
      t.etablissementId,
      t.slotId,
      t.role,
      t.refId,
    ),
  ],
);

export const portesOuvertesSchema = {
  portesOuvertesConfig,
  portesOuvertesSlot,
  portesOuvertesRegistration,
  portesOuvertesSlotStaff,
};
