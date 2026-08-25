import {
  boolean,
  date,
  index,
  pgTable,
  text,
  timestamp,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

/**
 * VS Phase 3 — Carnet de correspondance.
 * Canal unidirectionnel établissement → famille + accusé de lecture/signature.
 * categorie : correspondance | accompagnement | information
 */
export const vsCarnetEntree = pgTable(
  "vs_carnet_entree",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    dateEntree: date("date_entree").notNull(),
    categorie: text("categorie").notNull().default("correspondance"),
    titre: text("titre").notNull(),
    corps: text("corps").notNull(),
    visibleFamille: boolean("visible_famille").notNull().default(true),
    createdByUserId: text("created_by_user_id"),
    createdByNom: text("created_by_nom"),
    signeAt: timestamp("signe_at", { withTimezone: true }),
    signeParUserId: text("signe_par_user_id"),
    signeParNom: text("signe_par_nom"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("vs_carnet_etab_date_idx").on(t.etablissementId, t.dateEntree),
    index("vs_carnet_eleve_idx").on(t.etablissementId, t.eleveId),
    index("vs_carnet_signe_idx").on(t.etablissementId, t.signeAt),
  ],
);

export const charlemagneP5Schema = {
  vsCarnetEntree,
};
