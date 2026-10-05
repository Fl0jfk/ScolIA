/**
 * Faits administratifs absents du matin, prévus pour tout l’établissement.
 * Pas d’écran. Comptabilité = partie simple dans l’ENT (familles, dépenses, caisse).
 * Pas de plan comptable, pas de partie double.
 * FKs vers eleve / annee_scolaire / personnel : posées en SQL (évite le cycle schema.ts).
 */
import { sql } from "drizzle-orm";
import {
  boolean,
  check,
  date,
  index,
  integer,
  numeric,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";
import { edtCreneau, encaissement, facture, noteMatiere, notePeriode } from "./schema-charlemagne-p1";

/** Porte ou self. Le repas pris = un passage `lieu = self`, pas une seconde table. */
export const passage = pgTable(
  "passage",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id"),
    /** personnel.id est un text. */
    personnelId: text("personnel_id"),
    inviteNom: text("invite_nom"),
    /** entree | sortie */
    sens: text("sens").notNull(),
    /** portail | self | internat | infirmerie | autre */
    lieu: text("lieu").notNull(),
    horodatage: timestamp("horodatage", { withTimezone: true }).notNull(),
    /** badge | app | manuel */
    source: text("source").notNull().default("manuel"),
    anneeScolaireId: uuid("annee_scolaire_id"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("passage_etab_horaire_idx").on(t.etablissementId, t.horodatage),
    index("passage_eleve_idx").on(t.etablissementId, t.eleveId, t.horodatage),
    index("passage_lieu_idx").on(t.etablissementId, t.lieu, t.horodatage),
    check(
      "passage_sujet_chk",
      sql`${t.eleveId} is not null or ${t.personnelId} is not null or (${t.inviteNom} is not null and btrim(${t.inviteNom}) <> '')`,
    ),
    check("passage_sens_chk", sql`${t.sens} in ('entree', 'sortie')`),
    check(
      "passage_lieu_chk",
      sql`${t.lieu} in ('portail', 'self', 'internat', 'infirmerie', 'autre')`,
    ),
  ],
);

/**
 * Présence à l’infirmerie. Le motif court n’est pas le dossier médical.
 * `signal_vie_scolaire` = « l’élève est à l’infirmerie », sans le contenu de santé.
 */
export const infirmeriePassage = pgTable(
  "infirmerie_passage",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    arrivee: timestamp("arrivee", { withTimezone: true }).notNull(),
    sortie: timestamp("sortie", { withTimezone: true }),
    motifCourt: text("motif_court").notNull().default(""),
    /**
     * Suite du passage (soin), renseignée à la clôture.
     * repos | retour_cours | renvoi_famille | urgence | autre
     */
    suite: text("suite"),
    /** Ce qui a été fait — reste à l’infirmerie, pas le signal VS. */
    soinsNotes: text("soins_notes"),
    /** journee | nuit_internat — même geste, autre horaire. */
    contexte: text("contexte").notNull().default("journee"),
    signalVieScolaire: boolean("signal_vie_scolaire").notNull().default(true),
    auteurUserId: text("auteur_user_id"),
    auteurNom: text("auteur_nom"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("infirmerie_passage_eleve_idx").on(t.etablissementId, t.eleveId, t.arrivee),
    index("infirmerie_passage_ouverts_idx").on(t.etablissementId, t.sortie),
    check(
      "infirmerie_passage_suite_chk",
      sql`${t.suite} is null or ${t.suite} in ('repos', 'retour_cours', 'renvoi_famille', 'urgence', 'autre')`,
    ),
    check(
      "infirmerie_passage_contexte_chk",
      sql`${t.contexte} in ('journee', 'nuit_internat')`,
    ),
  ],
);

/** Extrait opérationnel (allergie cantine, inaptitude EPS). Le PAI reste un document. */
export const santeExtrait = pgTable(
  "sante_extrait",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    /** cantine | eps | voyage | internat | periscolaire */
    portee: text("portee").notNull(),
    libelle: text("libelle").notNull(),
    documentId: uuid("document_id"),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("sante_extrait_eleve_idx").on(t.etablissementId, t.eleveId),
    index("sante_extrait_portee_idx").on(t.etablissementId, t.portee),
    check(
      "sante_extrait_portee_chk",
      sql`${t.portee} in ('cantine', 'eps', 'voyage', 'internat', 'periscolaire')`,
    ),
  ],
);

