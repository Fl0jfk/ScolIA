import { parseDashboardAccent } from "@/app/lib/dashboard-brand-presets";
import type { SchoolHolidayZone } from "@/app/lib/fr-school-holidays";

export type OrganizationKind = "standalone" | "groupe";

export type SiteIdentity = {
  name: string;
  shortName?: string;
  headerLogoUrl?: string;
  dashboardAccent?: string;
  organizationKind?: OrganizationKind;
  onboardingCompleted?: boolean;
  onboardingCompletedAt?: string;
  /** Dernière étape validée dans l'assistant (1–12). */
  onboardingStep?: number;
  assistanceEmail?: string;
  schoolHolidayZone?: SchoolHolidayZone;
  address?: {
    street?: string;
    city?: string;
    zip?: string;
    full?: string;
    fullCompact?: string;
    mapsEmbed?: string;
    mapsItinerary?: string;
    latitude?: number;
    longitude?: number;
  };
  phone?: {
    display?: string;
    tel?: string;
    hours?: string;
  };
  preinscriptionUrl?: string;
  reglementFinancier?: string;
  /**
   * Site vitrine Scola (Next.js packagé) — active le CMS Actus dans Communication.
   */
  customWebsite?: {
    enabled: boolean;
    primaryDomain?: string;
  };
};

export type EstablishmentKind = "ecole" | "college" | "lycee" | "custom";

export type Establishment = {
  id: string;
  label: string;
  kind?: EstablishmentKind;
  directorName?: string;
  directorEmail?: string;
  /** Utilisateur Clerk désigné comme responsable de l’établissement. */
  directorClerkUserId?: string;
  /** Couleur d’affichage (#RRGGBB) — voyages, filtres, badges. */
  colorHex?: string;
  /** Clé S3 relative dans le dataBucket (ex. settings/signatures/ecole.png) — jamais une URL publique. */
  signatureS3Key?: string;
  grades?: string;
  clerkRoleSlugs?: string[];
  active?: boolean;
};

export type ZeendocIntegration = {
  enabled: boolean;
  buttonLabel?: string;
  destinationEmail?: string;
};

export type EcoleDirecteIntegration = {
  /** Conservé pour compatibilité JSON legacy — non utilisé par les modules. */
  enabled?: boolean;
  preinscriptionUrl?: string;
};

type OneDriveSecteur = "ecole" | "college" | "lycee";

/** Dossier racine OneDrive où ranger les documents élèves, par cycle. */
type OneDriveSecteurBase = {
  basePath: string;
  label?: string;
};

/** Associe un utilisateur Clerk (ou e-mail / nom) à un cycle, pour le classement OCR/stages. */
type OneDriveUserSecteur = {
  /** Identifiant Clerk — préféré pour le matching. */
  clerkUserId?: string;
  /** E-mail (repli et affichage). */
  match: string;
  /** Nom affiché dans les paramètres. */
  displayName?: string;
  secteur: OneDriveSecteur;
};

/** Lien public (sans secret) du OneDrive RH — attachée de gestion. */
type MicrosoftRhDriveIntegration = {
  enabled: boolean;
  linked: boolean;
  linkedUpn?: string;
  linkedDisplayName?: string;
  linkedAt?: string;
  basePath: string;
};

export type MicrosoftOneDriveIntegration = {
  enabled: boolean;
  /** Surcharge des dossiers racine par cycle (sinon valeurs par défaut en dur). */
  basesBySecteur?: Partial<Record<OneDriveSecteur, OneDriveSecteurBase>>;
  /** Mapping utilisateur → cycle (pour les comptes non câblés en dur). */
  userSecteurs?: OneDriveUserSecteur[];
  /** OneDrive cible RH (meta publique ; refresh token dans secrets tenant). */
  rhDrive?: MicrosoftRhDriveIntegration;
};

export type IntegrationsConfig = {
  zeendoc?: ZeendocIntegration;
  ecoleDirecte?: EcoleDirecteIntegration;
  microsoftOneDrive?: MicrosoftOneDriveIntegration;
};

export type ExternalQuickLinkConfig = {
  id: string;
  name: string;
  link: string;
  img?: string;
  allowedRoles: string[];
};

