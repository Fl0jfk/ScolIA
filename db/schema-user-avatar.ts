import { customType, pgTable, text, timestamp } from "drizzle-orm/pg-core";

const bytea = customType<{ data: Buffer; driverData: Buffer | Uint8Array }>({
  dataType: () => "bytea",
  fromDriver: (value) => Buffer.from(value),
});

/**
 * Photo de profil d'un membre du personnel (petit JPEG carré, sans métadonnées).
 * En base plutôt que dans le bucket d'un établissement : un compte peut en avoir plusieurs.
 * Le lien vers `user` (suppression en cascade) est posé par la migration SQL.
 */
export const userAvatar = pgTable("user_avatar", {
  userId: text("user_id").primaryKey(),
  image: bytea("image").notNull(),
  updatedBy: text("updated_by").notNull(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
});

export const userAvatarSchema = { userAvatar };
