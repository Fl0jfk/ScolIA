/**
 * Tables métier ENT relationnelles (zéro JSON fichier / zéro payload jsonb).
 * Les structures imbriquées passent par tables filles ou attributs EAV textuels.
 */
import {
  boolean,
  date,
  doublePrecision,
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

/** Enseignant référentiel (ex-enseignants.json). */
export const enseignant = pgTable(
  "enseignant",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    nom: text("nom").notNull(),
    prenom: text("prenom").notNull(),
    folderName: text("folder_name").notNull(),
    secteur: text("secteur").notNull(),
    email: text("email"),
    emailPro: text("email_pro"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    index("enseignant_etablissement_idx").on(t.etablissementId),
    index("enseignant_etablissement_secteur_idx").on(t.etablissementId, t.secteur),
  ],
);

/** Absence personnel / OGEC. */
export const absence = pgTable(
  "absence",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull(),
    source: text("source").notNull(),
    displayName: text("display_name").notNull(),
    calendarVisible: boolean("calendar_visible").notNull().default(true),
    createdByUserId: text("created_by_user_id").notNull().default(""),
    createdByName: text("created_by_name").notNull().default(""),
    createdByEmail: text("created_by_email").notNull().default(""),
    createdByRoles: text("created_by_roles").array().notNull().default([]),
    scope: text("scope").notNull(),
    siteLabel: text("site_label"),
    periodType: text("period_type"),
    startDate: date("start_date").notNull(),
    endDate: date("end_date").notNull(),
    startTime: text("start_time"),
    endTime: text("end_time"),
    startAt: timestamp("start_at", { withTimezone: true }).notNull(),
    endAt: timestamp("end_at", { withTimezone: true }).notNull(),
    reason: text("reason").notNull().default(""),
    details: text("details").notNull().default(""),
    sourceDocument: text("source_document"),
    documentKeys: text("document_keys").array().notNull().default([]),
    confidence: doublePrecision("confidence"),
    workflowStatus: text("workflow_status").notNull(),
    managerDecision: text("manager_decision").notNull(),
    closedAt: timestamp("closed_at", { withTimezone: true }),
    justificationFileName: text("justification_file_name"),
    justificationFileUrl: text("justification_file_url"),
    justificationUploadedAt: timestamp("justification_uploaded_at", { withTimezone: true }),
    justificationUploadedBy: text("justification_uploaded_by"),
    managerNote: text("manager_note"),
    hoursTreatment: text("hours_treatment"),
    justificatifRelanceAt: timestamp("justificatif_relance_at", { withTimezone: true }),
    privacyReasonRedacted: boolean("privacy_reason_redacted").notNull().default(false),
    privacyDocumentsPurgedAt: timestamp("privacy_documents_purged_at", { withTimezone: true }),
    personnelId: text("personnel_id"),
    enseignantId: text("enseignant_id"),
    adminTreatedAt: timestamp("admin_treated_at", { withTimezone: true }),
    adminTreatedBy: text("admin_treated_by"),
    adminNote: text("admin_note"),
  },
  (t) => [
    index("absence_etablissement_idx").on(t.etablissementId),
    index("absence_etablissement_status_idx").on(t.etablissementId, t.workflowStatus),
    index("absence_etablissement_start_idx").on(t.etablissementId, t.startAt),
    index("absence_etablissement_personnel_idx").on(t.etablissementId, t.personnelId),
    index("absence_etablissement_enseignant_idx").on(t.etablissementId, t.enseignantId),
  ],
);