export type InternatRollCallRecipients = {
  /** Destinataire principal des appels internat (onboarding / config générique). */
  appelContact?: string;
  directionLycee?: string;
  cpeLycee?: string;
  cpeCollege?: string;
};

export type NotificationsConfig = {
  travelsCompta: string[];
  travelsCuisine?: string;
  travelsZeendoc?: string;
  hseOps?: string;
  photocopiesOps?: string;
  absencesNotifyProfEcole?: { label?: string; email: string };
  absencesNotifyProfCollege?: { label?: string; email: string };
  absencesNotifyProfLycee?: { label?: string; email: string };
  /** @deprecated Préférer absencesNotifyProfCollege / absencesNotifyProfLycee */
  absencesNotifyProfCollegeLycee?: { label?: string; email: string };
  absencesNotifyOgecCompta: string[];
  internatRollCallRecipients?: InternatRollCallRecipients;
  internatEmergencyRecipients?: string[];
  /** Préconventions / conventions de stage — file administratif. */
  stagesAdminEmails?: string[];
  /** Signature direction (repli : e-mail directeur selon niveau élève). */
  stagesDirectionEmail?: string;
  /** PDF vierge remplissable — lien de téléchargement sur /stages/deposer */
  stagesConventionTemplateUrl?: string;
};

export type InternatModuleConfig = {
  rollCallDeadlineHour?: number;
  rollCallReminderHour?: number;
  rollCallReminderEnabled?: boolean;
  weeklySummaryEnabled?: boolean;
  weeklyParentDigestEnabled?: boolean;
  weeklyParentDigestWeekday?: number;
  weeklyParentDigestLastSent?: string;
};

export type StaffDirectoryRow = {
  email: string;
  branchId: string;
  role: "leader" | "executor";
  validUntil?: string;
};

export type TravelsModuleConfig = {
  comptaEmails: string[];
  transportProviders: { name: string; email: string }[];
  showGroupeScolaireOption?: boolean;
  pdfFooterText?: string;
  signatureImageUrls?: Record<string, string>;
};

export type ProfRoomModuleConfig = {
  classesByPole: Record<string, string[]>;
  subjectColors: Record<string, string>;
  hoursStart: number;
  hoursEnd: number;
  bookingHorizonDays: number;
  /** Utilisateurs Clerk autorisés comme administrateurs du module. */
  adminClerkUserIds: string[];
};

export type DomainPlanningModuleConfig = {
  classesByPole: Record<string, string[]>;
  activityColors: Record<string, string>;
  hoursStart: number;
  hoursEnd: number;
  bookingHorizonDays: number;
};

export type RoutingService = {
  id: string;
  label: string;
  category: string;
  manualOnly?: boolean;
};

export type RoutingTask = {
  id: string;
  label: string;
  hint: string;
  keywords: string[];
  active: boolean;
};

export type RoutingAssignment = {
  id: string;
  taskId: string;
  email: string;
  personName: string;
  serviceId: string;
  active: boolean;
};

export type RoutingDirectionQueue = {
  id: string;
  label: string;
  email: string;
  active: boolean;
};

/** Tags métier d’une personne (personnel OGEC) pour le routage IA. */
export type RoutingPersonnelTags = {
  email: string;
  personName: string;
  tags: string[];
};

export type RequestsRoutingConfig = {
  version: 1;
  services: RoutingService[];
  tasks: RoutingTask[];
  assignments: RoutingAssignment[];
  directionQueues: RoutingDirectionQueue[];
  /** Page publique parents pour déposer une demande. */
  parentPortal?: {
    enabled: boolean;
  };
  /** Catalogue de tags créés par l’admin (réutilisables). */
  tagCatalog?: string[];
  /** Tags par e-mail — l’IA s’en sert pour choisir à qui donner la demande. */
  personnelTags?: RoutingPersonnelTags[];
};

export type AppConfigBundle = {
  identity: SiteIdentity;
  establishments: Establishment[];
  notifications: NotificationsConfig;
  staffDirectory: StaffDirectoryRow[];
  travels: TravelsModuleConfig;
  profRoom: ProfRoomModuleConfig;
  domainPlanning: DomainPlanningModuleConfig;
  internat: InternatModuleConfig;
  integrations: IntegrationsConfig;
  externalLinks: ExternalQuickLinkConfig[];
  requestsRouting?: RequestsRoutingConfig;
  classAllocation?: {
    levels: {
      level: "ecole" | "college" | "lycee";
      sourceClassPrefixes: string[];
      targetClasses: string[];
    }[];
  };
};

