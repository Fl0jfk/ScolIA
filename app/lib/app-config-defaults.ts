import type {
  Establishment,
  ExternalQuickLinkConfig,
  IntegrationsConfig,
  InternatModuleConfig,
  NotificationsConfig,
  DomainPlanningModuleConfig,
  ProfRoomModuleConfig,
  SiteIdentity,
  StaffDirectoryRow,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import { DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS } from "@/app/lib/domain-planning-defaults";
import { DEFAULT_PROF_ROOM_SUBJECT_COLORS } from "@/app/lib/prof-room-defaults";

export function defaultSiteIdentity(): SiteIdentity {
  return {
    name: "",
    shortName: "",
    organizationKind: "standalone",
    onboardingCompleted: false,
    onboardingStep: 1,
  };
}

export function defaultEstablishments(): Establishment[] {
  return [];
}

export function defaultNotifications(): NotificationsConfig {
  return {
    travelsCompta: [],
    absencesNotifyOgecCompta: [],
  };
}

export function defaultInternatModule(): InternatModuleConfig {
  return {
    rollCallDeadlineHour: 22,
    rollCallReminderHour: 21,
    rollCallReminderEnabled: true,
    weeklySummaryEnabled: true,
    weeklyParentDigestEnabled: true,
    weeklyParentDigestWeekday: 0,
  };
}

export function defaultStaffDirectory(): StaffDirectoryRow[] {
  return [];
}

export function defaultTravelsModule(): TravelsModuleConfig {
  return {
    comptaEmails: [],
    transportProviders: [],
    showGroupeScolaireOption: false,
  };
}

export function defaultIntegrations(): IntegrationsConfig {
  return {
    zeendoc: { enabled: false, buttonLabel: "Envoyer par mail" },
    microsoftOneDrive: { enabled: false },
  };
}

export function defaultExternalLinks(): ExternalQuickLinkConfig[] {
  return [];
}

export function defaultDomainPlanningModule(): DomainPlanningModuleConfig {
  return {
    classesByPole: {},
    activityColors: { ...DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS },
    hoursStart: 8,
    hoursEnd: 17,
    bookingHorizonDays: 56,
  };
}

export function defaultProfRoomModule(): ProfRoomModuleConfig {
  return {
    classesByPole: {},
    subjectColors: { ...DEFAULT_PROF_ROOM_SUBJECT_COLORS },
    hoursStart: 8,
    hoursEnd: 17,
    bookingHorizonDays: 56,
    adminClerkUserIds: [],
  };
}

export function defaultClassAllocationSettings() {
  return {
    levels: [] as {
      level: "ecole" | "college" | "lycee";
      sourceClassPrefixes: string[];
      targetClasses: string[];
    }[],
  };
}
