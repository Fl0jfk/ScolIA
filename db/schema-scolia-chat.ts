import {
  index,
  integer,
  jsonb,
  pgTable,
  text,
  timestamp,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

/**
 * Conversations ScolIA (Brain AI) — historique multi-appareils par utilisateur.
 */

export const scoliaConversation = pgTable(
  "scolia_conversation",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    title: text("title").notNull().default("Nouvelle conversation"),
    /** État wizard / confirmation Brain (slots, pending…). */
    state: jsonb("state").$type<Record<string, unknown>>().notNull().default({}),
    lastMessageAt: timestamp("last_message_at", { withTimezone: true }),
    archivedAt: timestamp("archived_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("scolia_conversation_user_idx").on(t.etablissementId, t.userId, t.updatedAt),
    index("scolia_conversation_etab_idx").on(t.etablissementId),
  ],
);

export const scoliaMessage = pgTable(
  "scolia_message",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    conversationId: text("conversation_id")
      .notNull()
      .references(() => scoliaConversation.id, { onDelete: "cascade" }),
    role: text("role").$type<"user" | "assistant">().notNull(),
    content: text("content").notNull(),
    seq: integer("seq").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("scolia_message_conv_idx").on(t.conversationId, t.seq),
    index("scolia_message_etab_idx").on(t.etablissementId),
  ],
);

export const scoliaChatSchema = {
  scoliaConversation,
  scoliaMessage,
};

export type ScoliaConversationRow = typeof scoliaConversation.$inferSelect;
export type ScoliaMessageRow = typeof scoliaMessage.$inferSelect;
