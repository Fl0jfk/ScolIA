import { pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";

/** Établissement scolaire (tenant ScolIA). */
export const etablissement = pgTable(
  "etablissement",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    slug: text("slug").notNull(),
    name: text("name").notNull(),
    dataBucket: text("data_bucket"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [uniqueIndex("etablissement_slug_uidx").on(t.slug)],
);