export const absenceHistory = pgTable(
  "absence_history",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    absenceId: text("absence_id")
      .notNull()
      .references(() => absence.id, { onDelete: "cascade" }),
    at: timestamp("at", { withTimezone: true }).notNull(),
    by: text("by").notNull().default(""),
    action: text("action").notNull(),
    note: text("note"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [
    index("absence_history_absence_idx").on(t.etablissementId, t.absenceId),
  ],
);

/** Voyage scolaire. */
export const travel = pgTable(
  "travel",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    type: text("type").notNull(),
    status: text("status").notNull(),
    ownerName: text("owner_name"),
    ownerEmail: text("owner_email"),
    ownerId: text("owner_id"),
    createdAt: timestamp("created_at", { withTimezone: true }),
    updatedAt: timestamp("updated_at", { withTimezone: true }),
    imageUrl: text("image_url"),
    imageConfigId: text("image_config_id"),
    title: text("title"),
    destination: text("destination"),
    siteLabel: text("site_label"),
    classes: text("classes"),
    startDate: text("start_date"),
    endDate: text("end_date"),
    startTime: text("start_time"),
    endTime: text("end_time"),
    nbEleves: text("nb_eleves"),
    nbAccompagnateurs: text("nb_accompagnateurs"),
    listeElevesStatus: text("liste_eleves_status"),
  },
  (t) => [
    index("travel_etablissement_idx").on(t.etablissementId),
    index("travel_etablissement_status_idx").on(t.etablissementId, t.status),
  ],
);

/** Attributs voyage (chemins plats → valeur textuelle) pour le nested non indexé. */
export const travelAttr = pgTable(
  "travel_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    travelId: text("travel_id")
      .notNull()
      .references(() => travel.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.travelId, t.path], name: "travel_attr_pk" }),
    index("travel_attr_travel_idx").on(t.etablissementId, t.travelId),
  ],
);

export const travelParticipant = pgTable(
  "travel_participant",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    travelId: text("travel_id")
      .notNull()
      .references(() => travel.id, { onDelete: "cascade" }),
    eleveKey: text("eleve_key").notNull().default(""),
    nom: text("nom").notNull().default(""),
    prenom: text("prenom").notNull().default(""),
    classe: text("classe"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("travel_participant_travel_idx").on(t.etablissementId, t.travelId)],
);

export const travelHistory = pgTable(
  "travel_history",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    travelId: text("travel_id")
      .notNull()
      .references(() => travel.id, { onDelete: "cascade" }),
    at: text("at").notNull().default(""),
    by: text("by").notNull().default(""),
    action: text("action").notNull().default(""),
    note: text("note"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("travel_history_travel_idx").on(t.etablissementId, t.travelId)],
);

export const travelMessage = pgTable(
  "travel_message",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    travelId: text("travel_id")
      .notNull()
      .references(() => travel.id, { onDelete: "cascade" }),
    userLabel: text("user_label").notNull().default(""),
    role: text("role").notNull().default(""),
    body: text("body").notNull().default(""),
    at: text("at").notNull().default(""),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("travel_message_travel_idx").on(t.etablissementId, t.travelId)],
);

/** Demande interne. */
export const request = pgTable(
  "request",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull(),
    status: text("status").notNull(),
    category: text("category").notNull().default(""),
    subject: text("subject").notNull().default(""),
    description: text("description").notNull().default(""),
    requesterFirstName: text("requester_first_name").notNull().default(""),
    requesterLastName: text("requester_last_name").notNull().default(""),
    requesterFullName: text("requester_full_name").notNull().default(""),
    requesterEmail: text("requester_email").notNull().default(""),
    requesterPhone: text("requester_phone").notNull().default(""),
    requesterUserId: text("requester_user_id"),
    assignedUnit: text("assigned_unit").notNull().default(""),
    assignedRoleLabel: text("assigned_role_label").notNull().default(""),
    assignedEmail: text("assigned_email").notNull().default(""),
    assignedRouteId: text("assigned_route_id"),
    purgeAt: timestamp("purge_at", { withTimezone: true }),
  },
  (t) => [
    index("request_etablissement_idx").on(t.etablissementId),
    index("request_etablissement_status_idx").on(t.etablissementId, t.status),
  ],
);

export const requestAttr = pgTable(
  "request_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    requestId: text("request_id")
      .notNull()
      .references(() => request.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.requestId, t.path], name: "request_attr_pk" }),
  ],
);

