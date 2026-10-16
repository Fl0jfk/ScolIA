import "server-only";

import {
  isAbsencePendingForManager,
  isAbsenceVisibleOnCalendar,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import {
  isAbsencePendingForProcessor,
  viewerIsAbsenceProcessor,
} from "@/app/lib/absences-admin-access";
import { getAbsenceIndex } from "@/app/lib/absences-storage";
import { getJson } from "@/app/lib/s3-storage";
import { loadAppConfig } from "@/app/lib/app-config";
import { loadWeekSheetData } from "@/app/lib/dashboard-week-sheet-storage";
import { listPendingSignaturesForUser } from "@/app/lib/stage-pending-signatures";
import { loadSignaturesPendingStageConventions } from "@/app/lib/stage-convention-load";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import { resolveStageViewerRole } from "@/app/lib/stage-access";
import { canAccessHseModule, canViewHseDemand } from "@/app/lib/demandes-hse-access";
import { canAccessRequestsStaffBoardForUser } from "@/app/lib/requests-staff-access";
import { getRequestsIndex, isLeaderForRequestBranch } from "@/app/lib/requests";
import { getAllBranchStaffEmailsFromRouting } from "@/app/lib/requests-routing-config";
import { isVisibleOnStaffBoard } from "@/app/lib/requests-board";
import {
  isInternatEveningRollCallDay,
  rollCallPendingActivityStudents,
  sectionIsComplete,
  todayDateParis,
} from "@/app/lib/internat-stats";
import {
  hasVotedMoodPulse,
  moodPulseTodayKey,
  readMoodPulseDay,
} from "@/app/lib/rh/mood-pulse-storage";
import { getInternatRollCall, getInternatStudents } from "@/app/lib/internat-storage";
import { canAccessInternatModule, canSeeInternatRollCallSignal } from "@/app/lib/internat-rbac";
import type { TripIndexRow } from "@/app/lib/dashboard-trips";
import { defaultProfRoomModule } from "@/app/lib/app-config-defaults";
import { parseProfRoomModule } from "@/app/lib/app-config-schemas";
import { withDefaultProfRoomSubjects } from "@/app/lib/prof-room-defaults";
import { getPersonnelIndex } from "@/app/lib/personnel-storage";
import { getPersonnelLeaveRequests } from "@/app/lib/personnel-leave-storage";
import { PERSONNEL_LEAVE_TYPE_LABELS } from "@/app/lib/personnel-types";
import {
  absencesToLeaveSpans,
  mergeLeaveSpans,
} from "@/app/lib/absences-leave-spans";
import {
  findCurrentActivity,
  schoolWeekParity,
  type LeaveSpan,
} from "@/app/lib/rh/planning-calendar";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import { hasRole } from "@/app/lib/intranet-role-utils";
import type { RhPlanningDoc } from "@/app/lib/rh/planning-types";
import { listUnseenSharedFolderInvites } from "@/app/lib/documents-cloud";
import { canAccessSstRegistre } from "@/app/lib/sst-registre/access";
import { isSstEmargementPending } from "@/app/lib/sst-registre/db";
import { vsModulesScopeHash } from "@/app/lib/dashboard-signals-cache-keys";
import {
  runParallelSignalSources,
  unwrapSignalSource,
  type DashboardSignalSourceId,
  type SignalSourceStatus,
} from "@/app/lib/dashboard-signals-parallel";
import { valkeyCached } from "@/app/lib/valkey";
import {
  VALKEY_TTL,
  valkeyKeyDashboardSignalsAbsenceIndex,
  valkeyKeyDashboardSignalsRequestsBoard,
  valkeyKeyDashboardSignalsRhPlanningNow,
  valkeyKeyDashboardSignalsStagesPending,
  valkeyKeyDashboardSignalsVsCounts,
} from "@/app/lib/valkey-keys";
import type { ModuleAccessConfig } from "@/app/lib/module-access";
import type { NotificationsConfig, Establishment } from "@/app/lib/app-config-schemas";

async function safeJson<T>(path: string, timeoutMs = 8_000): Promise<T | null> {
  try {
    const hit = await Promise.race([
      getJson<T>(path),
      new Promise<null>((resolve) => {
        setTimeout(() => resolve(null), timeoutMs);
      }),
    ]);
    if (!hit || typeof hit !== "object" || !("data" in hit)) return null;
    return (hit.data as T) ?? null;
  } catch {
    return null;
  }
}

type RoomReservationSignal = {
  id: string;
  roomId: string;
  startsAt: string;
  endsAt?: string;
  subject?: string;
  className?: string;
  status?: string;
  userId?: string;
  email?: string;
  bookedForOther?: boolean;
  bookedByUserId?: string;
  firstName?: string;
  lastName?: string;
  bookedByFirstName?: string;
  bookedByLastName?: string;
};

export function normalizeRoomReservations(raw: unknown): RoomReservationSignal[] {
  if (!Array.isArray(raw)) return [];
  const out: RoomReservationSignal[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const r = item as Record<string, unknown>;
    const startsAt = typeof r.startsAt === "string" ? r.startsAt : "";
    const roomId = typeof r.roomId === "string" ? r.roomId : "";
    const id = typeof r.id === "string" ? r.id : "";
    if (!id || !roomId || !startsAt) continue;
    out.push({
      id,
      roomId,
      startsAt,
      endsAt: typeof r.endsAt === "string" ? r.endsAt : undefined,
      subject: typeof r.subject === "string" ? r.subject : undefined,
      className: typeof r.className === "string" ? r.className : undefined,
      status: typeof r.status === "string" ? r.status : undefined,
      userId: typeof r.userId === "string" ? r.userId : undefined,
      email: typeof r.email === "string" ? r.email : undefined,
      bookedForOther: r.bookedForOther === true,
      bookedByUserId: typeof r.bookedByUserId === "string" ? r.bookedByUserId : undefined,
      firstName: typeof r.firstName === "string" ? r.firstName : undefined,
      lastName: typeof r.lastName === "string" ? r.lastName : undefined,
      bookedByFirstName:
        typeof r.bookedByFirstName === "string" ? r.bookedByFirstName : undefined,
      bookedByLastName: typeof r.bookedByLastName === "string" ? r.bookedByLastName : undefined,
    });
  }
  return out;
}

function parseRoomsRaw(
  roomsRaw:
    | { rooms?: { id: string; name: string }[] }
    | { id: string; name: string }[]
    | null,
): { id: string; name: string }[] {
  if (Array.isArray(roomsRaw)) {
    return roomsRaw
      .filter((r): r is { id: string; name: string } =>
        Boolean(r && typeof r === "object" && typeof (r as { id?: unknown }).id === "string"),
      )
      .map((r) => ({
        id: r.id,
        name: typeof (r as { name?: unknown }).name === "string" ? (r as { name: string }).name : r.id,
      }));
  }
  if (roomsRaw && typeof roomsRaw === "object" && Array.isArray(roomsRaw.rooms)) {
    return roomsRaw.rooms
      .filter((r): r is { id: string; name: string } =>
        Boolean(r && typeof r === "object" && typeof (r as { id?: unknown }).id === "string"),
      )
      .map((r) => ({
        id: (r as { id: string }).id,
        name:
          typeof (r as { name?: unknown }).name === "string"
            ? (r as { name: string }).name
            : (r as { id: string }).id,
      }));
  }
  return [];
}

export type DashboardSignalsViewUser = {
  id: string;
  businessUserId: string;
  email: string | null;
  roles: string[];
  firstName: string | null;
  lastName: string | null;
  name: string | null;
  imageUrl: string | null;
  platformAdmin: boolean;
};

export type DashboardSignalsLoadContext = {
  signalsEtabId: string;
  user: DashboardSignalsViewUser;
  isOrgAdmin: boolean;
  accessibleModuleIds: Set<string>;
  moduleAccess: ModuleAccessConfig;
  isPhotoOps: boolean;
  photocopiesOpsEmails: string[];
  appBundle: Awaited<ReturnType<typeof loadAppConfig>> | null;
  establishments: Establishment[];
  absenceNotifications: NotificationsConfig | null;
};

export type InternatSourceValue = {
  internatRollCallStatus:
    | "validee"
    | "en_cours"
    | "non_demarre"
    | "attente_activite"
    | null;
};

export type RhSourceValue = {
  moodPulseSubmittedToday: boolean;
  sstEmargementPending: boolean;
  planningNow: {
    title: string;
    detail: string;
    start: string;
    end: string;
  } | null;
};

export type DemandesSourceValue = {
  requestsBoard: Array<{
    id: string;
    status: string;
    subject?: string;
    requesterName?: string;
    assignedTo?: {
      email?: string;
      claimedBy?: { email?: string; userId?: string | null } | null;
      routeId?: string;
      unit?: string;
      poolEmails?: string[];
    };
  }>;
  hse: Array<{
    id: string;
    status: string;
    etablissement: string;
    createdBy: { userId: string };
  }>;
  photocopies: Array<{
    id: string;
    status: string;
    etablissement?: string;
    createdBy?: { userId?: string; name?: string };
  }>;
};

export type AbsencesSourceValue = {
  absences: AbsenceRecord[];
  vsAbsencesATraiter: number;
  vsAbsencesJustifFamille: number;
  vsAppelsManquants: number;
};

export type DocumentsSourceValue = {
  unseenSharedFolders: Array<{ id: string; name: string }>;
};

export type StagesSourceValue = {
  stagesPendingSignatures: number;
};

export type OthersSourceValue = {
  tripsRaw: TripIndexRow[] | null;
  rooms: { id: string; name: string }[];
  reservations: RoomReservationSignal[];
  roomSubjectColors: Record<string, string>;
  weekSheet: Awaited<ReturnType<typeof loadWeekSheetData>> | null;
  vsSanctionsAujourdhui: number;
  vsCarnetNonSignees: number;
  facturesEnRetard: number;
  anneeScolaireLabel: string | null;
  unseenAccompagnementAlerts: Array<{
    documentId: string;
    eleveId: string;
    eleveNom: string;
    elevePrenom: string;
    classe: string | null;
    kind: "pap" | "pai" | "pps" | "gevasco";
    title: string;
    detail: string;
  }>;
  creneauxVidesCount: number;
  creneauxVidesTravelId: string | null;
  travelsUnreadMessages:
    | {
        tripCount: number;
        messageCount: number;
        firstTripId: string | null;
      }
    | undefined;
};

export type DashboardSignalSourcesBundle = {
  internat: InternatSourceValue;
  rh: RhSourceValue;
  demandes: DemandesSourceValue;
  absences: AbsencesSourceValue;
  documents: DocumentsSourceValue;
  stages: StagesSourceValue;
  others: OthersSourceValue;
};

const EMPTY_INTERNAT: InternatSourceValue = { internatRollCallStatus: null };
const EMPTY_RH: RhSourceValue = {
  moodPulseSubmittedToday: false,
  sstEmargementPending: false,
  planningNow: null,
};
const EMPTY_DEMANDES: DemandesSourceValue = {
  requestsBoard: [],
  hse: [],
  photocopies: [],
};
const EMPTY_ABSENCES: AbsencesSourceValue = {
  absences: [],
  vsAbsencesATraiter: 0,
  vsAbsencesJustifFamille: 0,
  vsAppelsManquants: 0,
};
const EMPTY_DOCUMENTS: DocumentsSourceValue = { unseenSharedFolders: [] };
const EMPTY_STAGES: StagesSourceValue = { stagesPendingSignatures: 0 };
const EMPTY_OTHERS: OthersSourceValue = {
  tripsRaw: null,
  rooms: [],
  reservations: [],
  roomSubjectColors: withDefaultProfRoomSubjects(defaultProfRoomModule()).subjectColors,
  weekSheet: null,
  vsSanctionsAujourdhui: 0,
  vsCarnetNonSignees: 0,
  facturesEnRetard: 0,
  anneeScolaireLabel: null,
  unseenAccompagnementAlerts: [],
  creneauxVidesCount: 0,
  creneauxVidesTravelId: null,
  travelsUnreadMessages: undefined,
};

async function loadInternatSource(ctx: DashboardSignalsLoadContext): Promise<InternatSourceValue> {
  const { accessibleModuleIds, user } = ctx;
  const roles = user.roles;
  if (
    !accessibleModuleIds.has("internat") ||
    !canAccessInternatModule(roles) ||
    !canSeeInternatRollCallSignal(roles)
  ) {
    return EMPTY_INTERNAT;
  }
  const today = todayDateParis();
  if (!isInternatEveningRollCallDay(today)) {
    return EMPTY_INTERNAT;
  }
  const [roll, internatStudents] = await Promise.all([
    getInternatRollCall(today),
    getInternatStudents(),
  ]);
  if (roll.status === "validee") {
    return { internatRollCallStatus: "validee" };
  }
  const pendingActivity = rollCallPendingActivityStudents(roll, internatStudents);
  const sectionsDone =
    sectionIsComplete(roll.boys, internatStudents, "M") &&
    sectionIsComplete(roll.girls, internatStudents, "F");
  if (sectionsDone && pendingActivity.length > 0) {
    return { internatRollCallStatus: "attente_activite" };
  }
  const marks = { ...roll.boys.marks, ...roll.girls.marks };
  const internatRollCallStatus =
    Object.keys(marks).length > 0 || roll.boys.completed || roll.girls.completed
      ? "en_cours"
      : "non_demarre";
  return { internatRollCallStatus };
}

async function loadRhPlanningNow(
  ctx: DashboardSignalsLoadContext,
): Promise<RhSourceValue["planningNow"]> {
  const { accessibleModuleIds, user, signalsEtabId } = ctx;
  const roles = user.roles;
  const userId = user.businessUserId;
  if (!accessibleModuleIds.has("mon-planning") && !accessibleModuleIds.has("rh")) {
    return null;
  }
  return valkeyCached({
    key: valkeyKeyDashboardSignalsRhPlanningNow(signalsEtabId, user.id),
    ttlSeconds: VALKEY_TTL.dashboardSignalsRhPlanningNow,
    loader: async () => {
      let doc: RhPlanningDoc | null = null;
      let leavePersonnelId: string | undefined;
      try {
        const index = await getPersonnelIndex();
        const self = index.find((e) => e.externalUserId === userId && e.active !== false);
        if (self) leavePersonnelId = self.id;
      } catch {
        leavePersonnelId = undefined;
      }

      if (hasRole(roles, "professeur")) {
        doc = await readRhPlanning("teacher", userId);
      } else {
        doc = await readRhPlanning("staff", userId);
      }
      let needsTeacherFallback = !doc;
      if (doc?.kind === "teacher") {
        needsTeacherFallback =
          doc.weekA.length === 0 &&
          doc.weekB.length === 0 &&
          !(doc.replacements?.length ?? 0);
      }
      if (needsTeacherFallback) {
        const teacherTry = await readRhPlanning("teacher", userId);
        if (teacherTry.kind === "teacher") {
          if (
            teacherTry.weekA.length > 0 ||
            teacherTry.weekB.length > 0 ||
            (teacherTry.replacements?.length ?? 0) > 0
          ) {
            doc = teacherTry;
          }
        }
      }
      if (!doc) return null;

      let leaves: LeaveSpan[] = [];
      if (leavePersonnelId) {
        const allLeaves = await getPersonnelLeaveRequests();
        leaves = allLeaves
          .filter((r) => r.status === "validee" && r.personnelId === leavePersonnelId)
          .map((r) => ({
            startDate: r.startDate,
            endDate: r.endDate,
            type: r.type,
            label: PERSONNEL_LEAVE_TYPE_LABELS[r.type] || r.type,
          }));
      }
      try {
        const absIndex = await getAbsenceIndex();
        const absenceSpans = absencesToLeaveSpans(absIndex, {
          personnelId: leavePersonnelId || null,
          userId,
        });
        leaves = mergeLeaveSpans(leaves, absenceSpans);
      } catch {
        /* optionnel */
      }
      let zone: "A" | "B" | "C" | null = null;
      try {
        const cfg = await loadAppConfig();
        zone = cfg.identity.schoolHolidayZone ?? null;
      } catch {
        zone = null;
      }
      const act = findCurrentActivity(
        doc,
        new Date(),
        schoolWeekParity(new Date()),
        leaves,
        zone,
      );
      if (!act) return null;
      return {
        title: act.title,
        detail: act.detail,
        start: act.start,
        end: act.end,
      };
    },
  });
}

async function loadRhSource(ctx: DashboardSignalsLoadContext): Promise<RhSourceValue> {
  const { accessibleModuleIds, user, signalsEtabId } = ctx;
  const roles = user.roles;
  const userId = user.businessUserId;
  const authUserId = user.id;
  let moodPulseSubmittedToday = false;
  let sstEmargementPending = false;
  if (accessibleModuleIds.has("rh")) {
    try {
      const dayDoc = await readMoodPulseDay(moodPulseTodayKey());
      moodPulseSubmittedToday = hasVotedMoodPulse(dayDoc, userId);
    } catch {
      moodPulseSubmittedToday = false;
    }
    if (canAccessSstRegistre(roles)) {
      try {
        sstEmargementPending = await isSstEmargementPending({
          etablissementId: signalsEtabId,
          userId: authUserId,
        });
      } catch {
        sstEmargementPending = false;
      }
    }
  }
  let planningNow: RhSourceValue["planningNow"] = null;
  try {
    planningNow = await loadRhPlanningNow(ctx);
  } catch {
    planningNow = null;
  }
  return { moodPulseSubmittedToday, sstEmargementPending, planningNow };
}

async function loadDemandesSource(ctx: DashboardSignalsLoadContext): Promise<DemandesSourceValue> {
  const { accessibleModuleIds, user, isOrgAdmin, isPhotoOps, signalsEtabId } = ctx;
  const roles = user.roles;
  const businessUserId = user.businessUserId;
  const email = user.email ?? "";
  const authUserId = user.id;
  const userId = user.businessUserId;

  const [hseRaw, photocopiesRaw] = await Promise.all([
    accessibleModuleIds.has("demandes-hse") && canAccessHseModule(roles)
      ? safeJson<
          Array<{
            id: string;
            status: string;
            etablissement: string;
            createdBy: { userId: string };
          }>
        >("demandes-hse/index.json")
      : Promise.resolve(null),
    accessibleModuleIds.has("photocopies-couleur") || isPhotoOps
      ? safeJson<
          Array<{
            id: string;
            status: string;
            etablissement?: string;
            createdBy?: { userId?: string; name?: string };
          }>
        >("photocopies-couleur/index.json")
      : Promise.resolve(null),
  ]);

  const hse = Array.isArray(hseRaw)
    ? hseRaw.filter((h) => canViewHseDemand(h, userId, roles))
    : [];
  const photocopies = Array.isArray(photocopiesRaw) ? photocopiesRaw : [];

  let requestsBoard: DemandesSourceValue["requestsBoard"] = [];
  if (
    accessibleModuleIds.has("requests-staff") &&
    (await canAccessRequestsStaffBoardForUser({
      id: businessUserId,
      firstName: user.firstName,
      lastName: user.lastName,
      fullName: user.name?.trim() || null,
      imageUrl: user.imageUrl ?? "",
      primaryEmailAddressId: "primary",
      primaryEmailAddress: email ? { emailAddress: email } : null,
      emailAddresses: email ? [{ id: "primary", emailAddress: email }] : [],
      publicMetadata: {
        role: roles,
        org_admin: isOrgAdmin,
        platform_admin: user.platformAdmin,
      },
    }))
  ) {
    requestsBoard = await valkeyCached({
      key: valkeyKeyDashboardSignalsRequestsBoard(signalsEtabId, authUserId),
      ttlSeconds: VALKEY_TTL.dashboardSignalsRequestsBoard,
      loader: async () => {
        const board: DemandesSourceValue["requestsBoard"] = [];
        const index = await getRequestsIndex();
        const allStaff = await getAllBranchStaffEmailsFromRouting();
        for (const r of index) {
          const isLeader = await isLeaderForRequestBranch(
            r.assignedTo.routeId,
            r.assignedTo.unit,
            email,
          );
          if (isVisibleOnStaffBoard(r.assignedTo, email, allStaff, isLeader)) {
            board.push({
              id: r.id,
              status: r.status,
              subject: r.subject,
              requesterName: r.requester?.fullName || undefined,
              assignedTo: r.assignedTo,
            });
          }
        }
        return board;
      },
    });
  }

  return { requestsBoard, hse, photocopies };
}

async function loadVsCounts(
  ctx: DashboardSignalsLoadContext,
): Promise<Pick<AbsencesSourceValue, "vsAbsencesATraiter" | "vsAbsencesJustifFamille" | "vsAppelsManquants">> {
  const { accessibleModuleIds, signalsEtabId } = ctx;
  const needsVs =
    accessibleModuleIds.has("vs-absences") ||
    accessibleModuleIds.has("vs-appels") ||
    accessibleModuleIds.has("vs-calendrier") ||
    accessibleModuleIds.has("vs-sanctions") ||
    accessibleModuleIds.has("vs-carnet");
  if (!needsVs) {
    return { vsAbsencesATraiter: 0, vsAbsencesJustifFamille: 0, vsAppelsManquants: 0 };
  }
  const scopeHash = vsModulesScopeHash(accessibleModuleIds);
  return valkeyCached({
    key: valkeyKeyDashboardSignalsVsCounts(signalsEtabId, scopeHash),
    ttlSeconds: VALKEY_TTL.dashboardSignalsVsCounts,
    loader: async () => {
      const { resolveCurrentEtablissementId } = await import("@/app/lib/ent-core-db");
      const {
        countAbsencesATraiter,
        countAbsencesJustifFamilleEnAttente,
        countAppelsManquants,
      } = await import("@/app/lib/vs-absences-db");
      const etabId = await resolveCurrentEtablissementId();
      if (!etabId) {
        return { vsAbsencesATraiter: 0, vsAbsencesJustifFamille: 0, vsAppelsManquants: 0 };
      }
      let vsAbsencesATraiter = 0;
      let vsAbsencesJustifFamille = 0;
      let vsAppelsManquants = 0;
      const tasks: Promise<void>[] = [];
      if (accessibleModuleIds.has("vs-absences")) {
        tasks.push(
          countAbsencesATraiter(etabId).then((n) => {
            vsAbsencesATraiter = n;
          }),
        );
        tasks.push(
          countAbsencesJustifFamilleEnAttente(etabId).then((n) => {
            vsAbsencesJustifFamille = n;
          }),
        );
      }
      if (accessibleModuleIds.has("vs-appels") || accessibleModuleIds.has("vs-absences")) {
        tasks.push(
          countAppelsManquants(etabId).then((n) => {
            vsAppelsManquants = n;
          }),
        );
      }
      await Promise.all(tasks);
      return { vsAbsencesATraiter, vsAbsencesJustifFamille, vsAppelsManquants };
    },
  });
}

async function loadAbsencesSource(ctx: DashboardSignalsLoadContext): Promise<AbsencesSourceValue> {
  const { accessibleModuleIds, user, establishments, absenceNotifications, signalsEtabId } =
    ctx;
  const roles = user.roles;
  const userId = user.businessUserId;
  const authUserId = user.id;
  const email = user.email ?? "";

  let absencesRaw: AbsenceRecord[] = [];
  if (accessibleModuleIds.has("rh")) {
    absencesRaw = await valkeyCached({
      key: valkeyKeyDashboardSignalsAbsenceIndex(signalsEtabId),
      ttlSeconds: VALKEY_TTL.dashboardSignalsAbsenceIndex,
      loader: () => getAbsenceIndex().catch(() => [] as AbsenceRecord[]),
    });
  }

  const absenceDirCtx = {
    establishments,
    userId,
    email,
    notifications: absenceNotifications,
  };
  let absences: AbsenceRecord[] = [];
  try {
    absences = absencesRaw.filter(
      (a) =>
        a.createdBy.userId === userId ||
        a.createdBy.userId === authUserId ||
        isAbsenceVisibleOnCalendar(a, userId, roles) ||
        isAbsencePendingForManager(a, userId, roles, absenceDirCtx) ||
        (isAbsencePendingForProcessor(a) &&
          viewerIsAbsenceProcessor(
            a,
            { email, userId, roles },
            absenceNotifications,
            establishments,
          )),
    );
  } catch {
    absences = [];
  }

  const vsCounts = await loadVsCounts(ctx).catch(() => ({
    vsAbsencesATraiter: 0,
    vsAbsencesJustifFamille: 0,
    vsAppelsManquants: 0,
  }));

  return { absences, ...vsCounts };
}

async function loadDocumentsSource(ctx: DashboardSignalsLoadContext): Promise<DocumentsSourceValue> {
  const { accessibleModuleIds, user } = ctx;
  const userId = user.businessUserId;
  if (!accessibleModuleIds.has("documents")) {
    return EMPTY_DOCUMENTS;
  }
  try {
    const unseenSharedFolders = await listUnseenSharedFolderInvites(userId);
    return { unseenSharedFolders };
  } catch {
    return EMPTY_DOCUMENTS;
  }
}

async function loadStagesSource(ctx: DashboardSignalsLoadContext): Promise<StagesSourceValue> {
  const { accessibleModuleIds, user, signalsEtabId } = ctx;
  const roles = user.roles;
  const userId = user.businessUserId;
  const email = user.email ?? "";
  if (!accessibleModuleIds.has("stages") || !resolveStageViewerRole(roles)) {
    return EMPTY_STAGES;
  }
  const stagesPendingSignatures = await valkeyCached({
    key: valkeyKeyDashboardSignalsStagesPending(signalsEtabId, user.id),
    ttlSeconds: VALKEY_TTL.dashboardSignalsStagesPending,
    loader: async () => {
      const pendingConventions = await loadSignaturesPendingStageConventions();
      const userEmail = email.trim().toLowerCase();
      const { listPrincipalClassesForUser } = await import("@/app/lib/stage-referents-config");
      const { canViewReferentConventions } = await import("@/app/lib/stage-access");
      const principalClassNames = canViewReferentConventions(roles)
        ? await listPrincipalClassesForUser(userId)
        : [];
      const conventions = pendingConventions.filter((c) =>
        conventionVisibleToUser(c, roles, userEmail, userId, principalClassNames),
      );
      return (
        await listPendingSignaturesForUser(conventions, userEmail, userId, roles, {
          includePeriodAlignment: false,
        })
      ).length;
    },
  });
  return { stagesPendingSignatures };
}

async function loadOthersSource(ctx: DashboardSignalsLoadContext): Promise<OthersSourceValue> {
  const { accessibleModuleIds, user, establishments, isOrgAdmin } = ctx;
  const roles = user.roles;
  const businessUserId = user.businessUserId;

  const [tripsRaw, roomsRaw, reservationsRaw, profRoomRaw, weekSheet] = await Promise.all([
    accessibleModuleIds.has("travels")
      ? safeJson<TripIndexRow[]>("travels/index.json")
      : Promise.resolve(null),
    accessibleModuleIds.has("prof-room")
      ? safeJson<{ rooms?: { id: string; name: string }[] } | { id: string; name: string }[]>(
          "reservation-rooms/rooms.json",
        )
      : Promise.resolve(null),
    accessibleModuleIds.has("prof-room")
      ? safeJson<unknown>("reservation-rooms/reservations.json", 5_000)
      : Promise.resolve(null),
    accessibleModuleIds.has("prof-room")
      ? safeJson<unknown>("settings/modules/prof-room.json")
      : Promise.resolve(null),
    loadWeekSheetData().catch(() => null),
  ]);

  const rooms = parseRoomsRaw(roomsRaw);
  const reservations = normalizeRoomReservations(reservationsRaw);
  let roomSubjectColors: Record<string, string> = {};
  try {
    roomSubjectColors = withDefaultProfRoomSubjects(
      profRoomRaw ? parseProfRoomModule(profRoomRaw) : defaultProfRoomModule(),
    ).subjectColors;
  } catch {
    roomSubjectColors = withDefaultProfRoomSubjects(defaultProfRoomModule()).subjectColors;
  }

  const { resolveCurrentEtablissementId } = await import("@/app/lib/ent-core-db");
  const etabId = await resolveCurrentEtablissementId().catch(() => null);

  const loadVsSanctionsCarnet = async (): Promise<{
    vsSanctionsAujourdhui: number;
    vsCarnetNonSignees: number;
  }> => {
    if (
      !etabId ||
      (!accessibleModuleIds.has("vs-sanctions") && !accessibleModuleIds.has("vs-carnet"))
    ) {
      return { vsSanctionsAujourdhui: 0, vsCarnetNonSignees: 0 };
    }
    try {
      const { countSanctionsAujourdhui } = await import("@/app/lib/vs-sanctions-db");
      const { countCarnetNonSignees } = await import("@/app/lib/vs-carnet-db");
      const { parisDateKey } = await import("@/app/lib/paris-time");
      let vsSanctionsAujourdhui = 0;
      let vsCarnetNonSignees = 0;
      const tasks: Promise<void>[] = [];
      if (accessibleModuleIds.has("vs-sanctions")) {
        tasks.push(
          countSanctionsAujourdhui(etabId, parisDateKey(new Date())).then((n) => {
            vsSanctionsAujourdhui = n;
          }),
        );
      }
      if (accessibleModuleIds.has("vs-carnet")) {
        tasks.push(
          countCarnetNonSignees(etabId).then((n) => {
            vsCarnetNonSignees = n;
          }),
        );
      }
      await Promise.all(tasks);
      return { vsSanctionsAujourdhui, vsCarnetNonSignees };
    } catch {
      return { vsSanctionsAujourdhui: 0, vsCarnetNonSignees: 0 };
    }
  };

  const loadAnneeLabel = async (): Promise<string | null> => {
    if (!etabId) return null;
    try {
      const { resolveAnneeCouranteMeta } = await import("@/app/lib/annees-scolaires-db");
      return (await resolveAnneeCouranteMeta(etabId)).label;
    } catch {
      return null;
    }
  };

  const loadFactures = async (): Promise<number> => {
    if (!etabId) return 0;
    try {
      const { countFacturesEnRetard } = await import("@/app/lib/facturation-db");
      const { parisDateKey } = await import("@/app/lib/paris-time");
      return await countFacturesEnRetard(etabId, parisDateKey(new Date()));
    } catch {
      return 0;
    }
  };

  const loadCreneaux = async (): Promise<{
    creneauxVidesCount: number;
    creneauxVidesTravelId: string | null;
  }> => {
    if (!etabId) return { creneauxVidesCount: 0, creneauxVidesTravelId: null };
    try {
      const { loadCreneauVideSignalsForDashboard } = await import("@/app/lib/impact-engine");
      const { parisDateKey } = await import("@/app/lib/paris-time");
      const signals = await loadCreneauVideSignalsForDashboard({
        etablissementId: etabId,
        date: parisDateKey(new Date()),
      });
      const travelIds = [...new Set(signals.map((s) => s.travelId).filter(Boolean))];
      return {
        creneauxVidesCount: signals.length,
        creneauxVidesTravelId: travelIds.length === 1 ? travelIds[0]! : null,
      };
    } catch (err) {
      console.warn("[dashboard/signals] creneaux vides", err);
      return { creneauxVidesCount: 0, creneauxVidesTravelId: null };
    }
  };

  const loadAccompagnement = async (): Promise<OthersSourceValue["unseenAccompagnementAlerts"]> => {
    if (
      !etabId ||
      !accessibleModuleIds.has("eleve-dossier") ||
      !businessUserId ||
      !hasRole(roles, "professeur")
    ) {
      return [];
    }
    try {
      const { isProfesseurScopedDossierViewer } = await import("@/app/lib/eleve-dossier-scope");
      if (!isProfesseurScopedDossierViewer({ roles, orgAdmin: isOrgAdmin })) return [];
      const {
        listUnseenAccompagnementAlertsForTeacher,
        formatAccompagnementAlertDetail,
      } = await import("@/app/lib/eleve-accompagnement-alerts");
      const alerts = await listUnseenAccompagnementAlertsForTeacher({
        etablissementId: etabId,
        businessUserId,
      });
      return alerts.map((a) => ({
        documentId: a.documentId,
        eleveId: a.eleveId,
        eleveNom: a.eleveNom,
        elevePrenom: a.elevePrenom,
        classe: a.classe,
        kind: a.kind,
        title: a.title,
        detail: formatAccompagnementAlertDetail(a),
      }));
    } catch (err) {
      console.warn("[dashboard/signals] accompagnement alerts", err);
      return [];
    }
  };

  const loadTravelsUnread = async (): Promise<OthersSourceValue["travelsUnreadMessages"]> => {
    if (!accessibleModuleIds.has("travels")) return undefined;
    try {
      const { summarizeViewerTravelUnread } = await import("@/app/lib/travel-message-unread-db");
      const { travelThreadViewerFromStaff } = await import("@/app/lib/travels-thread-unread");
      const { listTravelsIndex } = await import("@/app/lib/travels-storage");
      const tripsForUnread =
        Array.isArray(tripsRaw) && tripsRaw.length > 0
          ? (tripsRaw as import("@/app/lib/travels-types").TravelsTrip[])
          : await listTravelsIndex();
      const { travelsDbReady } = await import("@/app/lib/travel-db");
      const travelsEtabId = await travelsDbReady();
      if (!travelsEtabId) return undefined;
      return await summarizeViewerTravelUnread({
        etablissementId: travelsEtabId,
        trips: tripsForUnread,
        viewer: travelThreadViewerFromStaff({
          id: user.id,
          businessUserId: user.businessUserId,
          email: user.email,
          roles: user.roles,
          firstName: user.firstName,
          lastName: user.lastName,
          name: user.name,
        }),
        establishments,
      });
    } catch (err) {
      console.warn("[dashboard/signals] travels unread", err);
      return undefined;
    }
  };

  const [vsBlock, anneeScolaireLabel, facturesEnRetard, creneauxBlock, unseenAccompagnementAlerts, travelsUnreadMessages] =
    await Promise.all([
      loadVsSanctionsCarnet(),
      loadAnneeLabel(),
      loadFactures(),
      loadCreneaux(),
      loadAccompagnement(),
      loadTravelsUnread(),
    ]);

  return {
    tripsRaw: Array.isArray(tripsRaw) ? tripsRaw : null,
    rooms,
    reservations,
    roomSubjectColors,
    weekSheet,
    vsSanctionsAujourdhui: vsBlock.vsSanctionsAujourdhui,
    vsCarnetNonSignees: vsBlock.vsCarnetNonSignees,
    facturesEnRetard,
    anneeScolaireLabel,
    unseenAccompagnementAlerts,
    creneauxVidesCount: creneauxBlock.creneauxVidesCount,
    creneauxVidesTravelId: creneauxBlock.creneauxVidesTravelId,
    travelsUnreadMessages,
  };
}

export async function loadDashboardSignalSources(ctx: DashboardSignalsLoadContext): Promise<{
  bundle: DashboardSignalSourcesBundle;
  stepsMs: Record<DashboardSignalSourceId, number>;
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus>;
}> {
  const { outcomes, stepsMs, sourceStatus } = await runParallelSignalSources<DashboardSignalSourcesBundle>({
    internat: () => loadInternatSource(ctx),
    rh: () => loadRhSource(ctx),
    demandes: () => loadDemandesSource(ctx),
    absences: () => loadAbsencesSource(ctx),
    documents: () => loadDocumentsSource(ctx),
    stages: () => loadStagesSource(ctx),
    others: () => loadOthersSource(ctx),
  });

  const bundle: DashboardSignalSourcesBundle = {
    internat: unwrapSignalSource(outcomes.internat, EMPTY_INTERNAT),
    rh: unwrapSignalSource(outcomes.rh, EMPTY_RH),
    demandes: unwrapSignalSource(outcomes.demandes, EMPTY_DEMANDES),
    absences: unwrapSignalSource(outcomes.absences, EMPTY_ABSENCES),
    documents: unwrapSignalSource(outcomes.documents, EMPTY_DOCUMENTS),
    stages: unwrapSignalSource(outcomes.stages, EMPTY_STAGES),
    others: unwrapSignalSource(outcomes.others, EMPTY_OTHERS),
  };

  for (const id of Object.keys(outcomes) as DashboardSignalSourceId[]) {
    const o = outcomes[id];
    if (!o.ok) {
      console.error(`[dashboard/signals] source ${id}`, o.error);
    }
  }

  return { bundle, stepsMs, sourceStatus };
}

export function buildLoadContextFromViewUser(
  viewUser: DashboardSignalsViewUser,
  opts: {
    signalsEtabId: string;
    isOrgAdmin: boolean;
    accessibleModuleIds: Set<string>;
    moduleAccess: ModuleAccessConfig;
    isPhotoOps: boolean;
    photocopiesOpsEmails: string[];
    appBundle: Awaited<ReturnType<typeof loadAppConfig>> | null;
    establishments: Establishment[];
    absenceNotifications: NotificationsConfig | null;
  },
): DashboardSignalsLoadContext {
  return {
    signalsEtabId: opts.signalsEtabId,
    user: viewUser,
    isOrgAdmin: opts.isOrgAdmin,
    accessibleModuleIds: opts.accessibleModuleIds,
    moduleAccess: opts.moduleAccess,
    isPhotoOps: opts.isPhotoOps,
    photocopiesOpsEmails: opts.photocopiesOpsEmails,
    appBundle: opts.appBundle,
    establishments: opts.establishments,
    absenceNotifications: opts.absenceNotifications,
  };
}