/**
 * Fiche infirmerie par élève (antécédents utiles à l’établissement).
 * Pas le dossier du médecin scolaire. Une fiche ouverte par élève.
 */
export const infirmerieFiche = pgTable(
  "infirmerie_fiche",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    antecedents: text("antecedents").notNull().default(""),
    personnesAPrevenir: text("personnes_a_prevenir").notNull().default(""),
    notes: text("notes").notNull().default(""),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("infirmerie_fiche_eleve_uidx").on(t.etablissementId, t.eleveId),
  ],
);

/** Prise de médicament à l’infirmerie. L’ordonnance peut pointer un document. */
export const santeMedicamentPrise = pgTable(
  "sante_medicament_prise",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    medicament: text("medicament").notNull(),
    dose: text("dose"),
    prisAt: timestamp("pris_at", { withTimezone: true }).notNull(),
    auteurUserId: text("auteur_user_id"),
    auteurNom: text("auteur_nom"),
    documentId: uuid("document_id"),
    notes: text("notes"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("sante_medicament_prise_eleve_idx").on(t.etablissementId, t.eleveId, t.prisAt),
  ],
);

/** Registre des accidents. Conservation longue — pas de DELETE de masse. */
export const santeAccident = pgTable(
  "sante_accident",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    dateAccident: date("date_accident").notNull(),
    circonstances: text("circonstances").notNull().default(""),
    soins: text("soins").notNull().default(""),
    suite: text("suite").notNull().default(""),
    lieu: text("lieu"),
    auteurUserId: text("auteur_user_id"),
    auteurNom: text("auteur_nom"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("sante_accident_eleve_idx").on(t.etablissementId, t.eleveId, t.dateAccident),
    index("sante_accident_date_idx").on(t.etablissementId, t.dateAccident),
  ],
);

/**
 * PAI tenu par l’infirmerie : protocole + traitements + lien document.
 * Le PDF reste dans `eleve_document` (tiroir santé). Statuts soft — pas de DELETE.
 */
export const santePai = pgTable(
  "sante_pai",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    /** brouillon | valide | expire | revoque */
    statut: text("statut").notNull().default("brouillon"),
    protocole: text("protocole").notNull().default(""),
    traitementsAutorises: text("traitements_autorises").notNull().default(""),
    documentId: uuid("document_id"),
    dateDebut: date("date_debut"),
    dateFin: date("date_fin"),
    valideAt: timestamp("valide_at", { withTimezone: true }),
    valideParUserId: text("valide_par_user_id"),
    valideParNom: text("valide_par_nom"),
    notes: text("notes").notNull().default(""),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("sante_pai_eleve_idx").on(t.etablissementId, t.eleveId),
    index("sante_pai_statut_idx").on(t.etablissementId, t.statut),
    check(
      "sante_pai_statut_chk",
      sql`${t.statut} in ('brouillon', 'valide', 'expire', 'revoque')`,
    ),
  ],
);

/**
 * Inaptitude EPS datée. Crée / tient un extrait `portee=eps`.
 * Soft-désactivation — pas de DELETE.
 */