export const requestComment = pgTable(
  "request_comment",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    requestId: text("request_id")
      .notNull()
      .references(() => request.id, { onDelete: "cascade" }),
    at: text("at").notNull().default(""),
    byName: text("by_name").notNull().default(""),
    byEmail: text("by_email").notNull().default(""),
    byUserId: text("by_user_id"),
    body: text("body").notNull().default(""),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("request_comment_request_idx").on(t.etablissementId, t.requestId)],
);

export const requestHistory = pgTable(
  "request_history",
  {
    id: uuid("id").primaryKey().defaultRandom(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    requestId: text("request_id")
      .notNull()
      .references(() => request.id, { onDelete: "cascade" }),
    at: text("at").notNull().default(""),
    by: text("by").notNull().default(""),
    action: text("action").notNull().default(""),
    note: text("note"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("request_history_request_idx").on(t.etablissementId, t.requestId)],
);

export const requestAttachment = pgTable(
  "request_attachment",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    requestId: text("request_id")
      .notNull()
      .references(() => request.id, { onDelete: "cascade" }),
    commentId: text("comment_id"),
    fileName: text("file_name").notNull().default(""),
    s3Key: text("s3_key").notNull().default(""),
    contentType: text("content_type"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("request_attachment_request_idx").on(t.etablissementId, t.requestId)],
);

export const requestRouting = pgTable(
  "request_routing",
  {
    etablissementId: uuid("etablissement_id")
      .primaryKey()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
);

export const requestRoutingAttr = pgTable(
  "request_routing_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.path], name: "request_routing_attr_pk" }),
  ],
);

/** Config organisation services demandes (singleton par tenant). */
export const requestOrg = pgTable(
  "request_org",
  {
    etablissementId: uuid("etablissement_id")
      .primaryKey()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
);

export const requestOrgAttr = pgTable(
  "request_org_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.path], name: "request_org_attr_pk" }),
  ],
);

/** Domain planning. */
export const domainPlanningDomain = pgTable(
  "domain_planning_domain",
  {
    id: text("id").notNull(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    name: text("name").notNull(),
    description: text("description"),
    color: text("color"),
    coordinatorExternalUserIds: text("coordinator_external_user_ids").array().notNull().default([]),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.id], name: "domain_planning_domain_pk" }),
  ],
);

export const domainPlanningBooking = pgTable(
  "domain_planning_booking",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    domainId: text("domain_id").notNull(),
    title: text("title"),
    startsAt: text("starts_at"),
    endsAt: text("ends_at"),
    assigneeExternalUserId: text("assignee_external_user_id"),
    notes: text("notes"),
  },
  (t) => [index("domain_planning_booking_etab_idx").on(t.etablissementId)],
);

export const domainPlanningSession = pgTable(
  "domain_planning_session",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    title: text("title").notNull().default(""),
    startsAt: text("starts_at"),
    endsAt: text("ends_at"),
    constraintKind: text("constraint_kind"),
    capacity: integer("capacity"),
  },
  (t) => [index("domain_planning_session_etab_idx").on(t.etablissementId)],
);

export const domainPlanningSignup = pgTable(
  "domain_planning_signup",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    sessionId: text("session_id").notNull(),
    externalUserId: text("external_user_id"),
    displayName: text("display_name"),
    email: text("email"),
    createdAt: text("created_at"),
  },
  (t) => [index("domain_planning_signup_etab_idx").on(t.etablissementId)],
);

/** Congés RH. */
export const personnelLeave = pgTable(
  "personnel_leave",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    personnelName: text("personnel_name").notNull().default(""),
    type: text("type").notNull(),
    startDate: text("start_date").notNull(),
    endDate: text("end_date").notNull(),
    reason: text("reason"),
    status: text("status").notNull(),
    createdAt: text("created_at").notNull(),
    decidedAt: text("decided_at"),
    decidedBy: text("decided_by"),
    decisionNote: text("decision_note"),
  },
  (t) => [index("personnel_leave_etab_idx").on(t.etablissementId)],
);

