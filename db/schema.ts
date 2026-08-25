import { sql } from "drizzle-orm";
import {
  bigint,
  boolean,
  date,
  index,
  integer,
  jsonb,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";
import { charlemagneP1Schema } from "./schema-charlemagne-p1";
import { charlemagneP2Schema } from "./schema-charlemagne-p2";
import { charlemagneP3Schema } from "./schema-charlemagne-p3";
import { charlemagneP4Schema } from "./schema-charlemagne-p4";
import { charlemagneP5Schema } from "./schema-charlemagne-p5";

export { etablissement };
export * from "./schema-ent-relational";
export * from "./schema-charlemagne-p1";
export * from "./schema-charlemagne-p2";
export * from "./schema-charlemagne-p3";
export * from "./schema-charlemagne-p4";
export * from "./schema-charlemagne-p5";

/** Utilisateur Better-Auth (multi-tenant via etablissement_id). */
export const user = pgTable(
  "user",
  {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    email: text("email").notNull(),
    emailVerified: boolean("email_verified").notNull().default(false),
    image: text("image"),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Id métier historique (chemins S3 / données existantes). */
    externalUserId: text("external_user_id"),
    firstName: text("first_name"),
    lastName: text("last_name"),
    platformAdmin: boolean("platform_admin").notNull().default(false),
    orgAdmin: boolean("org_admin").notNull().default(false),
    /** true = MDP provisoire / migration — forcer le changement avant l’intranet. */
    mustChangePassword: boolean("must_change_password").notNull().default(true),
    /** Plugin Better-Auth twoFactor. */
    twoFactorEnabled: boolean("two_factor_enabled").notNull().default(false),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("user_email_etablissement_uidx").on(t.email, t.etablissementId),
    index("user_etablissement_idx").on(t.etablissementId),
    index("user_external_user_id_idx").on(t.externalUserId),
  ],
);

export const session = pgTable(
  "session",
  {
    id: text("id").primaryKey(),
    expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
    token: text("token").notNull().unique(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    ipAddress: text("ip_address"),
    userAgent: text("user_agent"),
    userId: text("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
  },
  (t) => [index("session_user_id_idx").on(t.userId)],
);

export const account = pgTable(
  "account",
  {
    id: text("id").primaryKey(),
    /** Better-Auth ≥1.7 : identité scoped par issuer (ex. local:credential). */
    issuer: text("issuer").notNull().default("local:credential"),
    accountId: text("account_id").notNull(),
    providerId: text("provider_id").notNull(),
    userId: text("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    accessToken: text("access_token"),
    refreshToken: text("refresh_token"),
    idToken: text("id_token"),
    accessTokenExpiresAt: timestamp("access_token_expires_at", { withTimezone: true }),
    refreshTokenExpiresAt: timestamp("refresh_token_expires_at", { withTimezone: true }),
    scope: text("scope"),
    password: text("password"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("account_user_id_idx").on(t.userId),
    uniqueIndex("account_issuer_account_id_uidx").on(t.issuer, t.accountId),
  ],
);

export const verification = pgTable("verification", {
  id: text("id").primaryKey(),
  identifier: text("identifier").notNull(),
  value: text("value").notNull(),
  expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
});

/** Secrets TOTP / codes de secours (plugin Better-Auth twoFactor). */
export const twoFactor = pgTable(
  "two_factor",
  {
    id: text("id").primaryKey(),
    secret: text("secret").notNull(),
    backupCodes: text("backup_codes").notNull(),
    userId: text("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    verified: boolean("verified").default(true),
    failedVerificationCount: integer("failed_verification_count").default(0),
    lockedUntil: timestamp("locked_until", { withTimezone: true }),
  },
  (t) => [
    index("two_factor_user_id_idx").on(t.userId),
    index("two_factor_secret_idx").on(t.secret),
  ],
);

/** Rate-limit Better-Auth (stockage database multi-réplicas). */
export const rateLimit = pgTable(
  "rate_limit",
  {
    id: text("id").primaryKey(),
    key: text("key").notNull().unique(),
    count: integer("count").notNull(),
    lastRequest: bigint("last_request", { mode: "number" }).notNull(),
  },
  (t) => [uniqueIndex("rate_limit_key_uidx").on(t.key)],
);

/** Rate-limit applicatif (hors routes Better-Auth). */
export const appRateLimit = pgTable("app_rate_limit", {
  key: text("key").primaryKey(),
  count: integer("count").notNull().default(0),
  resetAt: timestamp("reset_at", { withTimezone: true }).notNull(),
});

/** Journal d’audit sécurité compte (MDP, e-mail, 2FA). */
export const securityAuditLog = pgTable(
  "security_audit_log",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    userId: text("user_id").references(() => user.id, { onDelete: "set null" }),
    action: text("action").notNull(),
    ipAddress: text("ip_address"),
    userAgent: text("user_agent"),
    metadata: jsonb("metadata").$type<Record<string, unknown> | null>(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("security_audit_log_user_idx").on(t.userId),
    index("security_audit_log_created_idx").on(t.createdAt),
  ],
);

/** Rôles intranet (RBAC). */
export const userRole = pgTable(
  "user_role",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    userId: text("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    role: text("role").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("user_role_etablissement_user_role_uidx").on(t.etablissementId, t.userId, t.role),
    index("user_role_user_idx").on(t.userId),
  ],
);

/**
 * Rattachement compte ↔ établissement (1 login, N contextes).
 * Source de vérité pour savoir où une personne peut se connecter — pas un cookie.
 */
export const userMembership = pgTable(
  "user_membership",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    userId: text("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** staff | parent | eleve */
    context: text("context").notNull().default("staff"),
    active: boolean("active").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("user_membership_user_etablissement_uidx").on(t.userId, t.etablissementId),
    index("user_membership_user_idx").on(t.userId),
    index("user_membership_etablissement_idx").on(t.etablissementId),
  ],
);

/** Correspondance id métier historique → user interne. */
export const authUserMapping = pgTable(
  "auth_user_mapping",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    externalUserId: text("external_user_id").notNull(),
    userId: text("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    migratedAt: timestamp("migrated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("auth_user_mapping_external_uidx").on(t.etablissementId, t.externalUserId),
    uniqueIndex("auth_user_mapping_user_uidx").on(t.userId),
  ],
);

/** Année scolaire (référence partagée par les modules ENT). */
export const anneeScolaire = pgTable(
  "annee_scolaire",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    label: text("label").notNull(),
    startsOn: date("starts_on"),
    endsOn: date("ends_on"),
    isCurrent: boolean("is_current").notNull().default(false),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("annee_scolaire_etablissement_label_uidx").on(t.etablissementId, t.label),
    index("annee_scolaire_etablissement_idx").on(t.etablissementId),
  ],
);

/**
 * Site scolaire dans le tenant (école / collège / lycée…).
 * Distinct de `etablissement` (tenant SaaS).
 */
export const etablissementSite = pgTable(
  "etablissement_site",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Id métier historique (ex. ecole, college, lycee). */
    siteId: text("site_id").notNull(),
    label: text("label").notNull(),
    kind: text("kind"),
    directorName: text("director_name"),
    directorEmail: text("director_email"),
    directorExternalUserId: text("director_external_user_id"),
    colorHex: text("color_hex"),
    signatureS3Key: text("signature_s3_key"),
    grades: text("grades"),
    roleSlugs: text("role_slugs").array().notNull().default([]),
    active: boolean("active").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.siteId], name: "etablissement_site_pk" }),
    index("etablissement_site_etablissement_idx").on(t.etablissementId),
  ],
);

/** Élève du référentiel ENT (ex-eleves.json) — identité stable multi-années / multi-sites. */
export const eleve = pgTable(
  "eleve",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Clé de fusion import : ine:XXX ou person:nom§prenom. */
    sourceKey: text("source_key").notNull(),
    ine: text("ine"),
    nom: text("nom").notNull(),
    prenom: text("prenom").notNull(),
    folderName: text("folder_name").notNull(),
    classe: text("classe"),
    email: text("email"),
    parentEmail: text("parent_email"),
    parent1Email: text("parent1_email"),
    parent2Email: text("parent2_email"),
    parentPhone: text("parent_phone"),
    parent1Phone: text("parent1_phone"),
    parent2Phone: text("parent2_phone"),
    dateNaissance: date("date_naissance"),
    lieuNaissance: text("lieu_naissance"),
    /** preinscrit | inscrit | ancien | archive */
    status: text("status").notNull().default("inscrit"),
    mef: text("mef"),
    secteur: text("secteur"),
    /** Régime scolaire (INT / DP / EXT ou libellé). */
    regime: text("regime"),
    /** Sexe M/F. */
    sexe: text("sexe"),
    /** Clé objet photo (S3 / storage) — eleves/photos/…. */
    photoKey: text("photo_key"),
    pilotageKey: text("pilotage_key"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("eleve_etablissement_source_uidx").on(t.etablissementId, t.sourceKey),
    uniqueIndex("eleve_etablissement_ine_uidx")
      .on(t.etablissementId, t.ine)
      .where(sql`${t.ine} is not null and ${t.ine} <> ''`),
    index("eleve_etablissement_classe_idx").on(t.etablissementId, t.classe),
    index("eleve_etablissement_idx").on(t.etablissementId),
    index("eleve_etablissement_status_idx").on(t.etablissementId, t.status),
  ],
);

/** Scolarité par année / site — continuité groupe = même eleve_id. */
export const eleveScolarite = pgTable(
  "eleve_scolarite",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id")
      .notNull()
      .references(() => eleve.id, { onDelete: "cascade" }),
    anneeScolaireId: uuid("annee_scolaire_id").references(() => anneeScolaire.id, {
      onDelete: "set null",
    }),
    siteId: text("site_id"),
    classe: text("classe"),
    /** en_cours | prevue | terminee | annulee */
    statut: text("statut").notNull().default("en_cours"),
    demiPension: boolean("demi_pension").notNull().default(false),
    repasParSemaine: integer("repas_par_semaine"),
    /** Grille L–V : midi / soir / étude / garderie / sort seul (Passage). */
    grilleRepas: jsonb("grille_repas").$type<Record<string, unknown> | null>(),
    etablissementPrecedent: text("etablissement_precedent"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("eleve_scolarite_eleve_idx").on(t.etablissementId, t.eleveId),
    index("eleve_scolarite_site_idx").on(t.etablissementId, t.siteId),
    index("eleve_scolarite_annee_idx").on(t.etablissementId, t.anneeScolaireId),
  ],
);

export const foyer = pgTable(
  "foyer",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    label: text("label").notNull().default("Foyer"),
    adresse: text("adresse"),
    codePostal: text("code_postal"),
    ville: text("ville"),
    payeurEstFoyer: boolean("payeur_est_foyer").notNull().default(true),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("foyer_etablissement_idx").on(t.etablissementId)],
);