function isEmail(s: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(s.trim());
}

function str(v: unknown, fallback = ""): string {
  return typeof v === "string" ? v : fallback;
}

function strArr(v: unknown): string[] {
  if (!Array.isArray(v)) return [];
  return v.map((x) => str(x).trim()).filter(Boolean);
}

export function parseSiteIdentity(raw: unknown, opts?: { allowEmptyName?: boolean }): SiteIdentity {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const addr = o.address && typeof o.address === "object" ? (o.address as Record<string, unknown>) : {};
  const phone = o.phone && typeof o.phone === "object" ? (o.phone as Record<string, unknown>) : {};
  const name = str(o.name).trim();
  if (!name && !opts?.allowEmptyName) throw new Error("Le nom du groupe est requis.");
  const lat = Number(addr.latitude);
  const lng = Number(addr.longitude);
  const orgKind = str(o.organizationKind);
  const organizationKind: OrganizationKind | undefined =
    orgKind === "standalone" || orgKind === "groupe" ? orgKind : undefined;
  const onboardingStep = Number(o.onboardingStep);
  const assistanceEmail = str(o.assistanceEmail).trim();
  return {
    name: name || (opts?.allowEmptyName ? "" : "Mon établissement"),
    shortName: str(o.shortName) || undefined,
    organizationKind,
    onboardingCompleted:
      o.onboardingCompleted === true
        ? true
        : "onboardingCompleted" in o && o.onboardingCompleted === false
          ? false
          : undefined,
    onboardingCompletedAt: str(o.onboardingCompletedAt) || undefined,
    onboardingStep: Number.isFinite(onboardingStep) && onboardingStep >= 1 ? onboardingStep : undefined,
    assistanceEmail: assistanceEmail && isEmail(assistanceEmail) ? assistanceEmail : undefined,
    address: {
      street: str(addr.street) || undefined,
      city: str(addr.city) || undefined,
      zip: str(addr.zip) || undefined,
      full: str(addr.full) || undefined,
      fullCompact: str(addr.fullCompact) || undefined,
      mapsEmbed: str(addr.mapsEmbed) || undefined,
      mapsItinerary: str(addr.mapsItinerary) || undefined,
      latitude: Number.isFinite(lat) ? lat : undefined,
      longitude: Number.isFinite(lng) ? lng : undefined,
    },
    phone: {
      display: str(phone.display) || undefined,
      tel: str(phone.tel) || undefined,
      hours: str(phone.hours) || undefined,
    },
    preinscriptionUrl: str(o.preinscriptionUrl) || undefined,
    reglementFinancier: str(o.reglementFinancier) || undefined,
    customWebsite: (() => {
      const cw = o.customWebsite && typeof o.customWebsite === "object"
        ? (o.customWebsite as Record<string, unknown>)
        : null;
      if (!cw) return undefined;
      const domain = str(cw.primaryDomain).trim().toLowerCase().replace(/^https?:\/\//, "").replace(/\/$/, "");
      return {
        enabled: cw.enabled === true,
        primaryDomain: domain || undefined,
      };
    })(),
    headerLogoUrl: str(o.headerLogoUrl).trim() || undefined,
    dashboardAccent: parseDashboardAccent(o.dashboardAccent),
    schoolHolidayZone: (() => {
      const z = str(o.schoolHolidayZone).trim().toUpperCase();
      return z === "A" || z === "B" || z === "C" ? (z as SchoolHolidayZone) : undefined;
    })(),
  };
}

export function parseEstablishment(raw: unknown): Establishment {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const id = str(o.id).trim();
  const label = str(o.label).trim();
  if (!id || !label) throw new Error("Chaque établissement doit avoir un id et un libellé.");
  const email = str(o.directorEmail).trim();
  if (email && !isEmail(email)) throw new Error(`Email direction invalide pour ${label}.`);
  const kindRaw = str(o.kind).trim();
  const kind: EstablishmentKind | undefined =
    kindRaw === "ecole" || kindRaw === "college" || kindRaw === "lycee" || kindRaw === "custom"
      ? kindRaw
      : undefined;
  return {
    id,
    label,
    kind,
    directorName: str(o.directorName) || undefined,
    directorEmail: email || undefined,
    directorClerkUserId: str(o.directorClerkUserId).trim() || undefined,
    colorHex: (() => {
      const raw = str(o.colorHex).trim();
      if (/^#[0-9a-fA-F]{6}$/.test(raw)) return raw.toUpperCase();
      if (/^[0-9a-fA-F]{6}$/.test(raw)) return `#${raw.toUpperCase()}`;
      return undefined;
    })(),
    signatureS3Key: str(o.signatureS3Key).trim() || undefined,
    grades: str(o.grades) || undefined,
    clerkRoleSlugs: strArr(o.clerkRoleSlugs),
    active: o.active !== false,
  };
}

export function parseEstablishmentsFile(raw: unknown): Establishment[] {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const list = Array.isArray(o.establishments) ? o.establishments : Array.isArray(raw) ? raw : [];
  return list.map(parseEstablishment);
}

export function parseNotifications(raw: unknown): NotificationsConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const compta = strArr(o.travelsCompta).filter(isEmail);
  const ogec = strArr(o.absencesNotifyOgecCompta).filter(isEmail);
  const parseNotify = (block: unknown) => {
    if (!block || typeof block !== "object") return undefined;
    const b = block as Record<string, unknown>;
    const email = str(b.email).trim();
    if (!email || !isEmail(email)) return undefined;
    return { label: str(b.label) || undefined, email };
  };
  const parseInternatRollCall = (block: unknown): InternatRollCallRecipients | undefined => {
    if (!block || typeof block !== "object") return undefined;
    const b = block as Record<string, unknown>;
    const out: InternatRollCallRecipients = {};
    const appel = str(b.appelContact).trim();
    const d = str(b.directionLycee).trim();
    const cL = str(b.cpeLycee).trim();
    const cC = str(b.cpeCollege).trim();
    if (appel && isEmail(appel)) out.appelContact = appel;
    if (d && isEmail(d)) out.directionLycee = d;
    if (cL && isEmail(cL)) out.cpeLycee = cL;
    if (cC && isEmail(cC)) out.cpeCollege = cC;
    return Object.keys(out).length ? out : undefined;
  };

  return {
    travelsCompta: compta,
    travelsCuisine: str(o.travelsCuisine) || undefined,
    travelsZeendoc: str(o.travelsZeendoc) || undefined,
    hseOps: str(o.hseOps) || undefined,
    photocopiesOps: str(o.photocopiesOps) || undefined,
    absencesNotifyProfEcole: parseNotify(o.absencesNotifyProfEcole),
    absencesNotifyProfCollege: parseNotify(o.absencesNotifyProfCollege),
    absencesNotifyProfLycee: parseNotify(o.absencesNotifyProfLycee),
    absencesNotifyProfCollegeLycee: parseNotify(o.absencesNotifyProfCollegeLycee),
    absencesNotifyOgecCompta: ogec,
    internatRollCallRecipients: parseInternatRollCall(o.internatRollCallRecipients),
    internatEmergencyRecipients: strArr(o.internatEmergencyRecipients).filter(isEmail),
    stagesAdminEmails: strArr(o.stagesAdminEmails).filter(isEmail),
    stagesDirectionEmail: (() => {
      const e = str(o.stagesDirectionEmail).trim();
      return e && isEmail(e) ? e : undefined;
    })(),
    stagesConventionTemplateUrl: (() => {
      const u = str(o.stagesConventionTemplateUrl).trim();
      return u.startsWith("http") ? u : undefined;
    })(),
  };
}

export function parseInternatModule(raw: unknown): InternatModuleConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const hour = Number(o.rollCallDeadlineHour);
  const reminderHour = Number(o.rollCallReminderHour);
  const weekday = Number(o.weeklyParentDigestWeekday);
  return {
    rollCallDeadlineHour: Number.isFinite(hour) ? hour : 22,
    rollCallReminderHour: Number.isFinite(reminderHour) ? reminderHour : 21,
    rollCallReminderEnabled: o.rollCallReminderEnabled !== false,
    weeklySummaryEnabled: o.weeklySummaryEnabled !== false,
    weeklyParentDigestEnabled: o.weeklyParentDigestEnabled !== false,
    weeklyParentDigestWeekday: Number.isFinite(weekday) && weekday >= 0 && weekday <= 6 ? weekday : 0,
    weeklyParentDigestLastSent: str(o.weeklyParentDigestLastSent) || undefined,
  };
}

