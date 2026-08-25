import {
  boolean,
  date,
  index,
  integer,
  numeric,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";
import { noteMatiere, notePeriode, noteTypeDevoir } from "./schema-charlemagne-p1";

/** Devoir / évaluation — Notes Phase 2. */
export const noteDevoir = pgTable(
  "note_devoir",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    matiereId: uuid("matiere_id")
      .notNull()
      .references(() => noteMatiere.id, { onDelete: "cascade" }),
    periodeId: uuid("periode_id")
      .notNull()
      .references(() => notePeriode.id, { onDelete: "cascade" }),
    typeDevoirId: uuid("type_devoir_id").references(() => noteTypeDevoir.id, {
      onDelete: "set null",
    }),
    classe: text("classe").notNull(),
    groupeId: uuid("groupe_id"),
    libelle: text("libelle").notNull(),
    dateDevoir: date("date_devoir"),
    coefficient: numeric("coefficient", { precision: 6, scale: 2 }).notNull().default("1"),
    createdByUserId: text("created_by_user_id"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("note_devoir_classe_idx").on(t.etablissementId, t.classe),
    index("note_devoir_periode_idx").on(t.etablissementId, t.periodeId),
  ],
);

/** Note élève sur un devoir. */
export const noteValeur = pgTable(
  "note_valeur",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    devoirId: uuid("devoir_id")
      .notNull()
      .references(() => noteDevoir.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    valeur: numeric("valeur", { precision: 5, scale: 2 }),
    absent: boolean("absent").notNull().default(false),
    dispense: boolean("dispense").notNull().default(false),
    appreciation: text("appreciation"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.devoirId, t.eleveId],
      name: "note_valeur_pk",
    }),
    index("note_valeur_eleve_idx").on(t.etablissementId, t.eleveId),
  ],
);

/** Moyenne calculée (cache) par élève × matière × période. */
export const noteMoyenneEleve = pgTable(
  "note_moyenne_eleve",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    matiereId: uuid("matiere_id")
      .notNull()
      .references(() => noteMatiere.id, { onDelete: "cascade" }),
    periodeId: uuid("periode_id")
      .notNull()
      .references(() => notePeriode.id, { onDelete: "cascade" }),
    moyenne: numeric("moyenne", { precision: 5, scale: 2 }),
    nbNotes: integer("nb_notes").notNull().default(0),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("note_moyenne_eleve_uidx").on(
      t.etablissementId,
      t.eleveId,
      t.matiereId,
      t.periodeId,
    ),
  ],
);

export const charlemagneP2Schema = {
  noteDevoir,
  noteValeur,
  noteMoyenneEleve,
};