export const foyerResponsable = pgTable(
  "foyer_responsable",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id")
      .notNull()
      .references(() => foyer.id, { onDelete: "cascade" }),
    nom: text("nom").notNull(),
    prenom: text("prenom").notNull(),
    email: text("email"),
    telephone: text("telephone"),
    autoriteParentale: boolean("autorite_parentale").notNull().default(false),
    contactUrgence: boolean("contact_urgence").notNull().default(false),
    payeur: boolean("payeur").notNull().default(false),
    rang: integer("rang").notNull().default(1),
    userId: text("user_id").references(() => user.id, { onDelete: "set null" }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("foyer_responsable_foyer_idx").on(t.etablissementId, t.foyerId),
    uniqueIndex("foyer_responsable_foyer_rang_uidx").on(t.foyerId, t.rang),
  ],
);

export const eleveFoyerLink = pgTable(
  "eleve_foyer_link",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id")
      .notNull()
      .references(() => eleve.id, { onDelete: "cascade" }),
    foyerId: uuid("foyer_id")
      .notNull()
      .references(() => foyer.id, { onDelete: "cascade" }),
    relation: text("relation").notNull().default("principal"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.eleveId, t.foyerId],
      name: "eleve_foyer_link_pk",
    }),
    index("eleve_foyer_link_eleve_idx").on(t.etablissementId, t.eleveId),
  ],
);