export const santeInaptitudeEps = pgTable(
  "sante_inaptitude_eps",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    dateDebut: date("date_debut").notNull(),
    dateFin: date("date_fin"),
    /** Motif court côté infirmerie (pas diffusé tel quel). */
    motif: text("motif").notNull().default(""),
    /** Libellé vu par l’EPS via sante_extrait. */
    libelleExtrait: text("libelle_extrait").notNull(),
    documentId: uuid("document_id"),
    extraitId: uuid("extrait_id"),
    actif: boolean("actif").notNull().default(true),
    auteurUserId: text("auteur_user_id"),
    auteurNom: text("auteur_nom"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("sante_inaptitude_eps_eleve_idx").on(t.etablissementId, t.eleveId, t.dateDebut),
    index("sante_inaptitude_eps_actif_idx").on(t.etablissementId, t.actif),
  ],
);

export const conseilSeance = pgTable(
  "conseil_seance",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    periodeId: uuid("periode_id").references(() => notePeriode.id, { onDelete: "set null" }),
    classe: text("classe").notNull(),
    dateSeance: date("date_seance").notNull(),
    /** preparee | tenue | close */
    statut: text("statut").notNull().default("preparee"),
    pv: text("pv"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("conseil_seance_classe_idx").on(t.etablissementId, t.classe, t.dateSeance),
    uniqueIndex("conseil_seance_uidx").on(t.etablissementId, t.classe, t.dateSeance),
    check("conseil_seance_statut_chk", sql`${t.statut} in ('preparee', 'tenue', 'close')`),
  ],
);