export function parseStaffDirectoryFile(raw: unknown): StaffDirectoryRow[] {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const rows = Array.isArray(o.rows) ? o.rows : [];
  return rows.map((r) => {
    const row = r && typeof r === "object" ? (r as Record<string, unknown>) : {};
    const email = str(row.email).trim();
    const branchId = str(row.branchId).trim();
    const role = str(row.role) === "executor" ? "executor" : "leader";
    if (!email || !isEmail(email) || !branchId) throw new Error("Ligne annuaire invalide (email, branche).");
    return { email, branchId, role, validUntil: str(row.validUntil) || undefined };
  });
}

export function parseTravelsModule(raw: unknown): TravelsModuleConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const providers = Array.isArray(o.transportProviders) ? o.transportProviders : [];
  const sigUrls: Record<string, string> = {};
  if (o.signatureImageUrls && typeof o.signatureImageUrls === "object") {
    for (const [k, v] of Object.entries(o.signatureImageUrls as Record<string, unknown>)) {
      const url = str(v).trim();
      if (url) sigUrls[k] = url;
    }
  }
  return {
    comptaEmails: strArr(o.comptaEmails).filter(isEmail),
    transportProviders: providers.flatMap((p) => {
      const x = p && typeof p === "object" ? (p as Record<string, unknown>) : {};
      const email = str(x.email).trim();
      if (!email) return [];
      if (!isEmail(email)) throw new Error("Transporteur : email invalide.");
      return [{ name: str(x.name) || "Transporteur", email }];
    }),
    showGroupeScolaireOption: o.showGroupeScolaireOption === true,
    pdfFooterText: str(o.pdfFooterText) || undefined,
    signatureImageUrls: Object.keys(sigUrls).length ? sigUrls : undefined,
  };
}