export const eleveDocument = pgTable(
  "eleve_document",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    eleveId: uuid("eleve_id")
      .notNull()
      .references(() => eleve.id, { onDelete: "cascade" }),
    tiroir: text("tiroir").notNull(),
    title: text("title").notNull(),
    mimeType: text("mime_type"),
    s3Key: text("s3_key"),
    fileUrl: text("file_url"),
    anneeLabel: text("annee_label"),
    confidentialite: text("confidentialite").notNull().default("standard"),
    source: text("source").notNull().default("upload"),
    createdByUserId: text("created_by_user_id").references(() => user.id, {
      onDelete: "set null",
    }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("eleve_document_eleve_idx").on(t.etablissementId, t.eleveId),
    index("eleve_document_tiroir_idx").on(t.etablissementId, t.eleveId, t.tiroir),
  ],
);

export const documentAccessRequest = pgTable(
  "document_access_request",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    documentId: uuid("document_id")
      .notNull()
      .references(() => eleveDocument.id, { onDelete: "cascade" }),
    requesterUserId: text("requester_user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    status: text("status").notNull().default("pending"),
    durationDays: integer("duration_days").notNull().default(1),
    decidedByUserId: text("decided_by_user_id").references(() => user.id, {
      onDelete: "set null",
    }),
    decidedAt: timestamp("decided_at", { withTimezone: true }),
    expiresAt: timestamp("expires_at", { withTimezone: true }),
    note: text("note"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("document_access_request_doc_idx").on(t.etablissementId, t.documentId),
    index("document_access_request_status_idx").on(t.etablissementId, t.status),
  ],
);

export const eleveAccessAudit = pgTable(
  "eleve_access_audit",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    actorUserId: text("actor_user_id").references(() => user.id, { onDelete: "set null" }),
    resourceType: text("resource_type").notNull(),
    resourceId: text("resource_id").notNull(),
    eleveId: uuid("eleve_id").references(() => eleve.id, { onDelete: "set null" }),
    action: text("action").notNull(),
    metadata: jsonb("metadata"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("eleve_access_audit_etab_idx").on(t.etablissementId, t.createdAt),
    index("eleve_access_audit_eleve_idx").on(t.etablissementId, t.eleveId),
  ],
);

/** Journal RGPD — lectures / exports de données sensibles (listes massives, RH, documents). */
export const dataAccessAudit = pgTable(
  "data_access_audit",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    userId: text("user_id").references(() => user.id, { onDelete: "set null" }),
    resourceType: text("resource_type").notNull(),
    resourceId: text("resource_id"),
    action: text("action").notNull(),
    ipAddress: text("ip_address"),
    userAgent: text("user_agent"),
    metadata: jsonb("metadata").$type<Record<string, unknown> | null>(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("data_access_audit_etab_created_idx").on(t.etablissementId, t.createdAt),
    index("data_access_audit_resource_idx").on(t.etablissementId, t.resourceType, t.createdAt),
  ],
);

export const preinscription = pgTable(
  "preinscription",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    siteId: text("site_id"),
    niveauVise: text("niveau_vise"),
    filiereVisee: text("filiere_visee"),
    nom: text("nom").notNull(),
    prenom: text("prenom").notNull(),
    dateNaissance: date("date_naissance"),
    lieuNaissance: text("lieu_naissance"),
    demiPension: boolean("demi_pension").notNull().default(false),
    etablissementPrecedent: text("etablissement_precedent"),
    status: text("status").notNull().default("pending"),
    payload: jsonb("payload"),
    eleveId: uuid("eleve_id").references(() => eleve.id, { onDelete: "set null" }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("preinscription_etab_idx").on(t.etablissementId, t.status),
    index("preinscription_site_idx").on(t.etablissementId, t.siteId),
  ],
);

/** Meta roster enseignants (catalog + horodatage). */
export const schoolRosterMeta = pgTable("school_roster_meta", {
  etablissementId: uuid("etablissement_id")
    .primaryKey()
    .references(() => etablissement.id, { onDelete: "cascade" }),
  teacherCatalog: text("teacher_catalog").array().notNull().default([]),
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  updatedBy: text("updated_by"),
});

/** Affectation professeur ↔ classe (roster). */
export const schoolClassAssignment = pgTable(
  "school_class_assignment",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    className: text("class_name").notNull(),
    classKey: text("class_key").notNull(),
    externalUserId: text("external_user_id").notNull(),
    name: text("name").notNull(),
    email: text("email").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("school_class_assignment_class_uidx").on(t.etablissementId, t.classKey),
    index("school_class_assignment_etablissement_idx").on(t.etablissementId),
    index("school_class_assignment_external_idx").on(t.etablissementId, t.externalUserId),
  ],
);

