import {
  boolean,
  date,
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
import { edtCreneau } from "./schema-charlemagne-p1";

/**
 * VS Phase 2 — Appels de classe & absences élèves.
 * Appel = instance datée d'un créneau EDT (ou saisie libre classe + horaire).
 * eleve_id : FK SQL vers eleve (évite import circulaire Drizzle).
 */
export const vsAppel = pgTable(
  "vs_appel",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    dateAppel: date("date_appel").notNull(),
    creneauId: uuid("creneau_id").references(() => edtCreneau.id, { onDelete: "set null" }),
    classe: text("classe").notNull(),
    heureDebut: text("heure_debut"),
    heureFin: text("heure_fin"),
    matiereLibelle: text("matiere_libelle"),
    enseignantUserId: text("enseignant_user_id"),
    enseignantNom: text("enseignant_nom"),
    statut: text("statut").notNull().default("en_cours"),
    closAt: timestamp("clos_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("vs_appel_etab_date_idx").on(t.etablissementId, t.dateAppel),
    index("vs_appel_classe_idx").on(t.etablissementId, t.classe, t.dateAppel),
    uniqueIndex("vs_appel_creneau_date_uidx").on(t.etablissementId, t.creneauId, t.dateAppel),
  ],
);

/** Ligne d'appel : present | absent | retard | dispense. */
export const vsAppelLigne = pgTable(
  "vs_appel_ligne",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    appelId: uuid("appel_id")
      .notNull()
      .references(() => vsAppel.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    statut: text("statut").notNull().default("present"),
    retardMinutes: integer("retard_minutes"),
    note: text("note"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.appelId, t.eleveId], name: "vs_appel_ligne_pk" }),
    index("vs_appel_ligne_eleve_idx").on(t.etablissementId, t.eleveId),
    index("vs_appel_ligne_statut_idx").on(t.etablissementId, t.statut),
  ],
);

/**
 * Absence / retard consolidé pour le suivi CPE (justifs, relances).
 * Sources : appel de classe, saisie accueil, famille, pont Charlemagne (futur).
 */
export const vsAbsenceEleve = pgTable(
  "vs_absence_eleve",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    appelId: uuid("appel_id").references(() => vsAppel.id, { onDelete: "set null" }),
    dateDebut: date("date_debut").notNull(),
    dateFin: date("date_fin").notNull(),
    heureDebut: text("heure_debut"),
    heureFin: text("heure_fin"),
    type: text("type").notNull().default("absence"),
    statut: text("statut").notNull().default("a_traiter"),
    justifie: boolean("justifie").notNull().default(false),
    motif: text("motif"),
    justificatifKey: text("justificatif_key"),
    relanceAt: timestamp("relance_at", { withTimezone: true }),
    traiteParUserId: text("traite_par_user_id"),
    traiteAt: timestamp("traite_at", { withTimezone: true }),
    noteCpe: text("note_cpe"),
    /** appel | accueil | famille | charlemagne */
    source: text("source").notNull().default("appel"),
    createdByUserId: text("created_by_user_id"),
    createdByNom: text("created_by_nom"),
    /** telephone | physique | mail */
    canal: text("canal"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("vs_absence_eleve_etab_statut_idx").on(t.etablissementId, t.statut),
    index("vs_absence_eleve_eleve_idx").on(t.etablissementId, t.eleveId),
    index("vs_absence_eleve_date_idx").on(t.etablissementId, t.dateDebut),
    index("vs_absence_eleve_etab_source_date_idx").on(t.etablissementId, t.source, t.dateDebut),
    uniqueIndex("vs_absence_eleve_appel_uidx").on(t.etablissementId, t.appelId, t.eleveId),
  ],
);

/** Catalogue court de sanctions (CPE) — pas un moteur permis à points Phase 3. */
export const vsSanctionType = pgTable(
  "vs_sanction_type",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    gravite: integer("gravite").notNull().default(1),
    actif: boolean("actif").notNull().default(true),
    ordre: integer("ordre").notNull().default(1),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("vs_sanction_type_uidx").on(t.etablissementId, t.code),
    index("vs_sanction_type_etab_idx").on(t.etablissementId),
  ],
);

export const vsSanction = pgTable(
  "vs_sanction",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    typeId: uuid("type_id")
      .notNull()
      .references(() => vsSanctionType.id, { onDelete: "restrict" }),
    dateSanction: date("date_sanction").notNull(),
    motif: text("motif"),
    statut: text("statut").notNull().default("active"),
    createdByUserId: text("created_by_user_id"),
    createdByNom: text("created_by_nom"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("vs_sanction_etab_date_idx").on(t.etablissementId, t.dateSanction),
    index("vs_sanction_eleve_idx").on(t.etablissementId, t.eleveId),
    index("vs_sanction_statut_idx").on(t.etablissementId, t.statut),
  ],
);

export const charlemagneP4Schema = {
  vsAppel,
  vsAppelLigne,
  vsAbsenceEleve,
  vsSanctionType,
  vsSanction,
};