function parseZeendocIntegration(raw: unknown): ZeendocIntegration | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const o = raw as Record<string, unknown>;
  const enabled = o.enabled === true;
  const destinationEmail = str(o.destinationEmail).trim();
  return {
    enabled,
    buttonLabel: str(o.buttonLabel) || undefined,
    destinationEmail: destinationEmail && isEmail(destinationEmail) ? destinationEmail : undefined,
  };
}

function parseEcoleDirecteIntegration(raw: unknown): EcoleDirecteIntegration | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const o = raw as Record<string, unknown>;
  return {
    enabled: o.enabled === true,
    preinscriptionUrl: str(o.preinscriptionUrl) || undefined,
  };
}

function parseOneDriveSecteur(value: unknown): OneDriveSecteur | null {
  const s = String(value ?? "").trim().toLowerCase();
  return s === "ecole" || s === "college" || s === "lycee" ? s : null;
}

function parseOneDriveIntegration(raw: Record<string, unknown>): MicrosoftOneDriveIntegration {
  const result: MicrosoftOneDriveIntegration = { enabled: raw.enabled === true };

  const basesRaw = raw.basesBySecteur;
  if (basesRaw && typeof basesRaw === "object") {
    const bases: Partial<Record<OneDriveSecteur, OneDriveSecteurBase>> = {};
    for (const secteur of ["ecole", "college", "lycee"] as const) {
      const row = (basesRaw as Record<string, unknown>)[secteur];
      if (row && typeof row === "object") {
        const basePath = str((row as Record<string, unknown>).basePath);
        if (basePath) {
          bases[secteur] = {
            basePath,
            label: str((row as Record<string, unknown>).label) || undefined,
          };
        }
      }
    }
    if (Object.keys(bases).length > 0) result.basesBySecteur = bases;
  }

  if (Array.isArray(raw.userSecteurs)) {
    const list: OneDriveUserSecteur[] = [];
    for (const item of raw.userSecteurs) {
      if (!item || typeof item !== "object") continue;
      const row = item as Record<string, unknown>;
      const clerkUserId = str(row.clerkUserId) || undefined;
      const match = str(row.match);
      const displayName = str(row.displayName) || undefined;
      const secteur = parseOneDriveSecteur(row.secteur);
      const resolvedMatch = match || clerkUserId;
      if (secteur && resolvedMatch) {
        list.push({ clerkUserId, match: resolvedMatch, displayName, secteur });
      }
    }
    if (list.length > 0) result.userSecteurs = list;
  }

  const rhRaw = raw.rhDrive;
  if (rhRaw && typeof rhRaw === "object") {
    const rh = rhRaw as Record<string, unknown>;
    const basePath = str(rh.basePath) || "Dossier personnel";
    result.rhDrive = {
      enabled: rh.enabled !== false,
      linked: rh.linked === true,
      linkedUpn: str(rh.linkedUpn) || undefined,
      linkedDisplayName: str(rh.linkedDisplayName) || undefined,
      linkedAt: str(rh.linkedAt) || undefined,
      basePath,
    };
  }

  return result;
}

