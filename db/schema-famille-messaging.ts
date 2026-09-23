/**
 * Messagerie familles ↔ établissement (canal dédié, cloisonné du Messenger staff).
 * Thread foyer + messages texte (Value Gate light).
 */

import {
  index,
  pgTable,
  text,
  timestamp,
  uuid,
  check,
} from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { etablissement } from "./etablissement-table";

export const familleThread = pgTable(
  "famille_thread",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id").notNull(),
    eleveId: uuid("eleve_id"),
    sujet: text("sujet").notNull(),
    createdByUserId: text("created_by_user_id"),
    createdByNom: text("created_by_nom"),
    lastMessageAt: timestamp("last_message_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("famille_thread_etab_foyer_idx").on(t.etablissementId, t.foyerId),
    index("famille_thread_etab_last_idx").on(t.etablissementId, t.lastMessageAt),
  ],
);

export const familleThreadMessage = pgTable(
  "famille_thread_message",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    threadId: uuid("thread_id")
      .notNull()
      .references(() => familleThread.id, { onDelete: "cascade" }),
    /** staff = établissement ; parent = famille */
    auteurCote: text("auteur_cote").notNull(),
    auteurUserId: text("auteur_user_id"),
    auteurNom: text("auteur_nom"),
    corps: text("corps").notNull(),
    luAt: timestamp("lu_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("famille_thread_message_thread_idx").on(t.etablissementId, t.threadId, t.createdAt),
    check(
      "famille_thread_message_cote_chk",
      sql`${t.auteurCote} in ('staff', 'parent')`,
    ),
  ],
);

export const familleMessagingSchema = {
  familleThread,
  familleThreadMessage,
};
