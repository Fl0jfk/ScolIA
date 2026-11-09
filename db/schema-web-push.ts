import { index, integer, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { etablissement } from "./etablissement-table";

/**
 * Notifications navigateur (Web Push, standard W3C + VAPID).
 * Un abonnement = un navigateur / appareil d'un utilisateur (Better-Auth `user.id`).
 */
export const webPushSubscription = pgTable(
  "web_push_subscription",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    endpoint: text("endpoint").notNull(),
    p256dh: text("p256dh").notNull(),
    auth: text("auth").notNull(),
    userAgent: text("user_agent"),
    /** Échecs consécutifs d'envoi (abonnement supprimé à 404/410 ou après plusieurs échecs). */
    failureCount: integer("failure_count").notNull().default(0),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    lastSuccessAt: timestamp("last_success_at", { withTimezone: true }),
  },
  (t) => [
    uniqueIndex("web_push_subscription_endpoint_uidx").on(t.endpoint),
    index("web_push_subscription_user_idx").on(t.etablissementId, t.userId),
  ],
);

/** Préférences : catégories coupées et heures calmes (minutes depuis minuit, heure de Paris). */
export const webPushPreference = pgTable("web_push_preference", {
  userId: text("user_id").primaryKey(),
  etablissementId: uuid("etablissement_id")
    .notNull()
    .references(() => etablissement.id, { onDelete: "cascade" }),
  mutedKinds: text("muted_kinds").array().notNull().default(sql`'{}'::text[]`),
  quietStartMinute: integer("quiet_start_minute"),
  quietEndMinute: integer("quiet_end_minute"),
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
});

export const webPushSchema = { webPushSubscription, webPushPreference };

export type WebPushSubscriptionRow = typeof webPushSubscription.$inferSelect;
export type WebPushPreferenceRow = typeof webPushPreference.$inferSelect;