export function parseIntegrations(raw: unknown): IntegrationsConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const oneDrive = o.microsoftOneDrive && typeof o.microsoftOneDrive === "object"
    ? (o.microsoftOneDrive as Record<string, unknown>)
    : null;
  return {
    zeendoc: parseZeendocIntegration(o.zeendoc),
    ecoleDirecte: parseEcoleDirecteIntegration(o.ecoleDirecte),
    microsoftOneDrive: oneDrive ? parseOneDriveIntegration(oneDrive) : undefined,
  };
}

export function parseExternalLinksFile(raw: unknown): ExternalQuickLinkConfig[] {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const list = Array.isArray(o.links) ? o.links : Array.isArray(raw) ? raw : [];
  return list.map((item) => {
    const x = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
    const id = str(x.id).trim();
    const name = str(x.name).trim();
    const link = str(x.link).trim();
    if (!id || !name || !link) throw new Error("Lien externe : id, nom et URL requis.");
    return {
      id,
      name,
      link,
      img: str(x.img) || undefined,
      allowedRoles: strArr(x.allowedRoles),
    };
  });
}

export function parseDomainPlanningModule(raw: unknown): DomainPlanningModuleConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const classesByPole: Record<string, string[]> = {};
  if (o.classesByPole && typeof o.classesByPole === "object") {
    for (const [k, v] of Object.entries(o.classesByPole as Record<string, unknown>)) {
      classesByPole[k] = strArr(v);
    }
  }
  const activityColors: Record<string, string> = {};
  if (o.activityColors && typeof o.activityColors === "object") {
    for (const [k, v] of Object.entries(o.activityColors as Record<string, unknown>)) {
      activityColors[k] = str(v);
    }
  }
  return {
    classesByPole,
    activityColors,
    hoursStart: typeof o.hoursStart === "number" ? o.hoursStart : 8,
    hoursEnd: typeof o.hoursEnd === "number" ? o.hoursEnd : 17,
    bookingHorizonDays: typeof o.bookingHorizonDays === "number" ? o.bookingHorizonDays : 56,
  };
}

export function parseProfRoomModule(raw: unknown): ProfRoomModuleConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const classesByPole: Record<string, string[]> = {};
  if (o.classesByPole && typeof o.classesByPole === "object") {
    for (const [k, v] of Object.entries(o.classesByPole as Record<string, unknown>)) {
      classesByPole[k] = strArr(v);
    }
  }
  const subjectColors: Record<string, string> = {};
  if (o.subjectColors && typeof o.subjectColors === "object") {
    for (const [k, v] of Object.entries(o.subjectColors as Record<string, unknown>)) {
      subjectColors[k] = str(v);
    }
  }
  return {
    classesByPole,
    subjectColors,
    hoursStart: typeof o.hoursStart === "number" ? o.hoursStart : 8,
    hoursEnd: typeof o.hoursEnd === "number" ? o.hoursEnd : 17,
    bookingHorizonDays: typeof o.bookingHorizonDays === "number" ? o.bookingHorizonDays : 56,
    adminClerkUserIds: strArr(o.adminClerkUserIds),
  };
}