/** Documents partagés RH. */
export const personnelSharedDoc = pgTable(
  "personnel_shared_doc",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    name: text("name").notNull(),
    fileUrl: text("file_url").notNull().default(""),
    uploadedAt: text("uploaded_at").notNull().default(""),
    uploadedBy: text("uploaded_by").notNull().default(""),
  },
  (t) => [index("personnel_shared_doc_etab_idx").on(t.etablissementId)],
);

/** Sous-tables personnel (remplace payload jsonb). */
export const personnelDocument = pgTable(
  "personnel_document",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    kind: text("kind").notNull().default(""),
    name: text("name").notNull().default(""),
    fileUrl: text("file_url").notNull().default(""),
    uploadedAt: text("uploaded_at"),
    expiresAt: text("expires_at"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("personnel_document_personnel_idx").on(t.etablissementId, t.personnelId)],
);

export const personnelFormation = pgTable(
  "personnel_formation",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    title: text("title").notNull().default(""),
    organism: text("organism"),
    completedAt: text("completed_at"),
    expiresAt: text("expires_at"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("personnel_formation_personnel_idx").on(t.etablissementId, t.personnelId)],
);

export const personnelHabilitation = pgTable(
  "personnel_habilitation",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    title: text("title").notNull().default(""),
    issuedAt: text("issued_at"),
    expiresAt: text("expires_at"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("personnel_habilitation_personnel_idx").on(t.etablissementId, t.personnelId)],
);

export const personnelEntretien = pgTable(
  "personnel_entretien",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    kind: text("kind").notNull().default(""),
    status: text("status").notNull().default(""),
    completedAt: text("completed_at"),
    nextDueAt: text("next_due_at"),
    notes: text("notes"),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => [index("personnel_entretien_personnel_idx").on(t.etablissementId, t.personnelId)],
);

export const personnelAttr = pgTable(
  "personnel_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    personnelId: text("personnel_id").notNull(),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.personnelId, t.path],
      name: "personnel_attr_pk",
    }),
  ],
);

/** Stages. */
export const stageOffer = pgTable(
  "stage_offer",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    title: text("title").notNull().default(""),
    status: text("status"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("stage_offer_etab_idx").on(t.etablissementId)],
);

export const stageOfferAttr = pgTable(
  "stage_offer_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    offerId: text("offer_id")
      .notNull()
      .references(() => stageOffer.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.offerId, t.path], name: "stage_offer_attr_pk" }),
  ],
);

export const stageConvention = pgTable(
  "stage_convention",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    status: text("status"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("stage_convention_etab_idx").on(t.etablissementId)],
);

export const stageConventionAttr = pgTable(
  "stage_convention_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    conventionId: text("convention_id")
      .notNull()
      .references(() => stageConvention.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.conventionId, t.path],
      name: "stage_convention_attr_pk",
    }),
  ],
);

export const stageApplication = pgTable(
  "stage_application",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    offerId: text("offer_id"),
    status: text("status"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("stage_application_etab_idx").on(t.etablissementId)],
);

export const stageApplicationAttr = pgTable(
  "stage_application_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    applicationId: text("application_id")
      .notNull()
      .references(() => stageApplication.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.applicationId, t.path],
      name: "stage_application_attr_pk",
    }),
  ],
);

export const stageToken = pgTable(
  "stage_token",
  {
    token: text("token").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    kind: text("kind").notNull().default(""),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("stage_token_etab_idx").on(t.etablissementId)],
);

export const stageTokenAttr = pgTable(
  "stage_token_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    token: text("token")
      .notNull()
      .references(() => stageToken.token, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.token, t.path], name: "stage_token_attr_pk" }),
  ],
);

/** Certificats. */
export const certificateProgram = pgTable(
  "certificate_program",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    title: text("title").notNull().default(""),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("certificate_program_etab_idx").on(t.etablissementId)],
);

export const certificateProgramAttr = pgTable(
  "certificate_program_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    programId: text("program_id")
      .notNull()
      .references(() => certificateProgram.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.programId, t.path],
      name: "certificate_program_attr_pk",
    }),
  ],
);