/** Meta EDT professeur (semaines types A/B + remplacements). */
export const teacherPlanning = pgTable(
  "teacher_planning",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    /** Id métier (Better-Auth business / external) — clé historique S3. */
    externalUserId: text("external_user_id").notNull(),
    source: text("source"),
    sourceFileName: text("source_file_name"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    updatedBy: text("updated_by").notNull().default(""),
  },
  (t) => [
    uniqueIndex("teacher_planning_etab_user_uidx").on(t.etablissementId, t.externalUserId),
    index("teacher_planning_etablissement_idx").on(t.etablissementId),
  ],
);

/** Créneau semaine type A ou B. */
export const teacherPlanningSlot = pgTable(
  "teacher_planning_slot",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    planningId: uuid("planning_id")
      .notNull()
      .references(() => teacherPlanning.id, { onDelete: "cascade" }),
    /** A | B */
    weekType: text("week_type").notNull(),
    /** 1–5 (lun–ven) */
    day: integer("day").notNull(),
    startTime: text("start_time").notNull(),
    endTime: text("end_time").notNull(),
    subject: text("subject").notNull(),
    classes: text("classes").array().notNull().default([]),
    room: text("room"),
  },
  (t) => [
    index("teacher_planning_slot_planning_idx").on(t.planningId, t.weekType),
    index("teacher_planning_slot_etab_idx").on(t.etablissementId),
  ],
);

