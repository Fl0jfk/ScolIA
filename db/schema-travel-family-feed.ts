import { boolean, index, integer, pgTable, primaryKey, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { etablissement } from "./etablissement-table";

/**
 * « Fil du voyage » : fil PRIVÉ réservé aux familles des élèves participants (lien personnel par adresse).
 * Jamais public. Tout est supprimé 7 jours après le retour. Toutes les tables sont filtrées par `etablissement_id`.
 */

/** Réglages et compteurs du fil d'un voyage (une ligne par voyage une fois le fil ouvert). */
export const travelFeedSettings = pgTable(
  "travel_feed_settings",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    tripId: text("trip_id").notNull(),
    openedAt: timestamp("opened_at", { withTimezone: true }).notNull().defaultNow(),
    openedBy: text("opened_by"),
    /** Téléchargement des photos par les familles : désactivé par défaut (réglable plus tard). */
    allowDownload: boolean("allow_download").notNull().default(false),
    /** E-mails « nouveautés » déjà envoyés pour ce voyage (plafond par voyage). */
    emailsSent: integer("emails_sent").notNull().default(0),
    /** Date de retour (AAAA-MM-JJ), remise à jour à chaque action : sert à la lecture seule et à la purge. */
    returnDate: text("return_date"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [primaryKey({ columns: [t.etablissementId, t.tripId] })],
);

/** Accès personnel d'une adresse de responsable au fil d'un voyage (jetons hachés, jamais en clair). */
export const travelFeedAccess = pgTable(
  "travel_feed_access",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    tripId: text("trip_id").notNull(),
    email: text("email").notNull(),
    /** INE des élèves participants rattachés à cette adresse (contrôlés à chaque lecture contre la liste officielle). */
    ines: text("ines").array().notNull().default(sql`'{}'::text[]`),
    /** Clé de famille (premier INE trié) : une réaction par type et par famille. */
    familyKey: text("family_key").notNull(),
    tokenHashes: text("token_hashes").array().notNull().default(sql`'{}'::text[]`),
    firstSentAt: timestamp("first_sent_at", { withTimezone: true }),
    sendFailures: integer("send_failures").notNull().default(0),
    /** Dernier e-mail « nouveautés » envoyé (au plus un par heure et par famille). */
    lastNotifiedAt: timestamp("last_notified_at", { withTimezone: true }),
    notifyCount: integer("notify_count").notNull().default(0),
    /** Publications arrivées depuis le dernier e-mail (résumé « N nouveautés »). */
    pendingMessages: integer("pending_messages").notNull().default(0),
    pendingPhotos: integer("pending_photos").notNull().default(0),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("travel_feed_access_trip_email_uidx").on(t.etablissementId, t.tripId, t.email),
    index("travel_feed_access_trip_idx").on(t.etablissementId, t.tripId),
  ],
);

/** Publications des profs (raccourcis, programme, messages courts, photos). */
export const travelFeedPost = pgTable(
  "travel_feed_post",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    tripId: text("trip_id").notNull(),
    /** arrives | ok | delay | departure_back | at_school | program | message | photos */
    kind: text("kind").notNull(),
    body: text("body"),
    delayMinutes: integer("delay_minutes"),
    authorUserId: text("author_user_id"),
    authorName: text("author_name"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    hiddenAt: timestamp("hidden_at", { withTimezone: true }),
    hiddenBy: text("hidden_by"),
  },
  (t) => [index("travel_feed_post_trip_idx").on(t.etablissementId, t.tripId, t.createdAt)],
);

/** Photos d'une publication : octets déjà nettoyés (sans EXIF) dans le stockage objet privé. */
export const travelFeedPhoto = pgTable(
  "travel_feed_photo",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    tripId: text("trip_id").notNull(),
    postId: uuid("post_id")
      .notNull()
      .references(() => travelFeedPost.id, { onDelete: "cascade" }),
    storageKey: text("storage_key").notNull(),
    width: integer("width").notNull(),
    height: integer("height").notNull(),
    bytes: integer("bytes").notNull(),
    position: integer("position").notNull().default(0),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("travel_feed_photo_post_idx").on(t.etablissementId, t.postId)],
);

/** Réactions des familles : une par type et par famille, retirable. Aucun nom affiché. */
export const travelFeedReaction = pgTable(
  "travel_feed_reaction",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    postId: uuid("post_id")
      .notNull()
      .references(() => travelFeedPost.id, { onDelete: "cascade" }),
    tripId: text("trip_id").notNull(),
    familyKey: text("family_key").notNull(),
    emoji: text("emoji").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.postId, t.familyKey, t.emoji] }),
    index("travel_feed_reaction_trip_idx").on(t.etablissementId, t.tripId),
  ],
);

/** Journal de modération et d administration du fil (qui, quoi, quand) : ni texte ni photo, conservé après la purge du fil. */
export const travelFeedAudit = pgTable(
  "travel_feed_audit",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    tripId: text("trip_id").notNull(),
    actorUserId: text("actor_user_id"),
    actorName: text("actor_name"),
    action: text("action").notNull(),
    postId: uuid("post_id"),
    detail: text("detail"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("travel_feed_audit_trip_idx").on(t.etablissementId, t.tripId, t.createdAt)],
);