export const certificateAward = pgTable(
  "certificate_award",
  {
    id: text("id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    programId: text("program_id"),
    status: text("status"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [index("certificate_award_etab_idx").on(t.etablissementId)],
);

export const certificateAwardAttr = pgTable(
  "certificate_award_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    awardId: text("award_id")
      .notNull()
      .references(() => certificateAward.id, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.awardId, t.path],
      name: "certificate_award_attr_pk",
    }),
  ],
);

/** Settings tenant (sections plates + attrs). */
export const tenantSettingSection = pgTable(
  "tenant_setting_section",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    section: text("section").notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.section], name: "tenant_setting_section_pk" }),
  ],
);

export const tenantSettingAttr = pgTable(
  "tenant_setting_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    section: text("section").notNull(),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.section, t.path],
      name: "tenant_setting_attr_pk",
    }),
  ],
);

/**
 * Collection générique relationnelle pour modules longue traîne
 * (internat, channels, covoiturage…): une ligne + attrs EAV textuels.
 * Remplace les fichiers JSON S3 sans payload jsonb.
 */
export const entCollectionRecord = pgTable(
  "ent_collection_record",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    collection: text("collection").notNull(),
    recordId: text("record_id").notNull(),
    status: text("status"),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.collection, t.recordId],
      name: "ent_collection_record_pk",
    }),
    index("ent_collection_record_coll_idx").on(t.etablissementId, t.collection),
    index("ent_collection_record_status_idx").on(t.etablissementId, t.collection, t.status),
  ],
);

export const entCollectionAttr = pgTable(
  "ent_collection_attr",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    collection: text("collection").notNull(),
    recordId: text("record_id").notNull(),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({
      columns: [t.etablissementId, t.collection, t.recordId, t.path],
      name: "ent_collection_attr_pk",
    }),
    index("ent_collection_attr_rec_idx").on(t.etablissementId, t.collection, t.recordId),
  ],
);

/** Documents plateforme (registry, signup…) — hors tenant. */
export const platformDocument = pgTable("platform_document", {
  docKey: text("doc_key").primaryKey(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
});

export const platformDocumentAttr = pgTable(
  "platform_document_attr",
  {
    docKey: text("doc_key")
      .notNull()
      .references(() => platformDocument.docKey, { onDelete: "cascade" }),
    path: text("path").notNull(),
    value: text("value").notNull(),
  },
  (t) => [
    primaryKey({ columns: [t.docKey, t.path], name: "platform_document_attr_pk" }),
  ],
);

/**
 * Jobs d’import PDF absences (ex-absences/ingest-jobs/*.json sur S3).
 * Les PDF restent sur S3 (`document_key`) ; l’état du job est 100 % Postgres.
 */
export const absenceIngestJob = pgTable(
  "absence_ingest_job",
  {
    jobId: text("job_id").primaryKey(),
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    userId: text("user_id").notNull(),
    creatorName: text("creator_name").notNull().default(""),
    creatorEmail: text("creator_email").notNull().default(""),
    creatorRoles: text("creator_roles").array().notNull().default([]),
    status: text("status").notNull().default("pending"),
    startedAt: timestamp("started_at", { withTimezone: true }).notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull(),
    sourceFileName: text("source_file_name").notNull().default(""),
    /** Clé objet S3 du PDF source (binaire, pas une fiche métier). */
    documentKey: text("document_key").notNull(),
    processingStartedAt: timestamp("processing_started_at", { withTimezone: true }),
    phase: text("phase"),
    error: text("error"),
    code: text("code"),
    /** Sérialisation texte des absences créées (id, nom, dates). */
    createdPayload: text("created_payload"),
    parsedPayload: text("parsed_payload"),
    lockedAt: timestamp("locked_at", { withTimezone: true }),
    lockedBy: text("locked_by"),
  },
  (t) => [
    index("absence_ingest_job_etab_idx").on(t.etablissementId),
    index("absence_ingest_job_status_idx").on(t.etablissementId, t.status),
  ],
);