export function parseRequestsRouting(raw: unknown): RequestsRoutingConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const services = Array.isArray(o.services) ? o.services : [];
  const tasks = Array.isArray(o.tasks) ? o.tasks : [];
  const assignments = Array.isArray(o.assignments) ? o.assignments : [];
  const directionQueues = Array.isArray(o.directionQueues) ? o.directionQueues : [];

  const parsedServices: RoutingService[] = services.map((s) => {
    const x = s && typeof s === "object" ? (s as Record<string, unknown>) : {};
    const id = str(x.id).trim();
    if (!id) throw new Error("Service : id requis.");
    return {
      id,
      label: str(x.label) || id,
      category: str(x.category) || "Général",
      manualOnly: x.manualOnly === true,
    };
  });

  const parsedTasks: RoutingTask[] = tasks.map((t) => {
    const x = t && typeof t === "object" ? (t as Record<string, unknown>) : {};
    const id = str(x.id).trim();
    if (!id) throw new Error("Tâche : id requis.");
    return {
      id,
      label: str(x.label) || id,
      hint: str(x.hint),
      keywords: strArr(x.keywords),
      active: x.active !== false,
    };
  });

  const parsedAssignments: RoutingAssignment[] = assignments.map((a) => {
    const x = a && typeof a === "object" ? (a as Record<string, unknown>) : {};
    const id = str(x.id).trim();
    const email = str(x.email).trim();
    if (!id) throw new Error("Affectation : id requis.");
    if (!email || !isEmail(email)) throw new Error("Affectation : email invalide.");
    return {
      id,
      taskId: str(x.taskId).trim(),
      email,
      personName: str(x.personName) || email.split("@")[0] || email,
      serviceId: str(x.serviceId).trim() || "administratif",
      active: x.active !== false,
    };
  });

  const parsedDirection: RoutingDirectionQueue[] = directionQueues.map((d) => {
    const x = d && typeof d === "object" ? (d as Record<string, unknown>) : {};
    const id = str(x.id).trim();
    const email = str(x.email).trim();
    if (!id) throw new Error("File direction : id requis.");
    if (!email || !isEmail(email)) throw new Error("File direction : email invalide.");
    return {
      id,
      label: str(x.label) || id,
      email,
      active: x.active !== false,
    };
  });

  const parentPortal = {
    enabled:
      o.parentPortal && typeof o.parentPortal === "object"
        ? (o.parentPortal as Record<string, unknown>).enabled === true
        : false,
  };

  const personnelTagsRaw = Array.isArray(o.personnelTags) ? o.personnelTags : [];
  const personnelTags: RoutingPersonnelTags[] = [];
  const seenEmails = new Set<string>();
  const usedTags = new Set<string>();
  for (const row of personnelTagsRaw) {
    const x = row && typeof row === "object" ? (row as Record<string, unknown>) : {};
    const email = str(x.email).trim().toLowerCase();
    if (!email || !isEmail(email) || seenEmails.has(email)) continue;
    seenEmails.add(email);
    const tags = [
      ...new Set(
        strArr(x.tags)
          .map((t) => t.trim())
          .filter(Boolean),
      ),
    ];
    for (const t of tags) usedTags.add(t);
    personnelTags.push({
      email,
      personName: str(x.personName).trim() || email.split("@")[0] || email,
      tags,
    });
  }

  const catalogFromConfig = strArr(o.tagCatalog)
    .map((t) => t.trim())
    .filter(Boolean);
  const tagCatalog = [...new Set([...catalogFromConfig, ...usedTags])].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );

  return {
    version: 1,
    services: parsedServices,
    tasks: parsedTasks,
    assignments: parsedAssignments,
    directionQueues: parsedDirection,
    parentPortal,
    tagCatalog,
    personnelTags,
  };
}

export function parseClassAllocationSettings(raw: unknown): NonNullable<AppConfigBundle["classAllocation"]> {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const levelsRaw = Array.isArray(o.levels) ? o.levels : [];
  const levels = levelsRaw
    .map((l) => {
      const x = l && typeof l === "object" ? (l as Record<string, unknown>) : {};
      const level = str(x.level).trim() as "ecole" | "college" | "lycee";
      if (!["ecole", "college", "lycee"].includes(level)) return null;
      return {
        level,
        sourceClassPrefixes: strArr(x.sourceClassPrefixes),
        targetClasses: strArr(x.targetClasses),
      };
    })
    .filter((v): v is NonNullable<typeof v> => Boolean(v));
  return { levels };
}
