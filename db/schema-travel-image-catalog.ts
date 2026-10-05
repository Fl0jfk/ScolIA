/**
 * Catalogue d'images de couverture voyages — partagé entre tous les établissements.
 * Enrichi automatiquement (Wikimedia / Unsplash) puis réutilisé.
 */
import {
  index,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
} from "drizzle-orm/pg-core";

export type TravelImageCatalogSource = "manual" | "wikimedia" | "unsplash" | "openverse";

export const travelImageCatalog = pgTable(
  "travel_image_catalog",
  {
    /** Identifiant stable (slug ou id historique du JSON). */
    id: text("id").primaryKey(),
    label: text("label").notNull(),
    url: text("url").notNull(),
    keywords: text("keywords").notNull().default(""),
    /** Clé normalisée pour dédup (ex. disneylandparis). */
    normalizeKey: text("normalize_key").notNull(),
    source: text("source").$type<TravelImageCatalogSource>().notNull().default("manual"),
    author: text("author"),
    license: text("license"),
    attributionUrl: text("attribution_url"),
    sourcePageUrl: text("source_page_url"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("travel_image_catalog_normalize_key_uidx").on(t.normalizeKey),
    index("travel_image_catalog_source_idx").on(t.source),
  ],
);