/** Remplacement daté (hors semaine type). */
export const teacherPlanningReplacement = pgTable(
  "teacher_planning_replacement",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    planningId: uuid("planning_id")
      .notNull()
      .references(() => teacherPlanning.id, { onDelete: "cascade" }),
    date: date("date").notNull(),
    startTime: text("start_time").notNull(),
    endTime: text("end_time").notNull(),
    subject: text("subject").notNull(),
    classes: text("classes").array().notNull().default([]),
    room: text("room"),
    note: text("note"),
    createdBy: text("created_by").notNull().default(""),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("teacher_planning_repl_planning_idx").on(t.planningId, t.date),
    index("teacher_planning_repl_etab_date_idx").on(t.etablissementId, t.date),
  ],
);

/**
 * Fiche personnel OGEC (cœur).
 * Nested RH → tables filles (personnel_document, personnel_attr…).
 * Colonne payload conservée temporairement pour migration, puis droppée.
 */
export const personnel = pgTable(
  "personnel",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    externalUserId: text("external_user_id"),
    email: text("email").notNull().default(""),
    emailPerso: text("email_perso"),
    emailPro: text("email_pro"),
    firstName: text("first_name").notNull().default(""),
    lastName: text("last_name").notNull().default(""),
    displayName: text("display_name").notNull().default(""),
    category: text("category").notNull(),
    jobTitle: text("job_title"),
    hireDate: text("hire_date"),
    active: boolean("active").notNull().default(true),
    establishmentLabel: text("establishment_label"),
    managerId: text("manager_id"),
    /** @deprecated migrer vers tables filles puis DROP */
    payload: jsonb("payload").$type<Record<string, unknown>>().notNull().default({}),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("personnel_etablissement_idx").on(t.etablissementId),
    uniqueIndex("personnel_etablissement_external_uidx")
      .on(t.etablissementId, t.externalUserId)
      .where(sql`${t.externalUserId} is not null and ${t.externalUserId} <> ''`),
    index("personnel_etablissement_email_idx").on(t.etablissementId, t.email),
  ],
);