export const conseilAvis = pgTable(
  "conseil_avis",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    seanceId: uuid("seance_id")
      .notNull()
      .references(() => conseilSeance.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    /** Texte libre : la liste des décisions est un réglage, pas un enum figé. */
    decision: text("decision"),
    mention: text("mention"),
    appreciation: text("appreciation"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("conseil_avis_eleve_uidx").on(t.etablissementId, t.seanceId, t.eleveId),
    index("conseil_avis_eleve_idx").on(t.etablissementId, t.eleveId),
  ],
);

/** Bulletin figé d’une période. La publication ne réécrit pas les notes. */
export const bulletin = pgTable(
  "bulletin",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    periodeId: uuid("periode_id")
      .notNull()
      .references(() => notePeriode.id, { onDelete: "restrict" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    /** brouillon | verrouille | publie */
    statut: text("statut").notNull().default("brouillon"),
    appreciationGenerale: text("appreciation_generale"),
    publieAt: timestamp("publie_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("bulletin_eleve_periode_uidx").on(t.etablissementId, t.eleveId, t.periodeId),
    index("bulletin_statut_idx").on(t.etablissementId, t.statut),
    check("bulletin_statut_chk", sql`${t.statut} in ('brouillon', 'verrouille', 'publie')`),
  ],
);

/** Appréciation et moyenne copiées au verrouillage. Distinct de note_moyenne_eleve (cache vivant). */
export const bulletinLigne = pgTable(
  "bulletin_ligne",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    bulletinId: uuid("bulletin_id")
      .notNull()
      .references(() => bulletin.id, { onDelete: "cascade" }),
    matiereId: uuid("matiere_id").references(() => noteMatiere.id, { onDelete: "set null" }),
    libelle: text("libelle").notNull(),
    appreciation: text("appreciation"),
    moyenneFigee: numeric("moyenne_figee", { precision: 5, scale: 2 }),
    ordre: integer("ordre").notNull().default(1),
  },
  (t) => [
    index("bulletin_ligne_bulletin_idx").on(t.etablissementId, t.bulletinId),
    uniqueIndex("bulletin_ligne_matiere_uidx")
      .on(t.bulletinId, t.matiereId)
      .where(sql`${t.matiereId} is not null`),
  ],
);

/** Leçon et travail à faire. Distinct du devoir noté (`note_devoir`). */
export const cahierTexte = pgTable(
  "cahier_texte",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id"),
    dateSeance: date("date_seance").notNull(),
    classe: text("classe"),
    groupeId: uuid("groupe_id"),
    creneauId: uuid("creneau_id").references(() => edtCreneau.id, { onDelete: "set null" }),
    matiereLibelle: text("matiere_libelle"),
    contenu: text("contenu").notNull().default(""),
    travail: text("travail").notNull().default(""),
    aRendreLe: date("a_rendre_le"),
    enseignantUserId: text("enseignant_user_id"),
    enseignantNom: text("enseignant_nom"),
    visibleFamille: boolean("visible_famille").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("cahier_texte_date_idx").on(t.etablissementId, t.dateSeance),
    index("cahier_texte_classe_idx").on(t.etablissementId, t.classe, t.dateSeance),
    index("cahier_texte_creneau_idx").on(t.etablissementId, t.creneauId),
  ],
);

/**
 * Décision de bascule. Les scolarités passées restent.
 * Valider ne supprime rien : le rituel (backend) fait passer `prevue` → `en_cours`.
 */
export const anneeBascule = pgTable(
  "annee_bascule",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    anneeSourceId: uuid("annee_source_id").notNull(),
    anneeCibleId: uuid("annee_cible_id").notNull(),
    /** preparee | validee | annulee */
    statut: text("statut").notNull().default("preparee"),
    valideParUserId: text("valide_par_user_id"),
    valideAt: timestamp("valide_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("annee_bascule_etab_idx").on(t.etablissementId, t.statut),
    uniqueIndex("annee_bascule_one_preparee_uidx")
      .on(t.etablissementId)
      .where(sql`${t.statut} = 'preparee'`),
    check("annee_bascule_annees_chk", sql`${t.anneeSourceId} <> ${t.anneeCibleId}`),
    check("annee_bascule_statut_chk", sql`${t.statut} in ('preparee', 'validee', 'annulee')`),
  ],
);

export const internatBatiment = pgTable(
  "internat_batiment",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    label: text("label").notNull(),
    notes: text("notes"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("internat_batiment_label_uidx").on(t.etablissementId, t.label),
  ],
);

export const internatChambre = pgTable(
  "internat_chambre",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    batimentId: uuid("batiment_id")
      .notNull()
      .references(() => internatBatiment.id, { onDelete: "cascade" }),
    label: text("label").notNull(),
    etage: text("etage"),
    capacite: integer("capacite").notNull().default(2),
    /** garcons | filles | mixte */
    aile: text("aile"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("internat_chambre_label_uidx").on(t.etablissementId, t.batimentId, t.label),
    index("internat_chambre_batiment_idx").on(t.etablissementId, t.batimentId),
    check("internat_chambre_capacite_chk", sql`${t.capacite} between 1 and 8`),
  ],
);

/** Lit daté. Une affectation ouverte (date_fin nulle) par élève. */
export const internatAffectation = pgTable(
  "internat_affectation",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    chambreId: uuid("chambre_id")
      .notNull()
      .references(() => internatChambre.id, { onDelete: "restrict" }),
    eleveId: uuid("eleve_id").notNull(),
    anneeScolaireId: uuid("annee_scolaire_id"),
    dateDebut: date("date_debut").notNull(),
    dateFin: date("date_fin"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("internat_affectation_chambre_idx").on(t.etablissementId, t.chambreId),
    index("internat_affectation_eleve_idx").on(t.etablissementId, t.eleveId),
    uniqueIndex("internat_affectation_open_uidx")
      .on(t.etablissementId, t.eleveId)
      .where(sql`${t.dateFin} is null`),
  ],
);

export const internatAppel = pgTable(
  "internat_appel",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    batimentId: uuid("batiment_id").references(() => internatBatiment.id, {
      onDelete: "set null",
    }),
    dateAppel: date("date_appel").notNull(),
    /** ouverte | validee */
    statut: text("statut").notNull().default("ouverte"),
    valideAt: timestamp("valide_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("internat_appel_date_idx").on(t.etablissementId, t.dateAppel),
    uniqueIndex("internat_appel_global_uidx")
      .on(t.etablissementId, t.dateAppel)
      .where(sql`${t.batimentId} is null`),
    uniqueIndex("internat_appel_batiment_uidx")
      .on(t.etablissementId, t.dateAppel, t.batimentId)
      .where(sql`${t.batimentId} is not null`),
    check("internat_appel_statut_chk", sql`${t.statut} in ('ouverte', 'validee')`),
  ],
);

export const internatAppelLigne = pgTable(
  "internat_appel_ligne",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    appelId: uuid("appel_id")
      .notNull()
      .references(() => internatAppel.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    /** present | absent | excuse | activite */
    marque: text("marque").notNull().default("present"),
    note: text("note"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("internat_appel_ligne_uidx").on(t.etablissementId, t.appelId, t.eleveId),
    check(
      "internat_appel_ligne_marque_chk",
      sql`${t.marque} in ('present', 'absent', 'excuse', 'activite')`,
    ),
  ],
);

/** Sortie d’internat (week-end, correspondant). Distinct d’une sortie scolaire. */
export const internatSortie = pgTable(
  "internat_sortie",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id").notNull(),
    dateDebut: date("date_debut").notNull(),
    dateFin: date("date_fin").notNull(),
    motif: text("motif"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("internat_sortie_eleve_idx").on(t.etablissementId, t.eleveId, t.dateDebut),
    check("internat_sortie_dates_chk", sql`${t.dateFin} >= ${t.dateDebut}`),
  ],
);

/** Période de paie établissement. Isolée des factures familles. Pas de DSN. */
export const paiePeriode = pgTable(
  "paie_periode",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    label: text("label").notNull(),
    dateDebut: date("date_debut").notNull(),
    dateFin: date("date_fin").notNull(),
    /** brouillon | figee */
    statut: text("statut").notNull().default("brouillon"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("paie_periode_label_uidx").on(t.etablissementId, t.label),
    check("paie_periode_statut_chk", sql`${t.statut} in ('brouillon', 'figee')`),
    check("paie_periode_dates_chk", sql`${t.dateFin} >= ${t.dateDebut}`),
  ],
);

/**
 * Élément de paie (absence, heure, prime, retenue).
 * Le montant peut rester vide : l’établissement suit parfois seulement les heures.
 */
export const paieElement = pgTable(
  "paie_element",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    periodeId: uuid("periode_id")
      .notNull()
      .references(() => paiePeriode.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    /** absence | heure | prime | retenue */
    nature: text("nature").notNull(),
    libelle: text("libelle").notNull(),
    quantite: numeric("quantite", { precision: 10, scale: 2 }),
    montant: numeric("montant", { precision: 12, scale: 2 }),
    absenceId: text("absence_id"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("paie_element_periode_idx").on(t.etablissementId, t.periodeId),
    index("paie_element_personnel_idx").on(t.etablissementId, t.personnelId),
    check(
      "paie_element_nature_chk",
      sql`${t.nature} in ('absence', 'heure', 'prime', 'retenue')`,
    ),
  ],
);

/**
 * Trace d’un envoi académique (LSU, LSL, Siècle, STS).
 * Pas une copie des notes : l’export lit les compétences et la scolarité.
 */
export const exportAcademique = pgTable(
  "export_academique",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** lsu | lsl | siecle | sts */
    canal: text("canal").notNull(),
    anneeScolaireId: uuid("annee_scolaire_id"),
    periodeId: uuid("periode_id").references(() => notePeriode.id, { onDelete: "set null" }),
    /** brouillon | envoye */
    statut: text("statut").notNull().default("brouillon"),
    genereAt: timestamp("genere_at", { withTimezone: true }),
    envoyeAt: timestamp("envoye_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("export_academique_canal_idx").on(t.etablissementId, t.canal, t.statut),
    check("export_academique_canal_chk", sql`${t.canal} in ('lsu', 'lsl', 'siecle', 'sts')`),
    check("export_academique_statut_chk", sql`${t.statut} in ('brouillon', 'envoye')`),
  ],
);

/** Échéances d’une facture famille (mensuel, trimestriel). La date unique reste sur `facture`. */
export const factureEcheance = pgTable(
  "facture_echeance",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    factureId: uuid("facture_id")
      .notNull()
      .references(() => facture.id, { onDelete: "cascade" }),
    dateEcheance: date("date_echeance").notNull(),
    montant: numeric("montant", { precision: 12, scale: 2 }).notNull(),
    ordre: integer("ordre").notNull().default(1),
  },
  (t) => [
    index("facture_echeance_facture_idx").on(t.etablissementId, t.factureId),
    uniqueIndex("facture_echeance_ordre_uidx").on(t.factureId, t.ordre),
  ],
);

/** Caisse ou banque de l’établissement. Pas un compte du plan comptable. */
export const tresorerieCompte = pgTable(
  "tresorerie_compte",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    libelle: text("libelle").notNull(),
    /** caisse | banque */
    nature: text("nature").notNull(),
    actif: boolean("actif").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("tresorerie_compte_libelle_uidx").on(t.etablissementId, t.libelle),
    check("tresorerie_compte_nature_chk", sql`${t.nature} in ('caisse', 'banque')`),
  ],
);

/** Dépense de l’établissement (voyage, cantine, internat, fourniture). */
export const depense = pgTable(
  "depense",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    dateDepense: date("date_depense").notNull(),
    libelle: text("libelle").notNull(),
    fournisseur: text("fournisseur"),
    /** cantine | internat | voyage | scolarite | autre */
    portee: text("portee").notNull().default("autre"),
    montant: numeric("montant", { precision: 12, scale: 2 }).notNull(),
    /** prevue | payee */
    statut: text("statut").notNull().default("prevue"),
    travelId: text("travel_id"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("depense_date_idx").on(t.etablissementId, t.dateDepense),
    index("depense_portee_idx").on(t.etablissementId, t.portee),
    check(
      "depense_portee_chk",
      sql`${t.portee} in ('cantine', 'internat', 'voyage', 'scolarite', 'autre')`,
    ),
    check("depense_statut_chk", sql`${t.statut} in ('prevue', 'payee')`),
  ],
);

/**
 * Livre de caisse / banque. Partie simple : une entrée ou une sortie.
 * Relié à un encaissement famille ou à une dépense quand il en vient.
 */
export const mouvementTresorerie = pgTable(
  "mouvement_tresorerie",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    compteId: uuid("compte_id")
      .notNull()
      .references(() => tresorerieCompte.id, { onDelete: "restrict" }),
    dateMouvement: date("date_mouvement").notNull(),
    /** entree | sortie */
    sens: text("sens").notNull(),
    montant: numeric("montant", { precision: 12, scale: 2 }).notNull(),
    libelle: text("libelle").notNull(),
    encaissementId: uuid("encaissement_id").references(() => encaissement.id, {
      onDelete: "set null",
    }),
    depenseId: uuid("depense_id").references(() => depense.id, { onDelete: "set null" }),
    factureId: uuid("facture_id").references(() => facture.id, { onDelete: "set null" }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("mouvement_tresorerie_compte_idx").on(t.etablissementId, t.compteId, t.dateMouvement),
    index("mouvement_tresorerie_depense_idx").on(t.etablissementId, t.depenseId),
    check("mouvement_tresorerie_sens_chk", sql`${t.sens} in ('entree', 'sortie')`),
    check("mouvement_tresorerie_montant_chk", sql`${t.montant} > 0`),
  ],
);

export const socleFaitsSchema = {
  factureEcheance,
  tresorerieCompte,
  depense,
  mouvementTresorerie,
  passage,
  infirmeriePassage,
  santeExtrait,
  infirmerieFiche,
  santeMedicamentPrise,
  santeAccident,
  santePai,
  santeInaptitudeEps,
  conseilSeance,
  conseilAvis,
  bulletin,
  bulletinLigne,
  cahierTexte,
  anneeBascule,
  internatBatiment,
  internatChambre,
  internatAffectation,
  internatAppel,
  internatAppelLigne,
  internatSortie,
  paiePeriode,
  paieElement,
  exportAcademique,
};
