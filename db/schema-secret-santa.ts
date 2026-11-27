import { index, jsonb, pgTable, primaryKey, text, timestamp, uuid } from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

/** Secret Santa : un organisateur, ses participants, un tirage à une heure précise. */
export const secretSanta = pgTable(
  "secret_santa",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    organizerUserId: text("organizer_user_id").notNull(),
    organizerName: text("organizer_name").notNull(),
    title: text("title").notNull(),
    budget: text("budget").notNull().default(""),
    message: text("message").notNull().default(""),
    revealAt: timestamp("reveal_at", { withTimezone: true }).notNull(),
    drawnAt: timestamp("drawn_at", { withTimezone: true }),
    /** Rendez-vous de remise des cadeaux (facultatif). */
    eventPlace: text("event_place").notNull().default(""),
    eventAt: timestamp("event_at", { withTimezone: true }),
    /** Dernier envoi « les informations ont changé ». */
    updatesSentAt: timestamp("updates_sent_at", { withTimezone: true }),
    /** Paires interdites : `[[userId, userId], …]`. */
    exclusions: jsonb("exclusions").$type<[string, string][]>().notNull().default([]),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("secret_santa_organizer_idx").on(t.etablissementId, t.organizerUserId)],
);

export const secretSantaParticipant = pgTable(
  "secret_santa_participant",
  {
    santaId: uuid("santa_id")
      .notNull()
      .references(() => secretSanta.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    displayName: text("display_name").notNull(),
    email: text("email").notNull(),
    /** Rempli au tirage. Montré uniquement à la personne `userId`. */
    receiverUserId: text("receiver_user_id"),
    invitedAt: timestamp("invited_at", { withTimezone: true }),
    openedAt: timestamp("opened_at", { withTimezone: true }),
    /** pending | accepted | declined | withdrawn (retiré après le tirage). */
    response: text("response").$type<"pending" | "accepted" | "declined" | "withdrawn">().notNull().default("pending"),
    respondedAt: timestamp("responded_at", { withTimezone: true }),
    /** Son nom tiré a changé après qu'elle l'a vu ; effacé quand elle rouvre son cadeau. */
    changedAt: timestamp("changed_at", { withTimezone: true }),
  },
  (t) => [
    primaryKey({ name: "secret_santa_participant_pk", columns: [t.santaId, t.userId] }),
    index("secret_santa_participant_user_idx").on(t.userId),
  ],
);

/**
 * Incompatibilités confidentielles de l'établissement (`userA` < `userB`). Les liens vers `user`
 * (suppression en cascade) sont posés par la migration SQL.
 */
export const secretSantaIncompatibility = pgTable(
  "secret_santa_incompatibility",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    userA: text("user_a").notNull(),
    userB: text("user_b").notNull(),
    createdBy: text("created_by").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [primaryKey({ name: "secret_santa_incompatibility_pk", columns: [t.etablissementId, t.userA, t.userB] })],
);

export const secretSantaSchema = { secretSanta, secretSantaParticipant, secretSantaIncompatibility };
