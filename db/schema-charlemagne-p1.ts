/**
 * Tables Phase 1 Charlemagne — Admin (Pont EN + facturation) + Notes config + groupes VS.
 * Voir plans charlemagne_admin_ent / pont_en.
 * Pas d’import depuis schema.ts (évite cycle) — FKs croisées renforcées en migration SQL.
 */
import {
  boolean,
  date,
  index,
  integer,
  jsonb,
  numeric,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

/** Nomenclatures Siècle / STS (MEF, matières, régimes, communes…). */
export const refNomenclature = pgTable(
  "ref_nomenclature",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    type: text("type").notNull(),
    code: text("code").notNull(),
    libelleCourt: text("libelle_court"),
    libelleLong: text("libelle_long"),
    source: text("source").notNull().default("siecle"),
    metadataJson: jsonb("metadata_json"),
    validFrom: date("valid_from"),
    validTo: date("valid_to"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("ref_nomenclature_etab_type_code_uidx").on(t.etablissementId, t.type, t.code),
    index("ref_nomenclature_etab_type_idx").on(t.etablissementId, t.type),
  ],
);

/** Établissements France (RNE) — référentiel global Siècle. */
export const refEtablissement = pgTable(
  "ref_etablissement",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    codeRne: text("code_rne").notNull(),
    codeNature: text("code_nature"),
    codeType: text("code_type"),
    codeSecteur: text("code_secteur"),
    sigle: text("sigle"),
    denomPrinc: text("denom_princ"),
    denomCompl: text("denom_compl"),
    adresse: text("adresse"),
    dateOuverture: date("date_ouverture"),
    dateFermeture: date("date_fermeture"),
    source: text("source").notNull().default("siecle"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [uniqueIndex("ref_etablissement_rne_uidx").on(t.codeRne)],
);

export const nomenclatureImportLog = pgTable(
  "nomenclature_import_log",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    fichier: text("fichier").notNull(),
    source: text("source").notNull().default("siecle_xml"),
    dateImport: timestamp("date_import", { withTimezone: true }).notNull().defaultNow(),
    statut: text("statut").notNull().default("ok"),
    nbInserts: integer("nb_inserts").notNull().default(0),
    nbUpdates: integer("nb_updates").notNull().default(0),
    nbDeletes: integer("nb_deletes").notNull().default(0),
    rapportJson: jsonb("rapport_json"),
  },
  (t) => [index("nomenclature_import_log_etab_idx").on(t.etablissementId, t.dateImport)],
);

export const noteMatiere = pgTable(
  "note_matiere",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    groupeMatiere: text("groupe_matiere"),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("note_matiere_etab_code_uidx").on(t.etablissementId, t.code),
    index("note_matiere_etab_idx").on(t.etablissementId),
  ],
);

export const notePeriode = pgTable(
  "note_periode",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    niveauModele: text("niveau_modele").notNull().default("tous"),
    dateDebut: date("date_debut"),
    dateFin: date("date_fin"),
    statut: text("statut").notNull().default("ouverte"),
    ordre: integer("ordre").notNull().default(1),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("note_periode_etab_idx").on(t.etablissementId),
    uniqueIndex("note_periode_etab_code_uidx").on(t.etablissementId, t.code, t.anneeScolaireId),
  ],
);

export const noteTypeDevoir = pgTable(
  "note_type_devoir",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    coefDefaut: numeric("coef_defaut", { precision: 6, scale: 2 }).default("1"),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [uniqueIndex("note_type_devoir_etab_code_uidx").on(t.etablissementId, t.code)],
);

export const noteMatiereClasse = pgTable(
  "note_matiere_classe",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    matiereId: uuid("matiere_id")
      .notNull()
      .references(() => noteMatiere.id, { onDelete: "cascade" }),
    classe: text("classe").notNull(),
    enseignantUserId: text("enseignant_user_id"),
    enseignantNom: text("enseignant_nom"),
    coef: numeric("coef", { precision: 6, scale: 2 }).notNull().default("1"),
    compteDansMg: boolean("compte_dans_mg").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("note_matiere_classe_uidx").on(t.etablissementId, t.matiereId, t.classe),
    index("note_matiere_classe_classe_idx").on(t.etablissementId, t.classe),
  ],
);

/** Groupe transversal Notes + Vie scolaire. */
export const groupePedagogique = pgTable(
  "groupe_pedagogique",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    type: text("type").notNull().default("autre"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("groupe_pedagogique_etab_code_uidx").on(t.etablissementId, t.code),
    index("groupe_pedagogique_etab_idx").on(t.etablissementId),
  ],
);

export const groupePedagogiqueMembre = pgTable(
  "groupe_pedagogique_membre",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    groupeId: uuid("groupe_id")
      .notNull()
      .references(() => groupePedagogique.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.groupeId, t.eleveId],
      name: "groupe_pedagogique_membre_pk",
    }),
    index("groupe_pedagogique_membre_eleve_idx").on(t.etablissementId, t.eleveId),
  ],
);

