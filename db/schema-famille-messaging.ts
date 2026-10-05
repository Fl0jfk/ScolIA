/**
 * Messagerie familles ↔ établissement (canal dédié, cloisonné du Messenger staff).
 * Threads, messages, pièces, notifs in-app, matrice de rôles.
 */

import {
  boolean,
  index,
  integer,
  jsonb,
  pgTable,
  text,
  timestamp,
  uuid,
  check,
  uniqueIndex,
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
    /** true = message envoyé en diffusion (même sujet/corps vers plusieurs foyers). */
    isBroadcast: boolean("is_broadcast").notNull().default(false),
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

export const familleThreadAttachment = pgTable(
  "famille_thread_attachment",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    messageId: uuid("message_id")
      .notNull()
      .references(() => familleThreadMessage.id, { onDelete: "cascade" }),
    fileName: text("file_name").notNull(),
    mime: text("mime").notNull(),
    size: integer("size").notNull(),
    /** Clé S3 ; null si stockage Postgres (labo local). */
    s3Key: text("s3_key"),
    /** Contenu inline (labo / repli) — max ~1 Mo côté app. */
    contentBase64: text("content_base64"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("famille_thread_attachment_message_idx").on(t.messageId),
    index("famille_thread_attachment_etab_idx").on(t.etablissementId),
  ],
);

/** Matrice : qui peut écrire / diffuser aux foyers. Une ligne par établissement. */
export const familleMessagingSettings = pgTable(
  "famille_messaging_settings",
  {
    etablissementId: uuid("etablissement_id")
      .primaryKey()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Rôles autorisés à initier un thread (ex. ["admin","cpe","direction","administratif","professeur"]). */
    rolesCanInitiate: jsonb("roles_can_initiate")
      .$type<string[]>()
      .notNull()
      .default(sql`'["admin","cpe","direction","directeur","directrice","administratif"]'::jsonb`),
    /** Si true, un professeur ne peut écrire qu’aux foyers de ses classes. */
    profOwnClassesOnly: boolean("prof_own_classes_only").notNull().default(true),
    /** Diffusion multi-foyers réservée aux rôles initiateurs + admin. */
    allowBroadcast: boolean("allow_broadcast").notNull().default(true),
    allowParentAttachments: boolean("allow_parent_attachments").notNull().default(true),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    updatedByUserId: text("updated_by_user_id"),
  },
);

/** Notifs in-app parent (badge / push soft — pas de web-push natif). */
export const familleNotif = pgTable(
  "famille_notif",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id").notNull(),
    threadId: uuid("thread_id").references(() => familleThread.id, { onDelete: "cascade" }),
    kind: text("kind").notNull().default("new_message"),
    titre: text("titre").notNull(),
    preview: text("preview").notNull().default(""),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    luAt: timestamp("lu_at", { withTimezone: true }),
  },
  (t) => [
    index("famille_notif_foyer_idx").on(t.etablissementId, t.foyerId, t.luAt),
    index("famille_notif_thread_idx").on(t.threadId),
    check(
      "famille_notif_kind_chk",
      sql`${t.kind} in ('new_message', 'broadcast', 'reply_staff')`,
    ),
    uniqueIndex("famille_notif_id_uidx").on(t.id),
  ],
);

export const familleMessagingSchema = {
  familleThread,
  familleThreadMessage,
  familleThreadAttachment,
  familleMessagingSettings,
  familleNotif,
};
