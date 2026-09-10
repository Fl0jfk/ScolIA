import {
  boolean,
  index,
  integer,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { etablissement } from "./etablissement-table";

/**
 * Messagerie interne (1:1 + groupes) — style Facebook Messenger.
 */

export type MessagingMessageType =
  | "text"
  | "image"
  | "file"
  | "audio"
  | "video"
  | "system";

export type MessagingConversationKind = "dm" | "group";

export const messagingConversation = pgTable(
  "messaging_conversation",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** dm | group */
    kind: text("kind").$type<MessagingConversationKind>().notNull().default("dm"),
    /** Nom du groupe (null pour dm). */
    title: text("title"),
    createdById: text("created_by_id"),
    /** Pair normalisé pour dm : userAId < userBId. Null pour les groupes. */
    userAId: text("user_a_id"),
    userBId: text("user_b_id"),
    lastMessageAt: timestamp("last_message_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("messaging_conversation_dm_pair_uidx")
      .on(t.etablissementId, t.userAId, t.userBId)
      .where(sql`${t.kind} = 'dm'`),
    index("messaging_conversation_etab_idx").on(t.etablissementId),
    index("messaging_conversation_last_msg_idx").on(t.etablissementId, t.lastMessageAt),
    index("messaging_conversation_kind_idx").on(t.etablissementId, t.kind),
  ],
);

export const messagingParticipant = pgTable(
  "messaging_participant",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    conversationId: text("conversation_id")
      .notNull()
      .references(() => messagingConversation.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    lastReadAt: timestamp("last_read_at", { withTimezone: true }).notNull().defaultNow(),
    lastReadMessageId: text("last_read_message_id"),
    muted: boolean("muted").notNull().default(false),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("messaging_participant_conv_user_uidx").on(t.conversationId, t.userId),
    index("messaging_participant_user_idx").on(t.etablissementId, t.userId),
    index("messaging_participant_etab_idx").on(t.etablissementId),
  ],
);

export const messagingMessage = pgTable(
  "messaging_message",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    conversationId: text("conversation_id")
      .notNull()
      .references(() => messagingConversation.id, { onDelete: "cascade" }),
    senderId: text("sender_id").notNull(),
    type: text("type").$type<MessagingMessageType>().notNull().default("text"),
    body: text("body"),
    replyToId: text("reply_to_id"),
    forwardedFromId: text("forwarded_from_id"),
    editedAt: timestamp("edited_at", { withTimezone: true }),
    deletedAt: timestamp("deleted_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("messaging_message_conv_created_idx").on(t.etablissementId, t.conversationId, t.createdAt),
    index("messaging_message_etab_idx").on(t.etablissementId),
    index("messaging_message_sender_idx").on(t.etablissementId, t.senderId),
  ],
);

export const messagingAttachment = pgTable(
  "messaging_attachment",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    messageId: text("message_id")
      .notNull()
      .references(() => messagingMessage.id, { onDelete: "cascade" }),
    s3Key: text("s3_key").notNull(),
    mime: text("mime").notNull(),
    size: integer("size").notNull(),
    fileName: text("file_name").notNull(),
    width: integer("width"),
    height: integer("height"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("messaging_attachment_message_idx").on(t.messageId),
    index("messaging_attachment_etab_idx").on(t.etablissementId),
  ],
);

export const messagingReaction = pgTable(
  "messaging_reaction",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    messageId: text("message_id")
      .notNull()
      .references(() => messagingMessage.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    emoji: text("emoji").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("messaging_reaction_unique_uidx").on(t.messageId, t.userId, t.emoji),
    index("messaging_reaction_message_idx").on(t.messageId),
    index("messaging_reaction_etab_idx").on(t.etablissementId),
  ],
);

export const messagingSchema = {
  messagingConversation,
  messagingParticipant,
  messagingMessage,
  messagingAttachment,
  messagingReaction,
};

export type MessagingConversationRow = typeof messagingConversation.$inferSelect;
export type MessagingParticipantRow = typeof messagingParticipant.$inferSelect;
export type MessagingMessageRow = typeof messagingMessage.$inferSelect;
export type MessagingAttachmentRow = typeof messagingAttachment.$inferSelect;
export type MessagingReactionRow = typeof messagingReaction.$inferSelect;