export const tarif = pgTable(
  "tarif",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    code: text("code").notNull(),
    libelle: text("libelle").notNull(),
    compteProduit: text("compte_produit"),
    tvaTaux: numeric("tva_taux", { precision: 5, scale: 2 }).notNull().default("0"),
    periodicite: text("periodicite").notNull().default("mensuel"),
    portee: text("portee").notNull().default("autre"),
    porteeValeur: text("portee_valeur"),
    prixUnitaire: numeric("prix_unitaire", { precision: 12, scale: 2 }).notNull().default("0"),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("tarif_etab_code_uidx").on(t.etablissementId, t.code, t.anneeScolaireId),
    index("tarif_etab_idx").on(t.etablissementId),
  ],
);

export const foyerFacturation = pgTable(
  "foyer_facturation",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id").notNull(),
    codeAuxiliaire: text("code_auxiliaire"),
    categorieQuotient: text("categorie_quotient"),
    quotientFamilial: numeric("quotient_familial", { precision: 12, scale: 2 }),
    iban: text("iban"),
    bic: text("bic"),
    rum: text("rum"),
    mandatDate: date("mandat_date"),
    acceptePrelevement: boolean("accepte_prelevement").notNull().default(false),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("foyer_facturation_foyer_uidx").on(t.etablissementId, t.foyerId),
    index("foyer_facturation_etab_idx").on(t.etablissementId),
  ],
);

export const facture = pgTable(
  "facture",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id").notNull(),
    anneeScolaireId: uuid("annee_scolaire_id"),
    numero: text("numero").notNull(),
    statut: text("statut").notNull().default("brouillon"),
    dateEmission: date("date_emission"),
    dateEcheance: date("date_echeance"),
    totalHt: numeric("total_ht", { precision: 12, scale: 2 }).notNull().default("0"),
    totalTtc: numeric("total_ttc", { precision: 12, scale: 2 }).notNull().default("0"),
    pdfKey: text("pdf_key"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("facture_etab_numero_uidx").on(t.etablissementId, t.numero),
    index("facture_foyer_idx").on(t.etablissementId, t.foyerId),
    index("facture_statut_idx").on(t.etablissementId, t.statut),
  ],
);

export const factureLigne = pgTable(
  "facture_ligne",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    factureId: uuid("facture_id")
      .notNull()
      .references(() => facture.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id"),
    tarifId: uuid("tarif_id"),
    libelle: text("libelle").notNull(),
    periode: text("periode"),
    quantite: numeric("quantite", { precision: 10, scale: 2 }).notNull().default("1"),
    prixUnitaire: numeric("prix_unitaire", { precision: 12, scale: 2 }).notNull().default("0"),
    remise: numeric("remise", { precision: 12, scale: 2 }).notNull().default("0"),
    totalHt: numeric("total_ht", { precision: 12, scale: 2 }).notNull().default("0"),
    totalTtc: numeric("total_ttc", { precision: 12, scale: 2 }).notNull().default("0"),
    ordre: integer("ordre").notNull().default(1),
  },
  (t) => [index("facture_ligne_facture_idx").on(t.etablissementId, t.factureId)],
);

export const encaissement = pgTable(
  "encaissement",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id").notNull(),
    mode: text("mode").notNull().default("virement"),
    montant: numeric("montant", { precision: 12, scale: 2 }).notNull(),
    dateEncaissement: date("date_encaissement").notNull(),
    reference: text("reference"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("encaissement_foyer_idx").on(t.etablissementId, t.foyerId)],
);

export const factureEncaissement = pgTable(
  "facture_encaissement",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    factureId: uuid("facture_id")
      .notNull()
      .references(() => facture.id, { onDelete: "cascade" }),
    encaissementId: uuid("encaissement_id")
      .notNull()
      .references(() => encaissement.id, { onDelete: "cascade" }),
    montant: numeric("montant", { precision: 12, scale: 2 }).notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.factureId, t.encaissementId],
      name: "facture_encaissement_pk",
    }),
  ],
);

export const calendrierScolaire = pgTable(
  "calendrier_scolaire",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    label: text("label").notNull(),
    dateDebut: date("date_debut").notNull(),
    dateFin: date("date_fin").notNull(),
    type: text("type").notNull().default("vacances"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("calendrier_scolaire_etab_idx").on(t.etablissementId, t.anneeScolaireId)],
);

export const edtCreneau = pgTable(
  "edt_creneau",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    jourSemaine: integer("jour_semaine").notNull(),
    heureDebut: text("heure_debut").notNull(),
    heureFin: text("heure_fin").notNull(),
    classe: text("classe"),
    groupeId: uuid("groupe_id"),
    matiereId: uuid("matiere_id"),
    enseignantNom: text("enseignant_nom"),
    salle: text("salle"),
    semaine: text("semaine").notNull().default("AB"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("edt_creneau_classe_idx").on(t.etablissementId, t.classe),
    index("edt_creneau_jour_idx").on(t.etablissementId, t.jourSemaine),
  ],
);

export const charlemagneP1Schema = {
  refNomenclature,
  refEtablissement,
  nomenclatureImportLog,
  noteMatiere,
  notePeriode,
  noteTypeDevoir,
  noteMatiereClasse,
  groupePedagogique,
  groupePedagogiqueMembre,
  tarif,
  foyerFacturation,
  facture,
  factureLigne,
  encaissement,
  factureEncaissement,
  calendrierScolaire,
  edtCreneau,
};
