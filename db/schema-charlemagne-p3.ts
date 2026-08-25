import {
  boolean,
  index,
  integer,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";
import { noteMatiere, notePeriode } from "./schema-charlemagne-p1";

/** Domaine de compétences (socle / LSU collège). */
export const noteCompetenceDomaine = pgTable(
  "note_competence_domaine",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    /** college | lycee | tous */
    cycle: text("cycle").notNull().default("college"),
    ordre: integer("ordre").notNull().default(1),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("note_competence_domaine_uidx").on(t.etablissementId, t.code),
    index("note_competence_domaine_etab_idx").on(t.etablissementId),
  ],
);

/** Item de compétence rattaché à un domaine (optionnellement une matière). */
export const noteCompetenceItem = pgTable(
  "note_competence_item",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    domaineId: uuid("domaine_id")
      .notNull()
      .references(() => noteCompetenceDomaine.id, { onDelete: "cascade" }),
    matiereId: uuid("matiere_id").references(() => noteMatiere.id, { onDelete: "set null" }),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    ordre: integer("ordre").notNull().default(1),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("note_competence_item_uidx").on(t.etablissementId, t.domaineId, t.code),
    index("note_competence_item_domaine_idx").on(t.etablissementId, t.domaineId),
  ],
);

/** Évaluation compétence élève × item × période (niveau de maîtrise LSU). */
export const noteCompetenceValeur = pgTable(
  "note_competence_valeur",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    itemId: uuid("item_id")
      .notNull()
      .references(() => noteCompetenceItem.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    periodeId: uuid("periode_id")
      .notNull()
      .references(() => notePeriode.id, { onDelete: "cascade" }),
    /** 1=insuffisant, 2=fragile, 3=satisfaisant, 4=très bon */
    niveau: text("niveau"),
    appreciation: text("appreciation"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.itemId, t.eleveId, t.periodeId],
      name: "note_competence_valeur_pk",
    }),
    index("note_competence_valeur_eleve_idx").on(t.etablissementId, t.eleveId),
    index("note_competence_valeur_periode_idx").on(t.etablissementId, t.periodeId),
  ],
);

export const charlemagneP3Schema = {
  noteCompetenceDomaine,
  noteCompetenceItem,
  noteCompetenceValeur,
};
