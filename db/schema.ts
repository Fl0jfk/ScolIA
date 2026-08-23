import { sql } from "drizzle-orm";
import {
  boolean,
  date,
  index,
  jsonb,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

export { etablissement };
export * from "./schema-ent-relational";

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

/** Élève du référentiel ENT (ex-eleves.json). */
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
    mef: text("mef"),
    secteur: text("secteur"),
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
};

export const appSchema = {
  etablissement,
  userRole,
  authUserMapping,
  anneeScolaire,
  etablissementSite,
  eleve,
  schoolRosterMeta,
  schoolClassAssignment,
  personnel,
  entEntity,
  tenantDocument,
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
export type PersonnelRow = typeof personnel.$inferSelect;
export type EntEntityRow = typeof entEntity.$inferSelect;
export type TenantDocumentRow = typeof tenantDocument.$inferSelect;