/**
 * @deprecated Droppé en migration 0007 — types conservés pour scripts legacy uniquement.
 * Ne plus utiliser en runtime.
 */
export const entEntity = pgTable(
  "ent_entity",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    kind: text("kind").notNull(),
    recordId: text("record_id").notNull(),
    status: text("status"),
    payload: jsonb("payload").$type<Record<string, unknown>>().notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.kind, t.recordId], name: "ent_entity_pk" }),
    index("ent_entity_kind_idx").on(t.etablissementId, t.kind),
    index("ent_entity_kind_status_idx").on(t.etablissementId, t.kind, t.status),
  ],
);

/**
 * @deprecated Droppé en migration 0007.
 */
export const tenantDocument = pgTable(
  "tenant_document",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    docKey: text("doc_key").notNull(),
    payload: jsonb("payload").$type<unknown>().notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.docKey], name: "tenant_document_pk" }),
    index("tenant_document_etablissement_idx").on(t.etablissementId),
  ],
);

export const authSchema = {
  user,
  session,
  account,
  verification,
  twoFactor,
  rateLimit,
};

export const appSchema = {
  etablissement,
  userRole,
  userMembership,
  authUserMapping,
  anneeScolaire,
  etablissementSite,
  eleve,
  eleveScolarite,
  foyer,
  foyerResponsable,
  eleveFoyerLink,
  eleveDocument,
  documentAccessRequest,
  eleveAccessAudit,
  dataAccessAudit,
  preinscription,
  schoolRosterMeta,
  schoolClassAssignment,
  teacherPlanning,
  teacherPlanningSlot,
  teacherPlanningReplacement,
  personnel,
  entEntity,
  tenantDocument,
  ...charlemagneP1Schema,
  ...charlemagneP2Schema,
  ...charlemagneP3Schema,
  ...charlemagneP4Schema,
  ...charlemagneP5Schema,
};

export const schema = {
  ...authSchema,
  ...appSchema,
};

export type EtablissementRow = typeof etablissement.$inferSelect;
export type UserRow = typeof user.$inferSelect;
export type UserRoleRow = typeof userRole.$inferSelect;
export type AnneeScolaireRow = typeof anneeScolaire.$inferSelect;
export type EtablissementSiteRow = typeof etablissementSite.$inferSelect;
export type EleveRow = typeof eleve.$inferSelect;
export type EleveScolariteRow = typeof eleveScolarite.$inferSelect;
export type FoyerRow = typeof foyer.$inferSelect;
export type FoyerResponsableRow = typeof foyerResponsable.$inferSelect;
export type EleveDocumentRow = typeof eleveDocument.$inferSelect;
export type PreinscriptionRow = typeof preinscription.$inferSelect;
export type PersonnelRow = typeof personnel.$inferSelect;
export type EntEntityRow = typeof entEntity.$inferSelect;
export type TenantDocumentRow = typeof tenantDocument.$inferSelect;
